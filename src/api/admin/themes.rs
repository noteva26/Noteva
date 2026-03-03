//! Theme management endpoints (list, switch, store, updates, reload)

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};

/// Request for theme switching
#[derive(Debug, Deserialize)]
pub struct ThemeSwitchRequest {
    pub theme: String,
}

/// Response for theme info
#[derive(Debug, Serialize)]
pub struct ThemeResponse {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub url: Option<String>,
    pub preview: Option<String>,
    pub active: bool,
    pub requires_noteva: String,
    pub compatible: bool,
    pub compatibility_message: Option<String>,
    pub has_settings: bool,
}

/// Response for theme list
#[derive(Debug, Serialize)]
pub struct ThemeListResponse {
    pub themes: Vec<ThemeResponse>,
    pub current: String,
}

/// Theme update info
#[derive(Debug, Serialize)]
pub struct ThemeUpdateInfo {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
}

/// Theme updates response
#[derive(Debug, Serialize)]
pub struct ThemeUpdatesResponse {
    pub updates: Vec<ThemeUpdateInfo>,
}

/// Store theme info
#[derive(Debug, Serialize, Deserialize)]
pub struct StoreThemeInfo {
    pub slug: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub github_url: Option<String>,
    pub external_url: Option<String>,
    pub license_type: String,
    pub price_info: Option<String>,
    pub download_source: String,
    pub download_count: i64,
    pub avg_rating: Option<f64>,
    pub rating_count: Option<i64>,
    pub tags: Vec<String>,
    pub installed: bool,
}

/// Theme store response
#[derive(Debug, Serialize)]
pub struct ThemeStoreResponse {
    pub themes: Vec<StoreThemeInfo>,
}

/// Store API item (matches store external API response)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StoreApiItem {
    slug: String,
    name: String,
    description: String,
    #[serde(default)]
    cover_image: String,
    author: String,
    version: String,
    github_url: Option<String>,
    external_url: Option<String>,
    license_type: String,
    price_info: Option<String>,
    download_source: String,
    download_count: i64,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StoreApiListResponse {
    items: Vec<StoreApiItem>,
    total: i64,
    page: i64,
    per_page: i64,
}

/// Store check-updates types
#[derive(Debug, Serialize)]
struct StoreInstalledItem {
    slug: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct StoreCheckUpdatesRequest {
    items: Vec<StoreInstalledItem>,
}

#[derive(Debug, Deserialize)]
struct StoreUpdateInfo {
    slug: String,
    current_version: String,
    latest_version: String,
}

/// GET /api/v1/admin/themes - List available themes
///
/// Requires admin authentication.
/// Satisfies requirement 6.1: Theme switching
pub async fn list_themes(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ThemeListResponse>, ApiError> {
    let theme_engine = state
        .theme_engine
        .read()
        .map_err(|e| ApiError::internal_error(format!("Failed to acquire theme lock: {}", e)))?;

    let themes = theme_engine.list_themes();

    let current = theme_engine.get_current_theme().to_string();

    let theme_responses: Vec<ThemeResponse> = themes
        .into_iter()
        .map(|info| ThemeResponse {
            name: info.name.clone(),
            display_name: info.display_name,
            description: info.description,
            version: info.version,
            author: info.author,
            url: info.url,
            preview: info.preview,
            active: info.name == current,
            requires_noteva: info.requires_noteva,
            compatible: info.compatible,
            compatibility_message: info.compatibility_message,
            has_settings: info.has_settings,
        })
        .collect();

    Ok(Json(ThemeListResponse {
        themes: theme_responses,
        current,
    }))
}

/// POST /api/v1/admin/themes/switch - Switch theme
///
/// Requires admin authentication.
/// Satisfies requirement 6.1: Theme switching
/// Triggers `theme_activate` filter hook — companion plugins can deny the switch.
pub async fn switch_theme(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<ThemeSwitchRequest>,
) -> Result<Json<ThemeResponse>, ApiError> {
    use crate::plugin::hook_names;

    // Remember previous theme for rollback
    let previous_theme = {
        let engine = state.theme_engine.read()
            .map_err(|e| ApiError::internal_error(format!("Failed to acquire theme lock: {}", e)))?;
        engine.get_current_theme().to_string()
    };

    // Switch theme and get result
    let (actual_theme, theme_info) = {
        let mut theme_engine = state
            .theme_engine
            .write()
            .map_err(|e| ApiError::internal_error(format!("Failed to acquire theme lock: {}", e)))?;

        let result = theme_engine.set_theme_with_fallback(&body.theme);

        if !result.success {
            return Err(ApiError::internal_error(
                result.error.unwrap_or_else(|| "Failed to switch theme".to_string())
            ));
        }

        let actual_theme = if result.used_fallback {
            theme_engine.get_default_theme().to_string()
        } else {
            body.theme
        };

        // Get theme info before releasing lock
        let theme_info = theme_engine.get_theme_info(&actual_theme).cloned();
        
        (actual_theme, theme_info)
    }; // Lock released here

    // Trigger theme_activate filter hook — companion plugins can deny the switch
    let theme_version = theme_info.as_ref().map(|i| i.version.clone()).unwrap_or_default();
    let activate_data = {
        let site_url = state.settings_service.get("site_url").await.ok().flatten().unwrap_or_default();
        serde_json::json!({
            "theme_name": actual_theme,
            "theme_version": theme_version,
            "site_url": site_url,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
    };

    let activate_result = state.hook_manager.trigger(
        hook_names::THEME_ACTIVATE,
        activate_data,
    );

    // Check if a companion plugin denied the switch
    if activate_result.get("allow").and_then(|v| v.as_bool()) == Some(false) {
        let message = activate_result.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Theme activation denied");
        tracing::warn!("Theme '{}' activation denied: {}", actual_theme, message);

        // Rollback: switch back to previous theme
        {
            let mut theme_engine = state.theme_engine.write()
                .map_err(|e| ApiError::internal_error(format!("Failed to acquire theme lock for rollback: {}", e)))?;
            let _ = theme_engine.set_theme_with_fallback(&previous_theme);
        }

        return Err(ApiError::validation_error(format!(
            "Theme activation denied: {}", message
        )));
    }

    // Save active theme to database
    state.settings_service
        .set("active_theme", &actual_theme)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to save theme setting: {}", e)))?;

    // Auto-create pages declared by the theme (non-destructive, skips existing)
    if let Some(ref info) = theme_info {
        if !info.pages.is_empty() {
            let pages: Vec<(String, String)> = info.pages.iter()
                .map(|p| (p.slug.clone(), p.title.clone()))
                .collect();
            let source = format!("theme:{}", actual_theme);
            if let Err(e) = state.page_service.ensure_pages(&pages, &source).await {
                tracing::warn!("Failed to auto-create pages for theme '{}': {}", actual_theme, e);
            }
        }
    }

    Ok(Json(ThemeResponse {
        name: actual_theme.clone(),
        display_name: theme_info.as_ref().map(|i| i.display_name.clone()).unwrap_or_else(|| actual_theme.clone()),
        description: theme_info.as_ref().and_then(|i| i.description.clone()),
        version: theme_info.as_ref().map(|i| i.version.clone()).unwrap_or_else(|| "1.0.0".to_string()),
        author: theme_info.as_ref().and_then(|i| i.author.clone()),
        url: theme_info.as_ref().and_then(|i| i.url.clone()),
        preview: theme_info.as_ref().and_then(|i| i.preview.clone()),
        active: true,
        requires_noteva: theme_info.as_ref().map(|i| i.requires_noteva.clone()).unwrap_or_default(),
        compatible: theme_info.as_ref().map(|i| i.compatible).unwrap_or(true),
        compatibility_message: theme_info.as_ref().and_then(|i| i.compatibility_message.clone()),
        has_settings: theme_info.as_ref().map(|i| i.has_settings).unwrap_or(false),
    }))
}

/// GET /api/v1/admin/themes/store - Get theme store from Noteva Store API
#[axum::debug_handler]
pub async fn get_theme_store(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ThemeStoreResponse>, ApiError> {
    let store_url = state.store_url.as_deref().unwrap_or("https://store.noteva.com");

    let client = reqwest::Client::builder()
        .user_agent("Noteva")
        .timeout(std::time::Duration::from_secs(15))
        .no_proxy()
        .build()
        .map_err(|e| ApiError::internal_error(format!("HTTP client error: {}", e)))?;

    let url = format!("{}/api/v1/store/themes?per_page=100", store_url);
    let response = client.get(&url).send().await
        .map_err(|e| ApiError::internal_error(format!("Failed to fetch store: {}", e)))?;

    if !response.status().is_success() {
        return Ok(Json(ThemeStoreResponse { themes: vec![] }));
    }

    let store_data: StoreApiListResponse = response.json().await
        .map_err(|e| ApiError::internal_error(format!("Failed to parse store response: {}", e)))?;

    // Get installed themes
    let installed_names: std::collections::HashSet<String> = {
        let theme_engine = state.theme_engine.read()
            .map_err(|e| ApiError::internal_error(format!("Lock error: {}", e)))?;
        theme_engine.list_themes().iter().map(|t| t.name.clone()).collect()
    };

    let themes: Vec<StoreThemeInfo> = store_data.items.into_iter().map(|item| {
        StoreThemeInfo {
            installed: installed_names.contains(&item.slug),
            slug: item.slug,
            name: item.name,
            version: item.version,
            description: Some(item.description),
            author: Some(item.author),
            github_url: item.github_url,
            external_url: item.external_url,
            license_type: item.license_type,
            price_info: item.price_info,
            download_source: item.download_source,
            download_count: item.download_count,
            avg_rating: None,
            rating_count: None,
            tags: item.tags,
        }
    }).collect();

    Ok(Json(ThemeStoreResponse { themes }))
}

/// GET /api/v1/admin/themes/updates - Check for theme updates via Store API
#[axum::debug_handler]
pub async fn check_theme_updates(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ThemeUpdatesResponse>, ApiError> {
    let store_url = match state.store_url.as_deref() {
        Some(url) => url.to_string(),
        None => "https://store.noteva.com".to_string(),
    };

    // Get installed themes
    let installed: Vec<StoreInstalledItem> = {
        let theme_engine = state.theme_engine.read()
            .map_err(|e| ApiError::internal_error(format!("Lock error: {}", e)))?;
        theme_engine.list_themes().iter().map(|t| StoreInstalledItem {
            slug: t.name.clone(),
            version: t.version.clone(),
        }).collect()
    };

    if installed.is_empty() {
        return Ok(Json(ThemeUpdatesResponse { updates: vec![] }));
    }

    let client = reqwest::Client::builder()
        .user_agent("Noteva")
        .timeout(std::time::Duration::from_secs(15))
        .no_proxy()
        .build()
        .map_err(|e| ApiError::internal_error(format!("HTTP client error: {}", e)))?;

    let url = format!("{}/api/v1/store/check-updates", store_url);
    let response = client.post(&url)
        .json(&StoreCheckUpdatesRequest { items: installed })
        .send().await
        .map_err(|e| ApiError::internal_error(format!("Failed to check updates: {}", e)))?;

    if !response.status().is_success() {
        return Ok(Json(ThemeUpdatesResponse { updates: vec![] }));
    }

    let store_updates: Vec<StoreUpdateInfo> = response.json().await
        .map_err(|e| ApiError::internal_error(format!("Failed to parse updates: {}", e)))?;

    let updates = store_updates.into_iter().map(|u| ThemeUpdateInfo {
        name: u.slug,
        current_version: u.current_version,
        latest_version: u.latest_version,
        has_update: true,
    }).collect();

    Ok(Json(ThemeUpdatesResponse { updates }))
}
