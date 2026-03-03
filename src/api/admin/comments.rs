//! Comment moderation endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::common::{default_page_i64, default_per_page};
use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};

/// Query params for pending comments list
#[derive(Debug, Deserialize)]
pub struct PendingCommentsQuery {
    #[serde(default = "default_page_i64")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

/// Response for pending comments list
#[derive(Debug, Serialize)]
pub struct PendingCommentsResponse {
    pub comments: Vec<PendingCommentResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

/// Response for a pending comment
#[derive(Debug, Serialize)]
pub struct PendingCommentResponse {
    pub id: i64,
    pub article_id: i64,
    pub content: String,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

/// GET /api/v1/admin/comments/pending - List pending comments
pub async fn list_pending_comments(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(query): Query<PendingCommentsQuery>,
) -> Result<Json<PendingCommentsResponse>, ApiError> {
    let (comments, total) = state
        .comment_service
        .list_pending(query.page, query.per_page)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let total_pages = (total as f64 / query.per_page as f64).ceil() as i64;

    let comments: Vec<PendingCommentResponse> = comments
        .into_iter()
        .map(|c| PendingCommentResponse {
            id: c.id,
            article_id: c.article_id,
            content: c.content,
            nickname: c.nickname,
            email: c.email.clone(),
            avatar_url: Some(c.avatar_url),
            created_at: c.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(PendingCommentsResponse {
        comments,
        total,
        page: query.page,
        per_page: query.per_page,
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
