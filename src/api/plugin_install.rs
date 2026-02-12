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
    
    // Reload plugins and ensure database record exists
    {
        let mut manager = state.plugin_manager.write().await;
        let _ = manager.reload().await;
        
        // CRITICAL: Create initial database record for new plugin
        if let Some(plugin) = manager.get(&plugin_name) {
            let initial_state = crate::db::repositories::PluginState {
                plugin_id: plugin_name.clone(),
                enabled: false,
                settings: std::collections::HashMap::new(),
                last_version: None,
            };
            
            if let Err(e) = manager.ensure_state_exists(&initial_state).await {
                tracing::error!("Failed to create plugin state for {}: {}", plugin_name, e);
                return Err(ApiError::internal_error(format!("Failed to save plugin state: {}", e)));
            }
            
            // Use plugin display name for user-friendly message
            let display_name = &plugin.metadata.name;
            return Ok(Json(PluginInstallResponse {
                success: true,
                plugin_name: plugin_name.clone(),
                message: format!("插件「{}」安装成功", display_name),
            }));
        }
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

#[derive(Debug, Deserialize)]
pub struct InstallFromRepoRequest {
    pub repo: String,
    pub plugin_id: String,
}

#[derive(Debug, Deserialize)]
struct GitHubTreeResponse {
    tree: Vec<GitHubTreeItem>,
}

#[derive(Debug, Deserialize)]
struct GitHubTreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
}

/// POST /api/v1/admin/plugins/install-from-repo - Install plugin from GitHub repo
pub async fn install_from_repo(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<InstallFromRepoRequest>,
) -> Result<Json<PluginInstallResponse>, ApiError> {
    // Parse repo from URL
    let repo = if body.repo.contains("github.com") {
        body.repo
            .split("github.com/")
            .nth(1)
            .ok_or_else(|| ApiError::validation_error("Invalid GitHub URL"))?
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .to_string()
    } else {
        body.repo.clone()
    };
    
    let client = reqwest::Client::builder()
        .user_agent("Noteva-Plugin-Installer")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| ApiError::internal_error(format!("HTTP client error: {}", e)))?;
    
    // Check if it's official repo
    if repo == "noteva26/noteva-plugins" {
        // Official plugin: download specific directory using GitHub API
        install_official_plugin(&client, &body.plugin_id, &state).await
    } else {
        // Third-party plugin: download entire repo ZIP
        install_third_party_plugin(&client, &repo, &body.plugin_id, &state).await
    }
}

/// Install official plugin from noteva26/noteva-plugins
async fn install_official_plugin(
    client: &reqwest::Client,
    plugin_id: &str,
    state: &AppState,
) -> Result<Json<PluginInstallResponse>, ApiError> {
    const REPO: &str = "noteva26/noteva-plugins";
    
    // Get file tree
    let tree_url = format!("https://api.github.com/repos/{}/git/trees/main?recursive=1", REPO);
    let tree_response = client
        .get(&tree_url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to fetch tree: {}", e)))?;
    
    if !tree_response.status().is_success() {
        return Err(ApiError::internal_error("Failed to fetch repository tree"));
    }
    
    let tree: GitHubTreeResponse = tree_response
        .json()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to parse tree: {}", e)))?;
    
    // Find all files in plugin directory
    let plugin_files: Vec<String> = tree.tree
        .iter()
        .filter(|item| {
            item.item_type == "blob" && 
            item.path.starts_with(&format!("{}/", plugin_id))
        })
        .map(|item| item.path.clone())
        .collect();
    
    if plugin_files.is_empty() {
        return Err(ApiError::not_found(format!("Plugin '{}' not found in repository", plugin_id)));
    }
    
    // Create plugin directory
    let plugins_path = Path::new("plugins");
    if !plugins_path.exists() {
        fs::create_dir_all(plugins_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to create plugins dir: {}", e)))?;
    }
    
    let plugin_path = plugins_path.join(plugin_id);
    if plugin_path.exists() {
        fs::remove_dir_all(&plugin_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to remove existing plugin: {}", e)))?;
    }
    fs::create_dir_all(&plugin_path)
        .map_err(|e| ApiError::internal_error(format!("Failed to create plugin dir: {}", e)))?;
    
    // Download each file
    for file_path in plugin_files {
        let raw_url = format!("https://raw.githubusercontent.com/{}/main/{}", REPO, file_path);
        let response = client
            .get(&raw_url)
            .send()
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to download {}: {}", file_path, e)))?;
        
        if !response.status().is_success() {
            return Err(ApiError::internal_error(format!("Failed to download file: {}", file_path)));
        }
        
        let content = response.bytes()
            .await
            .map_err(|e| ApiError::internal_error(format!("Failed to read {}: {}", file_path, e)))?;
        
        // Remove plugin_id prefix from path
        let relative_path = file_path.strip_prefix(&format!("{}/", plugin_id))
            .unwrap_or(&file_path);
        let dest_path = plugin_path.join(relative_path);
        
        // Create parent directories if needed
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ApiError::internal_error(format!("Failed to create dir: {}", e)))?;
        }
        
        fs::write(&dest_path, content)
            .map_err(|e| ApiError::internal_error(format!("Failed to write {}: {}", relative_path, e)))?;
    }
    
    // Reload plugins and ensure database record exists
    {
        let mut manager = state.plugin_manager.write().await;
        let _ = manager.reload().await;
        
        // CRITICAL: Create initial database record for new plugin
        // This ensures the plugin can be enabled/disabled properly
        if let Some(plugin) = manager.get(plugin_id) {
            let initial_state = crate::db::repositories::PluginState {
                plugin_id: plugin_id.to_string(),
                enabled: false,
                settings: std::collections::HashMap::new(),
                last_version: None,
            };
            
            if let Err(e) = manager.ensure_state_exists(&initial_state).await {
                tracing::error!("Failed to create plugin state for {}: {}", plugin_id, e);
                return Err(ApiError::internal_error(format!("Failed to save plugin state: {}", e)));
            }
            
            // Use plugin display name for user-friendly message
            let display_name = &plugin.metadata.name;
            return Ok(Json(PluginInstallResponse {
                success: true,
                plugin_name: plugin_id.to_string(),
                message: format!("插件「{}」安装成功", display_name),
            }));
        }
    }
    
    Ok(Json(PluginInstallResponse {
        success: true,
        plugin_name: plugin_id.to_string(),
        message: format!("Plugin '{}' installed successfully", plugin_id),
    }))
}

/// Install third-party plugin from author's repo
async fn install_third_party_plugin(
    client: &reqwest::Client,
    repo: &str,
    plugin_id: &str,
    state: &AppState,
) -> Result<Json<PluginInstallResponse>, ApiError> {
    // Download repo ZIP
    let zip_url = format!("https://github.com/{}/archive/refs/heads/main.zip", repo);
    let response = client
        .get(&zip_url)
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
    
    // Extract ZIP
    extract_zip(&data, temp_dir.path())?;
    
    // Find plugin.json in extracted files
    let plugin_dir = find_plugin_dir(temp_dir.path(), plugin_id)?;
    
    let plugins_path = Path::new("plugins");
    if !plugins_path.exists() {
        fs::create_dir_all(plugins_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to create plugins dir: {}", e)))?;
    }
    
    let dest_path = plugins_path.join(plugin_id);
    if dest_path.exists() {
        fs::remove_dir_all(&dest_path)
            .map_err(|e| ApiError::internal_error(format!("Remove error: {}", e)))?;
    }
    
    copy_dir_all(&plugin_dir, &dest_path)
        .map_err(|e| ApiError::internal_error(format!("Install error: {}", e)))?;
    
    // Reload plugins and ensure database record exists
    {
        let mut manager = state.plugin_manager.write().await;
        let _ = manager.reload().await;
        
        // CRITICAL: Create initial database record for new plugin
        if let Some(plugin) = manager.get(plugin_id) {
            let initial_state = crate::db::repositories::PluginState {
                plugin_id: plugin_id.to_string(),
                enabled: false,
                settings: std::collections::HashMap::new(),
                last_version: None,
            };
            
            if let Err(e) = manager.ensure_state_exists(&initial_state).await {
                tracing::error!("Failed to create plugin state for {}: {}", plugin_id, e);
                return Err(ApiError::internal_error(format!("Failed to save plugin state: {}", e)));
            }
            
            // Use plugin display name for user-friendly message
            let display_name = &plugin.metadata.name;
            return Ok(Json(PluginInstallResponse {
                success: true,
                plugin_name: plugin_id.to_string(),
                message: format!("插件「{}」安装成功（来自 {}）", display_name, repo),
            }));
        }
    }
    
    Ok(Json(PluginInstallResponse {
        success: true,
        plugin_name: plugin_id.to_string(),
        message: format!("Plugin '{}' installed from {}", plugin_id, repo),
    }))
}

/// Find plugin directory containing plugin.json with matching ID
fn find_plugin_dir(base_path: &Path, plugin_id: &str) -> Result<std::path::PathBuf, ApiError> {
    for entry in fs::read_dir(base_path)
        .map_err(|e| ApiError::internal_error(format!("Read dir error: {}", e)))? 
    {
        let entry = entry.map_err(|e| ApiError::internal_error(format!("Entry error: {}", e)))?;
        let path = entry.path();
        
        if path.is_dir() {
            // Check if plugin.json exists in this directory
            let plugin_json_path = path.join("plugin.json");
            if plugin_json_path.exists() {
                // Verify plugin ID matches
                let content = fs::read_to_string(&plugin_json_path)
                    .map_err(|e| ApiError::internal_error(format!("Read plugin.json error: {}", e)))?;
                
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
                        if id == plugin_id {
                            return Ok(path);
                        }
                    }
                }
            }
            
            // Recursively search subdirectories
            if let Ok(found) = find_plugin_dir(&path, plugin_id) {
                return Ok(found);
            }
        }
    }
    
    Err(ApiError::not_found(format!("Plugin '{}' not found in archive", plugin_id)))
}

/// POST /api/v1/admin/plugins/github/install - Install plugin from GitHub (legacy, kept for compatibility)
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
    
    // Reload plugins and ensure database record exists
    {
        let mut manager = state.plugin_manager.write().await;
        let _ = manager.reload().await;
        
        // CRITICAL: Create initial database record for new plugin
        if let Some(plugin) = manager.get(&plugin_name) {
            let initial_state = crate::db::repositories::PluginState {
                plugin_id: plugin_name.clone(),
                enabled: false,
                settings: std::collections::HashMap::new(),
                last_version: None,
            };
            
            if let Err(e) = manager.ensure_state_exists(&initial_state).await {
                tracing::error!("Failed to create plugin state for {}: {}", plugin_name, e);
                return Err(ApiError::internal_error(format!("Failed to save plugin state: {}", e)));
            }
            
            // Use plugin display name for user-friendly message
            let display_name = &plugin.metadata.name;
            return Ok(Json(PluginInstallResponse {
                success: true,
                plugin_name: plugin_name.clone(),
                message: format!("插件「{}」安装成功", display_name),
            }));
        }
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


/// POST /api/v1/admin/plugins/:id/update - Update plugin to latest version
pub async fn update_plugin(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<PluginInstallResponse>, ApiError> {
    // Read current plugin.json to get homepage
    let plugin_path = Path::new("plugins").join(&id);
    if !plugin_path.exists() {
        return Err(ApiError::not_found(format!("Plugin '{}' not found", id)));
    }
    
    let plugin_json_path = plugin_path.join("plugin.json");
    if !plugin_json_path.exists() {
        return Err(ApiError::not_found("plugin.json not found"));
    }
    
    let content = fs::read_to_string(&plugin_json_path)
        .map_err(|e| ApiError::internal_error(format!("Failed to read plugin.json: {}", e)))?;
    
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| ApiError::internal_error(format!("Failed to parse plugin.json: {}", e)))?;
    
    let old_version = json.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    
    let homepage = json.get("homepage")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::validation_error("Plugin homepage not found"))?
        .to_string();
    
    // Install from repo (this will overwrite the existing plugin)
    let install_request = InstallFromRepoRequest {
        repo: homepage,
        plugin_id: id.clone(),
    };
    
    let _result = install_from_repo(State(state.clone()), _user, Json(install_request)).await?;
    
    // Read new version and display name
    let new_content = fs::read_to_string(&plugin_json_path)
        .map_err(|e| ApiError::internal_error(format!("Failed to read updated plugin.json: {}", e)))?;
    
    let new_json: serde_json::Value = serde_json::from_str(&new_content)
        .map_err(|e| ApiError::internal_error(format!("Failed to parse updated plugin.json: {}", e)))?;
    
    let new_version = new_json.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    
    let display_name = new_json.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();
    
    Ok(Json(PluginInstallResponse {
        success: true,
        plugin_name: id.clone(),
        message: format!("插件「{}」已从 {} 更新到 {}", display_name, old_version, new_version),
    }))
}
