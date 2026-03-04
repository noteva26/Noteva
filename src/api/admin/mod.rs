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

mod dashboard;
mod taxonomy;
mod themes;
mod settings;
mod update;
mod comments;
mod reload;
mod security;
mod backup;

pub use comments::{
    list_comments, list_pending_comments, approve_comment, reject_comment,
    CommentsQuery, AdminCommentsResponse, AdminCommentResponse,
};
pub use security::{LoginLogEntry, LoginLogsQuery, LoginLogsResponse};
pub use update::APP_VERSION;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use crate::api::middleware::AppState;

/// Build the admin router
pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/dashboard", get(dashboard::get_dashboard))
        // System stats
        .route("/stats", get(dashboard::get_system_stats))
        // Update check
        .route("/update-check", get(update::check_update))
        .route("/update-perform", post(update::perform_update))
        // Category management
        .route("/categories", post(taxonomy::create_category))
        .route("/categories/:id", put(taxonomy::update_category))
        .route("/categories/:id", delete(taxonomy::delete_category))
        // Tag management
        .route("/tags", post(taxonomy::create_tag))
        .route("/tags/:id", delete(taxonomy::delete_tag))
        // Theme management
        .route("/themes", get(themes::list_themes))
        .route("/themes/switch", post(themes::switch_theme))
        .route("/themes/store", get(themes::get_theme_store))
        .route("/themes/updates", get(themes::check_theme_updates))
        .route("/themes/reload", post(reload::reload_themes))
        // Plugin management
        .route("/plugins/reload", post(reload::reload_plugins))
        // Site settings
        .route("/settings", get(settings::get_settings))
        .route("/settings", put(settings::update_settings))
        // Comment management
        .route("/comments", get(list_comments))
        .route("/comments/pending", get(list_pending_comments))
        .route("/comments/:id/approve", post(approve_comment))
        .route("/comments/:id/reject", post(reject_comment))
        // Login logs (security)
        .route("/login-logs", get(security::list_login_logs))
        // Backup & Restore
        .route("/backup", get(backup::download_backup))
        .route("/backup/restore", post(backup::restore_backup_endpoint))
        .route("/backup/export-markdown", get(backup::export_markdown_endpoint))
        .route("/backup/import", post(backup::import_articles_endpoint))
}
