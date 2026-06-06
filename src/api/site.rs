//! Public site information API
//!
//! Provides public access to site settings (no authentication required).
//! Used by frontend to display site name, logo, etc.

use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::middleware::{AppState, AuthenticatedUser};

/// Response for public site info
#[derive(Debug, Serialize)]
pub struct SiteInfoResponse {
    pub version: String,
    pub site_name: String,
    pub site_description: String,
    pub site_subtitle: String,
    pub site_logo: String,
    pub site_footer: String,
    pub site_url: String,
    pub email_verification_enabled: String,
    pub permalink_structure: String,
    pub demo_mode: bool,
    pub custom_css: String,
    pub custom_js: String,
    /// Whether the article table-of-contents sidebar is shown (default: true)
    pub show_toc: bool,
    /// Whether the previous/next article navigation is shown (default: true)
    pub show_post_nav: bool,
    /// Whether the related-articles section is shown (default: true)
    pub show_related_posts: bool,
    /// Whether the comments section is shown (default: true)
    pub show_comments: bool,
    /// Whether the built-in friend-links page appears in theme navigation.
    pub friend_links_nav_enabled: bool,
    /// Whether the built-in about page appears in theme navigation.
    pub about_nav_enabled: bool,
    pub stats: SiteStats,
}

/// Public site statistics
#[derive(Debug, Serialize)]
pub struct SiteStats {
    pub total_articles: i64,
    pub total_categories: i64,
    pub total_tags: i64,
    pub total_comments: i64,
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
    Router::new().route("/info", get(get_site_info))
}

pub use render_content as render_content_handler;

/// GET /api/v1/site/info - Get public site information
///
/// No authentication required.
async fn get_site_info(State(state): State<AppState>) -> Json<SiteInfoResponse> {
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

    // Get permalink structure setting
    let permalink_structure = state
        .settings_service
        .get("permalink_structure")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "/posts/{slug}".to_string());

    // Get custom CSS/JS
    let custom_css = state
        .settings_service
        .get("custom_css")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let custom_js = state
        .settings_service
        .get("custom_js")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    // Get site_url
    let site_url = state
        .settings_service
        .get("site_url")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    // Gather public stats
    let total_articles = state.article_service.count_published().await.unwrap_or(0);
    let total_categories = state
        .category_service
        .list()
        .await
        .map(|c| c.len() as i64)
        .unwrap_or(0);
    let total_tags = state
        .tag_service
        .list()
        .await
        .map(|t| t.len() as i64)
        .unwrap_or(0);
    let total_comments = state.comment_service.count_approved().await.unwrap_or(0);

    // Display toggles. Default to ON (true) when unset, so existing sites
    // keep their current behavior after upgrading.
    let read_toggle = |value: Option<String>| -> bool {
        match value {
            Some(v) => !matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "false" | "0" | "no" | "off"
            ),
            None => true,
        }
    };
    let show_toc = read_toggle(state.settings_service.get("show_toc").await.ok().flatten());
    let show_post_nav = read_toggle(
        state
            .settings_service
            .get("show_post_nav")
            .await
            .ok()
            .flatten(),
    );
    let show_related_posts = read_toggle(
        state
            .settings_service
            .get("show_related_posts")
            .await
            .ok()
            .flatten(),
    );
    let show_comments = read_toggle(
        state
            .settings_service
            .get("show_comments")
            .await
            .ok()
            .flatten(),
    );
    let friend_links_nav_enabled = read_toggle(
        state
            .settings_service
            .get("friend_links_nav_enabled")
            .await
            .ok()
            .flatten(),
    );
    let about_nav_enabled = state.about_service.is_nav_enabled().await;

    Json(SiteInfoResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        site_name: settings.site_name,
        site_description: settings.site_description,
        site_subtitle: settings.site_subtitle,
        site_logo: settings.site_logo,
        site_footer: settings.site_footer,
        site_url,
        email_verification_enabled,
        permalink_structure,
        demo_mode: crate::api::middleware::is_demo_mode(),
        custom_css,
        custom_js,
        show_toc,
        show_post_nav,
        show_related_posts,
        show_comments,
        friend_links_nav_enabled,
        about_nav_enabled,
        stats: SiteStats {
            total_articles,
            total_categories,
            total_tags,
            total_comments,
        },
    })
}

/// POST /api/v1/site/render - Render markdown content with shortcode processing
///
/// Used by admin preview to show how content will look with shortcodes processed.
pub async fn render_content(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(req): Json<RenderRequest>,
) -> Json<RenderResponse> {
    // Use article service to render with shortcode processing
    let html = state
        .article_service
        .render_markdown_with_shortcodes(&req.content, None, None);
    Json(RenderResponse { html })
}
