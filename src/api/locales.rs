//! Custom locale API endpoints
//!
//! Public: GET /api/v1/locales — list available custom locales
//! Public: GET /api/v1/locales/:code — get locale JSON content
//! Admin:  POST /api/v1/admin/locales — upsert a custom locale
//! Admin:  DELETE /api/v1/admin/locales/:code — delete a custom locale

use axum::{
    extract::Path,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::services::locale;

// ── Response types ──────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct LocaleListResponse {
    pub locales: Vec<LocaleItem>,
}

#[derive(Serialize)]
pub struct LocaleItem {
    pub code: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct LocaleDetailResponse {
    pub code: String,
    pub name: String,
    pub translations: serde_json::Value,
}

#[derive(Serialize)]
pub struct LocaleDeleteResponse {
    pub deleted: bool,
}

// ── Public endpoints ────────────────────────────────────────────────────

/// GET /api/v1/locales — list all custom locales (code + name)
pub async fn list_locales() -> Result<Json<LocaleListResponse>, super::ApiError> {
    let items = locale::list_locales().await
        .map_err(|e| super::ApiError::internal_error(format!("Failed to list locales: {e}")))?;

    Ok(Json(LocaleListResponse {
        locales: items.into_iter().map(|i| LocaleItem { code: i.code, name: i.name }).collect(),
    }))
}

/// GET /api/v1/locales/:code — get full locale JSON by code
pub async fn get_locale(
    Path(code): Path<String>,
) -> Result<Json<LocaleDetailResponse>, super::ApiError> {
    let loc = locale::get_locale(&code).await
        .map_err(|e| super::ApiError::internal_error(format!("Failed to get locale: {e}")))?;

    match loc {
        Some(loc) => {
            let json_value: serde_json::Value = serde_json::from_str(&loc.json_content)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            Ok(Json(LocaleDetailResponse {
                code: loc.code,
                name: loc.name,
                translations: json_value,
            }))
        }
        None => Err(super::ApiError::not_found("Locale not found")),
    }
}

// ── Admin endpoints ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpsertLocaleInput {
    pub code: String,
    pub name: String,
    pub json_content: serde_json::Value,
}

/// POST /api/v1/admin/locales — create or update a custom locale
pub async fn upsert_locale(
    Json(input): Json<UpsertLocaleInput>,
) -> Result<Json<LocaleItem>, super::ApiError> {
    if input.code.is_empty() || input.code.len() > 20 {
        return Err(super::ApiError::validation_error("Invalid locale code"));
    }
    if input.name.is_empty() || input.name.len() > 100 {
        return Err(super::ApiError::validation_error("Invalid locale name"));
    }
    if !input.json_content.is_object() {
        return Err(super::ApiError::validation_error("json_content must be a JSON object"));
    }

    let json_string = serde_json::to_string(&input.json_content)
        .map_err(|e| super::ApiError::internal_error(format!("Failed to serialize JSON: {e}")))?;

    locale::upsert_locale(&input.code, &input.name, &json_string).await
        .map_err(|e| super::ApiError::internal_error(format!("Failed to save locale: {e}")))?;

    Ok(Json(LocaleItem {
        code: input.code,
        name: input.name,
    }))
}

/// DELETE /api/v1/admin/locales/:code — delete a custom locale
pub async fn delete_locale(
    Path(code): Path<String>,
) -> Result<Json<LocaleDeleteResponse>, super::ApiError> {
    let deleted = locale::delete_locale(&code).await
        .map_err(|e| super::ApiError::internal_error(format!("Failed to delete locale: {e}")))?;

    if deleted {
        Ok(Json(LocaleDeleteResponse { deleted: true }))
    } else {
        Err(super::ApiError::not_found("Locale not found"))
    }
}
