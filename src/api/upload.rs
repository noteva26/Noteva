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
use crate::plugin::hook_names;
use chrono::Utc;

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
        .route("/file", post(upload_file))
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

        // Trigger image_upload_filter hook — plugins can intercept and upload to S3/COS
        if state.hook_manager.has_handlers(hook_names::IMAGE_UPLOAD_FILTER) {
            let ext = get_extension(&filename, &content_type);
            let new_filename = format!("{}.{}", Uuid::new_v4(), ext);
            let hook_data = serde_json::json!({
                "filename": new_filename,
                "original_filename": filename,
                "content_type": content_type,
                "size": data.len(),
                "data_base64": simple_base64_encode(&data),
                "timestamp": Utc::now().to_rfc3339(),
            });
            let result = state.hook_manager.trigger(hook_names::IMAGE_UPLOAD_FILTER, hook_data);
            if let Some(true) = result.get("handled").and_then(|v| v.as_bool()) {
                if let Some(url) = result.get("url").and_then(|v| v.as_str()) {
                    return Ok(Json(UploadResponse {
                        url: url.to_string(),
                        filename: new_filename,
                        size: data.len() as u64,
                        content_type,
                    }));
                }
            }
        }

        // Default: save locally
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
    let has_upload_hook = state.hook_manager.has_handlers(hook_names::IMAGE_UPLOAD_FILTER);

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

        let ext = get_extension(&filename, &content_type);
        let new_filename = format!("{}.{}", Uuid::new_v4(), ext);

        // Try plugin upload hook first
        if has_upload_hook {
            let hook_data = serde_json::json!({
                "filename": new_filename,
                "original_filename": filename,
                "content_type": content_type,
                "size": data.len(),
                "data_base64": simple_base64_encode(&data),
                "timestamp": Utc::now().to_rfc3339(),
            });
            let result = state.hook_manager.trigger(hook_names::IMAGE_UPLOAD_FILTER, hook_data);
            if let Some(true) = result.get("handled").and_then(|v| v.as_bool()) {
                if let Some(url) = result.get("url").and_then(|v| v.as_str()) {
                    files.push(UploadResponse {
                        url: url.to_string(),
                        filename: new_filename,
                        size: data.len() as u64,
                        content_type,
                    });
                    continue;
                }
            }
        }

        // Default: save locally
        let file_path = config.path.join(&new_filename);
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

/// POST /api/v1/upload/file - Upload a generic file (any type)
///
/// Requires authentication. No MIME type restriction — only size limit applies.
/// Files are saved locally (no S3 hook). Returns URL for shortcode embedding.
async fn upload_file(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, ApiError> {
    let config = &state.upload_config;
    ensure_upload_dir(&config.path).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to read multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name != "file" { continue; }

        let filename = field.file_name().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string());
        let content_type = field.content_type().map(|s| s.to_string()).unwrap_or_else(|| "application/octet-stream".to_string());

        let data = field.bytes().await
            .map_err(|e| ApiError::internal_error(format!("Failed to read file: {}", e)))?;

        if data.len() as u64 > config.max_file_size {
            return Err(ApiError::validation_error(format!(
                "File too large. Maximum size: {} MB",
                config.max_file_size / 1024 / 1024
            )));
        }

        let ext = get_extension(&filename, &content_type);
        let new_filename = format!("{}.{}", Uuid::new_v4(), ext);
        let file_path = config.path.join(&new_filename);

        fs::write(&file_path, &data).await
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

/// Simple base64 encoder (no external dependency)
fn simple_base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}
