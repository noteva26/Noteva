//! Plugin database operations
//!
//! Handles plugin migration execution and SQL sandboxing.
//! Plugins can create their own tables (prefixed with `plugin_{id}_`)
//! and run queries against them via host functions.

use anyhow::{bail, Context, Result};
use sqlx::{Column, Row};
use tracing::{debug, info};

use crate::config::DatabaseDriver;
use crate::db::migrations::split_sql_statements;
use crate::db::DynDatabasePool;

/// Run pending migrations for a plugin.
///
/// Reads SQL files from `plugins/{id}/migrations/`, checks which have
/// already been applied (via `plugin_migrations` table), and executes
/// any new ones in filename order.
///
/// Table name validation: all CREATE TABLE statements must use the
/// `plugin_{id}_` prefix. Migrations that violate this are rejected.
pub async fn run_plugin_migrations(
    pool: &DynDatabasePool,
    plugin_id: &str,
    migrations: &[(String, String)], // (filename, sql_content)
) -> Result<usize> {
    if migrations.is_empty() {
        return Ok(0);
    }

    let applied = get_applied_plugin_migrations(pool, plugin_id).await?;
    let mut count = 0;

    for (filename, sql) in migrations {
        if applied.contains(filename) {
            debug!(
                "Plugin {} migration {} already applied, skipping",
                plugin_id, filename
            );
            continue;
        }

        // Validate table names in the SQL
        validate_plugin_sql(plugin_id, sql)?;

        // Execute the migration
        info!("Running plugin {} migration: {}", plugin_id, filename);
        execute_plugin_migration(pool, plugin_id, filename, sql)
            .await
            .with_context(|| format!("Plugin {} migration {} failed", plugin_id, filename))?;
        count += 1;
    }

    if count > 0 {
        info!("Plugin {} applied {} migration(s)", plugin_id, count);
    }

    Ok(count)
}

/// Return the SQL table prefix reserved for a plugin.
///
/// Plugin ids use kebab-case in `plugin.json`, while SQL identifiers should use
/// underscores. For example, `ai-summary` owns tables named
/// `plugin_ai_summary_*`.
pub fn plugin_table_prefix(plugin_id: &str) -> String {
    format!("plugin_{}_", plugin_id.replace('-', "_"))
}

/// Validate plugin migration SQL.
pub(crate) fn validate_plugin_migration_sql(plugin_id: &str, sql: &str) -> Result<()> {
    validate_plugin_sql(plugin_id, sql)
}

/// Validate that plugin SQL only touches tables with the correct prefix.
/// Rejects schema changes on core tables.
fn validate_plugin_sql(plugin_id: &str, sql: &str) -> Result<()> {
    let statements = split_sql_statements(sql);
    if statements.is_empty() {
        bail!(
            "Plugin '{}' migration contains no SQL statements",
            plugin_id
        );
    }

    for statement in statements {
        let cleaned = mask_sql_comments_and_literals(statement);
        let first = first_keyword(&cleaned).unwrap_or_default();
        if !matches!(
            first.as_str(),
            "CREATE" | "ALTER" | "DROP" | "INSERT" | "UPDATE" | "DELETE"
        ) {
            bail!(
                "Plugin '{}' migration statement '{}' is not allowed",
                plugin_id,
                truncate(statement.trim(), 80)
            );
        }

        validate_index_identifier(plugin_id, &cleaned)?;
        validate_table_references(plugin_id, &cleaned, "migration")?;
    }

    Ok(())
}

/// Validate a runtime SQL query from a plugin (SELECT/INSERT/UPDATE/DELETE).
/// Ensures it only touches `plugin_{id}_*` tables and doesn't contain DDL.
pub fn validate_plugin_query(plugin_id: &str, sql: &str) -> Result<()> {
    validate_runtime_sql(plugin_id, sql, RuntimeSqlMode::Any)
}

#[derive(Copy, Clone)]
enum RuntimeSqlMode {
    Any,
    Query,
    Statement,
}

fn validate_runtime_sql(plugin_id: &str, sql: &str, mode: RuntimeSqlMode) -> Result<()> {
    let statements = split_sql_statements(sql);
    if statements.len() != 1 {
        bail!(
            "Plugin '{}': runtime SQL must contain exactly one statement",
            plugin_id
        );
    }

    let statement = statements[0].trim();
    let cleaned = mask_sql_comments_and_literals(statement);
    let first = first_keyword(&cleaned).unwrap_or_default();

    match mode {
        RuntimeSqlMode::Any => {
            if !matches!(first.as_str(), "SELECT" | "INSERT" | "UPDATE" | "DELETE") {
                bail!(
                    "Plugin '{}': runtime SQL must be SELECT, INSERT, UPDATE, or DELETE",
                    plugin_id
                );
            }
        }
        RuntimeSqlMode::Query => {
            if first != "SELECT" {
                bail!("Plugin '{}': host_db_query only accepts SELECT", plugin_id);
            }
        }
        RuntimeSqlMode::Statement => {
            if !matches!(first.as_str(), "INSERT" | "UPDATE" | "DELETE") {
                bail!(
                    "Plugin '{}': host_db_execute only accepts INSERT, UPDATE, or DELETE",
                    plugin_id
                );
            }
        }
    }

    for keyword in [
        "CREATE", "ALTER", "DROP", "TRUNCATE", "PRAGMA", "ATTACH", "DETACH", "VACUUM",
    ] {
        if contains_keyword(&cleaned, keyword) {
            bail!(
                "Plugin '{}': DDL/control statements are not allowed at runtime. Use migrations/ instead.",
                plugin_id
            );
        }
    }

    validate_table_references(plugin_id, &cleaned, "query")
}

/// Execute a plugin SQL query (SELECT) and return JSON rows.
pub async fn execute_plugin_query(
    pool: &DynDatabasePool,
    plugin_id: &str,
    sql: &str,
    params_json: &str,
) -> Result<String> {
    validate_runtime_sql(plugin_id, sql, RuntimeSqlMode::Query)?;

    let params: Vec<serde_json::Value> = serde_json::from_str(params_json).unwrap_or_default();

    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let sqlite_pool = pool.as_sqlite_or_err()?;
            let mut query = sqlx::query(sql);
            for p in &params {
                query = bind_json_param_sqlite(query, p);
            }
            let rows = query
                .fetch_all(sqlite_pool)
                .await
                .with_context(|| format!("Plugin {} query failed", plugin_id))?;

            let mut result = Vec::new();
            for row in &rows {
                let mut obj = serde_json::Map::new();
                for col in row.columns() {
                    let name = col.name();
                    let val = sqlite_row_value(row, col);
                    obj.insert(name.to_string(), val);
                }
                result.push(serde_json::Value::Object(obj));
            }
            Ok(serde_json::to_string(&result)?)
        }
        DatabaseDriver::Mysql => {
            let mysql_pool = pool.as_mysql_or_err()?;
            let mut query = sqlx::query(sql);
            for p in &params {
                query = bind_json_param_mysql(query, p);
            }
            let rows = query
                .fetch_all(mysql_pool)
                .await
                .with_context(|| format!("Plugin {} query failed", plugin_id))?;

            let mut result = Vec::new();
            for row in &rows {
                let mut obj = serde_json::Map::new();
                for col in row.columns() {
                    let name = col.name();
                    let val = mysql_row_value(row, col);
                    obj.insert(name.to_string(), val);
                }
                result.push(serde_json::Value::Object(obj));
            }
            Ok(serde_json::to_string(&result)?)
        }
    }
}

/// Execute a plugin SQL statement (INSERT/UPDATE/DELETE) and return affected rows.
pub async fn execute_plugin_statement(
    pool: &DynDatabasePool,
    plugin_id: &str,
    sql: &str,
    params_json: &str,
) -> Result<u64> {
    validate_runtime_sql(plugin_id, sql, RuntimeSqlMode::Statement)?;

    let params: Vec<serde_json::Value> = serde_json::from_str(params_json).unwrap_or_default();

    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let sqlite_pool = pool.as_sqlite_or_err()?;
            let mut query = sqlx::query(sql);
            for p in &params {
                query = bind_json_param_sqlite(query, p);
            }
            let result = query
                .execute(sqlite_pool)
                .await
                .with_context(|| format!("Plugin {} execute failed", plugin_id))?;
            Ok(result.rows_affected())
        }
        DatabaseDriver::Mysql => {
            let mysql_pool = pool.as_mysql_or_err()?;
            let mut query = sqlx::query(sql);
            for p in &params {
                query = bind_json_param_mysql(query, p);
            }
            let result = query
                .execute(mysql_pool)
                .await
                .with_context(|| format!("Plugin {} execute failed", plugin_id))?;
            Ok(result.rows_affected())
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn validate_table_references(plugin_id: &str, sql: &str, context: &str) -> Result<()> {
    for keyword in ["FROM", "JOIN", "INTO", "TABLE", "REFERENCES"] {
        let mut search_from = 0;
        while let Some(pos) = find_keyword_from(sql, keyword, search_from) {
            let after_keyword = pos + keyword.len();
            let table = next_identifier_after(sql, after_keyword, keyword);
            if let Some(table) = table {
                if table == "(" || table.eq_ignore_ascii_case("SELECT") {
                    search_from = after_keyword;
                    continue;
                }
                ensure_plugin_identifier(plugin_id, &table, context)?;
            }
            search_from = after_keyword;
        }
    }

    if first_keyword(sql).as_deref() == Some("UPDATE") {
        if let Some(update_pos) = find_keyword_from(sql, "UPDATE", 0) {
            if let Some(table) = next_identifier_after(sql, update_pos + "UPDATE".len(), "UPDATE") {
                ensure_plugin_identifier(plugin_id, &table, context)?;
            }
        }
    }

    if starts_with_create_index(sql) {
        if let Some(on_pos) = find_keyword_from(sql, "ON", 0) {
            if let Some(table) = next_identifier_after(sql, on_pos + 2, "ON") {
                ensure_plugin_identifier(plugin_id, &table, context)?;
            }
        }
    }

    Ok(())
}

fn validate_index_identifier(plugin_id: &str, sql: &str) -> Result<()> {
    let upper = sql.to_ascii_uppercase();
    let Some(first) = first_keyword(sql) else {
        return Ok(());
    };
    if first != "CREATE" && first != "DROP" {
        return Ok(());
    }
    if !contains_keyword(sql, "INDEX") {
        return Ok(());
    }

    let Some(index_pos) = find_keyword_from(&upper, "INDEX", 0) else {
        return Ok(());
    };
    if let Some(index_name) = next_identifier_after(sql, index_pos + "INDEX".len(), "INDEX") {
        ensure_plugin_identifier(plugin_id, &index_name, "migration index")?;
    }
    Ok(())
}

fn ensure_plugin_identifier(plugin_id: &str, identifier: &str, context: &str) -> Result<()> {
    let normalized = normalize_sql_identifier(identifier);
    if normalized.is_empty() {
        return Ok(());
    }

    let prefix = plugin_table_prefix(plugin_id);
    if !normalized
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
    {
        bail!(
            "Plugin '{}': {} references '{}' but only '{}*' identifiers are allowed",
            plugin_id,
            context,
            normalized,
            prefix
        );
    }

    Ok(())
}

fn next_identifier_after(sql: &str, start: usize, keyword: &str) -> Option<String> {
    let mut index = skip_whitespace(sql, start);

    if keyword == "TABLE" {
        index = skip_optional_words(sql, index, &["IF", "NOT", "EXISTS"]);
    } else if keyword == "INDEX" {
        index = skip_optional_words(sql, index, &["IF", "NOT", "EXISTS"]);
    }

    read_identifier(sql, index)
}

fn skip_optional_words(sql: &str, mut index: usize, words: &[&str]) -> usize {
    for word in words {
        let next = skip_whitespace(sql, index);
        let Some(identifier) = read_identifier(sql, next) else {
            break;
        };
        if normalize_sql_identifier(&identifier).eq_ignore_ascii_case(word) {
            index = next + identifier.len();
        } else {
            break;
        }
    }
    index
}

fn read_identifier(sql: &str, start: usize) -> Option<String> {
    let start = skip_whitespace(sql, start);
    let rest = sql.get(start..)?;
    let mut chars = rest.char_indices();
    let (_, first) = chars.next()?;

    if first == '(' {
        return Some("(".to_string());
    }

    let closing = match first {
        '`' => Some('`'),
        '"' => Some('"'),
        '[' => Some(']'),
        _ => None,
    };

    if let Some(closing) = closing {
        for (offset, ch) in chars {
            if ch == closing {
                let end = offset + ch.len_utf8();
                return Some(rest[..end].to_string());
            }
        }
        return Some(rest.to_string());
    }

    let mut end = first.len_utf8();
    for (offset, ch) in chars {
        if ch.is_whitespace() || matches!(ch, '(' | ')' | ',' | ';') {
            break;
        }
        end = offset + ch.len_utf8();
    }
    Some(rest[..end].to_string())
}

fn skip_whitespace(sql: &str, mut index: usize) -> usize {
    while let Some(ch) = sql.get(index..).and_then(|rest| rest.chars().next()) {
        if !ch.is_whitespace() {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn normalize_sql_identifier(identifier: &str) -> String {
    let trimmed = identifier
        .trim()
        .trim_end_matches(';')
        .trim_end_matches(',')
        .trim();
    let last_part = trimmed.rsplit('.').next().unwrap_or(trimmed).trim();
    last_part
        .trim_matches('`')
        .trim_matches('"')
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string()
}

fn mask_sql_comments_and_literals(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut in_string = false;

    while let Some(ch) = chars.next() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                result.push('\n');
            } else {
                result.push(' ');
            }
            continue;
        }

        if in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
                result.push(' ');
                result.push(' ');
            } else {
                result.push(if ch == '\n' { '\n' } else { ' ' });
            }
            continue;
        }

        if in_string {
            if ch == '\'' {
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    result.push(' ');
                    result.push(' ');
                    continue;
                }
                in_string = false;
            }
            result.push(if ch == '\n' { '\n' } else { ' ' });
            continue;
        }

        if ch == '-' && chars.peek() == Some(&'-') {
            chars.next();
            in_line_comment = true;
            result.push(' ');
            result.push(' ');
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_block_comment = true;
            result.push(' ');
            result.push(' ');
            continue;
        }
        if ch == '\'' {
            in_string = true;
            result.push(' ');
            continue;
        }

        result.push(ch);
    }

    result
}

fn first_keyword(sql: &str) -> Option<String> {
    sql.split(|ch: char| !ch.is_ascii_alphabetic())
        .find(|part| !part.is_empty())
        .map(|part| part.to_ascii_uppercase())
}

fn contains_keyword(sql: &str, keyword: &str) -> bool {
    find_keyword_from(sql, keyword, 0).is_some()
}

fn find_keyword_from(sql: &str, keyword: &str, start: usize) -> Option<usize> {
    let upper = sql.to_ascii_uppercase();
    let keyword = keyword.to_ascii_uppercase();
    let mut search_from = start.min(upper.len());

    while let Some(pos) = upper[search_from..].find(&keyword) {
        let abs = search_from + pos;
        let before = upper[..abs].chars().next_back();
        let after = upper[abs + keyword.len()..].chars().next();
        let before_ok = before.map(|ch| !is_identifier_char(ch)).unwrap_or(true);
        let after_ok = after.map(|ch| !is_identifier_char(ch)).unwrap_or(true);
        if before_ok && after_ok {
            return Some(abs);
        }
        search_from = abs + keyword.len();
    }

    None
}

fn starts_with_create_index(sql: &str) -> bool {
    let upper = sql.trim_start().to_ascii_uppercase();
    upper.starts_with("CREATE INDEX") || upper.starts_with("CREATE UNIQUE INDEX")
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// Get list of already-applied migration filenames for a plugin.
async fn get_applied_plugin_migrations(
    pool: &DynDatabasePool,
    plugin_id: &str,
) -> Result<Vec<String>> {
    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let sqlite_pool = pool.as_sqlite_or_err()?;
            let rows = sqlx::query(
                "SELECT filename FROM plugin_migrations WHERE plugin_id = ? ORDER BY filename",
            )
            .bind(plugin_id)
            .fetch_all(sqlite_pool)
            .await
            .context("Failed to query plugin_migrations")?;
            Ok(rows
                .iter()
                .map(|r| r.get::<String, _>("filename"))
                .collect())
        }
        DatabaseDriver::Mysql => {
            let mysql_pool = pool.as_mysql_or_err()?;
            let rows = sqlx::query(
                "SELECT filename FROM plugin_migrations WHERE plugin_id = ? ORDER BY filename",
            )
            .bind(plugin_id)
            .fetch_all(mysql_pool)
            .await
            .context("Failed to query plugin_migrations")?;
            Ok(rows
                .iter()
                .map(|r| r.get::<String, _>("filename"))
                .collect())
        }
    }
}

/// Execute raw migration SQL and record it atomically where the database allows it.
async fn execute_plugin_migration(
    pool: &DynDatabasePool,
    plugin_id: &str,
    filename: &str,
    sql: &str,
) -> Result<()> {
    let statements = split_sql_statements(sql);
    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let sqlite_pool = pool.as_sqlite_or_err()?;
            let mut tx = sqlite_pool.begin().await.with_context(|| {
                format!("Failed to start plugin {} migration transaction", plugin_id)
            })?;

            let result = async {
                for stmt in &statements {
                    let stmt = stmt.trim();
                    if !stmt.is_empty() {
                        sqlx::query(stmt).execute(&mut *tx).await.with_context(|| {
                            format!("Plugin SQL failed: {}", truncate(stmt, 120))
                        })?;
                    }
                }

                sqlx::query("INSERT INTO plugin_migrations (plugin_id, filename) VALUES (?, ?)")
                    .bind(plugin_id)
                    .bind(filename)
                    .execute(&mut *tx)
                    .await
                    .context("Failed to record plugin migration")?;

                Ok::<_, anyhow::Error>(())
            }
            .await;

            if let Err(err) = result {
                tx.rollback().await.with_context(|| {
                    format!("Failed to roll back plugin {} migration", plugin_id)
                })?;
                return Err(err);
            }

            tx.commit()
                .await
                .with_context(|| format!("Failed to commit plugin {} migration", plugin_id))?;
        }
        DatabaseDriver::Mysql => {
            let mysql_pool = pool.as_mysql_or_err()?;
            let mut tx = mysql_pool.begin().await.with_context(|| {
                format!("Failed to start plugin {} migration transaction", plugin_id)
            })?;

            let result = async {
                for stmt in &statements {
                    let stmt = stmt.trim();
                    if !stmt.is_empty() {
                        sqlx::query(stmt).execute(&mut *tx).await.with_context(|| {
                            format!("Plugin SQL failed: {}", truncate(stmt, 120))
                        })?;
                    }
                }

                sqlx::query("INSERT INTO plugin_migrations (plugin_id, filename) VALUES (?, ?)")
                    .bind(plugin_id)
                    .bind(filename)
                    .execute(&mut *tx)
                    .await
                    .context("Failed to record plugin migration")?;

                Ok::<_, anyhow::Error>(())
            }
            .await;

            if let Err(err) = result {
                tx.rollback().await.with_context(|| {
                    format!("Failed to roll back plugin {} migration", plugin_id)
                })?;
                return Err(err);
            }

            tx.commit()
                .await
                .with_context(|| format!("Failed to commit plugin {} migration", plugin_id))?;
        }
    }
    Ok(())
}

/// Bind a serde_json::Value as a parameter to a SQLite query.
fn bind_json_param_sqlite<'q>(
    query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    value: &'q serde_json::Value,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    match value {
        serde_json::Value::Null => query.bind(None::<String>),
        serde_json::Value::Bool(b) => query.bind(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                query.bind(i)
            } else if let Some(f) = n.as_f64() {
                query.bind(f)
            } else {
                query.bind(n.to_string())
            }
        }
        serde_json::Value::String(s) => query.bind(s.as_str()),
        _ => query.bind(value.to_string()),
    }
}

/// Bind a serde_json::Value as a parameter to a MySQL query.
fn bind_json_param_mysql<'q>(
    query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    value: &'q serde_json::Value,
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match value {
        serde_json::Value::Null => query.bind(None::<String>),
        serde_json::Value::Bool(b) => query.bind(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                query.bind(i)
            } else if let Some(f) = n.as_f64() {
                query.bind(f)
            } else {
                query.bind(n.to_string())
            }
        }
        serde_json::Value::String(s) => query.bind(s.as_str()),
        _ => query.bind(value.to_string()),
    }
}

/// Extract a column value from a SQLite row as serde_json::Value.
fn sqlite_row_value(
    row: &sqlx::sqlite::SqliteRow,
    col: &sqlx::sqlite::SqliteColumn,
) -> serde_json::Value {
    use sqlx::TypeInfo;
    let type_name = col.type_info().name();
    let idx = col.ordinal();
    match type_name {
        "INTEGER" | "INT" | "BIGINT" => row
            .try_get::<i64, _>(idx)
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null),
        "REAL" | "FLOAT" | "DOUBLE" => row
            .try_get::<f64, _>(idx)
            .map(|f| {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            })
            .unwrap_or(serde_json::Value::Null),
        "BOOLEAN" => row
            .try_get::<bool, _>(idx)
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null),
        _ => {
            // TEXT, VARCHAR, BLOB, etc. — try as string
            row.try_get::<String, _>(idx)
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null)
        }
    }
}

/// Extract a column value from a MySQL row as serde_json::Value.
fn mysql_row_value(
    row: &sqlx::mysql::MySqlRow,
    col: &sqlx::mysql::MySqlColumn,
) -> serde_json::Value {
    use sqlx::TypeInfo;
    let type_name = col.type_info().name();
    let idx = col.ordinal();
    match type_name {
        "BIGINT" | "INT" | "MEDIUMINT" | "SMALLINT" | "TINYINT" => row
            .try_get::<i64, _>(idx)
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null),
        "FLOAT" | "DOUBLE" | "DECIMAL" => row
            .try_get::<f64, _>(idx)
            .map(|f| {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            })
            .unwrap_or(serde_json::Value::Null),
        "BOOLEAN" => row
            .try_get::<bool, _>(idx)
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null),
        _ => row
            .try_get::<String, _>(idx)
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null),
    }
}

/// Truncate a string for error messages.
fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_test_pool, migrations};

    #[test]
    fn plugin_table_prefix_normalizes_hyphenated_ids() {
        assert_eq!(plugin_table_prefix("ai-summary"), "plugin_ai_summary_");
    }

    #[test]
    fn validate_query_allows_normalized_plugin_tables() {
        validate_plugin_query(
            "ai-summary",
            "SELECT id, title FROM plugin_ai_summary_items WHERE id = ?",
        )
        .expect("normalized table prefix should be valid");
    }

    #[test]
    fn validate_query_rejects_raw_hyphen_prefix() {
        let result = validate_plugin_query("ai-summary", "SELECT * FROM plugin_ai-summary_items");
        assert!(result.is_err());
    }

    #[test]
    fn validate_query_rejects_core_tables() {
        let result = validate_plugin_query("demo", "SELECT * FROM articles");
        assert!(result.is_err());
    }

    #[test]
    fn validate_query_rejects_runtime_ddl() {
        let result = validate_plugin_query(
            "demo",
            "CREATE TABLE plugin_demo_items (id INTEGER PRIMARY KEY)",
        );
        assert!(result.is_err());
    }

    #[test]
    fn validate_migration_rejects_core_table_references() {
        let result = validate_plugin_migration_sql(
            "demo",
            "CREATE TABLE plugin_demo_items (id INTEGER PRIMARY KEY, article_id INTEGER REFERENCES articles(id));",
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn plugin_migration_failure_rolls_back_sqlite() {
        let pool = create_test_pool().await.expect("test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("core migrations");

        let result = run_plugin_migrations(
            &pool,
            "demo",
            &[(
                "001_fail.sql".to_string(),
                r#"
                CREATE TABLE plugin_demo_probe (id INTEGER PRIMARY KEY);
                INSERT INTO plugin_demo_missing (id) VALUES (1);
                "#
                .to_string(),
            )],
        )
        .await;
        assert!(result.is_err());

        let sqlite = pool.as_sqlite().expect("sqlite pool");
        let table_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'plugin_demo_probe'",
        )
        .fetch_one(sqlite)
        .await
        .expect("query sqlite_master");
        assert_eq!(table_exists, 0);

        let record_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM plugin_migrations WHERE plugin_id = ? AND filename = ?",
        )
        .bind("demo")
        .bind("001_fail.sql")
        .fetch_one(sqlite)
        .await
        .expect("query plugin_migrations");
        assert_eq!(record_count, 0);
    }
}
