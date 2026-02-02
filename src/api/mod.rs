//! API layer - HTTP handlers and routing
//!
//! This module contains all HTTP API endpoints for the Noteva blog system.
//! It includes:
//! - Article API endpoints
//! - Category API endpoints
//! - Tag API endpoints
//! - User/Auth API endpoints
//! - Admin API endpoints
//! - Upload API endpoints
//! - Site info API endpoints
//! - Comment API endpoints
//! - Page API endpoints
//! - Navigation API endpoints
//! - Plugin API endpoints
//! - Static file serving with config injection

pub mod admin;
pub mod articles;
pub mod auth;
pub mod categories;
pub mod comments;
pub mod common;
pub mod middleware;
pub mod nav;
pub mod pages;
pub mod plugins;
pub mod plugin_install;
pub mod responses;
pub mod site;
pub mod static_files;
pub mod tags;
pub mod theme_install;
pub mod upload;

use axum::{
    http::{header, HeaderValue, Method},
    middleware as axum_middleware,
    Router,
};
use tower_http::cors::CorsLayer;

pub use middleware::{
    AppState, ApiError, CacheConfig, CachedResponse,
    generate_etag, generate_weak_etag, etag_matches,
    cache_control_static, cache_control_api, cache_control_private, cache_control_no_cache,
    add_static_cache_headers, add_api_cache_headers, check_if_none_match,
    RequestStats,
};

/// Build the main API router
pub fn build_api_router(state: AppState) -> Router<AppState> {
    // Admin routes (need admin role)
    let admin_routes = Router::new()
        .nest("/admin", admin::router())
        .nest("/admin/pages", pages::router())
        .nest("/admin/nav", nav::router())
        .nest("/admin/plugins", plugins::router())
        // Theme installation routes
        .route("/admin/themes/upload", axum::routing::post(theme_install::upload_theme))
        .route("/admin/themes/github/releases", axum::routing::get(theme_install::list_github_releases))
        .route("/admin/themes/github/install", axum::routing::post(theme_install::install_github_theme))
        .route("/admin/themes/:name", axum::routing::delete(theme_install::delete_theme))
        // Plugin installation routes
        .route("/admin/plugins/upload", axum::routing::post(plugin_install::upload_plugin))
        .route("/admin/plugins/github/releases", axum::routing::get(plugin_install::list_github_releases))
        .route("/admin/plugins/github/install", axum::routing::post(plugin_install::install_github_plugin))
        .route("/admin/plugins/:id/uninstall", axum::routing::delete(plugin_install::uninstall_plugin))
        // Admin article operations by ID
        .route("/admin/articles/:id", axum::routing::get(articles::get_article_by_id_handler))
        .route("/admin/articles/:id", axum::routing::put(articles::update_article_handler))
        .route("/admin/articles/:id", axum::routing::delete(articles::delete_article_handler))
        // Admin comment operations
        .route("/admin/comments/:id", axum::routing::delete(comments::delete_comment))
        .route_layer(axum_middleware::from_fn(middleware::require_admin))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ));

    // Protected routes (need auth but not admin)
    let protected_routes = Router::new()
        .nest("/auth", auth::protected_router())
        .nest("/upload", upload::router())
        .route("/articles", axum::routing::post(articles::create_article_handler))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ));

    // Public routes
    Router::new()
        .route("/articles", axum::routing::get(articles::list_articles_handler))
        .route("/articles/resolve", axum::routing::get(articles::resolve_article_handler))
        .route("/articles/:slug", axum::routing::get(articles::get_article_handler))
        .nest("/categories", categories::router())
        .nest("/tags", tags::router())
        .nest("/auth", auth::public_router())
        .nest("/site", site::router())
        .nest("/pages", pages::public_router())
        .nest("/page", pages::slug_router())
        .nest("/nav", nav::public_router())
        // Plugin assets (public)
        .route("/plugins/assets/plugins.js", axum::routing::get(plugins::get_plugins_js_public))
        .route("/plugins/assets/plugins.css", axum::routing::get(plugins::get_plugins_css_public))
        .route("/plugins/enabled", axum::routing::get(plugins::get_enabled_plugins_public))
        // Comment routes
        .route("/comments/:article_id", axum::routing::get(comments::get_comments))
        .route("/comments", axum::routing::post(comments::create_comment))
        .route("/like", axum::routing::post(comments::like))
        .route("/like/check", axum::routing::get(comments::check_like))
        .route("/view/:article_id", axum::routing::post(comments::increment_view))
        .merge(admin_routes)
        .merge(protected_routes)
}

/// Build the complete router with middleware
pub fn build_router(state: AppState, cors_origin: &str) -> Router {
    // CORS configuration - 支持 cookie 认证
    let cors = CorsLayer::new()
        .allow_origin(cors_origin.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE])
        .allow_credentials(true);

    Router::new()
        .nest("/api/v1", build_api_router(state.clone()))
        // Static file serving (for production)
        .fallback(static_files::serve_static)
        .layer(cors)
        // Demo mode guard (blocks write operations when compiled with --features demo)
        .layer(axum_middleware::from_fn(middleware::demo_guard))
        // Request stats middleware (outermost layer, runs for all requests)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::request_stats_middleware,
        ))
        .with_state(state)
}
