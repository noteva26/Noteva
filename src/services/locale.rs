//! Custom locale file-based storage
//!
//! Stores locale packs as JSON files in `data/locales/{code}.json`.
//! Each file has the structure: { "name": "...", "translations": { ... } }

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

const LOCALES_DIR: &str = "data/locales";

/// Custom locale entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomLocale {
    pub code: String,
    pub name: String,
    pub json_content: String,
}

/// Locale list item (without full JSON content)
#[derive(Debug, Clone, Serialize)]
pub struct LocaleListItem {
    pub code: String,
    pub name: String,
}

/// File-based locale wrapper
#[derive(Debug, Serialize, Deserialize)]
struct LocaleFile {
    name: String,
    translations: serde_json::Value,
}

/// Ensure data/locales directory exists
async fn ensure_dir() -> Result<()> {
    fs::create_dir_all(LOCALES_DIR).await?;
    Ok(())
}

/// Get the file path for a locale code
pub fn is_valid_locale_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 20
        && code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn locale_path(code: &str) -> Result<PathBuf> {
    if !is_valid_locale_code(code) {
        bail!("invalid locale code");
    }
    Ok(Path::new(LOCALES_DIR).join(format!("{}.json", code)))
}

/// List all custom locales (code + name only)
pub async fn list_locales() -> Result<Vec<LocaleListItem>> {
    ensure_dir().await?;
    let mut entries = fs::read_dir(LOCALES_DIR).await?;
    let mut items = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Some(code) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(file) = serde_json::from_str::<LocaleFile>(&content) {
                        items.push(LocaleListItem {
                            code: code.to_string(),
                            name: file.name,
                        });
                    }
                }
            }
        }
    }

    items.sort_by(|a, b| a.code.cmp(&b.code));
    Ok(items)
}

/// Get a locale by code (full content)
pub async fn get_locale(code: &str) -> Result<Option<CustomLocale>> {
    let path = locale_path(code)?;
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path).await?;
    let file: LocaleFile = serde_json::from_str(&content)?;

    Ok(Some(CustomLocale {
        code: code.to_string(),
        name: file.name,
        json_content: serde_json::to_string(&file.translations)?,
    }))
}

/// Insert or update a locale
pub async fn upsert_locale(code: &str, name: &str, json_content: &str) -> Result<()> {
    ensure_dir().await?;

    let translations: serde_json::Value = serde_json::from_str(json_content)?;
    let file = LocaleFile {
        name: name.to_string(),
        translations,
    };
    let content = serde_json::to_string_pretty(&file)?;

    fs::write(locale_path(code)?, content).await?;
    Ok(())
}

/// Delete a locale by code
pub async fn delete_locale(code: &str) -> Result<bool> {
    let path = locale_path(code)?;
    if path.exists() {
        fs::remove_file(&path).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
