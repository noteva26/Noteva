//! Page repository

use crate::config::DatabaseDriver;
use crate::db::DynDatabasePool;
use crate::models::{Page, PageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{MySqlPool, Row, SqlitePool};
use std::sync::Arc;

#[async_trait]
pub trait PageRepository: Send + Sync {
    async fn create(&self, page: &Page) -> Result<Page>;
    async fn get_by_id(&self, id: i64) -> Result<Option<Page>>;
    async fn get_by_slug(&self, slug: &str) -> Result<Option<Page>>;
    async fn list(&self) -> Result<Vec<Page>>;
    async fn list_published(&self) -> Result<Vec<Page>>;
    async fn update(&self, page: &Page) -> Result<Page>;
    async fn delete(&self, id: i64) -> Result<()>;
    async fn exists_by_slug(&self, slug: &str) -> Result<bool>;
}

pub struct SqlxPageRepository {
    pool: DynDatabasePool,
}

impl SqlxPageRepository {
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }

    pub fn boxed(pool: DynDatabasePool) -> Arc<dyn PageRepository> {
        Arc::new(Self::new(pool))
    }
}

#[async_trait]
impl PageRepository for SqlxPageRepository {
    async fn create(&self, page: &Page) -> Result<Page> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => create_sqlite(self.pool.as_sqlite().unwrap(), page).await,
            DatabaseDriver::Mysql => create_mysql(self.pool.as_mysql().unwrap(), page).await,
        }
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<Page>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => get_by_id_sqlite(self.pool.as_sqlite().unwrap(), id).await,
            DatabaseDriver::Mysql => get_by_id_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Option<Page>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => get_by_slug_sqlite(self.pool.as_sqlite().unwrap(), slug).await,
            DatabaseDriver::Mysql => get_by_slug_mysql(self.pool.as_mysql().unwrap(), slug).await,
        }
    }

    async fn list(&self) -> Result<Vec<Page>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => list_sqlite(self.pool.as_sqlite().unwrap()).await,
            DatabaseDriver::Mysql => list_mysql(self.pool.as_mysql().unwrap()).await,
        }
    }

    async fn list_published(&self) -> Result<Vec<Page>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => list_published_sqlite(self.pool.as_sqlite().unwrap()).await,
            DatabaseDriver::Mysql => list_published_mysql(self.pool.as_mysql().unwrap()).await,
        }
    }

    async fn update(&self, page: &Page) -> Result<Page> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => update_sqlite(self.pool.as_sqlite().unwrap(), page).await,
            DatabaseDriver::Mysql => update_mysql(self.pool.as_mysql().unwrap(), page).await,
        }
    }

    async fn delete(&self, id: i64) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => delete_sqlite(self.pool.as_sqlite().unwrap(), id).await,
            DatabaseDriver::Mysql => delete_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
    }

    async fn exists_by_slug(&self, slug: &str) -> Result<bool> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => exists_by_slug_sqlite(self.pool.as_sqlite().unwrap(), slug).await,
            DatabaseDriver::Mysql => exists_by_slug_mysql(self.pool.as_mysql().unwrap(), slug).await,
        }
    }
}

// SQLite implementations
async fn create_sqlite(pool: &SqlitePool, page: &Page) -> Result<Page> {
    let now = Utc::now();
    let result = sqlx::query(
        "INSERT INTO pages (slug, title, content, content_html, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&page.slug)
    .bind(&page.title)
    .bind(&page.content)
    .bind(&page.content_html)
    .bind(page.status.to_string())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create page")?;

    Ok(Page {
        id: result.last_insert_rowid(),
        slug: page.slug.clone(),
        title: page.title.clone(),
        content: page.content.clone(),
        content_html: page.content_html.clone(),
        status: page.status.clone(),
        created_at: now,
        updated_at: now,
    })
}

async fn get_by_id_sqlite(pool: &SqlitePool, id: i64) -> Result<Option<Page>> {
    let row = sqlx::query("SELECT id, slug, title, content, content_html, status, created_at, updated_at FROM pages WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get page")?;
    Ok(row.map(|r| row_to_page_sqlite(&r)).transpose()?)
}

async fn get_by_slug_sqlite(pool: &SqlitePool, slug: &str) -> Result<Option<Page>> {
    let row = sqlx::query("SELECT id, slug, title, content, content_html, status, created_at, updated_at FROM pages WHERE slug = ?")
        .bind(slug)
        .fetch_optional(pool)
        .await
        .context("Failed to get page")?;
    Ok(row.map(|r| row_to_page_sqlite(&r)).transpose()?)
}

async fn list_sqlite(pool: &SqlitePool) -> Result<Vec<Page>> {
    let rows = sqlx::query("SELECT id, slug, title, content, content_html, status, created_at, updated_at FROM pages ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
        .context("Failed to list pages")?;
    rows.iter().map(row_to_page_sqlite).collect()
}

async fn list_published_sqlite(pool: &SqlitePool) -> Result<Vec<Page>> {
    let rows = sqlx::query("SELECT id, slug, title, content, content_html, status, created_at, updated_at FROM pages WHERE status = 'published' ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
        .context("Failed to list published pages")?;
    rows.iter().map(row_to_page_sqlite).collect()
}

async fn update_sqlite(pool: &SqlitePool, page: &Page) -> Result<Page> {
    let now = Utc::now();
    sqlx::query("UPDATE pages SET slug = ?, title = ?, content = ?, content_html = ?, status = ?, updated_at = ? WHERE id = ?")
        .bind(&page.slug)
        .bind(&page.title)
        .bind(&page.content)
        .bind(&page.content_html)
        .bind(page.status.to_string())
        .bind(now)
        .bind(page.id)
        .execute(pool)
        .await
        .context("Failed to update page")?;
    get_by_id_sqlite(pool, page.id).await?.ok_or_else(|| anyhow::anyhow!("Page not found after update"))
}

async fn delete_sqlite(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM pages WHERE id = ?").bind(id).execute(pool).await.context("Failed to delete page")?;
    Ok(())
}

async fn exists_by_slug_sqlite(pool: &SqlitePool, slug: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM pages WHERE slug = ?").bind(slug).fetch_one(pool).await?;
    Ok(row.get::<i64, _>("count") > 0)
}

fn row_to_page_sqlite(row: &sqlx::sqlite::SqliteRow) -> Result<Page> {
    let status_str: String = row.get("status");
    Ok(Page {
        id: row.get("id"),
        slug: row.get("slug"),
        title: row.get("title"),
        content: row.get("content"),
        content_html: row.get("content_html"),
        status: status_str.parse().unwrap_or_default(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

// MySQL implementations
async fn create_mysql(pool: &MySqlPool, page: &Page) -> Result<Page> {
    let now = Utc::now();
    let result = sqlx::query(
        "INSERT INTO pages (slug, title, content, content_html, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&page.slug)
    .bind(&page.title)
    .bind(&page.content)
    .bind(&page.content_html)
    .bind(page.status.to_string())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create page")?;

    Ok(Page {
        id: result.last_insert_id() as i64,
        slug: page.slug.clone(),
        title: page.title.clone(),
        content: page.content.clone(),
        content_html: page.content_html.clone(),
        status: page.status.clone(),
        created_at: now,
        updated_at: now,
    })
}

async fn get_by_id_mysql(pool: &MySqlPool, id: i64) -> Result<Option<Page>> {
    let row = sqlx::query("SELECT id, slug, title, content, content_html, status, created_at, updated_at FROM pages WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get page")?;
    Ok(row.map(|r| row_to_page_mysql(&r)).transpose()?)
}

async fn get_by_slug_mysql(pool: &MySqlPool, slug: &str) -> Result<Option<Page>> {
    let row = sqlx::query("SELECT id, slug, title, content, content_html, status, created_at, updated_at FROM pages WHERE slug = ?")
        .bind(slug)
        .fetch_optional(pool)
        .await
        .context("Failed to get page")?;
    Ok(row.map(|r| row_to_page_mysql(&r)).transpose()?)
}

async fn list_mysql(pool: &MySqlPool) -> Result<Vec<Page>> {
    let rows = sqlx::query("SELECT id, slug, title, content, content_html, status, created_at, updated_at FROM pages ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
        .context("Failed to list pages")?;
    rows.iter().map(row_to_page_mysql).collect()
}

async fn list_published_mysql(pool: &MySqlPool) -> Result<Vec<Page>> {
    let rows = sqlx::query("SELECT id, slug, title, content, content_html, status, created_at, updated_at FROM pages WHERE status = 'published' ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
        .context("Failed to list published pages")?;
    rows.iter().map(row_to_page_mysql).collect()
}

async fn update_mysql(pool: &MySqlPool, page: &Page) -> Result<Page> {
    let now = Utc::now();
    sqlx::query("UPDATE pages SET slug = ?, title = ?, content = ?, content_html = ?, status = ?, updated_at = ? WHERE id = ?")
        .bind(&page.slug)
        .bind(&page.title)
        .bind(&page.content)
        .bind(&page.content_html)
        .bind(page.status.to_string())
        .bind(now)
        .bind(page.id)
        .execute(pool)
        .await
        .context("Failed to update page")?;
    get_by_id_mysql(pool, page.id).await?.ok_or_else(|| anyhow::anyhow!("Page not found after update"))
}

async fn delete_mysql(pool: &MySqlPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM pages WHERE id = ?").bind(id).execute(pool).await.context("Failed to delete page")?;
    Ok(())
}

async fn exists_by_slug_mysql(pool: &MySqlPool, slug: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM pages WHERE slug = ?").bind(slug).fetch_one(pool).await?;
    Ok(row.get::<i64, _>("count") > 0)
}

fn row_to_page_mysql(row: &sqlx::mysql::MySqlRow) -> Result<Page> {
    let status_str: String = row.get("status");
    Ok(Page {
        id: row.get("id"),
        slug: row.get("slug"),
        title: row.get("title"),
        content: row.get("content"),
        content_html: row.get("content_html"),
        status: status_str.parse().unwrap_or_default(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
