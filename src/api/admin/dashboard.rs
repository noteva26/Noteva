//! Dashboard and system stats endpoints

use axum::{extract::State, Json};
use serde::Serialize;
use sysinfo::{Pid, System};
use std::process;

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};
use super::update::APP_VERSION;

/// Response for dashboard stats
#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub total_articles: i64,
    pub published_articles: i64,
    pub total_categories: i64,
    pub total_tags: i64,
}

/// Response for system stats (CPU, memory usage)
#[derive(Debug, Serialize)]
pub struct SystemStatsResponse {
    /// App version
    pub version: String,
    /// Process memory usage in bytes
    pub memory_bytes: u64,
    /// Process memory usage formatted (e.g., "45.2 MB")
    pub memory_formatted: String,
    /// System total memory in bytes
    pub system_total_memory: u64,
    /// System used memory in bytes
    pub system_used_memory: u64,
    /// Operating system name
    pub os_name: String,
    /// Process uptime in seconds
    pub uptime_seconds: u64,
    /// Uptime formatted (e.g., "2h 15m")
    pub uptime_formatted: String,
    /// Total requests processed
    pub total_requests: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
}

/// GET /api/v1/admin/dashboard - Get dashboard stats
///
/// Requires admin authentication.
/// Satisfies requirement 5.1: Admin dashboard
pub async fn get_dashboard(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<DashboardResponse>, ApiError> {
    // Get article counts
    let total_articles = state
        .article_service
        .count()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let published_articles = state
        .article_service
        .count_published()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Get category count
    let categories = state
        .category_service
        .list()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Get tag count
    let tags = state
        .tag_service
        .list()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(DashboardResponse {
        total_articles,
        published_articles,
        total_categories: categories.len() as i64,
        total_tags: tags.len() as i64,
    }))
}

/// GET /api/v1/admin/stats - Get system resource stats
///
/// Returns memory usage and request statistics for the current process.
/// Requires admin authentication.
pub async fn get_system_stats(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<SystemStatsResponse>, ApiError> {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let pid = Pid::from_u32(process::id());
    
    let memory_bytes = if let Some(proc) = sys.process(pid) {
        proc.memory()
    } else {
        0
    };
    
    // Format memory
    let memory_formatted = format_bytes(memory_bytes);
    
    // System-wide stats
    let system_total_memory = sys.total_memory();
    let system_used_memory = sys.used_memory();
    
    // OS name
    let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
    
    // Request stats from middleware
    let uptime_seconds = state.request_stats.uptime_seconds();
    let uptime_formatted = format_uptime(uptime_seconds);
    let total_requests = state.request_stats.total_requests();
    let avg_response_time_ms = state.request_stats.avg_response_time_us() / 1000.0;
    
    Ok(Json(SystemStatsResponse {
        version: APP_VERSION.to_string(),
        memory_bytes,
        memory_formatted,
        system_total_memory,
        system_used_memory,
        os_name,
        uptime_seconds,
        uptime_formatted,
        total_requests,
        avg_response_time_ms,
    }))
}

/// Format uptime to human readable string
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    
    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else {
        format!("{}s", seconds)
    }
}

/// Format bytes to human readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
