//! Friend link repository.

use crate::db::DynDatabasePool;
use crate::models::{FriendLink, FriendLinkStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{MySqlPool, SqlitePool};
use std::sync::Arc;

#[async_trait]
pub trait FriendLinkRepository: Send + Sync {
    async fn create(&self, link: &FriendLink) -> Result<FriendLink>;
    async fn get_by_id(&self, id: i64) -> Result<Option<FriendLink>>;
    async fn list(&self) -> Result<Vec<FriendLink>>;
    async fn list_public(&self) -> Result<Vec<FriendLink>>;
    async fn update(&self, link: &FriendLink) -> Result<FriendLink>;
    async fn update_order(&self, id: i64, sort_order: i32) -> Result<()>;
    async fn delete(&self, id: i64) -> Result<()>;
}

pub struct SqlxFriendLinkRepository {
    pool: DynDatabasePool,
}

impl SqlxFriendLinkRepository {
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }

    pub fn boxed(pool: DynDatabasePool) -> Arc<dyn FriendLinkRepository> {
        Arc::new(Self::new(pool))
    }
}

#[async_trait]
impl FriendLinkRepository for SqlxFriendLinkRepository {
    async fn create(&self, link: &FriendLink) -> Result<FriendLink> {
        dispatch!(self, create, link)
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<FriendLink>> {
        dispatch!(self, get_by_id, id)
    }

    async fn list(&self) -> Result<Vec<FriendLink>> {
        dispatch!(self, list)
    }

    async fn list_public(&self) -> Result<Vec<FriendLink>> {
        dispatch!(self, list_public)
    }

    async fn update(&self, link: &FriendLink) -> Result<FriendLink> {
        dispatch!(self, update, link)
    }

    async fn update_order(&self, id: i64, sort_order: i32) -> Result<()> {
        dispatch!(self, update_order, id, sort_order)
    }

    async fn delete(&self, id: i64) -> Result<()> {
        dispatch!(self, delete, id)
    }
}

impl_dual_fn! {
    async fn list(pool) -> Result<Vec<FriendLink>> {
        let rows = sqlx::query(
            "SELECT id, name, url, logo, description, category, sort_order, status, is_recommended, created_at, updated_at FROM friend_links ORDER BY category, sort_order, name"
        )
        .fetch_all(pool)
        .await
        .context("Failed to list friend links")?;
        rows.iter().map(row_to_friend_link).collect()
    }
}

impl_dual_fn! {
    async fn list_public(pool) -> Result<Vec<FriendLink>> {
        let rows = sqlx::query(
            "SELECT id, name, url, logo, description, category, sort_order, status, is_recommended, created_at, updated_at FROM friend_links WHERE status = 'approved' ORDER BY category, sort_order, name"
        )
        .fetch_all(pool)
        .await
        .context("Failed to list public friend links")?;
        rows.iter().map(row_to_friend_link).collect()
    }
}

impl_dual_fn! {
    async fn update_order(pool, id: i64, sort_order: i32) -> Result<()> {
        sqlx::query("UPDATE friend_links SET sort_order = ?, updated_at = ? WHERE id = ?")
            .bind(sort_order)
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await
            .context("Failed to update friend link order")?;
        Ok(())
    }
}

impl_dual_fn! {
    async fn delete(pool, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM friend_links WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .context("Failed to delete friend link")?;
        Ok(())
    }
}

fn row_to_friend_link<'r, R>(row: &'r R) -> Result<FriendLink>
where
    R: sqlx::Row,
    &'r str: sqlx::ColumnIndex<R>,
    String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    i64: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    i32: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    bool: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    chrono::DateTime<Utc>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let status: String = row.get("status");
    Ok(FriendLink {
        id: row.get("id"),
        name: row.get("name"),
        url: row.get("url"),
        logo: row.get("logo"),
        description: row.get("description"),
        category: row.get("category"),
        sort_order: row.get("sort_order"),
        status: status.parse().unwrap_or(FriendLinkStatus::Approved),
        is_recommended: row.get("is_recommended"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

async fn create_sqlite(pool: &SqlitePool, link: &FriendLink) -> Result<FriendLink> {
    let now = Utc::now();
    let result = sqlx::query(
        "INSERT INTO friend_links (name, url, logo, description, category, sort_order, status, is_recommended, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&link.name)
    .bind(&link.url)
    .bind(&link.logo)
    .bind(&link.description)
    .bind(&link.category)
    .bind(link.sort_order)
    .bind(link.status.to_string())
    .bind(link.is_recommended)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create friend link")?;

    Ok(FriendLink {
        id: result.last_insert_rowid(),
        created_at: now,
        updated_at: now,
        ..link.clone()
    })
}

async fn get_by_id_sqlite(pool: &SqlitePool, id: i64) -> Result<Option<FriendLink>> {
    let row = sqlx::query(
        "SELECT id, name, url, logo, description, category, sort_order, status, is_recommended, created_at, updated_at FROM friend_links WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get friend link")?;
    Ok(row.map(|r| row_to_friend_link(&r)).transpose()?)
}

async fn update_sqlite(pool: &SqlitePool, link: &FriendLink) -> Result<FriendLink> {
    let now = Utc::now();
    sqlx::query(
        "UPDATE friend_links SET name = ?, url = ?, logo = ?, description = ?, category = ?, sort_order = ?, status = ?, is_recommended = ?, updated_at = ? WHERE id = ?"
    )
    .bind(&link.name)
    .bind(&link.url)
    .bind(&link.logo)
    .bind(&link.description)
    .bind(&link.category)
    .bind(link.sort_order)
    .bind(link.status.to_string())
    .bind(link.is_recommended)
    .bind(now)
    .bind(link.id)
    .execute(pool)
    .await
    .context("Failed to update friend link")?;
    get_by_id_sqlite(pool, link.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Friend link not found after update"))
}

async fn create_mysql(pool: &MySqlPool, link: &FriendLink) -> Result<FriendLink> {
    let now = Utc::now();
    let result = sqlx::query(
        "INSERT INTO friend_links (name, url, logo, description, category, sort_order, status, is_recommended, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&link.name)
    .bind(&link.url)
    .bind(&link.logo)
    .bind(&link.description)
    .bind(&link.category)
    .bind(link.sort_order)
    .bind(link.status.to_string())
    .bind(link.is_recommended)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create friend link")?;

    Ok(FriendLink {
        id: result.last_insert_id() as i64,
        created_at: now,
        updated_at: now,
        ..link.clone()
    })
}

async fn get_by_id_mysql(pool: &MySqlPool, id: i64) -> Result<Option<FriendLink>> {
    let row = sqlx::query(
        "SELECT id, name, url, logo, description, category, sort_order, status, is_recommended, created_at, updated_at FROM friend_links WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get friend link")?;
    Ok(row.map(|r| row_to_friend_link(&r)).transpose()?)
}

async fn update_mysql(pool: &MySqlPool, link: &FriendLink) -> Result<FriendLink> {
    let now = Utc::now();
    sqlx::query(
        "UPDATE friend_links SET name = ?, url = ?, logo = ?, description = ?, category = ?, sort_order = ?, status = ?, is_recommended = ?, updated_at = ? WHERE id = ?"
    )
    .bind(&link.name)
    .bind(&link.url)
    .bind(&link.logo)
    .bind(&link.description)
    .bind(&link.category)
    .bind(link.sort_order)
    .bind(link.status.to_string())
    .bind(link.is_recommended)
    .bind(now)
    .bind(link.id)
    .execute(pool)
    .await
    .context("Failed to update friend link")?;
    get_by_id_mysql(pool, link.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Friend link not found after update"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_test_pool, migrations};

    async fn setup_test_repo() -> (crate::db::DynDatabasePool, SqlxFriendLinkRepository) {
        let pool = create_test_pool()
            .await
            .expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");
        let repo = SqlxFriendLinkRepository::new(pool.clone());
        (pool, repo)
    }

    fn test_link(name: &str, url: &str, status: FriendLinkStatus, sort_order: i32) -> FriendLink {
        let mut link = FriendLink::new(name.to_string(), url.to_string());
        link.category = Some("Friends".to_string());
        link.status = status;
        link.sort_order = sort_order;
        link
    }

    #[tokio::test]
    async fn list_public_only_returns_approved_links_in_sort_order() {
        let (_pool, repo) = setup_test_repo().await;

        repo.create(&test_link(
            "Visible B",
            "https://visible-b.example.com",
            FriendLinkStatus::Approved,
            20,
        ))
        .await
        .unwrap();
        repo.create(&test_link(
            "Hidden",
            "https://hidden.example.com",
            FriendLinkStatus::Hidden,
            5,
        ))
        .await
        .unwrap();
        repo.create(&test_link(
            "Visible A",
            "https://visible-a.example.com",
            FriendLinkStatus::Approved,
            10,
        ))
        .await
        .unwrap();
        repo.create(&test_link(
            "Rejected",
            "https://rejected.example.com",
            FriendLinkStatus::Rejected,
            1,
        ))
        .await
        .unwrap();

        let links = repo.list_public().await.unwrap();

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].name, "Visible A");
        assert_eq!(links[1].name, "Visible B");
        assert!(links
            .iter()
            .all(|link| link.status == FriendLinkStatus::Approved));
    }
}
