//! Category and tag management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};
use serde_json::json;

/// Request for creating/updating a category
#[derive(Debug, Deserialize)]
pub struct CategoryRequest {
    pub name: String,
    #[serde(default)]
    pub slug: String,
    pub description: Option<String>,
    #[allow(dead_code)]
    pub parent_id: Option<i64>,
}

/// Response for a category
#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i64>,
    pub created_at: String,
}

impl From<crate::models::Category> for CategoryResponse {
    fn from(cat: crate::models::Category) -> Self {
        Self {
            id: cat.id,
            slug: cat.slug,
            name: cat.name,
            description: cat.description,
            parent_id: cat.parent_id,
            created_at: cat.created_at.to_rfc3339(),
        }
    }
}

/// Request for creating/updating a tag
#[derive(Debug, Deserialize)]
pub struct TagRequest {
    pub name: String,
}

/// Response for a tag
#[derive(Debug, Serialize)]
pub struct TagResponse {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub created_at: String,
}

impl From<crate::models::Tag> for TagResponse {
    fn from(tag: crate::models::Tag) -> Self {
        Self {
            id: tag.id,
            slug: tag.slug,
            name: tag.name,
            created_at: tag.created_at.to_rfc3339(),
        }
    }
}

/// POST /api/v1/admin/categories - Create category
///
/// Requires admin authentication.
/// Satisfies requirement 5.2: Content management
pub async fn create_category(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<CategoryRequest>,
) -> Result<(StatusCode, Json<CategoryResponse>), ApiError> {
    let input = crate::services::category::CreateCategoryInput::new(&body.name)
        .with_description(body.description.unwrap_or_default());

    let input = if !body.slug.is_empty() {
        input.with_slug(&body.slug)
    } else {
        input
    };

    let category = state
        .category_service
        .create(input)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Hook: category_after_create
    state.hook_manager.trigger(
        "category_after_create",
        json!({ "id": category.id, "name": category.name, "slug": category.slug }),
    );

    Ok((StatusCode::CREATED, Json(category.into())))
}

/// PUT /api/v1/admin/categories/:id - Update category
///
/// Requires admin authentication.
pub async fn update_category(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
    Json(body): Json<CategoryRequest>,
) -> Result<Json<CategoryResponse>, ApiError> {
    let mut input = crate::services::category::UpdateCategoryInput::new().with_name(&body.name);

    if !body.slug.is_empty() {
        input = input.with_slug(&body.slug);
    }

    if let Some(desc) = body.description {
        input = input.with_description(Some(desc));
    }

    let category = state
        .category_service
        .update(id, input)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(category.into()))
}

/// DELETE /api/v1/admin/categories/:id - Delete category
///
/// Requires admin authentication.
pub async fn delete_category(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state
        .category_service
        .delete(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Hook: category_after_delete
    state
        .hook_manager
        .trigger("category_after_delete", json!({ "id": id }));

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/admin/tags - Create tag
///
/// Requires admin authentication.
pub async fn create_tag(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<TagRequest>,
) -> Result<(StatusCode, Json<TagResponse>), ApiError> {
    let tag = state
        .tag_service
        .create_or_get(&body.name)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Hook: tag_after_create
    state.hook_manager.trigger(
        "tag_after_create",
        json!({ "id": tag.id, "name": tag.name, "slug": tag.slug }),
    );

    Ok((StatusCode::CREATED, Json(tag.into())))
}

/// DELETE /api/v1/admin/tags/:id - Delete tag
///
/// Requires admin authentication.
pub async fn delete_tag(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state
        .tag_service
        .delete(id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Hook: tag_after_delete
    state
        .hook_manager
        .trigger("tag_after_delete", json!({ "id": id }));

    Ok(StatusCode::NO_CONTENT)
}
