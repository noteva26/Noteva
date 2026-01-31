//! Public site information API
//!
//! Provides public access to site settings (no authentication required).
//! Used by frontend to display site name, logo, etc.

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::api::middleware::AppState;

/// Response for public site info
#[derive(Debug, Serialize)]
pub struct SiteInfoResponse {
    pub site_name: String,
    pub site_description: String,
    pub site_subtitle: String,
    pub site_logo: String,
    pub site_footer: String,
    pub email_verification_enabled: String,
}

/// Request for rendering markdown content
#[derive(Debug, Deserialize)]
pub struct RenderRequest {
    pub content: String,
}

/// Response for rendered content
#[derive(Debug, Serialize)]
pub struct RenderResponse {
    pub html: String,
}

/// Build the public site router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/info", get(get_site_info))
        .route("/render", post(render_content))
}

/// GET /api/v1/site/info - Get public site information
///
/// No authentication required.
async fn get_site_info(
    State(state): State<AppState>,
) -> Json<SiteInfoResponse> {
    let settings = state
        .settings_service
        .get_site_settings()
        .await
        .unwrap_or_else(|_| crate::services::settings::SiteSettings {
            site_name: "Noteva".to_string(),
            site_description: "A lightweight blog powered by Noteva".to_string(),
            site_subtitle: String::new(),
            site_logo: String::new(),
            site_footer: String::new(),
            posts_per_page: 10,
        });

    // Get email verification setting
    let email_verification_enabled = state
        .settings_service
        .get("email_verification_enabled")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "false".to_string());

    Json(SiteInfoResponse {
        site_name: settings.site_name,
        site_description: settings.site_description,
        site_subtitle: settings.site_subtitle,
        site_logo: settings.site_logo,
        site_footer: settings.site_footer,
        email_verification_enabled,
    })
}


/// POST /api/v1/site/render - Render markdown content with shortcode processing
///
/// Used by admin preview to show how content will look with shortcodes processed.
async fn render_content(
    State(state): State<AppState>,
    Json(req): Json<RenderRequest>,
) -> Json<RenderResponse> {
    // Use article service to render with shortcode processing
    let html = state.article_service.render_markdown_with_shortcodes(&req.content, None, None);
    Json(RenderResponse { html })
}
