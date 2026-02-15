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
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};
use std::process;

use crate::api::common::{default_page_i64, default_per_page};
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
    pub requires_noteva: String,
    pub compatible: bool,
    pub compatibility_message: Option<String>,
}

/// Response for theme list
#[derive(Debug, Serialize)]
pub struct ThemeListResponse {
    pub themes: Vec<ThemeResponse>,
    pub current: String,
}

/// Theme update info
#[derive(Debug, Serialize)]
pub struct ThemeUpdateInfo {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
}

/// Theme updates response
#[derive(Debug, Serialize)]
pub struct ThemeUpdatesResponse {
    pub updates: Vec<ThemeUpdateInfo>,
}

/// Request for updating site settings (supports dynamic fields)
pub type SiteSettingsRequest = std::collections::HashMap<String, String>;

/// Response for site settings
#[derive(Debug, Serialize)]
pub struct SiteSettingsResponse {
    pub site_name: String,
    pub site_description: String,
    pub site_subtitle: String,
    pub site_logo: String,
    pub site_footer: String,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, String>,
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

/// Response for update check
#[derive(Debug, Serialize)]
pub struct UpdateCheckResponse {
    /// Current version
    pub current_version: String,
    /// Latest version available
    pub latest_version: Option<String>,
    /// Whether an update is available
    pub update_available: bool,
    /// Release URL
    pub release_url: Option<String>,
    /// Release notes
    pub release_notes: Option<String>,
    /// Release date
    pub release_date: Option<String>,
    /// Whether checking beta releases
    pub is_beta: bool,
    /// Error message if check failed
    pub error: Option<String>,
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
        // Update check
        .route("/update-check", get(check_update))
        .route("/update-perform", post(perform_update))
        // Category management
        .route("/categories", post(create_category))
        .route("/categories/:id", put(update_category))
        .route("/categories/:id", delete(delete_category))
        // Tag management
        .route("/tags", post(create_tag))
        .route("/tags/:id", delete(delete_tag))
        // Theme management
        .route("/themes", get(list_themes))
        .route("/themes/switch", post(switch_theme))
        .route("/themes/store", get(get_theme_store))
        .route("/themes/updates", get(check_theme_updates))
        .route("/themes/reload", post(reload_themes))
        // Plugin management
        .route("/plugins/reload", post(reload_plugins))
        // Site settings
        .route("/settings", get(get_settings))
        .route("/settings", put(update_settings))
        // Comment moderation
        .route("/comments/pending", get(list_pending_comments))
        .route("/comments/:id/approve", post(approve_comment))
        .route("/comments/:id/reject", post(reject_comment))
        // Login logs (security)
        .route("/login-logs", get(list_login_logs))
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

    let themes = theme_engine.list_themes();

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
            requires_noteva: info.requires_noteva,
            compatible: info.compatible,
            compatibility_message: info.compatibility_message,
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
    // Switch theme and get result
    let (actual_theme, theme_info) = {
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

        // Get theme info before releasing lock
        let theme_info = theme_engine.get_theme_info(&actual_theme).cloned();
        
        (actual_theme, theme_info)
    }; // Lock released here

    // Save active theme to database
    state.settings_service
        .set("active_theme", &actual_theme)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to save theme setting: {}", e)))?;

    Ok(Json(ThemeResponse {
        name: actual_theme.clone(),
        display_name: theme_info.as_ref().map(|i| i.display_name.clone()).unwrap_or_else(|| actual_theme.clone()),
        description: theme_info.as_ref().and_then(|i| i.description.clone()),
        version: theme_info.as_ref().map(|i| i.version.clone()).unwrap_or_else(|| "1.0.0".to_string()),
        author: theme_info.as_ref().and_then(|i| i.author.clone()),
        url: theme_info.as_ref().and_then(|i| i.url.clone()),
        preview: theme_info.as_ref().and_then(|i| i.preview.clone()),
        active: true,
        requires_noteva: theme_info.as_ref().map(|i| i.requires_noteva.clone()).unwrap_or_default(),
        compatible: theme_info.as_ref().map(|i| i.compatible).unwrap_or(true),
        compatibility_message: theme_info.and_then(|i| i.compatibility_message.clone()),
    }))
}

/// Store theme info
#[derive(Debug, Serialize, Deserialize)]
pub struct StoreThemeInfo {
    pub slug: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub github_url: Option<String>,
    pub external_url: Option<String>,
    pub license_type: String,
    pub price_info: Option<String>,
    pub download_source: String,
    pub download_count: i64,
    pub avg_rating: Option<f64>,
    pub rating_count: Option<i64>,
    pub tags: Vec<String>,
    pub installed: bool,
}

/// Theme store response
#[derive(Debug, Serialize)]
pub struct ThemeStoreResponse {
    pub themes: Vec<StoreThemeInfo>,
}

/// Store API item (matches store external API response)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StoreApiItem {
    slug: String,
    name: String,
    description: String,
    #[serde(default)]
    cover_image: String,
    author: String,
    version: String,
    github_url: Option<String>,
    external_url: Option<String>,
    license_type: String,
    price_info: Option<String>,
    download_source: String,
    download_count: i64,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StoreApiListResponse {
    items: Vec<StoreApiItem>,
    total: i64,
    page: i64,
    per_page: i64,
}

/// GET /api/v1/admin/themes/store - Get theme store from Noteva Store API
#[axum::debug_handler]
async fn get_theme_store(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ThemeStoreResponse>, ApiError> {
    let store_url = state.store_url.as_deref().unwrap_or("https://store.noteva.com");

    let client = reqwest::Client::builder()
        .user_agent("Noteva")
        .timeout(std::time::Duration::from_secs(15))
        .no_proxy()
        .build()
        .map_err(|e| ApiError::internal_error(format!("HTTP client error: {}", e)))?;

    let url = format!("{}/api/v1/store/themes?per_page=100", store_url);
    let response = client.get(&url).send().await
        .map_err(|e| ApiError::internal_error(format!("Failed to fetch store: {}", e)))?;

    if !response.status().is_success() {
        return Ok(Json(ThemeStoreResponse { themes: vec![] }));
    }

    let store_data: StoreApiListResponse = response.json().await
        .map_err(|e| ApiError::internal_error(format!("Failed to parse store response: {}", e)))?;

    // Get installed themes
    let installed_names: std::collections::HashSet<String> = {
        let theme_engine = state.theme_engine.read()
            .map_err(|e| ApiError::internal_error(format!("Lock error: {}", e)))?;
        theme_engine.list_themes().iter().map(|t| t.name.clone()).collect()
    };

    let themes: Vec<StoreThemeInfo> = store_data.items.into_iter().map(|item| {
        StoreThemeInfo {
            installed: installed_names.contains(&item.slug),
            slug: item.slug,
            name: item.name,
            version: item.version,
            description: Some(item.description),
            author: Some(item.author),
            github_url: item.github_url,
            external_url: item.external_url,
            license_type: item.license_type,
            price_info: item.price_info,
            download_source: item.download_source,
            download_count: item.download_count,
            avg_rating: None,
            rating_count: None,
            tags: item.tags,
        }
    }).collect();

    Ok(Json(ThemeStoreResponse { themes }))
}

/// Store check-updates types
#[derive(Debug, Serialize)]
struct StoreInstalledItem {
    slug: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct StoreCheckUpdatesRequest {
    items: Vec<StoreInstalledItem>,
}

#[derive(Debug, Deserialize)]
struct StoreUpdateInfo {
    slug: String,
    current_version: String,
    latest_version: String,
}

/// GET /api/v1/admin/themes/updates - Check for theme updates via Store API
#[axum::debug_handler]
async fn check_theme_updates(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ThemeUpdatesResponse>, ApiError> {
    let store_url = match state.store_url.as_deref() {
        Some(url) => url.to_string(),
        None => "https://store.noteva.com".to_string(),
    };

    // Get installed themes
    let installed: Vec<StoreInstalledItem> = {
        let theme_engine = state.theme_engine.read()
            .map_err(|e| ApiError::internal_error(format!("Lock error: {}", e)))?;
        theme_engine.list_themes().iter().map(|t| StoreInstalledItem {
            slug: t.name.clone(),
            version: t.version.clone(),
        }).collect()
    };

    if installed.is_empty() {
        return Ok(Json(ThemeUpdatesResponse { updates: vec![] }));
    }

    let client = reqwest::Client::builder()
        .user_agent("Noteva")
        .timeout(std::time::Duration::from_secs(15))
        .no_proxy()
        .build()
        .map_err(|e| ApiError::internal_error(format!("HTTP client error: {}", e)))?;

    let url = format!("{}/api/v1/store/check-updates", store_url);
    let response = client.post(&url)
        .json(&StoreCheckUpdatesRequest { items: installed })
        .send().await
        .map_err(|e| ApiError::internal_error(format!("Failed to check updates: {}", e)))?;

    if !response.status().is_success() {
        return Ok(Json(ThemeUpdatesResponse { updates: vec![] }));
    }

    let store_updates: Vec<StoreUpdateInfo> = response.json().await
        .map_err(|e| ApiError::internal_error(format!("Failed to parse updates: {}", e)))?;

    let updates = store_updates.into_iter().map(|u| ThemeUpdateInfo {
        name: u.slug,
        current_version: u.current_version,
        latest_version: u.latest_version,
        has_update: true,
    }).collect();

    Ok(Json(ThemeUpdatesResponse { updates }))
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

    // Get all settings for extra fields
    let all_settings = state
        .settings_service
        .get_all_settings()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Filter out the main fields to put in extra
    let main_keys = ["site_name", "site_description", "site_subtitle", "site_logo", "site_footer"];
    let extra: std::collections::HashMap<String, String> = all_settings
        .into_iter()
        .filter(|(k, _)| !main_keys.contains(&k.as_str()))
        .collect();

    Ok(Json(SiteSettingsResponse {
        site_name: settings.site_name,
        site_description: settings.site_description,
        site_subtitle: settings.site_subtitle,
        site_logo: settings.site_logo,
        site_footer: settings.site_footer,
        extra,
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
    // Update each setting from the request
    for (key, value) in body.iter() {
        state
            .settings_service
            .set_setting(key, value)
            .await
            .map_err(|e| ApiError::internal_error(e.to_string()))?;
    }

    // Return updated settings
    let settings = state
        .settings_service
        .get_site_settings()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Get all settings for extra fields
    let all_settings = state
        .settings_service
        .get_all_settings()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Filter out the main fields to put in extra
    let main_keys = ["site_name", "site_description", "site_subtitle", "site_logo", "site_footer"];
    let extra: std::collections::HashMap<String, String> = all_settings
        .into_iter()
        .filter(|(k, _)| !main_keys.contains(&k.as_str()))
        .collect();

    Ok(Json(SiteSettingsResponse {
        site_name: settings.site_name,
        site_description: settings.site_description,
        site_subtitle: settings.site_subtitle,
        site_logo: settings.site_logo,
        site_footer: settings.site_footer,
        extra,
    }))
}

/// Query parameters for update check
#[derive(Debug, Deserialize)]
pub struct UpdateCheckQuery {
    /// Whether to check beta releases
    #[serde(default)]
    pub beta: bool,
}

/// GitHub release info
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    published_at: Option<String>,
    #[allow(dead_code)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

/// GitHub release asset
#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// GET /api/v1/admin/update-check - Check for updates
///
/// Requires admin authentication.
/// Checks GitHub releases for new versions.
async fn check_update(
    _user: AuthenticatedUser,
    axum::extract::Query(query): axum::extract::Query<UpdateCheckQuery>,
) -> Json<UpdateCheckResponse> {
    let current_version = APP_VERSION.to_string();
    
    // Determine which branch to check
    let api_url = if query.beta {
        "https://api.github.com/repos/noteva26/Noteva/releases"
    } else {
        "https://api.github.com/repos/noteva26/Noteva/releases/latest"
    };
    
    // Make HTTP request to GitHub API
    let client = match reqwest::Client::builder()
        .user_agent("Noteva-Update-Checker")
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Json(UpdateCheckResponse {
                current_version,
                latest_version: None,
                update_available: false,
                release_url: None,
                release_notes: None,
                release_date: None,
                is_beta: query.beta,
                error: Some(format!("Failed to create HTTP client: {}", e)),
            });
        }
    };
    
    let response = match client.get(api_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return Json(UpdateCheckResponse {
                current_version,
                latest_version: None,
                update_available: false,
                release_url: None,
                release_notes: None,
                release_date: None,
                is_beta: query.beta,
                error: Some(format!("Failed to fetch releases: {}", e)),
            });
        }
    };
    
    if !response.status().is_success() {
        return Json(UpdateCheckResponse {
            current_version,
            latest_version: None,
            update_available: false,
            release_url: None,
            release_notes: None,
            release_date: None,
            is_beta: query.beta,
            error: Some(format!("GitHub API returned status: {}", response.status())),
        });
    }
    
    // Parse response
    let release = if query.beta {
        // For beta, get all releases and find the latest (including prereleases)
        match response.json::<Vec<GitHubRelease>>().await {
            Ok(releases) => {
                releases.into_iter().next()
            }
            Err(e) => {
                return Json(UpdateCheckResponse {
                    current_version,
                    latest_version: None,
                    update_available: false,
                    release_url: None,
                    release_notes: None,
                    release_date: None,
                    is_beta: query.beta,
                    error: Some(format!("Failed to parse releases: {}", e)),
                });
            }
        }
    } else {
        // For stable, get the latest release
        match response.json::<GitHubRelease>().await {
            Ok(r) => Some(r),
            Err(e) => {
                return Json(UpdateCheckResponse {
                    current_version,
                    latest_version: None,
                    update_available: false,
                    release_url: None,
                    release_notes: None,
                    release_date: None,
                    is_beta: query.beta,
                    error: Some(format!("Failed to parse release: {}", e)),
                });
            }
        }
    };
    
    match release {
        Some(rel) => {
            // Remove 'v' prefix if present for comparison
            let latest = rel.tag_name.trim_start_matches('v').to_string();
            let current = current_version.trim_start_matches('v');
            
            // Simple version comparison (works for semver)
            let update_available = version_compare(&latest, current);
            
            Json(UpdateCheckResponse {
                current_version,
                latest_version: Some(latest),
                update_available,
                release_url: Some(rel.html_url),
                release_notes: rel.body,
                release_date: rel.published_at,
                is_beta: query.beta,
                error: None,
            })
        }
        None => {
            Json(UpdateCheckResponse {
                current_version,
                latest_version: None,
                update_available: false,
                release_url: None,
                release_notes: None,
                release_date: None,
                is_beta: query.beta,
                error: Some("No releases found".to_string()),
            })
        }
    }
}

/// Compare two version strings
/// Returns true if latest > current
fn version_compare(latest: &str, current: &str) -> bool {
    // Parse versions into comparable parts
    let parse_version = |v: &str| -> Vec<(u32, String)> {
        let mut parts = Vec::new();
        let mut num = String::new();
        let mut suffix = String::new();
        let mut in_suffix = false;
        
        for c in v.chars() {
            if c.is_ascii_digit() && !in_suffix {
                num.push(c);
            } else if c == '.' || c == '-' {
                if !num.is_empty() {
                    parts.push((num.parse().unwrap_or(0), suffix.clone()));
                    num.clear();
                    suffix.clear();
                }
                if c == '-' {
                    in_suffix = true;
                }
            } else {
                suffix.push(c);
                in_suffix = true;
            }
        }
        if !num.is_empty() || !suffix.is_empty() {
            parts.push((num.parse().unwrap_or(0), suffix));
        }
        parts
    };
    
    let latest_parts = parse_version(latest);
    let current_parts = parse_version(current);
    
    for i in 0..latest_parts.len().max(current_parts.len()) {
        let (l_num, l_suffix) = latest_parts.get(i).cloned().unwrap_or((0, String::new()));
        let (c_num, c_suffix) = current_parts.get(i).cloned().unwrap_or((0, String::new()));
        
        if l_num > c_num {
            return true;
        } else if l_num < c_num {
            return false;
        }
        
        // Compare suffixes (empty > "beta" > "alpha")
        let suffix_order = |s: &str| -> i32 {
            if s.is_empty() { 3 }
            else if s.contains("rc") { 2 }
            else if s.contains("beta") { 1 }
            else if s.contains("alpha") { 0 }
            else { 3 }
        };
        
        let l_order = suffix_order(&l_suffix);
        let c_order = suffix_order(&c_suffix);
        
        if l_order > c_order {
            return true;
        } else if l_order < c_order {
            return false;
        }
    }
    
    false
}

// ============================================================================
// Perform Update
// ============================================================================

/// Request for performing update
#[derive(Debug, Deserialize)]
pub struct PerformUpdateRequest {
    /// Target version to update to (e.g. "0.2.0")
    pub version: String,
    /// Whether this is a beta release
    #[serde(default)]
    pub beta: bool,
}

/// Response for perform update
#[derive(Debug, Serialize)]
pub struct PerformUpdateResponse {
    pub success: bool,
    pub message: String,
}

/// Get the expected asset name for the current platform
fn get_platform_asset_name() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { return Some("noteva-linux-x86_64.tar.gz"); }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    { return Some("noteva-linux-arm64.tar.gz"); }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    { return Some("noteva-windows-x86_64.zip"); }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    { return Some("noteva-windows-arm64.zip"); }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { return Some("noteva-macos-x86_64.tar.gz"); }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { return Some("noteva-macos-arm64.tar.gz"); }
    #[allow(unreachable_code)]
    None
}

/// Extract binary from archive bytes
fn extract_binary(archive_bytes: &[u8], asset_name: &str) -> Result<Vec<u8>, String> {
    if asset_name.ends_with(".zip") {
        // ZIP archive (Windows)
        let cursor = std::io::Cursor::new(archive_bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| format!("Failed to open zip: {}", e))?;
        
        // Find the binary in the archive
        let binary_name = if cfg!(windows) { "noteva.exe" } else { "noteva" };
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;
            if file.name().ends_with(binary_name) {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut buf)
                    .map_err(|e| format!("Failed to read binary from zip: {}", e))?;
                return Ok(buf);
            }
        }
        Err("Binary not found in zip archive".to_string())
    } else {
        // tar.gz archive (Linux/macOS)
        let cursor = std::io::Cursor::new(archive_bytes);
        let gz = flate2::read::GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(gz);
        
        let binary_name = "noteva";
        for entry in archive.entries().map_err(|e| format!("Failed to read tar: {}", e))? {
            let mut entry = entry.map_err(|e| format!("Failed to read tar entry: {}", e))?;
            let path = entry.path().map_err(|e| format!("Failed to get path: {}", e))?;
            if path.file_name().map(|n| n == binary_name).unwrap_or(false) {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut buf)
                    .map_err(|e| format!("Failed to read binary from tar: {}", e))?;
                return Ok(buf);
            }
        }
        Err("Binary not found in tar archive".to_string())
    }
}

/// POST /api/v1/admin/update-perform - Download and apply update
///
/// Downloads the new binary from GitHub, replaces the current one, then exits.
/// The process manager (systemd/Docker restart policy) should restart the process.
async fn perform_update(
    _user: AuthenticatedUser,
    Json(body): Json<PerformUpdateRequest>,
) -> Result<Json<PerformUpdateResponse>, ApiError> {
    let target_version = body.version.trim_start_matches('v').to_string();
    let current = APP_VERSION.trim_start_matches('v');
    
    // Verify update is actually newer
    if !version_compare(&target_version, current) {
        return Err(ApiError::validation_error(format!(
            "Version {} is not newer than current {}", target_version, current
        )));
    }
    
    // Determine platform asset
    let asset_name = get_platform_asset_name()
        .ok_or_else(|| ApiError::internal_error("Unsupported platform for auto-update"))?;
    
    // Fetch release info to get download URL
    let tag = format!("v{}", target_version);
    let api_url = format!(
        "https://api.github.com/repos/noteva26/Noteva/releases/tags/{}",
        tag
    );
    
    let client = reqwest::Client::builder()
        .user_agent("Noteva-Updater")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ApiError::internal_error(format!("HTTP client error: {}", e)))?;
    
    let release: GitHubRelease = client
        .get(&api_url)
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to fetch release: {}", e)))?
        .json()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to parse release: {}", e)))?;
    
    // Find matching asset
    let asset = release.assets.iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| ApiError::internal_error(format!(
            "No asset found for platform: {}", asset_name
        )))?;
    
    tracing::info!("Downloading update: {} from {}", asset.name, asset.browser_download_url);
    
    // Download the archive
    let archive_bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to download: {}", e)))?
        .bytes()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to read download: {}", e)))?;
    
    tracing::info!("Downloaded {} bytes, extracting binary...", archive_bytes.len());
    
    // Extract binary from archive
    let binary_data = extract_binary(&archive_bytes, asset_name)
        .map_err(|e| ApiError::internal_error(e))?;
    
    // Get current executable path
    let self_path = std::env::current_exe()
        .map_err(|e| ApiError::internal_error(format!("Failed to get exe path: {}", e)))?;
    
    tracing::info!("Replacing binary at: {:?}", self_path);
    
    // Replace binary
    #[cfg(unix)]
    {
        let tmp_path = self_path.with_extension("new");
        std::fs::write(&tmp_path, &binary_data)
            .map_err(|e| ApiError::internal_error(format!("Failed to write new binary: {}", e)))?;
        
        // Set executable permission
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| ApiError::internal_error(format!("Failed to set permissions: {}", e)))?;
        
        std::fs::rename(&tmp_path, &self_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to replace binary: {}", e)))?;
    }
    
    #[cfg(windows)]
    {
        let old_path = self_path.with_extension("old.exe");
        let tmp_path = self_path.with_extension("new.exe");
        
        std::fs::write(&tmp_path, &binary_data)
            .map_err(|e| ApiError::internal_error(format!("Failed to write new binary: {}", e)))?;
        
        // Windows: rename current -> .old, then new -> current
        let _ = std::fs::remove_file(&old_path); // clean up previous .old if exists
        std::fs::rename(&self_path, &old_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to rename old binary: {}", e)))?;
        std::fs::rename(&tmp_path, &self_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to rename new binary: {}", e)))?;
    }
    
    tracing::info!("Update to v{} complete, scheduling restart...", target_version);
    
    // Schedule exit after response is sent
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        tracing::info!("Exiting for update restart...");
        std::process::exit(0);
    });
    
    Ok(Json(PerformUpdateResponse {
        success: true,
        message: format!("Updated to v{}. Restarting...", target_version),
    }))
}

// ============================================================================
// Comment Moderation
// ============================================================================

/// Query params for pending comments list
#[derive(Debug, Deserialize)]
pub struct PendingCommentsQuery {
    #[serde(default = "default_page_i64")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

/// Response for pending comments list
#[derive(Debug, Serialize)]
pub struct PendingCommentsResponse {
    pub comments: Vec<PendingCommentResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

/// Response for a pending comment
#[derive(Debug, Serialize)]
pub struct PendingCommentResponse {
    pub id: i64,
    pub article_id: i64,
    pub content: String,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

/// GET /api/v1/admin/comments/pending - List pending comments
pub async fn list_pending_comments(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(query): Query<PendingCommentsQuery>,
) -> Result<Json<PendingCommentsResponse>, ApiError> {
    let (comments, total) = state
        .comment_service
        .list_pending(query.page, query.per_page)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let total_pages = (total as f64 / query.per_page as f64).ceil() as i64;

    let comments: Vec<PendingCommentResponse> = comments
        .into_iter()
        .map(|c| PendingCommentResponse {
            id: c.id,
            article_id: c.article_id,
            content: c.content,
            nickname: c.nickname,
            email: c.email.clone(),
            avatar_url: Some(c.avatar_url),
            created_at: c.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(PendingCommentsResponse {
        comments,
        total,
        page: query.page,
        per_page: query.per_page,
        total_pages,
    }))
}

/// POST /api/v1/admin/comments/:id/approve - Approve a comment
pub async fn approve_comment(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let success = state
        .comment_service
        .approve(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if success {
        Ok(StatusCode::OK)
    } else {
        Err(ApiError::not_found("Comment not found"))
    }
}

/// POST /api/v1/admin/comments/:id/reject - Reject a comment
pub async fn reject_comment(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let success = state
        .comment_service
        .reject(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if success {
        Ok(StatusCode::OK)
    } else {
        Err(ApiError::not_found("Comment not found"))
    }
}


// ============================================================================
// Plugin Hot Reload
// ============================================================================

/// Response for plugin reload
#[derive(Debug, Serialize)]
pub struct ReloadResponse {
    pub success: bool,
    pub message: String,
    pub plugin_count: usize,
}

/// POST /api/v1/admin/plugins/reload - Reload plugins from disk
///
/// Rescans the plugins directory and loads any new plugins without restarting the server.
/// Requires admin authentication.
async fn reload_plugins(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ReloadResponse>, ApiError> {
    // Reload plugins
    let mut plugin_manager = state.plugin_manager.write().await;
    plugin_manager
        .reload()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to reload plugins: {}", e)))?;
    
    let plugin_count = plugin_manager.get_all().len();
    drop(plugin_manager);
    
    Ok(Json(ReloadResponse {
        success: true,
        message: "Plugins reloaded successfully".to_string(),
        plugin_count,
    }))
}

// ============================================================================
// Theme Hot Reload
// ============================================================================

/// POST /api/v1/admin/themes/reload - Reload themes from disk
///
/// Rescans the themes directory and refreshes the theme list without restarting the server.
/// Requires admin authentication.
async fn reload_themes(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ReloadResponse>, ApiError> {
    // Reload themes
    let mut theme_engine = state.theme_engine.write()
        .map_err(|e| ApiError::internal_error(format!("Failed to lock theme engine: {}", e)))?;
    theme_engine
        .refresh_themes()
        .map_err(|e| ApiError::internal_error(format!("Failed to reload themes: {}", e)))?;
    
    let theme_count = theme_engine.list_themes().len();
    drop(theme_engine);
    
    Ok(Json(ReloadResponse {
        success: true,
        message: "Themes reloaded successfully".to_string(),
        plugin_count: theme_count,
    }))
}


// ============================================================================
// Login Logs (Security)
// ============================================================================

/// Login log entry
#[derive(Debug, Serialize)]
pub struct LoginLogEntry {
    pub id: i64,
    pub username: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub created_at: String,
}

/// Query parameters for login logs
#[derive(Debug, Deserialize)]
pub struct LoginLogsQuery {
    #[serde(default = "default_page_i64")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    pub username: Option<String>,
    pub ip_address: Option<String>,
    pub success: Option<bool>,
}

/// Response for login logs list
#[derive(Debug, Serialize)]
pub struct LoginLogsResponse {
    pub logs: Vec<LoginLogEntry>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub success_count: i64,
    pub failed_count: i64,
}

/// GET /api/v1/admin/login-logs - List login logs
///
/// Query parameters:
/// - page: Page number (default: 1)
/// - per_page: Items per page (default: 20)
/// - username: Filter by username (optional)
/// - ip_address: Filter by IP address (optional)
/// - success: Filter by success status (optional)
///
/// Requires admin authentication.
async fn list_login_logs(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(query): Query<LoginLogsQuery>,
) -> Result<Json<LoginLogsResponse>, ApiError> {
    use crate::config::DatabaseDriver;
    
    let offset = (query.page - 1) * query.per_page;
    
    // Build WHERE clause
    let mut where_clauses = Vec::new();
    
    if query.username.is_some() {
        where_clauses.push("username LIKE ?".to_string());
    }
    if query.ip_address.is_some() {
        where_clauses.push("ip_address = ?".to_string());
    }
    if query.success.is_some() {
        where_clauses.push("success = ?".to_string());
    }
    
    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };
    
    // Get total count
    let count_sql = format!("SELECT COUNT(*) FROM login_logs {}", where_sql);
    let total: i64 = match state.pool.driver() {
        DatabaseDriver::Sqlite => {
            let mut query_builder = sqlx::query_scalar(&count_sql);
            if let Some(ref username) = query.username {
                query_builder = query_builder.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                query_builder = query_builder.bind(ip);
            }
            if let Some(success) = query.success {
                query_builder = query_builder.bind(if success { 1 } else { 0 });
            }
            query_builder
                .fetch_one(state.pool.as_sqlite().unwrap())
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?
        }
        DatabaseDriver::Mysql => {
            let mut query_builder = sqlx::query_scalar(&count_sql);
            if let Some(ref username) = query.username {
                query_builder = query_builder.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                query_builder = query_builder.bind(ip);
            }
            if let Some(success) = query.success {
                query_builder = query_builder.bind(if success { 1 } else { 0 });
            }
            query_builder
                .fetch_one(state.pool.as_mysql().unwrap())
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?
        }
    };
    
    // Get success/failed counts with same filters (excluding success filter)
    // We need to count success and failed separately, so we build WHERE clause without success filter
    let mut base_where_clauses = Vec::new();
    
    if query.username.is_some() {
        base_where_clauses.push("username LIKE ?".to_string());
    }
    if query.ip_address.is_some() {
        base_where_clauses.push("ip_address = ?".to_string());
    }
    
    let base_where_sql = if base_where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", base_where_clauses.join(" AND "))
    };
    
    // Build WHERE clause for success/failed counts
    let success_where = if base_where_sql.is_empty() {
        "WHERE success = 1".to_string()
    } else {
        format!("{} AND success = 1", base_where_sql)
    };
    let failed_where = if base_where_sql.is_empty() {
        "WHERE success = 0".to_string()
    } else {
        format!("{} AND success = 0", base_where_sql)
    };
    
    let success_count_sql = format!("SELECT COUNT(*) FROM login_logs {}", success_where);
    let failed_count_sql = format!("SELECT COUNT(*) FROM login_logs {}", failed_where);
    
    let (success_count, failed_count): (i64, i64) = match state.pool.driver() {
        DatabaseDriver::Sqlite => {
            let mut success_query = sqlx::query_scalar(&success_count_sql);
            let mut failed_query = sqlx::query_scalar(&failed_count_sql);
            
            // Bind parameters for success count (only username and ip, not success)
            if let Some(ref username) = query.username {
                success_query = success_query.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                success_query = success_query.bind(ip);
            }
            
            // Bind parameters for failed count (only username and ip, not success)
            if let Some(ref username) = query.username {
                failed_query = failed_query.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                failed_query = failed_query.bind(ip);
            }
            
            let success: i64 = success_query
                .fetch_one(state.pool.as_sqlite().unwrap())
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?;
            let failed: i64 = failed_query
                .fetch_one(state.pool.as_sqlite().unwrap())
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?;
            (success, failed)
        }
        DatabaseDriver::Mysql => {
            let mut success_query = sqlx::query_scalar(&success_count_sql);
            let mut failed_query = sqlx::query_scalar(&failed_count_sql);
            
            // Bind parameters for success count (only username and ip, not success)
            if let Some(ref username) = query.username {
                success_query = success_query.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                success_query = success_query.bind(ip);
            }
            
            // Bind parameters for failed count (only username and ip, not success)
            if let Some(ref username) = query.username {
                failed_query = failed_query.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                failed_query = failed_query.bind(ip);
            }
            
            let success: i64 = success_query
                .fetch_one(state.pool.as_mysql().unwrap())
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?;
            let failed: i64 = failed_query
                .fetch_one(state.pool.as_mysql().unwrap())
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?;
            (success, failed)
        }
    };
    
    // Get logs
    let logs_sql = format!(
        "SELECT id, username, ip_address, user_agent, success, failure_reason, {} FROM login_logs {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        match state.pool.driver() {
            DatabaseDriver::Sqlite => "created_at",
            DatabaseDriver::Mysql => "DATE_FORMAT(created_at, '%Y-%m-%dT%H:%i:%sZ') as created_at",
        },
        where_sql
    );
    
    let logs: Vec<LoginLogEntry> = match state.pool.driver() {
        DatabaseDriver::Sqlite => {
            let mut query_builder = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, i64, Option<String>, String)>(&logs_sql);
            if let Some(ref username) = query.username {
                query_builder = query_builder.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                query_builder = query_builder.bind(ip);
            }
            if let Some(success) = query.success {
                query_builder = query_builder.bind(if success { 1 } else { 0 });
            }
            query_builder
                .bind(query.per_page)
                .bind(offset)
                .fetch_all(state.pool.as_sqlite().unwrap())
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?
                .into_iter()
                .map(|(id, username, ip_address, user_agent, success, failure_reason, created_at)| {
                    LoginLogEntry {
                        id,
                        username,
                        ip_address,
                        user_agent,
                        success: success != 0,
                        failure_reason,
                        created_at,
                    }
                })
                .collect()
        }
        DatabaseDriver::Mysql => {
            let mut query_builder = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, i8, Option<String>, String)>(&logs_sql);
            if let Some(ref username) = query.username {
                query_builder = query_builder.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                query_builder = query_builder.bind(ip);
            }
            if let Some(success) = query.success {
                query_builder = query_builder.bind(if success { 1 } else { 0 });
            }
            query_builder
                .bind(query.per_page)
                .bind(offset)
                .fetch_all(state.pool.as_mysql().unwrap())
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?
                .into_iter()
                .map(|(id, username, ip_address, user_agent, success, failure_reason, created_at)| {
                    LoginLogEntry {
                        id,
                        username,
                        ip_address,
                        user_agent,
                        success: success != 0,
                        failure_reason,
                        created_at,
                    }
                })
                .collect()
        }
    };
    
    Ok(Json(LoginLogsResponse {
        logs,
        total,
        page: query.page,
        per_page: query.per_page,
        success_count,
        failed_count,
    }))
}
