//! Comment API endpoints

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::middleware::{ApiError, AppState};
use crate::models::{CommentWithMeta, CreateCommentInput, LikeTargetType};
use crate::services::generate_fingerprint;

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct CommentsResponse {
    pub comments: Vec<CommentWithMeta>,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub comment: crate::models::Comment,
}

#[derive(Debug, Serialize)]
pub struct LikeResponse {
    pub success: bool,
    pub liked: bool,
    pub like_count: i64,
}

#[derive(Debug, Serialize)]
pub struct LikeStatusResponse {
    pub liked: bool,
}

// ============================================================================
// Request Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub article_id: i64,
    pub parent_id: Option<i64>,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct LikeRequest {
    pub target_type: String,
    pub target_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct CheckLikeQuery {
    pub target_type: String,
    pub target_id: i64,
}

// ============================================================================
// Handlers
// ============================================================================

/// Get comments for an article
pub async fn get_comments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(article_id): Path<i64>,
) -> Result<Json<CommentsResponse>, ApiError> {
    let fingerprint = extract_fingerprint(&headers);
    
    let comments = state
        .comment_service
        .get_by_article(article_id, fingerprint.as_deref())
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    
    // Trigger comment_before_display hook
    let hook_data = serde_json::json!({
        "article_id": article_id,
        "comments": &comments,
        "count": comments.len()
    });
    let modified = state.hook_manager.trigger(
        crate::plugin::hook_names::COMMENT_BEFORE_DISPLAY,
        hook_data,
    );
    
    // Use modified comments if hook returned them
    if let Some(modified_comments) = modified.get("comments") {
        if let Ok(comments) = serde_json::from_value(modified_comments.clone()) {
            return Ok(Json(CommentsResponse { comments }));
        }
    }
    
    Ok(Json(CommentsResponse { comments }))
}

/// Create a comment
pub async fn create_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<CommentResponse>), ApiError> {
    // Check if login is required
    let require_login = state
        .comment_service
        .check_require_login()
        .await
        .unwrap_or(false);
    
    // Try to get user from session
    let user_id = get_user_id_from_headers(&state, &headers).await;
    
    if require_login && user_id.is_none() {
        return Err(ApiError::unauthorized("Login required to comment"));
    }
    
    // For guest comments, require nickname
    if user_id.is_none() && req.nickname.as_ref().map(|n| n.trim().is_empty()).unwrap_or(true) {
        return Err(ApiError::validation_error("Nickname is required for guest comments"));
    }
    
    if req.content.trim().is_empty() {
        return Err(ApiError::validation_error("Content is required"));
    }
    
    let ip = extract_ip(&headers);
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    
    let input = CreateCommentInput {
        article_id: req.article_id,
        parent_id: req.parent_id,
        nickname: req.nickname,
        email: req.email,
        content: req.content,
    };
    
    let comment = state
        .comment_service
        .create(input, user_id, ip, ua)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    
    Ok((StatusCode::CREATED, Json(CommentResponse { comment })))
}

/// Check if user has liked an article or comment
pub async fn check_like(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CheckLikeQuery>,
) -> Result<Json<LikeStatusResponse>, ApiError> {
    let target_type = parse_target_type(&query.target_type)?;
    
    let user_id = get_user_id_from_headers(&state, &headers).await;
    let fingerprint = if user_id.is_none() {
        extract_fingerprint(&headers)
    } else {
        None
    };
    
    let liked = state
        .comment_service
        .is_liked(target_type, query.target_id, user_id, fingerprint.as_deref())
        .await
        .unwrap_or(false);
    
    Ok(Json(LikeStatusResponse { liked }))
}

/// Like an article or comment
pub async fn like(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LikeRequest>,
) -> Result<Json<LikeResponse>, ApiError> {
    let target_type = parse_target_type(&req.target_type)?;
    
    let user_id = get_user_id_from_headers(&state, &headers).await;
    let fingerprint = if user_id.is_none() {
        extract_fingerprint(&headers)
    } else {
        None
    };
    
    if user_id.is_none() && fingerprint.is_none() {
        return Err(ApiError::validation_error("Unable to identify user"));
    }
    
    // Check if already liked
    let is_liked = state
        .comment_service
        .is_liked(target_type.clone(), req.target_id, user_id, fingerprint.as_deref())
        .await
        .unwrap_or(false);
    
    let result = if is_liked {
        state.comment_service.unlike(target_type.clone(), req.target_id, user_id, fingerprint).await
    } else {
        state.comment_service.like(target_type.clone(), req.target_id, user_id, fingerprint).await
    };
    
    result.map_err(|e| ApiError::internal_error(e.to_string()))?;
    
    // Get updated like count for articles and clear cache
    let like_count = if target_type == LikeTargetType::Article {
        let article_repo = crate::db::repositories::SqlxArticleRepository::new(state.pool.clone());
        use crate::db::repositories::ArticleRepository;
        let article = article_repo.get_by_id(req.target_id).await.ok().flatten();
        
        if let Some(ref art) = article {
            let _ = state.article_service.invalidate_article_cache(art.id, &art.slug).await;
        }
        
        article.map(|a| a.like_count).unwrap_or(0)
    } else {
        0
    };
    
    Ok(Json(LikeResponse {
        success: true,
        liked: !is_liked,
        like_count,
    }))
}

/// Increment view count for an article
pub async fn increment_view(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state
        .comment_service
        .increment_view(article_id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    
    // Clear article cache so next request gets fresh view count
    let article_repo = crate::db::repositories::SqlxArticleRepository::new(state.pool.clone());
    use crate::db::repositories::ArticleRepository;
    if let Ok(Some(art)) = article_repo.get_by_id(article_id).await {
        let _ = state.article_service.invalidate_article_cache(art.id, &art.slug).await;
    }
    
    Ok(StatusCode::OK)
}

/// Delete a comment (admin only)
pub async fn delete_comment(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let deleted = state
        .comment_service
        .delete(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("Comment not found"))
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse target type string to enum
fn parse_target_type(s: &str) -> Result<LikeTargetType, ApiError> {
    match s {
        "article" => Ok(LikeTargetType::Article),
        "comment" => Ok(LikeTargetType::Comment),
        _ => Err(ApiError::validation_error("Invalid target type")),
    }
}

/// Extract IP from headers
fn extract_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
}

/// Extract fingerprint from headers
fn extract_fingerprint(headers: &HeaderMap) -> Option<String> {
    let ip = extract_ip(headers)?;
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    Some(generate_fingerprint(&ip, ua))
}

/// Get user ID from session cookie
async fn get_user_id_from_headers(state: &AppState, headers: &HeaderMap) -> Option<i64> {
    let cookie = headers.get("cookie")?.to_str().ok()?;
    let session_id = cookie.split(';').find_map(|c| {
        let c = c.trim();
        c.strip_prefix("session=")
    })?;
    
    let user = state.user_service.validate_session(session_id).await.ok()??;
    Some(user.id)
}
