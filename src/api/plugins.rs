//! Plugin API endpoints
//!
//! Handles HTTP requests for plugin management:
//! - List plugins
//! - Enable/disable plugins
//! - Plugin settings
//! - Plugin assets (JS/CSS)

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};
use crate::plugin::loader::{check_version_requirement, NOTEVA_VERSION};

/// Plugin info response
#[derive(Debug, Serialize)]
pub struct PluginResponse {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub enabled: bool,
    pub has_settings: bool,
    pub shortcodes: Vec<String>,
    pub requires_noteva: String,
    pub compatible: bool,
    pub compatibility_message: Option<String>,
}

/// Plugin list response
#[derive(Debug, Serialize)]
pub struct PluginListResponse {
    pub plugins: Vec<PluginResponse>,
}

/// Plugin enable/disable request
#[derive(Debug, Deserialize)]
pub struct PluginToggleRequest {
    pub enabled: bool,
}

/// Plugin update info
#[derive(Debug, Serialize)]
pub struct PluginUpdateInfo {
    pub id: String,
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
}

/// Plugin updates response
#[derive(Debug, Serialize)]
pub struct PluginUpdatesResponse {
    pub updates: Vec<PluginUpdateInfo>,
}

/// Build the plugins router
pub fn router() -> Router<AppState> {
    Router::new()
        // Public routes (for frontend)
        .route("/assets/plugins.js", get(get_plugins_js))
        .route("/assets/plugins.css", get(get_plugins_css))
        .route("/enabled", get(get_enabled_plugins))
        // Admin routes
        .route("/", get(list_plugins))
        .route("/store", get(get_plugin_store))
        .route("/updates", get(check_plugin_updates))
        .route("/:id", get(get_plugin))
        .route("/:id/toggle", post(toggle_plugin))
        .route("/:id/settings", get(get_plugin_settings))
        .route("/:id/settings", post(update_plugin_settings))
}

/// GET /api/v1/plugins - List all plugins
async fn list_plugins(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<PluginListResponse>, ApiError> {
    let manager = state.plugin_manager.read().await;
    
    let plugins: Vec<PluginResponse> = manager.get_all()
        .iter()
        .map(|p| {
            let version_check = check_version_requirement(&p.metadata.requires.noteva, NOTEVA_VERSION);
            PluginResponse {
                id: p.metadata.id.clone(),
                name: p.metadata.name.clone(),
                version: p.metadata.version.clone(),
                description: p.metadata.description.clone(),
                author: p.metadata.author.clone(),
                enabled: p.enabled,
                has_settings: p.metadata.settings,
                shortcodes: p.metadata.shortcodes.clone(),
                requires_noteva: p.metadata.requires.noteva.clone(),
                compatible: version_check.compatible,
                compatibility_message: version_check.message,
            }
        })
        .collect();
    
    Ok(Json(PluginListResponse { plugins }))
}

/// Enabled plugin info for frontend (public)
#[derive(Debug, Serialize)]
pub struct EnabledPluginInfo {
    pub id: String,
    pub settings: std::collections::HashMap<String, serde_json::Value>,
}

/// GET /api/v1/plugins/enabled - Get enabled plugins with settings (public, no auth)
async fn get_enabled_plugins(
    State(state): State<AppState>,
) -> Result<Json<Vec<EnabledPluginInfo>>, ApiError> {
    let manager = state.plugin_manager.read().await;
    
    let plugins: Vec<EnabledPluginInfo> = manager.get_enabled()
        .iter()
        .map(|p| EnabledPluginInfo {
            id: p.metadata.id.clone(),
            settings: p.settings.clone(),
        })
        .collect();
    
    Ok(Json(plugins))
}

/// GET /api/v1/plugins/:id - Get plugin details
async fn get_plugin(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<Json<PluginResponse>, ApiError> {
    let manager = state.plugin_manager.read().await;
    
    let plugin = manager.get(&id)
        .ok_or_else(|| ApiError::not_found(format!("Plugin not found: {}", id)))?;
    
    let version_check = check_version_requirement(&plugin.metadata.requires.noteva, NOTEVA_VERSION);
    
    Ok(Json(PluginResponse {
        id: plugin.metadata.id.clone(),
        name: plugin.metadata.name.clone(),
        version: plugin.metadata.version.clone(),
        description: plugin.metadata.description.clone(),
        author: plugin.metadata.author.clone(),
        enabled: plugin.enabled,
        has_settings: plugin.metadata.settings,
        shortcodes: plugin.metadata.shortcodes.clone(),
        requires_noteva: plugin.metadata.requires.noteva.clone(),
        compatible: version_check.compatible,
        compatibility_message: version_check.message,
    }))
}

/// POST /api/v1/plugins/:id/toggle - Enable/disable plugin
/// 
/// # Hooks
/// - `plugin_activate` - Triggered when a plugin is enabled
/// - `plugin_deactivate` - Triggered when a plugin is disabled
async fn toggle_plugin(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
    Json(body): Json<PluginToggleRequest>,
) -> Result<Json<PluginResponse>, ApiError> {
    use crate::plugin::hook_names;
    
    {
        let mut manager = state.plugin_manager.write().await;
        
        if body.enabled {
            manager.enable(&id).await
                .map_err(|e| ApiError::internal_error(e.to_string()))?;
        } else {
            manager.disable(&id).await
                .map_err(|e| ApiError::internal_error(e.to_string()))?;
        }
    }
    
    // Trigger hooks after releasing lock
    if body.enabled {
        state.hook_manager.trigger(
            hook_names::PLUGIN_ACTIVATE,
            serde_json::json!({ "plugin_id": id })
        );
    } else {
        state.hook_manager.trigger(
            hook_names::PLUGIN_DEACTIVATE,
            serde_json::json!({ "plugin_id": id })
        );
    }
    
    // Re-acquire read lock to get updated plugin info
    let manager = state.plugin_manager.read().await;
    let plugin = manager.get(&id)
        .ok_or_else(|| ApiError::not_found(format!("Plugin not found: {}", id)))?;
    
    let version_check = check_version_requirement(&plugin.metadata.requires.noteva, NOTEVA_VERSION);
    
    Ok(Json(PluginResponse {
        id: plugin.metadata.id.clone(),
        name: plugin.metadata.name.clone(),
        version: plugin.metadata.version.clone(),
        description: plugin.metadata.description.clone(),
        author: plugin.metadata.author.clone(),
        enabled: plugin.enabled,
        has_settings: plugin.metadata.settings,
        shortcodes: plugin.metadata.shortcodes.clone(),
        requires_noteva: plugin.metadata.requires.noteva.clone(),
        compatible: version_check.compatible,
        compatibility_message: version_check.message,
    }))
}

/// GET /api/v1/plugins/:id/settings - Get plugin settings schema
async fn get_plugin_settings(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let manager = state.plugin_manager.read().await;
    
    let plugin = manager.get(&id)
        .ok_or_else(|| ApiError::not_found(format!("Plugin not found: {}", id)))?;
    
    let schema = plugin.get_settings_schema()
        .unwrap_or(serde_json::json!({}));
    
    Ok(Json(serde_json::json!({
        "schema": schema,
        "values": plugin.settings
    })))
}

/// POST /api/v1/plugins/:id/settings - Update plugin settings
async fn update_plugin_settings(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut manager = state.plugin_manager.write().await;
    
    // Convert body to HashMap
    let settings: std::collections::HashMap<String, serde_json::Value> = body
        .as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    
    // Update settings (this also persists to database)
    manager.update_settings(&id, settings.clone()).await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "settings": settings
    })))
}

/// GET /api/v1/plugins/assets/plugins.js - Combined JS for all enabled plugins
async fn get_plugins_js(
    State(state): State<AppState>,
) -> Response {
    let manager = state.plugin_manager.read().await;
    let js = manager.get_combined_frontend_js();
    
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/javascript")
        .header(header::CACHE_CONTROL, "public, max-age=300")
        .body(Body::from(js))
        .unwrap()
}

/// GET /api/v1/plugins/assets/plugins.css - Combined CSS for all enabled plugins
async fn get_plugins_css(
    State(state): State<AppState>,
) -> Response {
    let manager = state.plugin_manager.read().await;
    let css = manager.get_combined_frontend_css();
    
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/css")
        .header(header::CACHE_CONTROL, "public, max-age=300")
        .body(Body::from(css))
        .unwrap()
}

// Public handlers (no auth required)

/// GET /api/v1/plugins/assets/plugins.js - Public endpoint
pub async fn get_plugins_js_public(
    State(state): State<AppState>,
) -> Response {
    get_plugins_js(State(state)).await
}

/// GET /api/v1/plugins/assets/plugins.css - Public endpoint
pub async fn get_plugins_css_public(
    State(state): State<AppState>,
) -> Response {
    get_plugins_css(State(state)).await
}

/// GET /api/v1/plugins/enabled - Public endpoint for enabled plugins with settings
pub async fn get_enabled_plugins_public(
    State(state): State<AppState>,
) -> Result<Json<Vec<EnabledPluginInfo>>, ApiError> {
    get_enabled_plugins(State(state)).await
}

/// Store plugin info from official repository
#[derive(Debug, Serialize, Deserialize)]
pub struct StorePluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub homepage: String,
    pub license: Option<String>,
    pub requires_noteva: String,
    pub compatible: bool,
    pub compatibility_message: Option<String>,
    pub installed: bool,
}

/// Store response
#[derive(Debug, Serialize)]
pub struct PluginStoreResponse {
    pub plugins: Vec<StorePluginInfo>,
}

/// GitHub tree item
#[derive(Debug, Deserialize)]
struct GitHubTreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
}

/// GitHub tree response
#[derive(Debug, Deserialize)]
struct GitHubTreeResponse {
    tree: Vec<GitHubTreeItem>,
}

/// GET /api/v1/plugins/store - Get plugin store from official repository
async fn get_plugin_store(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<PluginStoreResponse>, ApiError> {
    const STORE_REPO: &str = "noteva26/noteva-plugins";
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
        return Ok(Json(PluginStoreResponse { plugins: vec![] }));
    }
    
    let tree: GitHubTreeResponse = tree_response
        .json()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to parse tree: {}", e)))?;
    
    // Find all plugin.json files in root directory (official plugins) and store/ (third-party plugins)
    let plugin_paths: Vec<String> = tree.tree
        .iter()
        .filter(|item| {
            if item.item_type != "blob" || !item.path.ends_with("/plugin.json") {
                return false;
            }
            
            // Include root-level plugins (e.g., "hide-until-reply/plugin.json")
            // and store plugins (e.g., "store/some-plugin/plugin.json")
            let parts: Vec<&str> = item.path.split('/').collect();
            if parts.len() == 2 {
                // Root level: plugin-name/plugin.json
                return true;
            } else if parts.len() == 3 && parts[0] == STORE_PATH {
                // Store level: store/plugin-name/plugin.json
                return true;
            }
            false
        })
        .map(|item| item.path.clone())
        .collect();
    
    // Get installed plugins
    let manager = state.plugin_manager.read().await;
    let installed_ids: std::collections::HashSet<String> = manager
        .get_all()
        .iter()
        .map(|p| p.metadata.id.clone())
        .collect();
    
    let mut plugins = Vec::new();
    
    // Fetch each plugin.json
    for path in plugin_paths {
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
            if let Ok(plugin_json) = response.json::<serde_json::Value>().await {
                let id = plugin_json.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                if id.is_empty() {
                    continue;
                }
                
                let requires_noteva = plugin_json.get("requires")
                    .and_then(|r| r.get("noteva"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let version_check = check_version_requirement(&requires_noteva, NOTEVA_VERSION);
                
                plugins.push(StorePluginInfo {
                    id: id.clone(),
                    name: plugin_json.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&id)
                        .to_string(),
                    version: plugin_json.get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1.0.0")
                        .to_string(),
                    description: plugin_json.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    author: plugin_json.get("author")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    homepage: plugin_json.get("homepage")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    license: plugin_json.get("license")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    requires_noteva,
                    compatible: version_check.compatible,
                    compatibility_message: version_check.message,
                    installed: installed_ids.contains(&id),
                });
            }
        }
    }
    
    Ok(Json(PluginStoreResponse { plugins }))
}


/// GET /api/v1/plugins/updates - Check for plugin updates
async fn check_plugin_updates(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<PluginUpdatesResponse>, ApiError> {
    const STORE_REPO: &str = "noteva26/noteva-plugins";
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
        return Ok(Json(PluginUpdatesResponse { updates: vec![] }));
    }
    
    let tree: GitHubTreeResponse = tree_response
        .json()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to parse tree: {}", e)))?;
    
    // Find all plugin.json files
    let plugin_paths: Vec<String> = tree.tree
        .iter()
        .filter(|item| {
            if item.item_type != "blob" || !item.path.ends_with("/plugin.json") {
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
    
    // Get installed plugins
    let manager = state.plugin_manager.read().await;
    let installed_plugins: std::collections::HashMap<String, String> = manager
        .get_all()
        .iter()
        .map(|p| (p.metadata.id.clone(), p.metadata.version.clone()))
        .collect();
    
    let mut updates = Vec::new();
    
    // Check each installed plugin for updates
    for path in plugin_paths {
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
            if let Ok(plugin_json) = response.json::<serde_json::Value>().await {
                let id = plugin_json.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                if id.is_empty() {
                    continue;
                }
                
                // Check if this plugin is installed
                if let Some(current_version) = installed_plugins.get(&id) {
                    let latest_version = plugin_json.get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1.0.0")
                        .to_string();
                    
                    // Compare versions
                    let has_update = compare_versions(&latest_version, current_version);
                    
                    if has_update {
                        updates.push(PluginUpdateInfo {
                            id,
                            current_version: current_version.clone(),
                            latest_version,
                            has_update: true,
                        });
                    }
                }
            }
        }
    }
    
    Ok(Json(PluginUpdatesResponse { updates }))
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

