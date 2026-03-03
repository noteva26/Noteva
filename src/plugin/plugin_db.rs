//! Plugin database operations
//!
//! Handles plugin migration execution and SQL sandboxing.
//! Plugins can create their own tables (prefixed with `plugin_{id}_`)
//! and run queries against them via host functions.

use anyhow::{bail, Context, Result};
use sqlx::{Column, Row};
use tracing::{debug, info};

use crate::db::migrations::split_sql_statements;
use crate::db::DynDatabasePool;
use crate::config::DatabaseDriver;

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
            debug!("Plugin {} migration {} already applied, skipping", plugin_id, filename);
            continue;
        }

        // Validate table names in the SQL
        validate_plugin_sql(plugin_id, sql)?;

        // Execute the migration
        info!("Running plugin {} migration: {}", plugin_id, filename);
        execute_plugin_sql(pool, sql)
            .await
            .with_context(|| format!("Plugin {} migration {} failed", plugin_id, filename))?;

        // Record it
        record_plugin_migration(pool, plugin_id, filename).await?;
        count += 1;
    }

    if count > 0 {
        info!("Plugin {} applied {} migration(s)", plugin_id, count);
    }

    Ok(count)
}

/// Validate that plugin SQL only touches tables with the correct prefix.
/// Rejects DDL on core tables and any DROP/ALTER without the prefix.
fn validate_plugin_sql(plugin_id: &str, sql: &str) -> Result<()> {
    let prefix = format!("plugin_{}_", plugin_id);
    let sql_upper = sql.to_uppercase();

    for line in sql_upper.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }

        // Check CREATE TABLE statements
        if let Some(pos) = trimmed.find("CREATE TABLE") {
            let after = &trimmed[pos + 12..].trim_start();
            let after = after.strip_prefix("IF NOT EXISTS").unwrap_or(after).trim_start();
            let table_name = after.split(|c: char| c.is_whitespace() || c == '(')
                .next()
                .unwrap_or("");
            let prefix_upper = prefix.to_uppercase();
            if !table_name.starts_with(&prefix_upper) {
                bail!(
                    "Plugin '{}' migration error: table '{}' must start with '{}'",
                    plugin_id, table_name, prefix
                );
            }
        }

        // Check ALTER TABLE
        if let Some(pos) = trimmed.find("ALTER TABLE") {
            let after = &trimmed[pos + 11..].trim_start();
            let table_name = after.split_whitespace().next().unwrap_or("");
            let prefix_upper = prefix.to_uppercase();
            if !table_name.starts_with(&prefix_upper) {
                bail!(
                    "Plugin '{}' migration error: ALTER TABLE '{}' must target '{}_*' tables",
                    plugin_id, table_name, prefix
                );
            }
        }

        // Check DROP TABLE
        if let Some(pos) = trimmed.find("DROP TABLE") {
            let after = &trimmed[pos + 10..].trim_start();
            let after = after.strip_prefix("IF EXISTS").unwrap_or(after).trim_start();
            let table_name = after.split(|c: char| c.is_whitespace() || c == ';')
                .next()
                .unwrap_or("");
            let prefix_upper = prefix.to_uppercase();
            if !table_name.starts_with(&prefix_upper) {
                bail!(
                    "Plugin '{}' migration error: DROP TABLE '{}' must target '{}_*' tables",
                    plugin_id, table_name, prefix
                );
            }
        }
    }

    Ok(())
}

/// Validate a runtime SQL query from a plugin (SELECT/INSERT/UPDATE/DELETE).
/// Ensures it only touches `plugin_{id}_*` tables and doesn't contain DDL.
pub fn validate_plugin_query(plugin_id: &str, sql: &str) -> Result<()> {
    let sql_upper = sql.to_uppercase().trim().to_string();

    // Block DDL entirely — plugins must use migrations for schema changes
    let ddl_keywords = ["CREATE ", "ALTER ", "DROP ", "TRUNCATE "];
    for kw in &ddl_keywords {
        if sql_upper.contains(kw) {
            bail!("Plugin '{}': DDL statements not allowed in queries. Use migrations/ instead.", plugin_id);
        }
    }

    // Extract table references and validate prefix
    let prefix = format!("PLUGIN_{}_", plugin_id.to_uppercase());

    // Check FROM clauses
    for keyword in &["FROM ", "JOIN ", "INTO ", "UPDATE "] {
        let mut search_from = 0;
        while let Some(pos) = sql_upper[search_from..].find(keyword) {
            let abs_pos = search_from + pos + keyword.len();
            let after = sql_upper[abs_pos..].trim_start();
            let table_name = after.split(|c: char| c.is_whitespace() || c == '(' || c == ',' || c == ';')
                .next()
                .unwrap_or("");
            // Skip subqueries
            if table_name == "(" || table_name == "SELECT" || table_name.is_empty() {
                search_from = abs_pos;
                continue;
            }
            if !table_name.starts_with(&prefix) {
                bail!(
                    "Plugin '{}': query references table '{}' which doesn't have prefix 'plugin_{}_'",
                    plugin_id, table_name.to_lowercase(), plugin_id
                );
            }
            search_from = abs_pos;
        }
    }

    Ok(())
}

/// Execute a plugin SQL query (SELECT) and return JSON rows.
pub async fn execute_plugin_query(
    pool: &DynDatabasePool,
    plugin_id: &str,
    sql: &str,
    params_json: &str,
) -> Result<String> {
    validate_plugin_query(plugin_id, sql)?;

    let params: Vec<serde_json::Value> = serde_json::from_str(params_json)
        .unwrap_or_default();

    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let sqlite_pool = pool.as_sqlite_or_err()?;
            let mut query = sqlx::query(sql);
            for p in &params {
                query = bind_json_param_sqlite(query, p);
            }
            let rows = query.fetch_all(sqlite_pool).await
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
            let rows = query.fetch_all(mysql_pool).await
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
    validate_plugin_query(plugin_id, sql)?;

    let params: Vec<serde_json::Value> = serde_json::from_str(params_json)
        .unwrap_or_default();

    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let sqlite_pool = pool.as_sqlite_or_err()?;
            let mut query = sqlx::query(sql);
            for p in &params {
                query = bind_json_param_sqlite(query, p);
            }
            let result = query.execute(sqlite_pool).await
                .with_context(|| format!("Plugin {} execute failed", plugin_id))?;
            Ok(result.rows_affected())
        }
        DatabaseDriver::Mysql => {
            let mysql_pool = pool.as_mysql_or_err()?;
            let mut query = sqlx::query(sql);
            for p in &params {
                query = bind_json_param_mysql(query, p);
            }
            let result = query.execute(mysql_pool).await
                .with_context(|| format!("Plugin {} execute failed", plugin_id))?;
            Ok(result.rows_affected())
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Get list of already-applied migration filenames for a plugin.
async fn get_applied_plugin_migrations(
    pool: &DynDatabasePool,
    plugin_id: &str,
) -> Result<Vec<String>> {
    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let sqlite_pool = pool.as_sqlite_or_err()?;
            let rows = sqlx::query(
                "SELECT filename FROM plugin_migrations WHERE plugin_id = ? ORDER BY filename"
            )
            .bind(plugin_id)
            .fetch_all(sqlite_pool)
            .await
            .context("Failed to query plugin_migrations")?;
            Ok(rows.iter().map(|r| r.get::<String, _>("filename")).collect())
        }
        DatabaseDriver::Mysql => {
            let mysql_pool = pool.as_mysql_or_err()?;
            let rows = sqlx::query(
                "SELECT filename FROM plugin_migrations WHERE plugin_id = ? ORDER BY filename"
            )
            .bind(plugin_id)
            .fetch_all(mysql_pool)
            .await
            .context("Failed to query plugin_migrations")?;
            Ok(rows.iter().map(|r| r.get::<String, _>("filename")).collect())
        }
    }
}

/// Record a successfully applied plugin migration.
async fn record_plugin_migration(
    pool: &DynDatabasePool,
    plugin_id: &str,
    filename: &str,
) -> Result<()> {
    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let sqlite_pool = pool.as_sqlite_or_err()?;
            sqlx::query(
                "INSERT INTO plugin_migrations (plugin_id, filename) VALUES (?, ?)"
            )
            .bind(plugin_id)
            .bind(filename)
            .execute(sqlite_pool)
            .await
            .context("Failed to record plugin migration")?;
        }
        DatabaseDriver::Mysql => {
            let mysql_pool = pool.as_mysql_or_err()?;
            sqlx::query(
                "INSERT INTO plugin_migrations (plugin_id, filename) VALUES (?, ?)"
            )
            .bind(plugin_id)
            .bind(filename)
            .execute(mysql_pool)
            .await
            .context("Failed to record plugin migration")?;
        }
    }
    Ok(())
}

/// Execute raw migration SQL, splitting multi-statement strings by `;`.
async fn execute_plugin_sql(pool: &DynDatabasePool, sql: &str) -> Result<()> {
    let statements = split_sql_statements(sql);
    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let sqlite_pool = pool.as_sqlite_or_err()?;
            for stmt in statements {
                let stmt = stmt.trim();
                if !stmt.is_empty() {
                    sqlx::query(stmt)
                        .execute(sqlite_pool)
                        .await
                        .with_context(|| format!("Plugin SQL failed: {}", truncate(stmt, 120)))?;
                }
            }
        }
        DatabaseDriver::Mysql => {
            let mysql_pool = pool.as_mysql_or_err()?;
            for stmt in statements {
                let stmt = stmt.trim();
                if !stmt.is_empty() {
                    sqlx::query(stmt)
                        .execute(mysql_pool)
                        .await
                        .with_context(|| format!("Plugin SQL failed: {}", truncate(stmt, 120)))?;
                }
            }
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
fn sqlite_row_value(row: &sqlx::sqlite::SqliteRow, col: &sqlx::sqlite::SqliteColumn) -> serde_json::Value {
    use sqlx::TypeInfo;
    let type_name = col.type_info().name();
    let idx = col.ordinal();
    match type_name {
        "INTEGER" | "INT" | "BIGINT" => {
            row.try_get::<i64, _>(idx)
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null)
        }
        "REAL" | "FLOAT" | "DOUBLE" => {
            row.try_get::<f64, _>(idx)
                .map(|f| serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null))
                .unwrap_or(serde_json::Value::Null)
        }
        "BOOLEAN" => {
            row.try_get::<bool, _>(idx)
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null)
        }
        _ => {
            // TEXT, VARCHAR, BLOB, etc. — try as string
            row.try_get::<String, _>(idx)
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null)
        }
    }
}

/// Extract a column value from a MySQL row as serde_json::Value.
fn mysql_row_value(row: &sqlx::mysql::MySqlRow, col: &sqlx::mysql::MySqlColumn) -> serde_json::Value {
    use sqlx::TypeInfo;
    let type_name = col.type_info().name();
    let idx = col.ordinal();
    match type_name {
        "BIGINT" | "INT" | "MEDIUMINT" | "SMALLINT" | "TINYINT" => {
            row.try_get::<i64, _>(idx)
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null)
        }
        "FLOAT" | "DOUBLE" | "DECIMAL" => {
            row.try_get::<f64, _>(idx)
                .map(|f| serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null))
                .unwrap_or(serde_json::Value::Null)
        }
        "BOOLEAN" => {
            row.try_get::<bool, _>(idx)
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null)
        }
        _ => {
            row.try_get::<String, _>(idx)
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null)
        }
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
