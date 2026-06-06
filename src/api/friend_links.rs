//! Friend links API endpoints.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;

use crate::api::middleware::{ApiError, AppState};
use crate::models::{
    CreateFriendLinkInput, FriendLink, UpdateFriendLinkInput, UpdateFriendLinkOrderInput,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_friend_links))
        .route("/", post(create_friend_link))
        .route(
            "/import/friendlinks-plugin",
            post(import_friendlinks_plugin),
        )
        .route("/order", put(update_friend_link_order))
        .route("/{id}", get(get_friend_link))
        .route("/{id}", put(update_friend_link))
        .route("/{id}", delete(delete_friend_link))
}

pub fn public_router() -> Router<AppState> {
    Router::new().route("/", get(list_public_friend_links))
}

#[derive(Debug, Serialize)]
struct FriendLinksResponse {
    links: Vec<FriendLink>,
}

#[derive(Debug, Serialize)]
struct FriendLinkResponse {
    link: FriendLink,
}

#[derive(Debug, Serialize)]
struct ImportFriendLinksResponse {
    imported: usize,
    skipped: usize,
    links: Vec<FriendLink>,
}

async fn list_public_friend_links(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let links = state
        .friend_link_service
        .list_public()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    Ok(Json(FriendLinksResponse { links }))
}

async fn list_friend_links(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let links = state
        .friend_link_service
        .list()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    Ok(Json(FriendLinksResponse { links }))
}

async fn get_friend_link(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let link = state
        .friend_link_service
        .get_by_id(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    match link {
        Some(link) => Ok(Json(FriendLinkResponse { link })),
        None => Err(ApiError::not_found("Friend link not found")),
    }
}

async fn create_friend_link(
    State(state): State<AppState>,
    Json(input): Json<CreateFriendLinkInput>,
) -> Result<impl IntoResponse, ApiError> {
    let link = state
        .friend_link_service
        .create(input)
        .await
        .map_err(|e| ApiError::validation_error(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(FriendLinkResponse { link })))
}

async fn import_friendlinks_plugin(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let legacy_links = {
        let manager = state.plugin_manager.read().await;
        let plugin = manager
            .get("friendlinks")
            .ok_or_else(|| ApiError::not_found("Legacy friendlinks plugin not found"))?;

        plugin
            .settings
            .get("links_data")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default()
    };

    let existing_urls = state
        .friend_link_service
        .list()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .into_iter()
        .map(|link| link.url)
        .collect::<std::collections::HashSet<_>>();

    let mut seen_urls = existing_urls;
    let mut imported_links = Vec::new();
    let mut skipped = 0usize;

    for (index, item) in legacy_links.into_iter().enumerate() {
        let Some(obj) = item.as_object() else {
            skipped += 1;
            continue;
        };

        let name = obj
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .trim();
        let url = obj
            .get("url")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .trim();

        if name.is_empty() || url.is_empty() || seen_urls.contains(url) {
            skipped += 1;
            continue;
        }

        let input = CreateFriendLinkInput {
            name: name.to_string(),
            url: url.to_string(),
            logo: optional_string(obj.get("logo")),
            description: optional_string(obj.get("description")),
            category: optional_string(obj.get("category")),
            sort_order: Some(index as i32),
            status: Some("approved".to_string()),
            is_recommended: false,
        };

        match state.friend_link_service.create(input).await {
            Ok(link) => {
                seen_urls.insert(link.url.clone());
                imported_links.push(link);
            }
            Err(_) => {
                skipped += 1;
            }
        }
    }

    Ok(Json(ImportFriendLinksResponse {
        imported: imported_links.len(),
        skipped,
        links: imported_links,
    }))
}

async fn update_friend_link(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateFriendLinkInput>,
) -> Result<impl IntoResponse, ApiError> {
    let link = state
        .friend_link_service
        .update(id, input)
        .await
        .map_err(|e| ApiError::validation_error(e.to_string()))?;
    Ok(Json(FriendLinkResponse { link }))
}

async fn update_friend_link_order(
    State(state): State<AppState>,
    Json(input): Json<UpdateFriendLinkOrderInput>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .friend_link_service
        .update_order(input.items)
        .await
        .map_err(|e| ApiError::validation_error(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

fn optional_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

async fn delete_friend_link(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .friend_link_service
        .delete(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
