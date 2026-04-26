//! Admin backup & restore API endpoints

use axum::{
    extract::{Multipart, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::api::middleware::AppState;
use crate::services::backup;

/// GET /api/v1/admin/backup — download full backup as ZIP
pub async fn download_backup(State(state): State<AppState>) -> impl IntoResponse {
    let upload_dir = state.upload_config.path.clone();
    match backup::create_backup(&state.pool, &upload_dir).await {
        Ok(zip_bytes) => {
            let filename = format!(
                "noteva-backup-{}.zip",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            );
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/zip".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                zip_bytes,
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "backup failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": { "message": format!("Backup failed: {}", e) } })),
            )
                .into_response()
        }
    }
}

/// POST /api/v1/admin/backup/restore — upload ZIP to restore
pub async fn restore_backup_endpoint(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    tracing::info!("restore: reading multipart upload...");

    // Read the uploaded file
    let mut zip_data: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            tracing::info!("restore: found file field, reading bytes...");
            match field.bytes().await {
                Ok(bytes) => {
                    tracing::info!(size = bytes.len(), "restore: file read successfully");
                    zip_data = Some(bytes.to_vec());
                }
                Err(e) => {
                    tracing::error!(error = %e, "restore: failed to read file");
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": { "message": format!("Failed to read file: {}", e) } })),
                    ).into_response();
                }
            }
        }
    }

    let zip_data = match zip_data {
        Some(d) => d,
        None => {
            tracing::error!("restore: no file uploaded");
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": { "message": "No file uploaded" } })),
            )
                .into_response();
        }
    };

    tracing::info!(
        size = zip_data.len(),
        "restore: starting restore process..."
    );
    let upload_dir = state.upload_config.path.clone();
    match backup::restore_backup(&state.pool, &upload_dir, &zip_data).await {
        Ok(manifest) => {
            tracing::info!("restore: completed successfully!");
            (
                StatusCode::OK,
                Json(json!({
                    "status": "ok",
                    "message": "Backup restored successfully",
                    "manifest": {
                        "version": manifest.version,
                        "created_at": manifest.created_at,
                        "db_driver": manifest.db_driver,
                        "tables": manifest.tables,
                    }
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "restore failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": { "message": format!("Restore failed: {}", e) } })),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/admin/backup/export-markdown — download all articles as Markdown ZIP
pub async fn export_markdown_endpoint(State(state): State<AppState>) -> impl IntoResponse {
    match backup::export_markdown(&state.pool).await {
        Ok(zip_bytes) => {
            let filename = format!(
                "noteva-articles-{}.zip",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            );
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/zip".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                zip_bytes,
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "markdown export failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": { "message": format!("Export failed: {}", e) } })),
            )
                .into_response()
        }
    }
}

/// POST /api/v1/admin/backup/import — import articles from Markdown ZIP or WordPress XML
pub async fn import_articles_endpoint(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_data: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            match field.bytes().await {
                Ok(bytes) => file_data = Some(bytes.to_vec()),
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": { "message": format!("Failed to read file: {}", e) } })),
                    ).into_response();
                }
            }
        }
    }

    let file_data = match file_data {
        Some(d) => d,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": { "message": "No file uploaded" } })),
            )
                .into_response();
        }
    };

    // Use author_id = 1 (admin) for imported articles
    match backup::import_articles(&state.pool, &file_data, 1).await {
        Ok(result) => {
            tracing::info!(
                imported = result.imported,
                skipped = result.skipped,
                "article import completed"
            );
            (
                StatusCode::OK,
                Json(json!({
                    "status": "ok",
                    "imported": result.imported,
                    "skipped": result.skipped,
                    "errors": result.errors,
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "article import failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": { "message": format!("Import failed: {}", e) } })),
            )
                .into_response()
        }
    }
}
