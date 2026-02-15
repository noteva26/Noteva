//! Theme API endpoints
//!
//! Provides endpoints for theme configuration and information.

use axum::{
    extract::State,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::api::{AppState, ApiError};

/// Theme configuration response
#[derive(Debug, Serialize, Deserialize)]
pub struct ThemeConfigResponse {
    /// Theme name
    pub name: String,
    /// Theme configuration from theme.json
    pub config: HashMap<String, serde_json::Value>,
}

/// GET /api/v1/theme/config - Get current theme configuration
///
/// Returns the current theme's configuration from theme.json
pub async fn get_theme_config(
    State(state): State<AppState>,
) -> Result<Json<ThemeConfigResponse>, ApiError> {
    let theme_engine = state
        .theme_engine
        .read()
        .map_err(|e| ApiError::internal_error(format!("Failed to acquire theme lock: {}", e)))?;

    let theme_name = theme_engine.get_current_theme().to_string();
    
    // Get theme info which includes the full theme.json
    let theme_info = theme_engine.get_theme_info(&theme_name);
    
    let config = if let Some(info) = theme_info {
        // Extract config from theme info
        // The theme.json is already parsed, we can access its fields
        let mut config_map = HashMap::new();
        
        // Add basic theme info
        config_map.insert("name".to_string(), serde_json::json!(info.name));
        config_map.insert("display_name".to_string(), serde_json::json!(info.display_name));
        config_map.insert("version".to_string(), serde_json::json!(info.version));
        
        if let Some(desc) = &info.description {
            config_map.insert("description".to_string(), serde_json::json!(desc));
        }
        if let Some(author) = &info.author {
            config_map.insert("author".to_string(), serde_json::json!(author));
        }
        if let Some(url) = &info.url {
            config_map.insert("url".to_string(), serde_json::json!(url));
        }
        
        // Add custom config if exists
        if let Some(custom_config) = &info.config {
            config_map.insert("custom".to_string(), custom_config.clone());
        }
        
        config_map
    } else {
        HashMap::new()
    };

    Ok(Json(ThemeConfigResponse {
        name: theme_name,
        config,
    }))
}

/// GET /api/v1/theme/info - Get current theme information
///
/// Returns detailed information about the current theme
pub async fn get_theme_info(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let theme_engine = state
        .theme_engine
        .read()
        .map_err(|e| ApiError::internal_error(format!("Failed to acquire theme lock: {}", e)))?;

    let theme_name = theme_engine.get_current_theme().to_string();
    let theme_info = theme_engine.get_theme_info(&theme_name);
    
    if let Some(info) = theme_info {
        Ok(Json(serde_json::json!({
            "name": info.name,
            "display_name": info.display_name,
            "version": info.version,
            "description": info.description,
            "author": info.author,
            "url": info.url,
            "preview": info.preview,
            "requires_noteva": info.requires_noteva,
            "compatible": info.compatible,
            "compatibility_message": info.compatibility_message,
            "config": info.config,
            "has_settings": info.has_settings,
        })))
    } else {
        Ok(Json(serde_json::json!({
            "name": theme_name,
            "display_name": theme_name,
            "version": "1.0.0",
        })))
    }
}

/// GET /api/v1/theme/settings - Get current theme's public settings
///
/// Returns non-secret settings for the active theme. No authentication required.
/// Used by theme frontend to read settings like license tokens.
pub async fn get_theme_settings_public(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (theme_name, schema) = {
        let theme_engine = state
            .theme_engine
            .read()
            .map_err(|e| ApiError::internal_error(format!("Failed to acquire theme lock: {}", e)))?;

        let name = theme_engine.get_current_theme().to_string();
        let schema = theme_engine.get_settings_schema(&name)
            .unwrap_or(serde_json::json!({}));
        (name, schema)
    };

    // Collect secret field IDs to exclude them
    let mut secret_fields = std::collections::HashSet::new();
    if let Some(sections) = schema.get("sections").and_then(|s| s.as_array()) {
        for section in sections {
            if let Some(fields) = section.get("fields").and_then(|f| f.as_array()) {
                for field in fields {
                    if field.get("secret").and_then(|s| s.as_bool()).unwrap_or(false) {
                        if let Some(id) = field.get("id").and_then(|i| i.as_str()) {
                            secret_fields.insert(id.to_string());
                        }
                    }
                }
            }
        }
    }

    // Read saved values from settings table (prefixed with theme_{name}_)
    let prefix = format!("theme_{}_", theme_name);
    let all_settings = state
        .settings_service
        .get_all_settings()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let values: std::collections::HashMap<String, String> = all_settings
        .into_iter()
        .filter_map(|(k, v)| {
            k.strip_prefix(&prefix).and_then(|field_id| {
                if secret_fields.contains(field_id) {
                    None // Exclude secret fields
                } else {
                    Some((field_id.to_string(), v))
                }
            })
        })
        .collect();

    Ok(Json(serde_json::json!(values)))
}
