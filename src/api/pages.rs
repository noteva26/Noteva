//! Pages API endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;

use crate::api::middleware::{ApiError, AppState};
use crate::models::{CreatePageInput, Page, UpdatePageInput};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_pages))
        .route("/", post(create_page))
        .route("/:id", get(get_page))
        .route("/:id", put(update_page))
        .route("/:id", delete(delete_page))
}

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_published_pages))
}

pub fn slug_router() -> Router<AppState> {
    Router::new()
        .route("/:slug", get(get_page_by_slug))
}

#[derive(Serialize)]
struct PagesResponse {
    pages: Vec<Page>,
}

#[derive(Serialize)]
struct PageResponse {
    page: Page,
}

async fn list_pages(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let pages = state.page_service.list().await.map_err(|e| ApiError::internal_error(e.to_string()))?;
    Ok(Json(PagesResponse { pages }))
}

async fn list_published_pages(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let pages = state.page_service.list_published().await.map_err(|e| ApiError::internal_error(e.to_string()))?;
    Ok(Json(PagesResponse { pages }))
}

async fn get_page(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let page = state.page_service.get_by_id(id).await.map_err(|e| ApiError::internal_error(e.to_string()))?;
    match page {
        Some(p) => Ok(Json(PageResponse { page: p })),
        None => Err(ApiError::not_found("Page not found")),
    }
}

async fn get_page_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let page = state.page_service.get_published_by_slug(&slug).await.map_err(|e| ApiError::internal_error(e.to_string()))?;
    match page {
        Some(p) => Ok(Json(PageResponse { page: p })),
        None => Err(ApiError::not_found("Page not found")),
    }
}

async fn create_page(
    State(state): State<AppState>,
    Json(input): Json<CreatePageInput>,
) -> Result<impl IntoResponse, ApiError> {
    let page = state.page_service
        .create(input.slug, input.title, input.content, input.status)
        .await
        .map_err(|e| ApiError::validation_error(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(PageResponse { page })))
}

async fn update_page(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdatePageInput>,
) -> Result<impl IntoResponse, ApiError> {
    let page = state.page_service
        .update(id, input.slug, input.title, input.content, input.status)
        .await
        .map_err(|e| ApiError::validation_error(e.to_string()))?;
    Ok(Json(PageResponse { page }))
}

async fn delete_page(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    state.page_service.delete(id).await.map_err(|e| ApiError::internal_error(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
