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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            cache: CacheConfig::default(),
            theme: ThemeConfig::default(),
            upload: UploadConfig::default(),
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
    ParseError {
        path: String,
        message: String,
    },
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
        let config: Config = serde_yaml::from_str(&content).map_err(|e| {
            ConfigError::ParseError {
                path: path.display().to_string(),
                message: format_yaml_error(&e),
            }
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
#[cfg(test)]
static CONFIG_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        super::CONFIG_ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn test_load_missing_file_returns_defaults() {
        let path = std::path::Path::new("nonexistent_config.yml");
        let config = Config::load(path).unwrap();
        
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.driver, DatabaseDriver::Sqlite);
        assert_eq!(config.database.url, "data/noteva.db");
        assert_eq!(config.cache.driver, CacheDriver::Memory);
        assert_eq!(config.cache.ttl_seconds, 3600);
        assert_eq!(config.theme.active, "default");
        assert_eq!(config.theme.path, PathBuf::from("themes"));
    }

    #[test]
    fn test_load_empty_file_returns_defaults() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "").unwrap();
        
        let config = Config::load(file.path()).unwrap();
        
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
    }

    #[test]
    fn test_load_partial_config_fills_defaults() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "server:\n  port: 3000\n").unwrap();
        
        let config = Config::load(file.path()).unwrap();
        
        // Specified value
        assert_eq!(config.server.port, 3000);
        // Default values
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.database.driver, DatabaseDriver::Sqlite);
    }

    #[test]
    fn test_load_full_config() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"
server:
  host: "127.0.0.1"
  port: 9000
database:
  driver: mysql
  url: "mysql://user:pass@localhost/noteva"
cache:
  driver: redis
  redis_url: "redis://localhost:6379"
  ttl_seconds: 7200
theme:
  active: "custom"
  path: "custom_themes"
"#).unwrap();
        
        let config = Config::load(file.path()).unwrap();
        
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.database.driver, DatabaseDriver::Mysql);
        assert_eq!(config.database.url, "mysql://user:pass@localhost/noteva");
        assert_eq!(config.cache.driver, CacheDriver::Redis);
        assert_eq!(config.cache.redis_url, Some("redis://localhost:6379".to_string()));
        assert_eq!(config.cache.ttl_seconds, 7200);
        assert_eq!(config.theme.active, "custom");
        assert_eq!(config.theme.path, PathBuf::from("custom_themes"));
    }

    #[test]
    fn test_load_invalid_yaml_returns_error() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "server:\n  port: not_a_number\n").unwrap();
        
        let result = Config::load(file.path());
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("parse") || err_msg.contains("invalid"));
    }

    #[test]
    fn test_load_malformed_yaml_returns_error() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "server:\n  host: [invalid yaml").unwrap();
        
        let result = Config::load(file.path());
        
        assert!(result.is_err());
    }

    #[test]
    fn test_env_override_server_config() {
        let _guard = lock_env();
        
        // Clean up any leftover env vars first
        std::env::remove_var("NOTEVA_SERVER_HOST");
        std::env::remove_var("NOTEVA_SERVER_PORT");
        std::env::remove_var("NOTEVA_DATABASE_DRIVER");
        std::env::remove_var("NOTEVA_DATABASE_URL");
        std::env::remove_var("NOTEVA_CACHE_DRIVER");
        std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
        std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
        std::env::remove_var("NOTEVA_THEME_ACTIVE");
        std::env::remove_var("NOTEVA_THEME_PATH");
        
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "server:\n  host: \"0.0.0.0\"\n  port: 8080\n").unwrap();
        
        // Set environment variables
        std::env::set_var("NOTEVA_SERVER_HOST", "192.168.1.1");
        std::env::set_var("NOTEVA_SERVER_PORT", "4000");
        
        let config = Config::load_with_env(file.path()).unwrap();
        
        assert_eq!(config.server.host, "192.168.1.1");
        assert_eq!(config.server.port, 4000);
        
        // Clean up
        std::env::remove_var("NOTEVA_SERVER_HOST");
        std::env::remove_var("NOTEVA_SERVER_PORT");
    }

    #[test]
    fn test_env_override_database_config() {
        let _guard = lock_env();
        
        // Clean up any leftover env vars first
        std::env::remove_var("NOTEVA_SERVER_HOST");
        std::env::remove_var("NOTEVA_SERVER_PORT");
        std::env::remove_var("NOTEVA_DATABASE_DRIVER");
        std::env::remove_var("NOTEVA_DATABASE_URL");
        std::env::remove_var("NOTEVA_CACHE_DRIVER");
        std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
        std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
        std::env::remove_var("NOTEVA_THEME_ACTIVE");
        std::env::remove_var("NOTEVA_THEME_PATH");
        
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "").unwrap();
        
        std::env::set_var("NOTEVA_DATABASE_DRIVER", "mysql");
        std::env::set_var("NOTEVA_DATABASE_URL", "mysql://test@localhost/db");
        
        let config = Config::load_with_env(file.path()).unwrap();
        
        assert_eq!(config.database.driver, DatabaseDriver::Mysql);
        assert_eq!(config.database.url, "mysql://test@localhost/db");
        
        // Clean up
        std::env::remove_var("NOTEVA_DATABASE_DRIVER");
        std::env::remove_var("NOTEVA_DATABASE_URL");
    }

    #[test]
    fn test_env_override_cache_config() {
        let _guard = lock_env();
        
        // Clean up any leftover env vars first
        std::env::remove_var("NOTEVA_SERVER_HOST");
        std::env::remove_var("NOTEVA_SERVER_PORT");
        std::env::remove_var("NOTEVA_DATABASE_DRIVER");
        std::env::remove_var("NOTEVA_DATABASE_URL");
        std::env::remove_var("NOTEVA_CACHE_DRIVER");
        std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
        std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
        std::env::remove_var("NOTEVA_THEME_ACTIVE");
        std::env::remove_var("NOTEVA_THEME_PATH");
        
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "").unwrap();
        
        std::env::set_var("NOTEVA_CACHE_DRIVER", "redis");
        std::env::set_var("NOTEVA_CACHE_REDIS_URL", "redis://localhost:6379");
        std::env::set_var("NOTEVA_CACHE_TTL_SECONDS", "1800");
        
        let config = Config::load_with_env(file.path()).unwrap();
        
        assert_eq!(config.cache.driver, CacheDriver::Redis);
        assert_eq!(config.cache.redis_url, Some("redis://localhost:6379".to_string()));
        assert_eq!(config.cache.ttl_seconds, 1800);
        
        // Clean up
        std::env::remove_var("NOTEVA_CACHE_DRIVER");
        std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
        std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
    }

    #[test]
    fn test_env_override_theme_config() {
        let _guard = lock_env();
        
        // Clean up any leftover env vars first
        std::env::remove_var("NOTEVA_SERVER_HOST");
        std::env::remove_var("NOTEVA_SERVER_PORT");
        std::env::remove_var("NOTEVA_DATABASE_DRIVER");
        std::env::remove_var("NOTEVA_DATABASE_URL");
        std::env::remove_var("NOTEVA_CACHE_DRIVER");
        std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
        std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
        std::env::remove_var("NOTEVA_THEME_ACTIVE");
        std::env::remove_var("NOTEVA_THEME_PATH");
        
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "").unwrap();
        
        std::env::set_var("NOTEVA_THEME_ACTIVE", "dark");
        std::env::set_var("NOTEVA_THEME_PATH", "/var/themes");
        
        let config = Config::load_with_env(file.path()).unwrap();
        
        assert_eq!(config.theme.active, "dark");
        assert_eq!(config.theme.path, PathBuf::from("/var/themes"));
        
        // Clean up
        std::env::remove_var("NOTEVA_THEME_ACTIVE");
        std::env::remove_var("NOTEVA_THEME_PATH");
    }

    #[test]
    fn test_env_override_invalid_port_ignored() {
        let _guard = lock_env();
        
        // Clean up any leftover env vars first
        std::env::remove_var("NOTEVA_SERVER_HOST");
        std::env::remove_var("NOTEVA_SERVER_PORT");
        std::env::remove_var("NOTEVA_DATABASE_DRIVER");
        std::env::remove_var("NOTEVA_DATABASE_URL");
        std::env::remove_var("NOTEVA_CACHE_DRIVER");
        std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
        std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
        std::env::remove_var("NOTEVA_THEME_ACTIVE");
        std::env::remove_var("NOTEVA_THEME_PATH");
        
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "server:\n  port: 8080\n").unwrap();
        
        std::env::set_var("NOTEVA_SERVER_PORT", "not_a_number");
        
        let config = Config::load_with_env(file.path()).unwrap();
        
        // Should keep original value when env var is invalid
        assert_eq!(config.server.port, 8080);
        
        // Clean up
        std::env::remove_var("NOTEVA_SERVER_PORT");
    }

    #[test]
    fn test_env_override_invalid_driver_ignored() {
        let _guard = lock_env();
        
        // Clean up any leftover env vars first
        std::env::remove_var("NOTEVA_SERVER_HOST");
        std::env::remove_var("NOTEVA_SERVER_PORT");
        std::env::remove_var("NOTEVA_DATABASE_DRIVER");
        std::env::remove_var("NOTEVA_DATABASE_URL");
        std::env::remove_var("NOTEVA_CACHE_DRIVER");
        std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
        std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
        std::env::remove_var("NOTEVA_THEME_ACTIVE");
        std::env::remove_var("NOTEVA_THEME_PATH");
        
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "database:\n  driver: sqlite\n").unwrap();
        
        std::env::set_var("NOTEVA_DATABASE_DRIVER", "invalid_driver");
        
        let config = Config::load_with_env(file.path()).unwrap();
        
        // Should keep original value when env var is invalid
        assert_eq!(config.database.driver, DatabaseDriver::Sqlite);
        
        // Clean up
        std::env::remove_var("NOTEVA_DATABASE_DRIVER");
    }
}


/// Property-based tests for configuration parsing
/// 
/// These tests verify the correctness properties defined in the design document:
/// - Property 19: 配置解析往�?(Config roundtrip)
/// - Property 20: 配置默认值填�?(Default value filling)
/// - Property 21: 无效配置错误处理 (Invalid config error handling)
/// - Property 26: 环境变量覆盖 (Environment variable override)
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        super::CONFIG_ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
    }

    // ============================================================================
    // Strategies for generating test data
    // ============================================================================

    /// Strategy for generating valid host strings
    fn valid_host_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // IPv4 addresses
            (0u8..=255, 0u8..=255, 0u8..=255, 0u8..=255)
                .prop_map(|(a, b, c, d)| format!("{}.{}.{}.{}", a, b, c, d)),
            // Common hostnames
            Just("localhost".to_string()),
            Just("0.0.0.0".to_string()),
            Just("127.0.0.1".to_string()),
            // Simple alphanumeric hostnames
            "[a-z][a-z0-9]{0,10}".prop_map(|s| s),
        ]
    }

    /// Strategy for generating valid port numbers
    fn valid_port_strategy() -> impl Strategy<Value = u16> {
        1u16..=65535
    }

    /// Strategy for generating valid database drivers
    fn valid_database_driver_strategy() -> impl Strategy<Value = DatabaseDriver> {
        prop_oneof![
            Just(DatabaseDriver::Sqlite),
            Just(DatabaseDriver::Mysql),
        ]
    }

    /// Strategy for generating valid database URLs
    fn valid_database_url_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // SQLite paths
            "[a-z][a-z0-9_/]{0,20}\\.db".prop_map(|s| s),
            Just("data/noteva.db".to_string()),
            Just(":memory:".to_string()),
            // MySQL URLs
            Just("mysql://user:pass@localhost/db".to_string()),
            Just("mysql://root@127.0.0.1:3306/noteva".to_string()),
        ]
    }

    /// Strategy for generating valid cache drivers
    fn valid_cache_driver_strategy() -> impl Strategy<Value = CacheDriver> {
        prop_oneof![
            Just(CacheDriver::Memory),
            Just(CacheDriver::Redis),
        ]
    }

    /// Strategy for generating optional Redis URLs
    fn valid_redis_url_strategy() -> impl Strategy<Value = Option<String>> {
        prop_oneof![
            Just(None),
            Just(Some("redis://localhost:6379".to_string())),
            Just(Some("redis://127.0.0.1:6379/0".to_string())),
        ]
    }

    /// Strategy for generating valid TTL values
    fn valid_ttl_strategy() -> impl Strategy<Value = u64> {
        1u64..=86400 // 1 second to 24 hours
    }

    /// Strategy for generating valid theme names
    fn valid_theme_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("default".to_string()),
            Just("dark".to_string()),
            Just("light".to_string()),
            "[a-z][a-z0-9_-]{0,15}".prop_map(|s| s),
        ]
    }

    /// Strategy for generating valid theme paths
    fn valid_theme_path_strategy() -> impl Strategy<Value = PathBuf> {
        prop_oneof![
            Just(PathBuf::from("themes")),
            Just(PathBuf::from("custom_themes")),
            "[a-z][a-z0-9_/]{0,20}".prop_map(|s| PathBuf::from(s)),
        ]
    }

    /// Strategy for generating valid ServerConfig
    fn valid_server_config_strategy() -> impl Strategy<Value = ServerConfig> {
        (valid_host_strategy(), valid_port_strategy())
            .prop_map(|(host, port)| ServerConfig { 
                host, 
                port,
                cors_origin: "http://localhost:3000".to_string(),
            })
    }

    /// Strategy for generating valid DatabaseConfig
    fn valid_database_config_strategy() -> impl Strategy<Value = DatabaseConfig> {
        (valid_database_driver_strategy(), valid_database_url_strategy())
            .prop_map(|(driver, url)| DatabaseConfig { driver, url })
    }

    /// Strategy for generating valid CacheConfig
    fn valid_cache_config_strategy() -> impl Strategy<Value = CacheConfig> {
        (valid_cache_driver_strategy(), valid_redis_url_strategy(), valid_ttl_strategy())
            .prop_map(|(driver, redis_url, ttl_seconds)| CacheConfig {
                driver,
                redis_url,
                ttl_seconds,
            })
    }

    /// Strategy for generating valid ThemeConfig
    fn valid_theme_config_strategy() -> impl Strategy<Value = ThemeConfig> {
        (valid_theme_name_strategy(), valid_theme_path_strategy())
            .prop_map(|(active, path)| ThemeConfig { active, path })
    }

    /// Strategy for generating valid Config structures
    fn valid_config_strategy() -> impl Strategy<Value = Config> {
        (
            valid_server_config_strategy(),
            valid_database_config_strategy(),
            valid_cache_config_strategy(),
            valid_theme_config_strategy(),
        )
            .prop_map(|(server, database, cache, theme)| Config {
                server,
                database,
                cache,
                theme,
                upload: UploadConfig::default(),
            })
    }

    /// Strategy for generating malformed YAML strings that will fail to parse as Config
    /// 
    /// These are YAML strings that are either:
    /// 1. Syntactically invalid YAML
    /// 2. Valid YAML but with wrong types for Config fields
    fn malformed_yaml_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Invalid type for port (must be a number, not a string or other type)
            Just("server:\n  port: not_a_number".to_string()),
            Just("server:\n  port: \"8080\"".to_string()),  // String instead of number
            Just("server:\n  port: true".to_string()),
            Just("server:\n  port: [1, 2, 3]".to_string()),
            Just("server:\n  port: {key: value}".to_string()),
            Just("server:\n  port: 99999999999999999999".to_string()), // Overflow
            // Invalid type for ttl_seconds (must be a number)
            Just("cache:\n  ttl_seconds: invalid".to_string()),
            Just("cache:\n  ttl_seconds: \"3600\"".to_string()),
            Just("cache:\n  ttl_seconds: false".to_string()),
            Just("cache:\n  ttl_seconds: -100".to_string()), // Negative for u64
            // Invalid driver values (must be sqlite/mysql or memory/redis)
            Just("database:\n  driver: postgres".to_string()),
            Just("database:\n  driver: mongodb".to_string()),
            Just("database:\n  driver: 123".to_string()),
            Just("cache:\n  driver: memcached".to_string()),
            Just("cache:\n  driver: dynamodb".to_string()),
            // Invalid nested structure (expecting object, got scalar/array)
            Just("server: [invalid, list, for, server]".to_string()),
            Just("server: 12345".to_string()),
            Just("database: \"just_a_string\"".to_string()),
            Just("cache: true".to_string()),
            Just("theme: null".to_string()),
        ]
    }

    /// Strategy for generating partial config YAML (missing some fields)
    fn partial_config_yaml_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Only server section
            (valid_host_strategy(), valid_port_strategy())
                .prop_map(|(host, port)| format!("server:\n  host: \"{}\"\n  port: {}\n", host, port)),
            // Only database section
            Just("database:\n  driver: sqlite\n  url: \"test.db\"\n".to_string()),
            // Only cache section
            Just("cache:\n  driver: memory\n  ttl_seconds: 1800\n".to_string()),
            // Only theme section
            Just("theme:\n  active: \"dark\"\n  path: \"themes\"\n".to_string()),
            // Server with partial fields
            Just("server:\n  port: 9000\n".to_string()),
            // Database with partial fields
            Just("database:\n  driver: mysql\n".to_string()),
            // Cache with partial fields
            Just("cache:\n  ttl_seconds: 7200\n".to_string()),
            // Theme with partial fields
            Just("theme:\n  active: \"custom\"\n".to_string()),
            // Empty config
            Just("".to_string()),
            // Whitespace only
            Just("   \n\n   ".to_string()),
        ]
    }

    /// Strategy for generating environment variable values for server config
    fn env_server_value_strategy() -> impl Strategy<Value = (String, String)> {
        prop_oneof![
            valid_host_strategy().prop_map(|h| ("NOTEVA_SERVER_HOST".to_string(), h)),
            valid_port_strategy().prop_map(|p| ("NOTEVA_SERVER_PORT".to_string(), p.to_string())),
        ]
    }

    /// Strategy for generating environment variable values for database config
    fn env_database_value_strategy() -> impl Strategy<Value = (String, String)> {
        prop_oneof![
            valid_database_driver_strategy()
                .prop_map(|d| ("NOTEVA_DATABASE_DRIVER".to_string(), 
                    match d { DatabaseDriver::Sqlite => "sqlite", DatabaseDriver::Mysql => "mysql" }.to_string())),
            valid_database_url_strategy().prop_map(|u| ("NOTEVA_DATABASE_URL".to_string(), u)),
        ]
    }

    /// Strategy for generating environment variable values for cache config
    fn env_cache_value_strategy() -> impl Strategy<Value = (String, String)> {
        prop_oneof![
            valid_cache_driver_strategy()
                .prop_map(|d| ("NOTEVA_CACHE_DRIVER".to_string(),
                    match d { CacheDriver::Memory => "memory", CacheDriver::Redis => "redis" }.to_string())),
            valid_ttl_strategy().prop_map(|t| ("NOTEVA_CACHE_TTL_SECONDS".to_string(), t.to_string())),
        ]
    }

    /// Strategy for generating environment variable values for theme config
    fn env_theme_value_strategy() -> impl Strategy<Value = (String, String)> {
        prop_oneof![
            valid_theme_name_strategy().prop_map(|n| ("NOTEVA_THEME_ACTIVE".to_string(), n)),
            valid_theme_path_strategy().prop_map(|p| ("NOTEVA_THEME_PATH".to_string(), p.display().to_string())),
        ]
    }

    // ============================================================================
    // Property Tests
    // ============================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Property 19: 配置解析往�?*
        /// **Validates: Requirements 8.1**
        /// 
        /// For any valid config structure, serializing to YAML and parsing back 
        /// should yield equivalent config.
        #[test]
        fn property_19_config_roundtrip(config in valid_config_strategy()) {
            // Serialize config to YAML
            let yaml = serde_yaml::to_string(&config).expect("Failed to serialize config");
            
            // Write to temp file
            let mut file = NamedTempFile::new().expect("Failed to create temp file");
            write!(file, "{}", yaml).expect("Failed to write config");
            
            // Parse back
            let parsed = Config::load(file.path()).expect("Failed to parse config");
            
            // Verify equivalence
            prop_assert_eq!(config.server.host, parsed.server.host);
            prop_assert_eq!(config.server.port, parsed.server.port);
            prop_assert_eq!(config.database.driver, parsed.database.driver);
            prop_assert_eq!(config.database.url, parsed.database.url);
            prop_assert_eq!(config.cache.driver, parsed.cache.driver);
            prop_assert_eq!(config.cache.redis_url, parsed.cache.redis_url);
            prop_assert_eq!(config.cache.ttl_seconds, parsed.cache.ttl_seconds);
            prop_assert_eq!(config.theme.active, parsed.theme.active);
            prop_assert_eq!(config.theme.path, parsed.theme.path);
        }

        /// **Property 20: 配置默认值填�?*
        /// **Validates: Requirements 8.5**
        /// 
        /// For any config file missing optional items, parsing should fill 
        /// with predefined defaults.
        #[test]
        fn property_20_config_default_filling(yaml in partial_config_yaml_strategy()) {
            // Write partial config to temp file
            let mut file = NamedTempFile::new().expect("Failed to create temp file");
            write!(file, "{}", yaml).expect("Failed to write config");
            
            // Parse config
            let config = Config::load(file.path()).expect("Failed to parse config");
            
            // Verify defaults are applied for missing fields
            // Server defaults
            prop_assert!(!config.server.host.is_empty(), "Host should not be empty");
            prop_assert!(config.server.port > 0, "Port should be positive");
            
            // Database defaults
            prop_assert!(!config.database.url.is_empty(), "Database URL should not be empty");
            
            // Cache defaults
            prop_assert!(config.cache.ttl_seconds > 0, "TTL should be positive");
            
            // Theme defaults
            prop_assert!(!config.theme.active.is_empty(), "Theme name should not be empty");
            
            // If the YAML was empty or whitespace-only, verify all defaults
            if yaml.trim().is_empty() {
                prop_assert_eq!(config.server.host, "0.0.0.0");
                prop_assert_eq!(config.server.port, 8080);
                prop_assert_eq!(config.database.driver, DatabaseDriver::Sqlite);
                prop_assert_eq!(config.database.url, "data/noteva.db");
                prop_assert_eq!(config.cache.driver, CacheDriver::Memory);
                prop_assert_eq!(config.cache.ttl_seconds, 3600);
                prop_assert_eq!(config.theme.active, "default");
                prop_assert_eq!(config.theme.path, PathBuf::from("themes"));
            }
        }

        /// **Property 21: 无效配置错误处理**
        /// **Validates: Requirements 8.4**
        /// 
        /// For any malformed config file, parsing should return detailed error 
        /// with location and reason.
        #[test]
        fn property_21_invalid_config_error_handling(yaml in malformed_yaml_strategy()) {
            // Write malformed config to temp file
            let mut file = NamedTempFile::new().expect("Failed to create temp file");
            write!(file, "{}", yaml).expect("Failed to write config");
            
            // Attempt to parse
            let result = Config::load(file.path());
            
            // Should return an error
            prop_assert!(result.is_err(), "Malformed YAML should produce an error");
            
            // Error message should be descriptive
            let err = result.unwrap_err();
            let err_msg = err.to_string();
            
            // Error should contain useful information
            prop_assert!(
                err_msg.len() > 10,
                "Error message should be descriptive: {}",
                err_msg
            );
        }

        /// **Property 26: 环境变量覆盖 - Server Config**
        /// **Validates: Requirements 11.5**
        /// 
        /// For any config item, setting corresponding env var should override 
        /// the file value.
        #[test]
        fn property_26_env_override_server((env_key, env_value) in env_server_value_strategy()) {
            let _guard = lock_env();
            
            // Clean up any leftover env vars first
            std::env::remove_var("NOTEVA_SERVER_HOST");
            std::env::remove_var("NOTEVA_SERVER_PORT");
            std::env::remove_var("NOTEVA_DATABASE_DRIVER");
            std::env::remove_var("NOTEVA_DATABASE_URL");
            std::env::remove_var("NOTEVA_CACHE_DRIVER");
            std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
            std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
            std::env::remove_var("NOTEVA_THEME_ACTIVE");
            std::env::remove_var("NOTEVA_THEME_PATH");
            
            // Create a config file with default values
            let mut file = NamedTempFile::new().expect("Failed to create temp file");
            write!(file, "server:\n  host: \"original_host\"\n  port: 1234\n").expect("Failed to write config");
            
            // Set environment variable
            std::env::set_var(&env_key, &env_value);
            
            // Load config with env overrides
            let config = Config::load_with_env(file.path()).expect("Failed to load config");
            
            // Verify override was applied
            match env_key.as_str() {
                "NOTEVA_SERVER_HOST" => {
                    prop_assert_eq!(config.server.host, env_value);
                }
                "NOTEVA_SERVER_PORT" => {
                    let expected_port: u16 = env_value.parse().expect("Invalid port");
                    prop_assert_eq!(config.server.port, expected_port);
                }
                _ => {}
            }
            
            // Clean up
            std::env::remove_var(&env_key);
        }

        /// **Property 26: 环境变量覆盖 - Database Config**
        /// **Validates: Requirements 11.5**
        #[test]
        fn property_26_env_override_database((env_key, env_value) in env_database_value_strategy()) {
            let _guard = lock_env();
            
            // Clean up any leftover env vars first
            std::env::remove_var("NOTEVA_SERVER_HOST");
            std::env::remove_var("NOTEVA_SERVER_PORT");
            std::env::remove_var("NOTEVA_DATABASE_DRIVER");
            std::env::remove_var("NOTEVA_DATABASE_URL");
            std::env::remove_var("NOTEVA_CACHE_DRIVER");
            std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
            std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
            std::env::remove_var("NOTEVA_THEME_ACTIVE");
            std::env::remove_var("NOTEVA_THEME_PATH");
            
            // Create a config file with default values
            let mut file = NamedTempFile::new().expect("Failed to create temp file");
            write!(file, "database:\n  driver: sqlite\n  url: \"original.db\"\n").expect("Failed to write config");
            
            // Set environment variable
            std::env::set_var(&env_key, &env_value);
            
            // Load config with env overrides
            let config = Config::load_with_env(file.path()).expect("Failed to load config");
            
            // Verify override was applied
            match env_key.as_str() {
                "NOTEVA_DATABASE_DRIVER" => {
                    let expected_driver = match env_value.as_str() {
                        "sqlite" => DatabaseDriver::Sqlite,
                        "mysql" => DatabaseDriver::Mysql,
                        _ => panic!("Invalid driver"),
                    };
                    prop_assert_eq!(config.database.driver, expected_driver);
                }
                "NOTEVA_DATABASE_URL" => {
                    prop_assert_eq!(config.database.url, env_value);
                }
                _ => {}
            }
            
            // Clean up
            std::env::remove_var(&env_key);
        }

        /// **Property 26: 环境变量覆盖 - Cache Config**
        /// **Validates: Requirements 11.5**
        #[test]
        fn property_26_env_override_cache((env_key, env_value) in env_cache_value_strategy()) {
            let _guard = lock_env();
            
            // Clean up any leftover env vars first
            std::env::remove_var("NOTEVA_SERVER_HOST");
            std::env::remove_var("NOTEVA_SERVER_PORT");
            std::env::remove_var("NOTEVA_DATABASE_DRIVER");
            std::env::remove_var("NOTEVA_DATABASE_URL");
            std::env::remove_var("NOTEVA_CACHE_DRIVER");
            std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
            std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
            std::env::remove_var("NOTEVA_THEME_ACTIVE");
            std::env::remove_var("NOTEVA_THEME_PATH");
            
            // Create a config file with default values
            let mut file = NamedTempFile::new().expect("Failed to create temp file");
            write!(file, "cache:\n  driver: memory\n  ttl_seconds: 1000\n").expect("Failed to write config");
            
            // Set environment variable
            std::env::set_var(&env_key, &env_value);
            
            // Load config with env overrides
            let config = Config::load_with_env(file.path()).expect("Failed to load config");
            
            // Verify override was applied
            match env_key.as_str() {
                "NOTEVA_CACHE_DRIVER" => {
                    let expected_driver = match env_value.as_str() {
                        "memory" => CacheDriver::Memory,
                        "redis" => CacheDriver::Redis,
                        _ => panic!("Invalid driver"),
                    };
                    prop_assert_eq!(config.cache.driver, expected_driver);
                }
                "NOTEVA_CACHE_TTL_SECONDS" => {
                    let expected_ttl: u64 = env_value.parse().expect("Invalid TTL");
                    prop_assert_eq!(config.cache.ttl_seconds, expected_ttl);
                }
                _ => {}
            }
            
            // Clean up
            std::env::remove_var(&env_key);
        }

        /// **Property 26: 环境变量覆盖 - Theme Config**
        /// **Validates: Requirements 11.5**
        #[test]
        fn property_26_env_override_theme((env_key, env_value) in env_theme_value_strategy()) {
            let _guard = lock_env();
            
            // Clean up any leftover env vars first
            std::env::remove_var("NOTEVA_SERVER_HOST");
            std::env::remove_var("NOTEVA_SERVER_PORT");
            std::env::remove_var("NOTEVA_DATABASE_DRIVER");
            std::env::remove_var("NOTEVA_DATABASE_URL");
            std::env::remove_var("NOTEVA_CACHE_DRIVER");
            std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
            std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
            std::env::remove_var("NOTEVA_THEME_ACTIVE");
            std::env::remove_var("NOTEVA_THEME_PATH");
            
            // Create a config file with default values
            let mut file = NamedTempFile::new().expect("Failed to create temp file");
            write!(file, "theme:\n  active: \"original\"\n  path: \"original_path\"\n").expect("Failed to write config");
            
            // Set environment variable
            std::env::set_var(&env_key, &env_value);
            
            // Load config with env overrides
            let config = Config::load_with_env(file.path()).expect("Failed to load config");
            
            // Verify override was applied
            match env_key.as_str() {
                "NOTEVA_THEME_ACTIVE" => {
                    prop_assert_eq!(config.theme.active, env_value);
                }
                "NOTEVA_THEME_PATH" => {
                    prop_assert_eq!(config.theme.path, PathBuf::from(&env_value));
                }
                _ => {}
            }
            
            // Clean up
            std::env::remove_var(&env_key);
        }
    }

    // ============================================================================
    // Additional Property Tests for Edge Cases
    // ============================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Property 19 Extension: Config serialization preserves all fields**
        /// **Validates: Requirements 8.1**
        #[test]
        fn property_19_serialization_preserves_fields(
            host in valid_host_strategy(),
            port in valid_port_strategy(),
            db_driver in valid_database_driver_strategy(),
            db_url in valid_database_url_strategy(),
            cache_driver in valid_cache_driver_strategy(),
            ttl in valid_ttl_strategy(),
            theme_name in valid_theme_name_strategy(),
        ) {
            let config = Config {
                server: ServerConfig { host: host.clone(), port, cors_origin: "http://localhost:3000".to_string() },
                database: DatabaseConfig { driver: db_driver, url: db_url.clone() },
                cache: CacheConfig { driver: cache_driver, redis_url: None, ttl_seconds: ttl },
                theme: ThemeConfig { active: theme_name.clone(), path: PathBuf::from("themes") },
                upload: UploadConfig::default(),
            };
            
            // Serialize and deserialize
            let yaml = serde_yaml::to_string(&config).expect("Serialization failed");
            let parsed: Config = serde_yaml::from_str(&yaml).expect("Deserialization failed");
            
            // All fields should match
            prop_assert_eq!(parsed.server.host, host);
            prop_assert_eq!(parsed.server.port, port);
            prop_assert_eq!(parsed.database.driver, db_driver);
            prop_assert_eq!(parsed.database.url, db_url);
            prop_assert_eq!(parsed.cache.driver, cache_driver);
            prop_assert_eq!(parsed.cache.ttl_seconds, ttl);
            prop_assert_eq!(parsed.theme.active, theme_name);
        }

        /// **Property 20 Extension: Missing file returns complete defaults**
        /// **Validates: Requirements 8.5**
        #[test]
        fn property_20_missing_file_complete_defaults(suffix in "[a-z]{5,10}") {
            let path_str = format!("nonexistent_{}.yml", suffix);
            let path = std::path::Path::new(&path_str);
            
            // File should not exist
            prop_assert!(!path.exists());
            
            // Loading should succeed with defaults
            let config = Config::load(path).expect("Should return defaults for missing file");
            
            // Verify all defaults
            prop_assert_eq!(config.server.host, "0.0.0.0");
            prop_assert_eq!(config.server.port, 8080);
            prop_assert_eq!(config.database.driver, DatabaseDriver::Sqlite);
            prop_assert_eq!(config.database.url, "data/noteva.db");
            prop_assert_eq!(config.cache.driver, CacheDriver::Memory);
            prop_assert!(config.cache.redis_url.is_none());
            prop_assert_eq!(config.cache.ttl_seconds, 3600);
            prop_assert_eq!(config.theme.active, "default");
            prop_assert_eq!(config.theme.path, PathBuf::from("themes"));
        }

        /// **Property 26 Extension: Env vars take precedence over file values**
        /// **Validates: Requirements 11.5**
        #[test]
        fn property_26_env_precedence_over_file(
            file_port in 1000u16..2000,
            env_port in 3000u16..4000,
        ) {
            let _guard = lock_env();
            
            // Clean up any leftover env vars first
            std::env::remove_var("NOTEVA_SERVER_HOST");
            std::env::remove_var("NOTEVA_SERVER_PORT");
            std::env::remove_var("NOTEVA_DATABASE_DRIVER");
            std::env::remove_var("NOTEVA_DATABASE_URL");
            std::env::remove_var("NOTEVA_CACHE_DRIVER");
            std::env::remove_var("NOTEVA_CACHE_REDIS_URL");
            std::env::remove_var("NOTEVA_CACHE_TTL_SECONDS");
            std::env::remove_var("NOTEVA_THEME_ACTIVE");
            std::env::remove_var("NOTEVA_THEME_PATH");
            
            // Create config file with one port value
            let mut file = NamedTempFile::new().expect("Failed to create temp file");
            write!(file, "server:\n  port: {}\n", file_port).expect("Failed to write config");
            
            // Set env var with different port
            std::env::set_var("NOTEVA_SERVER_PORT", env_port.to_string());
            
            // Load with env overrides
            let config = Config::load_with_env(file.path()).expect("Failed to load config");
            
            // Env var should take precedence
            prop_assert_eq!(config.server.port, env_port);
            prop_assert_ne!(config.server.port, file_port);
            
            // Clean up
            std::env::remove_var("NOTEVA_SERVER_PORT");
        }
    }
}
