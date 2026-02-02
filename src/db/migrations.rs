//! Database migrations module
//!
//! This module provides code-based database migrations for the Noteva blog system.
//! All migrations are embedded directly in Rust code as SQL strings, supporting
//! both SQLite and MySQL databases for single-binary deployment.
//!
//! # Usage
//!
//! ```ignore
//! use noteva::db::{create_pool, migrations};
//!
//! let pool = create_pool(&config).await?;
//! migrations::run_migrations(&pool).await?;
//! ```
//!
//! # Architecture
//!
//! Each migration is defined as a `Migration` struct containing:
//! - `version`: Unique version number for ordering
//! - `name`: Human-readable migration name
//! - `up_sqlite`: SQL for SQLite database
//! - `up_mysql`: SQL for MySQL database
//!
//! Requirements: 1.1, 2.1, 3.1, 4.2

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{MySqlPool, Row, SqlitePool};

use super::DynDatabasePool;
use crate::config::DatabaseDriver;

/// A database migration with SQL for both SQLite and MySQL
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration version number (must be unique and sequential)
    pub version: i32,
    /// Human-readable migration name
    pub name: &'static str,
    /// SQL statements for SQLite
    pub up_sqlite: &'static str,
    /// SQL statements for MySQL
    pub up_mysql: &'static str,
}

/// Migration record stored in the database
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    /// Migration version number
    pub version: i64,
    /// Migration name/description
    pub name: String,
    /// When the migration was applied
    pub applied_at: DateTime<Utc>,
}

/// All migrations for the Noteva blog system.
/// These are embedded in the binary for single-binary deployment.
pub const MIGRATIONS: &[Migration] = &[
    // Migration 1: Create users table
    // Satisfies requirement 4.2: User registration and account management
    Migration {
        version: 1,
        name: "create_users",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username VARCHAR(50) NOT NULL UNIQUE,
                email VARCHAR(255) NOT NULL UNIQUE,
                password_hash VARCHAR(255) NOT NULL,
                role VARCHAR(20) NOT NULL DEFAULT 'author',
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
            CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS users (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                username VARCHAR(50) NOT NULL UNIQUE,
                email VARCHAR(255) NOT NULL UNIQUE,
                password_hash VARCHAR(255) NOT NULL,
                role VARCHAR(20) NOT NULL DEFAULT 'author',
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
            );
            CREATE INDEX idx_users_username ON users(username);
            CREATE INDEX idx_users_email ON users(email);
        "#,
    },
    // Migration 2: Create sessions table
    // Satisfies requirement 4.3, 4.4, 4.7: Session management and authentication
    Migration {
        version: 2,
        name: "create_sessions",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id VARCHAR(64) PRIMARY KEY,
                user_id INTEGER NOT NULL,
                expires_at TIMESTAMP NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id VARCHAR(64) PRIMARY KEY,
                user_id BIGINT NOT NULL,
                expires_at TIMESTAMP NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
        "#,
    },
    // Migration 3: Create categories table
    // Satisfies requirement 2.1: Category creation with parent support
    Migration {
        version: 3,
        name: "create_categories",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                slug VARCHAR(100) NOT NULL UNIQUE,
                name VARCHAR(100) NOT NULL,
                description TEXT,
                parent_id INTEGER,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (parent_id) REFERENCES categories(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug);
            CREATE INDEX IF NOT EXISTS idx_categories_parent_id ON categories(parent_id);
            INSERT OR IGNORE INTO categories (slug, name, description, sort_order) 
            VALUES ('uncategorized', 'Uncategorized', 'Default category for uncategorized articles', 0);
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS categories (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                slug VARCHAR(100) NOT NULL UNIQUE,
                name VARCHAR(100) NOT NULL,
                description TEXT,
                parent_id BIGINT,
                sort_order INT NOT NULL DEFAULT 0,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (parent_id) REFERENCES categories(id) ON DELETE SET NULL
            );
            CREATE INDEX idx_categories_slug ON categories(slug);
            CREATE INDEX idx_categories_parent_id ON categories(parent_id);
            INSERT IGNORE INTO categories (slug, name, description, sort_order) 
            VALUES ('uncategorized', 'Uncategorized', 'Default category for uncategorized articles', 0);
        "#,
    },
    // Migration 4: Create tags table
    // Satisfies requirement 3.1: Tag creation and management
    Migration {
        version: 4,
        name: "create_tags",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                slug VARCHAR(100) NOT NULL UNIQUE,
                name VARCHAR(100) NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_tags_slug ON tags(slug);
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS tags (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                slug VARCHAR(100) NOT NULL UNIQUE,
                name VARCHAR(100) NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX idx_tags_slug ON tags(slug);
        "#,
    },
    // Migration 5: Create articles table
    // Satisfies requirement 1.1: Article creation with unique identifier
    Migration {
        version: 5,
        name: "create_articles",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS articles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                slug VARCHAR(255) NOT NULL UNIQUE,
                title VARCHAR(255) NOT NULL,
                content TEXT NOT NULL,
                content_html TEXT NOT NULL,
                author_id INTEGER NOT NULL,
                category_id INTEGER NOT NULL DEFAULT 1,
                status VARCHAR(20) NOT NULL DEFAULT 'draft',
                published_at TIMESTAMP,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE,
                FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE SET DEFAULT
            );
            CREATE INDEX IF NOT EXISTS idx_articles_slug ON articles(slug);
            CREATE INDEX IF NOT EXISTS idx_articles_author_id ON articles(author_id);
            CREATE INDEX IF NOT EXISTS idx_articles_category_id ON articles(category_id);
            CREATE INDEX IF NOT EXISTS idx_articles_status ON articles(status);
            CREATE INDEX IF NOT EXISTS idx_articles_published_at ON articles(published_at);
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS articles (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                slug VARCHAR(255) NOT NULL UNIQUE,
                title VARCHAR(255) NOT NULL,
                content TEXT NOT NULL,
                content_html TEXT NOT NULL,
                author_id BIGINT NOT NULL,
                category_id BIGINT NOT NULL DEFAULT 1,
                status VARCHAR(20) NOT NULL DEFAULT 'draft',
                published_at TIMESTAMP NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE,
                FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE SET DEFAULT
            );
            CREATE INDEX idx_articles_slug ON articles(slug);
            CREATE INDEX idx_articles_author_id ON articles(author_id);
            CREATE INDEX idx_articles_category_id ON articles(category_id);
            CREATE INDEX idx_articles_status ON articles(status);
            CREATE INDEX idx_articles_published_at ON articles(published_at);
        "#,
    },
    // Migration 6: Create article_tags junction table
    // Satisfies requirement 3.1: Tag-article association
    Migration {
        version: 6,
        name: "create_article_tags",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS article_tags (
                article_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (article_id, tag_id),
                FOREIGN KEY (article_id) REFERENCES articles(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_article_tags_article_id ON article_tags(article_id);
            CREATE INDEX IF NOT EXISTS idx_article_tags_tag_id ON article_tags(tag_id);
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS article_tags (
                article_id BIGINT NOT NULL,
                tag_id BIGINT NOT NULL,
                PRIMARY KEY (article_id, tag_id),
                FOREIGN KEY (article_id) REFERENCES articles(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_article_tags_article_id ON article_tags(article_id);
            CREATE INDEX idx_article_tags_tag_id ON article_tags(tag_id);
        "#,
    },
    // Migration 7: Create settings table
    // Satisfies requirement 5.3: System configuration storage
    Migration {
        version: 7,
        name: "create_settings",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS settings (
                key VARCHAR(100) PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            INSERT OR IGNORE INTO settings (key, value) VALUES ('site_name', 'Noteva');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('site_description', 'A lightweight blog powered by Noteva');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('posts_per_page', '10');
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS settings (
                `key` VARCHAR(100) PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
            );
            INSERT IGNORE INTO settings (`key`, value) VALUES ('site_name', 'Noteva');
            INSERT IGNORE INTO settings (`key`, value) VALUES ('site_description', 'A lightweight blog powered by Noteva');
            INSERT IGNORE INTO settings (`key`, value) VALUES ('posts_per_page', '10');
        "#,
    },
    // Migration 8: Create comments table
    // Supports both guest comments (with nickname/email) and logged-in user comments
    Migration {
        version: 8,
        name: "create_comments",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS comments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                article_id INTEGER NOT NULL,
                user_id INTEGER,
                parent_id INTEGER,
                nickname VARCHAR(100),
                email VARCHAR(255),
                content TEXT NOT NULL,
                status VARCHAR(20) NOT NULL DEFAULT 'approved',
                ip_address VARCHAR(45),
                user_agent TEXT,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (article_id) REFERENCES articles(id) ON DELETE CASCADE,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL,
                FOREIGN KEY (parent_id) REFERENCES comments(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_comments_article_id ON comments(article_id);
            CREATE INDEX IF NOT EXISTS idx_comments_user_id ON comments(user_id);
            CREATE INDEX IF NOT EXISTS idx_comments_parent_id ON comments(parent_id);
            CREATE INDEX IF NOT EXISTS idx_comments_status ON comments(status);
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS comments (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                article_id BIGINT NOT NULL,
                user_id BIGINT,
                parent_id BIGINT,
                nickname VARCHAR(100),
                email VARCHAR(255),
                content TEXT NOT NULL,
                status VARCHAR(20) NOT NULL DEFAULT 'approved',
                ip_address VARCHAR(45),
                user_agent TEXT,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                FOREIGN KEY (article_id) REFERENCES articles(id) ON DELETE CASCADE,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL,
                FOREIGN KEY (parent_id) REFERENCES comments(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_comments_article_id ON comments(article_id);
            CREATE INDEX idx_comments_user_id ON comments(user_id);
            CREATE INDEX idx_comments_parent_id ON comments(parent_id);
            CREATE INDEX idx_comments_status ON comments(status);
        "#,
    },
    // Migration 9: Create likes table for articles and comments
    // Uses fingerprint for anonymous like tracking (hash of IP + User-Agent)
    Migration {
        version: 9,
        name: "create_likes",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS likes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                target_type VARCHAR(20) NOT NULL,
                target_id INTEGER NOT NULL,
                user_id INTEGER,
                fingerprint VARCHAR(64),
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                UNIQUE(target_type, target_id, user_id),
                UNIQUE(target_type, target_id, fingerprint)
            );
            CREATE INDEX IF NOT EXISTS idx_likes_target ON likes(target_type, target_id);
            CREATE INDEX IF NOT EXISTS idx_likes_user_id ON likes(user_id);
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS likes (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                target_type VARCHAR(20) NOT NULL,
                target_id BIGINT NOT NULL,
                user_id BIGINT,
                fingerprint VARCHAR(64),
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                UNIQUE KEY uk_likes_user (target_type, target_id, user_id),
                UNIQUE KEY uk_likes_fingerprint (target_type, target_id, fingerprint)
            );
            CREATE INDEX idx_likes_target ON likes(target_type, target_id);
            CREATE INDEX idx_likes_user_id ON likes(user_id);
        "#,
    },
    // Migration 10: Add view_count and like_count to articles
    Migration {
        version: 10,
        name: "add_article_stats",
        up_sqlite: r#"
            ALTER TABLE articles ADD COLUMN view_count INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE articles ADD COLUMN like_count INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE articles ADD COLUMN comment_count INTEGER NOT NULL DEFAULT 0;
        "#,
        up_mysql: r#"
            ALTER TABLE articles ADD COLUMN view_count INT NOT NULL DEFAULT 0;
            ALTER TABLE articles ADD COLUMN like_count INT NOT NULL DEFAULT 0;
            ALTER TABLE articles ADD COLUMN comment_count INT NOT NULL DEFAULT 0;
        "#,
    },
    // Migration 11: Add comment settings
    Migration {
        version: 11,
        name: "add_comment_settings",
        up_sqlite: r#"
            INSERT OR IGNORE INTO settings (key, value) VALUES ('require_login_to_comment', 'false');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('comment_moderation', 'false');
        "#,
        up_mysql: r#"
            INSERT IGNORE INTO settings (`key`, value) VALUES ('require_login_to_comment', 'false');
            INSERT IGNORE INTO settings (`key`, value) VALUES ('comment_moderation', 'false');
        "#,
    },
    // Migration 12: Add thumbnail and pinned fields to articles
    Migration {
        version: 12,
        name: "add_article_thumbnail_and_pinned",
        up_sqlite: r#"
            ALTER TABLE articles ADD COLUMN thumbnail VARCHAR(500);
            ALTER TABLE articles ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE articles ADD COLUMN pin_order INTEGER NOT NULL DEFAULT 0;
        "#,
        up_mysql: r#"
            ALTER TABLE articles ADD COLUMN thumbnail VARCHAR(500);
            ALTER TABLE articles ADD COLUMN is_pinned TINYINT NOT NULL DEFAULT 0;
            ALTER TABLE articles ADD COLUMN pin_order INT NOT NULL DEFAULT 0;
        "#,
    },
    // Migration 13: Create pages table for custom pages
    Migration {
        version: 13,
        name: "create_pages",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS pages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                slug VARCHAR(255) NOT NULL UNIQUE,
                title VARCHAR(255) NOT NULL,
                content TEXT NOT NULL,
                content_html TEXT NOT NULL,
                status VARCHAR(20) NOT NULL DEFAULT 'draft',
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_pages_slug ON pages(slug);
            CREATE INDEX IF NOT EXISTS idx_pages_status ON pages(status);
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS pages (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                slug VARCHAR(255) NOT NULL UNIQUE,
                title VARCHAR(255) NOT NULL,
                content TEXT NOT NULL,
                content_html TEXT NOT NULL,
                status VARCHAR(20) NOT NULL DEFAULT 'draft',
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
            );
            CREATE INDEX idx_pages_slug ON pages(slug);
            CREATE INDEX idx_pages_status ON pages(status);
        "#,
    },
    // Migration 14: Create nav_items table for custom navigation
    Migration {
        version: 14,
        name: "create_nav_items",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS nav_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                parent_id INTEGER,
                title VARCHAR(100) NOT NULL,
                nav_type VARCHAR(20) NOT NULL DEFAULT 'builtin',
                target VARCHAR(500) NOT NULL,
                open_new_tab INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0,
                visible INTEGER NOT NULL DEFAULT 1,
                FOREIGN KEY (parent_id) REFERENCES nav_items(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_nav_items_parent_id ON nav_items(parent_id);
            CREATE INDEX IF NOT EXISTS idx_nav_items_sort_order ON nav_items(sort_order);
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS nav_items (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                parent_id BIGINT,
                title VARCHAR(100) NOT NULL,
                nav_type VARCHAR(20) NOT NULL DEFAULT 'builtin',
                target VARCHAR(500) NOT NULL,
                open_new_tab TINYINT NOT NULL DEFAULT 0,
                sort_order INT NOT NULL DEFAULT 0,
                visible TINYINT NOT NULL DEFAULT 1,
                FOREIGN KEY (parent_id) REFERENCES nav_items(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_nav_items_parent_id ON nav_items(parent_id);
            CREATE INDEX idx_nav_items_sort_order ON nav_items(sort_order);
        "#,
    },
    // Migration 15: Add user status field for ban functionality
    Migration {
        version: 15,
        name: "add_user_status",
        up_sqlite: r#"
            ALTER TABLE users ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'active';
        "#,
        up_mysql: r#"
            ALTER TABLE users ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'active';
        "#,
    },
    // Migration 16: Add display_name and avatar fields to users
    Migration {
        version: 16,
        name: "add_user_profile_fields",
        up_sqlite: r#"
            ALTER TABLE users ADD COLUMN display_name VARCHAR(100);
            ALTER TABLE users ADD COLUMN avatar VARCHAR(500);
        "#,
        up_mysql: r#"
            ALTER TABLE users ADD COLUMN display_name VARCHAR(100);
            ALTER TABLE users ADD COLUMN avatar VARCHAR(500);
        "#,
    },
    // Migration 17: Create email_verifications table
    Migration {
        version: 17,
        name: "create_email_verifications_table",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS email_verifications (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email VARCHAR(255) NOT NULL,
                code VARCHAR(10) NOT NULL,
                expires_at DATETIME NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_email_verifications_email ON email_verifications(email);
            CREATE INDEX IF NOT EXISTS idx_email_verifications_expires ON email_verifications(expires_at);
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS email_verifications (
                id BIGINT PRIMARY KEY AUTO_INCREMENT,
                email VARCHAR(255) NOT NULL,
                code VARCHAR(10) NOT NULL,
                expires_at DATETIME NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX idx_email_verifications_email ON email_verifications(email);
            CREATE INDEX idx_email_verifications_expires ON email_verifications(expires_at);
        "#,
    },
    // Migration 18: Create plugin_states table for plugin configuration storage
    Migration {
        version: 18,
        name: "create_plugin_states",
        up_sqlite: r#"
            CREATE TABLE IF NOT EXISTS plugin_states (
                plugin_id VARCHAR(100) PRIMARY KEY,
                enabled INTEGER NOT NULL DEFAULT 0,
                settings TEXT NOT NULL DEFAULT '{}',
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
        "#,
        up_mysql: r#"
            CREATE TABLE IF NOT EXISTS plugin_states (
                plugin_id VARCHAR(100) PRIMARY KEY,
                enabled BOOLEAN NOT NULL DEFAULT FALSE,
                settings JSON NOT NULL,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
            );
        "#,
    },
];

/// Run all pending migrations
///
/// This function:
/// 1. Creates the migrations tracking table if it doesn't exist
/// 2. Checks which migrations have already been applied
/// 3. Runs any pending migrations in order
///
/// # Arguments
///
/// * `pool` - Database connection pool
///
/// # Returns
///
/// Number of migrations applied
///
/// # Errors
///
/// Returns an error if any migration fails to apply
pub async fn run_migrations(pool: &DynDatabasePool) -> Result<usize> {
    // Create migrations table
    create_migrations_table(pool).await?;

    // Get applied migrations
    let applied = get_applied_migrations(pool).await?;
    let applied_versions: Vec<i32> = applied.iter().map(|m| m.version as i32).collect();

    let mut count = 0;

    for migration in MIGRATIONS {
        if !applied_versions.contains(&migration.version) {
            tracing::info!(
                "Applying migration {}: {}",
                migration.version,
                migration.name
            );
            apply_migration(pool, migration)
                .await
                .with_context(|| format!("Failed to apply migration: {}", migration.name))?;
            count += 1;
        }
    }

    if count > 0 {
        tracing::info!("Applied {} migration(s)", count);
    } else {
        tracing::debug!("No pending migrations");
    }

    Ok(count)
}

/// Create the migrations tracking table if it doesn't exist
async fn create_migrations_table(pool: &DynDatabasePool) -> Result<()> {
    let sql = match pool.driver() {
        DatabaseDriver::Sqlite => {
            r#"
            CREATE TABLE IF NOT EXISTS _migrations (
                version INTEGER PRIMARY KEY,
                name VARCHAR(255) NOT NULL UNIQUE,
                applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        }
        DatabaseDriver::Mysql => {
            r#"
            CREATE TABLE IF NOT EXISTS _migrations (
                version INT PRIMARY KEY,
                name VARCHAR(255) NOT NULL UNIQUE,
                applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        }
    };

    pool.execute(sql).await?;
    Ok(())
}

/// Get list of already applied migrations
async fn get_applied_migrations(pool: &DynDatabasePool) -> Result<Vec<MigrationRecord>> {
    match pool.driver() {
        DatabaseDriver::Sqlite => get_applied_migrations_sqlite(pool.as_sqlite().unwrap()).await,
        DatabaseDriver::Mysql => get_applied_migrations_mysql(pool.as_mysql().unwrap()).await,
    }
}

async fn get_applied_migrations_sqlite(pool: &SqlitePool) -> Result<Vec<MigrationRecord>> {
    let rows =
        sqlx::query("SELECT version, name, applied_at FROM _migrations ORDER BY version")
            .fetch_all(pool)
            .await?;

    let mut records = Vec::new();
    for row in rows {
        records.push(MigrationRecord {
            version: row.get("version"),
            name: row.get("name"),
            applied_at: row.get("applied_at"),
        });
    }

    Ok(records)
}

async fn get_applied_migrations_mysql(pool: &MySqlPool) -> Result<Vec<MigrationRecord>> {
    let rows =
        sqlx::query("SELECT version, name, applied_at FROM _migrations ORDER BY version")
            .fetch_all(pool)
            .await?;

    let mut records = Vec::new();
    for row in rows {
        records.push(MigrationRecord {
            version: row.get("version"),
            name: row.get("name"),
            applied_at: row.get("applied_at"),
        });
    }

    Ok(records)
}

/// Apply a single migration
async fn apply_migration(pool: &DynDatabasePool, migration: &Migration) -> Result<()> {
    match pool.driver() {
        DatabaseDriver::Sqlite => {
            apply_migration_sqlite(pool.as_sqlite().unwrap(), migration).await
        }
        DatabaseDriver::Mysql => {
            apply_migration_mysql(pool.as_mysql().unwrap(), migration).await
        }
    }
}

async fn apply_migration_sqlite(pool: &SqlitePool, migration: &Migration) -> Result<()> {
    // Execute migration SQL (may contain multiple statements)
    for statement in split_sql_statements(migration.up_sqlite) {
        let statement = statement.trim();
        if !statement.is_empty() {
            sqlx::query(statement)
                .execute(pool)
                .await
                .with_context(|| format!("Failed to execute: {}", truncate_sql(statement)))?;
        }
    }

    // Record the migration
    sqlx::query("INSERT INTO _migrations (version, name) VALUES (?, ?)")
        .bind(migration.version)
        .bind(migration.name)
        .execute(pool)
        .await?;

    Ok(())
}

async fn apply_migration_mysql(pool: &MySqlPool, migration: &Migration) -> Result<()> {
    // Execute migration SQL (may contain multiple statements)
    for statement in split_sql_statements(migration.up_mysql) {
        let statement = statement.trim();
        if !statement.is_empty() {
            sqlx::query(statement)
                .execute(pool)
                .await
                .with_context(|| format!("Failed to execute: {}", truncate_sql(statement)))?;
        }
    }

    // Record the migration
    sqlx::query("INSERT INTO _migrations (version, name) VALUES (?, ?)")
        .bind(migration.version)
        .bind(migration.name)
        .execute(pool)
        .await?;

    Ok(())
}

/// Truncate SQL for error messages
fn truncate_sql(sql: &str) -> String {
    if sql.len() > 100 {
        format!("{}...", &sql[..100])
    } else {
        sql.to_string()
    }
}

/// Split SQL into individual statements, handling comments properly
fn split_sql_statements(sql: &str) -> Vec<&str> {
    let mut statements = Vec::new();
    let mut current_start = 0;
    let mut in_statement = false;

    for (i, c) in sql.char_indices() {
        match c {
            ';' => {
                if in_statement {
                    let stmt = sql[current_start..i].trim();
                    if !stmt.is_empty() && !is_comment_only(stmt) {
                        statements.push(stmt);
                    }
                    in_statement = false;
                }
                current_start = i + 1;
            }
            _ if !c.is_whitespace() && !in_statement => {
                current_start = i;
                in_statement = true;
            }
            _ => {}
        }
    }

    // Handle last statement without trailing semicolon
    if in_statement {
        let stmt = sql[current_start..].trim();
        if !stmt.is_empty() && !is_comment_only(stmt) {
            statements.push(stmt);
        }
    }

    statements
}

/// Check if a string contains only SQL comments
fn is_comment_only(s: &str) -> bool {
    for line in s.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("--") {
            return false;
        }
    }
    true
}

/// Check if migrations are up to date
pub async fn is_up_to_date(pool: &DynDatabasePool) -> Result<bool> {
    // Try to create migrations table (in case it doesn't exist)
    let _ = create_migrations_table(pool).await;

    let applied = get_applied_migrations(pool).await?;
    Ok(applied.len() == MIGRATIONS.len())
}

/// Get pending migrations count
pub async fn pending_count(pool: &DynDatabasePool) -> Result<usize> {
    // Try to create migrations table (in case it doesn't exist)
    let _ = create_migrations_table(pool).await;

    let applied = get_applied_migrations(pool).await?;
    Ok(MIGRATIONS.len().saturating_sub(applied.len()))
}

/// Get the total number of migrations defined
pub fn total_migrations() -> usize {
    MIGRATIONS.len()
}

/// Get migration by version
pub fn get_migration(version: i32) -> Option<&'static Migration> {
    MIGRATIONS.iter().find(|m| m.version == version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::create_test_pool;

    #[tokio::test]
    async fn test_run_migrations() {
        let pool = create_test_pool().await.expect("Failed to create test pool");

        let count = run_migrations(&pool).await.expect("Failed to run migrations");
        assert_eq!(count, MIGRATIONS.len());

        // Running again should apply 0 migrations
        let count = run_migrations(&pool).await.expect("Failed to run migrations");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_is_up_to_date() {
        let pool = create_test_pool().await.expect("Failed to create test pool");

        // Before migrations
        let up_to_date = is_up_to_date(&pool).await.expect("Failed to check");
        assert!(!up_to_date);

        // After migrations
        run_migrations(&pool).await.expect("Failed to run migrations");
        let up_to_date = is_up_to_date(&pool).await.expect("Failed to check");
        assert!(up_to_date);
    }

    #[tokio::test]
    async fn test_pending_count() {
        let pool = create_test_pool().await.expect("Failed to create test pool");

        // Before migrations
        let pending = pending_count(&pool).await.expect("Failed to check");
        assert_eq!(pending, MIGRATIONS.len());

        // After migrations
        run_migrations(&pool).await.expect("Failed to run migrations");
        let pending = pending_count(&pool).await.expect("Failed to check");
        assert_eq!(pending, 0);
    }

    #[tokio::test]
    async fn test_users_table_created() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        // Verify users table exists and has correct structure
        let sqlite_pool = pool.as_sqlite().unwrap();
        let result = sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind("testuser")
        .bind("test@example.com")
        .bind("hash123")
        .bind("admin")
        .execute(sqlite_pool)
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sessions_table_created() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        let sqlite_pool = pool.as_sqlite().unwrap();

        // First create a user
        sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind("testuser")
        .bind("test@example.com")
        .bind("hash123")
        .bind("admin")
        .execute(sqlite_pool)
        .await
        .expect("Failed to create user");

        // Then create a session
        let result = sqlx::query(
            "INSERT INTO sessions (id, user_id, expires_at) VALUES (?, ?, datetime('now', '+1 day'))",
        )
        .bind("session123")
        .bind(1i64)
        .execute(sqlite_pool)
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_categories_table_created() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        let sqlite_pool = pool.as_sqlite().unwrap();

        // Verify default category exists
        let row =
            sqlx::query("SELECT COUNT(*) as count FROM categories WHERE slug = 'uncategorized'")
                .fetch_one(sqlite_pool)
                .await
                .expect("Failed to query categories");

        let count: i64 = row.get("count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_tags_table_created() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        let sqlite_pool = pool.as_sqlite().unwrap();

        let result = sqlx::query("INSERT INTO tags (slug, name) VALUES (?, ?)")
            .bind("rust")
            .bind("Rust")
            .execute(sqlite_pool)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_articles_table_created() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        let sqlite_pool = pool.as_sqlite().unwrap();

        // First create a user
        sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind("author")
        .bind("author@example.com")
        .bind("hash123")
        .bind("author")
        .execute(sqlite_pool)
        .await
        .expect("Failed to create user");

        // Create an article
        let result = sqlx::query(
            "INSERT INTO articles (slug, title, content, content_html, author_id, category_id, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("hello-world")
        .bind("Hello World")
        .bind("# Hello\n\nThis is content.")
        .bind("<h1>Hello</h1><p>This is content.</p>")
        .bind(1i64)
        .bind(1i64)
        .bind("draft")
        .execute(sqlite_pool)
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_article_tags_table_created() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create user
        sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind("author")
        .bind("author@example.com")
        .bind("hash123")
        .bind("author")
        .execute(sqlite_pool)
        .await
        .expect("Failed to create user");

        // Create article
        sqlx::query(
            "INSERT INTO articles (slug, title, content, content_html, author_id, category_id, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("test-article")
        .bind("Test Article")
        .bind("Content")
        .bind("<p>Content</p>")
        .bind(1i64)
        .bind(1i64)
        .bind("draft")
        .execute(sqlite_pool)
        .await
        .expect("Failed to create article");

        // Create tag
        sqlx::query("INSERT INTO tags (slug, name) VALUES (?, ?)")
            .bind("test-tag")
            .bind("Test Tag")
            .execute(sqlite_pool)
            .await
            .expect("Failed to create tag");

        // Associate tag with article
        let result = sqlx::query("INSERT INTO article_tags (article_id, tag_id) VALUES (?, ?)")
            .bind(1i64)
            .bind(1i64)
            .execute(sqlite_pool)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_settings_table_created() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        let sqlite_pool = pool.as_sqlite().unwrap();

        // Verify default settings exist
        let row = sqlx::query("SELECT value FROM settings WHERE key = 'site_name'")
            .fetch_one(sqlite_pool)
            .await
            .expect("Failed to query settings");

        let value: String = row.get("value");
        assert_eq!(value, "Noteva");
    }

    #[tokio::test]
    async fn test_foreign_key_constraints() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        let sqlite_pool = pool.as_sqlite().unwrap();

        // Try to create a session with non-existent user (should fail due to FK constraint)
        let result = sqlx::query(
            "INSERT INTO sessions (id, user_id, expires_at) VALUES (?, ?, datetime('now', '+1 day'))",
        )
        .bind("session123")
        .bind(999i64) // Non-existent user
        .execute(sqlite_pool)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unique_constraints() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create first user
        sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind("testuser")
        .bind("test@example.com")
        .bind("hash123")
        .bind("admin")
        .execute(sqlite_pool)
        .await
        .expect("Failed to create first user");

        // Try to create user with same username (should fail)
        let result = sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind("testuser") // Duplicate username
        .bind("other@example.com")
        .bind("hash456")
        .bind("author")
        .execute(sqlite_pool)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_category_parent_self_reference() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create a parent category
        sqlx::query("INSERT INTO categories (slug, name) VALUES (?, ?)")
            .bind("parent-cat")
            .bind("Parent Category")
            .execute(sqlite_pool)
            .await
            .expect("Failed to create parent category");

        // Get the parent category id
        let row = sqlx::query("SELECT id FROM categories WHERE slug = 'parent-cat'")
            .fetch_one(sqlite_pool)
            .await
            .expect("Failed to get parent id");
        let parent_id: i64 = row.get("id");

        // Create a child category
        let result =
            sqlx::query("INSERT INTO categories (slug, name, parent_id) VALUES (?, ?, ?)")
                .bind("child-cat")
                .bind("Child Category")
                .bind(parent_id)
                .execute(sqlite_pool)
                .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_article_status_values() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create a user
        sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind("author")
        .bind("author@example.com")
        .bind("hash123")
        .bind("author")
        .execute(sqlite_pool)
        .await
        .expect("Failed to create user");

        // Test different status values
        for (i, status) in ["draft", "published", "archived"].iter().enumerate() {
            let result = sqlx::query(
                "INSERT INTO articles (slug, title, content, content_html, author_id, category_id, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(format!("article-{}", i))
            .bind(format!("Article {}", i))
            .bind("Content")
            .bind("<p>Content</p>")
            .bind(1i64)
            .bind(1i64)
            .bind(*status)
            .execute(sqlite_pool)
            .await;

            assert!(result.is_ok(), "Failed to create article with status: {}", status);
        }
    }

    #[tokio::test]
    async fn test_get_migration() {
        // Test getting existing migration
        let migration = get_migration(1);
        assert!(migration.is_some());
        assert_eq!(migration.unwrap().name, "create_users");

        // Test getting non-existent migration
        let migration = get_migration(999);
        assert!(migration.is_none());
    }

    #[tokio::test]
    async fn test_total_migrations() {
        assert_eq!(total_migrations(), 18);
    }

    #[test]
    fn test_split_sql_statements() {
        let sql = "CREATE TABLE a (id INT); CREATE TABLE b (id INT);";
        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 2);

        // Test with comments
        let sql_with_comments = "-- Comment\nCREATE TABLE a (id INT);";
        let statements = split_sql_statements(sql_with_comments);
        assert_eq!(statements.len(), 1);
    }

    #[test]
    fn test_is_comment_only() {
        assert!(is_comment_only("-- This is a comment"));
        assert!(is_comment_only("-- Line 1\n-- Line 2"));
        assert!(!is_comment_only("CREATE TABLE test"));
        assert!(!is_comment_only("-- Comment\nCREATE TABLE test"));
    }
}
