//! Database layer
//!
//! This module provides database abstraction for the Noteva blog system.
//! It supports:
//! - SQLite (default, for single-binary deployment)
//! - MySQL (for larger deployments)
//!
//! The database driver is selected based on configuration.
//!
//! # Architecture
//!
//! The database layer uses a trait-based abstraction (`DatabasePool`) that
//! allows the application to work with either SQLite or MySQL without
//! knowing the specific backend. This satisfies requirement 8.2:
//! THE Config_Manager SHALL 支持 SQLite 和 MySQL 数据库配置切换
//!
//! # Usage
//!
//! ```ignore
//! use noteva::config::DatabaseConfig;
//! use noteva::db::{create_pool, DatabasePool, migrations};
//!
//! // Create pool from configuration
//! let config = DatabaseConfig::default();
//! let pool = create_pool(&config).await?;
//!
//! // Run migrations
//! migrations::run_migrations(&pool).await?;
//!
//! // Use the pool
//! pool.ping().await?;
//!
//! // Access the underlying pool for specific operations
//! if let Some(sqlite_pool) = pool.as_sqlite() {
//!     // SQLite-specific operations
//! }
//! ```

pub mod migrations;
pub mod pool;
pub mod repositories;

pub use pool::{
    create_pool, create_test_pool, DatabasePool, DynDatabasePool, MysqlDatabase, SqliteDatabase,
};
