//! Category API endpoints
//!
//! Handles HTTP requests for category management:
//! - GET /api/v1/categories - Get flat category list
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

use crate::api::common::{default_page, default_page_size};
use crate::api::middleware::{ApiError, AppState};
use crate::api::responses::{ArticleSummary, PaginatedArticleSummaryResponse};
use crate::models::ListParams;

/// Query parameters for listing articles
#[derive(Debug, Deserialize)]
pub struct ListArticlesQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

/// Response for category list
#[derive(Debug, Serialize)]
pub struct CategoryTreeResponse {
    pub categories: Vec<CategoryNodeResponse>,
}

/// Response for a single category (flat, no children)
#[derive(Debug, Serialize)]
pub struct CategoryNodeResponse {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
}

/// Build the categories router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_category_list))
        .route("/{slug}/articles", get(get_category_articles))
}

/// GET /api/v1/categories - Get flat category list
async fn get_category_list(
    State(state): State<AppState>,
) -> Result<Json<CategoryTreeResponse>, ApiError> {
    let all = state
        .category_service
        .list()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let categories: Vec<CategoryNodeResponse> = all
        .into_iter()
        .map(|c| CategoryNodeResponse {
            id: c.id,
            slug: c.slug,
            name: c.name,
            description: c.description,
        })
        .collect();

    Ok(Json(CategoryTreeResponse { categories }))
}

/// GET /api/v1/categories/:slug/articles - Get articles in category
///
/// Satisfies requirement 2.3: Category article listing
async fn get_category_articles(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<ListArticlesQuery>,
) -> Result<Json<PaginatedArticleSummaryResponse>, ApiError> {
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
    let articles: Vec<ArticleSummary> = result.items.into_iter().map(Into::into).collect();

    Ok(Json(PaginatedArticleSummaryResponse {
        articles,
        total,
        page,
        page_size: per_page,
        total_pages,
    }))
}
