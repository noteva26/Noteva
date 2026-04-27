//! Static file serving with theme support

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderName, HeaderValue, StatusCode, Uri},
    response::Response,
};
use rust_embed::RustEmbed;
use std::path::{Component, Path, PathBuf};
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
pub async fn serve_static(State(state): State<AppState>, uri: Uri) -> Response {
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
        return serve_theme_static(path, &state).await;
    }

    // /noteva-sdk.js and /noteva-sdk.css -> serve SDK files
    if path == "/noteva-sdk.js" || path == "/noteva-sdk.css" {
        return serve_sdk_file(path).await;
    }

    // Root-level static files (e.g. /logo.png, /favicon.ico) -> try admin assets first
    // These are public resources used by both admin panel and themes
    {
        let asset_path = path.trim_start_matches('/');
        if asset_path.contains('.') && !asset_path.contains('/') {
            if let Some(content) = AdminAssets::get(asset_path) {
                return build_response(asset_path, &content.data);
            }
        }
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
            if let Ok(engine) = state.theme_engine.read() {
                engine.get_current_theme().to_string()
            } else {
                "default".to_string()
            }
        };

        if current_theme != "default" {
            let Some(rel_path) = safe_relative_path(asset_path) else {
                return not_found();
            };
            let theme_dir = PathBuf::from("themes").join(&current_theme).join("dist");
            if let Some(contents) = read_file_under(&theme_dir, &rel_path).await {
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
async fn serve_theme_static(path: &str, state: &AppState) -> Response {
    // path = /themes/default/preview.png
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if parts.len() < 3 {
        return not_found();
    }

    let theme_name = parts[1]; // e.g. "pixel" or "default"
    let file_name = parts[2..].join("/");
    let Some(safe_file_name) = safe_relative_path(&file_name) else {
        return not_found();
    };
    let embedded_file_name = safe_file_name.to_string_lossy().replace('\\', "/");

    // For default theme, try embedded assets first
    if theme_name == "default" {
        if let Some(content) = DefaultThemeAssets::get(&embedded_file_name) {
            let content_type = get_content_type(&embedded_file_name);
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(Body::from(content.data.to_vec()))
                .unwrap();
        }
    }

    if !crate::theme::validation::is_valid_theme_slug(theme_name) {
        return not_found();
    }

    // Resolve actual directory name via ThemeEngine (handles short name -> dir name mapping)
    let actual_dir = if let Ok(engine) = state.theme_engine.read() {
        let theme_path = engine.get_theme_path(theme_name);
        theme_path
    } else {
        PathBuf::from("themes").join(theme_name)
    };

    // Try serving from theme root directory (e.g. themes/Pixel Art/preview.png)
    if let Some(contents) = read_file_under(&actual_dir, &safe_file_name).await {
        let content_type = get_content_type(&embedded_file_name);
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(Body::from(contents))
            .unwrap();
    }

    // Also try from dist/ subdirectory
    let dist_dir = actual_dir.join("dist");
    if let Some(contents) = read_file_under(&dist_dir, &safe_file_name).await {
        let content_type = get_content_type(&embedded_file_name);
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(Body::from(contents))
            .unwrap();
    }

    not_found()
}

/// Serve uploaded files from disk
async fn serve_uploads(path: &str) -> Response {
    let file_path = PathBuf::from(".").join(path.trim_start_matches('/'));

    // Path traversal guard: resolve and verify path stays within uploads/
    let uploads_dir = match PathBuf::from("uploads").canonicalize() {
        Ok(p) => p,
        Err(_) => return not_found(),
    };
    let canonical = match file_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return not_found(),
    };
    if !canonical.starts_with(&uploads_dir) {
        return not_found();
    }

    match fs::read(&canonical).await {
        Ok(contents) => {
            let mut response = Response::builder()
                .status(StatusCode::OK)
                .header(
                    header::CONTENT_TYPE,
                    if is_active_upload(path) {
                        "application/octet-stream"
                    } else {
                        get_content_type(path)
                    },
                )
                .header(
                    HeaderName::from_static("x-content-type-options"),
                    HeaderValue::from_static("nosniff"),
                )
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .body(Body::from(contents))
                .unwrap();
            if is_active_upload(path) {
                response.headers_mut().insert(
                    header::CONTENT_DISPOSITION,
                    HeaderValue::from_static("attachment"),
                );
            }
            response
        }
        Err(_) => not_found(),
    }
}

/// Serve SDK files (noteva-sdk.js, noteva-sdk.css)
async fn serve_sdk_file(path: &str) -> Response {
    let filename = path.trim_start_matches('/');

    if filename == "noteva-sdk.js" {
        return build_response(filename, include_bytes!("noteva-sdk.js"));
    }

    // CSS is still theme-facing static styling, so it stays with the default theme assets.
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
        use crate::db::repositories::{SqlxUserRepository, UserRepository};
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
        use crate::db::repositories::{SqlxUserRepository, UserRepository};
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
    let asset_path = if asset_path.is_empty() {
        "index.html"
    } else {
        asset_path
    };

    // Get current theme
    let current_theme = {
        if let Ok(engine) = state.theme_engine.read() {
            engine.get_current_theme().to_string()
        } else {
            "default".to_string()
        }
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
    // Resolve actual directory name via ThemeEngine
    let theme_base = if let Ok(engine) = state.theme_engine.read() {
        engine.get_theme_path(theme)
    } else {
        PathBuf::from("themes").join(theme)
    };
    let theme_dir = theme_base.join("dist");
    let rel_path = safe_relative_path(asset_path)?;

    // Try exact file match (static assets like JS, CSS, images)
    if let Some(contents) = read_file_under(&theme_dir, &rel_path).await {
        if asset_path.ends_with(".html") {
            if let Some(injected) = inject_seo_into_html(&contents, state, asset_path).await {
                return Some(build_response(asset_path, &injected));
            }
        }
        return Some(build_response(asset_path, &contents));
    }

    // If the path looks like a static file (has extension), don't SPA fallback
    // This prevents returning index.html for missing images/fonts/etc.
    if asset_path.contains('.') && !asset_path.ends_with(".html") {
        return None;
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

fn safe_relative_path(input: &str) -> Option<PathBuf> {
    if input.is_empty() || input.contains('\\') {
        return None;
    }

    let mut clean = PathBuf::new();
    for component in Path::new(input).components() {
        match component {
            Component::Normal(part) => {
                let text = part.to_string_lossy();
                if text.is_empty() || text.contains(':') {
                    return None;
                }
                clean.push(part);
            }
            Component::CurDir => {}
            _ => return None,
        }
    }

    if clean.as_os_str().is_empty() {
        None
    } else {
        Some(clean)
    }
}

async fn read_file_under(root: &Path, relative_path: &Path) -> Option<Vec<u8>> {
    let root = root.canonicalize().ok()?;
    let file_path = root.join(relative_path);
    let canonical = file_path.canonicalize().ok()?;
    if !canonical.starts_with(&root) || !canonical.is_file() {
        return None;
    }
    fs::read(canonical).await.ok()
}

fn is_active_upload(path: &str) -> bool {
    matches!(
        path.rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "html" | "htm" | "js" | "mjs" | "svg" | "xml"
    )
}

/// Serve from embedded default theme
async fn serve_default_theme(asset_path: &str, state: Option<&AppState>) -> Response {
    // Try exact file match (static assets like JS, CSS, images)
    if let Some(content) = DefaultThemeAssets::get(asset_path) {
        // Inject config + SEO into HTML files
        if asset_path.ends_with(".html") {
            if let Some(state) = state {
                if let Some(injected) = inject_seo_into_html(&content.data, state, asset_path).await
                {
                    return build_response(asset_path, &injected);
                }
            }
        }
        return build_response(asset_path, &content.data);
    }

    // If the path looks like a static file (has extension), don't SPA fallback
    // This prevents returning index.html for missing images/fonts/etc.
    if asset_path.contains('.') && !asset_path.ends_with(".html") {
        return not_found();
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
async fn inject_seo_into_html(
    html_bytes: &[u8],
    state: &AppState,
    asset_path: &str,
) -> Option<Vec<u8>> {
    let html = String::from_utf8_lossy(html_bytes);

    // Get settings from database in a single batch query (was 8 individual queries)
    let settings_repo = SqlxSettingsRepository::new(state.pool.clone());
    let settings = settings_repo
        .get_many(&[
            "site_name",
            "site_description",
            "site_subtitle",
            "site_logo",
            "site_footer",
            "custom_css",
            "custom_js",
            "site_url",
            "permalink_structure",
        ])
        .await
        .unwrap_or_default();

    let site_name = settings
        .get("site_name")
        .cloned()
        .unwrap_or_else(|| "Noteva".to_string());
    let site_description = settings
        .get("site_description")
        .cloned()
        .unwrap_or_default();
    let site_subtitle = settings.get("site_subtitle").cloned().unwrap_or_default();
    let site_logo = settings
        .get("site_logo")
        .cloned()
        .unwrap_or_else(|| "/logo.png".to_string());
    let site_footer = settings.get("site_footer").cloned().unwrap_or_default();
    let custom_css = settings.get("custom_css").cloned().unwrap_or_default();
    let custom_js = settings.get("custom_js").cloned().unwrap_or_default();
    let site_url = settings.get("site_url").cloned().unwrap_or_default();
    let permalink_structure = settings
        .get("permalink_structure")
        .cloned()
        .unwrap_or_else(|| "/posts/{slug}".to_string());
    let is_id_mode = permalink_structure.contains("{id}");

    // Build config JSON
    let config_json = serde_json::json!({
        "site_name": site_name,
        "site_description": site_description,
        "site_subtitle": site_subtitle,
        "site_logo": site_logo,
        "site_footer": site_footer
    });

    let version = env!("CARGO_PKG_VERSION");

    let base_url = site_url.trim_end_matches('/');

    // Try to fetch article data for SEO if this is a post page
    let article_seo = if asset_path.starts_with("posts/") && !asset_path.contains('.') {
        let slug = asset_path
            .strip_prefix("posts/")
            .unwrap_or("")
            .trim_end_matches('/');
        if !slug.is_empty() {
            fetch_article_seo(slug, &state.pool, &site_name).await
        } else {
            None
        }
    } else {
        None
    };

    // Try to fetch page data for SEO (e.g. /about, /links)
    let page_seo = if article_seo.is_none()
        && !asset_path.is_empty()
        && asset_path != "index.html"
        && !asset_path.starts_with("posts/")
        && !asset_path.starts_with("categories/")
        && !asset_path.starts_with("tags/")
        && !asset_path.starts_with("manage/")
        && !asset_path.contains('.')
    {
        let slug = asset_path.trim_end_matches('/');
        if !slug.is_empty() {
            fetch_page_seo(slug, &state.pool).await
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
        let canonical_url = if base_url.is_empty() {
            String::new()
        } else {
            let identifier = if is_id_mode {
                seo.id.to_string()
            } else {
                seo.slug.clone()
            };
            format!("{}/posts/{}", base_url, identifier)
        };
        let og_image = seo.thumbnail.as_deref().unwrap_or("").to_string();
        let og_image_full = if og_image.is_empty() {
            String::new()
        } else if og_image.starts_with("http") {
            og_image.clone()
        } else if !base_url.is_empty() {
            format!("{}{}", base_url, og_image)
        } else {
            og_image.clone()
        };

        let mut meta = format!(
            r#"<title>{}</title>
<meta name="description" content="{}">
<link rel="canonical" href="{}">"#,
            html_escape(&title),
            html_escape(&description),
            html_escape(&canonical_url),
        );
        // Open Graph
        meta.push_str(&format!(
            r#"
<meta property="og:title" content="{}">
<meta property="og:description" content="{}">
<meta property="og:type" content="article">
<meta property="og:site_name" content="{}">"#,
            html_escape(&seo.title),
            html_escape(&description),
            html_escape(&site_name),
        ));
        if !canonical_url.is_empty() {
            meta.push_str(&format!(
                "\n<meta property=\"og:url\" content=\"{}\">",
                html_escape(&canonical_url)
            ));
        }
        if !og_image_full.is_empty() {
            meta.push_str(&format!(
                "\n<meta property=\"og:image\" content=\"{}\">",
                html_escape(&og_image_full)
            ));
        }
        meta.push_str(&format!(
            r#"
<meta property="article:published_time" content="{}">
<meta property="article:modified_time" content="{}">"#,
            seo.published_at, seo.updated_at,
        ));
        // Twitter Card
        meta.push_str(&format!(
            r#"
<meta name="twitter:card" content="{}">
<meta name="twitter:title" content="{}">
<meta name="twitter:description" content="{}">"#,
            if og_image_full.is_empty() {
                "summary"
            } else {
                "summary_large_image"
            },
            html_escape(&seo.title),
            html_escape(&description),
        ));
        if !og_image_full.is_empty() {
            meta.push_str(&format!(
                "\n<meta name=\"twitter:image\" content=\"{}\">",
                html_escape(&og_image_full)
            ));
        }
        // JSON-LD structured data
        let json_ld_image = if og_image_full.is_empty() {
            "null".to_string()
        } else {
            format!("\"{}\"", og_image_full.replace('"', "\\\""))
        };
        meta.push_str(&format!(
            r#"
<script type="application/ld+json">
{{"@context":"https://schema.org","@type":"BlogPosting","headline":"{}","description":"{}","datePublished":"{}","dateModified":"{}","image":{},"url":"{}","author":{{"@type":"Organization","name":"{}"}}}}</script>"#,
            seo.title.replace('"', "\\\""),
            seo.excerpt.replace('"', "\\\"").chars().take(160).collect::<String>(),
            seo.published_at,
            seo.updated_at,
            json_ld_image,
            canonical_url.replace('"', "\\\""),
            site_name.replace('"', "\\\""),
        ));
        // RSS feed discovery
        if !base_url.is_empty() {
            meta.push_str(&format!(
                "\n<link rel=\"alternate\" type=\"application/rss+xml\" title=\"{}\" href=\"{}/feed.xml\">",
                html_escape(&site_name), base_url
            ));
        }
        // Inject article content into <div id="root"> for crawlers
        let body = format!(
            r#"<article style="display:none" data-noteva-seo="true"><h1>{}</h1><div>{}</div></article>"#,
            html_escape(&seo.title),
            seo.content_html,
        );
        (Some(title), meta, body)
    } else if let Some(ref seo) = page_seo {
        // Page SEO (e.g. /about)
        let title = format!("{} - {}", seo.title, site_name);
        let description = seo.excerpt.replace('"', "&quot;");
        let canonical_url = if base_url.is_empty() {
            String::new()
        } else {
            format!("{}/{}", base_url, seo.slug)
        };
        let mut meta = format!(
            r#"<title>{}</title>
<meta name="description" content="{}">
<link rel="canonical" href="{}">
<meta property="og:title" content="{}">
<meta property="og:description" content="{}">
<meta property="og:type" content="website">
<meta property="og:site_name" content="{}">"#,
            html_escape(&title),
            html_escape(&description),
            html_escape(&canonical_url),
            html_escape(&seo.title),
            html_escape(&description),
            html_escape(&site_name),
        );
        if !canonical_url.is_empty() {
            meta.push_str(&format!(
                "\n<meta property=\"og:url\" content=\"{}\">",
                html_escape(&canonical_url)
            ));
        }
        meta.push_str(&format!(
            r#"
<meta name="twitter:card" content="summary">
<meta name="twitter:title" content="{}">
<meta name="twitter:description" content="{}">"#,
            html_escape(&seo.title),
            html_escape(&description),
        ));
        if !base_url.is_empty() {
            meta.push_str(&format!(
                "\n<link rel=\"alternate\" type=\"application/rss+xml\" title=\"{}\" href=\"{}/feed.xml\">",
                html_escape(&site_name), base_url
            ));
        }
        let body = format!(
            r#"<article style="display:none" data-noteva-seo="true"><h1>{}</h1><div>{}</div></article>"#,
            html_escape(&seo.title),
            seo.content_html,
        );
        (Some(title), meta, body)
    } else {
        // Homepage / list pages
        let canonical_url = if base_url.is_empty() {
            String::new()
        } else if asset_path.is_empty() || asset_path == "index.html" {
            format!("{}/", base_url)
        } else {
            format!("{}/{}", base_url, asset_path.trim_end_matches('/'))
        };
        let mut meta = format!(
            r#"<title>{}</title>
<meta name="description" content="{}">"#,
            html_escape(&site_name),
            html_escape(&site_description),
        );
        if !canonical_url.is_empty() {
            meta.push_str(&format!(
                "\n<link rel=\"canonical\" href=\"{}\">",
                html_escape(&canonical_url)
            ));
            meta.push_str(&format!(
                r#"
<meta property="og:title" content="{}">
<meta property="og:description" content="{}">
<meta property="og:type" content="website">
<meta property="og:url" content="{}">
<meta property="og:site_name" content="{}">
<meta name="twitter:card" content="summary">
<meta name="twitter:title" content="{}">
<meta name="twitter:description" content="{}">"#,
                html_escape(&site_name),
                html_escape(&site_description),
                html_escape(&canonical_url),
                html_escape(&site_name),
                html_escape(&site_name),
                html_escape(&site_description),
            ));
        }
        // JSON-LD for website
        if !base_url.is_empty() {
            meta.push_str(&format!(
                r#"
<script type="application/ld+json">
{{"@context":"https://schema.org","@type":"WebSite","name":"{}","description":"{}","url":"{}"}}</script>"#,
                site_name.replace('"', "\\\""),
                site_description.replace('"', "\\\""),
                base_url.replace('"', "\\\""),
            ));
        }
        if !base_url.is_empty() {
            meta.push_str(&format!(
                "\n<link rel=\"alternate\" type=\"application/rss+xml\" title=\"{}\" href=\"{}/feed.xml\">",
                html_escape(&site_name), base_url
            ));
        }
        (None, meta, String::new())
    };

    // Build head injection: meta tags + config + SDK + plugins + custom CSS
    let custom_css_tag = if custom_css.is_empty() {
        String::new()
    } else {
        format!("\n<style id=\"noteva-custom-css\">{}</style>", custom_css)
    };

    // Load custom locales from file storage for theme i18n
    let custom_locales_json = {
        use crate::services::locale;
        let locales = locale::list_locales().await.unwrap_or_default();
        if locales.is_empty() {
            String::from("[]")
        } else {
            let mut items = Vec::new();
            for loc in &locales {
                if let Ok(Some(full)) = locale::get_locale(&loc.code).await {
                    items.push(serde_json::json!({
                        "code": loc.code,
                        "name": loc.name,
                        "translations": serde_json::from_str::<serde_json::Value>(&full.json_content)
                            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
                    }));
                }
            }
            serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
        }
    };

    let head_injection = format!(
        r#"{}
<script>window.__SITE_CONFIG__={};window.__CUSTOM_LOCALES__={};</script>
<link rel="stylesheet" href="/noteva-sdk.css?v={}">
<script src="/noteva-sdk.js?v={}"></script>
<link rel="stylesheet" href="/api/v1/plugins/assets/plugins.css?v={}">
<script src="/api/v1/plugins/assets/plugins.js?v={}"></script>{}"#,
        meta_tags,
        serde_json::to_string(&config_json).unwrap_or_else(|_| "{}".to_string()),
        custom_locales_json,
        version,
        version,
        version,
        version,
        custom_css_tag
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

    // Inject custom JS before </body>
    if !custom_js.is_empty() {
        let custom_js_tag = format!(r#"<script id="noteva-custom-js">{}</script>"#, custom_js);
        if let Some(pos) = result.find("</body>") {
            result.insert_str(pos, &custom_js_tag);
        }
    }

    Some(result.into_bytes())
}

/// Article SEO data
struct ArticleSeo {
    id: i64,
    title: String,
    excerpt: String,
    content_html: String,
    published_at: String,
    updated_at: String,
    thumbnail: Option<String>,
    slug: String,
}

/// Page SEO data
struct PageSeo {
    title: String,
    excerpt: String,
    content_html: String,
    slug: String,
}

/// Fetch article data for SEO injection
async fn fetch_article_seo(
    slug: &str,
    pool: &crate::db::DynDatabasePool,
    _site_name: &str,
) -> Option<ArticleSeo> {
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
    let excerpt = article
        .content
        .replace('#', "")
        .replace('*', "")
        .replace('`', "")
        .replace('\n', " ")
        .chars()
        .take(200)
        .collect::<String>();

    let published_at = article
        .published_at
        .map(|d| d.to_rfc3339())
        .unwrap_or_default();

    let updated_at = article.updated_at.to_rfc3339();

    Some(ArticleSeo {
        id: article.id,
        title: article.title,
        excerpt,
        content_html: article.content_html,
        published_at,
        updated_at,
        thumbnail: article.thumbnail,
        slug: article.slug,
    })
}

/// Fetch page data for SEO injection
async fn fetch_page_seo(slug: &str, pool: &crate::db::DynDatabasePool) -> Option<PageSeo> {
    use crate::db::repositories::{PageRepository, SqlxPageRepository};

    let repo = SqlxPageRepository::new(pool.clone());
    let page = repo.get_by_slug(slug).await.ok().flatten()?;

    if page.status != crate::models::PageStatus::Published {
        return None;
    }

    let excerpt = page
        .content
        .replace('#', "")
        .replace('*', "")
        .replace('`', "")
        .replace('\n', " ")
        .chars()
        .take(200)
        .collect::<String>();

    Some(PageSeo {
        title: page.title,
        excerpt,
        content_html: page.content_html,
        slug: page.slug,
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
    } else if content_type.starts_with("text/html") {
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
        .body(Body::from(
            "<html><body><h1>404 Not Found</h1></body></html>",
        ))
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
