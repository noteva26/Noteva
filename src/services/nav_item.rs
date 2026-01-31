//! Navigation item service

use crate::db::repositories::NavItemRepository;
use crate::models::{NavItem, NavItemTree, NavItemType, NavOrderItem};
use anyhow::{Context, Result};
use std::sync::Arc;

pub struct NavItemService {
    repo: Arc<dyn NavItemRepository>,
}

impl NavItemService {
    pub fn new(repo: Arc<dyn NavItemRepository>) -> Self {
        Self { repo }
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
        let nav_type: NavItemType = nav_type.parse().unwrap_or_default();
        let mut item = NavItem::new(title, nav_type, target);
        item.parent_id = parent_id;
        item.open_new_tab = open_new_tab;
        item.sort_order = sort_order;
        item.visible = visible;

        self.repo.create(&item).await.context("Failed to create nav item")
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<NavItem>> {
        self.repo.get_by_id(id).await
    }

    pub async fn list(&self) -> Result<Vec<NavItem>> {
        self.repo.list().await
    }

    pub async fn list_tree(&self) -> Result<Vec<NavItemTree>> {
        self.repo.list_tree().await
    }

    pub async fn list_visible_tree(&self) -> Result<Vec<NavItemTree>> {
        self.repo.list_visible_tree().await
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
        let mut item = self.repo.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Nav item not found"))?;

        if let Some(p) = parent_id {
            item.parent_id = p;
        }
        if let Some(t) = title {
            item.title = t;
        }
        if let Some(nt) = nav_type {
            item.nav_type = nt.parse().unwrap_or_default();
        }
        if let Some(tg) = target {
            item.target = tg;
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

        self.repo.update(&item).await
    }

    pub async fn update_order(&self, items: Vec<NavOrderItem>) -> Result<()> {
        for item in items {
            self.repo.update_order(item.id, item.parent_id, item.sort_order).await?;
        }
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        self.repo.delete(id).await
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
            let mut item = NavItem::new(title.to_string(), NavItemType::Builtin, target.to_string());
            item.sort_order = order;
            self.repo.create(&item).await?;
        }

        Ok(())
    }
}
