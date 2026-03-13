//! SEO endpoints: sitemap.xml, robots.txt, RSS feed
//!
//! All endpoints are public and cacheable.

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::Response,
};
use chrono::{DateTime, Utc};

use crate::api::middleware::AppState;
use crate::db::repositories::{
    ArticleRepository, CategoryRepository, PageRepository, SqlxArticleRepository,
    SqlxCategoryRepository, SqlxPageRepository, SqlxSettingsRepository, SettingsRepository,
    TagRepository, SqlxTagRepository,
};
use crate::models::ArticleStatus;

/// Helper: get site_url from settings, fallback to empty string
async fn get_site_url(state: &AppState) -> String {
    state.settings_service
        .get("site_url")
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Helper: get site_name from settings
async fn get_site_name(state: &AppState) -> String {
    let repo = SqlxSettingsRepository::new(state.pool.clone());
    repo.get("site_name")
        .await
        .ok()
        .flatten()
        .map(|s| s.value)
        .unwrap_or_else(|| "Noteva".to_string())
}

/// Helper: get site_description from settings
async fn get_site_description(state: &AppState) -> String {
    let repo = SqlxSettingsRepository::new(state.pool.clone());
    repo.get("site_description")
        .await
        .ok()
        .flatten()
        .map(|s| s.value)
        .unwrap_or_default()
}

/// Helper: build article URL based on permalink_structure setting
async fn build_article_url(base: &str, id: i64, slug: &str, state: &AppState) -> String {
    let permalink_structure = state.settings_service
        .get(crate::services::settings::keys::PERMALINK_STRUCTURE)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "/posts/{slug}".to_string());

    let identifier = if permalink_structure.contains("{id}") {
        id.to_string()
    } else {
        slug.to_string()
    };
    format!("{}/posts/{}", base.trim_end_matches('/'), identifier)
}

/// XML-escape a string
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

/// Format datetime as W3C format for sitemap
fn w3c_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S+00:00").to_string()
}

/// Format datetime as RFC 2822 for RSS
fn rfc2822_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%a, %d %b %Y %H:%M:%S +0000").to_string()
}

// ============================================================================
// GET /robots.txt
// ============================================================================

pub async fn robots_txt(State(state): State<AppState>) -> Response {
    let site_url = get_site_url(&state).await;

    let body = if site_url.is_empty() {
        "User-agent: *\nAllow: /\nDisallow: /manage/\nDisallow: /api/\n".to_string()
    } else {
        format!(
            "User-agent: *\nAllow: /\nDisallow: /manage/\nDisallow: /api/\n\nSitemap: {}/sitemap.xml\n",
            site_url.trim_end_matches('/')
        )
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(body))
        .unwrap()
}

// ============================================================================
// GET /sitemap.xml
// ============================================================================

pub async fn sitemap_xml(State(state): State<AppState>) -> Response {
    let site_url = get_site_url(&state).await;
    if site_url.is_empty() {
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from("site_url not configured"))
            .unwrap();
    }
    let base = site_url.trim_end_matches('/');

    let mut xml = String::with_capacity(8192);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);
    xml.push('\n');

    // Homepage
    xml.push_str(&format!(
        "  <url>\n    <loc>{}/</loc>\n    <changefreq>daily</changefreq>\n    <priority>1.0</priority>\n  </url>\n",
        base
    ));

    // Published articles (up to 5000)
    let article_repo = SqlxArticleRepository::new(state.pool.clone());
    if let Ok(articles) = article_repo.list_published(0, 5000).await {
        for article in &articles {
            if article.status != ArticleStatus::Published {
                continue;
            }
            let url = build_article_url(base, article.id, &article.slug, &state).await;
            let lastmod = w3c_datetime(&article.updated_at);
            xml.push_str(&format!(
                "  <url>\n    <loc>{}</loc>\n    <lastmod>{}</lastmod>\n    <changefreq>weekly</changefreq>\n    <priority>0.8</priority>\n  </url>\n",
                xml_escape(&url), lastmod
            ));
        }
    }

    // Published pages
    let page_repo = SqlxPageRepository::new(state.pool.clone());
    if let Ok(pages) = page_repo.list_published().await {
        for page in &pages {
            let url = format!("{}/{}", base, page.slug);
            let lastmod = w3c_datetime(&page.updated_at);
            xml.push_str(&format!(
                "  <url>\n    <loc>{}</loc>\n    <lastmod>{}</lastmod>\n    <changefreq>monthly</changefreq>\n    <priority>0.6</priority>\n  </url>\n",
                xml_escape(&url), lastmod
            ));
        }
    }

    // Categories
    let cat_repo = SqlxCategoryRepository::new(state.pool.clone());
    if let Ok(categories) = cat_repo.list().await {
        for cat in &categories {
            let url = format!("{}/categories/{}", base, cat.slug);
            xml.push_str(&format!(
                "  <url>\n    <loc>{}</loc>\n    <changefreq>weekly</changefreq>\n    <priority>0.5</priority>\n  </url>\n",
                xml_escape(&url)
            ));
        }
    }

    // Tags
    let tag_repo = SqlxTagRepository::new(state.pool.clone());
    if let Ok(tags) = tag_repo.list().await {
        for tag in &tags {
            let url = format!("{}/tags/{}", base, tag.slug);
            xml.push_str(&format!(
                "  <url>\n    <loc>{}</loc>\n    <changefreq>weekly</changefreq>\n    <priority>0.4</priority>\n  </url>\n",
                xml_escape(&url)
            ));
        }
    }

    xml.push_str("</urlset>\n");

    // Hook: sitemap_filter — allow plugins to modify sitemap XML
    let hook_result = state.hook_manager.trigger(
        "sitemap_filter",
        serde_json::json!({ "xml": xml }),
    );
    if let Some(modified) = hook_result.get("xml").and_then(|v| v.as_str()) {
        xml = modified.to_string();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(xml))
        .unwrap()
}

// ============================================================================
// GET /feed.xml (RSS 2.0)
// ============================================================================

pub async fn feed_xml(State(state): State<AppState>) -> Response {
    let site_url = get_site_url(&state).await;
    let base = if site_url.is_empty() { "" } else { site_url.trim_end_matches('/') };
    let site_name = get_site_name(&state).await;
    let site_desc = get_site_description(&state).await;

    let mut xml = String::with_capacity(16384);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:content="http://purl.org/rss/1.0/modules/content/">"#);
    xml.push('\n');
    xml.push_str("<channel>\n");
    xml.push_str(&format!("  <title>{}</title>\n", xml_escape(&site_name)));
    if !base.is_empty() {
        xml.push_str(&format!("  <link>{}</link>\n", xml_escape(base)));
        xml.push_str(&format!(
            "  <atom:link href=\"{}/feed.xml\" rel=\"self\" type=\"application/rss+xml\" />\n",
            xml_escape(base)
        ));
    }
    xml.push_str(&format!("  <description>{}</description>\n", xml_escape(&site_desc)));
    xml.push_str(&format!("  <generator>Noteva {}</generator>\n", env!("CARGO_PKG_VERSION")));
    let lang = state.settings_service
        .get("site_language")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "zh-CN".to_string());
    xml.push_str(&format!("  <language>{}</language>\n", xml_escape(&lang)));

    // Latest 50 published articles
    let article_repo = SqlxArticleRepository::new(state.pool.clone());
    if let Ok(articles) = article_repo.list_published(0, 50).await {
        // Build date from most recent article
        if let Some(first) = articles.first() {
            let pub_date = first.published_at.unwrap_or(first.created_at);
            xml.push_str(&format!("  <lastBuildDate>{}</lastBuildDate>\n", rfc2822_datetime(&pub_date)));
        }

        for article in &articles {
            if article.status != ArticleStatus::Published {
                continue;
            }
            let pub_date = article.published_at.unwrap_or(article.created_at);
            let url = build_article_url(base, article.id, &article.slug, &state).await;

            // Generate excerpt: strip markdown, limit 300 chars
            let excerpt: String = article.content
                .replace('#', "")
                .replace('*', "")
                .replace('`', "")
                .replace('\n', " ")
                .chars()
                .take(300)
                .collect();

            xml.push_str("  <item>\n");
            xml.push_str(&format!("    <title>{}</title>\n", xml_escape(&article.title)));
            xml.push_str(&format!("    <link>{}</link>\n", xml_escape(&url)));
            xml.push_str(&format!("    <guid isPermaLink=\"true\">{}</guid>\n", xml_escape(&url)));
            xml.push_str(&format!("    <pubDate>{}</pubDate>\n", rfc2822_datetime(&pub_date)));
            xml.push_str(&format!("    <description>{}</description>\n", xml_escape(&excerpt)));
            // Include full HTML content in CDATA
            xml.push_str("    <content:encoded><![CDATA[");
            xml.push_str(&article.content_html);
            xml.push_str("]]></content:encoded>\n");
            if let Some(ref thumb) = article.thumbnail {
                if !thumb.is_empty() {
                    let img_url = if thumb.starts_with("http") {
                        thumb.clone()
                    } else {
                        format!("{}{}", base, thumb)
                    };
                    xml.push_str(&format!("    <enclosure url=\"{}\" type=\"image/jpeg\" />\n", xml_escape(&img_url)));
                }
            }
            xml.push_str("  </item>\n");
        }
    }

    xml.push_str("</channel>\n</rss>\n");

    // Hook: feed_filter — allow plugins to modify RSS XML
    let hook_result = state.hook_manager.trigger(
        "feed_filter",
        serde_json::json!({ "xml": xml }),
    );
    if let Some(modified) = hook_result.get("xml").and_then(|v| v.as_str()) {
        xml = modified.to_string();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")
        .header(header::CACHE_CONTROL, "public, max-age=1800")
        .body(Body::from(xml))
        .unwrap()
}
