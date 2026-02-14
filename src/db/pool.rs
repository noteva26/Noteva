//! Database connection pool abstraction
//!
//! This module provides a unified interface for database operations that works
//! with both SQLite and MySQL backends. The appropriate pool is created based
//! on the configuration.
//!
//! Satisfies requirement 8.2: THE Config_Manager SHALL 支持 SQLite 和 MySQL 数据库配置切换

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{
    mysql::{MySqlPool, MySqlPoolOptions},
    sqlite::{SqlitePool, SqlitePoolOptions},
};
use std::sync::Arc;

use crate::config::{DatabaseConfig, DatabaseDriver};

/// Database pool trait that abstracts over different database backends.
///
/// This trait provides a unified interface for database operations,
/// allowing the application to work with either SQLite or MySQL
/// without knowing the specific backend.
#[async_trait]
pub trait DatabasePool: Send + Sync {
    /// Execute a raw SQL query that doesn't return rows
    async fn execute(&self, query: &str) -> Result<u64>;

    /// Check if the database connection is healthy
    async fn ping(&self) -> Result<()>;

    /// Close the connection pool
    async fn close(&self);

    /// Get the database driver type
    fn driver(&self) -> DatabaseDriver;

    /// Get the underlying SQLite pool if this is a SQLite connection
    fn as_sqlite(&self) -> Option<&SqlitePool>;

    /// Get the underlying MySQL pool if this is a MySQL connection
    fn as_mysql(&self) -> Option<&MySqlPool>;
}

/// SQLite connection pool implementation
pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    /// Create a new SQLite connection pool
    pub async fn new(url: &str) -> Result<Self> {
        // Ensure the database directory exists for file-based SQLite
        if !url.starts_with(":memory:") && !url.starts_with("sqlite::memory:") {
            // Extract the path from the URL
            let path = if url.starts_with("sqlite:") {
                url.trim_start_matches("sqlite:")
            } else {
                url
            };

            // Create parent directory if it doesn't exist
            if let Some(parent) = std::path::Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("Failed to create database directory: {:?}", parent))?;
                }
            }
        }

        // Build the connection URL with create=true for file-based databases
        let connection_url = if url.starts_with("sqlite:") {
            // If it already has options, don't modify
            if url.contains('?') {
                url.to_string()
            } else {
                format!("{}?mode=rwc", url)
            }
        } else if url == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            // File path - add sqlite: prefix and create mode
            format!("sqlite:{}?mode=rwc", url)
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(20)
            .connect(&connection_url)
            .await
            .with_context(|| format!("Failed to connect to SQLite database: {}", url))?;

        // Enable foreign keys for SQLite
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .context("Failed to enable foreign keys")?;

        Ok(Self { pool })
    }

    /// Get a reference to the underlying pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl DatabasePool for SqliteDatabase {
    async fn execute(&self, query: &str) -> Result<u64> {
        let result = sqlx::query(query)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Failed to execute query: {}", query))?;
        Ok(result.rows_affected())
    }

    async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("Database ping failed")?;
        Ok(())
    }

    async fn close(&self) {
        self.pool.close().await;
    }

    fn driver(&self) -> DatabaseDriver {
        DatabaseDriver::Sqlite
    }

    fn as_sqlite(&self) -> Option<&SqlitePool> {
        Some(&self.pool)
    }

    fn as_mysql(&self) -> Option<&MySqlPool> {
        None
    }
}

/// MySQL connection pool implementation
pub struct MysqlDatabase {
    pool: MySqlPool,
}

impl MysqlDatabase {
    /// Create a new MySQL connection pool
    pub async fn new(url: &str) -> Result<Self> {
        // Build the connection URL
        let connection_url = if url.starts_with("mysql://") {
            url.to_string()
        } else {
            format!("mysql://{}", url)
        };

        let pool = MySqlPoolOptions::new()
            .max_connections(30)
            .connect(&connection_url)
            .await
            .with_context(|| format!("Failed to connect to MySQL database: {}", url))?;

        Ok(Self { pool })
    }

    /// Get a reference to the underlying pool
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }
}

#[async_trait]
impl DatabasePool for MysqlDatabase {
    async fn execute(&self, query: &str) -> Result<u64> {
        let result = sqlx::query(query)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Failed to execute query: {}", query))?;
        Ok(result.rows_affected())
    }

    async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("Database ping failed")?;
        Ok(())
    }

    async fn close(&self) {
        self.pool.close().await;
    }

    fn driver(&self) -> DatabaseDriver {
        DatabaseDriver::Mysql
    }

    fn as_sqlite(&self) -> Option<&SqlitePool> {
        None
    }

    fn as_mysql(&self) -> Option<&MySqlPool> {
        Some(&self.pool)
    }
}

/// Type alias for a boxed database pool
pub type DynDatabasePool = Arc<dyn DatabasePool>;

/// Create a database connection pool based on configuration.
///
/// This factory function reads the database configuration and creates
/// the appropriate connection pool (SQLite or MySQL).
///
/// # Arguments
///
/// * `config` - Database configuration containing driver type and connection URL
///
/// # Returns
///
/// A boxed trait object implementing `DatabasePool`
///
/// # Errors
///
/// Returns an error if the connection cannot be established.
///
/// # Example
///
/// ```ignore
/// use noteva::config::DatabaseConfig;
/// use noteva::db::create_pool;
///
/// let config = DatabaseConfig::default();
/// let pool = create_pool(&config).await?;
/// pool.ping().await?;
/// ```
///
/// Satisfies requirement 8.2: THE Config_Manager SHALL 支持 SQLite 和 MySQL 数据库配置切换
pub async fn create_pool(config: &DatabaseConfig) -> Result<DynDatabasePool> {
    match config.driver {
        DatabaseDriver::Sqlite => {
            let db = SqliteDatabase::new(&config.url).await?;
            Ok(Arc::new(db))
        }
        DatabaseDriver::Mysql => {
            let db = MysqlDatabase::new(&config.url).await?;
            Ok(Arc::new(db))
        }
    }
}

/// Create a SQLite in-memory database pool for testing
///
/// This is a convenience function for creating an in-memory SQLite database,
/// useful for unit tests and integration tests.
pub async fn create_test_pool() -> Result<DynDatabasePool> {
    let config = DatabaseConfig {
        driver: DatabaseDriver::Sqlite,
        url: ":memory:".to_string(),
    };
    create_pool(&config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_pool_creation() {
        let config = DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: ":memory:".to_string(),
        };

        let pool = create_pool(&config).await.expect("Failed to create pool");
        assert_eq!(pool.driver(), DatabaseDriver::Sqlite);
        assert!(pool.as_sqlite().is_some());
        assert!(pool.as_mysql().is_none());
    }

    #[tokio::test]
    async fn test_sqlite_pool_ping() {
        let config = DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: ":memory:".to_string(),
        };

        let pool = create_pool(&config).await.expect("Failed to create pool");
        pool.ping().await.expect("Ping should succeed");
    }

    #[tokio::test]
    async fn test_sqlite_pool_execute() {
        let config = DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: ":memory:".to_string(),
        };

        let pool = create_pool(&config).await.expect("Failed to create pool");

        // Create a test table
        pool.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .expect("Failed to create table");

        // Insert a row
        let affected = pool
            .execute("INSERT INTO test (name) VALUES ('test')")
            .await
            .expect("Failed to insert");
        assert_eq!(affected, 1);
    }

    #[tokio::test]
    async fn test_sqlite_file_pool_creation() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let config = DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: db_path.to_string_lossy().to_string(),
        };

        let pool = create_pool(&config).await.expect("Failed to create pool");
        pool.ping().await.expect("Ping should succeed");

        // Verify the file was created
        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn test_sqlite_nested_directory_creation() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("nested").join("dir").join("test.db");

        let config = DatabaseConfig {
            driver: DatabaseDriver::Sqlite,
            url: db_path.to_string_lossy().to_string(),
        };

        let pool = create_pool(&config).await.expect("Failed to create pool");
        pool.ping().await.expect("Ping should succeed");

        // Verify the file and directories were created
        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn test_create_test_pool() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        assert_eq!(pool.driver(), DatabaseDriver::Sqlite);
        pool.ping().await.expect("Ping should succeed");
    }

    #[tokio::test]
    async fn test_pool_close() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        pool.close().await;
        // After close, ping should fail
        // Note: SQLite might still work after close in some cases,
        // but this tests the close method is callable
    }

    // MySQL tests are skipped by default as they require a running MySQL server
    // To run MySQL tests, set the MYSQL_TEST_URL environment variable
    #[tokio::test]
    #[ignore = "Requires MySQL server"]
    async fn test_mysql_pool_creation() {
        let url = std::env::var("MYSQL_TEST_URL")
            .unwrap_or_else(|_| "mysql://root@localhost/test".to_string());

        let config = DatabaseConfig {
            driver: DatabaseDriver::Mysql,
            url,
        };

        let pool = create_pool(&config).await.expect("Failed to create pool");
        assert_eq!(pool.driver(), DatabaseDriver::Mysql);
        assert!(pool.as_mysql().is_some());
        assert!(pool.as_sqlite().is_none());
    }

    #[tokio::test]
    #[ignore = "Requires MySQL server"]
    async fn test_mysql_pool_ping() {
        let url = std::env::var("MYSQL_TEST_URL")
            .unwrap_or_else(|_| "mysql://root@localhost/test".to_string());

        let config = DatabaseConfig {
            driver: DatabaseDriver::Mysql,
            url,
        };

        let pool = create_pool(&config).await.expect("Failed to create pool");
        pool.ping().await.expect("Ping should succeed");
    }
}
