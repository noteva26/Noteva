//! Admin API endpoints
//!
//! Handles HTTP requests for admin management:
//! - CRUD endpoints for backend management
//! - System settings endpoints
//! - Theme switching endpoints
//!
//! Satisfies requirements:
//! - 5.1: Admin dashboard
//! - 5.2: Content management
//! - 5.3: System configuration
//! - 6.1: Theme switching

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};
use std::process;

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};

/// Response for dashboard stats
#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub total_articles: i64,
    pub published_articles: i64,
    pub total_categories: i64,
    pub total_tags: i64,
}

/// Request for creating/updating a category
#[derive(Debug, Deserialize)]
pub struct CategoryRequest {
    pub name: String,
    #[serde(default)]
    pub slug: String,
    pub description: Option<String>,
    pub parent_id: Option<i64>,
}

/// Response for a category
#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i64>,
    pub created_at: String,
}

impl From<crate::models::Category> for CategoryResponse {
    fn from(cat: crate::models::Category) -> Self {
        Self {
            id: cat.id,
            slug: cat.slug,
            name: cat.name,
            description: cat.description,
            parent_id: cat.parent_id,
            created_at: cat.created_at.to_rfc3339(),
        }
    }
}

/// Request for creating/updating a tag
#[derive(Debug, Deserialize)]
pub struct TagRequest {
    pub name: String,
}

/// Response for a tag
#[derive(Debug, Serialize)]
pub struct TagResponse {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub created_at: String,
}

impl From<crate::models::Tag> for TagResponse {
    fn from(tag: crate::models::Tag) -> Self {
        Self {
            id: tag.id,
            slug: tag.slug,
            name: tag.name,
            created_at: tag.created_at.to_rfc3339(),
        }
    }
}

/// Request for theme switching
#[derive(Debug, Deserialize)]
pub struct ThemeSwitchRequest {
    pub theme: String,
}

/// Response for theme info
#[derive(Debug, Serialize)]
pub struct ThemeResponse {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub url: Option<String>,
    pub preview: Option<String>,
    pub active: bool,
}

/// Response for theme list
#[derive(Debug, Serialize)]
pub struct ThemeListResponse {
    pub themes: Vec<ThemeResponse>,
    pub current: String,
}

/// Request for updating site settings
#[derive(Debug, Deserialize)]
pub struct SiteSettingsRequest {
    pub site_name: String,
    pub site_description: String,
    pub site_subtitle: String,
    pub site_logo: String,
    pub site_footer: String,
}

/// Response for site settings
#[derive(Debug, Serialize)]
pub struct SiteSettingsResponse {
    pub site_name: String,
    pub site_description: String,
    pub site_subtitle: String,
    pub site_logo: String,
    pub site_footer: String,
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

/// App version constant - update when releasing
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build the admin router
pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/dashboard", get(get_dashboard))
        // System stats
        .route("/stats", get(get_system_stats))
        // Category management
        .route("/categories", post(create_category))
        .route("/categories/{id}", put(update_category))
        .route("/categories/{id}", delete(delete_category))
        // Tag management
        .route("/tags", post(create_tag))
        .route("/tags/{id}", delete(delete_tag))
        // Theme management
        .route("/themes", get(list_themes))
        .route("/themes/switch", post(switch_theme))
        // Site settings
        .route("/settings", get(get_settings))
        .route("/settings", put(update_settings))
}

/// GET /api/v1/admin/dashboard - Get dashboard stats
///
/// Requires admin authentication.
/// Satisfies requirement 5.1: Admin dashboard
async fn get_dashboard(
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
async fn get_system_stats(
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

/// POST /api/v1/admin/categories - Create category
///
/// Requires admin authentication.
/// Satisfies requirement 5.2: Content management
async fn create_category(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<CategoryRequest>,
) -> Result<(StatusCode, Json<CategoryResponse>), ApiError> {
    let input = crate::services::category::CreateCategoryInput::new(&body.name)
        .with_description(body.description.unwrap_or_default());
    
    let input = if !body.slug.is_empty() {
        input.with_slug(&body.slug)
    } else {
        input
    };
    
    let input = if let Some(parent_id) = body.parent_id {
        input.with_parent(parent_id)
    } else {
        input
    };

    let category = state
        .category_service
        .create(input)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(category.into())))
}

/// PUT /api/v1/admin/categories/:id - Update category
///
/// Requires admin authentication.
async fn update_category(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
    Json(body): Json<CategoryRequest>,
) -> Result<Json<CategoryResponse>, ApiError> {
    let mut input = crate::services::category::UpdateCategoryInput::new()
        .with_name(&body.name);
    
    if !body.slug.is_empty() {
        input = input.with_slug(&body.slug);
    }
    
    if let Some(desc) = body.description {
        input = input.with_description(Some(desc));
    }
    
    if let Some(parent_id) = body.parent_id {
        input = input.with_parent(Some(parent_id));
    }

    let category = state
        .category_service
        .update(id, input)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(category.into()))
}

/// DELETE /api/v1/admin/categories/:id - Delete category
///
/// Requires admin authentication.
async fn delete_category(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state
        .category_service
        .delete(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/admin/tags - Create tag
///
/// Requires admin authentication.
async fn create_tag(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<TagRequest>,
) -> Result<(StatusCode, Json<TagResponse>), ApiError> {
    let tag = state
        .tag_service
        .create_or_get(&body.name)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(tag.into())))
}

/// DELETE /api/v1/admin/tags/:id - Delete tag
///
/// Requires admin authentication.
async fn delete_tag(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state
        .tag_service
        .delete(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/admin/themes - List available themes
///
/// Requires admin authentication.
/// Satisfies requirement 6.1: Theme switching
async fn list_themes(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ThemeListResponse>, ApiError> {
    let theme_engine = state
        .theme_engine
        .read()
        .map_err(|e| ApiError::internal_error(format!("Failed to acquire theme lock: {}", e)))?;

    let themes = theme_engine
        .list_themes()
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let current = theme_engine.get_current_theme().to_string();

    let theme_responses: Vec<ThemeResponse> = themes
        .into_iter()
        .map(|info| ThemeResponse {
            name: info.name.clone(),
            display_name: info.display_name,
            description: info.description,
            version: info.version,
            author: info.author,
            url: info.url,
            preview: info.preview,
            active: info.name == current,
        })
        .collect();

    Ok(Json(ThemeListResponse {
        themes: theme_responses,
        current,
    }))
}

/// POST /api/v1/admin/themes/switch - Switch theme
///
/// Requires admin authentication.
/// Satisfies requirement 6.1: Theme switching
async fn switch_theme(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<ThemeSwitchRequest>,
) -> Result<Json<ThemeResponse>, ApiError> {
    let mut theme_engine = state
        .theme_engine
        .write()
        .map_err(|e| ApiError::internal_error(format!("Failed to acquire theme lock: {}", e)))?;

    let result = theme_engine.set_theme_with_fallback(&body.theme);

    if !result.success {
        return Err(ApiError::internal_error(
            result.error.unwrap_or_else(|| "Failed to switch theme".to_string())
        ));
    }

    let actual_theme = if result.used_fallback {
        theme_engine.get_default_theme().to_string()
    } else {
        body.theme
    };

    // Get theme info for the response
    let theme_info = theme_engine.get_theme_info(&actual_theme);

    Ok(Json(ThemeResponse {
        name: actual_theme.clone(),
        display_name: theme_info.map(|i| i.display_name.clone()).unwrap_or(actual_theme),
        description: theme_info.and_then(|i| i.description.clone()),
        version: theme_info.map(|i| i.version.clone()).unwrap_or_else(|| "1.0.0".to_string()),
        author: theme_info.and_then(|i| i.author.clone()),
        url: theme_info.and_then(|i| i.url.clone()),
        preview: theme_info.and_then(|i| i.preview.clone()),
        active: true,
    }))
}


/// GET /api/v1/admin/settings - Get site settings
///
/// Requires admin authentication.
/// Satisfies requirement 5.3: System configuration
async fn get_settings(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<SiteSettingsResponse>, ApiError> {
    let settings = state
        .settings_service
        .get_site_settings()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(SiteSettingsResponse {
        site_name: settings.site_name,
        site_description: settings.site_description,
        site_subtitle: settings.site_subtitle,
        site_logo: settings.site_logo,
        site_footer: settings.site_footer,
    }))
}

/// PUT /api/v1/admin/settings - Update site settings
///
/// Requires admin authentication.
/// Satisfies requirement 5.3: System configuration
async fn update_settings(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<SiteSettingsRequest>,
) -> Result<Json<SiteSettingsResponse>, ApiError> {
    let mut settings = state
        .settings_service
        .get_site_settings()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    settings.site_name = body.site_name;
    settings.site_description = body.site_description;
    settings.site_subtitle = body.site_subtitle;
    settings.site_logo = body.site_logo;
    settings.site_footer = body.site_footer;

    state
        .settings_service
        .update_site_settings(&settings)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(SiteSettingsResponse {
        site_name: settings.site_name,
        site_description: settings.site_description,
        site_subtitle: settings.site_subtitle,
        site_logo: settings.site_logo,
        site_footer: settings.site_footer,
    }))
}
