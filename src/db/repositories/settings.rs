//! Settings repository
//!
//! Repository for managing site settings in the database.
//! Satisfies requirement 5.3: System configuration storage

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{MySqlPool, Row, SqlitePool};
use std::collections::HashMap;

use crate::db::DynDatabasePool;

/// A setting key-value pair
#[derive(Debug, Clone)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

/// Repository trait for settings operations
#[async_trait]
pub trait SettingsRepository: Send + Sync {
    /// Get a single setting by key
    async fn get(&self, key: &str) -> Result<Option<Setting>>;

    /// Get all settings
    async fn get_all(&self) -> Result<Vec<Setting>>;

    /// Get multiple settings by keys
    async fn get_many(&self, keys: &[&str]) -> Result<HashMap<String, String>>;

    /// Set a single setting
    async fn set(&self, key: &str, value: &str) -> Result<()>;

    /// Set multiple settings at once
    async fn set_many(&self, settings: &HashMap<String, String>) -> Result<()>;

    /// Delete a setting
    async fn delete(&self, key: &str) -> Result<()>;
}

/// SQLx-based settings repository
pub struct SqlxSettingsRepository {
    pool: DynDatabasePool,
}

impl SqlxSettingsRepository {
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SettingsRepository for SqlxSettingsRepository {
    async fn get(&self, key: &str) -> Result<Option<Setting>> {
        dispatch!(self, get, key)
    }

    async fn get_all(&self) -> Result<Vec<Setting>> {
        dispatch!(self, get_all)
    }

    async fn get_many(&self, keys: &[&str]) -> Result<HashMap<String, String>> {
        dispatch!(self, get_many, keys)
    }

    async fn set(&self, key: &str, value: &str) -> Result<()> {
        dispatch!(self, set, key, value)
    }

    async fn set_many(&self, settings: &HashMap<String, String>) -> Result<()> {
        for (key, value) in settings {
            self.set(key, value).await?;
        }
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        dispatch!(self, delete, key)
    }
}

// ============================================================================
// Shared implementations (identical SQL for both backends)
// ============================================================================

impl_dual_fn! {
    async fn delete(pool, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key)
            .execute(pool)
            .await?;
        Ok(())
    }
}

// ============================================================================
// SQLite implementations (SQL dialect differences)
// ============================================================================

async fn get_sqlite(pool: &SqlitePool, key: &str) -> Result<Option<Setting>> {
    let row = sqlx::query("SELECT key, value, updated_at FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| Setting {
        key: r.get("key"),
        value: r.get("value"),
        updated_at: r.get("updated_at"),
    }))
}

async fn get_all_sqlite(pool: &SqlitePool) -> Result<Vec<Setting>> {
    let rows = sqlx::query("SELECT key, value, updated_at FROM settings ORDER BY key")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| Setting {
            key: r.get("key"),
            value: r.get("value"),
            updated_at: r.get("updated_at"),
        })
        .collect())
}

async fn get_many_sqlite(pool: &SqlitePool, keys: &[&str]) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    for key in keys {
        if let Some(setting) = get_sqlite(pool, key).await? {
            result.insert(setting.key, setting.value);
        }
    }
    Ok(result)
}

async fn set_sqlite(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

// ============================================================================
// MySQL implementations (SQL dialect differences)
// ============================================================================

async fn get_mysql(pool: &MySqlPool, key: &str) -> Result<Option<Setting>> {
    let row = sqlx::query("SELECT `key`, value, updated_at FROM settings WHERE `key` = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| Setting {
        key: r.get("key"),
        value: r.get("value"),
        updated_at: r.get("updated_at"),
    }))
}

async fn get_all_mysql(pool: &MySqlPool) -> Result<Vec<Setting>> {
    let rows = sqlx::query("SELECT `key`, value, updated_at FROM settings ORDER BY `key`")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| Setting {
            key: r.get("key"),
            value: r.get("value"),
            updated_at: r.get("updated_at"),
        })
        .collect())
}

async fn get_many_mysql(pool: &MySqlPool, keys: &[&str]) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    for key in keys {
        if let Some(setting) = get_mysql(pool, key).await? {
            result.insert(setting.key, setting.value);
        }
    }
    Ok(result)
}

async fn set_mysql(pool: &MySqlPool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO settings (`key`, value) VALUES (?, ?)
         ON DUPLICATE KEY UPDATE value = VALUES(value)",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}
