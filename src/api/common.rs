//! Common API utilities and shared types
//!
//! This module contains shared utilities used across multiple API endpoints.

use serde::Deserialize;

// ============================================================================
// Pagination Defaults
// ============================================================================

/// Default page number (1-indexed)
pub fn default_page() -> u32 {
    1
}

/// Default page size for public APIs
pub fn default_page_size() -> u32 {
    10
}

/// Default page number for admin APIs (i64 for compatibility)
pub fn default_page_i64() -> i64 {
    1
}

/// Default page size for admin APIs
pub fn default_per_page() -> i64 {
    20
}

// ============================================================================
// Pagination Query Types
// ============================================================================

/// Basic pagination query parameters
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

/// Admin pagination query parameters (uses i64)
#[derive(Debug, Deserialize)]
pub struct AdminPaginationQuery {
    #[serde(default = "default_page_i64")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}
