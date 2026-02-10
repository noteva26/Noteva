//! Cache API endpoints
//!
//! Provides endpoints for cache operations (for plugins and themes).
//! Uses plugin_data table as backend with a special plugin_id prefix.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

use crate::api::{AppState, ApiError};
use crate::db::repositories::{PluginDataRepository, SqlxPluginDataRepository};

const CACHE_PLUGIN_ID: &str = "_cache";

/// Cache value response
#[derive(Debug, Serialize)]
pub struct CacheValueResponse {
    pub value: String,
}

/// Cache set request
#[derive(Debug, Deserialize)]
pub struct CacheSetRequest {
    pub value: String,
    /// TTL in seconds (optional, not enforced but can be used by cleanup jobs)
    #[serde(default)]
    pub ttl: Option<u64>,
}

/// GET /api/v1/cache/:key - Get cached value
pub async fn get_cache(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<CacheValueResponse>, ApiError> {
    let repo = SqlxPluginDataRepository::new(state.pool.clone());
    
    let value = repo.get(CACHE_PLUGIN_ID, &key)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to get cache: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Cache key not found"))?;
    
    Ok(Json(CacheValueResponse { value }))
}

/// PUT /api/v1/cache/:key - Set cached value
pub async fn set_cache(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(body): Json<CacheSetRequest>,
) -> Result<StatusCode, ApiError> {
    let repo = SqlxPluginDataRepository::new(state.pool.clone());
    
    repo.set(CACHE_PLUGIN_ID, &key, &body.value)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to set cache: {}", e)))?;
    
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/cache/:key - Delete cached value
pub async fn delete_cache(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<StatusCode, ApiError> {
    let repo = SqlxPluginDataRepository::new(state.pool.clone());
    
    repo.delete(CACHE_PLUGIN_ID, &key)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to delete cache: {}", e)))?;
    
    Ok(StatusCode::NO_CONTENT)
}
