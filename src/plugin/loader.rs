//! Plugin loader and manager
//!
//! Handles plugin discovery, loading, and lifecycle management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn, error};

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

/// Persisted plugin state (stored in data/plugins.json)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginState {
    /// Whether plugin is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Plugin settings values
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
}

/// All plugin states (the entire data/plugins.json file)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginStates(pub HashMap<String, PluginState>);

impl PluginStates {
    /// Load from file
    pub fn load(path: &Path) -> Self {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(states) => return states,
                        Err(e) => warn!("Failed to parse plugins.json: {}", e),
                    }
                }
                Err(e) => warn!("Failed to read plugins.json: {}", e),
            }
        }
        Self::default()
    }
    
    /// Save to file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, &content)?;
        Ok(())
    }
    
    /// Get state for a plugin
    pub fn get(&self, id: &str) -> Option<&PluginState> {
        self.0.get(id)
    }
    
    /// Get or create state for a plugin
    pub fn get_or_default(&mut self, id: &str) -> &mut PluginState {
        self.0.entry(id.to_string()).or_default()
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
    /// Data directory path (for plugins.json)
    data_dir: PathBuf,
    /// Loaded plugins (id -> Plugin)
    plugins: HashMap<String, Plugin>,
    /// Plugin states (persisted)
    states: PluginStates,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(plugins_dir: &Path, data_dir: &Path) -> Self {
        Self {
            plugins_dir: plugins_dir.to_path_buf(),
            data_dir: data_dir.to_path_buf(),
            plugins: HashMap::new(),
            states: PluginStates::default(),
        }
    }
    
    /// Get the path to plugins.json
    fn states_path(&self) -> PathBuf {
        self.data_dir.join("plugins.json")
    }
    
    /// Initialize and load all plugins
    pub fn init(&mut self) -> Result<()> {
        // Ensure plugins directory exists
        if !self.plugins_dir.exists() {
            fs::create_dir_all(&self.plugins_dir)
                .context("Failed to create plugins directory")?;
        }
        
        // Load persisted states
        self.states = PluginStates::load(&self.states_path());
        
        // Scan for plugins
        self.scan_plugins()?;
        
        info!("Loaded {} plugins", self.plugins.len());
        Ok(())
    }
    
    /// Scan plugins directory and load plugin metadata
    fn scan_plugins(&mut self) -> Result<()> {
        let entries = fs::read_dir(&self.plugins_dir)
            .context("Failed to read plugins directory")?;
        
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                match self.load_plugin(&path) {
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
    fn load_plugin(&self, path: &Path) -> Result<Plugin> {
        let plugin_json = path.join("plugin.json");
        
        if !plugin_json.exists() {
            anyhow::bail!("plugin.json not found");
        }
        
        let content = fs::read_to_string(&plugin_json)
            .context("Failed to read plugin.json")?;
        
        let metadata: PluginMetadata = serde_json::from_str(&content)
            .context("Failed to parse plugin.json")?;
        
        // Get persisted state
        let state = self.states.get(&metadata.id);
        let enabled = state.map(|s| s.enabled).unwrap_or(false);
        let settings = state.map(|s| s.settings.clone()).unwrap_or_default();
        
        Ok(Plugin {
            metadata,
            path: path.to_path_buf(),
            enabled,
            settings,
        })
    }
    
    /// Save current states to file
    fn save_states(&self) -> Result<()> {
        self.states.save(&self.states_path())
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
    pub fn enable(&mut self, id: &str) -> Result<()> {
        if let Some(plugin) = self.plugins.get_mut(id) {
            plugin.enabled = true;
            
            // Update persisted state
            let state = self.states.get_or_default(id);
            state.enabled = true;
            self.save_states()?;
            
            info!("Enabled plugin: {}", id);
            Ok(())
        } else {
            anyhow::bail!("Plugin not found: {}", id)
        }
    }
    
    /// Disable a plugin
    pub fn disable(&mut self, id: &str) -> Result<()> {
        if let Some(plugin) = self.plugins.get_mut(id) {
            plugin.enabled = false;
            
            // Update persisted state
            let state = self.states.get_or_default(id);
            state.enabled = false;
            self.save_states()?;
            
            info!("Disabled plugin: {}", id);
            Ok(())
        } else {
            anyhow::bail!("Plugin not found: {}", id)
        }
    }
    
    /// Update plugin settings
    pub fn update_settings(&mut self, id: &str, settings: HashMap<String, serde_json::Value>) -> Result<()> {
        if let Some(plugin) = self.plugins.get_mut(id) {
            plugin.settings = settings.clone();
            
            // Update persisted state
            let state = self.states.get_or_default(id);
            state.settings = settings;
            self.save_states()?;
            
            info!("Updated settings for plugin: {}", id);
            Ok(())
        } else {
            anyhow::bail!("Plugin not found: {}", id)
        }
    }
    
    /// Reload plugins (rescan directory)
    pub fn reload(&mut self) -> Result<()> {
        self.plugins.clear();
        self.states = PluginStates::load(&self.states_path());
        self.scan_plugins()
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
