//! Category API endpoints
//!
//! Handles HTTP requests for category management:
//! - GET /api/v1/categories - Get category tree
//! - GET /api/v1/categories/:slug/articles - Get articles in category
//!
//! Satisfies requirements:
//! - 2.3: Category article listing

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::api::middleware::{ApiError, AppState};
use crate::models::ListParams;

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

/// Response for category tree
#[derive(Debug, Serialize)]
pub struct CategoryTreeResponse {
    pub categories: Vec<CategoryNodeResponse>,
}

/// Response for a category node in the tree
#[derive(Debug, Serialize)]
pub struct CategoryNodeResponse {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub children: Vec<CategoryNodeResponse>,
}

impl From<crate::models::CategoryTree> for CategoryNodeResponse {
    fn from(tree: crate::models::CategoryTree) -> Self {
        Self {
            id: tree.category.id,
            slug: tree.category.slug,
            name: tree.category.name,
            description: tree.category.description,
            children: tree.children.into_iter().map(Into::into).collect(),
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

/// Build the categories router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_category_tree))
        .route("/{slug}/articles", get(get_category_articles))
}

/// GET /api/v1/categories - Get category tree
async fn get_category_tree(
    State(state): State<AppState>,
) -> Result<Json<CategoryTreeResponse>, ApiError> {
    let tree = state
        .category_service
        .list_tree()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let categories: Vec<CategoryNodeResponse> = tree.into_iter().map(Into::into).collect();

    Ok(Json(CategoryTreeResponse { categories }))
}

/// GET /api/v1/categories/:slug/articles - Get articles in category
///
/// Satisfies requirement 2.3: Category article listing
async fn get_category_articles(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<ListArticlesQuery>,
) -> Result<Json<ArticleListResponse>, ApiError> {
    // Get category by slug
    let category = state
        .category_service
        .get_by_slug(&slug)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("Category not found: {}", slug)))?;

    let params = ListParams::new(query.page, query.page_size);

    let result = state
        .article_service
        .list_by_category(category.id, &params)
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
