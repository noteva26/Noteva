//! Navigation API endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;

use crate::api::middleware::{ApiError, AppState};
use crate::models::{CreateNavItemInput, NavItem, NavItemTree, UpdateNavItemInput, UpdateNavOrderInput};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_nav_items))
        .route("/", post(create_nav_item))
        .route("/tree", get(list_nav_tree))
        .route("/order", put(update_nav_order))
        .route("/:id", get(get_nav_item))
        .route("/:id", put(update_nav_item))
        .route("/:id", delete(delete_nav_item))
}

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_visible_nav_tree))
}

#[derive(Serialize)]
struct NavItemsResponse {
    items: Vec<NavItem>,
}

#[derive(Serialize)]
struct NavTreeResponse {
    items: Vec<NavItemTree>,
}

#[derive(Serialize)]
struct NavItemResponse {
    item: NavItem,
}

async fn list_nav_items(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let items = state.nav_service.list().await.map_err(|e| ApiError::internal_error(e.to_string()))?;
    Ok(Json(NavItemsResponse { items }))
}

async fn list_nav_tree(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let items = state.nav_service.list_tree().await.map_err(|e| ApiError::internal_error(e.to_string()))?;
    Ok(Json(NavTreeResponse { items }))
}

async fn list_visible_nav_tree(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let items = state.nav_service.list_visible_tree().await.map_err(|e| ApiError::internal_error(e.to_string()))?;
    Ok(Json(NavTreeResponse { items }))
}

async fn get_nav_item(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let item = state.nav_service.get_by_id(id).await.map_err(|e| ApiError::internal_error(e.to_string()))?;
    match item {
        Some(i) => Ok(Json(NavItemResponse { item: i })),
        None => Err(ApiError::not_found("Nav item not found")),
    }
}

async fn create_nav_item(
    State(state): State<AppState>,
    Json(input): Json<CreateNavItemInput>,
) -> Result<impl IntoResponse, ApiError> {
    let item = state.nav_service
        .create(input.parent_id, input.title, input.nav_type, input.target, input.open_new_tab, input.sort_order, input.visible)
        .await
        .map_err(|e| ApiError::validation_error(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(NavItemResponse { item })))
}

async fn update_nav_item(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateNavItemInput>,
) -> Result<impl IntoResponse, ApiError> {
    let item = state.nav_service
        .update(id, input.parent_id, input.title, input.nav_type, input.target, input.open_new_tab, input.sort_order, input.visible)
        .await
        .map_err(|e| ApiError::validation_error(e.to_string()))?;
    Ok(Json(NavItemResponse { item }))
}

async fn update_nav_order(
    State(state): State<AppState>,
    Json(input): Json<UpdateNavOrderInput>,
) -> Result<impl IntoResponse, ApiError> {
    state.nav_service.update_order(input.items).await.map_err(|e| ApiError::internal_error(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_nav_item(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    state.nav_service.delete(id).await.map_err(|e| ApiError::internal_error(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
