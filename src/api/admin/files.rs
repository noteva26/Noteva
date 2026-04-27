//! File management API endpoints
//!
//! Provides endpoints for managing uploaded files:
//! - List files with search/filter
//! - Delete files
//! - Storage statistics

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};

/// File info response
#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub file_type: String,
    pub is_image: bool,
    pub created_at: String,
}

/// File list response
#[derive(Debug, Serialize)]
pub struct FileListResponse {
    pub files: Vec<FileInfo>,
    pub total: usize,
}

/// Storage stats response
#[derive(Debug, Serialize)]
pub struct StorageStatsResponse {
    pub total_files: usize,
    pub total_size: u64,
    pub total_size_display: String,
    pub image_count: usize,
    pub other_count: usize,
}

/// Query parameters for file listing
#[derive(Debug, Deserialize)]
pub struct FileListQuery {
    pub search: Option<String>,
    pub file_type: Option<String>, // "image", "file", or empty for all
}

/// Determine MIME type category from file extension
fn get_file_type(name: &str) -> String {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "ico" | "tiff" => {
            "image".to_string()
        }
        "pdf" => "pdf".to_string(),
        "mp4" | "webm" | "avi" | "mov" => "video".to_string(),
        "mp3" | "wav" | "ogg" | "flac" => "audio".to_string(),
        "zip" | "tar" | "gz" | "7z" | "rar" => "archive".to_string(),
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md" | "csv" => {
            "document".to_string()
        }
        _ => "other".to_string(),
    }
}

fn is_image(name: &str) -> bool {
    get_file_type(name) == "image"
}

fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// GET /api/v1/admin/files — List uploaded files
pub async fn list_files(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(query): Query<FileListQuery>,
) -> Result<Json<FileListResponse>, ApiError> {
    let upload_path = &state.upload_config.path;

    if !upload_path.exists() {
        return Ok(Json(FileListResponse {
            files: vec![],
            total: 0,
        }));
    }

    let mut files = Vec::new();
    let mut entries = fs::read_dir(upload_path)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to read uploads dir: {}", e)))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to read dir entry: {}", e)))?
    {
        let metadata = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Skip directories and hidden files
        if metadata.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let ft = get_file_type(&name);

        // Filter by search keyword
        if let Some(ref search) = query.search {
            if !search.is_empty() && !name.to_lowercase().contains(&search.to_lowercase()) {
                continue;
            }
        }

        // Filter by file type
        if let Some(ref type_filter) = query.file_type {
            match type_filter.as_str() {
                "image" => {
                    if ft != "image" {
                        continue;
                    }
                }
                "file" => {
                    if ft == "image" {
                        continue;
                    }
                }
                _ => {}
            }
        }

        let created_at = metadata
            .created()
            .or_else(|_| metadata.modified())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_default();

        files.push(FileInfo {
            url: format!("/uploads/{}", name),
            is_image: is_image(&name),
            file_type: ft,
            size: metadata.len(),
            name,
            created_at,
        });
    }

    // Sort by created_at descending (newest first)
    files.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total = files.len();
    Ok(Json(FileListResponse { files, total }))
}

/// GET /api/v1/admin/files/stats — Storage statistics
pub async fn get_storage_stats(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<StorageStatsResponse>, ApiError> {
    let upload_path = &state.upload_config.path;

    if !upload_path.exists() {
        return Ok(Json(StorageStatsResponse {
            total_files: 0,
            total_size: 0,
            total_size_display: "0 B".to_string(),
            image_count: 0,
            other_count: 0,
        }));
    }

    let mut total_files = 0usize;
    let mut total_size = 0u64;
    let mut image_count = 0usize;
    let mut other_count = 0usize;

    let mut entries = fs::read_dir(upload_path)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to read uploads dir: {}", e)))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to read dir entry: {}", e)))?
    {
        let metadata = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };

        if metadata.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        total_files += 1;
        total_size += metadata.len();

        if is_image(&name) {
            image_count += 1;
        } else {
            other_count += 1;
        }
    }

    Ok(Json(StorageStatsResponse {
        total_files,
        total_size,
        total_size_display: format_size(total_size),
        image_count,
        other_count,
    }))
}

/// DELETE /api/v1/admin/files/:filename — Delete a file
pub async fn delete_file(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(filename): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let upload_path = &state.upload_config.path;

    if filename.is_empty()
        || filename == "."
        || filename == ".."
        || filename.contains("..")
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains(':')
    {
        return Err(ApiError::validation_error("Invalid filename"));
    }
    let safe_name = filename;

    let file_path = upload_path.join(&safe_name);

    if !file_path.exists() {
        return Err(ApiError::not_found("File not found"));
    }

    fs::remove_file(&file_path)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to delete file: {}", e)))?;

    Ok(Json(serde_json::json!({
        "message": "File deleted successfully",
        "filename": safe_name,
    })))
}
