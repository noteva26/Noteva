//! Friend link service.

use crate::cache::{Cache, CacheLayer};
use crate::db::repositories::FriendLinkRepository;
use crate::models::{
    CreateFriendLinkInput, FriendLink, FriendLinkOrderItem, FriendLinkStatus, UpdateFriendLinkInput,
};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;

const FRIEND_LINK_CACHE_TTL_SECS: u64 = 3600;
const CACHE_KEY_FRIEND_LINK_LIST: &str = "friend_links:list";
const CACHE_KEY_FRIEND_LINK_PUBLIC: &str = "friend_links:public";

pub struct FriendLinkService {
    repo: Arc<dyn FriendLinkRepository>,
    cache: Arc<Cache>,
    cache_ttl: Duration,
}

impl FriendLinkService {
    pub fn new(repo: Arc<dyn FriendLinkRepository>, cache: Arc<Cache>) -> Self {
        Self {
            repo,
            cache,
            cache_ttl: Duration::from_secs(FRIEND_LINK_CACHE_TTL_SECS),
        }
    }

    pub async fn create(&self, input: CreateFriendLinkInput) -> Result<FriendLink> {
        let mut link = FriendLink::new(
            normalize_required_text(input.name, "Friend link name", 120)?,
            normalize_url(input.url)?,
        );
        link.logo = normalize_optional_url(input.logo, "Logo URL")?;
        link.description = normalize_optional_text(input.description, 500);
        link.category = normalize_optional_text(input.category, 100);
        link.sort_order = input.sort_order.unwrap_or(0);
        link.status = parse_status(input.status)?;
        link.is_recommended = input.is_recommended;

        let created = self
            .repo
            .create(&link)
            .await
            .context("Failed to create friend link")?;
        self.invalidate_cache().await?;
        Ok(created)
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<FriendLink>> {
        self.repo.get_by_id(id).await
    }

    pub async fn list(&self) -> Result<Vec<FriendLink>> {
        if let Ok(Some(links)) = self
            .cache
            .get::<Vec<FriendLink>>(CACHE_KEY_FRIEND_LINK_LIST)
            .await
        {
            return Ok(links);
        }

        let links = self.repo.list().await?;
        let _ = self
            .cache
            .set(CACHE_KEY_FRIEND_LINK_LIST, &links, self.cache_ttl)
            .await;
        Ok(links)
    }

    pub async fn list_public(&self) -> Result<Vec<FriendLink>> {
        if let Ok(Some(links)) = self
            .cache
            .get::<Vec<FriendLink>>(CACHE_KEY_FRIEND_LINK_PUBLIC)
            .await
        {
            return Ok(links);
        }

        let links = self.repo.list_public().await?;
        let _ = self
            .cache
            .set(CACHE_KEY_FRIEND_LINK_PUBLIC, &links, self.cache_ttl)
            .await;
        Ok(links)
    }

    pub async fn update(&self, id: i64, input: UpdateFriendLinkInput) -> Result<FriendLink> {
        let mut link = self
            .repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Friend link not found"))?;

        if let Some(name) = input.name {
            link.name = normalize_required_text(name, "Friend link name", 120)?;
        }
        if let Some(url) = input.url {
            link.url = normalize_url(url)?;
        }
        if let Some(logo) = input.logo {
            link.logo = normalize_optional_url(logo, "Logo URL")?;
        }
        if let Some(description) = input.description {
            link.description = normalize_optional_text(description, 500);
        }
        if let Some(category) = input.category {
            link.category = normalize_optional_text(category, 100);
        }
        if let Some(sort_order) = input.sort_order {
            link.sort_order = sort_order;
        }
        if let Some(status) = input.status {
            link.status = parse_status(Some(status))?;
        }
        if let Some(is_recommended) = input.is_recommended {
            link.is_recommended = is_recommended;
        }

        let updated = self.repo.update(&link).await?;
        self.invalidate_cache().await?;
        Ok(updated)
    }

    pub async fn update_order(&self, items: Vec<FriendLinkOrderItem>) -> Result<()> {
        for item in items {
            self.repo.update_order(item.id, item.sort_order).await?;
        }
        self.invalidate_cache().await?;
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        self.repo.delete(id).await?;
        self.invalidate_cache().await?;
        Ok(())
    }

    async fn invalidate_cache(&self) -> Result<()> {
        let _ = self.cache.delete(CACHE_KEY_FRIEND_LINK_LIST).await;
        let _ = self.cache.delete(CACHE_KEY_FRIEND_LINK_PUBLIC).await;
        Ok(())
    }
}

fn normalize_required_text(value: String, field: &str, max_len: usize) -> Result<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        anyhow::bail!("{} cannot be empty", field);
    }
    if value.chars().count() > max_len {
        anyhow::bail!("{} cannot exceed {} characters", field, max_len);
    }
    Ok(value)
}

fn normalize_optional_text(value: Option<String>, max_len: usize) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .map(|text| text.chars().take(max_len).collect())
}

fn normalize_url(value: String) -> Result<String> {
    let value = value.trim().to_string();
    validate_http_url(&value, "URL")?;
    Ok(value)
}

fn normalize_optional_url(value: Option<String>, field: &str) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    validate_http_url(&value, field)?;
    Ok(Some(value))
}

fn validate_http_url(value: &str, field: &str) -> Result<()> {
    let parsed =
        reqwest::Url::parse(value).with_context(|| format!("{} must be a valid URL", field))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        anyhow::bail!("{} must start with http:// or https://", field);
    }
    Ok(())
}

fn parse_status(value: Option<String>) -> Result<FriendLinkStatus> {
    value
        .map(|status| status.parse::<FriendLinkStatus>())
        .transpose()
        .map(|status| status.unwrap_or(FriendLinkStatus::Approved))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_http_urls_are_accepted() {
        assert!(normalize_url("https://example.com".to_string()).is_ok());
        assert!(normalize_url("http://example.com/path".to_string()).is_ok());
    }

    #[test]
    fn non_http_urls_are_rejected() {
        assert!(normalize_url("javascript:alert(1)".to_string()).is_err());
        assert!(normalize_url("mailto:test@example.com".to_string()).is_err());
        assert!(normalize_url("/relative".to_string()).is_err());
    }

    #[test]
    fn status_parser_rejects_unknown_status() {
        assert!(parse_status(Some("approved".to_string())).is_ok());
        assert!(parse_status(Some("unknown".to_string())).is_err());
    }
}
