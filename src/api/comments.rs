//! Comment API endpoints

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::middleware::AppState;
use crate::db::repositories::{CommentRepositoryImpl, SettingsRepository, SqlxSettingsRepository};
use crate::models::{CreateCommentInput, LikeTargetType};
use crate::services::{generate_fingerprint, CommentService};

/// Get comments for an article
pub async fn get_comments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(article_id): Path<i64>,
) -> impl IntoResponse {
    let repo = Arc::new(CommentRepositoryImpl::new(state.pool.clone()));
    let service = CommentService::with_hooks(repo, state.hook_manager.clone());
    
    let fingerprint = extract_fingerprint(&headers);
    
    match service.get_by_article(article_id, fingerprint.as_deref()).await {
        Ok(comments) => {
            // Trigger comment_before_display hook
            let hook_data = serde_json::json!({
                "article_id": article_id,
                "comments": &comments,
                "count": comments.len()
            });
            let modified = state.hook_manager.trigger(
                crate::plugin::hook_names::COMMENT_BEFORE_DISPLAY,
                hook_data
            );
            
            // Use modified comments if hook returned them
            if let Some(modified_comments) = modified.get("comments") {
                return Json(serde_json::json!({ "comments": modified_comments })).into_response();
            }
            
            Json(serde_json::json!({ "comments": comments })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub article_id: i64,
    pub parent_id: Option<i64>,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub content: String,
}

/// Create a comment
pub async fn create_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateCommentRequest>,
) -> impl IntoResponse {
    // Check if login is required
    let settings_repo = SqlxSettingsRepository::new(state.pool.clone());
    let require_login = settings_repo
        .get("require_login_to_comment")
        .await
        .ok()
        .flatten()
        .map(|s| s.value == "true")
        .unwrap_or(false);
    
    // Try to get user from session
    let user_id = get_user_id_from_headers(&state, &headers).await;
    
    if require_login && user_id.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Login required to comment" })),
        ).into_response();
    }
    
    // For guest comments, require nickname
    if user_id.is_none() && req.nickname.as_ref().map(|n| n.trim().is_empty()).unwrap_or(true) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Nickname is required for guest comments" })),
        ).into_response();
    }
    
    if req.content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Content is required" })),
        ).into_response();
    }
    
    let repo = Arc::new(CommentRepositoryImpl::new(state.pool.clone()));
    let service = CommentService::with_hooks(repo, state.hook_manager.clone());
    
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
    
    match service.create(input, user_id, ip, ua).await {
        Ok(comment) => (StatusCode::CREATED, Json(serde_json::json!({ "comment": comment }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct LikeRequest {
    pub target_type: String,
    pub target_id: i64,
}

#[derive(Debug, Serialize)]
pub struct LikeResponse {
    pub success: bool,
    pub liked: bool,
    pub like_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct CheckLikeQuery {
    pub target_type: String,
    pub target_id: i64,
}

/// Check if user has liked an article or comment
pub async fn check_like(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<CheckLikeQuery>,
) -> impl IntoResponse {
    let target_type = match query.target_type.as_str() {
        "article" => LikeTargetType::Article,
        "comment" => LikeTargetType::Comment,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid target type" })),
            ).into_response();
        }
    };
    
    let user_id = get_user_id_from_headers(&state, &headers).await;
    let fingerprint = if user_id.is_none() {
        extract_fingerprint(&headers)
    } else {
        None
    };
    
    let repo = Arc::new(CommentRepositoryImpl::new(state.pool.clone()));
    let service = CommentService::new(repo);
    
    let is_liked = service
        .is_liked(target_type, query.target_id, user_id, fingerprint.as_deref())
        .await
        .unwrap_or(false);
    
    Json(serde_json::json!({ "liked": is_liked })).into_response()
}

/// Like an article or comment
pub async fn like(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LikeRequest>,
) -> impl IntoResponse {
    let target_type = match req.target_type.as_str() {
        "article" => LikeTargetType::Article,
        "comment" => LikeTargetType::Comment,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid target type" })),
            ).into_response();
        }
    };
    
    let user_id = get_user_id_from_headers(&state, &headers).await;
    let fingerprint = if user_id.is_none() {
        extract_fingerprint(&headers)
    } else {
        None
    };
    
    if user_id.is_none() && fingerprint.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Unable to identify user" })),
        ).into_response();
    }
    
    let repo = Arc::new(CommentRepositoryImpl::new(state.pool.clone()));
    let service = CommentService::new(repo);
    
    // Check if already liked
    let is_liked = service
        .is_liked(target_type.clone(), req.target_id, user_id, fingerprint.as_deref())
        .await
        .unwrap_or(false);
    
    let result = if is_liked {
        // Unlike
        service.unlike(target_type.clone(), req.target_id, user_id, fingerprint).await
    } else {
        // Like
        service.like(target_type.clone(), req.target_id, user_id, fingerprint).await
    };
    
    // Get updated like count for articles and clear cache
    let like_count = if target_type == LikeTargetType::Article {
        // Query the article to get updated like_count
        let article_repo = crate::db::repositories::SqlxArticleRepository::new(state.pool.clone());
        use crate::db::repositories::ArticleRepository;
        let article = article_repo.get_by_id(req.target_id).await.ok().flatten();
        
        // Clear article cache so next request gets fresh data
        if let Some(ref art) = article {
            let _ = state.article_service.invalidate_article_cache(art.id, &art.slug).await;
        }
        
        article.map(|a| a.like_count).unwrap_or(0)
    } else {
        0
    };
    
    match result {
        Ok(_) => Json(LikeResponse {
            success: true,
            liked: !is_liked,
            like_count,
        }).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    }
}

/// Increment view count for an article
pub async fn increment_view(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> impl IntoResponse {
    let repo = Arc::new(CommentRepositoryImpl::new(state.pool.clone()));
    let service = CommentService::new(repo);
    
    match service.increment_view(article_id).await {
        Ok(_) => {
            // Clear article cache so next request gets fresh view count
            let article_repo = crate::db::repositories::SqlxArticleRepository::new(state.pool.clone());
            use crate::db::repositories::ArticleRepository;
            if let Ok(Some(art)) = article_repo.get_by_id(article_id).await {
                let _ = state.article_service.invalidate_article_cache(art.id, &art.slug).await;
            }
            StatusCode::OK.into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Delete a comment (admin only)
pub async fn delete_comment(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let repo = Arc::new(CommentRepositoryImpl::new(state.pool.clone()));
    let service = CommentService::new(repo);
    
    match service.delete(id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Comment not found" })),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
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
    let session_id = cookie
        .split(';')
        .find_map(|c| {
            let c = c.trim();
            if c.starts_with("session=") {
                Some(c.trim_start_matches("session="))
            } else {
                None
            }
        })?;
    
    let user = state.user_service.validate_session(session_id).await.ok()??;
    Some(user.id)
}
