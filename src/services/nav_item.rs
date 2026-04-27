//! Navigation item service

use crate::cache::{Cache, CacheLayer};
use crate::db::repositories::NavItemRepository;
use crate::models::{NavItem, NavItemTree, NavItemType, NavOrderItem};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;

/// Default cache TTL for navigation (1 day - navigation rarely changes)
const NAV_CACHE_TTL_SECS: u64 = 86400;

/// Cache key prefixes
const CACHE_KEY_NAV_LIST: &str = "nav:list";
const CACHE_KEY_NAV_TREE: &str = "nav:tree";
const CACHE_KEY_NAV_VISIBLE_TREE: &str = "nav:visible:tree";

fn parse_nav_type(value: &str) -> Result<NavItemType> {
    value
        .parse()
        .with_context(|| format!("Invalid nav item type: {}", value))
}

fn validate_nav_target(nav_type: &NavItemType, target: &str) -> Result<()> {
    let trimmed = target.trim();
    if trimmed.chars().any(char::is_control) {
        anyhow::bail!("Navigation target contains invalid characters");
    }

    match nav_type {
        NavItemType::Builtin => {
            if trimmed.is_empty() || matches!(trimmed, "home" | "archives" | "categories" | "tags")
            {
                Ok(())
            } else {
                anyhow::bail!("Invalid built-in navigation target: {}", trimmed)
            }
        }
        NavItemType::Page => {
            if trimmed.is_empty() {
                anyhow::bail!("Page navigation target cannot be empty");
            }
            Ok(())
        }
        NavItemType::External => {
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("http://")
                || lower.starts_with("https://")
                || lower.starts_with("mailto:")
                || lower.starts_with("tel:")
            {
                Ok(())
            } else {
                anyhow::bail!(
                    "External navigation URL must start with http://, https://, mailto:, or tel:"
                )
            }
        }
    }
}

pub struct NavItemService {
    repo: Arc<dyn NavItemRepository>,
    cache: Arc<Cache>,
    cache_ttl: Duration,
}

impl NavItemService {
    pub fn new(repo: Arc<dyn NavItemRepository>, cache: Arc<Cache>) -> Self {
        Self {
            repo,
            cache,
            cache_ttl: Duration::from_secs(NAV_CACHE_TTL_SECS),
        }
    }

    pub async fn create(
        &self,
        parent_id: Option<i64>,
        title: String,
        nav_type: String,
        target: String,
        open_new_tab: bool,
        sort_order: i32,
        visible: bool,
    ) -> Result<NavItem> {
        let nav_type = parse_nav_type(&nav_type)?;
        let target = target.trim().to_string();
        validate_nav_target(&nav_type, &target)?;

        let mut item = NavItem::new(title, nav_type, target);
        item.parent_id = parent_id;
        item.open_new_tab = open_new_tab;
        item.sort_order = sort_order;
        item.visible = visible;

        let created = self
            .repo
            .create(&item)
            .await
            .context("Failed to create nav item")?;

        // Invalidate cache - CRITICAL: must clear all nav caches
        self.invalidate_cache().await?;

        Ok(created)
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<NavItem>> {
        self.repo.get_by_id(id).await
    }

    pub async fn list(&self) -> Result<Vec<NavItem>> {
        // Try cache first
        if let Ok(Some(items)) = self.cache.get::<Vec<NavItem>>(CACHE_KEY_NAV_LIST).await {
            return Ok(items);
        }

        // Get from database
        let items = self.repo.list().await?;

        // Cache the result
        let _ = self
            .cache
            .set(CACHE_KEY_NAV_LIST, &items, self.cache_ttl)
            .await;

        Ok(items)
    }

    pub async fn list_tree(&self) -> Result<Vec<NavItemTree>> {
        // Try cache first
        if let Ok(Some(tree)) = self.cache.get::<Vec<NavItemTree>>(CACHE_KEY_NAV_TREE).await {
            return Ok(tree);
        }

        // Get from database
        let tree = self.repo.list_tree().await?;

        // Cache the result
        let _ = self
            .cache
            .set(CACHE_KEY_NAV_TREE, &tree, self.cache_ttl)
            .await;

        Ok(tree)
    }

    pub async fn list_visible_tree(&self) -> Result<Vec<NavItemTree>> {
        // Try cache first
        if let Ok(Some(tree)) = self
            .cache
            .get::<Vec<NavItemTree>>(CACHE_KEY_NAV_VISIBLE_TREE)
            .await
        {
            return Ok(tree);
        }

        // Get from database
        let tree = self.repo.list_visible_tree().await?;

        // Cache the result
        let _ = self
            .cache
            .set(CACHE_KEY_NAV_VISIBLE_TREE, &tree, self.cache_ttl)
            .await;

        Ok(tree)
    }

    pub async fn update(
        &self,
        id: i64,
        parent_id: Option<Option<i64>>,
        title: Option<String>,
        nav_type: Option<String>,
        target: Option<String>,
        open_new_tab: Option<bool>,
        sort_order: Option<i32>,
        visible: Option<bool>,
    ) -> Result<NavItem> {
        let mut item = self
            .repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Nav item not found"))?;

        if let Some(p) = parent_id {
            item.parent_id = p;
        }
        if let Some(t) = title {
            item.title = t;
        }
        if let Some(nt) = nav_type {
            item.nav_type = parse_nav_type(&nt)?;
        }
        if let Some(tg) = target {
            item.target = tg.trim().to_string();
        }
        if let Some(o) = open_new_tab {
            item.open_new_tab = o;
        }
        if let Some(s) = sort_order {
            item.sort_order = s;
        }
        if let Some(v) = visible {
            item.visible = v;
        }

        validate_nav_target(&item.nav_type, &item.target)?;

        let updated = self.repo.update(&item).await?;

        // Invalidate cache - CRITICAL: must clear all nav caches
        self.invalidate_cache().await?;

        Ok(updated)
    }

    pub async fn update_order(&self, items: Vec<NavOrderItem>) -> Result<()> {
        for item in items {
            self.repo
                .update_order(item.id, item.parent_id, item.sort_order)
                .await?;
        }

        // Invalidate cache - CRITICAL: must clear all nav caches
        self.invalidate_cache().await?;

        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        self.repo.delete(id).await?;

        // Invalidate cache - CRITICAL: must clear all nav caches
        self.invalidate_cache().await?;

        Ok(())
    }

    /// Invalidate all navigation caches
    async fn invalidate_cache(&self) -> Result<()> {
        let _ = self.cache.delete(CACHE_KEY_NAV_LIST).await;
        let _ = self.cache.delete(CACHE_KEY_NAV_TREE).await;
        let _ = self.cache.delete(CACHE_KEY_NAV_VISIBLE_TREE).await;
        Ok(())
    }

    /// Initialize default navigation items if none exist
    pub async fn init_defaults(&self) -> Result<()> {
        let items = self.repo.list().await?;
        if !items.is_empty() {
            return Ok(());
        }

        // Create default builtin nav items
        let defaults = [
            ("首页", "home", 0),
            ("归档", "archives", 1),
            ("分类", "categories", 2),
            ("标签", "tags", 3),
        ];

        for (title, target, order) in defaults {
            let mut item =
                NavItem::new(title.to_string(), NavItemType::Builtin, target.to_string());
            item.sort_order = order;
            self.repo.create(&item).await?;
        }

        Ok(())
    }
}
