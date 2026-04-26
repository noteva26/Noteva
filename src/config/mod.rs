//! Configuration management
//!
//! This module handles loading and parsing configuration for the Noteva blog system.
//! Configuration can be loaded from:
//! - config.yml file
//! - Environment variables (override file settings)
//!
//! Missing optional values are filled with sensible defaults.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,
    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,
    /// Cache configuration
    #[serde(default)]
    pub cache: CacheConfig,
    /// Theme configuration
    #[serde(default)]
    pub theme: ThemeConfig,
    /// Upload configuration
    #[serde(default)]
    pub upload: UploadConfig,
    /// Noteva Store URL (e.g. "https://store.noteva.org")
    #[serde(default)]
    pub store_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            cache: CacheConfig::default(),
            theme: ThemeConfig::default(),
            upload: UploadConfig::default(),
            store_url: None,
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host address to bind to
    #[serde(default = "default_host")]
    pub host: String,
    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,
    /// CORS allowed origin (for cookie-based auth)
    #[serde(default = "default_cors_origin")]
    pub cors_origin: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            cors_origin: default_cors_origin(),
        }
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_cors_origin() -> String {
    "http://localhost:3000".to_string()
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database driver (sqlite or mysql)
    #[serde(default)]
    pub driver: DatabaseDriver,
    /// Database connection URL
    #[serde(default = "default_database_url")]
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            driver: DatabaseDriver::default(),
            url: default_database_url(),
        }
    }
}

fn default_database_url() -> String {
    "data/noteva.db".to_string()
}

/// Database driver type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseDriver {
    /// SQLite (default)
    #[default]
    Sqlite,
    /// MySQL
    Mysql,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache driver (memory or redis)
    #[serde(default)]
    pub driver: CacheDriver,
    /// Redis connection URL (optional)
    #[serde(default)]
    pub redis_url: Option<String>,
    /// Cache TTL in seconds
    #[serde(default = "default_ttl")]
    pub ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            driver: CacheDriver::default(),
            redis_url: None,
            ttl_seconds: default_ttl(),
        }
    }
}

fn default_ttl() -> u64 {
    3600
}

/// Cache driver type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CacheDriver {
    /// In-memory cache (default)
    #[default]
    Memory,
    /// Redis cache
    Redis,
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Active theme name
    #[serde(default = "default_theme")]
    pub active: String,
    /// Path to themes directory
    #[serde(default = "default_theme_path")]
    pub path: PathBuf,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            active: default_theme(),
            path: default_theme_path(),
        }
    }
}

fn default_theme() -> String {
    "default".to_string()
}

fn default_theme_path() -> PathBuf {
    PathBuf::from("themes")
}

/// Upload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadConfig {
    /// Upload directory path
    #[serde(default = "default_upload_path")]
    pub path: PathBuf,
    /// Maximum file size in bytes (default: 10MB)
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    /// Maximum plugin file size in bytes (default: 50MB)
    #[serde(default = "default_max_plugin_file_size")]
    pub max_plugin_file_size: u64,
    /// Allowed image MIME types
    #[serde(default = "default_allowed_types")]
    pub allowed_types: Vec<String>,
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            path: default_upload_path(),
            max_file_size: default_max_file_size(),
            max_plugin_file_size: default_max_plugin_file_size(),
            allowed_types: default_allowed_types(),
        }
    }
}

fn default_upload_path() -> PathBuf {
    PathBuf::from("uploads")
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024 // 10MB
}

fn default_max_plugin_file_size() -> u64 {
    50 * 1024 * 1024 // 50MB
}

fn default_allowed_types() -> Vec<String> {
    vec![
        "image/jpeg".to_string(),
        "image/png".to_string(),
        "image/gif".to_string(),
        "image/webp".to_string(),
        "image/svg+xml".to_string(),
    ]
}

impl UploadConfig {
    /// Check if a MIME type is allowed
    pub fn is_type_allowed(&self, mime_type: &str) -> bool {
        self.allowed_types.iter().any(|t| t == mime_type)
    }

    /// Get file extension for a MIME type
    pub fn get_extension(&self, mime_type: &str) -> &'static str {
        match mime_type {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            "image/bmp" => "bmp",
            "image/tiff" => "tiff",
            "image/x-icon" => "ico",
            _ => "bin",
        }
    }
}

/// Error type for configuration parsing
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file '{path}': {source}")]
    FileRead {
        path: String,
        source: std::io::Error,
    },
    #[error("Failed to parse config file '{path}': {message}")]
    ParseError { path: String, message: String },
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
}

impl Config {
    /// Load configuration from file
    ///
    /// If the file doesn't exist, returns default configuration.
    /// If the file exists but is invalid YAML, returns an error with details.
    ///
    /// Satisfies requirements:
    /// - 8.1: WHEN 系统启动 THEN Config_Manager SHALL 读取并解�?config.yml 配置文件
    /// - 8.5: WHEN 配置项缺�?THEN Config_Manager SHALL 使用合理的默认�?
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        // If file doesn't exist, return defaults (requirement 8.5)
        if !path.exists() {
            return Ok(Self::default());
        }

        // Read the file content
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::FileRead {
            path: path.display().to_string(),
            source: e,
        })?;

        // Handle empty file - return defaults
        if content.trim().is_empty() {
            return Ok(Self::default());
        }

        // Parse YAML with detailed error messages (requirement 8.4)
        let config: Config =
            serde_yaml::from_str(&content).map_err(|e| ConfigError::ParseError {
                path: path.display().to_string(),
                message: format_yaml_error(&e),
            })?;

        Ok(config)
    }

    /// Load configuration from file with environment variable overrides
    ///
    /// Environment variables follow the pattern:
    /// - NOTEVA_SERVER_HOST
    /// - NOTEVA_SERVER_PORT
    /// - NOTEVA_DATABASE_DRIVER
    /// - NOTEVA_DATABASE_URL
    /// - NOTEVA_CACHE_DRIVER
    /// - NOTEVA_CACHE_REDIS_URL
    /// - NOTEVA_CACHE_TTL_SECONDS
    /// - NOTEVA_THEME_ACTIVE
    /// - NOTEVA_THEME_PATH
    ///
    /// Satisfies requirement:
    /// - 11.5: THE Noteva_System SHALL 支持通过环境变量覆盖配置�?
    pub fn load_with_env(path: &std::path::Path) -> anyhow::Result<Self> {
        // First load from file (or defaults)
        let mut config = Self::load(path)?;

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides to the configuration
    fn apply_env_overrides(&mut self) {
        // Server configuration
        if let Ok(host) = std::env::var("NOTEVA_SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(port) = std::env::var("NOTEVA_SERVER_PORT") {
            if let Ok(port) = port.parse::<u16>() {
                self.server.port = port;
            }
        }
        if let Ok(cors_origin) = std::env::var("NOTEVA_SERVER_CORS_ORIGIN") {
            self.server.cors_origin = cors_origin;
        }

        // Database configuration
        if let Ok(driver) = std::env::var("NOTEVA_DATABASE_DRIVER") {
            match driver.to_lowercase().as_str() {
                "sqlite" => self.database.driver = DatabaseDriver::Sqlite,
                "mysql" => self.database.driver = DatabaseDriver::Mysql,
                _ => {} // Ignore invalid values
            }
        }
        if let Ok(url) = std::env::var("NOTEVA_DATABASE_URL") {
            self.database.url = url;
        }

        // Cache configuration
        if let Ok(driver) = std::env::var("NOTEVA_CACHE_DRIVER") {
            match driver.to_lowercase().as_str() {
                "memory" => self.cache.driver = CacheDriver::Memory,
                "redis" => self.cache.driver = CacheDriver::Redis,
                _ => {} // Ignore invalid values
            }
        }
        if let Ok(redis_url) = std::env::var("NOTEVA_CACHE_REDIS_URL") {
            self.cache.redis_url = Some(redis_url);
        }
        if let Ok(ttl) = std::env::var("NOTEVA_CACHE_TTL_SECONDS") {
            if let Ok(ttl) = ttl.parse::<u64>() {
                self.cache.ttl_seconds = ttl;
            }
        }

        // Theme configuration
        if let Ok(active) = std::env::var("NOTEVA_THEME_ACTIVE") {
            self.theme.active = active;
        }
        if let Ok(path) = std::env::var("NOTEVA_THEME_PATH") {
            self.theme.path = PathBuf::from(path);
        }
    }
}

/// Format YAML parsing error with location and context
fn format_yaml_error(e: &serde_yaml::Error) -> String {
    if let Some(location) = e.location() {
        format!(
            "at line {}, column {}: {}",
            location.line(),
            location.column(),
            e
        )
    } else {
        e.to_string()
    }
}

// Shared mutex for all config tests that modify environment variables.
// Both `tests` and `property_tests` modules use this to prevent race conditions.

// Shared mutex for all config tests that modify environment variables.
#[cfg(test)]
static CONFIG_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests;

#[cfg(test)]
mod property_tests;
