//! Plugin data repository
//!
//! Handles database operations for plugin key-value data storage.

use anyhow::Result;
use async_trait::async_trait;
use sqlx::{MySqlPool, SqlitePool, Row};

use crate::db::DynDatabasePool;

/// Plugin data entry
#[derive(Debug, Clone)]
pub struct PluginData {
    pub plugin_id: String,
    pub key: String,
    pub value: String,
}

/// Repository trait for plugin data operations
#[async_trait]
pub trait PluginDataRepository: Send + Sync {
    /// Get a value by plugin_id and key
    async fn get(&self, plugin_id: &str, key: &str) -> Result<Option<String>>;
    
    /// Get all data for a plugin
    async fn get_all(&self, plugin_id: &str) -> Result<Vec<PluginData>>;
    
    /// Set a value
    async fn set(&self, plugin_id: &str, key: &str, value: &str) -> Result<()>;
    
    /// Delete a value
    async fn delete(&self, plugin_id: &str, key: &str) -> Result<bool>;
    
    /// Delete all data for a plugin
    async fn delete_all(&self, plugin_id: &str) -> Result<usize>;
}

/// SQLx-based plugin data repository
pub struct SqlxPluginDataRepository {
    pool: DynDatabasePool,
}

impl SqlxPluginDataRepository {
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PluginDataRepository for SqlxPluginDataRepository {
    async fn get(&self, plugin_id: &str, key: &str) -> Result<Option<String>> {
        dispatch!(self, get, plugin_id, key)
    }
    
    async fn get_all(&self, plugin_id: &str) -> Result<Vec<PluginData>> {
        dispatch!(self, get_all, plugin_id)
    }
    
    async fn set(&self, plugin_id: &str, key: &str, value: &str) -> Result<()> {
        dispatch!(self, set, plugin_id, key, value)
    }
    
    async fn delete(&self, plugin_id: &str, key: &str) -> Result<bool> {
        dispatch!(self, delete, plugin_id, key)
    }
    
    async fn delete_all(&self, plugin_id: &str) -> Result<usize> {
        dispatch!(self, delete_all, plugin_id)
    }
}

// ============================================================================
// Shared implementations (identical SQL)
// ============================================================================

impl_dual_fn! {
    async fn delete(pool, plugin_id: &str, key: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM plugin_data WHERE plugin_id = ? AND key = ?"
        )
        .bind(plugin_id)
        .bind(key)
        .execute(pool)
        .await?;
        
        Ok(result.rows_affected() > 0)
    }
}

impl_dual_fn! {
    async fn delete_all(pool, plugin_id: &str) -> Result<usize> {
        let result = sqlx::query(
            "DELETE FROM plugin_data WHERE plugin_id = ?"
        )
        .bind(plugin_id)
        .execute(pool)
        .await?;
        
        Ok(result.rows_affected() as usize)
    }
}

// ============================================================================
// SQLite implementations (SQL dialect differences)
// ============================================================================

async fn get_sqlite(pool: &SqlitePool, plugin_id: &str, key: &str) -> Result<Option<String>> {
    let row = sqlx::query(
        "SELECT value FROM plugin_data WHERE plugin_id = ? AND key = ?"
    )
    .bind(plugin_id)
    .bind(key)
    .fetch_optional(pool)
    .await?;
    
    Ok(row.map(|r| r.get("value")))
}

async fn get_all_sqlite(pool: &SqlitePool, plugin_id: &str) -> Result<Vec<PluginData>> {
    let rows = sqlx::query(
        "SELECT plugin_id, key, value FROM plugin_data WHERE plugin_id = ?"
    )
    .bind(plugin_id)
    .fetch_all(pool)
    .await?;
    
    Ok(rows.iter().map(|r| PluginData {
        plugin_id: r.get("plugin_id"),
        key: r.get("key"),
        value: r.get("value"),
    }).collect())
}

async fn set_sqlite(pool: &SqlitePool, plugin_id: &str, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO plugin_data (plugin_id, key, value, updated_at)
           VALUES (?, ?, ?, datetime('now'))
           ON CONFLICT(plugin_id, key) DO UPDATE SET
               value = excluded.value,
               updated_at = datetime('now')"#
    )
    .bind(plugin_id)
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    
    Ok(())
}

// ============================================================================
// MySQL implementations (SQL dialect differences)
// ============================================================================

async fn get_mysql(pool: &MySqlPool, plugin_id: &str, key: &str) -> Result<Option<String>> {
    let row = sqlx::query(
        "SELECT value FROM plugin_data WHERE plugin_id = ? AND `key` = ?"
    )
    .bind(plugin_id)
    .bind(key)
    .fetch_optional(pool)
    .await?;
    
    Ok(row.map(|r| r.get("value")))
}

async fn get_all_mysql(pool: &MySqlPool, plugin_id: &str) -> Result<Vec<PluginData>> {
    let rows = sqlx::query(
        "SELECT plugin_id, `key`, value FROM plugin_data WHERE plugin_id = ?"
    )
    .bind(plugin_id)
    .fetch_all(pool)
    .await?;
    
    Ok(rows.iter().map(|r| PluginData {
        plugin_id: r.get("plugin_id"),
        key: r.get("key"),
        value: r.get("value"),
    }).collect())
}

async fn set_mysql(pool: &MySqlPool, plugin_id: &str, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO plugin_data (plugin_id, `key`, value)
           VALUES (?, ?, ?)
           ON DUPLICATE KEY UPDATE value = VALUES(value)"#
    )
    .bind(plugin_id)
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    
    Ok(())
}
