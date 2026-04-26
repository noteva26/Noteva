
use super::*;
use proptest::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    super::CONFIG_ENV_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner())
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
    prop_oneof![Just(DatabaseDriver::Sqlite), Just(DatabaseDriver::Mysql),]
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
    prop_oneof![Just(CacheDriver::Memory), Just(CacheDriver::Redis),]
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
    (valid_host_strategy(), valid_port_strategy()).prop_map(|(host, port)| ServerConfig {
        host,
        port,
        cors_origin: "http://localhost:3000".to_string(),
    })
}

/// Strategy for generating valid DatabaseConfig
fn valid_database_config_strategy() -> impl Strategy<Value = DatabaseConfig> {
    (
        valid_database_driver_strategy(),
        valid_database_url_strategy(),
    )
        .prop_map(|(driver, url)| DatabaseConfig { driver, url })
}

/// Strategy for generating valid CacheConfig
fn valid_cache_config_strategy() -> impl Strategy<Value = CacheConfig> {
    (
        valid_cache_driver_strategy(),
        valid_redis_url_strategy(),
        valid_ttl_strategy(),
    )
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
            store_url: None,
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
        Just("server:\n  port: \"8080\"".to_string()), // String instead of number
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
        valid_database_driver_strategy().prop_map(|d| (
            "NOTEVA_DATABASE_DRIVER".to_string(),
            match d {
                DatabaseDriver::Sqlite => "sqlite",
                DatabaseDriver::Mysql => "mysql",
            }
            .to_string()
        )),
        valid_database_url_strategy().prop_map(|u| ("NOTEVA_DATABASE_URL".to_string(), u)),
    ]
}

/// Strategy for generating environment variable values for cache config
fn env_cache_value_strategy() -> impl Strategy<Value = (String, String)> {
    prop_oneof![
        valid_cache_driver_strategy().prop_map(|d| (
            "NOTEVA_CACHE_DRIVER".to_string(),
            match d {
                CacheDriver::Memory => "memory",
                CacheDriver::Redis => "redis",
            }
            .to_string()
        )),
        valid_ttl_strategy().prop_map(|t| ("NOTEVA_CACHE_TTL_SECONDS".to_string(), t.to_string())),
    ]
}

/// Strategy for generating environment variable values for theme config
fn env_theme_value_strategy() -> impl Strategy<Value = (String, String)> {
    prop_oneof![
        valid_theme_name_strategy().prop_map(|n| ("NOTEVA_THEME_ACTIVE".to_string(), n)),
        valid_theme_path_strategy()
            .prop_map(|p| ("NOTEVA_THEME_PATH".to_string(), p.display().to_string())),
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
            store_url: None,
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
