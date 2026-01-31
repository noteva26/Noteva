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

/// Build the plugins router
pub fn router() -> Router<AppState> {
    Router::new()
        // Public routes (for frontend)
        .route("/assets/plugins.js", get(get_plugins_js))
        .route("/assets/plugins.css", get(get_plugins_css))
        .route("/enabled", get(get_enabled_plugins))
        // Admin routes
        .route("/", get(list_plugins))
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
    let manager = state.plugin_manager.read()
        .map_err(|e| ApiError::internal_error(format!("Failed to acquire plugin lock: {}", e)))?;
    
    let plugins: Vec<PluginResponse> = manager.get_all()
        .iter()
        .map(|p| PluginResponse {
            id: p.metadata.id.clone(),
            name: p.metadata.name.clone(),
            version: p.metadata.version.clone(),
            description: p.metadata.description.clone(),
            author: p.metadata.author.clone(),
            enabled: p.enabled,
            has_settings: p.metadata.settings,
            shortcodes: p.metadata.shortcodes.clone(),
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
    let manager = state.plugin_manager.read()
        .map_err(|e| ApiError::internal_error(format!("Failed to acquire plugin lock: {}", e)))?;
    
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
    let manager = state.plugin_manager.read()
        .map_err(|e| ApiError::internal_error(format!("Failed to acquire plugin lock: {}", e)))?;
    
    let plugin = manager.get(&id)
        .ok_or_else(|| ApiError::not_found(format!("Plugin not found: {}", id)))?;
    
    Ok(Json(PluginResponse {
        id: plugin.metadata.id.clone(),
        name: plugin.metadata.name.clone(),
        version: plugin.metadata.version.clone(),
        description: plugin.metadata.description.clone(),
        author: plugin.metadata.author.clone(),
        enabled: plugin.enabled,
        has_settings: plugin.metadata.settings,
        shortcodes: plugin.metadata.shortcodes.clone(),
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
    
    let mut manager = state.plugin_manager.write()
        .map_err(|e| ApiError::internal_error(format!("Failed to acquire plugin lock: {}", e)))?;
    
    if body.enabled {
        manager.enable(&id)
            .map_err(|e| ApiError::internal_error(e.to_string()))?;
        
        // Trigger plugin_activate hook
        state.hook_manager.trigger(
            hook_names::PLUGIN_ACTIVATE,
            serde_json::json!({ "plugin_id": id })
        );
    } else {
        // Trigger plugin_deactivate hook before disabling
        state.hook_manager.trigger(
            hook_names::PLUGIN_DEACTIVATE,
            serde_json::json!({ "plugin_id": id })
        );
        
        manager.disable(&id)
            .map_err(|e| ApiError::internal_error(e.to_string()))?;
    }
    
    let plugin = manager.get(&id)
        .ok_or_else(|| ApiError::not_found(format!("Plugin not found: {}", id)))?;
    
    Ok(Json(PluginResponse {
        id: plugin.metadata.id.clone(),
        name: plugin.metadata.name.clone(),
        version: plugin.metadata.version.clone(),
        description: plugin.metadata.description.clone(),
        author: plugin.metadata.author.clone(),
        enabled: plugin.enabled,
        has_settings: plugin.metadata.settings,
        shortcodes: plugin.metadata.shortcodes.clone(),
    }))
}

/// GET /api/v1/plugins/:id/settings - Get plugin settings schema
async fn get_plugin_settings(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let manager = state.plugin_manager.read()
        .map_err(|e| ApiError::internal_error(format!("Failed to acquire plugin lock: {}", e)))?;
    
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
    let mut manager = state.plugin_manager.write()
        .map_err(|e| ApiError::internal_error(format!("Failed to acquire plugin lock: {}", e)))?;
    
    // Convert body to HashMap
    let settings: std::collections::HashMap<String, serde_json::Value> = body
        .as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    
    // Update settings (this also persists to file)
    manager.update_settings(&id, settings.clone())
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
    let manager = match state.plugin_manager.read() {
        Ok(m) => m,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("// Error loading plugins"))
                .unwrap();
        }
    };
    
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
    let manager = match state.plugin_manager.read() {
        Ok(m) => m,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("/* Error loading plugins */"))
                .unwrap();
        }
    };
    
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
