//! Site settings management endpoints

use axum::{extract::State, Json};
use serde::Serialize;

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};

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

/// GET /api/v1/admin/settings - Get site settings
///
/// Requires admin authentication.
/// Satisfies requirement 5.3: System configuration
pub async fn get_settings(
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
pub async fn update_settings(
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
