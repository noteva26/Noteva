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