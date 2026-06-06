//! Built-in public about/profile API.

use axum::{
    extract::State,
    routing::{get, put},
    Json, Router,
};
use serde::Serialize;

use crate::api::middleware::{ApiError, AppState};
use crate::models::AboutProfile;

pub fn public_router() -> Router<AppState> {
    Router::new().route("/", get(get_public_about))
}

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_admin_about))
        .route("/", put(update_admin_about))
}

#[derive(Debug, Serialize)]
struct AboutProfileResponse {
    profile: AboutProfile,
    extra_html: String,
}

async fn build_response(state: &AppState, profile: AboutProfile) -> AboutProfileResponse {
    let extra_html =
        state
            .article_service
            .render_markdown_with_shortcodes(&profile.extra_markdown, None, None);
    AboutProfileResponse {
        profile,
        extra_html,
    }
}

async fn get_public_about(
    State(state): State<AppState>,
) -> Result<Json<AboutProfileResponse>, ApiError> {
    let profile = state
        .about_service
        .get_public()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if !profile.enabled {
        return Err(ApiError::not_found("About page is disabled"));
    }

    Ok(Json(build_response(&state, profile).await))
}

async fn get_admin_about(
    State(state): State<AppState>,
) -> Result<Json<AboutProfileResponse>, ApiError> {
    let profile = state
        .about_service
        .get_admin()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(build_response(&state, profile).await))
}

async fn update_admin_about(
    State(state): State<AppState>,
    Json(input): Json<AboutProfile>,
) -> Result<Json<AboutProfileResponse>, ApiError> {
    let profile = state
        .about_service
        .update(input)
        .await
        .map_err(|e| ApiError::validation_error(e.to_string()))?;

    Ok(Json(build_response(&state, profile).await))
}
