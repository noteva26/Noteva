//! Article repository
//!
//! Database operations for articles.
//!
//! This module provides:
//! - `ArticleRepository` trait defining the interface for article data access
//! - `SqlxArticleRepository` implementing the trait for SQLite and MySQL
//!
//! Satisfies requirements:
//! - 1.1: WHEN 用户提交新文章 THEN Article_Manager SHALL 创建文章记录并生成唯一标识符
//! - 1.2: WHEN 用户请求文章列表 THEN Article_Manager SHALL 返回分页的文章列表，支持按时间排序

use crate::db::DynDatabasePool;
use crate::models::{
    Article, ArticleSortBy, ArticleStatus, CreateArticleInput, UpdateArticleInput,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{MySqlPool, Row, SqlitePool};
use std::sync::Arc;

mod mysql_impl;
mod sqlite_impl;

#[cfg(test)]
mod tests;

use self::mysql_impl::*;
use self::sqlite_impl::*;

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
    async fn list(&self, offset: i64, limit: i64, sort_by: ArticleSortBy) -> Result<Vec<Article>>;

    /// Count total articles (all statuses)
    async fn count(&self) -> Result<i64>;

    /// Update an article
    async fn update(&self, id: i64, input: &UpdateArticleInput) -> Result<Article>;

    /// Delete an article
    async fn delete(&self, id: i64) -> Result<()>;

    /// List articles by category with pagination
    async fn list_by_category(
        &self,
        category_id: i64,
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>>;

    /// List articles by tag with pagination
    async fn list_by_tag(
        &self,
        tag_id: i64,
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>>;

    /// List only published articles with pagination (ordered by published_at DESC)
    async fn list_published(
        &self,
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>>;

    /// Count published articles
    async fn count_published(&self) -> Result<i64>;

    /// List published articles in any of the provided categories.
    async fn list_published_by_category_ids(
        &self,
        category_ids: &[i64],
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>>;

    /// Count published articles in any of the provided categories.
    async fn count_published_by_category_ids(&self, category_ids: &[i64]) -> Result<i64>;

    /// List published articles with a tag.
    async fn list_published_by_tag(
        &self,
        tag_id: i64,
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>>;

    /// Count published articles with a tag.
    async fn count_published_by_tag(&self, tag_id: i64) -> Result<i64>;

    /// List articles by status with pagination
    async fn list_by_status(
        &self,
        status: ArticleStatus,
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>>;

    /// Count articles by status
    async fn count_by_status(&self, status: ArticleStatus) -> Result<i64>;

    /// Count articles in a category
    async fn count_by_category(&self, category_id: i64) -> Result<i64>;

    /// Count articles with a tag
    async fn count_by_tag(&self, tag_id: i64) -> Result<i64>;

    /// Check if a slug already exists
    async fn exists_by_slug(&self, slug: &str) -> Result<bool>;

    /// Check if a slug exists for a different article (for updates)
    async fn exists_by_slug_excluding(&self, slug: &str, exclude_id: i64) -> Result<bool>;

    /// Search articles by keyword in title and content
    async fn search(
        &self,
        keyword: &str,
        offset: i64,
        limit: i64,
        published_only: bool,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>>;

    /// Count search results
    async fn count_search(&self, keyword: &str, published_only: bool) -> Result<i64>;

    /// Update article meta JSON (merge plugin_id namespace)
    async fn update_meta(
        &self,
        article_id: i64,
        plugin_id: &str,
        data: &serde_json::Value,
    ) -> Result<()>;

    /// List draft articles whose scheduled_at has passed (for auto-publishing)
    async fn list_scheduled_due(&self) -> Result<Vec<Article>>;

    /// Get adjacent (prev/next) published articles relative to a given published_at time.
    /// Returns (prev_article, next_article) where prev is newer and next is older.
    async fn get_adjacent(
        &self,
        article_id: i64,
        published_at: chrono::DateTime<Utc>,
    ) -> Result<(Option<Article>, Option<Article>)>;

    /// Get monthly archive counts for published articles using SQL GROUP BY.
    /// Returns Vec of (month_string, count) sorted newest first.
    async fn get_archives_monthly(&self) -> Result<Vec<(String, i64)>>;

    /// Get related articles (same category, excluding self, published only, limited).
    async fn get_related(
        &self,
        article_id: i64,
        category_id: i64,
        limit: i64,
    ) -> Result<Vec<Article>>;
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
        dispatch!(self, create_article, input)
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<Article>> {
        dispatch!(self, get_article_by_id, id)
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Option<Article>> {
        dispatch!(self, get_article_by_slug, slug)
    }

    async fn list(&self, offset: i64, limit: i64, sort_by: ArticleSortBy) -> Result<Vec<Article>> {
        dispatch!(self, list_articles, offset, limit, sort_by)
    }

    async fn count(&self) -> Result<i64> {
        dispatch!(self, count_articles)
    }

    async fn update(&self, id: i64, input: &UpdateArticleInput) -> Result<Article> {
        dispatch!(self, update_article, id, input)
    }

    async fn delete(&self, id: i64) -> Result<()> {
        dispatch!(self, delete_article, id)
    }

    async fn list_by_category(
        &self,
        category_id: i64,
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>> {
        dispatch!(
            self,
            list_articles_by_category,
            category_id,
            offset,
            limit,
            sort_by
        )
    }

    async fn list_by_tag(
        &self,
        tag_id: i64,
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>> {
        dispatch!(self, list_articles_by_tag, tag_id, offset, limit, sort_by)
    }

    async fn list_published(
        &self,
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>> {
        dispatch!(self, list_published_articles, offset, limit, sort_by)
    }

    async fn count_published(&self) -> Result<i64> {
        dispatch!(self, count_published)
    }

    async fn list_published_by_category_ids(
        &self,
        category_ids: &[i64],
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>> {
        dispatch!(
            self,
            list_published_articles_by_category_ids,
            category_ids,
            offset,
            limit,
            sort_by
        )
    }

    async fn count_published_by_category_ids(&self, category_ids: &[i64]) -> Result<i64> {
        dispatch!(self, count_published_by_category_ids, category_ids)
    }

    async fn list_published_by_tag(
        &self,
        tag_id: i64,
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>> {
        dispatch!(
            self,
            list_published_articles_by_tag,
            tag_id,
            offset,
            limit,
            sort_by
        )
    }

    async fn count_published_by_tag(&self, tag_id: i64) -> Result<i64> {
        dispatch!(self, count_published_by_tag, tag_id)
    }

    async fn list_by_status(
        &self,
        status: ArticleStatus,
        offset: i64,
        limit: i64,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>> {
        dispatch!(
            self,
            list_articles_by_status,
            status,
            offset,
            limit,
            sort_by
        )
    }

    async fn count_by_status(&self, status: ArticleStatus) -> Result<i64> {
        dispatch!(self, count_articles_by_status, status)
    }

    async fn count_by_category(&self, category_id: i64) -> Result<i64> {
        dispatch!(self, count_by_category, category_id)
    }

    async fn count_by_tag(&self, tag_id: i64) -> Result<i64> {
        dispatch!(self, count_by_tag, tag_id)
    }

    async fn exists_by_slug(&self, slug: &str) -> Result<bool> {
        dispatch!(self, exists_by_slug, slug)
    }

    async fn exists_by_slug_excluding(&self, slug: &str, exclude_id: i64) -> Result<bool> {
        dispatch!(self, exists_by_slug_excluding, exclude_id, slug)
    }

    async fn search(
        &self,
        keyword: &str,
        offset: i64,
        limit: i64,
        published_only: bool,
        sort_by: ArticleSortBy,
    ) -> Result<Vec<Article>> {
        dispatch!(
            self,
            search_articles,
            keyword,
            offset,
            limit,
            published_only,
            sort_by
        )
    }

    async fn count_search(&self, keyword: &str, published_only: bool) -> Result<i64> {
        dispatch!(self, count_search, keyword, published_only)
    }

    async fn update_meta(
        &self,
        article_id: i64,
        plugin_id: &str,
        data: &serde_json::Value,
    ) -> Result<()> {
        // Read current meta, merge plugin namespace, write back
        let article = self
            .get_by_id(article_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Article not found: {}", article_id))?;

        let mut meta = article.meta.clone();
        if !meta.is_object() {
            meta = serde_json::json!({});
        }
        meta.as_object_mut()
            .unwrap()
            .insert(plugin_id.to_string(), data.clone());

        dispatch!(self, update_article_meta, article_id, &meta.to_string())
    }

    async fn list_scheduled_due(&self) -> Result<Vec<Article>> {
        let now = Utc::now();
        dispatch!(self, list_scheduled_due_articles, &now)
    }

    async fn get_adjacent(
        &self,
        article_id: i64,
        published_at: chrono::DateTime<Utc>,
    ) -> Result<(Option<Article>, Option<Article>)> {
        let prev = dispatch!(self, get_prev_article, article_id, &published_at)?;
        let next = dispatch!(self, get_next_article, article_id, &published_at)?;
        Ok((prev, next))
    }

    async fn get_archives_monthly(&self) -> Result<Vec<(String, i64)>> {
        dispatch!(self, get_archives_monthly)
    }

    async fn get_related(
        &self,
        article_id: i64,
        category_id: i64,
        limit: i64,
    ) -> Result<Vec<Article>> {
        dispatch!(self, get_related_articles, article_id, category_id, limit)
    }
}

// ============================================================================
// Shared implementations (identical SQL across SQLite and MySQL)
// ============================================================================

impl_row_mapper! {
    pub(super) fn row_to_article(row) -> Result<Article> {
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
            meta: row.try_get::<String, _>("meta")
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::json!({})),
            scheduled_at: row.try_get("scheduled_at").ok().flatten(),
        })
    }
}

impl_dual_fn! {
    pub(super) async fn delete_article(pool, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM articles WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .context("Failed to delete article")?;
        Ok(())
    }
}

impl_dual_fn! {
    pub(super) async fn count_articles(pool) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM articles")
            .fetch_one(pool)
            .await
            .context("Failed to count articles")?;
        Ok(row.get("count"))
    }
}

impl_dual_fn! {
    pub(super) async fn count_published(pool) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE status = 'published'")
            .fetch_one(pool)
            .await
            .context("Failed to count published articles")?;
        Ok(row.get("count"))
    }
}

impl_dual_fn! {
    pub(super) async fn count_articles_by_status(pool, status: ArticleStatus) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE status = ?")
            .bind(status.as_str())
            .fetch_one(pool)
            .await
            .context("Failed to count articles by status")?;
        Ok(row.get("count"))
    }
}

impl_dual_fn! {
    pub(super) async fn count_by_category(pool, category_id: i64) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE category_id = ?")
            .bind(category_id)
            .fetch_one(pool)
            .await
            .context("Failed to count articles by category")?;
        Ok(row.get("count"))
    }
}

impl_dual_fn! {
    pub(super) async fn count_published_by_category_ids(pool, category_ids: &[i64]) -> Result<i64> {
        if category_ids.is_empty() {
            return Ok(0);
        }

        let placeholders = std::iter::repeat("?")
            .take(category_ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "SELECT COUNT(*) as count FROM articles WHERE status = 'published' AND category_id IN ({})",
            placeholders
        );
        let mut query = sqlx::query(&query);
        for category_id in category_ids {
            query = query.bind(category_id);
        }

        let row = query
            .fetch_one(pool)
            .await
            .context("Failed to count published articles by category")?;
        Ok(row.get("count"))
    }
}

impl_dual_fn! {
    pub(super) async fn count_by_tag(pool, tag_id: i64) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM article_tags WHERE tag_id = ?")
            .bind(tag_id)
            .fetch_one(pool)
            .await
            .context("Failed to count articles by tag")?;
        Ok(row.get("count"))
    }
}

impl_dual_fn! {
    pub(super) async fn count_published_by_tag(pool, tag_id: i64) -> Result<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count \
             FROM article_tags at INNER JOIN articles a ON a.id = at.article_id \
             WHERE at.tag_id = ? AND a.status = 'published'",
        )
        .bind(tag_id)
        .fetch_one(pool)
        .await
        .context("Failed to count published articles by tag")?;
        Ok(row.get("count"))
    }
}

impl_dual_fn! {
    pub(super) async fn exists_by_slug(pool, slug: &str) -> Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE slug = ?")
            .bind(slug)
            .fetch_one(pool)
            .await
            .context("Failed to check article slug existence")?;
        let count: i64 = row.get("count");
        Ok(count > 0)
    }
}

impl_dual_fn! {
    pub(super) async fn exists_by_slug_excluding(pool, exclude_id: i64, slug: &str) -> Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM articles WHERE slug = ? AND id != ?")
            .bind(slug)
            .bind(exclude_id)
            .fetch_one(pool)
            .await
            .context("Failed to check article slug existence")?;
        let count: i64 = row.get("count");
        Ok(count > 0)
    }
}

impl_dual_fn! {
    pub(super) async fn count_search(pool, keyword: &str, published_only: bool) -> Result<i64> {
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
}

impl_dual_fn! {
    pub(super) async fn update_article_meta(pool, article_id: i64, meta_str: &str) -> Result<()> {
        sqlx::query("UPDATE articles SET meta = ?, updated_at = ? WHERE id = ?")
            .bind(meta_str)
            .bind(Utc::now())
            .bind(article_id)
            .execute(pool)
            .await
            .context("Failed to update article meta")?;
        Ok(())
    }
}

/// SQL for prev/next queries (same for both DBs)
const PREV_ARTICLE_SQL: &str = r#"
    SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
    FROM articles
    WHERE status = 'published' AND published_at > ? AND id != ?
    ORDER BY published_at ASC
    LIMIT 1
"#;

const NEXT_ARTICLE_SQL: &str = r#"
    SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
    FROM articles
    WHERE status = 'published' AND published_at < ? AND id != ?
    ORDER BY published_at DESC
    LIMIT 1
"#;

const RELATED_ARTICLES_SQL: &str = r#"
    SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
    FROM articles
    WHERE status = 'published' AND category_id = ? AND id != ?
    ORDER BY published_at DESC
    LIMIT ?
"#;

pub(super) async fn get_prev_article_sqlite(
    pool: &SqlitePool,
    article_id: i64,
    published_at: &chrono::DateTime<Utc>,
) -> Result<Option<Article>> {
    let row = sqlx::query(PREV_ARTICLE_SQL)
        .bind(published_at)
        .bind(article_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get previous article")?;
    match row {
        Some(row) => Ok(Some(row_to_article_sqlite(&row)?)),
        None => Ok(None),
    }
}

pub(super) async fn get_prev_article_mysql(
    pool: &MySqlPool,
    article_id: i64,
    published_at: &chrono::DateTime<Utc>,
) -> Result<Option<Article>> {
    let row = sqlx::query(PREV_ARTICLE_SQL)
        .bind(published_at)
        .bind(article_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get previous article")?;
    match row {
        Some(row) => Ok(Some(row_to_article_mysql(&row)?)),
        None => Ok(None),
    }
}

pub(super) async fn get_next_article_sqlite(
    pool: &SqlitePool,
    article_id: i64,
    published_at: &chrono::DateTime<Utc>,
) -> Result<Option<Article>> {
    let row = sqlx::query(NEXT_ARTICLE_SQL)
        .bind(published_at)
        .bind(article_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get next article")?;
    match row {
        Some(row) => Ok(Some(row_to_article_sqlite(&row)?)),
        None => Ok(None),
    }
}

pub(super) async fn get_next_article_mysql(
    pool: &MySqlPool,
    article_id: i64,
    published_at: &chrono::DateTime<Utc>,
) -> Result<Option<Article>> {
    let row = sqlx::query(NEXT_ARTICLE_SQL)
        .bind(published_at)
        .bind(article_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get next article")?;
    match row {
        Some(row) => Ok(Some(row_to_article_mysql(&row)?)),
        None => Ok(None),
    }
}

/// SQLite: monthly archive counts via strftime
pub(super) async fn get_archives_monthly_sqlite(pool: &SqlitePool) -> Result<Vec<(String, i64)>> {
    let rows = sqlx::query(
        r#"
        SELECT strftime('%Y-%m', published_at) as month, COUNT(*) as count
        FROM articles
        WHERE status = 'published' AND published_at IS NOT NULL
        GROUP BY month
        ORDER BY month DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to get monthly archives")?;

    let mut result = Vec::new();
    for row in &rows {
        let month: String = row.get("month");
        let count: i64 = row.get("count");
        result.push((month, count));
    }
    Ok(result)
}

/// MySQL: monthly archive counts via DATE_FORMAT
pub(super) async fn get_archives_monthly_mysql(pool: &MySqlPool) -> Result<Vec<(String, i64)>> {
    let rows = sqlx::query(
        r#"
        SELECT DATE_FORMAT(published_at, '%Y-%m') as month, COUNT(*) as count
        FROM articles
        WHERE status = 'published' AND published_at IS NOT NULL
        GROUP BY month
        ORDER BY month DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to get monthly archives")?;

    let mut result = Vec::new();
    for row in &rows {
        let month: String = row.get("month");
        let count: i64 = row.get("count");
        result.push((month, count));
    }
    Ok(result)
}

pub(super) async fn get_related_articles_sqlite(
    pool: &SqlitePool,
    article_id: i64,
    category_id: i64,
    limit: i64,
) -> Result<Vec<Article>> {
    let rows = sqlx::query(RELATED_ARTICLES_SQL)
        .bind(category_id)
        .bind(article_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to get related articles")?;
    rows.iter().map(row_to_article_sqlite).collect()
}

pub(super) async fn get_related_articles_mysql(
    pool: &MySqlPool,
    article_id: i64,
    category_id: i64,
    limit: i64,
) -> Result<Vec<Article>> {
    let rows = sqlx::query(RELATED_ARTICLES_SQL)
        .bind(category_id)
        .bind(article_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to get related articles")?;
    rows.iter().map(row_to_article_mysql).collect()
}
