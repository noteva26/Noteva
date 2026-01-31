//! Cache layer
//!
//! This module provides caching abstraction for the Noteva blog system.
//! It supports:
//! - In-memory cache (moka) - default, for single-instance deployment
//! - Redis cache - optional, for distributed deployment
//!
//! The cache driver is selected based on configuration.
//!
//! # Usage
//!
//! ```rust,ignore
//! use noteva::cache::{create_cache, Cache};
//! use noteva::config::CacheConfig;
//!
//! let config = CacheConfig::default();
//! let cache = create_cache(&config).await?;
//! cache.set("key", &"value", Duration::from_secs(60)).await?;
//! ```

pub mod memory;
#[cfg(feature = "redis-cache")]
pub mod redis;

use anyhow::Result;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::config::{CacheConfig, CacheDriver};
use crate::plugin::HookManager;

/// Cache layer trait
///
/// This trait defines the interface for cache implementations.
/// Note: Due to Rust's object safety rules, this trait cannot be used
/// as a trait object (`dyn CacheLayer`). Use the `Cache` enum instead
/// for runtime polymorphism.
#[async_trait]
pub trait CacheLayer: Send + Sync {
    /// Get a value from cache
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>>;

    /// Set a value in cache with TTL
    async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T, ttl: Duration) -> Result<()>;

    /// Delete a value from cache
    async fn delete(&self, key: &str) -> Result<()>;

    /// Delete all values matching a pattern
    async fn delete_pattern(&self, pattern: &str) -> Result<()>;

    /// Clear all cache entries
    async fn clear(&self) -> Result<()>;
}

pub use memory::MemoryCache;
#[cfg(feature = "redis-cache")]
pub use redis::RedisCache;

/// Unified cache enum for runtime polymorphism
///
/// Since `CacheLayer` trait has generic methods, it cannot be used as a trait object.
/// This enum provides runtime polymorphism by wrapping concrete cache implementations.
///
/// # Requirements
/// - 9.1: THE Cache_Layer SHALL 默认使用进程内缓存存储热点数据
/// - 9.2: WHERE Redis 已配置 THEN Cache_Layer SHALL 使用 Redis 作为分布式缓存
#[derive(Debug)]
pub enum Cache {
    /// In-memory cache using moka
    Memory(MemoryCache),
    /// Redis cache for distributed deployment
    #[cfg(feature = "redis-cache")]
    Redis(RedisCache),
}

#[async_trait]
impl CacheLayer for Cache {
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>> {
        match self {
            Cache::Memory(cache) => cache.get(key).await,
            #[cfg(feature = "redis-cache")]
            Cache::Redis(cache) => cache.get(key).await,
        }
    }

    async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        match self {
            Cache::Memory(cache) => cache.set(key, value, ttl).await,
            #[cfg(feature = "redis-cache")]
            Cache::Redis(cache) => cache.set(key, value, ttl).await,
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        match self {
            Cache::Memory(cache) => cache.delete(key).await,
            #[cfg(feature = "redis-cache")]
            Cache::Redis(cache) => cache.delete(key).await,
        }
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        match self {
            Cache::Memory(cache) => cache.delete_pattern(pattern).await,
            #[cfg(feature = "redis-cache")]
            Cache::Redis(cache) => cache.delete_pattern(pattern).await,
        }
    }

    async fn clear(&self) -> Result<()> {
        match self {
            Cache::Memory(cache) => cache.clear().await,
            #[cfg(feature = "redis-cache")]
            Cache::Redis(cache) => cache.clear().await,
        }
    }
}

/// Cache wrapper with hook support
/// 
/// Wraps a Cache instance and triggers hooks on cache operations.
/// Triggers `cache_clear` hook when cache is cleared or patterns are deleted.
pub struct HookedCache {
    inner: Arc<Cache>,
    hook_manager: Option<Arc<HookManager>>,
}

impl HookedCache {
    /// Create a new HookedCache
    pub fn new(cache: Arc<Cache>) -> Self {
        Self {
            inner: cache,
            hook_manager: None,
        }
    }
    
    /// Create a HookedCache with hook support
    pub fn with_hooks(cache: Arc<Cache>, hook_manager: Arc<HookManager>) -> Self {
        Self {
            inner: cache,
            hook_manager: Some(hook_manager),
        }
    }
    
    /// Trigger cache_clear hook
    fn trigger_clear_hook(&self, pattern: Option<&str>) {
        if let Some(ref hook_manager) = self.hook_manager {
            let data = serde_json::json!({
                "pattern": pattern,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            hook_manager.trigger(crate::plugin::hook_names::CACHE_CLEAR, data);
        }
    }
    
    /// Get a value from cache
    pub async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>> {
        self.inner.get(key).await
    }
    
    /// Set a value in cache with TTL
    pub async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        self.inner.set(key, value, ttl).await
    }
    
    /// Delete a value from cache
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.inner.delete(key).await
    }
    
    /// Delete all values matching a pattern
    /// Triggers `cache_clear` hook with the pattern
    pub async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        self.trigger_clear_hook(Some(pattern));
        self.inner.delete_pattern(pattern).await
    }
    
    /// Clear all cache entries
    /// Triggers `cache_clear` hook
    pub async fn clear(&self) -> Result<()> {
        self.trigger_clear_hook(None);
        self.inner.clear().await
    }
}

/// Create a cache instance based on configuration
///
/// This factory function creates the appropriate cache implementation
/// based on the `CacheConfig`:
/// - `CacheDriver::Memory` - Creates an in-memory cache using moka
/// - `CacheDriver::Redis` - Creates a Redis cache (requires `redis-cache` feature)
///
/// # Arguments
/// * `config` - Cache configuration specifying driver and settings
///
/// # Returns
/// An `Arc<Cache>` that can be shared across threads
///
/// # Errors
/// - Returns an error if Redis is configured but the `redis-cache` feature is not enabled
/// - Returns an error if Redis connection fails
///
/// # Requirements
/// - 9.1: THE Cache_Layer SHALL 默认使用进程内缓存存储热点数据
/// - 9.2: WHERE Redis 已配置 THEN Cache_Layer SHALL 使用 Redis 作为分布式缓存
///
/// # Example
/// ```rust,ignore
/// use noteva::cache::create_cache;
/// use noteva::config::{CacheConfig, CacheDriver};
///
/// // Create memory cache (default)
/// let config = CacheConfig::default();
/// let cache = create_cache(&config).await?;
///
/// // Create Redis cache
/// let config = CacheConfig {
///     driver: CacheDriver::Redis,
///     redis_url: Some("redis://localhost:6379".to_string()),
///     ttl_seconds: 3600,
/// };
/// let cache = create_cache(&config).await?;
/// ```
pub async fn create_cache(config: &CacheConfig) -> Result<Arc<Cache>> {
    let ttl = Duration::from_secs(config.ttl_seconds);

    match config.driver {
        CacheDriver::Memory => {
            let cache = MemoryCache::with_capacity_and_ttl(10_000, ttl);
            Ok(Arc::new(Cache::Memory(cache)))
        }
        CacheDriver::Redis => {
            #[cfg(feature = "redis-cache")]
            {
                let redis_url = config.redis_url.as_ref()
                    .ok_or_else(|| anyhow::anyhow!(
                        "Redis URL is required when using Redis cache driver. \
                         Set 'redis_url' in cache configuration or use NOTEVA_CACHE_REDIS_URL environment variable."
                    ))?;
                
                let cache = RedisCache::with_ttl(redis_url, ttl).await?;
                Ok(Arc::new(Cache::Redis(cache)))
            }
            
            #[cfg(not(feature = "redis-cache"))]
            {
                anyhow::bail!(
                    "Redis cache driver is configured but the 'redis-cache' feature is not enabled. \
                     Either enable the feature with `--features redis-cache` or use 'memory' cache driver."
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_memory_cache() {
        let config = CacheConfig::default();
        let cache = create_cache(&config).await.unwrap();
        
        // Test basic operations
        cache.set("test_key", &"test_value".to_string(), Duration::from_secs(60)).await.unwrap();
        let result: Option<String> = cache.get("test_key").await.unwrap();
        assert_eq!(result, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_create_memory_cache_with_custom_ttl() {
        let config = CacheConfig {
            driver: CacheDriver::Memory,
            redis_url: None,
            ttl_seconds: 1800,
        };
        let cache = create_cache(&config).await.unwrap();
        
        // Cache should be created successfully
        cache.set("key", &"value".to_string(), Duration::from_secs(60)).await.unwrap();
        let result: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(result, Some("value".to_string()));
    }

    #[cfg(not(feature = "redis-cache"))]
    #[tokio::test]
    async fn test_create_redis_cache_without_feature() {
        let config = CacheConfig {
            driver: CacheDriver::Redis,
            redis_url: Some("redis://localhost:6379".to_string()),
            ttl_seconds: 3600,
        };
        
        let result = create_cache(&config).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("redis-cache") && err.contains("feature"));
    }

    #[cfg(feature = "redis-cache")]
    #[tokio::test]
    async fn test_create_redis_cache_without_url() {
        let config = CacheConfig {
            driver: CacheDriver::Redis,
            redis_url: None,
            ttl_seconds: 3600,
        };
        
        let result = create_cache(&config).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Redis URL"));
    }

    #[cfg(feature = "redis-cache")]
    #[tokio::test]
    #[ignore = "requires running Redis server"]
    async fn test_create_redis_cache_success() {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        
        let config = CacheConfig {
            driver: CacheDriver::Redis,
            redis_url: Some(redis_url),
            ttl_seconds: 3600,
        };
        
        let cache = create_cache(&config).await.unwrap();
        
        // Test basic operations
        cache.set("factory_test_key", &"test_value".to_string(), Duration::from_secs(60)).await.unwrap();
        let result: Option<String> = cache.get("factory_test_key").await.unwrap();
        assert_eq!(result, Some("test_value".to_string()));
        
        // Clean up
        cache.delete("factory_test_key").await.unwrap();
    }
}
