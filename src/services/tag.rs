//! Tag service
//!
//! Implements business logic for tag management:
//! - Create or reuse tags (Requirement 3.1)
//! - Tag cloud with frequency (Requirement 3.4)
//! - Tag-article associations (Requirement 3.2, 3.3)
//!
//! Satisfies requirements:
//! - 3.1: WHEN 用户为文章添加标�?THEN Tag_Service SHALL 创建或复用已有标签并建立关联
//! - 3.2: WHEN 用户请求某标签下的文�?THEN Tag_Service SHALL 返回所有包含该标签的文�?
//! - 3.3: WHEN 用户移除文章标签 THEN Tag_Service SHALL 解除关联，若标签无引用则可选删�?
//! - 3.4: THE Tag_Service SHALL 提供标签云功能，按使用频率排�?

use crate::db::repositories::TagRepository;
use crate::models::{Tag, TagWithCount};
use anyhow::{Context, Result};
use std::sync::Arc;

/// Error types for tag service operations
#[derive(Debug, thiserror::Error)]
pub enum TagServiceError {
    /// Tag not found
    #[error("Tag not found: {0}")]
    NotFound(String),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

/// Tag service for managing blog tags
///
/// Provides business logic for tag operations including:
/// - Creating or reusing existing tags
/// - Tag cloud functionality with frequency sorting
/// - Tag-article associations
pub struct TagService {
    repo: Arc<dyn TagRepository>,
}

impl TagService {
    /// Create a new tag service
    ///
    /// # Arguments
    /// * `repo` - Tag repository for database operations
    pub fn new(repo: Arc<dyn TagRepository>) -> Self {
        Self { repo }
    }

    /// Create a new tag or get existing one by name
    ///
    /// If a tag with the given name already exists, returns the existing tag.
    /// Otherwise, creates a new tag with a generated slug.
    ///
    /// # Arguments
    /// * `name` - Tag name
    ///
    /// # Returns
    /// The existing or newly created tag
    ///
    /// # Errors
    /// - `ValidationError` if the name is empty
    ///
    /// Satisfies requirement 3.1: WHEN 用户为文章添加标�?THEN Tag_Service SHALL 创建或复用已有标签并建立关联
    pub async fn create_or_get(&self, name: &str) -> Result<Tag, TagServiceError> {
        // Validate input
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(TagServiceError::ValidationError(
                "Tag name cannot be empty".to_string(),
            ));
        }

        // Check if tag already exists by name (Requirement 3.1 - reuse existing)
        if let Some(existing) = self
            .repo
            .get_by_name(trimmed_name)
            .await
            .context("Failed to check existing tag")?
        {
            return Ok(existing);
        }

        // Generate slug from name
        let slug = generate_tag_slug(trimmed_name);

        // Create new tag
        let tag = Tag::new(slug, trimmed_name.to_string());
        let created = self
            .repo
            .create(&tag)
            .await
            .context("Failed to create tag")?;

        Ok(created)
    }

    /// Get tag by slug
    ///
    /// # Arguments
    /// * `slug` - Tag slug
    ///
    /// # Returns
    /// The tag if found, None otherwise
    pub async fn get_by_slug(&self, slug: &str) -> Result<Option<Tag>, TagServiceError> {
        self.repo
            .get_by_slug(slug)
            .await
            .context("Failed to get tag by slug")
            .map_err(Into::into)
    }

    /// Get tag by ID
    ///
    /// # Arguments
    /// * `id` - Tag ID
    ///
    /// # Returns
    /// The tag if found, None otherwise
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Tag>, TagServiceError> {
        self.repo
            .get_by_id(id)
            .await
            .context("Failed to get tag by ID")
            .map_err(Into::into)
    }

    /// List all tags
    ///
    /// Returns all tags ordered by name.
    ///
    /// # Returns
    /// Vector of all tags
    pub async fn list(&self) -> Result<Vec<Tag>, TagServiceError> {
        self.repo
            .list()
            .await
            .context("Failed to list tags")
            .map_err(Into::into)
    }

    /// Get tag cloud (tags with usage count, sorted by frequency)
    ///
    /// Returns tags sorted by article count in descending order.
    /// This is useful for displaying a tag cloud where more popular
    /// tags are shown more prominently.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of tags to return
    ///
    /// # Returns
    /// Vector of tags with their article counts, sorted by count descending
    ///
    /// Satisfies requirement 3.4: THE Tag_Service SHALL 提供标签云功能，按使用频率排�?
    pub async fn get_tag_cloud(&self, limit: usize) -> Result<Vec<TagWithCount>, TagServiceError> {
        self.repo
            .get_with_counts(limit)
            .await
            .context("Failed to get tag cloud")
            .map_err(Into::into)
    }

    /// Delete a tag
    ///
    /// Removes the tag and all its article associations.
    /// Note: Article associations are automatically removed due to CASCADE delete.
    ///
    /// # Arguments
    /// * `id` - Tag ID to delete
    ///
    /// # Errors
    /// - `NotFound` if the tag doesn't exist
    ///
    /// Satisfies requirement 3.3: WHEN 用户移除文章标签 THEN Tag_Service SHALL 解除关联，若标签无引用则可选删�?
    pub async fn delete(&self, id: i64) -> Result<(), TagServiceError> {
        // Check if tag exists
        let tag = self
            .repo
            .get_by_id(id)
            .await
            .context("Failed to get tag")?
            .ok_or_else(|| TagServiceError::NotFound(format!("Tag with ID {} not found", id)))?;

        // Delete the tag (associations are removed via CASCADE)
        self.repo
            .delete(tag.id)
            .await
            .context("Failed to delete tag")?;

        Ok(())
    }

    /// Add a tag to an article
    ///
    /// Creates an association between a tag and an article.
    /// If the association already exists, this is a no-op.
    ///
    /// # Arguments
    /// * `tag_id` - Tag ID
    /// * `article_id` - Article ID
    ///
    /// Satisfies requirement 3.1: WHEN 用户为文章添加标�?THEN Tag_Service SHALL 创建或复用已有标签并建立关联
    pub async fn add_to_article(&self, tag_id: i64, article_id: i64) -> Result<(), TagServiceError> {
        self.repo
            .add_to_article(tag_id, article_id)
            .await
            .context("Failed to add tag to article")
            .map_err(Into::into)
    }

    /// Remove a tag from an article
    ///
    /// Removes the association between a tag and an article.
    ///
    /// # Arguments
    /// * `tag_id` - Tag ID
    /// * `article_id` - Article ID
    ///
    /// Satisfies requirement 3.3: WHEN 用户移除文章标签 THEN Tag_Service SHALL 解除关联
    pub async fn remove_from_article(
        &self,
        tag_id: i64,
        article_id: i64,
    ) -> Result<(), TagServiceError> {
        self.repo
            .remove_from_article(tag_id, article_id)
            .await
            .context("Failed to remove tag from article")
            .map_err(Into::into)
    }

    /// Check if a tag name already exists
    ///
    /// # Arguments
    /// * `name` - Tag name to check
    ///
    /// # Returns
    /// True if a tag with the given name exists
    pub async fn exists_by_name(&self, name: &str) -> Result<bool, TagServiceError> {
        let tag = self
            .repo
            .get_by_name(name)
            .await
            .context("Failed to check tag existence")?;
        Ok(tag.is_some())
    }
    
    /// Get tags for an article
    ///
    /// # Arguments
    /// * `article_id` - Article ID
    ///
    /// # Returns
    /// Vector of tags associated with the article
    pub async fn get_by_article_id(&self, article_id: i64) -> Result<Vec<Tag>, TagServiceError> {
        self.repo
            .get_by_article_id(article_id)
            .await
            .context("Failed to get tags by article")
            .map_err(Into::into)
    }
}

/// Generate a URL-friendly slug from a tag name
///
/// Converts the name to lowercase, replaces spaces and special characters
/// with hyphens, and removes consecutive hyphens.
/// Handles Unicode characters including Chinese.
///
/// # Arguments
/// * `name` - Tag name
///
/// # Returns
/// URL-friendly slug
pub fn generate_tag_slug(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else if c == ' ' || c == '_' || c == '-' {
                '-'
            } else if !c.is_ascii() {
                // For non-ASCII characters (like Chinese), keep them
                c
            } else {
                // Replace other ASCII special characters with hyphen
                '-'
            }
        })
        .collect();

    // Remove consecutive hyphens and trim hyphens from ends
    let mut result = String::new();
    let mut prev_hyphen = false;

    for c in slug.chars() {
        if c == '-' {
            if !prev_hyphen && !result.is_empty() {
                result.push(c);
                prev_hyphen = true;
            }
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }

    // Trim trailing hyphen
    result.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repositories::SqlxTagRepository;
    use crate::db::{create_test_pool, migrations, DynDatabasePool};

    async fn setup_test_service() -> (DynDatabasePool, TagService) {
        let pool = create_test_pool()
            .await
            .expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let repo = SqlxTagRepository::boxed(pool.clone());
        let service = TagService::new(repo);

        (pool, service)
    }

    // ========================================================================
    // Slug generation tests
    // ========================================================================

    #[test]
    fn test_generate_tag_slug_simple() {
        assert_eq!(generate_tag_slug("Hello World"), "hello-world");
    }

    #[test]
    fn test_generate_tag_slug_with_special_chars() {
        let slug = generate_tag_slug("Hello, World!");
        assert!(!slug.contains(','));
        assert!(!slug.contains('!'));
    }

    #[test]
    fn test_generate_tag_slug_lowercase() {
        assert_eq!(generate_tag_slug("UPPERCASE"), "uppercase");
    }

    // ========================================================================
    // create_or_get tests
    // ========================================================================

    #[tokio::test]
    async fn test_create_or_get_creates_new_tag() {
        let (_pool, service) = setup_test_service().await;

        let tag = service
            .create_or_get("Rust Programming")
            .await
            .expect("Failed to create tag");

        assert!(tag.id > 0);
        assert_eq!(tag.name, "Rust Programming");
        assert_eq!(tag.slug, "rust-programming");
    }

    #[tokio::test]
    async fn test_create_or_get_returns_existing_tag() {
        let (_pool, service) = setup_test_service().await;

        // Create first tag
        let tag1 = service
            .create_or_get("Existing Tag")
            .await
            .expect("Failed to create tag");

        // Try to create same tag again - should return existing
        let tag2 = service
            .create_or_get("Existing Tag")
            .await
            .expect("Failed to get existing tag");

        // Should be the same tag (same ID)
        assert_eq!(tag1.id, tag2.id);
        assert_eq!(tag1.name, tag2.name);
        assert_eq!(tag1.slug, tag2.slug);
    }

    #[tokio::test]
    async fn test_create_or_get_empty_name_fails() {
        let (_pool, service) = setup_test_service().await;

        let result = service.create_or_get("").await;
        assert!(matches!(result, Err(TagServiceError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_create_or_get_whitespace_name_fails() {
        let (_pool, service) = setup_test_service().await;

        let result = service.create_or_get("   ").await;
        assert!(matches!(result, Err(TagServiceError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_create_or_get_trims_whitespace() {
        let (_pool, service) = setup_test_service().await;

        let tag = service
            .create_or_get("  Trimmed Tag  ")
            .await
            .expect("Failed to create tag");

        assert_eq!(tag.name, "Trimmed Tag");
    }

    // ========================================================================
    // get_by_slug tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_by_slug_found() {
        let (_pool, service) = setup_test_service().await;

        // Create a tag
        service
            .create_or_get("Find By Slug")
            .await
            .expect("Failed to create tag");

        // Find by slug
        let found = service
            .get_by_slug("find-by-slug")
            .await
            .expect("Failed to get tag")
            .expect("Tag not found");

        assert_eq!(found.name, "Find By Slug");
    }

    #[tokio::test]
    async fn test_get_by_slug_not_found() {
        let (_pool, service) = setup_test_service().await;

        let result = service
            .get_by_slug("nonexistent")
            .await
            .expect("Failed to get tag");

        assert!(result.is_none());
    }

    // ========================================================================
    // list tests
    // ========================================================================

    #[tokio::test]
    async fn test_list_empty() {
        let (_pool, service) = setup_test_service().await;

        let tags = service.list().await.expect("Failed to list tags");
        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn test_list_returns_all_tags() {
        let (_pool, service) = setup_test_service().await;

        // Create some tags
        service.create_or_get("Tag A").await.unwrap();
        service.create_or_get("Tag B").await.unwrap();
        service.create_or_get("Tag C").await.unwrap();

        let tags = service.list().await.expect("Failed to list tags");
        assert_eq!(tags.len(), 3);
    }

    #[tokio::test]
    async fn test_list_ordered_by_name() {
        let (_pool, service) = setup_test_service().await;

        // Create tags in non-alphabetical order
        service.create_or_get("Zebra").await.unwrap();
        service.create_or_get("Apple").await.unwrap();
        service.create_or_get("Mango").await.unwrap();

        let tags = service.list().await.expect("Failed to list tags");

        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].name, "Apple");
        assert_eq!(tags[1].name, "Mango");
        assert_eq!(tags[2].name, "Zebra");
    }

    // ========================================================================
    // get_tag_cloud tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_tag_cloud_empty() {
        let (_pool, service) = setup_test_service().await;

        let cloud = service
            .get_tag_cloud(10)
            .await
            .expect("Failed to get tag cloud");

        assert!(cloud.is_empty());
    }

    #[tokio::test]
    async fn test_get_tag_cloud_no_articles() {
        let (_pool, service) = setup_test_service().await;

        // Create tags without articles
        service.create_or_get("Tag 1").await.unwrap();
        service.create_or_get("Tag 2").await.unwrap();

        let cloud = service
            .get_tag_cloud(10)
            .await
            .expect("Failed to get tag cloud");

        assert_eq!(cloud.len(), 2);
        // All counts should be 0
        for twc in &cloud {
            assert_eq!(twc.article_count, 0);
        }
    }

    #[tokio::test]
    async fn test_get_tag_cloud_respects_limit() {
        let (_pool, service) = setup_test_service().await;

        // Create 5 tags
        for i in 1..=5 {
            service
                .create_or_get(&format!("Tag {}", i))
                .await
                .unwrap();
        }

        // Request only 3
        let cloud = service
            .get_tag_cloud(3)
            .await
            .expect("Failed to get tag cloud");

        assert_eq!(cloud.len(), 3);
    }

    // ========================================================================
    // delete tests
    // ========================================================================

    #[tokio::test]
    async fn test_delete_success() {
        let (_pool, service) = setup_test_service().await;

        let tag = service
            .create_or_get("To Delete")
            .await
            .expect("Failed to create tag");

        service.delete(tag.id).await.expect("Failed to delete tag");

        // Verify tag is gone
        let found = service
            .get_by_id(tag.id)
            .await
            .expect("Failed to get tag");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let (_pool, service) = setup_test_service().await;

        let result = service.delete(99999).await;
        assert!(matches!(result, Err(TagServiceError::NotFound(_))));
    }

    // ========================================================================
    // Tag-article association tests
    // ========================================================================

    #[tokio::test]
    async fn test_add_to_article() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create test user and article
        let user_id = create_test_user(sqlite_pool).await;
        let article_id = create_test_article(sqlite_pool, user_id, "test-article").await;

        // Create tag
        let tag = service
            .create_or_get("Test Tag")
            .await
            .expect("Failed to create tag");

        // Add tag to article
        service
            .add_to_article(tag.id, article_id)
            .await
            .expect("Failed to add tag to article");

        // Verify association exists
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM article_tags WHERE article_id = ? AND tag_id = ?",
        )
        .bind(article_id)
        .bind(tag.id)
        .fetch_one(sqlite_pool)
        .await
        .expect("Failed to query article_tags");

        let count: i64 = sqlx::Row::get(&row, "count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_remove_from_article() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();

        // Create test user and article
        let user_id = create_test_user(sqlite_pool).await;
        let article_id = create_test_article(sqlite_pool, user_id, "test-article").await;

        // Create tag and add to article
        let tag = service
            .create_or_get("Test Tag")
            .await
            .expect("Failed to create tag");
        service
            .add_to_article(tag.id, article_id)
            .await
            .expect("Failed to add tag to article");

        // Remove tag from article
        service
            .remove_from_article(tag.id, article_id)
            .await
            .expect("Failed to remove tag from article");

        // Verify association is removed
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM article_tags WHERE article_id = ? AND tag_id = ?",
        )
        .bind(article_id)
        .bind(tag.id)
        .fetch_one(sqlite_pool)
        .await
        .expect("Failed to query article_tags");

        let count: i64 = sqlx::Row::get(&row, "count");
        assert_eq!(count, 0);
    }

    // ========================================================================
    // Helper functions for tests
    // ========================================================================

    /// Helper to create a user for article tests
    async fn create_test_user(pool: &sqlx::SqlitePool) -> i64 {
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
    async fn create_test_article(pool: &sqlx::SqlitePool, author_id: i64, slug: &str) -> i64 {
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

    // ========================================================================
    // Property-Based Tests
    // ========================================================================

    use proptest::prelude::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Counter for generating unique test data across property test iterations
    static PROPERTY_TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Generate a unique suffix for test data
    fn unique_suffix() -> u64 {
        PROPERTY_TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// Setup a fresh test service for property tests
    async fn setup_property_test_service() -> (DynDatabasePool, TagService) {
        let pool = create_test_pool()
            .await
            .expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let repo = SqlxTagRepository::boxed(pool.clone());
        let service = TagService::new(repo);

        (pool, service)
    }

    /// Helper to create a test user for property tests
    async fn create_property_test_user(pool: &sqlx::SqlitePool, suffix: u64) -> i64 {
        let result = sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind(format!("testuser_{}", suffix))
        .bind(format!("test_{}@example.com", suffix))
        .bind("hash123")
        .bind("author")
        .execute(pool)
        .await
        .expect("Failed to create test user");
        result.last_insert_rowid()
    }

    /// Helper to create a test article for property tests
    async fn create_property_test_article(
        pool: &sqlx::SqlitePool,
        author_id: i64,
        suffix: u64,
        index: usize,
    ) -> i64 {
        let result = sqlx::query(
            r#"INSERT INTO articles (slug, title, content, content_html, author_id, category_id, status) 
               VALUES (?, ?, ?, ?, ?, 1, 'published')"#,
        )
        .bind(format!("article-{}-{}", suffix, index))
        .bind(format!("Article {} {}", suffix, index))
        .bind("Content")
        .bind("<p>Content</p>")
        .bind(author_id)
        .execute(pool)
        .await
        .expect("Failed to create test article");
        result.last_insert_rowid()
    }

    // ========================================================================
    // Property 7: 标签复用一致�?(Tag Reuse Consistency)
    // For any tag name, calling create_or_get multiple times with the same name
    // should reuse the same tag record (same ID), not create new records.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 7: 标签复用一致�?
        /// **Validates: Requirements 3.1**
        ///
        /// Property 7: Tag Reuse Consistency
        /// For any tag name, calling create_or_get multiple times with the same name
        /// should reuse the same tag record (same ID), not create new records.
        #[test]
        fn property_7_tag_reuse_consistency(
            tag_name_base in "[a-zA-Z]{3,20}",
            call_count in 2..10usize
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let (_pool, service) = setup_property_test_service().await;
                let suffix = unique_suffix();

                // Create a unique tag name for this iteration
                let tag_name = format!("{}_{}", tag_name_base, suffix);

                // Call create_or_get multiple times with the same name
                let mut tag_ids: Vec<i64> = Vec::new();
                let mut tag_slugs: Vec<String> = Vec::new();
                let mut tag_names: Vec<String> = Vec::new();

                for _ in 0..call_count {
                    let tag = service.create_or_get(&tag_name).await
                        .expect("create_or_get should succeed");
                    tag_ids.push(tag.id);
                    tag_slugs.push(tag.slug.clone());
                    tag_names.push(tag.name.clone());
                }

                // Property: All returned tags should have the same ID
                let first_id = tag_ids[0];
                for (i, &id) in tag_ids.iter().enumerate() {
                    prop_assert_eq!(
                        id,
                        first_id,
                        "Tag ID at call {} should match first ID. Expected {}, got {}. \
                         All IDs: {:?}",
                        i,
                        first_id,
                        id,
                        tag_ids
                    );
                }

                // Property: All returned tags should have the same slug
                let first_slug = &tag_slugs[0];
                for (i, slug) in tag_slugs.iter().enumerate() {
                    prop_assert_eq!(
                        slug,
                        first_slug,
                        "Tag slug at call {} should match first slug. Expected {}, got {}",
                        i,
                        first_slug,
                        slug
                    );
                }

                // Property: All returned tags should have the same name
                let first_name = &tag_names[0];
                for (i, name) in tag_names.iter().enumerate() {
                    prop_assert_eq!(
                        name,
                        first_name,
                        "Tag name at call {} should match first name. Expected {}, got {}",
                        i,
                        first_name,
                        name
                    );
                }

                // Property: Only one tag should exist in the database
                let all_tags = service.list().await.expect("list should succeed");
                let matching_tags: Vec<_> = all_tags
                    .iter()
                    .filter(|t| t.name == tag_name)
                    .collect();
                prop_assert_eq!(
                    matching_tags.len(),
                    1,
                    "Only one tag with name '{}' should exist in database. Found {}",
                    tag_name,
                    matching_tags.len()
                );

                Ok(())
            });
            result?;
        }
    }

    // ========================================================================
    // Property 8: 标签云频率排�?(Tag Cloud Frequency Sorting)
    // For any set of tags, the tag cloud should return results sorted by
    // usage frequency (article count) in descending order.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 8: 标签云频率排�?
        /// **Validates: Requirements 3.4**
        ///
        /// Property 8: Tag Cloud Frequency Sorting
        /// For any set of tags with varying article counts, the tag cloud
        /// should return results sorted by article_count in descending order.
        #[test]
        fn property_8_tag_cloud_frequency_sorting(
            tag_count in 2..8usize,
            articles_per_tag in proptest::collection::vec(0..5usize, 2..8)
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let (pool, service) = setup_property_test_service().await;
                let suffix = unique_suffix();
                let sqlite_pool = pool.as_sqlite().unwrap();

                // Create a test user
                let user_id = create_property_test_user(sqlite_pool, suffix).await;

                // Determine actual tag count (limited by articles_per_tag length)
                let actual_tag_count = tag_count.min(articles_per_tag.len());

                // Create tags and associate them with varying numbers of articles
                let mut expected_counts: Vec<(String, i64)> = Vec::new();
                let mut article_counter = 0usize;

                for tag_idx in 0..actual_tag_count {
                    let tag_name = format!("Tag_{}_{}", suffix, tag_idx);
                    let tag = service.create_or_get(&tag_name).await
                        .expect("create_or_get should succeed");

                    let article_count = articles_per_tag[tag_idx];
                    expected_counts.push((tag_name.clone(), article_count as i64));

                    // Create articles and associate them with this tag
                    for _ in 0..article_count {
                        let article_id = create_property_test_article(
                            sqlite_pool,
                            user_id,
                            suffix,
                            article_counter,
                        ).await;
                        article_counter += 1;

                        service.add_to_article(tag.id, article_id).await
                            .expect("add_to_article should succeed");
                    }
                }

                // Get tag cloud
                let tag_cloud = service.get_tag_cloud(actual_tag_count + 10).await
                    .expect("get_tag_cloud should succeed");

                // Filter to only our test tags
                let our_tags: Vec<_> = tag_cloud
                    .iter()
                    .filter(|twc| twc.tag.name.starts_with(&format!("Tag_{}_", suffix)))
                    .collect();

                // Property: Tag cloud should be sorted by article_count descending
                for i in 1..our_tags.len() {
                    prop_assert!(
                        our_tags[i - 1].article_count >= our_tags[i].article_count,
                        "Tag cloud should be sorted by article_count descending. \
                         Tag '{}' has count {} but comes before tag '{}' with count {}",
                        our_tags[i - 1].tag.name,
                        our_tags[i - 1].article_count,
                        our_tags[i].tag.name,
                        our_tags[i].article_count
                    );
                }

                // Property: Article counts should match expected values
                for (tag_name, expected_count) in &expected_counts {
                    let found_tag = our_tags.iter().find(|twc| &twc.tag.name == tag_name);
                    if let Some(twc) = found_tag {
                        prop_assert_eq!(
                            twc.article_count,
                            *expected_count,
                            "Tag '{}' should have article_count {}. Got {}",
                            tag_name,
                            expected_count,
                            twc.article_count
                        );
                    } else {
                        // Tag might not be in cloud if limit was reached
                        // This is acceptable behavior
                    }
                }

                Ok(())
            });
            result?;
        }
    }
}
