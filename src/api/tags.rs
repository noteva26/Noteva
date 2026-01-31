//! Tag API endpoints
//!
//! Handles HTTP requests for tag management:
//! - GET /api/v1/tags - Get tag list/cloud
//! - GET /api/v1/tags/:slug/articles - Get articles with tag
//!
//! Satisfies requirements:
//! - 3.2: Tag article listing
//! - 3.4: Tag cloud

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::api::middleware::{ApiError, AppState};
use crate::models::ListParams;

/// Query parameters for tag list
#[derive(Debug, Deserialize)]
pub struct ListTagsQuery {
    /// If true, return tag cloud with counts sorted by frequency
    #[serde(default)]
    pub cloud: bool,
    /// Limit for tag cloud
    #[serde(default = "default_cloud_limit")]
    pub limit: usize,
}

fn default_cloud_limit() -> usize { 50 }

/// Query parameters for listing articles
#[derive(Debug, Deserialize)]
pub struct ListArticlesQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 { 1 }
fn default_page_size() -> u32 { 10 }

/// Response for tag list
#[derive(Debug, Serialize)]
pub struct TagListResponse {
    pub tags: Vec<TagResponse>,
}

/// Response for a single tag
#[derive(Debug, Serialize)]
pub struct TagResponse {
    pub id: i64,
    pub slug: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub article_count: Option<i64>,
}

impl From<crate::models::Tag> for TagResponse {
    fn from(tag: crate::models::Tag) -> Self {
        Self {
            id: tag.id,
            slug: tag.slug,
            name: tag.name,
            article_count: None,
        }
    }
}

impl From<crate::models::TagWithCount> for TagResponse {
    fn from(twc: crate::models::TagWithCount) -> Self {
        Self {
            id: twc.tag.id,
            slug: twc.tag.slug,
            name: twc.tag.name,
            article_count: Some(twc.article_count),
        }
    }
}

/// Response for article list
#[derive(Debug, Serialize)]
pub struct ArticleListResponse {
    pub articles: Vec<ArticleResponse>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// Response for a single article
#[derive(Debug, Serialize)]
pub struct ArticleResponse {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
}

impl From<crate::models::Article> for ArticleResponse {
    fn from(article: crate::models::Article) -> Self {
        Self {
            id: article.id,
            slug: article.slug,
            title: article.title,
            status: article.status.to_string(),
            published_at: article.published_at.map(|dt| dt.to_rfc3339()),
            created_at: article.created_at.to_rfc3339(),
        }
    }
}

/// Build the tags router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tags))
        .route("/{slug}/articles", get(get_tag_articles))
}

/// GET /api/v1/tags - Get tag list or tag cloud
///
/// Satisfies requirement 3.4: Tag cloud
async fn list_tags(
    State(state): State<AppState>,
    Query(query): Query<ListTagsQuery>,
) -> Result<Json<TagListResponse>, ApiError> {
    let tags = if query.cloud {
        // Return tag cloud with counts sorted by frequency
        let cloud = state
            .tag_service
            .get_tag_cloud(query.limit)
            .await
            .map_err(|e| ApiError::internal_error(e.to_string()))?;

        cloud.into_iter().map(TagResponse::from).collect()
    } else {
        // Return simple tag list
        let list = state
            .tag_service
            .list()
            .await
            .map_err(|e| ApiError::internal_error(e.to_string()))?;

        list.into_iter().map(TagResponse::from).collect()
    };

    Ok(Json(TagListResponse { tags }))
}

/// GET /api/v1/tags/:slug/articles - Get articles with tag
///
/// Satisfies requirement 3.2: Tag article listing
async fn get_tag_articles(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<ListArticlesQuery>,
) -> Result<Json<ArticleListResponse>, ApiError> {
    // Get tag by slug
    let tag = state
        .tag_service
        .get_by_slug(&slug)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("Tag not found: {}", slug)))?;

    let params = ListParams::new(query.page, query.page_size);

    let result = state
        .article_service
        .list_by_tag(tag.id, &params)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let total = result.total;
    let page = result.page;
    let per_page = result.per_page;
    let total_pages = result.total_pages();
    let articles: Vec<ArticleResponse> = result.items.into_iter().map(Into::into).collect();

    Ok(Json(ArticleListResponse {
        articles,
        total,
        page,
        page_size: per_page,
        total_pages,
    }))
}
