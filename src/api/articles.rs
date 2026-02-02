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
use serde::Deserialize;

use crate::api::common::{default_page, default_page_size};
use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};
use crate::api::responses::{ArticleResponse, PaginatedArticlesResponse};
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
    /// Search keyword for title and content
    pub keyword: Option<String>,
    /// Filter by category (ID or slug)
    pub category: Option<String>,
    /// Filter by tag (ID or slug)
    pub tag: Option<String>,
}

/// Query parameters for resolving article path
#[derive(Debug, Deserialize)]
pub struct ResolveArticleQuery {
    /// The path to resolve (e.g., "hello-world" or "42")
    pub path: String,
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
pub use resolve_article as resolve_article_handler;

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
) -> Result<Json<PaginatedArticlesResponse>, ApiError> {
    let params = ListParams::new(query.page, query.page_size);

    // Check if we should filter by published status
    let filter_published = query.published_only || query.status.as_deref() == Some("published");

    // Resolve category: try as ID first, then as slug
    let category_id = if let Some(ref cat) = query.category {
        if let Ok(id) = cat.parse::<i64>() {
            Some(id)
        } else {
            // Try to find by slug
            state.category_service
                .get_by_slug(cat)
                .await
                .ok()
                .flatten()
                .map(|c| c.id)
        }
    } else {
        None
    };

    // Resolve tag: try as ID first, then as slug
    let tag_id = if let Some(ref t) = query.tag {
        if let Ok(id) = t.parse::<i64>() {
            Some(id)
        } else {
            // Try to find by slug
            state.tag_service
                .get_by_slug(t)
                .await
                .ok()
                .flatten()
                .map(|t| t.id)
        }
    } else {
        None
    };

    let result = if let Some(ref keyword) = query.keyword {
        // Search by keyword
        state
            .article_service
            .search(keyword, &params, filter_published)
            .await
            .map_err(|e| ApiError::internal_error(e.to_string()))?
    } else if let Some(cat_id) = category_id {
        // Filter by category
        state
            .article_service
            .list_by_category(cat_id, &params)
            .await
            .map_err(|e| ApiError::internal_error(e.to_string()))?
    } else if let Some(t_id) = tag_id {
        // Filter by tag
        state
            .article_service
            .list_by_tag(t_id, &params)
            .await
            .map_err(|e| ApiError::internal_error(e.to_string()))?
    } else if filter_published {
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

    Ok(Json(PaginatedArticlesResponse {
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
    Path(identifier): Path<String>,
) -> Result<Json<ArticleResponse>, ApiError> {
    // Try to parse as ID first, then fall back to slug
    let article = if let Ok(id) = identifier.parse::<i64>() {
        // It's a numeric ID
        state
            .article_service
            .get_by_id(id)
            .await
            .map_err(|e| ApiError::internal_error(e.to_string()))?
            .ok_or_else(|| ApiError::not_found(format!("Article not found: {}", identifier)))?
    } else {
        // It's a slug
        state
            .article_service
            .get_by_slug(&identifier)
            .await
            .map_err(|e| ApiError::internal_error(e.to_string()))?
            .ok_or_else(|| ApiError::not_found(format!("Article not found: {}", identifier)))?
    };

    // Only return published articles for public access
    if article.status != crate::models::ArticleStatus::Published {
        return Err(ApiError::not_found(format!("Article not found: {}", identifier)));
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
            "identifier": &identifier,
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
        "identifier": &identifier,
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

/// Response for resolve endpoint
#[derive(Debug, serde::Serialize)]
pub struct ResolveArticleResponse {
    /// The resolved article
    pub article: ArticleResponse,
    /// The canonical URL for this article (based on current permalink setting)
    pub canonical_url: String,
    /// Whether a redirect is recommended (path doesn't match canonical)
    pub should_redirect: bool,
}

/// GET /api/v1/articles/resolve - Resolve article by path
/// 
/// This endpoint resolves an article from various path formats:
/// - By slug: "hello-world"
/// - By ID: "42"
/// 
/// Returns the article along with its canonical URL based on current permalink settings.
/// If the requested path doesn't match the canonical URL, `should_redirect` will be true.
pub async fn resolve_article(
    State(state): State<AppState>,
    Query(query): Query<ResolveArticleQuery>,
) -> Result<Json<ResolveArticleResponse>, ApiError> {
    let path = query.path.trim_start_matches('/');
    
    // Try to find article by different methods
    let article = if let Ok(id) = path.parse::<i64>() {
        // Try as ID first
        state.article_service.get_by_id(id).await
            .map_err(|e| ApiError::internal_error(e.to_string()))?
    } else {
        // Try as slug
        state.article_service.get_by_slug(path).await
            .map_err(|e| ApiError::internal_error(e.to_string()))?
    };
    
    let article = article.ok_or_else(|| ApiError::not_found("Article not found"))?;
    
    // Only return published articles for public access
    if article.status != crate::models::ArticleStatus::Published {
        return Err(ApiError::not_found("Article not found"));
    }
    
    // Get permalink structure from settings
    let permalink_structure = state.settings_service
        .get(crate::services::settings::keys::PERMALINK_STRUCTURE)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "/posts/{slug}".to_string());
    
    // Generate canonical URL
    let canonical_url = crate::services::settings::generate_article_url(
        &permalink_structure,
        article.id,
        &article.slug,
        article.published_at.as_ref(),
    );
    
    // Check if redirect is needed
    let requested_path = format!("/posts/{}", path);
    let should_redirect = requested_path != canonical_url;
    
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
    let response = response.with_category(category).with_tags(tags);
    
    Ok(Json(ResolveArticleResponse {
        article: response,
        canonical_url,
        should_redirect,
    }))
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
