//! Upload API endpoints
//!
//! Handles file uploads for:
//! - Images (for articles)
//!
//! Satisfies requirements:
//! - 1.1: Article creation with images

use axum::{
    extract::{Multipart, State},
    routing::post,
    Json, Router,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};
use crate::config::UploadConfig;

/// Response for successful upload
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub url: String,
    pub filename: String,
    pub size: u64,
    pub content_type: String,
}

/// Response for multiple uploads
#[derive(Debug, Serialize)]
pub struct MultiUploadResponse {
    pub files: Vec<UploadResponse>,
    pub failed: Vec<String>,
}

/// Upload state containing configuration
#[derive(Clone)]
pub struct UploadState {
    pub config: Arc<UploadConfig>,
}

/// Build the upload router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/image", post(upload_image))
        .route("/images", post(upload_images))
        .route("/plugin/:plugin_id/file", post(upload_plugin_file))
}

/// POST /api/v1/upload/image - Upload a single image
///
/// Requires authentication.
/// Accepts multipart/form-data with a single file field named "file".
async fn upload_image(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, ApiError> {
    let config = &state.upload_config;
    
    // Ensure upload directory exists
    ensure_upload_dir(&config.path).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to read multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name != "file" {
            continue;
        }

        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        // Validate content type
        if !config.is_type_allowed(&content_type) {
            return Err(ApiError::validation_error(format!(
                "Invalid file type: {}. Allowed types: {:?}",
                content_type, config.allowed_types
            )));
        }

        // Read file data
        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to read file: {}", e)))?;

        // Validate file size
        if data.len() as u64 > config.max_file_size {
            return Err(ApiError::validation_error(format!(
                "File too large. Maximum size: {} bytes ({} MB)",
                config.max_file_size,
                config.max_file_size / 1024 / 1024
            )));
        }

        // Generate unique filename
        let ext = get_extension(&filename, &content_type);
        let new_filename = format!("{}.{}", Uuid::new_v4(), ext);
        let file_path = config.path.join(&new_filename);

        // Save file
        fs::write(&file_path, &data)
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to save file: {}", e)))?;

        return Ok(Json(UploadResponse {
            url: format!("/uploads/{}", new_filename),
            filename: new_filename,
            size: data.len() as u64,
            content_type,
        }));
    }

    Err(ApiError::validation_error("No file provided"))
}

/// POST /api/v1/upload/images - Upload multiple images
///
/// Requires authentication.
/// Accepts multipart/form-data with multiple file fields named "files".
async fn upload_images(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<Json<MultiUploadResponse>, ApiError> {
    let config = &state.upload_config;
    
    // Ensure upload directory exists
    ensure_upload_dir(&config.path).await?;

    let mut files = Vec::new();
    let mut failed = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to read multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name != "files" && name != "file" {
            continue;
        }

        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        // Validate content type
        if !config.is_type_allowed(&content_type) {
            failed.push(format!("{}: invalid type {}", filename, content_type));
            continue;
        }

        // Read file data
        let data = match field.bytes().await {
            Ok(d) => d,
            Err(e) => {
                failed.push(format!("{}: {}", filename, e));
                continue;
            }
        };

        // Validate file size
        if data.len() as u64 > config.max_file_size {
            failed.push(format!("{}: file too large (max {} MB)", filename, config.max_file_size / 1024 / 1024));
            continue;
        }

        // Generate unique filename
        let ext = get_extension(&filename, &content_type);
        let new_filename = format!("{}.{}", Uuid::new_v4(), ext);
        let file_path = config.path.join(&new_filename);

        // Save file
        match fs::write(&file_path, &data).await {
            Ok(_) => {
                files.push(UploadResponse {
                    url: format!("/uploads/{}", new_filename),
                    filename: new_filename,
                    size: data.len() as u64,
                    content_type,
                });
            }
            Err(e) => {
                failed.push(format!("{}: {}", filename, e));
            }
        }
    }

    Ok(Json(MultiUploadResponse { files, failed }))
}

/// Ensure upload directory exists
async fn ensure_upload_dir(path: &PathBuf) -> Result<(), ApiError> {
    if !path.exists() {
        fs::create_dir_all(path)
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to create upload dir: {}", e)))?;
    }
    Ok(())
}

/// Get file extension from filename or content type
fn get_extension(filename: &str, content_type: &str) -> String {
    // Try to get from filename first
    if let Some(ext) = filename.rsplit('.').next() {
        if !ext.is_empty() && ext.len() < 10 {
            return ext.to_lowercase();
        }
    }

    // Fall back to content type
    match content_type {
        "image/jpeg" => "jpg".to_string(),
        "image/png" => "png".to_string(),
        "image/gif" => "gif".to_string(),
        "image/webp" => "webp".to_string(),
        "image/svg+xml" => "svg".to_string(),
        "image/bmp" => "bmp".to_string(),
        "image/tiff" => "tiff".to_string(),
        "image/x-icon" => "ico".to_string(),
        _ => "bin".to_string(),
    }
}

/// POST /api/v1/upload/plugin/:plugin_id/file - Upload file for plugin
///
/// Uploads a file to the plugin's dedicated directory.
/// Files are stored in uploads/plugins/{plugin_id}/
async fn upload_plugin_file(
    State(state): State<AppState>,
    axum::extract::Path(plugin_id): axum::extract::Path<String>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, ApiError> {
    let config = &state.upload_config;
    
    // Create plugin-specific upload directory
    let plugin_upload_dir = config.path.join("plugins").join(&plugin_id);
    ensure_upload_dir(&plugin_upload_dir).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to read multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name != "file" {
            continue;
        }

        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        // Read file data
        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to read file: {}", e)))?;

        // Validate file size (use plugin file size limit from config)
        if data.len() as u64 > config.max_plugin_file_size {
            return Err(ApiError::validation_error(format!(
                "File too large. Maximum size: {} MB",
                config.max_plugin_file_size / 1024 / 1024
            )));
        }

        // Generate unique filename
        let ext = get_extension(&filename, &content_type);
        let new_filename = format!("{}.{}", Uuid::new_v4(), ext);
        let file_path = plugin_upload_dir.join(&new_filename);

        // Save file
        fs::write(&file_path, &data)
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to save file: {}", e)))?;

        return Ok(Json(UploadResponse {
            url: format!("/uploads/plugins/{}/{}", plugin_id, new_filename),
            filename: new_filename,
            size: data.len() as u64,
            content_type,
        }));
    }

    Err(ApiError::validation_error("No file provided"))
}
