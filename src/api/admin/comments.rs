//! Comment management endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::common::{default_page_i64, default_per_page};
use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};

/// Query params for comments list
#[derive(Debug, Deserialize)]
pub struct CommentsQuery {
    #[serde(default = "default_page_i64")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    /// Optional status filter: "pending", "approved", "spam", or empty for all
    pub status: Option<String>,
}

/// Response for comments list
#[derive(Debug, Serialize)]
pub struct AdminCommentsResponse {
    pub comments: Vec<AdminCommentResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

/// Response for a single comment
#[derive(Debug, Serialize)]
pub struct AdminCommentResponse {
    pub id: i64,
    pub article_id: i64,
    pub content: String,
    pub status: String,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

/// GET /api/v1/admin/comments - List all comments with optional status filter
pub async fn list_comments(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(query): Query<CommentsQuery>,
) -> Result<Json<AdminCommentsResponse>, ApiError> {
    let status_filter = query.status.as_deref().filter(|s| !s.is_empty());
    let page = query.page.max(1);
    let per_page = query.per_page.clamp(1, 100);

    let (comments, total) = state
        .comment_service
        .list_all(status_filter, page, per_page)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    let comments: Vec<AdminCommentResponse> = comments
        .into_iter()
        .map(|c| AdminCommentResponse {
            id: c.id,
            article_id: c.article_id,
            content: c.content,
            status: c.status.to_string(),
            nickname: c.nickname,
            email: c.email.clone(),
            avatar_url: Some(c.avatar_url),
            created_at: c.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(AdminCommentsResponse {
        comments,
        total,
        page,
        per_page,
        total_pages,
    }))
}

/// GET /api/v1/admin/comments/pending - List pending comments (legacy)
pub async fn list_pending_comments(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(query): Query<CommentsQuery>,
) -> Result<Json<AdminCommentsResponse>, ApiError> {
    let page = query.page.max(1);
    let per_page = query.per_page.clamp(1, 100);

    let (comments, total) = state
        .comment_service
        .list_pending(page, per_page)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    let comments: Vec<AdminCommentResponse> = comments
        .into_iter()
        .map(|c| AdminCommentResponse {
            id: c.id,
            article_id: c.article_id,
            content: c.content,
            status: c.status.to_string(),
            nickname: c.nickname,
            email: c.email.clone(),
            avatar_url: Some(c.avatar_url),
            created_at: c.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(AdminCommentsResponse {
        comments,
        total,
        page,
        per_page,
        total_pages,
    }))
}

/// POST /api/v1/admin/comments/:id/approve - Approve a comment
pub async fn approve_comment(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let success = state
        .comment_service
        .approve(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if success {
        Ok(StatusCode::OK)
    } else {
        Err(ApiError::not_found("Comment not found"))
    }
}

/// POST /api/v1/admin/comments/:id/reject - Reject a comment
pub async fn reject_comment(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let success = state
        .comment_service
        .reject(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if success {
        Ok(StatusCode::OK)
    } else {
        Err(ApiError::not_found("Comment not found"))
    }
}
