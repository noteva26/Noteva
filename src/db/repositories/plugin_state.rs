//! Plugin state repository
//!
//! Handles database operations for plugin states (enabled/disabled, settings).

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool, MySqlPool};
use std::collections::HashMap;

use crate::db::DynDatabasePool;

/// Plugin state stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginState {
    pub plugin_id: String,
    pub enabled: bool,
    pub settings: HashMap<String, serde_json::Value>,
    /// Last known plugin version (for upgrade detection)
    pub last_version: Option<String>,
}

/// Plugin state repository trait
#[async_trait]
pub trait PluginStateRepository: Send + Sync {
    /// Get state for a plugin
    async fn get(&self, plugin_id: &str) -> Result<Option<PluginState>>;
    
    /// Get all plugin states
    async fn get_all(&self) -> Result<Vec<PluginState>>;
    
    /// Save plugin state (upsert)
    async fn save(&self, state: &PluginState) -> Result<()>;
    
    /// Delete plugin state
    async fn delete(&self, plugin_id: &str) -> Result<bool>;

    /// Update last_version for a plugin
    async fn update_last_version(&self, plugin_id: &str, version: &str) -> Result<()>;
}

/// SQLx-based plugin state repository
pub struct SqlxPluginStateRepository {
    pool: DynDatabasePool,
}

impl SqlxPluginStateRepository {
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PluginStateRepository for SqlxPluginStateRepository {
    async fn get(&self, plugin_id: &str) -> Result<Option<PluginState>> {
        if let Some(pool) = self.pool.as_sqlite() {
            get_sqlite(pool, plugin_id).await
        } else if let Some(pool) = self.pool.as_mysql() {
            get_mysql(pool, plugin_id).await
        } else {
            Ok(None)
        }
    }
    
    async fn get_all(&self) -> Result<Vec<PluginState>> {
        if let Some(pool) = self.pool.as_sqlite() {
            get_all_sqlite(pool).await
        } else if let Some(pool) = self.pool.as_mysql() {
            get_all_mysql(pool).await
        } else {
            Ok(vec![])
        }
    }
    
    async fn save(&self, state: &PluginState) -> Result<()> {
        if let Some(pool) = self.pool.as_sqlite() {
            save_sqlite(pool, state).await
        } else if let Some(pool) = self.pool.as_mysql() {
            save_mysql(pool, state).await
        } else {
            Ok(())
        }
    }
    
    async fn delete(&self, plugin_id: &str) -> Result<bool> {
        if let Some(pool) = self.pool.as_sqlite() {
            delete_sqlite(pool, plugin_id).await
        } else if let Some(pool) = self.pool.as_mysql() {
            delete_mysql(pool, plugin_id).await
        } else {
            Ok(false)
        }
    }

    async fn update_last_version(&self, plugin_id: &str, version: &str) -> Result<()> {
        if let Some(pool) = self.pool.as_sqlite() {
            update_last_version_sqlite(pool, plugin_id, version).await
        } else if let Some(pool) = self.pool.as_mysql() {
            update_last_version_mysql(pool, plugin_id, version).await
        } else {
            Ok(())
        }
    }
}

// SQLite implementations

async fn get_sqlite(pool: &SqlitePool, plugin_id: &str) -> Result<Option<PluginState>> {
    let row = sqlx::query(
        "SELECT plugin_id, enabled, settings, last_version FROM plugin_states WHERE plugin_id = ?"
    )
    .bind(plugin_id)
    .fetch_optional(pool)
    .await?;
    
    Ok(row.map(|r| {
        let settings_str: String = r.get("settings");
        let settings: HashMap<String, serde_json::Value> = 
            serde_json::from_str(&settings_str).unwrap_or_default();
        
        PluginState {
            plugin_id: r.get("plugin_id"),
            enabled: r.get::<i32, _>("enabled") != 0,
            settings,
            last_version: r.try_get::<Option<String>, _>("last_version").unwrap_or(None),
        }
    }))
}

async fn get_all_sqlite(pool: &SqlitePool) -> Result<Vec<PluginState>> {
    let rows = sqlx::query(
        "SELECT plugin_id, enabled, settings, last_version FROM plugin_states"
    )
    .fetch_all(pool)
    .await?;
    
    Ok(rows.iter().map(|r| {
        let settings_str: String = r.get("settings");
        let settings: HashMap<String, serde_json::Value> = 
            serde_json::from_str(&settings_str).unwrap_or_default();
        
        PluginState {
            plugin_id: r.get("plugin_id"),
            enabled: r.get::<i32, _>("enabled") != 0,
            settings,
            last_version: r.try_get::<Option<String>, _>("last_version").unwrap_or(None),
        }
    }).collect())
}

async fn save_sqlite(pool: &SqlitePool, state: &PluginState) -> Result<()> {
    let settings_json = serde_json::to_string(&state.settings)?;
    
    sqlx::query(
        r#"INSERT INTO plugin_states (plugin_id, enabled, settings, last_version, updated_at)
           VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
           ON CONFLICT(plugin_id) DO UPDATE SET
               enabled = excluded.enabled,
               settings = excluded.settings,
               last_version = COALESCE(excluded.last_version, plugin_states.last_version),
               updated_at = CURRENT_TIMESTAMP"#
    )
    .bind(&state.plugin_id)
    .bind(if state.enabled { 1 } else { 0 })
    .bind(&settings_json)
    .bind(&state.last_version)
    .execute(pool)
    .await?;
    
    Ok(())
}

async fn delete_sqlite(pool: &SqlitePool, plugin_id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM plugin_states WHERE plugin_id = ?")
        .bind(plugin_id)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected() > 0)
}

async fn update_last_version_sqlite(pool: &SqlitePool, plugin_id: &str, version: &str) -> Result<()> {
    sqlx::query("UPDATE plugin_states SET last_version = ?, updated_at = CURRENT_TIMESTAMP WHERE plugin_id = ?")
        .bind(version)
        .bind(plugin_id)
        .execute(pool)
        .await?;
    Ok(())
}

// MySQL implementations

async fn get_mysql(pool: &MySqlPool, plugin_id: &str) -> Result<Option<PluginState>> {
    let row = sqlx::query(
        "SELECT plugin_id, enabled, settings, last_version FROM plugin_states WHERE plugin_id = ?"
    )
    .bind(plugin_id)
    .fetch_optional(pool)
    .await?;
    
    Ok(row.map(|r| {
        let settings_str: String = r.get("settings");
        let settings: HashMap<String, serde_json::Value> = 
            serde_json::from_str(&settings_str).unwrap_or_default();
        
        PluginState {
            plugin_id: r.get("plugin_id"),
            enabled: r.get::<i8, _>("enabled") != 0,
            settings,
            last_version: r.try_get::<Option<String>, _>("last_version").unwrap_or(None),
        }
    }))
}

async fn get_all_mysql(pool: &MySqlPool) -> Result<Vec<PluginState>> {
    let rows = sqlx::query(
        "SELECT plugin_id, enabled, settings, last_version FROM plugin_states"
    )
    .fetch_all(pool)
    .await?;
    
    Ok(rows.iter().map(|r| {
        let settings_str: String = r.get("settings");
        let settings: HashMap<String, serde_json::Value> = 
            serde_json::from_str(&settings_str).unwrap_or_default();
        
        PluginState {
            plugin_id: r.get("plugin_id"),
            enabled: r.get::<i8, _>("enabled") != 0,
            settings,
            last_version: r.try_get::<Option<String>, _>("last_version").unwrap_or(None),
        }
    }).collect())
}

async fn save_mysql(pool: &MySqlPool, state: &PluginState) -> Result<()> {
    let settings_json = serde_json::to_string(&state.settings)?;
    
    sqlx::query(
        r#"INSERT INTO plugin_states (plugin_id, enabled, settings, last_version, updated_at)
           VALUES (?, ?, ?, ?, NOW())
           ON DUPLICATE KEY UPDATE
               enabled = VALUES(enabled),
               settings = VALUES(settings),
               last_version = COALESCE(VALUES(last_version), last_version),
               updated_at = NOW()"#
    )
    .bind(&state.plugin_id)
    .bind(if state.enabled { 1 } else { 0 })
    .bind(&settings_json)
    .bind(&state.last_version)
    .execute(pool)
    .await?;
    
    Ok(())
}

async fn delete_mysql(pool: &MySqlPool, plugin_id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM plugin_states WHERE plugin_id = ?")
        .bind(plugin_id)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected() > 0)
}

async fn update_last_version_mysql(pool: &MySqlPool, plugin_id: &str, version: &str) -> Result<()> {
    sqlx::query("UPDATE plugin_states SET last_version = ?, updated_at = NOW() WHERE plugin_id = ?")
        .bind(version)
        .bind(plugin_id)
        .execute(pool)
        .await?;
    Ok(())
}
