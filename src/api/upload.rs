//! Upload API endpoints
//!
//! Handles file uploads for:
//! - Images (for articles) — supports plugin presign delegation
//! - Generic files — supports plugin presign delegation
//!
//! Presign flow: WASM plugin returns {"handled": true, "presign": {"url": "...", "method": "PUT", "headers": {...}}}
//! and the main process uploads the file natively (no base64, no WASM memory limit).

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
        .route("/plugin/{plugin_id}/file", post(upload_plugin_file))
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

        // Trigger image_upload_filter hook — plugins can intercept via presign or direct upload
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
            if let Some(url) = try_plugin_upload(&result, &data, &content_type, &new_filename).await {
                return Ok(Json(UploadResponse {
                    url,
                    filename: new_filename,
                    size: data.len() as u64,
                    content_type,
                }));
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

        // Try plugin upload hook first (presign or direct)
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
            if let Some(url) = try_plugin_upload(&result, &data, &content_type, &new_filename).await {
                files.push(UploadResponse {
                    url,
                    filename: new_filename,
                    size: data.len() as u64,
                    content_type,
                });
                continue;
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
/// Supports plugin presign delegation via file_upload_filter hook (no base64).
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

        // Try file_upload_filter hook — presign only, no data_base64
        if state.hook_manager.has_handlers(hook_names::FILE_UPLOAD_FILTER) {
            let hook_data = serde_json::json!({
                "filename": new_filename,
                "original_filename": filename,
                "content_type": content_type,
                "size": data.len(),
                "timestamp": Utc::now().to_rfc3339(),
            });
            let result = state.hook_manager.trigger(hook_names::FILE_UPLOAD_FILTER, hook_data);
            if let Some(url) = try_plugin_upload(&result, &data, &content_type, &new_filename).await {
                return Ok(Json(UploadResponse {
                    url,
                    filename: new_filename,
                    size: data.len() as u64,
                    content_type,
                }));
            }
        }

        // Default: save locally
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

/// Execute a presign-delegated upload: the plugin returned a presign URL + headers,
/// and we upload the file data natively using reqwest (streaming, no base64).
///
/// Returns the public URL on success, or an error string on failure.
async fn execute_presign_upload(
    presign: &serde_json::Value,
    data: &[u8],
    content_type: &str,
) -> Result<String, String> {
    let url = presign.get("url").and_then(|v| v.as_str())
        .ok_or_else(|| "presign missing 'url' field".to_string())?;
    let method = presign.get("method").and_then(|v| v.as_str()).unwrap_or("PUT");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut request_builder = match method.to_uppercase().as_str() {
        "PUT" => client.put(url),
        "POST" => client.post(url),
        _ => return Err(format!("Unsupported presign method: {}", method)),
    };

    // Apply headers from presign response
    if let Some(headers) = presign.get("headers").and_then(|v| v.as_object()) {
        for (key, value) in headers {
            if let Some(val_str) = value.as_str() {
                request_builder = request_builder.header(key.as_str(), val_str);
            }
        }
    }

    // Set content-type if not already in presign headers
    let has_content_type = presign.get("headers")
        .and_then(|v| v.as_object())
        .map(|h| h.keys().any(|k| k.to_lowercase() == "content-type"))
        .unwrap_or(false);
    if !has_content_type {
        request_builder = request_builder.header("Content-Type", content_type);
    }

    let response = request_builder
        .body(data.to_vec())
        .send()
        .await
        .map_err(|e| format!("Presign upload request failed: {}", e))?;

    let status = response.status().as_u16();
    if status >= 200 && status < 300 {
        // Use the public_url from presign if provided, otherwise use the presign URL itself
        let public_url = presign.get("public_url")
            .and_then(|v| v.as_str())
            .unwrap_or(url)
            .to_string();
        tracing::info!("Presign upload success (HTTP {}): {}", status, public_url);
        Ok(public_url)
    } else {
        let body = response.text().await.unwrap_or_default();
        let preview_len = body.len().min(200);
        Err(format!("Presign upload failed (HTTP {}): {}", status, &body[..preview_len]))
    }
}

/// Try to handle an upload via plugin hook (image_upload_filter or file_upload_filter).
///
/// Supports three response modes:
/// 1. Legacy direct: `{"handled": true, "url": "https://..."}` — plugin already uploaded
/// 2. Presign delegation: `{"handled": true, "presign": {...}}` — main process uploads
/// 3. Not handled: `{"handled": false}` — fall back to local storage
///
/// Returns `Some(url)` if plugin handled the upload, `None` to fall back to local.
async fn try_plugin_upload(
    hook_result: &serde_json::Value,
    data: &[u8],
    content_type: &str,
    filename: &str,
) -> Option<String> {
    if hook_result.get("handled").and_then(|v| v.as_bool()) != Some(true) {
        return None;
    }

    // Mode 1: presign delegation (preferred for large files)
    if let Some(presign) = hook_result.get("presign") {
        if presign.is_object() {
            match execute_presign_upload(presign, data, content_type).await {
                Ok(url) => return Some(url),
                Err(e) => {
                    tracing::error!("Presign upload failed for '{}': {}", filename, e);
                    // Fall through — if plugin also returned a direct URL, use that
                }
            }
        }
    }

    // Mode 2: legacy direct URL (plugin already uploaded the file)
    if let Some(url) = hook_result.get("url").and_then(|v| v.as_str()) {
        return Some(url.to_string());
    }

    // handled=true but no url and no presign — treat as not handled
    tracing::warn!("Plugin returned handled=true but no url or presign for '{}'", filename);
    None
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
    // Validate plugin_id to prevent path traversal
    if plugin_id.contains("..")
        || plugin_id.contains('/')
        || plugin_id.contains('\\')
        || !plugin_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ApiError::validation_error("Invalid plugin ID"));
    }

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

/// Base64-encode data (standard alphabet, padding)
/// Uses a simple inline implementation to avoid adding a crate dependency.
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
