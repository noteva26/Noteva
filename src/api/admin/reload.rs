//! Plugin and theme hot-reload endpoints

use axum::{extract::State, Json};
use serde::Serialize;

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};

/// Response for plugin/theme reload
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
pub async fn reload_plugins(
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

/// POST /api/v1/admin/themes/reload - Reload themes from disk
///
/// Rescans the themes directory and refreshes the theme list without restarting the server.
/// Requires admin authentication.
pub async fn reload_themes(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ReloadResponse>, ApiError> {
    // Reload themes
    let mut theme_engine = state
        .theme_engine
        .write()
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
