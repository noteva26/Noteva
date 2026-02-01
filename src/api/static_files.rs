//! Static file serving with theme support

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode, Uri},
    response::Response,
};
use rust_embed::RustEmbed;
use std::path::PathBuf;
use tokio::fs;
use urlencoding;

use crate::api::middleware::AppState;
use crate::db::repositories::{SettingsRepository, SqlxSettingsRepository};

/// Embedded admin files (management interface)
#[derive(RustEmbed)]
#[folder = "admin-dist/"]
#[include = "*"]
struct AdminAssets;

/// Embedded default theme files
#[derive(RustEmbed)]
#[folder = "themes/default/dist/"]
#[include = "*"]
struct DefaultThemeAssets;

/// Serve static files based on path
pub async fn serve_static(
    State(state): State<AppState>,
    uri: Uri,
) -> Response {
    let path = uri.path();
    // URL decode the path to handle encoded characters like %5B%5D -> []
    let decoded_path = urlencoding::decode(path).unwrap_or_else(|_| path.into());
    let path = decoded_path.as_ref();
    
    // /uploads/* -> serve uploaded files from disk
    if path.starts_with("/uploads/") {
        return serve_uploads(path).await;
    }
    
    // /themes/* -> serve theme static files (preview images, etc.)
    if path.starts_with("/themes/") {
        return serve_theme_static(path).await;
    }
    
    // /noteva-sdk.js and /noteva-sdk.css -> serve SDK files
    if path == "/noteva-sdk.js" || path == "/noteva-sdk.css" {
        return serve_sdk_file(path).await;
    }
    
    // /manage/* -> admin assets
    if path.starts_with("/manage") {
        return serve_admin(path, &state).await;
    }
    
    // /_next/* -> could be admin or theme, try both
    if path.starts_with("/_next") {
        // Try admin first, then theme
        let asset_path = path.trim_start_matches('/');
        if let Some(content) = AdminAssets::get(asset_path) {
            return build_response(asset_path, &content.data);
        }
        if let Some(content) = DefaultThemeAssets::get(asset_path) {
            return build_response(asset_path, &content.data);
        }
        return not_found();
    }
    
    // Everything else -> theme assets
    serve_theme(path, &state).await
}

/// Serve theme static files (preview images, etc.) from disk
/// Path format: /themes/{theme_name}/{file}
async fn serve_theme_static(path: &str) -> Response {
    // path = /themes/default/preview.png
    let file_path = PathBuf::from(".").join(path.trim_start_matches('/'));
    
    match fs::read(&file_path).await {
        Ok(contents) => {
            let content_type = get_content_type(path);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(Body::from(contents))
                .unwrap()
        }
        Err(_) => not_found(),
    }
}

/// Serve uploaded files from disk
async fn serve_uploads(path: &str) -> Response {
    let file_path = PathBuf::from(".").join(path.trim_start_matches('/'));
    
    match fs::read(&file_path).await {
        Ok(contents) => {
            let content_type = get_content_type(path);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .body(Body::from(contents))
                .unwrap()
        }
        Err(_) => not_found(),
    }
}

/// Serve SDK files (noteva-sdk.js, noteva-sdk.css)
async fn serve_sdk_file(path: &str) -> Response {
    let filename = path.trim_start_matches('/');
    
    // Try to serve from default theme's public folder (embedded)
    if let Some(content) = DefaultThemeAssets::get(filename) {
        return build_response(filename, &content.data);
    }
    
    // Try to serve from themes/default/public folder on disk (for development)
    let disk_path = PathBuf::from("themes/default/public").join(filename);
    if let Ok(contents) = fs::read(&disk_path).await {
        return build_response(filename, &contents);
    }
    
    not_found()
}

/// Serve admin files
async fn serve_admin(path: &str, state: &AppState) -> Response {
    let asset_path = path.trim_start_matches('/');
    let normalized_path = asset_path.trim_end_matches('/');
    
    // Special handling for /manage/setup - redirect to /manage/login if admin exists
    if normalized_path == "manage/setup" {
        // Check if any user exists (first user is always admin)
        use crate::db::repositories::{UserRepository, SqlxUserRepository};
        let user_repo = SqlxUserRepository::new(state.pool.clone());
        if let Ok(count) = user_repo.count().await {
            if count > 0 {
                // Admin exists, redirect to login
                return Response::builder()
                    .status(StatusCode::FOUND)
                    .header(header::LOCATION, "/manage/login")
                    .body(Body::empty())
                    .unwrap();
            }
        }
    }
    
    // Special handling for /manage/login - redirect to /manage/setup if no admin
    if normalized_path == "manage/login" {
        use crate::db::repositories::{UserRepository, SqlxUserRepository};
        let user_repo = SqlxUserRepository::new(state.pool.clone());
        if let Ok(count) = user_repo.count().await {
            if count == 0 {
                // No admin, redirect to setup
                return Response::builder()
                    .status(StatusCode::FOUND)
                    .header(header::LOCATION, "/manage/setup")
                    .body(Body::empty())
                    .unwrap();
            }
        }
    }
    
    // Try exact file (e.g., manage/index.html, manage/articles/index.html)
    if let Some(content) = AdminAssets::get(asset_path) {
        return build_response(asset_path, &content.data);
    }
    
    // Try with /index.html suffix for directories
    let with_index = format!("{}/index.html", asset_path.trim_end_matches('/'));
    if let Some(content) = AdminAssets::get(&with_index) {
        return build_response(&with_index, &content.data);
    }
    
    // SPA fallback for dynamic routes like /manage/articles/123
    // The static export generates /manage/articles/0/index.html as the template
    if asset_path.starts_with("manage/articles/") && !asset_path.contains('.') {
        let parts: Vec<&str> = asset_path.split('/').collect();
        // /manage/articles/123 -> parts = ["manage", "articles", "123"]
        if parts.len() >= 3 && parts[2] != "new" {
            // Serve the [id] template (generated as /0/)
            if let Some(content) = AdminAssets::get("manage/articles/0/index.html") {
                return build_response("manage/articles/0/index.html", &content.data);
            }
        }
    }
    
    // General SPA fallback - serve manage/index.html for all unmatched /manage/* routes
    if let Some(content) = AdminAssets::get("manage/index.html") {
        return build_response("manage/index.html", &content.data);
    }
    
    not_found()
}

/// Serve theme files
async fn serve_theme(path: &str, state: &AppState) -> Response {
    let asset_path = path.trim_start_matches('/');
    let asset_path = if asset_path.is_empty() { "index.html" } else { asset_path };
    
    // Get current theme
    let current_theme = {
        let engine = state.theme_engine.read().unwrap();
        engine.get_current_theme().to_string()
    };
    
    // Try user theme first (non-default)
    if current_theme != "default" {
        if let Some(response) = try_user_theme(&current_theme, asset_path, state).await {
            return response;
        }
    }
    
    // Fall back to embedded default theme
    serve_default_theme(asset_path, Some(state)).await
}

/// Try to serve from user theme directory
async fn try_user_theme(theme: &str, asset_path: &str, state: &AppState) -> Option<Response> {
    let theme_dir = PathBuf::from("themes").join(theme).join("dist");
    
    // Try exact file
    let file_path = theme_dir.join(asset_path);
    if let Ok(contents) = fs::read(&file_path).await {
        // Inject config into HTML files
        if asset_path.ends_with(".html") {
            if let Some(injected) = inject_config_into_html(&contents, state).await {
                return Some(build_response(asset_path, &injected));
            }
        }
        return Some(build_response(asset_path, &contents));
    }
    
    // Try with /index.html for directories
    let with_index = theme_dir.join(format!("{}/index.html", asset_path.trim_end_matches('/')));
    if let Ok(contents) = fs::read(&with_index).await {
        if let Some(injected) = inject_config_into_html(&contents, state).await {
            return Some(build_response(&with_index.to_string_lossy(), &injected));
        }
        return Some(build_response(&with_index.to_string_lossy(), &contents));
    }
    
    // SPA fallback for /posts/* routes
    if asset_path.starts_with("posts/") && !asset_path.contains('.') {
        let posts_index = theme_dir.join("posts/index.html");
        if let Ok(contents) = fs::read(&posts_index).await {
            if let Some(injected) = inject_config_into_html(&contents, state).await {
                return Some(build_response("posts/index.html", &injected));
            }
            return Some(build_response("posts/index.html", &contents));
        }
    }
    
    // SPA fallback for custom pages (single segment paths like /about, /test, /faq)
    if !asset_path.is_empty() 
        && !asset_path.contains('/') 
        && !asset_path.contains('.') 
        && !is_known_static_route(asset_path) 
    {
        let slug_index = theme_dir.join("_/index.html");
        if let Ok(contents) = fs::read(&slug_index).await {
            if let Some(injected) = inject_config_into_html(&contents, state).await {
                return Some(build_response("_/index.html", &injected));
            }
            return Some(build_response("_/index.html", &contents));
        }
    }
    
    // General SPA fallback
    let index_path = theme_dir.join("index.html");
    if let Ok(contents) = fs::read(&index_path).await {
        if let Some(injected) = inject_config_into_html(&contents, state).await {
            return Some(build_response("index.html", &injected));
        }
        return Some(build_response("index.html", &contents));
    }
    
    None
}

/// Serve from embedded default theme
async fn serve_default_theme(asset_path: &str, state: Option<&AppState>) -> Response {
    // Try exact file
    if let Some(content) = DefaultThemeAssets::get(asset_path) {
        // Inject config into HTML files
        if asset_path.ends_with(".html") || asset_path == "index.html" {
            if let Some(state) = state {
                if let Some(injected) = inject_config_into_html(&content.data, state).await {
                    return build_response(asset_path, &injected);
                }
            }
        }
        return build_response(asset_path, &content.data);
    }
    
    // Try with /index.html for directories
    let with_index = format!("{}/index.html", asset_path.trim_end_matches('/'));
    if let Some(content) = DefaultThemeAssets::get(&with_index) {
        // Inject config into HTML files
        if let Some(state) = state {
            if let Some(injected) = inject_config_into_html(&content.data, state).await {
                return build_response(&with_index, &injected);
            }
        }
        return build_response(&with_index, &content.data);
    }
    
    // SPA fallback for /posts/* routes
    if asset_path.starts_with("posts/") && !asset_path.contains('.') {
        if let Some(content) = DefaultThemeAssets::get("posts/index.html") {
            if let Some(state) = state {
                if let Some(injected) = inject_config_into_html(&content.data, state).await {
                    return build_response("posts/index.html", &injected);
                }
            }
            return build_response("posts/index.html", &content.data);
        }
    }
    
    // SPA fallback for custom pages (single segment paths like /about, /test, /faq)
    // These are handled by the [slug] dynamic route
    if !asset_path.is_empty() 
        && !asset_path.contains('/') 
        && !asset_path.contains('.') 
        && !is_known_static_route(asset_path) 
    {
        // Try the [slug] template (generated as /_/index.html)
        if let Some(content) = DefaultThemeAssets::get("_/index.html") {
            if let Some(state) = state {
                if let Some(injected) = inject_config_into_html(&content.data, state).await {
                    return build_response("_/index.html", &injected);
                }
            }
            return build_response("_/index.html", &content.data);
        }
    }
    
    // General SPA fallback
    if let Some(content) = DefaultThemeAssets::get("index.html") {
        if let Some(state) = state {
            if let Some(injected) = inject_config_into_html(&content.data, state).await {
                return build_response("index.html", &injected);
            }
        }
        return build_response("index.html", &content.data);
    }
    
    not_found()
}

/// Check if path is a known static route (not a custom page)
fn is_known_static_route(path: &str) -> bool {
    matches!(path, "archives" | "categories" | "tags" | "login" | "register" | "posts" | "manage")
}

/// Inject site config and SDK into HTML before </head>
async fn inject_config_into_html(html_bytes: &[u8], state: &AppState) -> Option<Vec<u8>> {
    let html = String::from_utf8_lossy(html_bytes);
    
    // Get settings from database
    let settings_repo = SqlxSettingsRepository::new(state.pool.clone());
    let site_name = settings_repo.get("site_name").await.ok().flatten()
        .map(|s| s.value).unwrap_or_else(|| "Noteva".to_string());
    let site_description = settings_repo.get("site_description").await.ok().flatten()
        .map(|s| s.value).unwrap_or_default();
    let site_subtitle = settings_repo.get("site_subtitle").await.ok().flatten()
        .map(|s| s.value).unwrap_or_default();
    let site_logo = settings_repo.get("site_logo").await.ok().flatten()
        .map(|s| s.value).unwrap_or_else(|| "/logo.png".to_string());
    let site_footer = settings_repo.get("site_footer").await.ok().flatten()
        .map(|s| s.value).unwrap_or_default();
    
    // Build config JSON
    let config_json = serde_json::json!({
        "site_name": site_name,
        "site_description": site_description,
        "site_subtitle": site_subtitle,
        "site_logo": site_logo,
        "site_footer": site_footer
    });
    
    // Create injection content:
    // 1. Site config
    // 2. SDK CSS
    // 3. SDK JS
    // 4. Plugin CSS
    // 5. Plugin JS
    // Add version query string to prevent caching issues
    let version = env!("CARGO_PKG_VERSION");
    let injection = format!(
        r#"<script>window.__SITE_CONFIG__={};</script>
<link rel="stylesheet" href="/noteva-sdk.css?v={}">
<script src="/noteva-sdk.js?v={}"></script>
<link rel="stylesheet" href="/api/v1/plugins/assets/plugins.css?v={}">
<script src="/api/v1/plugins/assets/plugins.js?v={}"></script>"#,
        serde_json::to_string(&config_json).unwrap_or_else(|_| "{}".to_string()),
        version,
        version,
        version,
        version
    );
    
    // Inject before </head>
    if let Some(pos) = html.find("</head>") {
        let mut result = String::with_capacity(html.len() + injection.len());
        result.push_str(&html[..pos]);
        result.push_str(&injection);
        result.push_str(&html[pos..]);
        return Some(result.into_bytes());
    }
    
    None
}

/// Build HTTP response with proper headers
fn build_response(path: &str, data: &[u8]) -> Response {
    let content_type = get_content_type(path);
    let cache_control = if is_immutable(path) {
        "public, max-age=31536000, immutable"
    } else if content_type == "text/html" {
        "no-cache"
    } else {
        "public, max-age=3600"
    };
    
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, cache_control)
        .body(Body::from(data.to_vec()))
        .unwrap()
}

/// 404 response
fn not_found() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from("<html><body><h1>404 Not Found</h1></body></html>"))
        .unwrap()
}

/// Get content type from file extension
fn get_content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "webp" => "image/webp",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

/// Check if file is immutable (hashed filename)
fn is_immutable(path: &str) -> bool {
    path.contains("/_next/static/") && (path.ends_with(".js") || path.ends_with(".css"))
}
