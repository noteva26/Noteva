//! Rate limiter for login attempts
//!
//! Provides protection against brute force attacks by:
//! - Limiting login attempts per username (5 attempts per 15 minutes)
//! - Limiting requests per IP address (10 requests per minute)

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

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
    
    /// Check if username is rate limited (5 attempts per 15 minutes)
    pub async fn is_username_limited(&self, username: &str) -> bool {
        let mut attempts = self.username_attempts.write().await;
        let now = Utc::now();
        let cutoff = now - Duration::minutes(15);
        
        // Get or create attempt list for this username
        let username_attempts = attempts.entry(username.to_lowercase()).or_insert_with(Vec::new);
        
        // Remove old attempts
        username_attempts.retain(|time| *time > cutoff);
        
        // Check if limited (5 or more attempts in last 15 minutes)
        username_attempts.len() >= 5
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
        let username_cutoff = now - Duration::minutes(15);
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
