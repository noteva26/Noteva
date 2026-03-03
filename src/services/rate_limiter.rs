//! Rate limiter for login attempts
//!
//! Provides protection against brute force attacks by:
//! - Limiting login attempts per username with progressive lockout:
//!   - Tier 1: 5 failures  → 15 minute lockout
//!   - Tier 2: 10 failures → 1 hour lockout
//!   - Tier 3: 20 failures → 24 hour lockout
//! - Limiting requests per IP address (10 requests per minute)

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

fn lockout_tiers() -> Vec<(usize, Duration)> {
    vec![
        (20, Duration::hours(24)),   // Tier 3: 20 failures → 24h lockout
        (10, Duration::hours(1)),    // Tier 2: 10 failures → 1h lockout
        (5,  Duration::minutes(15)), // Tier 1: 5 failures  → 15min lockout
    ]
}

/// Login rate limiter
pub struct LoginRateLimiter {
    /// Failed login attempts by username
    username_attempts: Arc<RwLock<HashMap<String, Vec<DateTime<Utc>>>>>,
    /// Request attempts by IP address
    ip_attempts: Arc<RwLock<HashMap<IpAddr, Vec<DateTime<Utc>>>>>,
}

impl LoginRateLimiter {
    /// Create a new rate limiter
    pub fn new() -> Self {
        Self {
            username_attempts: Arc::new(RwLock::new(HashMap::new())),
            ip_attempts: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Check if username is rate limited (progressive tiers).
    /// Returns `Some(lockout_seconds)` if limited, `None` otherwise.
    pub async fn check_username_limit(&self, username: &str) -> Option<i64> {
        let mut attempts = self.username_attempts.write().await;
        let now = Utc::now();
        let key = username.to_lowercase();
        let username_attempts = attempts.entry(key).or_insert_with(Vec::new);
        
        // Check tiers from strictest to least strict
        for (threshold, window) in lockout_tiers() {
            let cutoff = now - window;
            let count = username_attempts.iter().filter(|t| **t > cutoff).count();
            if count >= threshold {
                return Some(window.num_seconds());
            }
        }
        None
    }
    
    /// Backwards-compatible: check if username is rate limited (bool)
    pub async fn is_username_limited(&self, username: &str) -> bool {
        self.check_username_limit(username).await.is_some()
    }
    
    /// Get remaining attempts before the first lockout tier (5)
    pub async fn remaining_attempts(&self, username: &str) -> usize {
        let attempts = self.username_attempts.read().await;
        let key = username.to_lowercase();
        let now = Utc::now();
        let cutoff = now - Duration::minutes(15); // tier 1 window
        
        let count = attempts
            .get(&key)
            .map(|times| times.iter().filter(|t| **t > cutoff).count())
            .unwrap_or(0);
        
        5usize.saturating_sub(count)
    }
    
    /// Record a failed login attempt for username
    pub async fn record_failed_attempt(&self, username: &str) {
        let mut attempts = self.username_attempts.write().await;
        let now = Utc::now();
        
        let username_attempts = attempts.entry(username.to_lowercase()).or_insert_with(Vec::new);
        username_attempts.push(now);
    }
    
    /// Clear failed attempts for username (on successful login)
    pub async fn clear_username_attempts(&self, username: &str) {
        let mut attempts = self.username_attempts.write().await;
        attempts.remove(&username.to_lowercase());
    }
    
    /// Check if IP is rate limited (10 requests per minute)
    pub async fn is_ip_limited(&self, ip: IpAddr) -> bool {
        let mut attempts = self.ip_attempts.write().await;
        let now = Utc::now();
        let cutoff = now - Duration::minutes(1);
        
        // Get or create attempt list for this IP
        let ip_attempts = attempts.entry(ip).or_insert_with(Vec::new);
        
        // Remove old attempts
        ip_attempts.retain(|time| *time > cutoff);
        
        // Check if limited (10 or more requests in last minute)
        ip_attempts.len() >= 10
    }
    
    /// Record a request from IP
    pub async fn record_ip_request(&self, ip: IpAddr) {
        let mut attempts = self.ip_attempts.write().await;
        let now = Utc::now();
        
        let ip_attempts = attempts.entry(ip).or_insert_with(Vec::new);
        ip_attempts.push(now);
    }
    
    /// Clean up old entries (should be called periodically)
    pub async fn cleanup(&self) {
        let now = Utc::now();
        let username_cutoff = now - Duration::hours(24); // longest tier window
        let ip_cutoff = now - Duration::minutes(1);
        
        // Clean username attempts
        {
            let mut attempts = self.username_attempts.write().await;
            attempts.retain(|_, times| {
                times.retain(|time| *time > username_cutoff);
                !times.is_empty()
            });
        }
        
        // Clean IP attempts
        {
            let mut attempts = self.ip_attempts.write().await;
            attempts.retain(|_, times| {
                times.retain(|time| *time > ip_cutoff);
                !times.is_empty()
            });
        }
    }
}

impl Default for LoginRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[tokio::test]
    async fn test_username_rate_limit() {
        let limiter = LoginRateLimiter::new();
        
        // First 4 attempts should not be limited
        for _ in 0..4 {
            assert!(!limiter.is_username_limited("testuser").await);
            limiter.record_failed_attempt("testuser").await;
        }
        
        // Record 5th attempt
        limiter.record_failed_attempt("testuser").await;
        
        // Now should be limited (5 attempts recorded)
        assert!(limiter.is_username_limited("testuser").await);
        
        // Clear attempts
        limiter.clear_username_attempts("testuser").await;
        assert!(!limiter.is_username_limited("testuser").await);
    }
    
    #[tokio::test]
    async fn test_ip_rate_limit() {
        let limiter = LoginRateLimiter::new();
        let ip = IpAddr::from_str("127.0.0.1").unwrap();
        
        // First 9 requests should not be limited
        for _ in 0..9 {
            assert!(!limiter.is_ip_limited(ip).await);
            limiter.record_ip_request(ip).await;
        }
        
        // Record 10th request
        limiter.record_ip_request(ip).await;
        
        // Now should be limited (10 requests recorded)
        assert!(limiter.is_ip_limited(ip).await);
    }
    
    #[tokio::test]
    async fn test_case_insensitive_username() {
        let limiter = LoginRateLimiter::new();
        
        limiter.record_failed_attempt("TestUser").await;
        limiter.record_failed_attempt("testuser").await;
        limiter.record_failed_attempt("TESTUSER").await;
        
        // All should count as the same user
        assert!(!limiter.is_username_limited("testuser").await);
        limiter.record_failed_attempt("testuser").await;
        limiter.record_failed_attempt("testuser").await;
        assert!(limiter.is_username_limited("TestUser").await);
    }
}
