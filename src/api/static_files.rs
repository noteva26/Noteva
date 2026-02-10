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
#[folder = "web/dist/"]
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
    
    // /_next/* -> theme assets (Next.js based themes)
    if path.starts_with("/_next") {
        // Try default theme embedded assets
        let asset_path = path.trim_start_matches('/');
        if let Some(content) = DefaultThemeAssets::get(asset_path) {
            return build_response(asset_path, &content.data);
        }
        
        // Try current user theme from disk
        let current_theme = {
            let engine = state.theme_engine.read().unwrap();
            engine.get_current_theme().to_string()
        };
        
        if current_theme != "default" {
            let theme_file = PathBuf::from("themes").join(&current_theme).join("dist").join(asset_path);
            if let Ok(contents) = fs::read(&theme_file).await {
                return build_response(asset_path, &contents);
            }
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
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    
    // For default theme preview, try embedded assets first
    if parts.len() >= 3 && parts[1] == "default" {
        let file_name = parts[2..].join("/");
        // Try to serve from embedded dist folder
        if let Some(content) = DefaultThemeAssets::get(&file_name) {
            let content_type = get_content_type(&file_name);
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(Body::from(content.data.to_vec()))
                .unwrap();
        }
    }
    
    // Fall back to disk for all themes
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
        use crate::db::repositories::{UserRepository, SqlxUserRepository};
        let user_repo = SqlxUserRepository::new(state.pool.clone());
        if let Ok(count) = user_repo.count().await {
            if count > 0 {
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
                return Response::builder()
                    .status(StatusCode::FOUND)
                    .header(header::LOCATION, "/manage/setup")
                    .body(Body::empty())
                    .unwrap();
            }
        }
    }
    
    // For Vite SPA: strip the /manage prefix to match files in dist/
    // e.g., /manage/assets/index-xxx.js -> assets/index-xxx.js
    let relative_path = asset_path.strip_prefix("manage/").unwrap_or(asset_path);
    
    // Try exact file match (static assets like JS, CSS, images)
    if !relative_path.is_empty() {
        if let Some(content) = AdminAssets::get(relative_path) {
            return build_response(relative_path, &content.data);
        }
    }
    
    // SPA fallback: serve index.html for all /manage/* routes
    // React Router handles client-side routing
    if let Some(content) = AdminAssets::get("index.html") {
        return build_response("index.html", &content.data);
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
    
    // Try exact file match (static assets like JS, CSS, images)
    let file_path = theme_dir.join(asset_path);
    if let Ok(contents) = fs::read(&file_path).await {
        if asset_path.ends_with(".html") {
            if let Some(injected) = inject_seo_into_html(&contents, state, asset_path).await {
                return Some(build_response(asset_path, &injected));
            }
        }
        return Some(build_response(asset_path, &contents));
    }
    
    // SPA fallback: serve index.html for all routes
    let index_path = theme_dir.join("index.html");
    if let Ok(contents) = fs::read(&index_path).await {
        if let Some(injected) = inject_seo_into_html(&contents, state, asset_path).await {
            return Some(build_response("index.html", &injected));
        }
        return Some(build_response("index.html", &contents));
    }
    
    None
}

/// Serve from embedded default theme
async fn serve_default_theme(asset_path: &str, state: Option<&AppState>) -> Response {
    // Try exact file match (static assets like JS, CSS, images)
    if let Some(content) = DefaultThemeAssets::get(asset_path) {
        // Inject config + SEO into HTML files
        if asset_path.ends_with(".html") {
            if let Some(state) = state {
                if let Some(injected) = inject_seo_into_html(&content.data, state, asset_path).await {
                    return build_response(asset_path, &injected);
                }
            }
        }
        return build_response(asset_path, &content.data);
    }

    // SPA fallback: serve index.html for all routes
    // React Router handles client-side routing
    if let Some(content) = DefaultThemeAssets::get("index.html") {
        if let Some(state) = state {
            if let Some(injected) = inject_seo_into_html(&content.data, state, asset_path).await {
                return build_response("index.html", &injected);
            }
        }
        return build_response("index.html", &content.data);
    }

    not_found()
}


/// Inject site config, SDK, and SEO content into HTML
/// For article pages (/posts/*), also injects meta tags and article content into <div id="root">
async fn inject_seo_into_html(html_bytes: &[u8], state: &AppState, asset_path: &str) -> Option<Vec<u8>> {
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
    
    let version = env!("CARGO_PKG_VERSION");
    
    // Try to fetch article data for SEO if this is a post page
    let article_seo = if asset_path.starts_with("posts/") && !asset_path.contains('.') {
        let slug = asset_path.strip_prefix("posts/").unwrap_or("").trim_end_matches('/');
        if !slug.is_empty() {
            fetch_article_seo(slug, &state.pool, &site_name).await
        } else {
            None
        }
    } else {
        None
    };
    
    // Build meta tags for SEO
    let (title_tag, meta_tags, body_content) = if let Some(ref seo) = article_seo {
        let title = format!("{} - {}", seo.title, site_name);
        let description = seo.excerpt.replace('"', "&quot;");
        let meta = format!(
            r#"<title>{}</title>
<meta name="description" content="{}">
<meta property="og:title" content="{}">
<meta property="og:description" content="{}">
<meta property="og:type" content="article">
<meta property="article:published_time" content="{}">
<meta property="article:author" content="{}">"#,
            html_escape(&title),
            html_escape(&description),
            html_escape(&seo.title),
            html_escape(&description),
            seo.published_at,
            html_escape(&site_name),
        );
        // Inject article content into <div id="root"> for crawlers
        let body = format!(
            r#"<article style="display:none" data-noteva-seo="true"><h1>{}</h1><div>{}</div></article>"#,
            html_escape(&seo.title),
            seo.content_html,
        );
        (Some(title), meta, body)
    } else {
        let meta = format!(
            r#"<title>{}</title>
<meta name="description" content="{}">"#,
            html_escape(&site_name),
            html_escape(&site_description),
        );
        (None, meta, String::new())
    };
    
    // Build head injection: meta tags + config + SDK + plugins
    let head_injection = format!(
        r#"{}
<script>window.__SITE_CONFIG__={};</script>
<link rel="stylesheet" href="/noteva-sdk.css?v={}">
<script src="/noteva-sdk.js?v={}"></script>
<link rel="stylesheet" href="/api/v1/plugins/assets/plugins.css?v={}">
<script src="/api/v1/plugins/assets/plugins.js?v={}"></script>"#,
        meta_tags,
        serde_json::to_string(&config_json).unwrap_or_else(|_| "{}".to_string()),
        version, version, version, version
    );
    
    let mut result = html.to_string();
    
    // Remove existing <title> tag if we're injecting a new one
    if title_tag.is_some() {
        if let Some(start) = result.find("<title>") {
            if let Some(end) = result[start..].find("</title>") {
                result = format!("{}{}", &result[..start], &result[start + end + 8..]);
            }
        }
    }
    
    // Inject before </head>
    if let Some(pos) = result.find("</head>") {
        result.insert_str(pos, &head_injection);
    }
    
    // Inject SEO content into <div id="root"> for crawlers
    if !body_content.is_empty() {
        if let Some(pos) = result.find(r#"<div id="root">"#) {
            let insert_pos = pos + r#"<div id="root">"#.len();
            result.insert_str(insert_pos, &body_content);
        }
    }
    
    Some(result.into_bytes())
}

/// Article SEO data
struct ArticleSeo {
    title: String,
    excerpt: String,
    content_html: String,
    published_at: String,
}

/// Fetch article data for SEO injection
async fn fetch_article_seo(slug: &str, pool: &crate::db::DynDatabasePool, _site_name: &str) -> Option<ArticleSeo> {
    use crate::db::repositories::{ArticleRepository, SqlxArticleRepository};
    
    let repo = SqlxArticleRepository::new(pool.clone());
    
    // Try by slug first, then by ID
    let article = if let Ok(Some(a)) = repo.get_by_slug(slug).await {
        Some(a)
    } else if let Ok(id) = slug.parse::<i64>() {
        repo.get_by_id(id).await.ok().flatten()
    } else {
        None
    };
    
    let article = article?;
    
    // Only inject SEO for published articles
    if article.status != crate::models::ArticleStatus::Published {
        return None;
    }
    
    // Generate excerpt from content (strip markdown, limit to 200 chars)
    let excerpt = article.content
        .replace('#', "")
        .replace('*', "")
        .replace('`', "")
        .replace('\n', " ")
        .chars()
        .take(200)
        .collect::<String>();
    
    let published_at = article.published_at
        .map(|d| d.to_rfc3339())
        .unwrap_or_default();
    
    Some(ArticleSeo {
        title: article.title,
        excerpt,
        content_html: article.content_html,
        published_at,
    })
}

/// Simple HTML escaping for injected content
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
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
    // Next.js: /_next/static/xxx.js
    // Vite: /assets/index-xxx.js
    (path.contains("/_next/static/") || path.contains("/assets/")) 
        && (path.ends_with(".js") || path.ends_with(".css"))
}
