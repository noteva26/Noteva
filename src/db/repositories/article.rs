//! Article repository
//!
//! Database operations for articles.
//!
//! This module provides:
//! - `ArticleRepository` trait defining the interface for article data access
//! - `SqlxArticleRepository` implementing the trait for SQLite and MySQL
//!
//! Satisfies requirements:
//! - 1.1: WHEN 用户提交新文�?THEN Article_Manager SHALL 创建文章记录并生成唯一标识�?
//! - 1.2: WHEN 用户请求文章列表 THEN Article_Manager SHALL 返回分页的文章列表，支持按时间排�?

use crate::config::DatabaseDriver;
use crate::db::DynDatabasePool;
use crate::models::{Article, ArticleStatus, CreateArticleInput, UpdateArticleInput};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{MySqlPool, Row, SqlitePool};
use std::sync::Arc;

/// Article repository trait
#[async_trait]
pub trait ArticleRepository: Send + Sync {
    /// Create a new article
    async fn create(&self, input: &CreateArticleInput) -> Result<Article>;

    /// Get article by ID
    async fn get_by_id(&self, id: i64) -> Result<Option<Article>>;

    /// Get article by slug
    async fn get_by_slug(&self, slug: &str) -> Result<Option<Article>>;

    /// List articles with pagination (all statuses)
    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<Article>>;

    /// Count total articles (all statuses)
    async fn count(&self) -> Result<i64>;

    /// Update an article
    async fn update(&self, id: i64, input: &UpdateArticleInput) -> Result<Article>;

    /// Delete an article
    async fn delete(&self, id: i64) -> Result<()>;

    /// List articles by category with pagination
    async fn list_by_category(&self, category_id: i64, offset: i64, limit: i64) -> Result<Vec<Article>>;

    /// List articles by tag with pagination
    async fn list_by_tag(&self, tag_id: i64, offset: i64, limit: i64) -> Result<Vec<Article>>;

    /// List only published articles with pagination (ordered by published_at DESC)
    async fn list_published(&self, offset: i64, limit: i64) -> Result<Vec<Article>>;

    /// Count published articles
    async fn count_published(&self) -> Result<i64>;

    /// Count articles in a category
    async fn count_by_category(&self, category_id: i64) -> Result<i64>;

    /// Count articles with a tag
    async fn count_by_tag(&self, tag_id: i64) -> Result<i64>;

    /// Check if a slug already exists
    async fn exists_by_slug(&self, slug: &str) -> Result<bool>;

    /// Check if a slug exists for a different article (for updates)
    async fn exists_by_slug_excluding(&self, slug: &str, exclude_id: i64) -> Result<bool>;

    /// Search articles by keyword in title and content
    async fn search(&self, keyword: &str, offset: i64, limit: i64, published_only: bool) -> Result<Vec<Article>>;

    /// Count search results
    async fn count_search(&self, keyword: &str, published_only: bool) -> Result<i64>;
}

/// SQLx-based article repository implementation
///
/// Supports both SQLite and MySQL databases.
pub struct SqlxArticleRepository {
    pool: DynDatabasePool,
}

impl SqlxArticleRepository {
    /// Create a new SQLx article repository
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }

    /// Create a boxed repository for use with dependency injection
    pub fn boxed(pool: DynDatabasePool) -> Arc<dyn ArticleRepository> {
        Arc::new(Self::new(pool))
    }
}

#[async_trait]
impl ArticleRepository for SqlxArticleRepository {
    async fn create(&self, input: &CreateArticleInput) -> Result<Article> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                create_article_sqlite(self.pool.as_sqlite().unwrap(), input).await
            }
            DatabaseDriver::Mysql => {
                create_article_mysql(self.pool.as_mysql().unwrap(), input).await
            }
        }
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<Article>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_article_by_id_sqlite(self.pool.as_sqlite().unwrap(), id).await
            }
            DatabaseDriver::Mysql => {
                get_article_by_id_mysql(self.pool.as_mysql().unwrap(), id).await
            }
        }
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Option<Article>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_article_by_slug_sqlite(self.pool.as_sqlite().unwrap(), slug).await
            }
            DatabaseDriver::Mysql => {
                get_article_by_slug_mysql(self.pool.as_mysql().unwrap(), slug).await
            }
        }
    }

    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<Article>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                list_articles_sqlite(self.pool.as_sqlite().unwrap(), offset, limit).await
            }
            DatabaseDriver::Mysql => {
                list_articles_mysql(self.pool.as_mysql().unwrap(), offset, limit).await
            }
        }
    }

    async fn count(&self) -> Result<i64> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => count_articles_sqlite(self.pool.as_sqlite().unwrap()).await,
            DatabaseDriver::Mysql => count_articles_mysql(self.pool.as_mysql().unwrap()).await,
        }
    }

    async fn update(&self, id: i64, input: &UpdateArticleInput) -> Result<Article> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                update_article_sqlite(self.pool.as_sqlite().unwrap(), id, input).await
            }
            DatabaseDriver::Mysql => {
                update_article_mysql(self.pool.as_mysql().unwrap(), id, input).await
            }
        }
    }

    async fn delete(&self, id: i64) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => delete_article_sqlite(self.pool.as_sqlite().unwrap(), id).await,
            DatabaseDriver::Mysql => delete_article_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
    }

    async fn list_by_category(&self, category_id: i64, offset: i64, limit: i64) -> Result<Vec<Article>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                list_articles_by_category_sqlite(self.pool.as_sqlite().unwrap(), category_id, offset, limit).await
            }
            DatabaseDriver::Mysql => {
                list_articles_by_category_mysql(self.pool.as_mysql().unwrap(), category_id, offset, limit).await
            }
        }
    }

    async fn list_by_tag(&self, tag_id: i64, offset: i64, limit: i64) -> Result<Vec<Article>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                list_articles_by_tag_sqlite(self.pool.as_sqlite().unwrap(), tag_id, offset, limit).await
            }
            DatabaseDriver::Mysql => {
                list_articles_by_tag_mysql(self.pool.as_mysql().unwrap(), tag_id, offset, limit).await
            }
        }
    }

    async fn list_published(&self, offset: i64, limit: i64) -> Result<Vec<Article>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                list_published_articles_sqlite(self.pool.as_sqlite().unwrap(), offset, limit).await
            }
            DatabaseDriver::Mysql => {
                list_published_articles_mysql(self.pool.as_mysql().unwrap(), offset, limit).await
            }
        }
    }

    async fn count_published(&self) -> Result<i64> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => count_published_sqlite(self.pool.as_sqlite().unwrap()).await,
            DatabaseDriver::Mysql => count_published_mysql(self.pool.as_mysql().unwrap()).await,
        }
    }

    async fn count_by_category(&self, category_id: i64) -> Result<i64> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                count_by_category_sqlite(self.pool.as_sqlite().unwrap(), category_id).await
            }
            DatabaseDriver::Mysql => {
                count_by_category_mysql(self.pool.as_mysql().unwrap(), category_id).await
            }
        }
    }

    async fn count_by_tag(&self, tag_id: i64) -> Result<i64> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                count_by_tag_sqlite(self.pool.as_sqlite().unwrap(), tag_id).await
            }
            DatabaseDriver::Mysql => {
                count_by_tag_mysql(self.pool.as_mysql().unwrap(), tag_id).await
            }
        }
    }

    async fn exists_by_slug(&self, slug: &str) -> Result<bool> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                exists_by_slug_sqlite(self.pool.as_sqlite().unwrap(), slug).await
            }
            DatabaseDriver::Mysql => {
                exists_by_slug_mysql(self.pool.as_mysql().unwrap(), slug).await
            }
        }
    }

    async fn exists_by_slug_excluding(&self, slug: &str, exclude_id: i64) -> Result<bool> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                exists_by_slug_excluding_sqlite(self.pool.as_sqlite().unwrap(), slug, exclude_id).await
            }
            DatabaseDriver::Mysql => {
                exists_by_slug_excluding_mysql(self.pool.as_mysql().unwrap(), slug, exclude_id).await
            }
        }
    }

    async fn search(&self, keyword: &str, offset: i64, limit: i64, published_only: bool) -> Result<Vec<Article>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                search_articles_sqlite(self.pool.as_sqlite().unwrap(), keyword, offset, limit, published_only).await
            }
            DatabaseDriver::Mysql => {
                search_articles_mysql(self.pool.as_mysql().unwrap(), keyword, offset, limit, published_only).await
            }
        }
    }

    async fn count_search(&self, keyword: &str, published_only: bool) -> Result<i64> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                count_search_sqlite(self.pool.as_sqlite().unwrap(), keyword, published_only).await
            }
            DatabaseDriver::Mysql => {
                count_search_mysql(self.pool.as_mysql().unwrap(), keyword, published_only).await
            }
        }
    }
}


// ============================================================================
// SQLite implementations
// ============================================================================

async fn create_article_sqlite(pool: &SqlitePool, input: &CreateArticleInput) -> Result<Article> {
    let now = Utc::now();
    let status = input.status.unwrap_or_default();
    let published_at = if status == ArticleStatus::Published {
        Some(now)
    } else {
        None
    };
    let content_html = input.content_html.clone().unwrap_or_default();

    let result = sqlx::query(
        r#"
        INSERT INTO articles (slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, thumbnail, is_pinned, pin_order)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&input.slug)
    .bind(&input.title)
    .bind(&input.content)
    .bind(&content_html)
    .bind(input.author_id)
    .bind(input.category_id)
    .bind(status.as_str())
    .bind(published_at)
    .bind(now)
    .bind(now)
    .bind::<Option<&str>>(None)
    .bind(false)
    .bind(0)
    .execute(pool)
    .await
    .context("Failed to create article")?;

    let id = result.last_insert_rowid();

    Ok(Article {
        id,
        slug: input.slug.clone(),
        title: input.title.clone(),
        content: input.content.clone(),
        content_html,
        author_id: input.author_id,
        category_id: input.category_id,
        status,
        published_at,
        created_at: now,
        updated_at: now,
        view_count: 0,
        like_count: 0,
        comment_count: 0,
        thumbnail: None,
        is_pinned: false,
        pin_order: 0,
    })
}

async fn get_article_by_id_sqlite(pool: &SqlitePool, id: i64) -> Result<Option<Article>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get article by ID")?;

    match row {
        Some(row) => Ok(Some(row_to_article_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn get_article_by_slug_sqlite(pool: &SqlitePool, slug: &str) -> Result<Option<Article>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE slug = ?
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .context("Failed to get article by slug")?;

    match row {
        Some(row) => Ok(Some(row_to_article_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn list_articles_sqlite(pool: &SqlitePool, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list articles")?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row_to_article_sqlite(&row)?);
    }

    Ok(articles)
}

async fn count_articles_sqlite(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM articles")
        .fetch_one(pool)
        .await
        .context("Failed to count articles")?;

    Ok(row.get("count"))
}

async fn update_article_sqlite(pool: &SqlitePool, id: i64, input: &UpdateArticleInput) -> Result<Article> {
    // First get the existing article
    let existing = get_article_by_id_sqlite(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Article not found"))?;

    let now = Utc::now();
    let new_slug = input.slug.as_ref().unwrap_or(&existing.slug);
    let new_title = input.title.as_ref().unwrap_or(&existing.title);
    let new_content = input.content.as_ref().unwrap_or(&existing.content);
    let new_content_html = input.content_html.as_ref().unwrap_or(&existing.content_html);
    let new_category_id = input.category_id.unwrap_or(existing.category_id);
    let new_status = input.status.unwrap_or(existing.status);
    let new_thumbnail = input.thumbnail.clone().or(existing.thumbnail.clone());
    let new_is_pinned = input.is_pinned.unwrap_or(existing.is_pinned);
    let new_pin_order = input.pin_order.unwrap_or(existing.pin_order);

    // Update published_at if status changed to Published
    let new_published_at = if new_status == ArticleStatus::Published && existing.status != ArticleStatus::Published {
        Some(now)
    } else if new_status != ArticleStatus::Published {
        None
    } else {
        existing.published_at
    };

    sqlx::query(
        r#"
        UPDATE articles
        SET slug = ?, title = ?, content = ?, content_html = ?, category_id = ?, status = ?, published_at = ?, updated_at = ?, thumbnail = ?, is_pinned = ?, pin_order = ?
        WHERE id = ?
        "#,
    )
    .bind(new_slug)
    .bind(new_title)
    .bind(new_content)
    .bind(new_content_html)
    .bind(new_category_id)
    .bind(new_status.as_str())
    .bind(new_published_at)
    .bind(now)
    .bind(&new_thumbnail)
    .bind(new_is_pinned)
    .bind(new_pin_order)
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to update article")?;

    // Return the updated article
    get_article_by_id_sqlite(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Article not found after update"))
}

async fn delete_article_sqlite(pool: &SqlitePool, id: i64) -> Result<()> {
    // Note: article_tags entries will be deleted automatically due to ON DELETE CASCADE
    sqlx::query("DELETE FROM articles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete article")?;

    Ok(())
}

async fn list_articles_by_category_sqlite(pool: &SqlitePool, category_id: i64, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE category_id = ?
        ORDER BY is_pinned DESC, pin_order ASC, published_at DESC NULLS LAST, created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(category_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list articles by category")?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row_to_article_sqlite(&row)?);
    }

    Ok(articles)
}

async fn list_articles_by_tag_sqlite(pool: &SqlitePool, tag_id: i64, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT a.id, a.slug, a.title, a.content, a.content_html, a.author_id, a.category_id, a.status, a.published_at, a.created_at, a.updated_at, a.view_count, a.like_count, a.comment_count, a.thumbnail, a.is_pinned, a.pin_order
        FROM articles a
        INNER JOIN article_tags at ON a.id = at.article_id
        WHERE at.tag_id = ?
        ORDER BY a.is_pinned DESC, a.pin_order ASC, a.published_at DESC NULLS LAST, a.created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(tag_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list articles by tag")?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row_to_article_sqlite(&row)?);
    }

    Ok(articles)
}

async fn list_published_articles_sqlite(pool: &SqlitePool, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE status = 'published'
        ORDER BY is_pinned DESC, pin_order ASC, published_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list published articles")?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row_to_article_sqlite(&row)?);
    }

    Ok(articles)
}

async fn count_published_sqlite(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE status = 'published'")
        .fetch_one(pool)
        .await
        .context("Failed to count published articles")?;

    Ok(row.get("count"))
}

async fn count_by_category_sqlite(pool: &SqlitePool, category_id: i64) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE category_id = ?")
        .bind(category_id)
        .fetch_one(pool)
        .await
        .context("Failed to count articles by category")?;

    Ok(row.get("count"))
}

async fn count_by_tag_sqlite(pool: &SqlitePool, tag_id: i64) -> Result<i64> {
    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM article_tags WHERE tag_id = ?"
    )
    .bind(tag_id)
    .fetch_one(pool)
    .await
    .context("Failed to count articles by tag")?;

    Ok(row.get("count"))
}

async fn exists_by_slug_sqlite(pool: &SqlitePool, slug: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE slug = ?")
        .bind(slug)
        .fetch_one(pool)
        .await
        .context("Failed to check article slug existence")?;

    let count: i64 = row.get("count");
    Ok(count > 0)
}

async fn exists_by_slug_excluding_sqlite(pool: &SqlitePool, slug: &str, exclude_id: i64) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE slug = ? AND id != ?")
        .bind(slug)
        .bind(exclude_id)
        .fetch_one(pool)
        .await
        .context("Failed to check article slug existence")?;

    let count: i64 = row.get("count");
    Ok(count > 0)
}

fn row_to_article_sqlite(row: &sqlx::sqlite::SqliteRow) -> Result<Article> {
    let status_str: String = row.get("status");
    let status = ArticleStatus::from_str(&status_str)
        .ok_or_else(|| anyhow::anyhow!("Invalid article status: {}", status_str))?;

    Ok(Article {
        id: row.get("id"),
        slug: row.get("slug"),
        title: row.get("title"),
        content: row.get("content"),
        content_html: row.get("content_html"),
        author_id: row.get("author_id"),
        category_id: row.get("category_id"),
        status,
        published_at: row.get("published_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        view_count: row.try_get("view_count").unwrap_or(0),
        like_count: row.try_get("like_count").unwrap_or(0),
        comment_count: row.try_get("comment_count").unwrap_or(0),
        thumbnail: row.try_get("thumbnail").ok(),
        is_pinned: row.try_get("is_pinned").unwrap_or(false),
        pin_order: row.try_get("pin_order").unwrap_or(0),
    })
}

async fn search_articles_sqlite(pool: &SqlitePool, keyword: &str, offset: i64, limit: i64, published_only: bool) -> Result<Vec<Article>> {
    let search_pattern = format!("%{}%", keyword);
    
    let query = if published_only {
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE status = 'published' AND (title LIKE ? OR content LIKE ?)
        ORDER BY is_pinned DESC, pin_order ASC, published_at DESC
        LIMIT ? OFFSET ?
        "#
    } else {
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE title LIKE ? OR content LIKE ?
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#
    };

    let rows = sqlx::query(query)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to search articles")?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row_to_article_sqlite(&row)?);
    }

    Ok(articles)
}

async fn count_search_sqlite(pool: &SqlitePool, keyword: &str, published_only: bool) -> Result<i64> {
    let search_pattern = format!("%{}%", keyword);
    
    let query = if published_only {
        "SELECT COUNT(*) as count FROM articles WHERE status = 'published' AND (title LIKE ? OR content LIKE ?)"
    } else {
        "SELECT COUNT(*) as count FROM articles WHERE title LIKE ? OR content LIKE ?"
    };

    let row = sqlx::query(query)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_one(pool)
        .await
        .context("Failed to count search results")?;

    Ok(row.get("count"))
}


// ============================================================================
// MySQL implementations
// ============================================================================

async fn create_article_mysql(pool: &MySqlPool, input: &CreateArticleInput) -> Result<Article> {
    let now = Utc::now();
    let status = input.status.unwrap_or_default();
    let published_at = if status == ArticleStatus::Published {
        Some(now)
    } else {
        None
    };
    let content_html = input.content_html.clone().unwrap_or_default();

    let result = sqlx::query(
        r#"
        INSERT INTO articles (slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, thumbnail, is_pinned, pin_order)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&input.slug)
    .bind(&input.title)
    .bind(&input.content)
    .bind(&content_html)
    .bind(input.author_id)
    .bind(input.category_id)
    .bind(status.as_str())
    .bind(published_at)
    .bind(now)
    .bind(now)
    .bind::<Option<&str>>(None)
    .bind(false)
    .bind(0)
    .execute(pool)
    .await
    .context("Failed to create article")?;

    let id = result.last_insert_id() as i64;

    Ok(Article {
        id,
        slug: input.slug.clone(),
        title: input.title.clone(),
        content: input.content.clone(),
        content_html,
        author_id: input.author_id,
        category_id: input.category_id,
        status,
        published_at,
        created_at: now,
        updated_at: now,
        view_count: 0,
        like_count: 0,
        comment_count: 0,
        thumbnail: None,
        is_pinned: false,
        pin_order: 0,
    })
}

async fn get_article_by_id_mysql(pool: &MySqlPool, id: i64) -> Result<Option<Article>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get article by ID")?;

    match row {
        Some(row) => Ok(Some(row_to_article_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn get_article_by_slug_mysql(pool: &MySqlPool, slug: &str) -> Result<Option<Article>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE slug = ?
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .context("Failed to get article by slug")?;

    match row {
        Some(row) => Ok(Some(row_to_article_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn list_articles_mysql(pool: &MySqlPool, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list articles")?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row_to_article_mysql(&row)?);
    }

    Ok(articles)
}

async fn count_articles_mysql(pool: &MySqlPool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM articles")
        .fetch_one(pool)
        .await
        .context("Failed to count articles")?;

    Ok(row.get("count"))
}

async fn update_article_mysql(pool: &MySqlPool, id: i64, input: &UpdateArticleInput) -> Result<Article> {
    // First get the existing article
    let existing = get_article_by_id_mysql(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Article not found"))?;

    let now = Utc::now();
    let new_slug = input.slug.as_ref().unwrap_or(&existing.slug);
    let new_title = input.title.as_ref().unwrap_or(&existing.title);
    let new_content = input.content.as_ref().unwrap_or(&existing.content);
    let new_content_html = input.content_html.as_ref().unwrap_or(&existing.content_html);
    let new_category_id = input.category_id.unwrap_or(existing.category_id);
    let new_status = input.status.unwrap_or(existing.status);
    let new_thumbnail = input.thumbnail.clone().or(existing.thumbnail.clone());
    let new_is_pinned = input.is_pinned.unwrap_or(existing.is_pinned);
    let new_pin_order = input.pin_order.unwrap_or(existing.pin_order);

    // Update published_at if status changed to Published
    let new_published_at = if new_status == ArticleStatus::Published && existing.status != ArticleStatus::Published {
        Some(now)
    } else if new_status != ArticleStatus::Published {
        None
    } else {
        existing.published_at
    };

    sqlx::query(
        r#"
        UPDATE articles
        SET slug = ?, title = ?, content = ?, content_html = ?, category_id = ?, status = ?, published_at = ?, updated_at = ?, thumbnail = ?, is_pinned = ?, pin_order = ?
        WHERE id = ?
        "#,
    )
    .bind(new_slug)
    .bind(new_title)
    .bind(new_content)
    .bind(new_content_html)
    .bind(new_category_id)
    .bind(new_status.as_str())
    .bind(new_published_at)
    .bind(now)
    .bind(&new_thumbnail)
    .bind(new_is_pinned)
    .bind(new_pin_order)
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to update article")?;

    // Return the updated article
    get_article_by_id_mysql(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Article not found after update"))
}

async fn delete_article_mysql(pool: &MySqlPool, id: i64) -> Result<()> {
    // Note: article_tags entries will be deleted automatically due to ON DELETE CASCADE
    sqlx::query("DELETE FROM articles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete article")?;

    Ok(())
}

async fn list_articles_by_category_mysql(pool: &MySqlPool, category_id: i64, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE category_id = ?
        ORDER BY is_pinned DESC, pin_order ASC, COALESCE(published_at, created_at) DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(category_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list articles by category")?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row_to_article_mysql(&row)?);
    }

    Ok(articles)
}

async fn list_articles_by_tag_mysql(pool: &MySqlPool, tag_id: i64, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT a.id, a.slug, a.title, a.content, a.content_html, a.author_id, a.category_id, a.status, a.published_at, a.created_at, a.updated_at, a.view_count, a.like_count, a.comment_count, a.thumbnail, a.is_pinned, a.pin_order
        FROM articles a
        INNER JOIN article_tags at ON a.id = at.article_id
        WHERE at.tag_id = ?
        ORDER BY a.is_pinned DESC, a.pin_order ASC, COALESCE(a.published_at, a.created_at) DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(tag_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list articles by tag")?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row_to_article_mysql(&row)?);
    }

    Ok(articles)
}

async fn list_published_articles_mysql(pool: &MySqlPool, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE status = 'published'
        ORDER BY is_pinned DESC, pin_order ASC, published_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list published articles")?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row_to_article_mysql(&row)?);
    }

    Ok(articles)
}

async fn count_published_mysql(pool: &MySqlPool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE status = 'published'")
        .fetch_one(pool)
        .await
        .context("Failed to count published articles")?;

    Ok(row.get("count"))
}

async fn count_by_category_mysql(pool: &MySqlPool, category_id: i64) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE category_id = ?")
        .bind(category_id)
        .fetch_one(pool)
        .await
        .context("Failed to count articles by category")?;

    Ok(row.get("count"))
}

async fn count_by_tag_mysql(pool: &MySqlPool, tag_id: i64) -> Result<i64> {
    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM article_tags WHERE tag_id = ?"
    )
    .bind(tag_id)
    .fetch_one(pool)
    .await
    .context("Failed to count articles by tag")?;

    Ok(row.get("count"))
}

async fn exists_by_slug_mysql(pool: &MySqlPool, slug: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE slug = ?")
        .bind(slug)
        .fetch_one(pool)
        .await
        .context("Failed to check article slug existence")?;

    let count: i64 = row.get("count");
    Ok(count > 0)
}

async fn exists_by_slug_excluding_mysql(pool: &MySqlPool, slug: &str, exclude_id: i64) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE slug = ? AND id != ?")
        .bind(slug)
        .bind(exclude_id)
        .fetch_one(pool)
        .await
        .context("Failed to check article slug existence")?;

    let count: i64 = row.get("count");
    Ok(count > 0)
}

fn row_to_article_mysql(row: &sqlx::mysql::MySqlRow) -> Result<Article> {
    let status_str: String = row.get("status");
    let status = ArticleStatus::from_str(&status_str)
        .ok_or_else(|| anyhow::anyhow!("Invalid article status: {}", status_str))?;

    Ok(Article {
        id: row.get("id"),
        slug: row.get("slug"),
        title: row.get("title"),
        content: row.get("content"),
        content_html: row.get("content_html"),
        author_id: row.get("author_id"),
        category_id: row.get("category_id"),
        status,
        published_at: row.get("published_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        view_count: row.try_get("view_count").unwrap_or(0),
        like_count: row.try_get("like_count").unwrap_or(0),
        comment_count: row.try_get("comment_count").unwrap_or(0),
        thumbnail: row.try_get("thumbnail").ok(),
        is_pinned: row.try_get("is_pinned").unwrap_or(false),
        pin_order: row.try_get("pin_order").unwrap_or(0),
    })
}

async fn search_articles_mysql(pool: &MySqlPool, keyword: &str, offset: i64, limit: i64, published_only: bool) -> Result<Vec<Article>> {
    let search_pattern = format!("%{}%", keyword);
    
    let query = if published_only {
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE status = 'published' AND (title LIKE ? OR content LIKE ?)
        ORDER BY is_pinned DESC, pin_order ASC, published_at DESC
        LIMIT ? OFFSET ?
        "#
    } else {
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE title LIKE ? OR content LIKE ?
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#
    };

    let rows = sqlx::query(query)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to search articles")?;

    let mut articles = Vec::new();
    for row in rows {
        articles.push(row_to_article_mysql(&row)?);
    }

    Ok(articles)
}

async fn count_search_mysql(pool: &MySqlPool, keyword: &str, published_only: bool) -> Result<i64> {
    let search_pattern = format!("%{}%", keyword);
    
    let query = if published_only {
        "SELECT COUNT(*) as count FROM articles WHERE status = 'published' AND (title LIKE ? OR content LIKE ?)"
    } else {
        "SELECT COUNT(*) as count FROM articles WHERE title LIKE ? OR content LIKE ?"
    };

    let row = sqlx::query(query)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_one(pool)
        .await
        .context("Failed to count search results")?;

    Ok(row.get("count"))
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_test_pool, migrations};
    use crate::db::repositories::tag::{TagRepository, SqlxTagRepository};
    use crate::models::{ListParams, PagedResult, Tag};

    async fn setup_test_repo() -> (DynDatabasePool, SqlxArticleRepository) {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");
        let repo = SqlxArticleRepository::new(pool.clone());
        (pool, repo)
    }

    /// Helper to create a user for article tests
    async fn create_test_user(pool: &SqlitePool) -> i64 {
        let result = sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind("testuser")
        .bind("test@example.com")
        .bind("hash123")
        .bind("author")
        .execute(pool)
        .await
        .expect("Failed to create test user");
        result.last_insert_rowid()
    }

    /// Helper to create a category for article tests
    async fn create_test_category(pool: &SqlitePool, slug: &str) -> i64 {
        let result = sqlx::query(
            "INSERT INTO categories (slug, name, sort_order) VALUES (?, ?, ?)",
        )
        .bind(slug)
        .bind(format!("Category {}", slug))
        .bind(0)
        .execute(pool)
        .await
        .expect("Failed to create test category");
        result.last_insert_rowid()
    }

    fn create_test_input(slug: &str, title: &str, author_id: i64, category_id: i64) -> CreateArticleInput {
        CreateArticleInput {
            slug: slug.to_string(),
            title: title.to_string(),
            content: format!("Content for {}", title),
            content_html: Some(format!("<p>Content for {}</p>", title)),
            author_id,
            category_id,
            status: None,
        }
    }

    #[tokio::test]
    async fn test_create_article() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        let input = create_test_input("test-article", "Test Article", user_id, category_id);
        let created = repo.create(&input).await.expect("Failed to create article");

        assert!(created.id > 0);
        assert_eq!(created.slug, "test-article");
        assert_eq!(created.title, "Test Article");
        assert_eq!(created.status, ArticleStatus::Draft);
        assert!(created.published_at.is_none());
    }

    #[tokio::test]
    async fn test_create_published_article() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        let mut input = create_test_input("published-article", "Published Article", user_id, category_id);
        input.status = Some(ArticleStatus::Published);

        let created = repo.create(&input).await.expect("Failed to create article");

        assert_eq!(created.status, ArticleStatus::Published);
        assert!(created.published_at.is_some());
    }

    #[tokio::test]
    async fn test_get_article_by_id() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        let input = create_test_input("get-by-id", "Get By ID", user_id, category_id);
        let created = repo.create(&input).await.expect("Failed to create article");

        let found = repo
            .get_by_id(created.id)
            .await
            .expect("Failed to get article")
            .expect("Article not found");

        assert_eq!(found.id, created.id);
        assert_eq!(found.slug, "get-by-id");
        assert_eq!(found.title, "Get By ID");
    }

    #[tokio::test]
    async fn test_get_article_by_id_not_found() {
        let (_pool, repo) = setup_test_repo().await;

        let found = repo.get_by_id(99999).await.expect("Failed to get article");

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_get_article_by_slug() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        let input = create_test_input("unique-slug", "Unique Slug", user_id, category_id);
        repo.create(&input).await.expect("Failed to create article");

        let found = repo
            .get_by_slug("unique-slug")
            .await
            .expect("Failed to get article")
            .expect("Article not found");

        assert_eq!(found.slug, "unique-slug");
    }

    #[tokio::test]
    async fn test_get_article_by_slug_not_found() {
        let (_pool, repo) = setup_test_repo().await;

        let found = repo
            .get_by_slug("nonexistent")
            .await
            .expect("Failed to get article");

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_articles() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        // Create some articles
        for i in 1..=3 {
            let input = create_test_input(&format!("article-{}", i), &format!("Article {}", i), user_id, category_id);
            repo.create(&input).await.expect("Failed to create article");
        }

        let articles = repo.list(0, 10).await.expect("Failed to list articles");

        assert_eq!(articles.len(), 3);
    }

    #[tokio::test]
    async fn test_list_articles_pagination() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        // Create 5 articles
        for i in 1..=5 {
            let input = create_test_input(&format!("article-{}", i), &format!("Article {}", i), user_id, category_id);
            repo.create(&input).await.expect("Failed to create article");
        }

        // Get first page (2 items)
        let page1 = repo.list(0, 2).await.expect("Failed to list articles");
        assert_eq!(page1.len(), 2);

        // Get second page (2 items)
        let page2 = repo.list(2, 2).await.expect("Failed to list articles");
        assert_eq!(page2.len(), 2);

        // Get third page (1 item)
        let page3 = repo.list(4, 2).await.expect("Failed to list articles");
        assert_eq!(page3.len(), 1);
    }

    #[tokio::test]
    async fn test_count_articles() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        // Initially 0
        let count = repo.count().await.expect("Failed to count articles");
        assert_eq!(count, 0);

        // Create 3 articles
        for i in 1..=3 {
            let input = create_test_input(&format!("article-{}", i), &format!("Article {}", i), user_id, category_id);
            repo.create(&input).await.expect("Failed to create article");
        }

        let count = repo.count().await.expect("Failed to count articles");
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_update_article() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        let input = create_test_input("to-update", "To Update", user_id, category_id);
        let created = repo.create(&input).await.expect("Failed to create article");

        let update_input = UpdateArticleInput::new()
            .with_title("Updated Title".to_string())
            .with_content("Updated content".to_string());

        let updated = repo
            .update(created.id, &update_input)
            .await
            .expect("Failed to update article");

        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.content, "Updated content");
        assert_eq!(updated.slug, "to-update"); // Unchanged
    }

    #[tokio::test]
    async fn test_update_article_status_to_published() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        let input = create_test_input("draft-article", "Draft Article", user_id, category_id);
        let created = repo.create(&input).await.expect("Failed to create article");
        assert_eq!(created.status, ArticleStatus::Draft);
        assert!(created.published_at.is_none());

        let update_input = UpdateArticleInput::new()
            .with_status(ArticleStatus::Published);

        let updated = repo
            .update(created.id, &update_input)
            .await
            .expect("Failed to update article");

        assert_eq!(updated.status, ArticleStatus::Published);
        assert!(updated.published_at.is_some());
    }

    #[tokio::test]
    async fn test_delete_article() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        let input = create_test_input("to-delete", "To Delete", user_id, category_id);
        let created = repo.create(&input).await.expect("Failed to create article");

        repo.delete(created.id).await.expect("Failed to delete article");

        let found = repo.get_by_id(created.id).await.expect("Failed to get article");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_articles_by_category() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category1_id = create_test_category(sqlite_pool, "cat1").await;
        let category2_id = create_test_category(sqlite_pool, "cat2").await;

        // Create articles in different categories
        for i in 1..=3 {
            let input = create_test_input(&format!("cat1-article-{}", i), &format!("Cat1 Article {}", i), user_id, category1_id);
            repo.create(&input).await.expect("Failed to create article");
        }
        for i in 1..=2 {
            let input = create_test_input(&format!("cat2-article-{}", i), &format!("Cat2 Article {}", i), user_id, category2_id);
            repo.create(&input).await.expect("Failed to create article");
        }

        let cat1_articles = repo.list_by_category(category1_id, 0, 10).await.expect("Failed to list articles");
        assert_eq!(cat1_articles.len(), 3);

        let cat2_articles = repo.list_by_category(category2_id, 0, 10).await.expect("Failed to list articles");
        assert_eq!(cat2_articles.len(), 2);
    }

    #[tokio::test]
    async fn test_list_articles_by_tag() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        // Create tag repository
        let tag_repo = SqlxTagRepository::new(pool.clone());

        // Create a tag
        let tag = Tag::new("rust".to_string(), "Rust".to_string());
        let created_tag = tag_repo.create(&tag).await.expect("Failed to create tag");

        // Create articles
        let input1 = create_test_input("article-1", "Article 1", user_id, category_id);
        let article1 = repo.create(&input1).await.expect("Failed to create article");

        let input2 = create_test_input("article-2", "Article 2", user_id, category_id);
        let article2 = repo.create(&input2).await.expect("Failed to create article");

        let input3 = create_test_input("article-3", "Article 3", user_id, category_id);
        repo.create(&input3).await.expect("Failed to create article");

        // Associate tag with articles 1 and 2
        tag_repo.add_to_article(created_tag.id, article1.id).await.expect("Failed to add tag");
        tag_repo.add_to_article(created_tag.id, article2.id).await.expect("Failed to add tag");

        let tagged_articles = repo.list_by_tag(created_tag.id, 0, 10).await.expect("Failed to list articles");
        assert_eq!(tagged_articles.len(), 2);
    }

    #[tokio::test]
    async fn test_list_published_articles() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        // Create draft articles
        for i in 1..=2 {
            let input = create_test_input(&format!("draft-{}", i), &format!("Draft {}", i), user_id, category_id);
            repo.create(&input).await.expect("Failed to create article");
        }

        // Create published articles
        for i in 1..=3 {
            let mut input = create_test_input(&format!("published-{}", i), &format!("Published {}", i), user_id, category_id);
            input.status = Some(ArticleStatus::Published);
            repo.create(&input).await.expect("Failed to create article");
        }

        let published = repo.list_published(0, 10).await.expect("Failed to list published articles");
        assert_eq!(published.len(), 3);

        // All should be published
        for article in &published {
            assert_eq!(article.status, ArticleStatus::Published);
        }
    }

    #[tokio::test]
    async fn test_list_published_ordered_by_published_at_desc() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        // Create published articles with small delays to ensure different timestamps
        for i in 1..=3 {
            let mut input = create_test_input(&format!("published-{}", i), &format!("Published {}", i), user_id, category_id);
            input.status = Some(ArticleStatus::Published);
            repo.create(&input).await.expect("Failed to create article");
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let published = repo.list_published(0, 10).await.expect("Failed to list published articles");
        assert_eq!(published.len(), 3);

        // Should be ordered by published_at DESC (newest first)
        for i in 0..published.len() - 1 {
            let current = published[i].published_at.unwrap();
            let next = published[i + 1].published_at.unwrap();
            assert!(current >= next, "Articles should be ordered by published_at DESC");
        }
    }

    #[tokio::test]
    async fn test_count_published() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        // Create draft articles
        for i in 1..=2 {
            let input = create_test_input(&format!("draft-{}", i), &format!("Draft {}", i), user_id, category_id);
            repo.create(&input).await.expect("Failed to create article");
        }

        // Create published articles
        for i in 1..=3 {
            let mut input = create_test_input(&format!("published-{}", i), &format!("Published {}", i), user_id, category_id);
            input.status = Some(ArticleStatus::Published);
            repo.create(&input).await.expect("Failed to create article");
        }

        let count = repo.count_published().await.expect("Failed to count published");
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_count_by_category() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category1_id = create_test_category(sqlite_pool, "cat1").await;
        let category2_id = create_test_category(sqlite_pool, "cat2").await;

        // Create articles in different categories
        for i in 1..=3 {
            let input = create_test_input(&format!("cat1-article-{}", i), &format!("Cat1 Article {}", i), user_id, category1_id);
            repo.create(&input).await.expect("Failed to create article");
        }
        for i in 1..=2 {
            let input = create_test_input(&format!("cat2-article-{}", i), &format!("Cat2 Article {}", i), user_id, category2_id);
            repo.create(&input).await.expect("Failed to create article");
        }

        let count1 = repo.count_by_category(category1_id).await.expect("Failed to count");
        assert_eq!(count1, 3);

        let count2 = repo.count_by_category(category2_id).await.expect("Failed to count");
        assert_eq!(count2, 2);
    }

    #[tokio::test]
    async fn test_count_by_tag() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        // Create tag repository
        let tag_repo = SqlxTagRepository::new(pool.clone());

        // Create a tag
        let tag = Tag::new("rust".to_string(), "Rust".to_string());
        let created_tag = tag_repo.create(&tag).await.expect("Failed to create tag");

        // Create articles and associate with tag
        for i in 1..=3 {
            let input = create_test_input(&format!("article-{}", i), &format!("Article {}", i), user_id, category_id);
            let article = repo.create(&input).await.expect("Failed to create article");
            tag_repo.add_to_article(created_tag.id, article.id).await.expect("Failed to add tag");
        }

        let count = repo.count_by_tag(created_tag.id).await.expect("Failed to count");
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_exists_by_slug() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        // Initially doesn't exist
        let exists = repo.exists_by_slug("test-slug").await.expect("Failed to check");
        assert!(!exists);

        // Create article
        let input = create_test_input("test-slug", "Test Slug", user_id, category_id);
        repo.create(&input).await.expect("Failed to create article");

        // Now exists
        let exists = repo.exists_by_slug("test-slug").await.expect("Failed to check");
        assert!(exists);
    }

    #[tokio::test]
    async fn test_exists_by_slug_excluding() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let user_id = create_test_user(sqlite_pool).await;
        let category_id = create_test_category(sqlite_pool, "test-cat").await;

        // Create two articles
        let input1 = create_test_input("slug-1", "Article 1", user_id, category_id);
        let article1 = repo.create(&input1).await.expect("Failed to create article");

        let input2 = create_test_input("slug-2", "Article 2", user_id, category_id);
        let article2 = repo.create(&input2).await.expect("Failed to create article");

        // slug-1 exists when excluding article2
        let exists = repo.exists_by_slug_excluding("slug-1", article2.id).await.expect("Failed to check");
        assert!(exists);

        // slug-1 doesn't exist when excluding article1 (itself)
        let exists = repo.exists_by_slug_excluding("slug-1", article1.id).await.expect("Failed to check");
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_article_status_conversion() {
        assert_eq!(ArticleStatus::Draft.as_str(), "draft");
        assert_eq!(ArticleStatus::Published.as_str(), "published");
        assert_eq!(ArticleStatus::Archived.as_str(), "archived");

        assert_eq!(ArticleStatus::from_str("draft"), Some(ArticleStatus::Draft));
        assert_eq!(ArticleStatus::from_str("published"), Some(ArticleStatus::Published));
        assert_eq!(ArticleStatus::from_str("archived"), Some(ArticleStatus::Archived));
        assert_eq!(ArticleStatus::from_str("DRAFT"), Some(ArticleStatus::Draft)); // Case insensitive
        assert_eq!(ArticleStatus::from_str("invalid"), None);
    }

    #[tokio::test]
    async fn test_list_params() {
        let params = ListParams::new(1, 10);
        assert_eq!(params.offset(), 0);
        assert_eq!(params.limit(), 10);

        let params = ListParams::new(2, 10);
        assert_eq!(params.offset(), 10);

        let params = ListParams::new(3, 5);
        assert_eq!(params.offset(), 10);
        assert_eq!(params.limit(), 5);

        // Edge cases
        let params = ListParams::new(0, 10); // Page 0 should become 1
        assert_eq!(params.page, 1);
        assert_eq!(params.offset(), 0);

        let params = ListParams::new(1, 200); // per_page clamped to 100
        assert_eq!(params.per_page, 100);
    }

    #[tokio::test]
    async fn test_paged_result() {
        let params = ListParams::new(1, 10);
        let items = vec![1, 2, 3, 4, 5];
        let result = PagedResult::new(items, 25, &params);

        assert_eq!(result.len(), 5);
        assert_eq!(result.total, 25);
        assert_eq!(result.page, 1);
        assert_eq!(result.per_page, 10);
        assert_eq!(result.total_pages(), 3);
        assert!(result.has_next());
        assert!(!result.has_prev());

        let params = ListParams::new(2, 10);
        let items = vec![6, 7, 8, 9, 10];
        let result = PagedResult::new(items, 25, &params);
        assert!(result.has_next());
        assert!(result.has_prev());

        let params = ListParams::new(3, 10);
        let items = vec![21, 22, 23, 24, 25];
        let result = PagedResult::new(items, 25, &params);
        assert!(!result.has_next());
        assert!(result.has_prev());
    }
}
