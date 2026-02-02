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
        // User management
        .route("/users", get(list_users))
        .route("/users/:id", get(get_user))
        .route("/users/:id", put(update_user))
        .route("/users/:id", delete(delete_user))
        // Comment moderation
        .route("/comments/pending", get(list_pending_comments))
        .route("/comments/:id/approve", post(approve_comment))
        .route("/comments/:id/reject", post(reject_comment))
        // Email test
        .route("/email/test", post(test_email))
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
        display_name: theme_info.map(|i| i.display_name.clone()).unwrap_or_else(|| actual_theme.clone()),
        description: theme_info.and_then(|i| i.description.clone()),
        version: theme_info.map(|i| i.version.clone()).unwrap_or_else(|| "1.0.0".to_string()),
        author: theme_info.and_then(|i| i.author.clone()),
        url: theme_info.and_then(|i| i.url.clone()),
        preview: theme_info.and_then(|i| i.preview.clone()),
        active: true,
        requires_noteva: theme_info.map(|i| i.requires_noteva.clone()).unwrap_or_default(),
        compatible: theme_info.map(|i| i.compatible).unwrap_or(true),
        compatibility_message: theme_info.and_then(|i| i.compatibility_message.clone()),
    }))
}

/// Store theme info from official repository
#[derive(Debug, Serialize, Deserialize)]
pub struct StoreThemeInfo {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub url: String,
    pub preview: Option<String>,
    pub requires_noteva: String,
    pub compatible: bool,
    pub compatibility_message: Option<String>,
    pub installed: bool,
}

/// Theme store response
#[derive(Debug, Serialize)]
pub struct ThemeStoreResponse {
    pub themes: Vec<StoreThemeInfo>,
}

/// GET /api/v1/admin/themes/store - Get theme store from official repository
#[axum::debug_handler]
async fn get_theme_store(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ThemeStoreResponse>, ApiError> {
    use crate::plugin::loader::{check_version_requirement, NOTEVA_VERSION};
    
    const STORE_REPO: &str = "noteva26/noteva-themes";
    const STORE_PATH: &str = "store";
    
    let client = reqwest::Client::new();
    
    // Get the tree of store directory
    let tree_url = format!(
        "https://api.github.com/repos/{}/git/trees/main?recursive=1",
        STORE_REPO
    );
    
    let tree_response = client
        .get(&tree_url)
        .header("User-Agent", "Noteva")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to fetch store: {}", e)))?;
    
    if !tree_response.status().is_success() {
        return Ok(Json(ThemeStoreResponse { themes: vec![] }));
    }
    
    let tree_json: serde_json::Value = tree_response
        .json()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to parse tree: {}", e)))?;
    
    // Find all theme.json files in root directory (official themes) and store/ (third-party themes)
    let mut theme_paths = Vec::new();
    if let Some(tree_array) = tree_json.get("tree").and_then(|v| v.as_array()) {
        for item in tree_array {
            if let (Some(path), Some(item_type)) = (
                item.get("path").and_then(|v| v.as_str()),
                item.get("type").and_then(|v| v.as_str()),
            ) {
                if item_type != "blob" || !path.ends_with("/theme.json") {
                    continue;
                }
                
                // Include root-level themes (e.g., "default/theme.json")
                // and store themes (e.g., "store/some-theme/theme.json")
                let parts: Vec<&str> = path.split('/').collect();
                if parts.len() == 2 {
                    // Root level: theme-name/theme.json
                    theme_paths.push(path.to_string());
                } else if parts.len() == 3 && parts[0] == STORE_PATH {
                    // Store level: store/theme-name/theme.json
                    theme_paths.push(path.to_string());
                }
            }
        }
    }
    
    // Get installed themes
    let installed_names: std::collections::HashSet<String> = {
        let theme_engine = state
            .theme_engine
            .read()
            .map_err(|e| ApiError::internal_error(format!("Failed to acquire theme lock: {}", e)))?;
        
        let installed_themes = theme_engine.list_themes();
        
        installed_themes
            .iter()
            .map(|t| t.name.clone())
            .collect()
    };
    
    let mut themes = Vec::new();
    
    // Fetch each theme.json
    for path in theme_paths {
        let raw_url = format!(
            "https://raw.githubusercontent.com/{}/main/{}",
            STORE_REPO, path
        );
        
        if let Ok(response) = client
            .get(&raw_url)
            .header("User-Agent", "Noteva")
            .send()
            .await
        {
            if let Ok(theme_json) = response.json::<serde_json::Value>().await {
                let name = theme_json.get("short")
                    .or_else(|| theme_json.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                if name.is_empty() {
                    continue;
                }
                
                let requires_noteva = theme_json.get("requires")
                    .and_then(|r| r.get("noteva"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let version_check = check_version_requirement(&requires_noteva, NOTEVA_VERSION);
                
                themes.push(StoreThemeInfo {
                    name: name.clone(),
                    display_name: theme_json.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&name)
                        .to_string(),
                    version: theme_json.get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1.0.0")
                        .to_string(),
                    description: theme_json.get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    author: theme_json.get("author")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    url: theme_json.get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    preview: theme_json.get("preview")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    requires_noteva,
                    compatible: version_check.compatible,
                    compatibility_message: version_check.message,
                    installed: installed_names.contains(&name),
                });
            }
        }
    }
    
    Ok(Json(ThemeStoreResponse { themes }))
}

/// GET /api/v1/admin/themes/updates - Check for theme updates
#[axum::debug_handler]
async fn check_theme_updates(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ThemeUpdatesResponse>, ApiError> {
    const STORE_REPO: &str = "noteva26/noteva-themes";
    const STORE_PATH: &str = "store";
    
    let client = reqwest::Client::new();
    
    // Get the tree of store directory
    let tree_url = format!(
        "https://api.github.com/repos/{}/git/trees/main?recursive=1",
        STORE_REPO
    );
    
    let tree_response = client
        .get(&tree_url)
        .header("User-Agent", "Noteva")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to fetch store: {}", e)))?;
    
    if !tree_response.status().is_success() {
        return Ok(Json(ThemeUpdatesResponse { updates: vec![] }));
    }
    
    #[derive(Deserialize)]
    struct GitHubTreeItem {
        path: String,
        #[serde(rename = "type")]
        item_type: String,
    }
    
    #[derive(Deserialize)]
    struct GitHubTreeResponse {
        tree: Vec<GitHubTreeItem>,
    }
    
    let tree: GitHubTreeResponse = tree_response
        .json()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to parse tree: {}", e)))?;
    
    // Find all theme.json files
    let theme_paths: Vec<String> = tree.tree
        .iter()
        .filter(|item| {
            if item.item_type != "blob" || !item.path.ends_with("/theme.json") {
                return false;
            }
            
            let parts: Vec<&str> = item.path.split('/').collect();
            if parts.len() == 2 {
                return true;
            } else if parts.len() == 3 && parts[0] == STORE_PATH {
                return true;
            }
            false
        })
        .map(|item| item.path.clone())
        .collect();
    
    // Get installed themes
    let installed_themes: std::collections::HashMap<String, String> = {
        let theme_engine = state
            .theme_engine
            .read()
            .map_err(|e| ApiError::internal_error(format!("Failed to acquire theme lock: {}", e)))?;
        
        theme_engine
            .list_themes()
            .iter()
            .map(|t| (t.name.clone(), t.version.clone()))
            .collect()
    };
    
    let mut updates = Vec::new();
    
    // Check each installed theme for updates
    for path in theme_paths {
        let raw_url = format!(
            "https://raw.githubusercontent.com/{}/main/{}",
            STORE_REPO, path
        );
        
        if let Ok(response) = client
            .get(&raw_url)
            .header("User-Agent", "Noteva")
            .send()
            .await
        {
            if let Ok(theme_json) = response.json::<serde_json::Value>().await {
                let name = theme_json.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                if name.is_empty() {
                    continue;
                }
                
                // Check if this theme is installed
                if let Some(current_version) = installed_themes.get(&name) {
                    let latest_version = theme_json.get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1.0.0")
                        .to_string();
                    
                    // Compare versions
                    let has_update = compare_versions(&latest_version, current_version);
                    
                    if has_update {
                        updates.push(ThemeUpdateInfo {
                            name,
                            current_version: current_version.clone(),
                            latest_version,
                            has_update: true,
                        });
                    }
                }
            }
        }
    }
    
    Ok(Json(ThemeUpdatesResponse { updates }))
}

/// Compare two semantic versions (returns true if v1 > v2)
fn compare_versions(v1: &str, v2: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    };
    
    let ver1 = parse_version(v1);
    let ver2 = parse_version(v2);
    
    for i in 0..ver1.len().max(ver2.len()) {
        let n1 = ver1.get(i).copied().unwrap_or(0);
        let n2 = ver2.get(i).copied().unwrap_or(0);
        
        if n1 > n2 {
            return true;
        } else if n1 < n2 {
            return false;
        }
    }
    
    false
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
    prerelease: bool,
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
// User Management API
// ============================================================================

/// Response for user list
#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub users: Vec<UserAdminResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

/// Response for a user in admin context
#[derive(Debug, Serialize)]
pub struct UserAdminResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub status: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::models::User> for UserAdminResponse {
    fn from(user: crate::models::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role.to_string(),
            status: user.status.to_string(),
            display_name: user.display_name,
            avatar: user.avatar,
            created_at: user.created_at.to_rfc3339(),
            updated_at: user.updated_at.to_rfc3339(),
        }
    }
}

/// Request for updating a user
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    pub display_name: Option<String>,
}

/// Query params for user list
#[derive(Debug, Deserialize)]
pub struct UserListQuery {
    #[serde(default = "default_page_i64")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

/// GET /api/v1/admin/users - List all users
pub async fn list_users(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    axum::extract::Query(query): axum::extract::Query<UserListQuery>,
) -> Result<Json<UserListResponse>, ApiError> {
    let (users, total) = state
        .user_repo
        .list(query.page, query.per_page)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let total_pages = (total as f64 / query.per_page as f64).ceil() as i64;

    Ok(Json(UserListResponse {
        users: users.into_iter().map(|u| u.into()).collect(),
        total,
        page: query.page,
        per_page: query.per_page,
        total_pages,
    }))
}

/// GET /api/v1/admin/users/:id - Get a user by ID
pub async fn get_user(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<Json<UserAdminResponse>, ApiError> {
    let user = state
        .user_repo
        .get_by_id(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("User not found"))?;

    Ok(Json(user.into()))
}

/// PUT /api/v1/admin/users/:id - Update a user
pub async fn update_user(
    State(state): State<AppState>,
    current_user: AuthenticatedUser,
    Path(id): Path<i64>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<UserAdminResponse>, ApiError> {
    use std::str::FromStr;
    use crate::models::{UserRole, UserStatus};

    tracing::info!("Updating user {}: {:?}", id, body);

    // Get existing user
    let mut user = state
        .user_repo
        .get_by_id(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("User not found"))?;

    tracing::info!("Found user: {} ({})", user.username, user.email);

    // Prevent self-demotion or self-ban for admins
    if current_user.0.id == id {
        if let Some(ref role) = body.role {
            if role != "admin" {
                return Err(ApiError::validation_error("Cannot change your own role"));
            }
        }
        if let Some(ref status) = body.status {
            if status == "banned" {
                return Err(ApiError::validation_error("Cannot ban yourself"));
            }
        }
    }

    // Update fields
    if let Some(username) = body.username {
        // Check if username is taken by another user
        if let Some(existing) = state.user_repo.get_by_username(&username).await
            .map_err(|e| ApiError::internal_error(e.to_string()))? 
        {
            if existing.id != id {
                return Err(ApiError::validation_error("Username already taken"));
            }
        }
        user.username = username;
    }

    if let Some(email) = body.email {
        // Check if email is taken by another user
        if let Some(existing) = state.user_repo.get_by_email(&email).await
            .map_err(|e| ApiError::internal_error(e.to_string()))? 
        {
            if existing.id != id {
                return Err(ApiError::validation_error("Email already taken"));
            }
        }
        user.email = email;
    }

    if let Some(role) = body.role {
        user.role = UserRole::from_str(&role)
            .map_err(|_| ApiError::validation_error("Invalid role"))?;
    }

    if let Some(status) = body.status {
        user.status = UserStatus::from_str(&status)
            .map_err(|_| ApiError::validation_error("Invalid status"))?;
    }

    if let Some(display_name) = body.display_name {
        user.display_name = if display_name.is_empty() { None } else { Some(display_name) };
    }

    tracing::info!("Saving user: {} ({}) role={:?} status={:?}", user.username, user.email, user.role, user.status);

    // Save changes
    let updated = state
        .user_repo
        .update(&user)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    tracing::info!("User updated successfully: {} ({})", updated.username, updated.email);

    Ok(Json(updated.into()))
}

/// DELETE /api/v1/admin/users/:id - Delete a user
pub async fn delete_user(
    State(state): State<AppState>,
    current_user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    // Prevent self-deletion
    if current_user.0.id == id {
        return Err(ApiError::validation_error("Cannot delete yourself"));
    }

    // Check if user exists
    let _ = state
        .user_repo
        .get_by_id(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("User not found"))?;

    // Delete user
    state
        .user_repo
        .delete(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
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
// Email Test
// ============================================================================

/// Request for testing email
#[derive(Debug, Deserialize)]
pub struct TestEmailRequest {
    pub email: String,
}

/// POST /api/v1/admin/email/test - Send test email
pub async fn test_email(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<TestEmailRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use crate::db::repositories::SqlxSettingsRepository;
    use crate::services::EmailService;
    use std::sync::Arc;

    let settings_repo = Arc::new(SqlxSettingsRepository::new(state.pool.clone()));
    let email_service = EmailService::new(settings_repo);

    email_service.send_test_email(&body.email).await
        .map_err(|e| ApiError::internal_error(format!("Failed to send test email: {}", e)))?;

    Ok(Json(serde_json::json!({ "message": "Test email sent successfully" })))
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
