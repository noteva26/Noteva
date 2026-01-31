//! Theme installation API
//!
//! Handles theme installation from:
//! - ZIP/TAR file upload
//! - Any GitHub repository releases

use axum::{
    extract::{Multipart, State, Query},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read, Cursor};
use std::path::Path;
use tempfile::TempDir;
use zip::ZipArchive;
use flate2::read::GzDecoder;
use tar::Archive;

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};

#[derive(Debug, Serialize)]
pub struct ThemeInstallResponse {
    pub success: bool,
    pub theme_name: String,
    pub message: String,
}

/// GitHub release info
#[derive(Debug, Serialize)]
pub struct GitHubReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub published_at: Option<String>,
    pub assets: Vec<GitHubAssetInfo>,
}

#[derive(Debug, Serialize)]
pub struct GitHubAssetInfo {
    pub name: String,
    pub size: u64,
    pub download_url: String,
}

/// Query for fetching releases from a GitHub repo
#[derive(Debug, Deserialize)]
pub struct GitHubRepoQuery {
    /// GitHub repo in format "owner/repo"
    pub repo: String,
}

/// POST /api/v1/admin/themes/upload - Upload and install theme from ZIP/TAR
pub async fn upload_theme(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<Json<ThemeInstallResponse>, ApiError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::validation_error(format!("Failed to read upload: {}", e)))?
        .ok_or_else(|| ApiError::validation_error("No file uploaded"))?;
    
    let filename = field.file_name()
        .map(|s: &str| s.to_string())
        .unwrap_or_else(|| "theme.zip".to_string());
    
    let data = field.bytes()
        .await
        .map_err(|e| ApiError::validation_error(format!("Failed to read file: {}", e)))?;
    
    let temp_dir = TempDir::new()
        .map_err(|e| ApiError::internal_error(format!("Failed to create temp dir: {}", e)))?;
    
    let theme_name = if filename.ends_with(".zip") {
        extract_zip(&data, temp_dir.path())?
    } else if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        extract_tar_gz(&data, temp_dir.path())?
    } else if filename.ends_with(".tar") {
        extract_tar(&data, temp_dir.path())?
    } else {
        return Err(ApiError::validation_error("Unsupported format. Use .zip, .tar, or .tar.gz"));
    };
    
    let themes_path = {
        let engine = state.theme_engine.read()
            .map_err(|e| ApiError::internal_error(format!("Lock error: {}", e)))?;
        engine.get_theme_path(&theme_name).parent().unwrap().to_path_buf()
    };
    
    let dest_path = themes_path.join(&theme_name);
    let src_path = temp_dir.path().join(&theme_name);
    
    if dest_path.exists() {
        fs::remove_dir_all(&dest_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to remove existing: {}", e)))?;
    }
    
    copy_dir_all(&src_path, &dest_path)
        .map_err(|e| ApiError::internal_error(format!("Failed to install: {}", e)))?;
    
    {
        let mut engine = state.theme_engine.write()
            .map_err(|e| ApiError::internal_error(format!("Lock error: {}", e)))?;
        let _ = engine.reload_templates();
    }
    
    Ok(Json(ThemeInstallResponse {
        success: true,
        theme_name: theme_name.clone(),
        message: format!("Theme '{}' installed", theme_name),
    }))
}

/// GET /api/v1/admin/themes/github/releases - Get releases from any GitHub repo
pub async fn list_github_releases(
    _user: AuthenticatedUser,
    Query(query): Query<GitHubRepoQuery>,
) -> Result<Json<Vec<GitHubReleaseInfo>>, ApiError> {
    let client = reqwest::Client::builder()
        .user_agent("Noteva-Theme-Installer")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ApiError::internal_error(format!("HTTP client error: {}", e)))?;
    
    let url = format!("https://api.github.com/repos/{}/releases", query.repo);
    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to fetch: {}", e)))?;
    
    if !response.status().is_success() {
        return Err(ApiError::internal_error(format!("GitHub error: {} - Check if repo exists", response.status())));
    }
    
    let releases: Vec<GitHubRelease> = response.json()
        .await
        .map_err(|e| ApiError::internal_error(format!("Parse error: {}", e)))?;
    
    let result: Vec<GitHubReleaseInfo> = releases.into_iter().map(|r| {
        GitHubReleaseInfo {
            tag_name: r.tag_name,
            name: r.name.unwrap_or_default(),
            published_at: r.published_at,
            assets: r.assets.into_iter()
                .filter(|a| a.name.ends_with(".zip") || a.name.ends_with(".tar.gz") || a.name.ends_with(".tgz"))
                .map(|a| GitHubAssetInfo {
                    name: a.name,
                    size: a.size,
                    download_url: a.browser_download_url,
                })
                .collect(),
        }
    }).collect();
    
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct GitHubInstallRequest {
    pub download_url: String,
}

/// POST /api/v1/admin/themes/github/install - Install theme from GitHub release asset
pub async fn install_github_theme(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<GitHubInstallRequest>,
) -> Result<Json<ThemeInstallResponse>, ApiError> {
    let client = reqwest::Client::builder()
        .user_agent("Noteva-Theme-Installer")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| ApiError::internal_error(format!("HTTP client error: {}", e)))?;
    
    let response = client.get(&body.download_url)
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("Download failed: {}", e)))?;
    
    if !response.status().is_success() {
        return Err(ApiError::internal_error(format!("Download error: {}", response.status())));
    }
    
    let data = response.bytes()
        .await
        .map_err(|e| ApiError::internal_error(format!("Read error: {}", e)))?;
    
    let temp_dir = TempDir::new()
        .map_err(|e| ApiError::internal_error(format!("Temp dir error: {}", e)))?;
    
    let theme_name = if body.download_url.ends_with(".tar.gz") || body.download_url.ends_with(".tgz") {
        extract_tar_gz(&data, temp_dir.path())?
    } else {
        extract_zip(&data, temp_dir.path())?
    };
    
    let themes_path = {
        let engine = state.theme_engine.read()
            .map_err(|e| ApiError::internal_error(format!("Lock error: {}", e)))?;
        engine.get_theme_path(&theme_name).parent().unwrap().to_path_buf()
    };
    
    let dest_path = themes_path.join(&theme_name);
    let src_path = temp_dir.path().join(&theme_name);
    
    if dest_path.exists() {
        fs::remove_dir_all(&dest_path)
            .map_err(|e| ApiError::internal_error(format!("Remove error: {}", e)))?;
    }
    
    copy_dir_all(&src_path, &dest_path)
        .map_err(|e| ApiError::internal_error(format!("Install error: {}", e)))?;
    
    {
        let mut engine = state.theme_engine.write()
            .map_err(|e| ApiError::internal_error(format!("Lock error: {}", e)))?;
        let _ = engine.reload_templates();
    }
    
    Ok(Json(ThemeInstallResponse {
        success: true,
        theme_name: theme_name.clone(),
        message: format!("Theme '{}' installed from GitHub", theme_name),
    }))
}

/// DELETE /api/v1/admin/themes/:name - Delete a theme
pub async fn delete_theme(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    if name == "default" {
        return Err(ApiError::validation_error("Cannot delete default theme"));
    }
    
    let theme_path = {
        let engine = state.theme_engine.read()
            .map_err(|e| ApiError::internal_error(format!("Lock error: {}", e)))?;
        engine.get_theme_path(&name)
    };
    
    if !theme_path.exists() {
        return Err(ApiError::not_found(format!("Theme '{}' not found", name)));
    }
    
    fs::remove_dir_all(&theme_path)
        .map_err(|e| ApiError::internal_error(format!("Delete error: {}", e)))?;
    
    {
        let mut engine = state.theme_engine.write()
            .map_err(|e| ApiError::internal_error(format!("Lock error: {}", e)))?;
        let _ = engine.reload_templates();
    }
    
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

fn extract_zip(data: &[u8], dest: &Path) -> Result<String, ApiError> {
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| ApiError::validation_error(format!("Invalid ZIP: {}", e)))?;
    
    let theme_name: String = archive.file_names()
        .next()
        .and_then(|name: &str| name.split('/').next())
        .map(|s: &str| s.to_string())
        .ok_or_else(|| ApiError::validation_error("Empty archive"))?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| ApiError::internal_error(format!("Read error: {}", e)))?;
        
        let outpath = dest.join(file.name());
        
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)
                .map_err(|e| ApiError::internal_error(format!("Dir error: {}", e)))?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| ApiError::internal_error(format!("Dir error: {}", e)))?;
            }
            let mut outfile = File::create(&outpath)
                .map_err(|e| ApiError::internal_error(format!("File error: {}", e)))?;
            io::copy(&mut file, &mut outfile)
                .map_err(|e| ApiError::internal_error(format!("Write error: {}", e)))?;
        }
    }
    
    Ok(theme_name)
}

fn extract_tar_gz(data: &[u8], dest: &Path) -> Result<String, ApiError> {
    let cursor = Cursor::new(data);
    let decoder = GzDecoder::new(cursor);
    extract_tar_inner(decoder, dest)
}

fn extract_tar(data: &[u8], dest: &Path) -> Result<String, ApiError> {
    let cursor = Cursor::new(data);
    extract_tar_inner(cursor, dest)
}

fn extract_tar_inner<R: Read>(reader: R, dest: &Path) -> Result<String, ApiError> {
    let mut archive = Archive::new(reader);
    let mut theme_name = String::new();
    
    for entry in archive.entries()
        .map_err(|e| ApiError::validation_error(format!("Invalid TAR: {}", e)))? 
    {
        let mut entry = entry
            .map_err(|e| ApiError::internal_error(format!("Read error: {}", e)))?;
        
        let path = entry.path()
            .map_err(|e| ApiError::internal_error(format!("Path error: {}", e)))?;
        
        if theme_name.is_empty() {
            if let Some(first) = path.components().next() {
                theme_name = first.as_os_str().to_string_lossy().to_string();
            }
        }
        
        let outpath = dest.join(&path);
        
        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&outpath)
                .map_err(|e| ApiError::internal_error(format!("Dir error: {}", e)))?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| ApiError::internal_error(format!("Dir error: {}", e)))?;
            }
            entry.unpack(&outpath)
                .map_err(|e| ApiError::internal_error(format!("Unpack error: {}", e)))?;
        }
    }
    
    if theme_name.is_empty() {
        return Err(ApiError::validation_error("Empty archive"));
    }
    
    Ok(theme_name)
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
