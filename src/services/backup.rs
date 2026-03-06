//! Data backup & restore service
//!
//! Provides backup (JSON+ZIP), restore (ZIP upload), and Markdown export.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use tracing::{info, warn};
use zip::write::SimpleFileOptions;

use crate::config::DatabaseDriver;
use crate::db::DynDatabasePool;

/// Tables to back up, in dependency order (parents first)
const BACKUP_TABLES: &[&str] = &[
    "users",
    "categories",
    "tags",
    "articles",
    "article_tags",
    "comments",
    "likes",
    "pages",
    "nav_items",
    "settings",
    "plugin_states",
    "plugin_data",
];

/// Tables to restore in order (respecting FK constraints)
const RESTORE_ORDER: &[&str] = &[
    "settings",
    "users",
    "categories",
    "tags",
    "articles",
    "article_tags",
    "comments",
    "likes",
    "pages",
    "nav_items",
    "plugin_states",
    "plugin_data",
];

/// Backup manifest
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupManifest {
    pub version: String,
    pub created_at: String,
    pub db_driver: String,
    pub tables: HashMap<String, usize>,
}

/// Create a full backup as ZIP bytes
pub async fn create_backup(pool: &DynDatabasePool, upload_dir: &Path) -> Result<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let driver_name = match pool.driver() {
        DatabaseDriver::Sqlite => "sqlite",
        DatabaseDriver::Mysql => "mysql",
    };

    let mut table_counts = HashMap::new();

    for table in BACKUP_TABLES {
        let rows = export_table(pool, table).await?;
        table_counts.insert(table.to_string(), rows.len());
        let json = serde_json::to_string_pretty(&rows)?;
        zip.start_file(format!("data/{}.json", table), options)?;
        zip.write_all(json.as_bytes())?;
        info!(table = table, rows = rows.len(), "backed up table");
    }

    // Write manifest
    let manifest = BackupManifest {
        version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: Utc::now().to_rfc3339(),
        db_driver: driver_name.to_string(),
        tables: table_counts,
    };
    zip.start_file("manifest.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    // Add uploads directory
    if upload_dir.exists() && upload_dir.is_dir() {
        add_dir_to_zip(&mut zip, upload_dir, "uploads", options)?;
    }

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

/// Restore from ZIP bytes
pub async fn restore_backup(pool: &DynDatabasePool, upload_dir: &Path, zip_data: &[u8]) -> Result<BackupManifest> {
    let reader = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(reader)?;

    // Read manifest
    let manifest: BackupManifest = {
        let mut f = archive.by_name("manifest.json").context("Missing manifest.json in backup")?;
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;
        serde_json::from_str(&buf)?
    };
    info!(version = %manifest.version, driver = %manifest.db_driver, "restoring backup");

    // Pre-read all table data from ZIP (archive borrows can't overlap with async)
    let mut table_data: Vec<(&str, Vec<serde_json::Value>)> = Vec::new();
    for table in RESTORE_ORDER {
        let file_path = format!("data/{}.json", table);
        match archive.by_name(&file_path) {
            Ok(mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf)?;
                let rows: Vec<serde_json::Value> = serde_json::from_str(&buf)?;
                if !rows.is_empty() {
                    table_data.push((table, rows));
                }
            }
            Err(_) => {
                warn!(table = table, "table not found in backup, skipping");
            }
        }
    }

    // Execute restore on a SINGLE connection (PRAGMA is per-connection in SQLite)
    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let p = pool.as_sqlite_or_err()?;
            let mut conn = p.acquire().await?;

            // Disable FK constraints on THIS connection
            sqlx::query("PRAGMA foreign_keys = OFF").execute(&mut *conn).await?;

            // Clear tables in reverse order (children first)
            for table in RESTORE_ORDER.iter().rev() {
                let sql = format!("DELETE FROM {}", table);
                if let Err(e) = sqlx::query(&sql).execute(&mut *conn).await {
                    warn!(table = table, error = %e, "failed to clear table, continuing");
                }
            }

            // Insert rows in dependency order
            for (table, rows) in &table_data {
                for row in rows {
                    if let Some(obj) = row.as_object() {
                        if obj.is_empty() { continue; }
                        let columns: Vec<&String> = obj.keys().collect();
                        let col_names = columns.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", ");
                        let ph = (1..=columns.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(", ");
                        let sql = format!("INSERT OR REPLACE INTO {} ({}) VALUES ({})", table, col_names, ph);
                        let mut q = sqlx::query(&sql);
                        for col in &columns {
                            q = bind_json_value_sqlite(q, obj.get(*col).unwrap_or(&serde_json::Value::Null));
                        }
                        if let Err(e) = q.execute(&mut *conn).await {
                            warn!(table = table, error = %e, "failed to insert row, continuing");
                        }
                    }
                }
                info!(table = table, rows = rows.len(), "restored table");
            }

            // Re-enable FK constraints
            sqlx::query("PRAGMA foreign_keys = ON").execute(&mut *conn).await?;
        }
        DatabaseDriver::Mysql => {
            let p = pool.as_mysql_or_err()?;
            let mut conn = p.acquire().await?;

            sqlx::query("SET FOREIGN_KEY_CHECKS = 0").execute(&mut *conn).await?;

            for table in RESTORE_ORDER.iter().rev() {
                let sql = format!("DELETE FROM {}", table);
                if let Err(e) = sqlx::query(&sql).execute(&mut *conn).await {
                    warn!(table = table, error = %e, "failed to clear table, continuing");
                }
            }

            for (table, rows) in &table_data {
                for row in rows {
                    if let Some(obj) = row.as_object() {
                        if obj.is_empty() { continue; }
                        let columns: Vec<&String> = obj.keys().collect();
                        let col_names = columns.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", ");
                        let ph = columns.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                        let sql = format!("REPLACE INTO {} ({}) VALUES ({})", table, col_names, ph);
                        let mut q = sqlx::query(&sql);
                        for col in &columns {
                            q = bind_json_value_mysql(q, obj.get(*col).unwrap_or(&serde_json::Value::Null));
                        }
                        if let Err(e) = q.execute(&mut *conn).await {
                            warn!(table = table, error = %e, "failed to insert row, continuing");
                        }
                    }
                }
                info!(table = table, rows = rows.len(), "restored table");
            }

            sqlx::query("SET FOREIGN_KEY_CHECKS = 1").execute(&mut *conn).await?;
        }
    }

    // Restore uploads directory
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name.starts_with("uploads/") && !file.is_dir() {
            let rel = name.strip_prefix("uploads/").unwrap_or(&name);
            let dest = upload_dir.join(rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut file, &mut out)?;
        }
    }

    Ok(manifest)
}

/// Export all articles as Markdown ZIP
pub async fn export_markdown(pool: &DynDatabasePool) -> Result<Vec<u8>> {
    let articles = export_table(pool, "articles").await?;
    let categories = export_table(pool, "categories").await?;

    // Build category id->name map
    let cat_map: HashMap<i64, String> = categories.iter()
        .filter_map(|c| {
            let id = c.get("id")?.as_i64()?;
            let name = c.get("name")?.as_str()?.to_string();
            Some((id, name))
        })
        .collect();

    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for article in &articles {
        let title = article.get("title").and_then(|v| v.as_str()).unwrap_or("untitled");
        let slug = article.get("slug").and_then(|v| v.as_str()).unwrap_or("untitled");
        let content = article.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let status = article.get("status").and_then(|v| v.as_str()).unwrap_or("draft");
        let created = article.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
        let cat_id = article.get("category_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let category = cat_map.get(&cat_id).map(|s| s.as_str()).unwrap_or("uncategorized");

        let frontmatter = format!(
            "---\ntitle: \"{}\"\ndate: \"{}\"\nstatus: \"{}\"\ncategory: \"{}\"\n---\n\n",
            title.replace('"', "\\\""), created, status, category
        );

        zip.start_file(format!("{}.md", slug), options)?;
        zip.write_all(frontmatter.as_bytes())?;
        zip.write_all(content.as_bytes())?;
    }

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

/// Import result summary
#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Import articles from a zip/xml file
///
/// Detects format automatically:
/// - ZIP containing .md files → Markdown import (with YAML frontmatter)
/// - XML file → WordPress WXR import
pub async fn import_articles(pool: &DynDatabasePool, data: &[u8], author_id: i64) -> Result<ImportResult> {
    // Try to detect format
    if data.len() >= 4 && &data[0..4] == b"PK\x03\x04" {
        import_markdown_zip(pool, data, author_id).await
    } else if data.len() >= 5 && std::str::from_utf8(&data[..100.min(data.len())]).map_or(false, |s| s.contains("<?xml") || s.contains("<rss") || s.contains("<wp:")) {
        import_wordpress_xml(pool, data, author_id).await
    } else {
        anyhow::bail!("Unsupported file format. Expected a ZIP file (Markdown) or XML file (WordPress WXR).")
    }
}

/// Import articles from a ZIP of Markdown files with YAML frontmatter
async fn import_markdown_zip(pool: &DynDatabasePool, zip_data: &[u8], author_id: i64) -> Result<ImportResult> {
    // Phase 1: Synchronously extract all markdown files from ZIP
    let entries = {
        let reader = std::io::Cursor::new(zip_data);
        let mut archive = zip::ZipArchive::new(reader)?;
        let mut entries = Vec::new();
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            
            if !name.ends_with(".md") && !name.ends_with(".markdown") {
                continue;
            }
            
            let mut content = String::new();
            std::io::Read::read_to_string(&mut file, &mut content)?;
            entries.push((name, content));
        }
        entries
    };
    
    // Phase 2: Process entries with async DB operations
    let renderer = crate::services::MarkdownRenderer::new();
    let mut result = ImportResult { imported: 0, skipped: 0, errors: Vec::new() };
    
    for (name, content) in &entries {
        let (title, slug, body, status) = parse_markdown_frontmatter(content, name);
        
        if check_slug_exists(pool, &slug).await {
            result.skipped += 1;
            result.errors.push(format!("Skipped '{}': slug '{}' already exists", title, slug));
            continue;
        }
        
        let html = renderer.render(&body);
        
        match insert_article(pool, &slug, &title, &body, &html, author_id, &status).await {
            Ok(_) => result.imported += 1,
            Err(e) => {
                result.errors.push(format!("Failed to import '{}': {}", title, e));
            }
        }
    }
    
    Ok(result)
}

/// Import articles from WordPress WXR XML export
async fn import_wordpress_xml(pool: &DynDatabasePool, xml_data: &[u8], author_id: i64) -> Result<ImportResult> {
    let xml_str = std::str::from_utf8(xml_data).context("Invalid UTF-8 in XML file")?;
    let renderer = crate::services::MarkdownRenderer::new();
    
    let mut result = ImportResult { imported: 0, skipped: 0, errors: Vec::new() };
    
    // Simple XML parsing for WordPress WXR format
    // Look for <item> blocks with <wp:post_type>post</wp:post_type>
    for item in xml_str.split("<item>").skip(1) {
        let item_end = item.find("</item>").unwrap_or(item.len());
        let item = &item[..item_end];
        
        // Check post_type = post (not page, attachment, etc.)
        let post_type = extract_xml_tag(item, "wp:post_type").unwrap_or_default();
        if post_type != "post" {
            continue;
        }
        
        let title = extract_xml_tag(item, "title")
            .or_else(|| extract_cdata(item, "title"))
            .unwrap_or_else(|| "Untitled".to_string());
        let slug = extract_xml_tag(item, "wp:post_name")
            .unwrap_or_else(|| slugify(&title));
        let content = extract_cdata(item, "content:encoded")
            .unwrap_or_default();
        let status_str = extract_xml_tag(item, "wp:status").unwrap_or_default();
        let status = if status_str == "publish" { "published" } else { "draft" };
        
        // Check if slug already exists
        if check_slug_exists(pool, &slug).await {
            result.skipped += 1;
            result.errors.push(format!("Skipped '{}': slug '{}' already exists", title, slug));
            continue;
        }
        
        // WordPress exports HTML content; also store as markdown (best effort)
        let body = html_to_basic_markdown(&content);
        let html = renderer.render(&body);
        
        match insert_article(pool, &slug, &title, &body, &html, author_id, status).await {
            Ok(_) => result.imported += 1,
            Err(e) => {
                result.errors.push(format!("Failed to import '{}': {}", title, e));
            }
        }
    }
    
    Ok(result)
}

// ---- Import helpers ----

/// Parse YAML frontmatter from a markdown file
fn parse_markdown_frontmatter(content: &str, filename: &str) -> (String, String, String, String) {
    let default_slug = filename.trim_end_matches(".md")
        .trim_end_matches(".markdown")
        .rsplit('/')
        .next()
        .unwrap_or("untitled")
        .to_string();
    
    if !content.starts_with("---") {
        return ("Untitled".to_string(), default_slug, content.to_string(), "draft".to_string());
    }
    
    let rest = &content[3..];
    if let Some(end) = rest.find("\n---") {
        let frontmatter = &rest[..end];
        let body = rest[end + 4..].trim_start().to_string();
        
        let mut title = None;
        let mut slug = None;
        let mut status = None;
        
        for line in frontmatter.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("title:") {
                title = Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
            } else if let Some(val) = line.strip_prefix("slug:") {
                slug = Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
            } else if let Some(val) = line.strip_prefix("status:") {
                status = Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
        
        (
            title.unwrap_or_else(|| "Untitled".to_string()),
            slug.unwrap_or(default_slug),
            body,
            status.unwrap_or_else(|| "draft".to_string()),
        )
    } else {
        ("Untitled".to_string(), default_slug, content.to_string(), "draft".to_string())
    }
}

/// Check if an article slug already exists
async fn check_slug_exists(pool: &DynDatabasePool, slug: &str) -> bool {
    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let Ok(p) = pool.as_sqlite_or_err() else { return false };
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM articles WHERE slug = ?")
                .bind(slug)
                .fetch_one(p)
                .await
                .unwrap_or(0) > 0
        }
        DatabaseDriver::Mysql => {
            let Ok(p) = pool.as_mysql_or_err() else { return false };
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM articles WHERE slug = ?")
                .bind(slug)
                .fetch_one(p)
                .await
                .unwrap_or(0) > 0
        }
    }
}

/// Insert a single article into the database
async fn insert_article(pool: &DynDatabasePool, slug: &str, title: &str, content: &str, content_html: &str, author_id: i64, status: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let sql = "INSERT INTO articles (slug, title, content, content_html, author_id, category_id, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?)";
    match pool.driver() {
        DatabaseDriver::Sqlite => {
            let p = pool.as_sqlite_or_err()?;
            sqlx::query(sql)
                .bind(slug).bind(title).bind(content).bind(content_html)
                .bind(author_id).bind(status).bind(&now).bind(&now)
                .execute(p).await?;
        }
        DatabaseDriver::Mysql => {
            let p = pool.as_mysql_or_err()?;
            sqlx::query(sql)
                .bind(slug).bind(title).bind(content).bind(content_html)
                .bind(author_id).bind(status).bind(&now).bind(&now)
                .execute(p).await?;
        }
    }
    Ok(())
}

/// Extract content from an XML tag
fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

/// Extract CDATA content from an XML tag
fn extract_cdata(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    let inner = xml[start..end].trim();
    // Strip CDATA wrapper if present
    if let Some(stripped) = inner.strip_prefix("<![CDATA[") {
        Some(stripped.strip_suffix("]]>").unwrap_or(stripped).to_string())
    } else {
        Some(inner.to_string())
    }
}

/// Basic slugify function
fn slugify(title: &str) -> String {
    title.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Convert basic HTML to markdown (best effort for WordPress content)
fn html_to_basic_markdown(html: &str) -> String {
    html.replace("<p>", "").replace("</p>", "\n\n")
        .replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n")
        .replace("<strong>", "**").replace("</strong>", "**")
        .replace("<b>", "**").replace("</b>", "**")
        .replace("<em>", "*").replace("</em>", "*")
        .replace("<i>", "*").replace("</i>", "*")
        .replace("<h1>", "# ").replace("</h1>", "\n")
        .replace("<h2>", "## ").replace("</h2>", "\n")
        .replace("<h3>", "### ").replace("</h3>", "\n")
        .replace("<h4>", "#### ").replace("</h4>", "\n")
        .replace("<blockquote>", "> ").replace("</blockquote>", "\n")
        .replace("<code>", "`").replace("</code>", "`")
        .replace("<pre>", "```\n").replace("</pre>", "\n```\n")
        .replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"")
        .trim().to_string()
}

// ---- Internal helpers ----

/// Export a table as a Vec of JSON objects
async fn export_table(pool: &DynDatabasePool, table: &str) -> Result<Vec<serde_json::Value>> {
    let query = format!("SELECT * FROM {}", table);
    let rows = match pool.driver() {
        DatabaseDriver::Sqlite => {
            let p = pool.as_sqlite_or_err()?;
            let rows = sqlx::query(&query).fetch_all(p).await?;
            rows.iter().map(|r| sqlite_row_to_json(r)).collect()
        }
        DatabaseDriver::Mysql => {
            let p = pool.as_mysql_or_err()?;
            let rows = sqlx::query(&query).fetch_all(p).await?;
            rows.iter().map(|r| mysql_row_to_json(r)).collect()
        }
    };
    Ok(rows)
}

/// Convert a SQLite row to JSON
fn sqlite_row_to_json(row: &sqlx::sqlite::SqliteRow) -> serde_json::Value {
    use sqlx::{Row, Column};
    let columns = row.columns();
    let mut map = serde_json::Map::new();
    for col in columns {
        let name = col.name();
        // Try different types
        if let Ok(v) = row.try_get::<i64, _>(name) {
            map.insert(name.to_string(), serde_json::Value::Number(v.into()));
        } else if let Ok(v) = row.try_get::<f64, _>(name) {
            if let Some(n) = serde_json::Number::from_f64(v) {
                map.insert(name.to_string(), serde_json::Value::Number(n));
            }
        } else if let Ok(v) = row.try_get::<String, _>(name) {
            map.insert(name.to_string(), serde_json::Value::String(v));
        } else if let Ok(v) = row.try_get::<bool, _>(name) {
            map.insert(name.to_string(), serde_json::Value::Bool(v));
        } else {
            map.insert(name.to_string(), serde_json::Value::Null);
        }
    }
    serde_json::Value::Object(map)
}

/// Convert a MySQL row to JSON
fn mysql_row_to_json(row: &sqlx::mysql::MySqlRow) -> serde_json::Value {
    use sqlx::{Row, Column};
    let columns = row.columns();
    let mut map = serde_json::Map::new();
    for col in columns {
        let name = col.name();
        if let Ok(v) = row.try_get::<i64, _>(name) {
            map.insert(name.to_string(), serde_json::Value::Number(v.into()));
        } else if let Ok(v) = row.try_get::<f64, _>(name) {
            if let Some(n) = serde_json::Number::from_f64(v) {
                map.insert(name.to_string(), serde_json::Value::Number(n));
            }
        } else if let Ok(v) = row.try_get::<String, _>(name) {
            map.insert(name.to_string(), serde_json::Value::String(v));
        } else if let Ok(v) = row.try_get::<bool, _>(name) {
            map.insert(name.to_string(), serde_json::Value::Bool(v));
        } else {
            map.insert(name.to_string(), serde_json::Value::Null);
        }
    }
    serde_json::Value::Object(map)
}

/// Bind a JSON value to a SQLite query
fn bind_json_value_sqlite<'q>(
    q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    val: &'q serde_json::Value,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    match val {
        serde_json::Value::Null => q.bind(None::<String>),
        serde_json::Value::Bool(b) => q.bind(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                q.bind(i)
            } else if let Some(f) = n.as_f64() {
                q.bind(f)
            } else {
                q.bind(n.to_string())
            }
        }
        serde_json::Value::String(s) => q.bind(s.as_str()),
        other => q.bind(other.to_string()),
    }
}

/// Bind a JSON value to a MySQL query
fn bind_json_value_mysql<'q>(
    q: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    val: &'q serde_json::Value,
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match val {
        serde_json::Value::Null => q.bind(None::<String>),
        serde_json::Value::Bool(b) => q.bind(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                q.bind(i)
            } else if let Some(f) = n.as_f64() {
                q.bind(f)
            } else {
                q.bind(n.to_string())
            }
        }
        serde_json::Value::String(s) => q.bind(s.as_str()),
        other => q.bind(other.to_string()),
    }
}

/// Recursively add a directory to a ZIP archive
fn add_dir_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &Path,
    prefix: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = format!("{}/{}", prefix, entry.file_name().to_string_lossy());
        if path.is_dir() {
            add_dir_to_zip(zip, &path, &name, options)?;
        } else {
            zip.start_file(&name, options)?;
            let mut f = std::fs::File::open(&path)?;
            std::io::copy(&mut f, zip)?;
        }
    }
    Ok(())
}
