//! Update check and perform update endpoints

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::middleware::{ApiError, AuthenticatedUser};
use sha2::Digest;

/// App version constant - update when releasing
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_UPDATE_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_UPDATE_BINARY_BYTES: usize = 100 * 1024 * 1024;

/// Response for update check
#[derive(Debug, Serialize)]
pub struct UpdateCheckResponse {
    /// Current version
    pub current_version: String,
    /// Latest version available
    pub latest_version: Option<String>,
    /// Whether an update is available
    pub update_available: bool,
    /// Release URL
    pub release_url: Option<String>,
    /// Release notes
    pub release_notes: Option<String>,
    /// Release date
    pub release_date: Option<String>,
    /// Error message if check failed
    pub error: Option<String>,
}

/// GitHub release info
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    published_at: Option<String>,
    #[allow(dead_code)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

/// GitHub release asset
#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: Option<u64>,
}

/// Request for performing update
#[derive(Debug, Deserialize)]
pub struct PerformUpdateRequest {
    /// Target version to update to (e.g. "0.2.0")
    pub version: String,
}

/// Response for perform update
#[derive(Debug, Serialize)]
pub struct PerformUpdateResponse {
    pub success: bool,
    pub message: String,
}

/// GET /api/v1/admin/update-check - Check for updates
///
/// Requires admin authentication.
/// Checks GitHub releases for new versions.
pub async fn check_update(_user: AuthenticatedUser) -> Json<UpdateCheckResponse> {
    let current_version = APP_VERSION.to_string();

    let api_url = "https://api.github.com/repos/noteva26/Noteva/releases/latest";

    // Make HTTP request to GitHub API
    let client = match reqwest::Client::builder()
        .user_agent("Noteva-Update-Checker")
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Json(UpdateCheckResponse {
                current_version,
                latest_version: None,
                update_available: false,
                release_url: None,
                release_notes: None,
                release_date: None,
                error: Some(format!("Failed to create HTTP client: {}", e)),
            });
        }
    };

    let response = match client.get(api_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return Json(UpdateCheckResponse {
                current_version,
                latest_version: None,
                update_available: false,
                release_url: None,
                release_notes: None,
                release_date: None,
                error: Some(format!("Failed to fetch releases: {}", e)),
            });
        }
    };

    if !response.status().is_success() {
        return Json(UpdateCheckResponse {
            current_version,
            latest_version: None,
            update_available: false,
            release_url: None,
            release_notes: None,
            release_date: None,
            error: Some(format!("GitHub API returned status: {}", response.status())),
        });
    }

    // Parse latest release
    match response.json::<GitHubRelease>().await {
        Ok(rel) => {
            let latest = rel.tag_name.trim_start_matches('v').to_string();
            let current = current_version.trim_start_matches('v');
            let update_available = version_compare(&latest, current);

            Json(UpdateCheckResponse {
                current_version,
                latest_version: Some(latest),
                update_available,
                release_url: Some(rel.html_url),
                release_notes: rel.body,
                release_date: rel.published_at,
                error: None,
            })
        }
        Err(e) => Json(UpdateCheckResponse {
            current_version,
            latest_version: None,
            update_available: false,
            release_url: None,
            release_notes: None,
            release_date: None,
            error: Some(format!("Failed to parse release: {}", e)),
        }),
    }
}

/// Compare two version strings
/// Returns true if latest > current
pub(super) fn version_compare(latest: &str, current: &str) -> bool {
    // Parse versions into comparable parts
    let parse_version = |v: &str| -> Vec<(u32, String)> {
        let mut parts = Vec::new();
        let mut num = String::new();
        let mut suffix = String::new();
        let mut in_suffix = false;

        for c in v.chars() {
            if c.is_ascii_digit() && !in_suffix {
                num.push(c);
            } else if c == '.' || c == '-' {
                if !num.is_empty() {
                    parts.push((num.parse().unwrap_or(0), suffix.clone()));
                    num.clear();
                    suffix.clear();
                }
                if c == '-' {
                    in_suffix = true;
                }
            } else {
                suffix.push(c);
                in_suffix = true;
            }
        }
        if !num.is_empty() || !suffix.is_empty() {
            parts.push((num.parse().unwrap_or(0), suffix));
        }
        parts
    };

    let latest_parts = parse_version(latest);
    let current_parts = parse_version(current);

    for i in 0..latest_parts.len().max(current_parts.len()) {
        let (l_num, l_suffix) = latest_parts.get(i).cloned().unwrap_or((0, String::new()));
        let (c_num, c_suffix) = current_parts.get(i).cloned().unwrap_or((0, String::new()));

        if l_num > c_num {
            return true;
        } else if l_num < c_num {
            return false;
        }

        // Compare suffixes (empty > "beta" > "alpha")
        let suffix_order = |s: &str| -> i32 {
            if s.is_empty() {
                3
            } else if s.contains("rc") {
                2
            } else if s.contains("beta") {
                1
            } else if s.contains("alpha") {
                0
            } else {
                3
            }
        };

        let l_order = suffix_order(&l_suffix);
        let c_order = suffix_order(&c_suffix);

        if l_order > c_order {
            return true;
        } else if l_order < c_order {
            return false;
        }
    }

    false
}

/// Get the expected asset name for the current platform
fn get_platform_asset_name() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Some("noteva-linux-x86_64.tar.gz");
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return Some("noteva-linux-arm64.tar.gz");
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Some("noteva-windows-x86_64.zip");
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        return Some("noteva-windows-arm64.zip");
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Some("noteva-macos-x86_64.tar.gz");
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Some("noteva-macos-arm64.tar.gz");
    }
    #[allow(unreachable_code)]
    None
}

/// Extract binary from archive bytes
fn extract_binary(archive_bytes: &[u8], asset_name: &str) -> Result<Vec<u8>, String> {
    if asset_name.ends_with(".zip") {
        // ZIP archive (Windows)
        let cursor = std::io::Cursor::new(archive_bytes);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to open zip: {}", e))?;

        // Find the binary in the archive
        let binary_name = if cfg!(windows) {
            "noteva.exe"
        } else {
            "noteva"
        };
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;
            if file.name().ends_with(binary_name) {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut buf)
                    .map_err(|e| format!("Failed to read binary from zip: {}", e))?;
                return Ok(buf);
            }
        }
        Err("Binary not found in zip archive".to_string())
    } else {
        // tar.gz archive (Linux/macOS)
        let cursor = std::io::Cursor::new(archive_bytes);
        let gz = flate2::read::GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(gz);

        let binary_name = "noteva";
        for entry in archive
            .entries()
            .map_err(|e| format!("Failed to read tar: {}", e))?
        {
            let mut entry = entry.map_err(|e| format!("Failed to read tar entry: {}", e))?;
            let path = entry
                .path()
                .map_err(|e| format!("Failed to get path: {}", e))?;
            if path.file_name().map(|n| n == binary_name).unwrap_or(false) {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut buf)
                    .map_err(|e| format!("Failed to read binary from tar: {}", e))?;
                return Ok(buf);
            }
        }
        Err("Binary not found in tar archive".to_string())
    }
}

/// POST /api/v1/admin/update-perform - Download and apply update
///
/// Downloads the new binary from GitHub, replaces the current one, then exits.
/// The process manager (systemd/Docker restart policy) should restart the process.
pub async fn perform_update(
    _user: AuthenticatedUser,
    Json(body): Json<PerformUpdateRequest>,
) -> Result<Json<PerformUpdateResponse>, ApiError> {
    let target_version = body.version.trim_start_matches('v').to_string();
    let current = APP_VERSION.trim_start_matches('v');

    // Verify update is actually newer
    if !version_compare(&target_version, current) {
        return Err(ApiError::validation_error(format!(
            "Version {} is not newer than current {}",
            target_version, current
        )));
    }

    // Determine platform asset
    let asset_name = get_platform_asset_name()
        .ok_or_else(|| ApiError::internal_error("Unsupported platform for auto-update"))?;

    // Fetch release info to get download URL
    let tag = format!("v{}", target_version);
    let api_url = format!(
        "https://api.github.com/repos/noteva26/Noteva/releases/tags/{}",
        tag
    );

    let client = reqwest::Client::builder()
        .user_agent("Noteva-Updater")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ApiError::internal_error(format!("HTTP client error: {}", e)))?;

    let release: GitHubRelease = client
        .get(&api_url)
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to fetch release: {}", e)))?
        .json()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to parse release: {}", e)))?;

    // Find matching asset
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            ApiError::internal_error(format!("No asset found for platform: {}", asset_name))
        })?;

    tracing::info!(
        "Downloading update: {} from {}",
        asset.name,
        asset.browser_download_url
    );

    if asset
        .size
        .is_some_and(|size| size > MAX_UPDATE_ARCHIVE_BYTES)
    {
        return Err(ApiError::validation_error(format!(
            "Update package too large. Maximum size: {} MB",
            MAX_UPDATE_ARCHIVE_BYTES / 1024 / 1024
        )));
    }

    // Download the archive
    let response = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to download: {}", e)))?;
    if response
        .content_length()
        .is_some_and(|len| len > MAX_UPDATE_ARCHIVE_BYTES)
    {
        return Err(ApiError::validation_error(format!(
            "Update package too large. Maximum size: {} MB",
            MAX_UPDATE_ARCHIVE_BYTES / 1024 / 1024
        )));
    }
    let archive_bytes = response
        .bytes()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to read download: {}", e)))?;
    if archive_bytes.len() as u64 > MAX_UPDATE_ARCHIVE_BYTES {
        return Err(ApiError::validation_error(format!(
            "Update package too large. Maximum size: {} MB",
            MAX_UPDATE_ARCHIVE_BYTES / 1024 / 1024
        )));
    }

    tracing::info!(
        "Downloaded {} bytes, verifying checksum...",
        archive_bytes.len()
    );

    // SHA256 verification: try to download the .sha256 checksum file
    let checksum_url = format!("{}.sha256", asset.browser_download_url);
    match client.get(&checksum_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.text().await {
                Ok(checksum_text) => {
                    // Checksum file format: "<hash>  <filename>" or just "<hash>"
                    let expected_hash = checksum_text
                        .trim()
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_lowercase();
                    if expected_hash.len() != 64
                        || !expected_hash.chars().all(|ch| ch.is_ascii_hexdigit())
                    {
                        return Err(ApiError::validation_error("Invalid SHA256 checksum file"));
                    }

                    // Compute SHA256 of downloaded archive
                    let mut hasher = sha2::Sha256::new();
                    sha2::Digest::update(&mut hasher, &archive_bytes);
                    let actual_hash = format!("{:x}", sha2::Digest::finalize(hasher));

                    if expected_hash == actual_hash {
                        tracing::info!("SHA256 checksum verified: {}", actual_hash);
                    } else {
                        return Err(ApiError::internal_error(format!(
                            "SHA256 mismatch! Expected: {}, Got: {}. Download may be corrupted.",
                            expected_hash, actual_hash
                        )));
                    }
                }
                Err(e) => {
                    return Err(ApiError::internal_error(format!(
                        "Could not read checksum file: {}",
                        e
                    )));
                }
            }
        }
        _ => {
            return Err(ApiError::validation_error(
                "Missing SHA256 checksum file for update asset",
            ));
        }
    }

    // Extract binary from archive
    let binary_data =
        extract_binary(&archive_bytes, asset_name).map_err(|e| ApiError::internal_error(e))?;
    if binary_data.len() > MAX_UPDATE_BINARY_BYTES {
        return Err(ApiError::validation_error(format!(
            "Update binary too large. Maximum size: {} MB",
            MAX_UPDATE_BINARY_BYTES / 1024 / 1024
        )));
    }

    // Get current executable path
    let self_path = std::env::current_exe()
        .map_err(|e| ApiError::internal_error(format!("Failed to get exe path: {}", e)))?;

    tracing::info!("Replacing binary at: {:?}", self_path);

    // Replace binary
    #[cfg(unix)]
    {
        let tmp_path = self_path.with_extension("new");
        std::fs::write(&tmp_path, &binary_data)
            .map_err(|e| ApiError::internal_error(format!("Failed to write new binary: {}", e)))?;

        // Set executable permission
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| ApiError::internal_error(format!("Failed to set permissions: {}", e)))?;

        std::fs::rename(&tmp_path, &self_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to replace binary: {}", e)))?;
    }

    #[cfg(windows)]
    {
        let old_path = self_path.with_extension("old.exe");
        let tmp_path = self_path.with_extension("new.exe");

        std::fs::write(&tmp_path, &binary_data)
            .map_err(|e| ApiError::internal_error(format!("Failed to write new binary: {}", e)))?;

        // Windows: rename current -> .old, then new -> current
        let _ = std::fs::remove_file(&old_path); // clean up previous .old if exists
        std::fs::rename(&self_path, &old_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to rename old binary: {}", e)))?;
        std::fs::rename(&tmp_path, &self_path)
            .map_err(|e| ApiError::internal_error(format!("Failed to rename new binary: {}", e)))?;
    }

    tracing::info!(
        "Update to v{} complete, scheduling restart...",
        target_version
    );

    // Schedule self-restart: spawn a shell script that waits for us to exit, then starts new binary
    let exe_path = self_path.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        tracing::info!("Setting up self-restart...");

        #[cfg(unix)]
        {
            use std::process::Command;
            let exe_str = exe_path.to_string_lossy().to_string();
            let dir = exe_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_string_lossy()
                .to_string();
            // nohup a shell that sleeps 3s (enough for old process to exit) then starts new binary
            let script = format!("sleep 3 && cd '{}' && '{}' &", dir, exe_str);
            let _ = Command::new("sh")
                .arg("-c")
                .arg(&format!("nohup sh -c \"{}\" > /dev/null 2>&1 &", script))
                .spawn();
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            let exe_str = exe_path.to_string_lossy().to_string();
            let dir = exe_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_string_lossy()
                .to_string();
            let script = format!(
                "timeout /t 3 /nobreak >nul && cd /d \"{}\" && start \"\" \"{}\"",
                dir, exe_str
            );
            let _ = Command::new("cmd")
                .args(["/C", &format!("start /b cmd /C \"{}\"", script)])
                .spawn();
        }

        tracing::info!("Restart scheduled, exiting...");
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        std::process::exit(0);
    });

    Ok(Json(PerformUpdateResponse {
        success: true,
        message: format!("Updated to v{}. Restarting...", target_version),
    }))
}
