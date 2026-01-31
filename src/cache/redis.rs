//! Redis cache implementation
//!
//! Provides a distributed cache using Redis for multi-instance deployments.
//!
//! # Features
//! - TTL-based expiration via Redis SETEX/EXPIRE commands
//! - Pattern-based deletion via SCAN + DEL (production-safe, not KEYS)
//! - Thread-safe async access
//!
//! # Requirements
//! - 9.2: WHERE Redis 已配置 THEN Cache_Layer SHALL 使用 Redis 作为分布式缓存
//! - 9.4: THE Cache_Layer SHALL 支持配置缓存过期时间

use super::CacheLayer;
use anyhow::{Context, Result};
use async_trait::async_trait;
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

/// Default TTL for cache entries (1 hour)
const DEFAULT_TTL: Duration = Duration::from_secs(3600);

/// Number of keys to scan per iteration in delete_pattern
const SCAN_COUNT: usize = 100;

/// Redis cache implementation
///
/// This cache implementation uses Redis for distributed caching.
/// Values are stored as JSON strings to support generic types.
pub struct RedisCache {
    /// Multiplexed connection for async operations
    connection: MultiplexedConnection,
    /// Default TTL for entries when not specified
    default_ttl: Duration,
}

impl std::fmt::Debug for RedisCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisCache")
            .field("default_ttl", &self.default_ttl)
            .finish_non_exhaustive()
    }
}

impl RedisCache {
    /// Create a new Redis cache with the given connection URL
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379")
    ///
    /// # Errors
    /// Returns an error if the connection cannot be established.
    pub async fn new(redis_url: &str) -> Result<Self> {
        Self::with_ttl(redis_url, DEFAULT_TTL).await
    }

    /// Create a new Redis cache with custom default TTL
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL
    /// * `default_ttl` - Default time-to-live for cache entries
    ///
    /// # Errors
    /// Returns an error if the connection cannot be established.
    pub async fn with_ttl(redis_url: &str, default_ttl: Duration) -> Result<Self> {
        let client = Client::open(redis_url)
            .context("Failed to create Redis client")?;
        
        let connection = client
            .get_multiplexed_async_connection()
            .await
            .context("Failed to connect to Redis")?;

        Ok(Self {
            connection,
            default_ttl,
        })
    }

    /// Get the default TTL for this cache
    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }

    /// Convert a glob-style pattern to Redis SCAN pattern
    ///
    /// Redis SCAN uses glob-style patterns similar to our memory cache:
    /// - `*` matches any sequence of characters
    /// - `?` matches any single character
    /// - `[abc]` matches any character in the brackets
    ///
    /// Since Redis already uses glob patterns, we can pass through directly.
    fn to_redis_pattern(pattern: &str) -> String {
        pattern.to_string()
    }
}

#[async_trait]
impl CacheLayer for RedisCache {
    /// Get a value from Redis cache
    ///
    /// Returns `Ok(Some(value))` if the key exists,
    /// `Ok(None)` if the key doesn't exist.
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.connection.clone();
        
        let result: Option<String> = conn
            .get(key)
            .await
            .context("Failed to get value from Redis")?;

        match result {
            Some(json) => {
                let value = serde_json::from_str(&json)
                    .context("Failed to deserialize cached value")?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Set a value in Redis cache with TTL
    ///
    /// Uses SETEX to atomically set the value with expiration.
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `value` - The value to cache (must be serializable)
    /// * `ttl` - Time-to-live for this entry
    async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        let mut conn = self.connection.clone();
        
        let json = serde_json::to_string(value)
            .context("Failed to serialize cache value")?;

        // Use SETEX for atomic set with expiration
        // TTL is in seconds for Redis
        let ttl_secs = ttl.as_secs().max(1) as u64; // Minimum 1 second
        
        let _: () = conn.set_ex(key, json, ttl_secs)
            .await
            .context("Failed to set value in Redis")?;

        Ok(())
    }

    /// Delete a value from Redis cache
    ///
    /// If the key doesn't exist, this is a no-op.
    async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.connection.clone();
        
        let _: () = conn
            .del(key)
            .await
            .context("Failed to delete key from Redis")?;

        Ok(())
    }

    /// Delete all values matching a glob-style pattern
    ///
    /// Uses SCAN + DEL for production safety (not KEYS which can block).
    ///
    /// Supports:
    /// - `*` matches any sequence of characters
    /// - `?` matches any single character
    ///
    /// # Examples
    /// - `articles:*` deletes all keys starting with `articles:`
    /// - `user:*:profile` deletes all user profile keys
    async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        let mut conn = self.connection.clone();
        let redis_pattern = Self::to_redis_pattern(pattern);

        // Use SCAN to iterate through keys matching the pattern
        // This is production-safe as it doesn't block the server
        let mut cursor: u64 = 0;
        
        loop {
            // SCAN returns (new_cursor, keys)
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&redis_pattern)
                .arg("COUNT")
                .arg(SCAN_COUNT)
                .query_async(&mut conn)
                .await
                .context("Failed to scan keys in Redis")?;

            // Delete found keys
            if !keys.is_empty() {
                let _: () = conn
                    .del(&keys)
                    .await
                    .context("Failed to delete keys from Redis")?;
            }

            cursor = new_cursor;
            
            // Cursor 0 means we've completed the full iteration
            if cursor == 0 {
                break;
            }
        }

        Ok(())
    }

    /// Clear all cache entries
    ///
    /// Uses FLUSHDB to clear the current database.
    /// Note: This clears ALL keys in the current Redis database.
    async fn clear(&self) -> Result<()> {
        let mut conn = self.connection.clone();
        
        let _: () = redis::cmd("FLUSHDB")
            .query_async(&mut conn)
            .await
            .context("Failed to flush Redis database")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to get Redis URL from environment or use default
    fn get_redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
    }

    /// Tests are marked with #[ignore] because they require a running Redis server.
    /// Run with: cargo test --features redis-cache -- --ignored

    #[tokio::test]
    #[ignore = "requires running Redis server"]
    async fn test_set_and_get() {
        let cache = RedisCache::new(&get_redis_url()).await.unwrap();
        
        // Clean up first
        cache.delete("test:key1").await.unwrap();
        
        cache.set("test:key1", &"value1".to_string(), Duration::from_secs(60)).await.unwrap();
        
        let result: Option<String> = cache.get("test:key1").await.unwrap();
        assert_eq!(result, Some("value1".to_string()));
        
        // Clean up
        cache.delete("test:key1").await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running Redis server"]
    async fn test_get_nonexistent() {
        let cache = RedisCache::new(&get_redis_url()).await.unwrap();
        
        let result: Option<String> = cache.get("test:nonexistent_key_12345").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore = "requires running Redis server"]
    async fn test_delete() {
        let cache = RedisCache::new(&get_redis_url()).await.unwrap();
        
        cache.set("test:delete_key", &"value".to_string(), Duration::from_secs(60)).await.unwrap();
        cache.delete("test:delete_key").await.unwrap();
        
        let result: Option<String> = cache.get("test:delete_key").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore = "requires running Redis server"]
    async fn test_delete_pattern_star() {
        let cache = RedisCache::new(&get_redis_url()).await.unwrap();
        
        // Set up test data
        cache.set("test:pattern:articles:1", &"article1".to_string(), Duration::from_secs(60)).await.unwrap();
        cache.set("test:pattern:articles:2", &"article2".to_string(), Duration::from_secs(60)).await.unwrap();
        cache.set("test:pattern:users:1", &"user1".to_string(), Duration::from_secs(60)).await.unwrap();
        
        // Delete articles pattern
        cache.delete_pattern("test:pattern:articles:*").await.unwrap();
        
        let article1: Option<String> = cache.get("test:pattern:articles:1").await.unwrap();
        let article2: Option<String> = cache.get("test:pattern:articles:2").await.unwrap();
        let user1: Option<String> = cache.get("test:pattern:users:1").await.unwrap();
        
        assert_eq!(article1, None);
        assert_eq!(article2, None);
        assert_eq!(user1, Some("user1".to_string()));
        
        // Clean up
        cache.delete("test:pattern:users:1").await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running Redis server"]
    async fn test_delete_pattern_question_mark() {
        let cache = RedisCache::new(&get_redis_url()).await.unwrap();
        
        // Set up test data
        cache.set("test:qmark:user:1:profile", &"profile1".to_string(), Duration::from_secs(60)).await.unwrap();
        cache.set("test:qmark:user:2:profile", &"profile2".to_string(), Duration::from_secs(60)).await.unwrap();
        cache.set("test:qmark:user:10:profile", &"profile10".to_string(), Duration::from_secs(60)).await.unwrap();
        
        // Delete single-character user IDs
        cache.delete_pattern("test:qmark:user:?:profile").await.unwrap();
        
        let profile1: Option<String> = cache.get("test:qmark:user:1:profile").await.unwrap();
        let profile2: Option<String> = cache.get("test:qmark:user:2:profile").await.unwrap();
        let profile10: Option<String> = cache.get("test:qmark:user:10:profile").await.unwrap();
        
        assert_eq!(profile1, None);
        assert_eq!(profile2, None);
        // "10" has two characters, so it shouldn't match "?"
        assert_eq!(profile10, Some("profile10".to_string()));
        
        // Clean up
        cache.delete("test:qmark:user:10:profile").await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running Redis server"]
    async fn test_ttl_expiration() {
        let cache = RedisCache::new(&get_redis_url()).await.unwrap();
        
        // Set with 1 second TTL
        cache.set("test:ttl_key", &"value".to_string(), Duration::from_secs(1)).await.unwrap();
        
        // Should exist immediately
        let result: Option<String> = cache.get("test:ttl_key").await.unwrap();
        assert_eq!(result, Some("value".to_string()));
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Should be expired
        let result: Option<String> = cache.get("test:ttl_key").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore = "requires running Redis server"]
    async fn test_complex_types() {
        let cache = RedisCache::new(&get_redis_url()).await.unwrap();
        
        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
        struct Article {
            id: i64,
            title: String,
            content: String,
        }
        
        let article = Article {
            id: 1,
            title: "Test Article".to_string(),
            content: "This is the content".to_string(),
        };
        
        cache.set("test:article:1", &article, Duration::from_secs(60)).await.unwrap();
        
        let result: Option<Article> = cache.get("test:article:1").await.unwrap();
        assert_eq!(result, Some(article));
        
        // Clean up
        cache.delete("test:article:1").await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running Redis server"]
    async fn test_overwrite_existing_key() {
        let cache = RedisCache::new(&get_redis_url()).await.unwrap();
        
        cache.set("test:overwrite_key", &"value1".to_string(), Duration::from_secs(60)).await.unwrap();
        cache.set("test:overwrite_key", &"value2".to_string(), Duration::from_secs(60)).await.unwrap();
        
        let result: Option<String> = cache.get("test:overwrite_key").await.unwrap();
        assert_eq!(result, Some("value2".to_string()));
        
        // Clean up
        cache.delete("test:overwrite_key").await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running Redis server"]
    async fn test_clear() {
        let cache = RedisCache::new(&get_redis_url()).await.unwrap();
        
        // Note: clear() uses FLUSHDB which clears ALL keys in the database
        // This test should be run on a dedicated test database
        
        cache.set("test:clear:key1", &"value1".to_string(), Duration::from_secs(60)).await.unwrap();
        cache.set("test:clear:key2", &"value2".to_string(), Duration::from_secs(60)).await.unwrap();
        
        cache.clear().await.unwrap();
        
        let result1: Option<String> = cache.get("test:clear:key1").await.unwrap();
        let result2: Option<String> = cache.get("test:clear:key2").await.unwrap();
        
        assert_eq!(result1, None);
        assert_eq!(result2, None);
    }
}
