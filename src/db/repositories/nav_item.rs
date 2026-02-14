//! Navigation item repository

use crate::config::DatabaseDriver;
use crate::db::DynDatabasePool;
use crate::models::{NavItem, NavItemTree};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{MySqlPool, Row, SqlitePool};
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
        match self.pool.driver() {
            DatabaseDriver::Sqlite => create_sqlite(self.pool.as_sqlite().unwrap(), item).await,
            DatabaseDriver::Mysql => create_mysql(self.pool.as_mysql().unwrap(), item).await,
        }
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<NavItem>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => get_by_id_sqlite(self.pool.as_sqlite().unwrap(), id).await,
            DatabaseDriver::Mysql => get_by_id_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
    }

    async fn list(&self) -> Result<Vec<NavItem>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => list_sqlite(self.pool.as_sqlite().unwrap()).await,
            DatabaseDriver::Mysql => list_mysql(self.pool.as_mysql().unwrap()).await,
        }
    }

    async fn list_visible(&self) -> Result<Vec<NavItem>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => list_visible_sqlite(self.pool.as_sqlite().unwrap()).await,
            DatabaseDriver::Mysql => list_visible_mysql(self.pool.as_mysql().unwrap()).await,
        }
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
        match self.pool.driver() {
            DatabaseDriver::Sqlite => update_sqlite(self.pool.as_sqlite().unwrap(), item).await,
            DatabaseDriver::Mysql => update_mysql(self.pool.as_mysql().unwrap(), item).await,
        }
    }

    async fn update_order(&self, id: i64, parent_id: Option<i64>, sort_order: i32) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => update_order_sqlite(self.pool.as_sqlite().unwrap(), id, parent_id, sort_order).await,
            DatabaseDriver::Mysql => update_order_mysql(self.pool.as_mysql().unwrap(), id, parent_id, sort_order).await,
        }
    }

    async fn delete(&self, id: i64) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => delete_sqlite(self.pool.as_sqlite().unwrap(), id).await,
            DatabaseDriver::Mysql => delete_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
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

// SQLite implementations
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
        parent_id: item.parent_id,
        title: item.title.clone(),
        nav_type: item.nav_type.clone(),
        target: item.target.clone(),
        open_new_tab: item.open_new_tab,
        sort_order: item.sort_order,
        visible: item.visible,
    })
}

async fn get_by_id_sqlite(pool: &SqlitePool, id: i64) -> Result<Option<NavItem>> {
    let row = sqlx::query("SELECT id, parent_id, title, nav_type, target, open_new_tab, sort_order, visible FROM nav_items WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get nav item")?;
    Ok(row.map(|r| row_to_nav_item_sqlite(&r)).transpose()?)
}

async fn list_sqlite(pool: &SqlitePool) -> Result<Vec<NavItem>> {
    let rows = sqlx::query("SELECT id, parent_id, title, nav_type, target, open_new_tab, sort_order, visible FROM nav_items ORDER BY sort_order")
        .fetch_all(pool)
        .await
        .context("Failed to list nav items")?;
    rows.iter().map(row_to_nav_item_sqlite).collect()
}

async fn list_visible_sqlite(pool: &SqlitePool) -> Result<Vec<NavItem>> {
    let rows = sqlx::query("SELECT id, parent_id, title, nav_type, target, open_new_tab, sort_order, visible FROM nav_items WHERE visible = 1 ORDER BY sort_order")
        .fetch_all(pool)
        .await
        .context("Failed to list visible nav items")?;
    rows.iter().map(row_to_nav_item_sqlite).collect()
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
    get_by_id_sqlite(pool, item.id).await?.ok_or_else(|| anyhow::anyhow!("Nav item not found after update"))
}

async fn update_order_sqlite(pool: &SqlitePool, id: i64, parent_id: Option<i64>, sort_order: i32) -> Result<()> {
    sqlx::query("UPDATE nav_items SET parent_id = ?, sort_order = ? WHERE id = ?")
        .bind(parent_id)
        .bind(sort_order)
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to update nav item order")?;
    Ok(())
}

async fn delete_sqlite(pool: &SqlitePool, id: i64) -> Result<()> {
    // Also delete children
    sqlx::query("DELETE FROM nav_items WHERE parent_id = ?").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM nav_items WHERE id = ?").bind(id).execute(pool).await.context("Failed to delete nav item")?;
    Ok(())
}

fn row_to_nav_item_sqlite(row: &sqlx::sqlite::SqliteRow) -> Result<NavItem> {
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

// MySQL implementations
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
        parent_id: item.parent_id,
        title: item.title.clone(),
        nav_type: item.nav_type.clone(),
        target: item.target.clone(),
        open_new_tab: item.open_new_tab,
        sort_order: item.sort_order,
        visible: item.visible,
    })
}

async fn get_by_id_mysql(pool: &MySqlPool, id: i64) -> Result<Option<NavItem>> {
    let row = sqlx::query("SELECT id, parent_id, title, nav_type, target, open_new_tab, sort_order, visible FROM nav_items WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get nav item")?;
    Ok(row.map(|r| row_to_nav_item_mysql(&r)).transpose()?)
}

async fn list_mysql(pool: &MySqlPool) -> Result<Vec<NavItem>> {
    let rows = sqlx::query("SELECT id, parent_id, title, nav_type, target, open_new_tab, sort_order, visible FROM nav_items ORDER BY sort_order")
        .fetch_all(pool)
        .await
        .context("Failed to list nav items")?;
    rows.iter().map(row_to_nav_item_mysql).collect()
}

async fn list_visible_mysql(pool: &MySqlPool) -> Result<Vec<NavItem>> {
    let rows = sqlx::query("SELECT id, parent_id, title, nav_type, target, open_new_tab, sort_order, visible FROM nav_items WHERE visible = 1 ORDER BY sort_order")
        .fetch_all(pool)
        .await
        .context("Failed to list visible nav items")?;
    rows.iter().map(row_to_nav_item_mysql).collect()
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
    get_by_id_mysql(pool, item.id).await?.ok_or_else(|| anyhow::anyhow!("Nav item not found after update"))
}

async fn update_order_mysql(pool: &MySqlPool, id: i64, parent_id: Option<i64>, sort_order: i32) -> Result<()> {
    sqlx::query("UPDATE nav_items SET parent_id = ?, sort_order = ? WHERE id = ?")
        .bind(parent_id)
        .bind(sort_order)
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to update nav item order")?;
    Ok(())
}

async fn delete_mysql(pool: &MySqlPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM nav_items WHERE parent_id = ?").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM nav_items WHERE id = ?").bind(id).execute(pool).await.context("Failed to delete nav item")?;
    Ok(())
}

fn row_to_nav_item_mysql(row: &sqlx::mysql::MySqlRow) -> Result<NavItem> {
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
