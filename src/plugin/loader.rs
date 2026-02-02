//! Plugin loader and manager
//!
//! Handles plugin discovery, loading, and lifecycle management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

use crate::db::repositories::{PluginState, PluginStateRepository, SqlxPluginStateRepository};
use crate::db::DynDatabasePool;

/// Current Noteva version from Cargo.toml
pub const NOTEVA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Version compatibility check result
#[derive(Debug, Clone)]
pub struct VersionCheckResult {
    pub compatible: bool,
    pub message: Option<String>,
}

/// Check if a version requirement is satisfied
/// 
/// Supports formats:
/// - ">=0.0.8" - minimum version
/// - ">=0.0.8,<1.0.0" - version range
/// - "0.0.8" - exact version
pub fn check_version_requirement(requirement: &str, current: &str) -> VersionCheckResult {
    if requirement.is_empty() {
        return VersionCheckResult { compatible: true, message: None };
    }
    
    let current_parts = parse_version(current);
    
    // Handle multiple constraints (comma-separated)
    for constraint in requirement.split(',') {
        let constraint = constraint.trim();
        if constraint.is_empty() {
            continue;
        }
        
        let result = check_single_constraint(constraint, &current_parts, current);
        if !result.compatible {
            return result;
        }
    }
    
    VersionCheckResult { compatible: true, message: None }
}

fn check_single_constraint(constraint: &str, current_parts: &[u32], current: &str) -> VersionCheckResult {
    if constraint.starts_with(">=") {
        let min_version = &constraint[2..];
        let min_parts = parse_version(min_version);
        if compare_versions(current_parts, &min_parts) < 0 {
            return VersionCheckResult {
                compatible: false,
                message: Some(format!("需要 Noteva {} 或更高版本，当前版本: {}", min_version, current)),
            };
        }
    } else if constraint.starts_with("<=") {
        let max_version = &constraint[2..];
        let max_parts = parse_version(max_version);
        if compare_versions(current_parts, &max_parts) > 0 {
            return VersionCheckResult {
                compatible: false,
                message: Some(format!("最高支持 Noteva {}，当前版本: {}", max_version, current)),
            };
        }
    } else if constraint.starts_with('>') {
        let min_version = &constraint[1..];
        let min_parts = parse_version(min_version);
        if compare_versions(current_parts, &min_parts) <= 0 {
            return VersionCheckResult {
                compatible: false,
                message: Some(format!("需要高于 Noteva {} 的版本，当前版本: {}", min_version, current)),
            };
        }
    } else if constraint.starts_with('<') {
        let max_version = &constraint[1..];
        let max_parts = parse_version(max_version);
        if compare_versions(current_parts, &max_parts) >= 0 {
            return VersionCheckResult {
                compatible: false,
                message: Some(format!("需要低于 Noteva {} 的版本，当前版本: {}", max_version, current)),
            };
        }
    } else {
        // Exact version match
        let exact_parts = parse_version(constraint);
        if compare_versions(current_parts, &exact_parts) != 0 {
            return VersionCheckResult {
                compatible: false,
                message: Some(format!("需要 Noteva {} 版本，当前版本: {}", constraint, current)),
            };
        }
    }
    
    VersionCheckResult { compatible: true, message: None }
}

fn parse_version(version: &str) -> Vec<u32> {
    // Remove any suffix like -beta, -alpha, etc.
    let version = version.split('-').next().unwrap_or(version);
    version
        .split('.')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect()
}

fn compare_versions(a: &[u32], b: &[u32]) -> i32 {
    let max_len = a.len().max(b.len());
    for i in 0..max_len {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        if av < bv {
            return -1;
        }
        if av > bv {
            return 1;
        }
    }
    0
}

/// Plugin metadata from plugin.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique plugin identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Version string
    pub version: String,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Author name
    #[serde(default)]
    pub author: String,
    /// Homepage URL
    #[serde(default)]
    pub homepage: String,
    /// License
    #[serde(default)]
    pub license: String,
    /// Required Noteva version
    #[serde(default)]
    pub requires: PluginRequirements,
    /// Hooks this plugin uses
    #[serde(default)]
    pub hooks: PluginHooks,
    /// Shortcodes provided by this plugin
    #[serde(default)]
    pub shortcodes: Vec<String>,
    /// Permissions required
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Whether plugin has settings
    #[serde(default)]
    pub settings: bool,
    /// Whether plugin needs database tables
    #[serde(default)]
    pub database: bool,
}

/// Plugin requirements
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginRequirements {
    /// Minimum Noteva version
    #[serde(default)]
    pub noteva: String,
    /// Required plugins
    #[serde(default)]
    pub plugins: Vec<String>,
}

/// Plugin hooks configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginHooks {
    /// Backend hooks
    #[serde(default)]
    pub backend: Vec<String>,
    /// Frontend hooks
    #[serde(default)]
    pub frontend: Vec<String>,
    /// Admin hooks
    #[serde(default)]
    pub admin: Vec<String>,
    /// Editor hooks
    #[serde(default)]
    pub editor: Vec<String>,
}

/// Legacy plugin states from data/plugins.json (for migration)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LegacyPluginState {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LegacyPluginStates(pub HashMap<String, LegacyPluginState>);

impl LegacyPluginStates {
    /// Load from file
    pub fn load(path: &Path) -> Option<Self> {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(states) => return Some(states),
                        Err(e) => warn!("Failed to parse plugins.json: {}", e),
                    }
                }
                Err(e) => warn!("Failed to read plugins.json: {}", e),
            }
        }
        None
    }
}

/// Loaded plugin instance
#[derive(Debug, Clone)]
pub struct Plugin {
    /// Plugin metadata
    pub metadata: PluginMetadata,
    /// Plugin directory path
    pub path: PathBuf,
    /// Whether plugin is enabled
    pub enabled: bool,
    /// Plugin settings (loaded from database)
    pub settings: HashMap<String, serde_json::Value>,
}

impl Plugin {
    /// Get frontend.js content if exists
    pub fn get_frontend_js(&self) -> Option<String> {
        let path = self.path.join("frontend.js");
        fs::read_to_string(&path).ok()
    }
    
    /// Get frontend.css content if exists
    pub fn get_frontend_css(&self) -> Option<String> {
        let path = self.path.join("frontend.css");
        fs::read_to_string(&path).ok()
    }
    
    /// Get admin.js content if exists
    pub fn get_admin_js(&self) -> Option<String> {
        let path = self.path.join("admin.js");
        fs::read_to_string(&path).ok()
    }
    
    /// Get admin.css content if exists
    pub fn get_admin_css(&self) -> Option<String> {
        let path = self.path.join("admin.css");
        fs::read_to_string(&path).ok()
    }
    
    /// Get settings.json schema if exists
    pub fn get_settings_schema(&self) -> Option<serde_json::Value> {
        let path = self.path.join("settings.json");
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }
    
    /// Get migration SQL files
    pub fn get_migrations(&self) -> Vec<(String, String)> {
        let migrations_dir = self.path.join("migrations");
        if !migrations_dir.exists() {
            return Vec::new();
        }
        
        let mut migrations = Vec::new();
        if let Ok(entries) = fs::read_dir(&migrations_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "sql") {
                    if let (Some(name), Ok(content)) = (
                        path.file_name().and_then(|n| n.to_str()).map(String::from),
                        fs::read_to_string(&path)
                    ) {
                        migrations.push((name, content));
                    }
                }
            }
        }
        
        // Sort by filename (001_init.sql, 002_update.sql, etc.)
        migrations.sort_by(|a, b| a.0.cmp(&b.0));
        migrations
    }
    
    /// Get locale messages for a specific language
    pub fn get_locale(&self, lang: &str) -> Option<serde_json::Value> {
        let path = self.path.join("locales").join(format!("{}.json", lang));
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }
}

/// Plugin manager - handles loading and managing plugins
pub struct PluginManager {
    /// Plugins directory path
    plugins_dir: PathBuf,
    /// Data directory path (for legacy migration)
    data_dir: PathBuf,
    /// Database repository for plugin states
    repo: Arc<dyn PluginStateRepository>,
    /// Loaded plugins (id -> Plugin)
    plugins: HashMap<String, Plugin>,
}

impl PluginManager {
    /// Create a new plugin manager with database support
    pub fn new(plugins_dir: &Path, data_dir: &Path, pool: DynDatabasePool) -> Self {
        Self {
            plugins_dir: plugins_dir.to_path_buf(),
            data_dir: data_dir.to_path_buf(),
            repo: Arc::new(SqlxPluginStateRepository::new(pool)),
            plugins: HashMap::new(),
        }
    }
    
    /// Get the path to legacy plugins.json
    fn legacy_states_path(&self) -> PathBuf {
        self.data_dir.join("plugins.json")
    }
    
    /// Initialize and load all plugins
    pub async fn init(&mut self) -> Result<()> {
        // Ensure plugins directory exists
        if !self.plugins_dir.exists() {
            fs::create_dir_all(&self.plugins_dir)
                .context("Failed to create plugins directory")?;
        }
        
        // Migrate legacy plugins.json to database if exists
        self.migrate_legacy_states().await?;
        
        // Scan for plugins
        self.scan_plugins().await?;
        
        info!("Loaded {} plugins", self.plugins.len());
        Ok(())
    }
    
    /// Migrate legacy plugins.json to database
    async fn migrate_legacy_states(&self) -> Result<()> {
        let legacy_path = self.legacy_states_path();
        
        if let Some(legacy_states) = LegacyPluginStates::load(&legacy_path) {
            info!("Migrating legacy plugins.json to database...");
            
            for (plugin_id, state) in legacy_states.0 {
                let db_state = PluginState {
                    plugin_id: plugin_id.clone(),
                    enabled: state.enabled,
                    settings: state.settings,
                };
                
                if let Err(e) = self.repo.save(&db_state).await {
                    warn!("Failed to migrate plugin state for {}: {}", plugin_id, e);
                }
            }
            
            // Rename old file to .bak
            let backup_path = legacy_path.with_extension("json.bak");
            if let Err(e) = fs::rename(&legacy_path, &backup_path) {
                warn!("Failed to backup plugins.json: {}", e);
            } else {
                info!("Legacy plugins.json migrated and backed up to plugins.json.bak");
            }
        }
        
        Ok(())
    }
    
    /// Scan plugins directory and load plugin metadata
    async fn scan_plugins(&mut self) -> Result<()> {
        let entries = fs::read_dir(&self.plugins_dir)
            .context("Failed to read plugins directory")?;
        
        // Load all states from database
        let db_states = self.repo.get_all().await.unwrap_or_default();
        let states_map: HashMap<String, PluginState> = db_states
            .into_iter()
            .map(|s| (s.plugin_id.clone(), s))
            .collect();
        
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                match self.load_plugin(&path, &states_map) {
                    Ok(plugin) => {
                        info!("Loaded plugin: {} v{}", plugin.metadata.name, plugin.metadata.version);
                        self.plugins.insert(plugin.metadata.id.clone(), plugin);
                    }
                    Err(e) => {
                        warn!("Failed to load plugin from {:?}: {}", path, e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Load a single plugin from directory
    fn load_plugin(&self, path: &Path, states_map: &HashMap<String, PluginState>) -> Result<Plugin> {
        let plugin_json = path.join("plugin.json");
        
        if !plugin_json.exists() {
            anyhow::bail!("plugin.json not found");
        }
        
        let content = fs::read_to_string(&plugin_json)
            .context("Failed to read plugin.json")?;
        
        let metadata: PluginMetadata = serde_json::from_str(&content)
            .context("Failed to parse plugin.json")?;
        
        // Get state from database
        let state = states_map.get(&metadata.id);
        let enabled = state.map(|s| s.enabled).unwrap_or(false);
        let settings = state.map(|s| s.settings.clone()).unwrap_or_default();
        
        Ok(Plugin {
            metadata,
            path: path.to_path_buf(),
            enabled,
            settings,
        })
    }
    
    /// Get all loaded plugins
    pub fn get_all(&self) -> Vec<&Plugin> {
        self.plugins.values().collect()
    }
    
    /// Get enabled plugins only
    pub fn get_enabled(&self) -> Vec<&Plugin> {
        self.plugins.values().filter(|p| p.enabled).collect()
    }
    
    /// Get a plugin by ID
    pub fn get(&self, id: &str) -> Option<&Plugin> {
        self.plugins.get(id)
    }
    
    /// Get a mutable plugin by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Plugin> {
        self.plugins.get_mut(id)
    }
    
    /// Enable a plugin
    pub async fn enable(&mut self, id: &str) -> Result<()> {
        if let Some(plugin) = self.plugins.get_mut(id) {
            // Check version compatibility
            let version_check = check_version_requirement(&plugin.metadata.requires.noteva, NOTEVA_VERSION);
            if !version_check.compatible {
                anyhow::bail!("{}", version_check.message.unwrap_or_else(|| "版本不兼容".to_string()));
            }
            
            plugin.enabled = true;
            
            // Save to database
            let state = PluginState {
                plugin_id: id.to_string(),
                enabled: true,
                settings: plugin.settings.clone(),
            };
            self.repo.save(&state).await?;
            
            info!("Enabled plugin: {}", id);
            Ok(())
        } else {
            anyhow::bail!("Plugin not found: {}", id)
        }
    }
    
    /// Check if a plugin is compatible with current Noteva version
    pub fn check_compatibility(&self, id: &str) -> Option<VersionCheckResult> {
        self.plugins.get(id).map(|plugin| {
            check_version_requirement(&plugin.metadata.requires.noteva, NOTEVA_VERSION)
        })
    }
    
    /// Disable a plugin
    pub async fn disable(&mut self, id: &str) -> Result<()> {
        if let Some(plugin) = self.plugins.get_mut(id) {
            plugin.enabled = false;
            
            // Save to database
            let state = PluginState {
                plugin_id: id.to_string(),
                enabled: false,
                settings: plugin.settings.clone(),
            };
            self.repo.save(&state).await?;
            
            info!("Disabled plugin: {}", id);
            Ok(())
        } else {
            anyhow::bail!("Plugin not found: {}", id)
        }
    }
    
    /// Update plugin settings
    pub async fn update_settings(&mut self, id: &str, settings: HashMap<String, serde_json::Value>) -> Result<()> {
        if let Some(plugin) = self.plugins.get_mut(id) {
            plugin.settings = settings.clone();
            
            // Save to database
            let state = PluginState {
                plugin_id: id.to_string(),
                enabled: plugin.enabled,
                settings,
            };
            self.repo.save(&state).await?;
            
            info!("Updated settings for plugin: {}", id);
            Ok(())
        } else {
            anyhow::bail!("Plugin not found: {}", id)
        }
    }
    
    /// Delete plugin state from database (used when uninstalling)
    pub async fn delete_state(&self, id: &str) -> Result<bool> {
        self.repo.delete(id).await
    }
    
    /// Reload plugins (rescan directory)
    pub async fn reload(&mut self) -> Result<()> {
        self.plugins.clear();
        self.scan_plugins().await
    }
    
    /// Get combined frontend JS for all enabled plugins
    pub fn get_combined_frontend_js(&self) -> String {
        let mut js = String::new();
        for plugin in self.get_enabled() {
            if let Some(content) = plugin.get_frontend_js() {
                js.push_str(&format!("\n// Plugin: {}\n", plugin.metadata.id));
                js.push_str(&content);
                js.push('\n');
            }
        }
        js
    }
    
    /// Get combined frontend CSS for all enabled plugins
    pub fn get_combined_frontend_css(&self) -> String {
        let mut css = String::new();
        for plugin in self.get_enabled() {
            if let Some(content) = plugin.get_frontend_css() {
                css.push_str(&format!("\n/* Plugin: {} */\n", plugin.metadata.id));
                css.push_str(&content);
                css.push('\n');
            }
        }
        css
    }
    
    /// Get combined admin JS for all enabled plugins
    pub fn get_combined_admin_js(&self) -> String {
        let mut js = String::new();
        for plugin in self.get_enabled() {
            if let Some(content) = plugin.get_admin_js() {
                js.push_str(&format!("\n// Plugin: {}\n", plugin.metadata.id));
                js.push_str(&content);
                js.push('\n');
            }
        }
        js
    }
    
    /// Get combined admin CSS for all enabled plugins
    pub fn get_combined_admin_css(&self) -> String {
        let mut css = String::new();
        for plugin in self.get_enabled() {
            if let Some(content) = plugin.get_admin_css() {
                css.push_str(&format!("\n/* Plugin: {} */\n", plugin.metadata.id));
                css.push_str(&content);
                css.push('\n');
            }
        }
        css
    }
    
    /// Get all shortcodes from enabled plugins
    pub fn get_shortcodes(&self) -> Vec<(String, String)> {
        let mut shortcodes = Vec::new();
        for plugin in self.get_enabled() {
            for shortcode in &plugin.metadata.shortcodes {
                shortcodes.push((shortcode.clone(), plugin.metadata.id.clone()));
            }
        }
        shortcodes
    }
}
