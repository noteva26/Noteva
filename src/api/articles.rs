//! Article API endpoints
//!
//! Handles HTTP requests for article management:
//! - GET /api/v1/articles - List articles with pagination
//! - GET /api/v1/articles/:slug - Get article by slug
//! - POST /api/v1/articles - Create new article
//! - PUT /api/v1/articles/:id - Update article
//! - DELETE /api/v1/articles/:id - Delete article
//!
//! Satisfies requirements:
//! - 1.1: Article creation
//! - 1.2: Article listing with pagination
//! - 1.3: Article update
//! - 1.4: Article deletion

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};
use crate::models::{ArticleStatus, ListParams};

/// Query parameters for listing articles
#[derive(Debug, Deserialize)]
pub struct ListArticlesQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    /// Filter by status (published, draft, archived)
    pub status: Option<String>,
    /// If true, only return published articles (legacy)
    #[serde(default)]
    pub published_only: bool,
}

fn default_page() -> u32 { 1 }
fn default_page_size() -> u32 { 10 }

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
#[derive(Debug, Serialize, Deserialize)]
pub struct ArticleResponse {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub content_html: String,
    pub author_id: i64,
    pub category_id: i64,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub view_count: i64,
    pub like_count: i64,
    pub comment_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    pub is_pinned: bool,
    pub pin_order: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<CategoryInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<TagInfo>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryInfo {
    pub id: i64,
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagInfo {
    pub id: i64,
    pub slug: String,
    pub name: String,
}

impl From<crate::models::Article> for ArticleResponse {
    fn from(article: crate::models::Article) -> Self {
        Self {
            id: article.id,
            slug: article.slug,
            title: article.title,
            content: article.content,
            content_html: article.content_html,
            author_id: article.author_id,
            category_id: article.category_id,
            status: article.status.to_string(),
            published_at: article.published_at.map(|dt| dt.to_rfc3339()),
            created_at: article.created_at.to_rfc3339(),
            updated_at: article.updated_at.to_rfc3339(),
            view_count: article.view_count,
            like_count: article.like_count,
            comment_count: article.comment_count,
            thumbnail: article.thumbnail,
            is_pinned: article.is_pinned,
            pin_order: article.pin_order,
            category: None,
            tags: None,
        }
    }
}

impl ArticleResponse {
    pub fn with_category(mut self, category: Option<crate::models::Category>) -> Self {
        self.category = category.map(|c| CategoryInfo {
            id: c.id,
            slug: c.slug,
            name: c.name,
        });
        self
    }
    
    pub fn with_tags(mut self, tags: Vec<crate::models::Tag>) -> Self {
        self.tags = Some(tags.into_iter().map(|t| TagInfo {
            id: t.id,
            slug: t.slug,
            name: t.name,
        }).collect());
        self
    }
}

/// Request body for creating an article
#[derive(Debug, Deserialize)]
pub struct CreateArticleRequest {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub slug: String,
    pub category_id: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tag_ids: Option<Vec<i64>>,
}

/// Request body for updating an article
#[derive(Debug, Deserialize)]
pub struct UpdateArticleRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub slug: Option<String>,
    pub category_id: Option<i64>,
    pub status: Option<String>,
    #[serde(default)]
    pub tag_ids: Option<Vec<i64>>,
    pub thumbnail: Option<String>,
    pub is_pinned: Option<bool>,
    pub pin_order: Option<i32>,
}

/// Build the public articles router (read-only)
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_articles))
        .route("/{slug}", get(get_article))
}

/// Build the protected articles router (admin access by ID)
pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/{id}", get(get_article_by_id))
}

// Export handlers for use in mod.rs
pub use create_article as create_article_handler;
pub use update_article as update_article_handler;
pub use delete_article as delete_article_handler;
pub use get_article_by_id as get_article_by_id_handler;
pub use list_articles as list_articles_handler;
pub use get_article as get_article_handler;

/// Build the articles router (legacy, combines both)
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_articles))
        .route("/", post(create_article))
        .route("/{slug}", get(get_article))
        .route("/{id}", put(update_article))
        .route("/{id}", delete(delete_article))
}

/// GET /api/v1/articles - List articles with pagination
///
/// Satisfies requirement 1.2: Article listing with pagination
pub async fn list_articles(
    State(state): State<AppState>,
    Query(query): Query<ListArticlesQuery>,
) -> Result<Json<ArticleListResponse>, ApiError> {
    let params = ListParams::new(query.page, query.page_size);

    // Check if we should filter by published status
    let filter_published = query.published_only || query.status.as_deref() == Some("published");

    let result = if filter_published {
        state
            .article_service
            .list_published(&params)
            .await
            .map_err(|e| ApiError::internal_error(e.to_string()))?
    } else {
        state
            .article_service
            .list(&params)
            .await
            .map_err(|e| ApiError::internal_error(e.to_string()))?
    };

    let total = result.total;
    let page = result.page;
    let per_page = result.per_page;
    let total_pages = result.total_pages();
    
    // Fetch categories and tags for each article
    let mut articles: Vec<ArticleResponse> = Vec::new();
    for article in result.items {
        let category = state.category_service
            .get_by_id(article.category_id)
            .await
            .ok()
            .flatten();
        let tags = state.tag_service
            .get_by_article_id(article.id)
            .await
            .unwrap_or_default();
        
        let response: ArticleResponse = article.into();
        articles.push(response.with_category(category).with_tags(tags));
    }

    Ok(Json(ArticleListResponse {
        articles,
        total,
        page,
        page_size: per_page,
        total_pages,
    }))
}

/// GET /api/v1/articles/:slug - Get article by slug
/// 
/// For public access, only returns published articles.
/// Draft/archived articles return 404 to prevent information leakage.
/// 
/// Triggers hooks:
/// - `article_before_display`: Before returning article data (can modify/filter)
/// - `article_view`: After article is viewed (for statistics, logging)
pub async fn get_article(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<ArticleResponse>, ApiError> {
    let article = state
        .article_service
        .get_by_slug(&slug)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("Article not found: {}", slug)))?;

    // Only return published articles for public access
    if article.status != crate::models::ArticleStatus::Published {
        return Err(ApiError::not_found(format!("Article not found: {}", slug)));
    }

    // Fetch category and tags
    let category = state.category_service
        .get_by_id(article.category_id)
        .await
        .ok()
        .flatten();
    let tags = state.tag_service
        .get_by_article_id(article.id)
        .await
        .unwrap_or_default();

    let mut response: ArticleResponse = article.into();
    response = response.with_category(category).with_tags(tags);
    
    // Trigger article_before_display hook (can modify article data)
    let hook_data = serde_json::json!({
        "article": &response,
        "context": {
            "slug": &slug,
            "is_public": true
        }
    });
    let modified = state.hook_manager.trigger(
        crate::plugin::hook_names::ARTICLE_BEFORE_DISPLAY,
        hook_data
    );
    
    // If hook returned modified article, use it
    if let Some(modified_article) = modified.get("article") {
        if let Ok(updated) = serde_json::from_value::<ArticleResponse>(modified_article.clone()) {
            response = updated;
        }
    }
    
    // Trigger article_view hook (for statistics, logging - fire and forget)
    let view_data = serde_json::json!({
        "article_id": response.id,
        "slug": &slug,
        "title": &response.title,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    state.hook_manager.trigger(
        crate::plugin::hook_names::ARTICLE_VIEW,
        view_data
    );

    Ok(Json(response))
}

/// GET /api/v1/admin/articles/:id - Get article by ID (admin only)
/// 
/// Returns any article regardless of status for editing purposes.
/// Requires authentication.
pub async fn get_article_by_id(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<Json<ArticleResponse>, ApiError> {
    let article = state
        .article_service
        .get_by_id(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("Article not found: {}", id)))?;

    // Fetch category and tags
    let category = state.category_service
        .get_by_id(article.category_id)
        .await
        .ok()
        .flatten();
    let tags = state.tag_service
        .get_by_article_id(article.id)
        .await
        .unwrap_or_default();

    let response: ArticleResponse = article.into();
    Ok(Json(response.with_category(category).with_tags(tags)))
}

/// POST /api/v1/articles - Create new article
///
/// Requires authentication.
/// Satisfies requirement 1.1: Article creation
pub async fn create_article(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateArticleRequest>,
) -> Result<(StatusCode, Json<ArticleResponse>), ApiError> {
    let status = body
        .status
        .as_ref()
        .and_then(|s| ArticleStatus::from_str(s));

    let input = crate::models::CreateArticleInput {
        title: body.title,
        content: body.content,
        content_html: None,
        slug: body.slug,
        author_id: user.0.id,
        category_id: body.category_id.unwrap_or(1), // Default category
        status,
    };

    let article = state
        .article_service
        .create(input, body.tag_ids)
        .await
        .map_err(|e| match e {
            crate::services::article::ArticleServiceError::ValidationError(msg) => {
                ApiError::validation_error(msg)
            }
            crate::services::article::ArticleServiceError::DuplicateSlug(slug) => {
                ApiError::with_details(
                    "CONFLICT",
                    format!("Article slug already exists: {}", slug),
                    serde_json::json!({"field": "slug", "value": slug}),
                )
            }
            _ => ApiError::internal_error(e.to_string()),
        })?;

    Ok((StatusCode::CREATED, Json(article.into())))
}

/// PUT /api/v1/articles/:id - Update article
///
/// Requires authentication and permission to edit.
/// Satisfies requirement 1.3: Article update
pub async fn update_article(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<i64>,
    Json(body): Json<UpdateArticleRequest>,
) -> Result<Json<ArticleResponse>, ApiError> {
    // Check if article exists and user can edit
    let existing = state
        .article_service
        .get_by_id(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("Article not found: {}", id)))?;

    if !user.0.can_edit(existing.author_id) {
        return Err(ApiError::forbidden("You don't have permission to edit this article"));
    }

    let status = body.status.as_ref().and_then(|s| ArticleStatus::from_str(s));

    let input = crate::models::UpdateArticleInput {
        title: body.title,
        content: body.content,
        content_html: None,
        slug: body.slug,
        category_id: body.category_id,
        status,
        thumbnail: body.thumbnail,
        is_pinned: body.is_pinned,
        pin_order: body.pin_order,
    };

    let article = state
        .article_service
        .update(id, input, body.tag_ids)
        .await
        .map_err(|e| match e {
            crate::services::article::ArticleServiceError::NotFound(_) => {
                ApiError::not_found(format!("Article not found: {}", id))
            }
            crate::services::article::ArticleServiceError::ValidationError(msg) => {
                ApiError::validation_error(msg)
            }
            crate::services::article::ArticleServiceError::DuplicateSlug(slug) => {
                ApiError::with_details(
                    "CONFLICT",
                    format!("Article slug already exists: {}", slug),
                    serde_json::json!({"field": "slug", "value": slug}),
                )
            }
            _ => ApiError::internal_error(e.to_string()),
        })?;

    Ok(Json(article.into()))
}

/// DELETE /api/v1/articles/:id - Delete article
///
/// Requires authentication and permission to edit.
/// Satisfies requirement 1.4: Article deletion
pub async fn delete_article(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    // Check if article exists and user can edit
    let existing = state
        .article_service
        .get_by_id(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("Article not found: {}", id)))?;

    if !user.0.can_edit(existing.author_id) {
        return Err(ApiError::forbidden("You don't have permission to delete this article"));
    }

    state
        .article_service
        .delete(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// Extractor for AuthenticatedUser from request extensions
impl<S> axum::extract::FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut axum::http::request::Parts,
        _state: &'life1 S,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            parts
                .extensions
                .get::<AuthenticatedUser>()
                .cloned()
                .ok_or_else(|| ApiError::unauthorized("Authentication required"))
        })
    }
}
