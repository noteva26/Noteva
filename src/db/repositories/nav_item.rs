//! Navigation item repository

use crate::db::DynDatabasePool;
use crate::models::{NavItem, NavItemTree};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{MySqlPool, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait NavItemRepository: Send + Sync {
    async fn create(&self, item: &NavItem) -> Result<NavItem>;
    async fn get_by_id(&self, id: i64) -> Result<Option<NavItem>>;
    async fn list(&self) -> Result<Vec<NavItem>>;
    async fn list_visible(&self) -> Result<Vec<NavItem>>;
    async fn list_tree(&self) -> Result<Vec<NavItemTree>>;
    async fn list_visible_tree(&self) -> Result<Vec<NavItemTree>>;
    async fn update(&self, item: &NavItem) -> Result<NavItem>;
    async fn update_order(&self, id: i64, parent_id: Option<i64>, sort_order: i32) -> Result<()>;
    async fn delete(&self, id: i64) -> Result<()>;
}

pub struct SqlxNavItemRepository {
    pool: DynDatabasePool,
}

impl SqlxNavItemRepository {
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }

    pub fn boxed(pool: DynDatabasePool) -> Arc<dyn NavItemRepository> {
        Arc::new(Self::new(pool))
    }
}

#[async_trait]
impl NavItemRepository for SqlxNavItemRepository {
    async fn create(&self, item: &NavItem) -> Result<NavItem> {
        dispatch!(self, create, item)
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<NavItem>> {
        dispatch!(self, get_by_id, id)
    }

    async fn list(&self) -> Result<Vec<NavItem>> {
        dispatch!(self, list)
    }

    async fn list_visible(&self) -> Result<Vec<NavItem>> {
        dispatch!(self, list_visible)
    }

    async fn list_tree(&self) -> Result<Vec<NavItemTree>> {
        let items = self.list().await?;
        Ok(build_nav_tree(items))
    }

    async fn list_visible_tree(&self) -> Result<Vec<NavItemTree>> {
        let items = self.list_visible().await?;
        Ok(build_nav_tree(items))
    }

    async fn update(&self, item: &NavItem) -> Result<NavItem> {
        dispatch!(self, update, item)
    }

    async fn update_order(&self, id: i64, parent_id: Option<i64>, sort_order: i32) -> Result<()> {
        dispatch!(self, update_order, id, parent_id, sort_order)
    }

    async fn delete(&self, id: i64) -> Result<()> {
        dispatch!(self, delete, id)
    }
}

fn build_nav_tree(items: Vec<NavItem>) -> Vec<NavItemTree> {
    let mut item_map: HashMap<i64, NavItem> = HashMap::new();
    for item in items {
        item_map.insert(item.id, item);
    }

    let mut children_map: HashMap<Option<i64>, Vec<i64>> = HashMap::new();
    for (id, item) in &item_map {
        children_map.entry(item.parent_id).or_default().push(*id);
    }

    for children in children_map.values_mut() {
        children.sort_by(|a, b| {
            let item_a = item_map.get(a).unwrap();
            let item_b = item_map.get(b).unwrap();
            item_a.sort_order.cmp(&item_b.sort_order)
        });
    }

    fn build_subtree(
        parent_id: Option<i64>,
        item_map: &HashMap<i64, NavItem>,
        children_map: &HashMap<Option<i64>, Vec<i64>>,
    ) -> Vec<NavItemTree> {
        let Some(child_ids) = children_map.get(&parent_id) else {
            return Vec::new();
        };
        child_ids
            .iter()
            .filter_map(|id| {
                let item = item_map.get(id)?.clone();
                let children = build_subtree(Some(*id), item_map, children_map);
                Some(NavItemTree::with_children(item, children))
            })
            .collect()
    }

    build_subtree(None, &item_map, &children_map)
}

// ============================================================================
// Shared implementations (identical SQL)
// ============================================================================

impl_dual_fn! {
    async fn list(pool) -> Result<Vec<NavItem>> {
        let rows = sqlx::query("SELECT id, parent_id, title, nav_type, target, open_new_tab, sort_order, visible FROM nav_items ORDER BY sort_order")
            .fetch_all(pool)
            .await
            .context("Failed to list nav items")?;
        rows.iter().map(row_to_nav_item).collect()
    }
}

impl_dual_fn! {
    async fn list_visible(pool) -> Result<Vec<NavItem>> {
        let rows = sqlx::query("SELECT id, parent_id, title, nav_type, target, open_new_tab, sort_order, visible FROM nav_items WHERE visible = 1 ORDER BY sort_order")
            .fetch_all(pool)
            .await
            .context("Failed to list visible nav items")?;
        rows.iter().map(row_to_nav_item).collect()
    }
}

impl_dual_fn! {
    async fn update_order(pool, id: i64, parent_id: Option<i64>, sort_order: i32) -> Result<()> {
        sqlx::query("UPDATE nav_items SET parent_id = ?, sort_order = ? WHERE id = ?")
            .bind(parent_id)
            .bind(sort_order)
            .bind(id)
            .execute(pool)
            .await
            .context("Failed to update nav item order")?;
        Ok(())
    }
}

impl_dual_fn! {
    async fn delete(pool, id: i64) -> Result<()> {
        // Also delete children
        sqlx::query("DELETE FROM nav_items WHERE parent_id = ?").bind(id).execute(pool).await?;
        sqlx::query("DELETE FROM nav_items WHERE id = ?").bind(id).execute(pool).await.context("Failed to delete nav item")?;
        Ok(())
    }
}

// ============================================================================
// Row mapper (shared via generic)
// ============================================================================

fn row_to_nav_item<'r, R>(row: &'r R) -> Result<NavItem>
where
    R: sqlx::Row,
    &'r str: sqlx::ColumnIndex<R>,
    String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    i64: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    Option<i64>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    i32: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    bool: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let nav_type_str: String = row.get("nav_type");
    Ok(NavItem {
        id: row.get("id"),
        parent_id: row.get("parent_id"),
        title: row.get("title"),
        nav_type: nav_type_str.parse().unwrap_or_default(),
        target: row.get("target"),
        open_new_tab: row.get("open_new_tab"),
        sort_order: row.get("sort_order"),
        visible: row.get("visible"),
    })
}

// ============================================================================
// SQLite-specific (last_insert_rowid / references _sqlite fns)
// ============================================================================

async fn create_sqlite(pool: &SqlitePool, item: &NavItem) -> Result<NavItem> {
    let result = sqlx::query(
        "INSERT INTO nav_items (parent_id, title, nav_type, target, open_new_tab, sort_order, visible) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(item.parent_id)
    .bind(&item.title)
    .bind(item.nav_type.to_string())
    .bind(&item.target)
    .bind(item.open_new_tab)
    .bind(item.sort_order)
    .bind(item.visible)
    .execute(pool)
    .await
    .context("Failed to create nav item")?;

    Ok(NavItem {
        id: result.last_insert_rowid(),
        ..item.clone()
    })
}

async fn get_by_id_sqlite(pool: &SqlitePool, id: i64) -> Result<Option<NavItem>> {
    let row = sqlx::query("SELECT id, parent_id, title, nav_type, target, open_new_tab, sort_order, visible FROM nav_items WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get nav item")?;
    Ok(row.map(|r| row_to_nav_item(&r)).transpose()?)
}

async fn update_sqlite(pool: &SqlitePool, item: &NavItem) -> Result<NavItem> {
    sqlx::query("UPDATE nav_items SET parent_id = ?, title = ?, nav_type = ?, target = ?, open_new_tab = ?, sort_order = ?, visible = ? WHERE id = ?")
        .bind(item.parent_id)
        .bind(&item.title)
        .bind(item.nav_type.to_string())
        .bind(&item.target)
        .bind(item.open_new_tab)
        .bind(item.sort_order)
        .bind(item.visible)
        .bind(item.id)
        .execute(pool)
        .await
        .context("Failed to update nav item")?;
    get_by_id_sqlite(pool, item.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Nav item not found after update"))
}

// ============================================================================
// MySQL-specific (last_insert_id / references _mysql fns)
// ============================================================================

async fn create_mysql(pool: &MySqlPool, item: &NavItem) -> Result<NavItem> {
    let result = sqlx::query(
        "INSERT INTO nav_items (parent_id, title, nav_type, target, open_new_tab, sort_order, visible) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(item.parent_id)
    .bind(&item.title)
    .bind(item.nav_type.to_string())
    .bind(&item.target)
    .bind(item.open_new_tab)
    .bind(item.sort_order)
    .bind(item.visible)
    .execute(pool)
    .await
    .context("Failed to create nav item")?;

    Ok(NavItem {
        id: result.last_insert_id() as i64,
        ..item.clone()
    })
}

async fn get_by_id_mysql(pool: &MySqlPool, id: i64) -> Result<Option<NavItem>> {
    let row = sqlx::query("SELECT id, parent_id, title, nav_type, target, open_new_tab, sort_order, visible FROM nav_items WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get nav item")?;
    Ok(row.map(|r| row_to_nav_item(&r)).transpose()?)
}

async fn update_mysql(pool: &MySqlPool, item: &NavItem) -> Result<NavItem> {
    sqlx::query("UPDATE nav_items SET parent_id = ?, title = ?, nav_type = ?, target = ?, open_new_tab = ?, sort_order = ?, visible = ? WHERE id = ?")
        .bind(item.parent_id)
        .bind(&item.title)
        .bind(item.nav_type.to_string())
        .bind(&item.target)
        .bind(item.open_new_tab)
        .bind(item.sort_order)
        .bind(item.visible)
        .bind(item.id)
        .execute(pool)
        .await
        .context("Failed to update nav item")?;
    get_by_id_mysql(pool, item.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Nav item not found after update"))
}
