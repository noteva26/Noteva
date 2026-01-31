//! Tag repository
//!
//! Database operations for tags.
//!
//! This module provides:
//! - `TagRepository` trait defining the interface for tag data access
//! - `SqlxTagRepository` implementing the trait for SQLite and MySQL
//!
//! Satisfies requirements:
//! - 3.1: WHEN 用户为文章添加标签 THEN Tag_Service SHALL 创建或复用已有标签并建立关联
//! - 3.4: THE Tag_Service SHALL 提供标签云功能，按使用频率排序

use crate::config::DatabaseDriver;
use crate::db::DynDatabasePool;
use crate::models::{Tag, TagWithCount};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{MySqlPool, Row, SqlitePool};
use std::sync::Arc;

/// Tag repository trait
#[async_trait]
pub trait TagRepository: Send + Sync {
    /// Create a new tag
    async fn create(&self, tag: &Tag) -> Result<Tag>;

    /// Get tag by ID
    async fn get_by_id(&self, id: i64) -> Result<Option<Tag>>;

    /// Get tag by slug
    async fn get_by_slug(&self, slug: &str) -> Result<Option<Tag>>;

    /// Get tag by name
    async fn get_by_name(&self, name: &str) -> Result<Option<Tag>>;

    /// List all tags
    async fn list(&self) -> Result<Vec<Tag>>;

    /// Get tags with article count (for tag cloud)
    /// Returns tags sorted by article count in descending order
    async fn get_with_counts(&self, limit: usize) -> Result<Vec<TagWithCount>>;

    /// Delete a tag
    async fn delete(&self, id: i64) -> Result<()>;

    /// Associate tag with article
    async fn add_to_article(&self, tag_id: i64, article_id: i64) -> Result<()>;

    /// Remove tag from article
    async fn remove_from_article(&self, tag_id: i64, article_id: i64) -> Result<()>;
    
    /// Get tags for an article
    async fn get_by_article_id(&self, article_id: i64) -> Result<Vec<Tag>>;
}


/// SQLx-based tag repository implementation
///
/// Supports both SQLite and MySQL databases.
pub struct SqlxTagRepository {
    pool: DynDatabasePool,
}

impl SqlxTagRepository {
    /// Create a new SQLx tag repository
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }

    /// Create a boxed repository for use with dependency injection
    pub fn boxed(pool: DynDatabasePool) -> Arc<dyn TagRepository> {
        Arc::new(Self::new(pool))
    }
}

#[async_trait]
impl TagRepository for SqlxTagRepository {
    async fn create(&self, tag: &Tag) -> Result<Tag> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                create_tag_sqlite(self.pool.as_sqlite().unwrap(), tag).await
            }
            DatabaseDriver::Mysql => {
                create_tag_mysql(self.pool.as_mysql().unwrap(), tag).await
            }
        }
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<Tag>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_tag_by_id_sqlite(self.pool.as_sqlite().unwrap(), id).await
            }
            DatabaseDriver::Mysql => {
                get_tag_by_id_mysql(self.pool.as_mysql().unwrap(), id).await
            }
        }
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Option<Tag>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_tag_by_slug_sqlite(self.pool.as_sqlite().unwrap(), slug).await
            }
            DatabaseDriver::Mysql => {
                get_tag_by_slug_mysql(self.pool.as_mysql().unwrap(), slug).await
            }
        }
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<Tag>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_tag_by_name_sqlite(self.pool.as_sqlite().unwrap(), name).await
            }
            DatabaseDriver::Mysql => {
                get_tag_by_name_mysql(self.pool.as_mysql().unwrap(), name).await
            }
        }
    }

    async fn list(&self) -> Result<Vec<Tag>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => list_tags_sqlite(self.pool.as_sqlite().unwrap()).await,
            DatabaseDriver::Mysql => list_tags_mysql(self.pool.as_mysql().unwrap()).await,
        }
    }

    async fn get_with_counts(&self, limit: usize) -> Result<Vec<TagWithCount>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_tags_with_counts_sqlite(self.pool.as_sqlite().unwrap(), limit).await
            }
            DatabaseDriver::Mysql => {
                get_tags_with_counts_mysql(self.pool.as_mysql().unwrap(), limit).await
            }
        }
    }

    async fn delete(&self, id: i64) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => delete_tag_sqlite(self.pool.as_sqlite().unwrap(), id).await,
            DatabaseDriver::Mysql => delete_tag_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
    }

    async fn add_to_article(&self, tag_id: i64, article_id: i64) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                add_tag_to_article_sqlite(self.pool.as_sqlite().unwrap(), tag_id, article_id).await
            }
            DatabaseDriver::Mysql => {
                add_tag_to_article_mysql(self.pool.as_mysql().unwrap(), tag_id, article_id).await
            }
        }
    }

    async fn remove_from_article(&self, tag_id: i64, article_id: i64) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                remove_tag_from_article_sqlite(self.pool.as_sqlite().unwrap(), tag_id, article_id)
                    .await
            }
            DatabaseDriver::Mysql => {
                remove_tag_from_article_mysql(self.pool.as_mysql().unwrap(), tag_id, article_id)
                    .await
            }
        }
    }
    
    async fn get_by_article_id(&self, article_id: i64) -> Result<Vec<Tag>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_tags_by_article_sqlite(self.pool.as_sqlite().unwrap(), article_id).await
            }
            DatabaseDriver::Mysql => {
                get_tags_by_article_mysql(self.pool.as_mysql().unwrap(), article_id).await
            }
        }
    }
}


// ============================================================================
// SQLite implementations
// ============================================================================

async fn create_tag_sqlite(pool: &SqlitePool, tag: &Tag) -> Result<Tag> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"
        INSERT INTO tags (slug, name, created_at)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(&tag.slug)
    .bind(&tag.name)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create tag")?;

    let id = result.last_insert_rowid();

    Ok(Tag {
        id,
        slug: tag.slug.clone(),
        name: tag.name.clone(),
        created_at: now,
    })
}

async fn get_tag_by_id_sqlite(pool: &SqlitePool, id: i64) -> Result<Option<Tag>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, created_at
        FROM tags
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get tag by ID")?;

    match row {
        Some(row) => Ok(Some(row_to_tag_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn get_tag_by_slug_sqlite(pool: &SqlitePool, slug: &str) -> Result<Option<Tag>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, created_at
        FROM tags
        WHERE slug = ?
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .context("Failed to get tag by slug")?;

    match row {
        Some(row) => Ok(Some(row_to_tag_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn get_tag_by_name_sqlite(pool: &SqlitePool, name: &str) -> Result<Option<Tag>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, created_at
        FROM tags
        WHERE name = ?
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .context("Failed to get tag by name")?;

    match row {
        Some(row) => Ok(Some(row_to_tag_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn list_tags_sqlite(pool: &SqlitePool) -> Result<Vec<Tag>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, name, created_at
        FROM tags
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to list tags")?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row_to_tag_sqlite(&row)?);
    }

    Ok(tags)
}


/// Get tags with article counts for tag cloud functionality (SQLite)
/// Returns tags sorted by article count in descending order
async fn get_tags_with_counts_sqlite(pool: &SqlitePool, limit: usize) -> Result<Vec<TagWithCount>> {
    let rows = sqlx::query(
        r#"
        SELECT t.id, t.slug, t.name, t.created_at, COUNT(at.article_id) as article_count
        FROM tags t
        LEFT JOIN article_tags at ON t.id = at.tag_id
        GROUP BY t.id, t.slug, t.name, t.created_at
        ORDER BY article_count DESC, t.name ASC
        LIMIT ?
        "#,
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .context("Failed to get tags with counts")?;

    let mut tags_with_counts = Vec::new();
    for row in rows {
        let tag = row_to_tag_sqlite(&row)?;
        let article_count: i64 = row.get("article_count");
        tags_with_counts.push(TagWithCount::new(tag, article_count));
    }

    Ok(tags_with_counts)
}

async fn delete_tag_sqlite(pool: &SqlitePool, id: i64) -> Result<()> {
    // Note: article_tags entries will be deleted automatically due to ON DELETE CASCADE
    sqlx::query("DELETE FROM tags WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete tag")?;

    Ok(())
}

async fn add_tag_to_article_sqlite(pool: &SqlitePool, tag_id: i64, article_id: i64) -> Result<()> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO article_tags (article_id, tag_id)
        VALUES (?, ?)
        "#,
    )
    .bind(article_id)
    .bind(tag_id)
    .execute(pool)
    .await
    .context("Failed to add tag to article")?;

    Ok(())
}

async fn remove_tag_from_article_sqlite(
    pool: &SqlitePool,
    tag_id: i64,
    article_id: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM article_tags
        WHERE article_id = ? AND tag_id = ?
        "#,
    )
    .bind(article_id)
    .bind(tag_id)
    .execute(pool)
    .await
    .context("Failed to remove tag from article")?;

    Ok(())
}

fn row_to_tag_sqlite(row: &sqlx::sqlite::SqliteRow) -> Result<Tag> {
    Ok(Tag {
        id: row.get("id"),
        slug: row.get("slug"),
        name: row.get("name"),
        created_at: row.get("created_at"),
    })
}


// ============================================================================
// MySQL implementations
// ============================================================================

async fn create_tag_mysql(pool: &MySqlPool, tag: &Tag) -> Result<Tag> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"
        INSERT INTO tags (slug, name, created_at)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(&tag.slug)
    .bind(&tag.name)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create tag")?;

    let id = result.last_insert_id() as i64;

    Ok(Tag {
        id,
        slug: tag.slug.clone(),
        name: tag.name.clone(),
        created_at: now,
    })
}

async fn get_tag_by_id_mysql(pool: &MySqlPool, id: i64) -> Result<Option<Tag>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, created_at
        FROM tags
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get tag by ID")?;

    match row {
        Some(row) => Ok(Some(row_to_tag_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn get_tag_by_slug_mysql(pool: &MySqlPool, slug: &str) -> Result<Option<Tag>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, created_at
        FROM tags
        WHERE slug = ?
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .context("Failed to get tag by slug")?;

    match row {
        Some(row) => Ok(Some(row_to_tag_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn get_tag_by_name_mysql(pool: &MySqlPool, name: &str) -> Result<Option<Tag>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, created_at
        FROM tags
        WHERE name = ?
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .context("Failed to get tag by name")?;

    match row {
        Some(row) => Ok(Some(row_to_tag_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn list_tags_mysql(pool: &MySqlPool) -> Result<Vec<Tag>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, name, created_at
        FROM tags
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to list tags")?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row_to_tag_mysql(&row)?);
    }

    Ok(tags)
}


/// Get tags with article counts for tag cloud functionality (MySQL)
/// Returns tags sorted by article count in descending order
async fn get_tags_with_counts_mysql(pool: &MySqlPool, limit: usize) -> Result<Vec<TagWithCount>> {
    let rows = sqlx::query(
        r#"
        SELECT t.id, t.slug, t.name, t.created_at, COUNT(at.article_id) as article_count
        FROM tags t
        LEFT JOIN article_tags at ON t.id = at.tag_id
        GROUP BY t.id, t.slug, t.name, t.created_at
        ORDER BY article_count DESC, t.name ASC
        LIMIT ?
        "#,
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .context("Failed to get tags with counts")?;

    let mut tags_with_counts = Vec::new();
    for row in rows {
        let tag = row_to_tag_mysql(&row)?;
        let article_count: i64 = row.get("article_count");
        tags_with_counts.push(TagWithCount::new(tag, article_count));
    }

    Ok(tags_with_counts)
}

async fn delete_tag_mysql(pool: &MySqlPool, id: i64) -> Result<()> {
    // Note: article_tags entries will be deleted automatically due to ON DELETE CASCADE
    sqlx::query("DELETE FROM tags WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete tag")?;

    Ok(())
}

async fn add_tag_to_article_mysql(pool: &MySqlPool, tag_id: i64, article_id: i64) -> Result<()> {
    sqlx::query(
        r#"
        INSERT IGNORE INTO article_tags (article_id, tag_id)
        VALUES (?, ?)
        "#,
    )
    .bind(article_id)
    .bind(tag_id)
    .execute(pool)
    .await
    .context("Failed to add tag to article")?;

    Ok(())
}

async fn remove_tag_from_article_mysql(
    pool: &MySqlPool,
    tag_id: i64,
    article_id: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM article_tags
        WHERE article_id = ? AND tag_id = ?
        "#,
    )
    .bind(article_id)
    .bind(tag_id)
    .execute(pool)
    .await
    .context("Failed to remove tag from article")?;

    Ok(())
}

async fn get_tags_by_article_sqlite(pool: &SqlitePool, article_id: i64) -> Result<Vec<Tag>> {
    let rows = sqlx::query(
        r#"
        SELECT t.id, t.slug, t.name, t.created_at
        FROM tags t
        INNER JOIN article_tags at ON t.id = at.tag_id
        WHERE at.article_id = ?
        ORDER BY t.name
        "#,
    )
    .bind(article_id)
    .fetch_all(pool)
    .await
    .context("Failed to get tags by article")?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row_to_tag_sqlite(&row)?);
    }
    Ok(tags)
}

async fn get_tags_by_article_mysql(pool: &MySqlPool, article_id: i64) -> Result<Vec<Tag>> {
    let rows = sqlx::query(
        r#"
        SELECT t.id, t.slug, t.name, t.created_at
        FROM tags t
        INNER JOIN article_tags at ON t.id = at.tag_id
        WHERE at.article_id = ?
        ORDER BY t.name
        "#,
    )
    .bind(article_id)
    .fetch_all(pool)
    .await
    .context("Failed to get tags by article")?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row_to_tag_mysql(&row)?);
    }
    Ok(tags)
}

fn row_to_tag_mysql(row: &sqlx::mysql::MySqlRow) -> Result<Tag> {
    Ok(Tag {
        id: row.get("id"),
        slug: row.get("slug"),
        name: row.get("name"),
        created_at: row.get("created_at"),
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_test_pool, migrations};

    async fn setup_test_repo() -> (DynDatabasePool, SqlxTagRepository) {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");
        let repo = SqlxTagRepository::new(pool.clone());
        (pool, repo)
    }

    fn create_test_tag(slug: &str, name: &str) -> Tag {
        Tag::new(slug.to_string(), name.to_string())
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

    /// Helper to create an article for tag association tests
    async fn create_test_article(pool: &SqlitePool, author_id: i64, slug: &str) -> i64 {
        let result = sqlx::query(
            r#"INSERT INTO articles (slug, title, content, content_html, author_id, category_id, status) 
               VALUES (?, ?, ?, ?, ?, 1, 'published')"#,
        )
        .bind(slug)
        .bind(format!("Title for {}", slug))
        .bind("Content")
        .bind("<p>Content</p>")
        .bind(author_id)
        .execute(pool)
        .await
        .expect("Failed to create test article");
        result.last_insert_rowid()
    }

    #[tokio::test]
    async fn test_create_tag() {
        let (_pool, repo) = setup_test_repo().await;
        let tag = create_test_tag("rust", "Rust");

        let created = repo.create(&tag).await.expect("Failed to create tag");

        assert!(created.id > 0);
        assert_eq!(created.slug, "rust");
        assert_eq!(created.name, "Rust");
    }

    #[tokio::test]
    async fn test_get_tag_by_id() {
        let (_pool, repo) = setup_test_repo().await;
        let tag = create_test_tag("get-by-id", "Get By ID");
        let created = repo.create(&tag).await.expect("Failed to create tag");

        let found = repo
            .get_by_id(created.id)
            .await
            .expect("Failed to get tag")
            .expect("Tag not found");

        assert_eq!(found.id, created.id);
        assert_eq!(found.slug, "get-by-id");
    }

    #[tokio::test]
    async fn test_get_tag_by_id_not_found() {
        let (_pool, repo) = setup_test_repo().await;

        let found = repo.get_by_id(99999).await.expect("Failed to get tag");

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_get_tag_by_slug() {
        let (_pool, repo) = setup_test_repo().await;
        let tag = create_test_tag("unique-slug", "Unique Slug");
        repo.create(&tag).await.expect("Failed to create tag");

        let found = repo
            .get_by_slug("unique-slug")
            .await
            .expect("Failed to get tag")
            .expect("Tag not found");

        assert_eq!(found.slug, "unique-slug");
    }

    #[tokio::test]
    async fn test_get_tag_by_slug_not_found() {
        let (_pool, repo) = setup_test_repo().await;

        let found = repo
            .get_by_slug("nonexistent")
            .await
            .expect("Failed to get tag");

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_get_tag_by_name() {
        let (_pool, repo) = setup_test_repo().await;
        let tag = create_test_tag("name-test", "Unique Name");
        repo.create(&tag).await.expect("Failed to create tag");

        let found = repo
            .get_by_name("Unique Name")
            .await
            .expect("Failed to get tag")
            .expect("Tag not found");

        assert_eq!(found.name, "Unique Name");
    }

    #[tokio::test]
    async fn test_list_tags() {
        let (_pool, repo) = setup_test_repo().await;

        // Create some tags
        repo.create(&create_test_tag("tag1", "Tag 1"))
            .await
            .expect("Failed to create tag");
        repo.create(&create_test_tag("tag2", "Tag 2"))
            .await
            .expect("Failed to create tag");

        let tags = repo.list().await.expect("Failed to list tags");

        assert_eq!(tags.len(), 2);
    }

    #[tokio::test]
    async fn test_list_tags_ordered_by_name() {
        let (_pool, repo) = setup_test_repo().await;

        // Create tags in non-alphabetical order
        repo.create(&create_test_tag("zebra", "Zebra"))
            .await
            .expect("Failed to create tag");
        repo.create(&create_test_tag("apple", "Apple"))
            .await
            .expect("Failed to create tag");
        repo.create(&create_test_tag("mango", "Mango"))
            .await
            .expect("Failed to create tag");

        let tags = repo.list().await.expect("Failed to list tags");

        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].name, "Apple");
        assert_eq!(tags[1].name, "Mango");
        assert_eq!(tags[2].name, "Zebra");
    }

    #[tokio::test]
    async fn test_delete_tag() {
        let (_pool, repo) = setup_test_repo().await;
        let tag = create_test_tag("to-delete", "To Delete");
        let created = repo.create(&tag).await.expect("Failed to create tag");

        repo.delete(created.id).await.expect("Failed to delete tag");

        let found = repo.get_by_id(created.id).await.expect("Failed to get tag");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_add_tag_to_article() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create user and article
        let user_id = create_test_user(sqlite_pool).await;
        let article_id = create_test_article(sqlite_pool, user_id, "test-article").await;

        // Create tag
        let tag = repo
            .create(&create_test_tag("test-tag", "Test Tag"))
            .await
            .expect("Failed to create tag");

        // Add tag to article
        repo.add_to_article(tag.id, article_id)
            .await
            .expect("Failed to add tag to article");

        // Verify association exists
        let row = sqlx::query("SELECT COUNT(*) as count FROM article_tags WHERE article_id = ? AND tag_id = ?")
            .bind(article_id)
            .bind(tag.id)
            .fetch_one(sqlite_pool)
            .await
            .expect("Failed to query article_tags");

        let count: i64 = row.get("count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_add_tag_to_article_idempotent() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create user and article
        let user_id = create_test_user(sqlite_pool).await;
        let article_id = create_test_article(sqlite_pool, user_id, "test-article").await;

        // Create tag
        let tag = repo
            .create(&create_test_tag("test-tag", "Test Tag"))
            .await
            .expect("Failed to create tag");

        // Add tag to article twice (should not fail)
        repo.add_to_article(tag.id, article_id)
            .await
            .expect("Failed to add tag to article");
        repo.add_to_article(tag.id, article_id)
            .await
            .expect("Failed to add tag to article again");

        // Verify only one association exists
        let row = sqlx::query("SELECT COUNT(*) as count FROM article_tags WHERE article_id = ? AND tag_id = ?")
            .bind(article_id)
            .bind(tag.id)
            .fetch_one(sqlite_pool)
            .await
            .expect("Failed to query article_tags");

        let count: i64 = row.get("count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_remove_tag_from_article() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create user and article
        let user_id = create_test_user(sqlite_pool).await;
        let article_id = create_test_article(sqlite_pool, user_id, "test-article").await;

        // Create tag and add to article
        let tag = repo
            .create(&create_test_tag("test-tag", "Test Tag"))
            .await
            .expect("Failed to create tag");
        repo.add_to_article(tag.id, article_id)
            .await
            .expect("Failed to add tag to article");

        // Remove tag from article
        repo.remove_from_article(tag.id, article_id)
            .await
            .expect("Failed to remove tag from article");

        // Verify association is removed
        let row = sqlx::query("SELECT COUNT(*) as count FROM article_tags WHERE article_id = ? AND tag_id = ?")
            .bind(article_id)
            .bind(tag.id)
            .fetch_one(sqlite_pool)
            .await
            .expect("Failed to query article_tags");

        let count: i64 = row.get("count");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_get_with_counts_empty() {
        let (_pool, repo) = setup_test_repo().await;

        let tags_with_counts = repo
            .get_with_counts(10)
            .await
            .expect("Failed to get tags with counts");

        assert!(tags_with_counts.is_empty());
    }

    #[tokio::test]
    async fn test_get_with_counts_no_articles() {
        let (_pool, repo) = setup_test_repo().await;

        // Create tags without any articles
        repo.create(&create_test_tag("tag1", "Tag 1"))
            .await
            .expect("Failed to create tag");
        repo.create(&create_test_tag("tag2", "Tag 2"))
            .await
            .expect("Failed to create tag");

        let tags_with_counts = repo
            .get_with_counts(10)
            .await
            .expect("Failed to get tags with counts");

        assert_eq!(tags_with_counts.len(), 2);
        // All counts should be 0
        for twc in &tags_with_counts {
            assert_eq!(twc.article_count, 0);
        }
    }

    #[tokio::test]
    async fn test_get_with_counts_sorted_by_frequency() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create user
        let user_id = create_test_user(sqlite_pool).await;

        // Create articles
        let article1_id = create_test_article(sqlite_pool, user_id, "article-1").await;
        let article2_id = create_test_article(sqlite_pool, user_id, "article-2").await;
        let article3_id = create_test_article(sqlite_pool, user_id, "article-3").await;

        // Create tags
        let tag_popular = repo
            .create(&create_test_tag("popular", "Popular"))
            .await
            .expect("Failed to create tag");
        let tag_medium = repo
            .create(&create_test_tag("medium", "Medium"))
            .await
            .expect("Failed to create tag");
        let tag_rare = repo
            .create(&create_test_tag("rare", "Rare"))
            .await
            .expect("Failed to create tag");

        // Associate tags with articles (popular: 3, medium: 2, rare: 1)
        repo.add_to_article(tag_popular.id, article1_id).await.unwrap();
        repo.add_to_article(tag_popular.id, article2_id).await.unwrap();
        repo.add_to_article(tag_popular.id, article3_id).await.unwrap();

        repo.add_to_article(tag_medium.id, article1_id).await.unwrap();
        repo.add_to_article(tag_medium.id, article2_id).await.unwrap();

        repo.add_to_article(tag_rare.id, article1_id).await.unwrap();

        // Get tags with counts
        let tags_with_counts = repo
            .get_with_counts(10)
            .await
            .expect("Failed to get tags with counts");

        assert_eq!(tags_with_counts.len(), 3);
        
        // Should be sorted by count descending
        assert_eq!(tags_with_counts[0].tag.slug, "popular");
        assert_eq!(tags_with_counts[0].article_count, 3);
        
        assert_eq!(tags_with_counts[1].tag.slug, "medium");
        assert_eq!(tags_with_counts[1].article_count, 2);
        
        assert_eq!(tags_with_counts[2].tag.slug, "rare");
        assert_eq!(tags_with_counts[2].article_count, 1);
    }

    #[tokio::test]
    async fn test_get_with_counts_limit() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create user and article
        let user_id = create_test_user(sqlite_pool).await;
        let article_id = create_test_article(sqlite_pool, user_id, "article-1").await;

        // Create 5 tags
        for i in 1..=5 {
            let tag = repo
                .create(&create_test_tag(&format!("tag{}", i), &format!("Tag {}", i)))
                .await
                .expect("Failed to create tag");
            repo.add_to_article(tag.id, article_id).await.unwrap();
        }

        // Get only top 3
        let tags_with_counts = repo
            .get_with_counts(3)
            .await
            .expect("Failed to get tags with counts");

        assert_eq!(tags_with_counts.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_tag_cascades_to_article_tags() {
        let (pool, repo) = setup_test_repo().await;
        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create user and article
        let user_id = create_test_user(sqlite_pool).await;
        let article_id = create_test_article(sqlite_pool, user_id, "test-article").await;

        // Create tag and add to article
        let tag = repo
            .create(&create_test_tag("to-delete", "To Delete"))
            .await
            .expect("Failed to create tag");
        repo.add_to_article(tag.id, article_id)
            .await
            .expect("Failed to add tag to article");

        // Delete tag
        repo.delete(tag.id).await.expect("Failed to delete tag");

        // Verify article_tags entry is also deleted (CASCADE)
        let row = sqlx::query("SELECT COUNT(*) as count FROM article_tags WHERE tag_id = ?")
            .bind(tag.id)
            .fetch_one(sqlite_pool)
            .await
            .expect("Failed to query article_tags");

        let count: i64 = row.get("count");
        assert_eq!(count, 0);
    }
}