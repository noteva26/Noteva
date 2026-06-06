//! Built-in public about/profile service.

use std::sync::Arc;

use anyhow::{Context, Result};

use crate::models::AboutProfile;
use crate::services::settings::{SettingsService, SiteSettings};

pub const ABOUT_PROFILE_KEY: &str = "about_profile";

#[derive(Clone)]
pub struct AboutService {
    settings: Arc<SettingsService>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_profile_json_does_not_expose_account_credentials() {
        let profile = AboutProfile {
            enabled: true,
            nav_enabled: true,
            display_name: "Owner".to_string(),
            avatar: "/uploads/avatar.png".to_string(),
            headline: "Writing notes".to_string(),
            bio: "Public bio".to_string(),
            location: "Earth".to_string(),
            website: "https://example.com".to_string(),
            social_links: vec![],
            timeline: vec![],
            extra_markdown: String::new(),
        };

        let value = serde_json::to_value(profile).expect("about profile serializes");
        let object = value.as_object().expect("profile serializes as an object");

        for forbidden in [
            "email",
            "password",
            "password_hash",
            "username",
            "role",
            "totp_secret",
            "recovery_codes",
        ] {
            assert!(
                !object.contains_key(forbidden),
                "about profile must not expose account field {forbidden}"
            );
        }
    }
}

impl AboutService {
    pub fn new(settings: Arc<SettingsService>) -> Self {
        Self { settings }
    }

    pub async fn get_public(&self) -> Result<AboutProfile> {
        let settings = self.settings.get_site_settings().await?;
        let stored = self.load_profile().await?;
        Ok(self.apply_site_fallback(stored, settings).normalize())
    }

    pub async fn get_admin(&self) -> Result<AboutProfile> {
        self.get_public().await
    }

    pub async fn update(&self, input: AboutProfile) -> Result<AboutProfile> {
        let normalized = input.normalize();
        let serialized =
            serde_json::to_string(&normalized).context("failed to serialize about profile")?;
        self.settings
            .set_setting(ABOUT_PROFILE_KEY, &serialized)
            .await?;

        self.settings
            .set_setting(
                "about_nav_enabled",
                if normalized.enabled && normalized.nav_enabled {
                    "true"
                } else {
                    "false"
                },
            )
            .await?;

        Ok(normalized)
    }

    pub async fn is_nav_enabled(&self) -> bool {
        self.get_public()
            .await
            .map(|profile| profile.enabled && profile.nav_enabled)
            .unwrap_or(false)
    }

    async fn load_profile(&self) -> Result<AboutProfile> {
        let value = self.settings.get(ABOUT_PROFILE_KEY).await?;
        let Some(value) = value else {
            return Ok(AboutProfile::default());
        };

        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(AboutProfile::default());
        }

        serde_json::from_str(trimmed)
            .with_context(|| format!("invalid JSON in setting {}", ABOUT_PROFILE_KEY))
    }

    fn apply_site_fallback(
        &self,
        mut profile: AboutProfile,
        settings: SiteSettings,
    ) -> AboutProfile {
        if profile.display_name.trim().is_empty() {
            profile.display_name = settings.site_name;
        }
        if profile.avatar.trim().is_empty() {
            profile.avatar = settings.site_logo;
        }
        if profile.headline.trim().is_empty() {
            profile.headline = settings.site_subtitle;
        }
        if profile.bio.trim().is_empty() {
            profile.bio = settings.site_description;
        }
        profile
    }
}
