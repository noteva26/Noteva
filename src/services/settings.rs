//! Settings service
//!
//! Business logic for site settings management.
//! Satisfies requirement 5.3: System configuration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::db::repositories::{SettingsRepository, SqlxSettingsRepository};

/// Known setting keys
pub mod keys {
    pub const SITE_NAME: &str = "site_name";
    pub const SITE_DESCRIPTION: &str = "site_description";
    pub const SITE_SUBTITLE: &str = "site_subtitle";
    pub const SITE_LOGO: &str = "site_logo";
    pub const SITE_FOOTER: &str = "site_footer";
    pub const POSTS_PER_PAGE: &str = "posts_per_page";
}

/// Site settings structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteSettings {
    pub site_name: String,
    pub site_description: String,
    pub site_subtitle: String,
    pub site_logo: String,
    pub site_footer: String,
    pub posts_per_page: i32,
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            site_name: "Noteva Blog".to_string(),
            site_description: "A lightweight blog powered by Noteva".to_string(),
            site_subtitle: String::new(),
            site_logo: String::new(),
            site_footer: String::new(),
            posts_per_page: 10,
        }
    }
}

/// Settings service errors
#[derive(Debug, Error)]
pub enum SettingsServiceError {
    #[error("Failed to load settings: {0}")]
    LoadError(String),
    
    #[error("Failed to save settings: {0}")]
    SaveError(String),
    
    #[error("Invalid setting value: {0}")]
    InvalidValue(String),
}

/// Settings service for managing site configuration
pub struct SettingsService {
    repo: Arc<dyn SettingsRepository>,
}

impl SettingsService {
    /// Create a new settings service
    pub fn new(repo: Arc<dyn SettingsRepository>) -> Self {
        Self { repo }
    }
    
    /// Create from SQLx repository
    pub fn from_sqlx(repo: SqlxSettingsRepository) -> Self {
        Self::new(Arc::new(repo))
    }
    
    /// Get all site settings
    pub async fn get_site_settings(&self) -> Result<SiteSettings, SettingsServiceError> {
        let keys = &[
            keys::SITE_NAME,
            keys::SITE_DESCRIPTION,
            keys::SITE_SUBTITLE,
            keys::SITE_LOGO,
            keys::SITE_FOOTER,
            keys::POSTS_PER_PAGE,
        ];
        
        let settings = self.repo.get_many(keys).await
            .map_err(|e| SettingsServiceError::LoadError(e.to_string()))?;
        
        let defaults = SiteSettings::default();
        
        Ok(SiteSettings {
            site_name: settings.get(keys::SITE_NAME)
                .cloned()
                .unwrap_or(defaults.site_name),
            site_description: settings.get(keys::SITE_DESCRIPTION)
                .cloned()
                .unwrap_or(defaults.site_description),
            site_subtitle: settings.get(keys::SITE_SUBTITLE)
                .cloned()
                .unwrap_or(defaults.site_subtitle),
            site_logo: settings.get(keys::SITE_LOGO)
                .cloned()
                .unwrap_or(defaults.site_logo),
            site_footer: settings.get(keys::SITE_FOOTER)
                .cloned()
                .unwrap_or(defaults.site_footer),
            posts_per_page: settings.get(keys::POSTS_PER_PAGE)
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.posts_per_page),
        })
    }
    
    /// Update site settings
    pub async fn update_site_settings(&self, settings: &SiteSettings) -> Result<(), SettingsServiceError> {
        let mut map = HashMap::new();
        map.insert(keys::SITE_NAME.to_string(), settings.site_name.clone());
        map.insert(keys::SITE_DESCRIPTION.to_string(), settings.site_description.clone());
        map.insert(keys::SITE_SUBTITLE.to_string(), settings.site_subtitle.clone());
        map.insert(keys::SITE_LOGO.to_string(), settings.site_logo.clone());
        map.insert(keys::SITE_FOOTER.to_string(), settings.site_footer.clone());
        map.insert(keys::POSTS_PER_PAGE.to_string(), settings.posts_per_page.to_string());
        
        self.repo.set_many(&map).await
            .map_err(|e| SettingsServiceError::SaveError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Get a single setting value
    pub async fn get(&self, key: &str) -> Result<Option<String>, SettingsServiceError> {
        let setting = self.repo.get(key).await
            .map_err(|e| SettingsServiceError::LoadError(e.to_string()))?;
        Ok(setting.map(|s| s.value))
    }
    
    /// Set a single setting value
    pub async fn set(&self, key: &str, value: &str) -> Result<(), SettingsServiceError> {
        self.repo.set(key, value).await
            .map_err(|e| SettingsServiceError::SaveError(e.to_string()))?;
        Ok(())
    }
    
    /// Set a single setting (alias for set)
    pub async fn set_setting(&self, key: &str, value: &str) -> Result<(), SettingsServiceError> {
        self.set(key, value).await
    }
    
    /// Get all settings as a HashMap
    pub async fn get_all_settings(&self) -> Result<HashMap<String, String>, SettingsServiceError> {
        let settings = self.repo.get_all().await
            .map_err(|e| SettingsServiceError::LoadError(e.to_string()))?;
        Ok(settings.into_iter().map(|s| (s.key, s.value)).collect())
    }
}
