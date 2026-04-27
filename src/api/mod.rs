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
mod archive;
pub mod articles;
pub mod auth;
pub mod cache;
pub mod categories;
pub mod comments;
pub mod common;
mod github_update;
pub mod locales;
pub mod middleware;
pub mod nav;
pub mod pages;
pub mod plugin_install;
pub mod plugins;
pub mod proxy;
pub mod responses;
pub mod seo;
pub mod site;
pub mod static_files;
pub mod tags;
pub mod theme;
pub mod theme_install;
pub mod two_factor;
pub mod upload;

use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderName, HeaderValue, Method},
    middleware as axum_middleware, Router,
};
use tower_http::cors::CorsLayer;

pub use middleware::{
    add_api_cache_headers, add_static_cache_headers, cache_control_api, cache_control_no_cache,
    cache_control_private, cache_control_static, check_if_none_match, etag_matches, generate_etag,
    generate_weak_etag, ApiError, AppState, CacheConfig, CachedResponse, RequestStats,
};

/// Build the main API router
pub fn build_api_router(state: AppState) -> Router<AppState> {
    // Read body limits from config (add 1MB headroom for multipart overhead)
    let image_body_limit = (state.upload_config.max_file_size as usize).saturating_add(1024 * 1024);
    let admin_body_limit =
        (state.upload_config.max_plugin_file_size as usize).saturating_add(1024 * 1024);

    // Admin routes (need admin role)
    let admin_routes = Router::new()
        .nest("/admin", admin::router())
        .nest("/admin/pages", pages::router())
        .nest("/admin/nav", nav::router())
        .nest("/admin/plugins", plugins::router())
        // Theme installation routes
        .route(
            "/admin/themes/upload",
            axum::routing::post(theme_install::upload_theme),
        )
        .route(
            "/admin/themes/github/releases",
            axum::routing::get(theme_install::list_github_releases),
        )
        .route(
            "/admin/themes/github/install",
            axum::routing::post(theme_install::install_github_theme),
        )
        .route(
            "/admin/themes/install-from-repo",
            axum::routing::post(theme_install::install_from_repo),
        )
        .route(
            "/admin/themes/{name}/update",
            axum::routing::post(theme_install::update_theme),
        )
        .route(
            "/admin/themes/{name}/settings",
            axum::routing::get(theme::get_theme_settings_admin),
        )
        .route(
            "/admin/themes/{name}/settings",
            axum::routing::put(theme::update_theme_settings_admin),
        )
        .route(
            "/admin/themes/{name}",
            axum::routing::delete(theme_install::delete_theme),
        )
        // Plugin installation routes
        .route(
            "/admin/plugins/upload",
            axum::routing::post(plugin_install::upload_plugin),
        )
        .route(
            "/admin/plugins/github/releases",
            axum::routing::get(plugin_install::list_github_releases),
        )
        .route(
            "/admin/plugins/github/install",
            axum::routing::post(plugin_install::install_github_plugin),
        )
        .route(
            "/admin/plugins/install-from-repo",
            axum::routing::post(plugin_install::install_from_repo),
        )
        .route(
            "/admin/plugins/{id}/update",
            axum::routing::post(plugin_install::update_plugin),
        )
        .route(
            "/admin/plugins/{id}/uninstall",
            axum::routing::delete(plugin_install::uninstall_plugin),
        )
        // Admin article operations by ID
        .route(
            "/admin/articles",
            axum::routing::get(articles::list_articles_admin_handler)
                .post(articles::create_article_handler),
        )
        .route(
            "/admin/articles/{id}",
            axum::routing::get(articles::get_article_by_id_handler),
        )
        .route(
            "/admin/articles/{id}",
            axum::routing::put(articles::update_article_handler),
        )
        .route(
            "/admin/articles/{id}",
            axum::routing::delete(articles::delete_article_handler),
        )
        // Admin comment operations
        .route(
            "/admin/comments/{id}",
            axum::routing::delete(comments::delete_comment),
        )
        // Admin locale management
        .route(
            "/admin/locales",
            axum::routing::post(locales::upsert_locale),
        )
        .route(
            "/admin/locales/{code}",
            axum::routing::delete(locales::delete_locale),
        )
        .layer(DefaultBodyLimit::max(admin_body_limit))
        .route_layer(axum_middleware::from_fn(middleware::require_admin))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ));

    // Protected routes (need auth but not admin)
    let protected_routes = Router::new()
        .nest("/auth", auth::protected_router())
        .nest("/auth/2fa", two_factor::router())
        .nest(
            "/upload",
            upload::router().layer(DefaultBodyLimit::max(image_body_limit)),
        )
        .route(
            "/articles",
            axum::routing::post(articles::create_article_handler),
        )
        .nest(
            "/cache",
            Router::new()
                .route("/{key}", axum::routing::put(cache::set_cache))
                .route("/{key}", axum::routing::delete(cache::delete_cache)),
        )
        .route(
            "/site/render",
            axum::routing::post(site::render_content_handler),
        )
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ));

    // Public routes
    Router::new()
        .route(
            "/articles",
            axum::routing::get(articles::list_articles_handler),
        )
        .route(
            "/articles/resolve",
            axum::routing::get(articles::resolve_article_handler),
        )
        .route(
            "/articles/{slug}",
            axum::routing::get(articles::get_article_handler),
        )
        .nest("/categories", categories::router())
        .nest("/tags", tags::router())
        .nest("/auth", auth::public_router())
        .nest("/auth/2fa", two_factor::public_router())
        .nest("/site", site::router())
        .nest(
            "/theme",
            Router::new()
                .route("/config", axum::routing::get(theme::get_theme_config))
                .route("/info", axum::routing::get(theme::get_theme_info))
                .route(
                    "/settings",
                    axum::routing::get(theme::get_theme_settings_public),
                ),
        )
        .nest(
            "/cache",
            Router::new().route("/{key}", axum::routing::get(cache::get_cache)),
        )
        .nest("/pages", pages::public_router())
        .nest("/page", pages::slug_router())
        .nest("/nav", nav::public_router())
        // Plugin assets (public)
        .route(
            "/plugins/assets/plugins.js",
            axum::routing::get(plugins::get_plugins_js_public),
        )
        .route(
            "/plugins/assets/plugins.css",
            axum::routing::get(plugins::get_plugins_css_public),
        )
        .route(
            "/plugins/enabled",
            axum::routing::get(plugins::get_enabled_plugins_public),
        )
        .route("/plugins/proxy", axum::routing::post(proxy::proxy_request))
        // Plugin data (public, read-only for frontend)
        .route(
            "/plugins/{id}/data/{key}",
            axum::routing::get(plugins::get_plugin_data),
        )
        // Plugin custom API routes (public, proxied to WASM)
        .route(
            "/plugins/{id}/api/{*path}",
            axum::routing::any(plugins::plugin_api_handler),
        )
        // Comment routes (order matters: /recent before /:article_id)
        .route(
            "/comments/recent",
            axum::routing::get(comments::get_recent_comments),
        )
        .route(
            "/comments/{article_id}",
            axum::routing::get(comments::get_comments),
        )
        .route("/comments", axum::routing::post(comments::create_comment))
        .route("/like", axum::routing::post(comments::like))
        .route("/like/check", axum::routing::get(comments::check_like))
        .route(
            "/view/{article_id}",
            axum::routing::post(comments::increment_view),
        )
        // Public locale endpoints
        .route("/locales", axum::routing::get(locales::list_locales))
        .route("/locales/{code}", axum::routing::get(locales::get_locale))
        .merge(admin_routes)
        .merge(protected_routes)
}

/// Build the complete router with middleware
pub fn build_router(state: AppState, cors_origin: &str) -> Router {
    // CORS configuration - 支持 cookie 认证
    let cors_origin_header = if cors_origin.trim() == "*" {
        tracing::warn!("cors_origin='*' is invalid with credentials; using http://localhost:3000");
        HeaderValue::from_static("http://localhost:3000")
    } else {
        cors_origin.parse::<HeaderValue>().unwrap_or_else(|_| {
            tracing::warn!(
                cors_origin,
                "invalid cors_origin in config; using http://localhost:3000"
            );
            HeaderValue::from_static("http://localhost:3000")
        })
    };

    let cors = CorsLayer::new()
        .allow_origin(cors_origin_header)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::COOKIE,
            HeaderName::from_static("x-csrf-token"),
        ])
        .allow_credentials(true);

    Router::new()
        .nest("/api/v1", build_api_router(state.clone()))
        // SEO endpoints (top-level, before static file fallback)
        .route("/sitemap.xml", axum::routing::get(seo::sitemap_xml))
        .route("/robots.txt", axum::routing::get(seo::robots_txt))
        .route("/feed.xml", axum::routing::get(seo::feed_xml))
        .route("/rss.xml", axum::routing::get(seo::feed_xml))
        .route("/feed", axum::routing::get(seo::feed_xml))
        // Static file serving (for production)
        .fallback(static_files::serve_static)
        .layer(cors)
        // CSRF protection (after CORS, before demo guard)
        .layer(axum_middleware::from_fn(middleware::csrf_protection))
        // Demo mode guard (blocks write operations when compiled with --features demo)
        .layer(axum_middleware::from_fn(middleware::demo_guard))
        // Request stats middleware (outermost layer, runs for all requests)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::request_stats_middleware,
        ))
        .with_state(state)
}
