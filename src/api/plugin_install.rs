//! Plugin installation API
//!
//! Handles plugin installation from:
//! - ZIP/TAR file upload
//! - GitHub releases download

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
pub struct PluginInstallResponse {
    pub success: bool,
    pub plugin_name: String,
    pub message: String,
}

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

#[derive(Debug, Deserialize)]
pub struct GitHubRepoQuery {
    /// GitHub repo in format "owner/repo"
    pub repo: String,
}

/// POST /api/v1/admin/plugins/upload - Upload and install plugin
pub async fn upload_plugin(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<Json<PluginInstallResponse>, ApiError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::validation_error(format!("Failed to read upload: {}", e)))?
        .ok_or_else(|| ApiError::validation_error("No file uploaded"))?;
    
    let filename = field.file_name()
        .map(|s: &str| s.to_string())
        .unwrap_or_else(|| "plugin.zip".to_string());
    
    let data = field.bytes()
        .await
        .map_err(|e| ApiError::validation_error(format!("Failed to read file: {}", e)))?;
    
    let temp_dir = TempDir::new()
        .map_err(|e| ApiError::internal_error(format!("Failed to create temp dir: {}", e)))?;
    
    let plugin_name = if filename.ends_with(".zip") {
        extract_zip(&data, temp_dir.path())?
    } else if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        extract_tar_gz(&data, temp_dir.path())?
    } else if filename.ends_with(".tar") {
        extract_tar(&data, temp_dir.path())?
    } else {
        return Err(ApiError::validation_error("Unsupported format. Use .zip, .tar, or .tar.gz"));
    };
    
    // Get plugins directory path
    let plugins_path = Path::new("plugins");
    if !plugins_path.exists() {
        fs::create_dir_all(plugins_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to create plugins dir: {}", e)))?;
    }
    
    let dest_path = plugins_path.join(&plugin_name);
    let src_path = temp_dir.path().join(&plugin_name);
    
    if dest_path.exists() {
        fs::remove_dir_all(&dest_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to remove existing: {}", e)))?;
    }
    
    copy_dir_all(&src_path, &dest_path)
        .map_err(|e| ApiError::internal_error(format!("Failed to install: {}", e)))?;
    
    // Reload plugins
    {
        let mut manager = state.plugin_manager.write().await;
        let _ = manager.reload().await;
    }
    
    Ok(Json(PluginInstallResponse {
        success: true,
        plugin_name: plugin_name.clone(),
        message: format!("Plugin '{}' installed", plugin_name),
    }))
}

/// GET /api/v1/admin/plugins/github/releases - Get releases from any GitHub repo
pub async fn list_github_releases(
    _user: AuthenticatedUser,
    Query(query): Query<GitHubRepoQuery>,
) -> Result<Json<Vec<GitHubReleaseInfo>>, ApiError> {
    let client = reqwest::Client::builder()
        .user_agent("Noteva-Plugin-Installer")
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

/// POST /api/v1/admin/plugins/github/install - Install plugin from GitHub
pub async fn install_github_plugin(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<GitHubInstallRequest>,
) -> Result<Json<PluginInstallResponse>, ApiError> {
    let client = reqwest::Client::builder()
        .user_agent("Noteva-Plugin-Installer")
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
    
    let plugin_name = if body.download_url.ends_with(".tar.gz") || body.download_url.ends_with(".tgz") {
        extract_tar_gz(&data, temp_dir.path())?
    } else {
        extract_zip(&data, temp_dir.path())?
    };
    
    let plugins_path = Path::new("plugins");
    if !plugins_path.exists() {
        fs::create_dir_all(plugins_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to create plugins dir: {}", e)))?;
    }
    
    let dest_path = plugins_path.join(&plugin_name);
    let src_path = temp_dir.path().join(&plugin_name);
    
    if dest_path.exists() {
        fs::remove_dir_all(&dest_path)
            .map_err(|e| ApiError::internal_error(format!("Remove error: {}", e)))?;
    }
    
    copy_dir_all(&src_path, &dest_path)
        .map_err(|e| ApiError::internal_error(format!("Install error: {}", e)))?;
    
    // Reload plugins
    {
        let mut manager = state.plugin_manager.write().await;
        let _ = manager.reload().await;
    }
    
    Ok(Json(PluginInstallResponse {
        success: true,
        plugin_name: plugin_name.clone(),
        message: format!("Plugin '{}' installed from GitHub", plugin_name),
    }))
}

/// DELETE /api/v1/admin/plugins/:id/uninstall - Uninstall a plugin
pub async fn uninstall_plugin(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, ApiError> {
    let plugin_path = Path::new("plugins").join(&id);
    
    if !plugin_path.exists() {
        return Err(ApiError::not_found(format!("Plugin '{}' not found", id)));
    }
    
    // Delete plugin files
    fs::remove_dir_all(&plugin_path)
        .map_err(|e| ApiError::internal_error(format!("Delete error: {}", e)))?;
    
    // Clean up database state and reload plugins
    {
        let mut manager = state.plugin_manager.write().await;
        // Delete plugin state from database
        let _ = manager.delete_state(&id).await;
        // Reload plugins
        let _ = manager.reload().await;
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
    
    let plugin_name: String = archive.file_names()
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
    
    Ok(plugin_name)
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
    let mut plugin_name = String::new();
    
    for entry in archive.entries()
        .map_err(|e| ApiError::validation_error(format!("Invalid TAR: {}", e)))? 
    {
        let mut entry = entry
            .map_err(|e| ApiError::internal_error(format!("Read error: {}", e)))?;
        
        let path = entry.path()
            .map_err(|e| ApiError::internal_error(format!("Path error: {}", e)))?;
        
        if plugin_name.is_empty() {
            if let Some(first) = path.components().next() {
                plugin_name = first.as_os_str().to_string_lossy().to_string();
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
    
    if plugin_name.is_empty() {
        return Err(ApiError::validation_error("Empty archive"));
    }
    
    Ok(plugin_name)
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
