//! Category service
//!
//! Implements business logic for category management:
//! - Create, read, update, delete categories
//! - Hierarchical category tree
//! - Category-article associations
//! - Name uniqueness validation
//! - Slug generation from name
//! - Article migration on category deletion
//!
//! Satisfies requirements:
//! - 2.1: WHEN 用户创建分类 THEN Category_Service SHALL 创建分类记录并支持设置父分类
//! - 2.2: WHEN 用户为文章指定分�?THEN Category_Service SHALL 建立文章与分类的关联
//! - 2.3: WHEN 用户请求某分类下的文�?THEN Category_Service SHALL 返回该分类及其子分类下的所有文�?
//! - 2.4: WHEN 用户删除分类 THEN Category_Service SHALL 将该分类下的文章移至默认分类
//! - 2.5: IF 分类名称已存�?THEN Category_Service SHALL 返回重复错误

use crate::cache::{Cache, CacheLayer};
use crate::db::repositories::CategoryRepository;
use crate::db::DynDatabasePool;
use crate::models::{Category, CategoryTree};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;

/// Default cache TTL for categories (1 hour)
const CATEGORY_CACHE_TTL_SECS: u64 = 3600;

/// Cache key prefixes
const CACHE_KEY_CATEGORY_BY_ID: &str = "category:id:";
const CACHE_KEY_CATEGORY_BY_SLUG: &str = "category:slug:";
const CACHE_KEY_CATEGORY_TREE: &str = "category:tree";
const CACHE_KEY_CATEGORY_LIST: &str = "category:list";

/// Error types for category service operations
#[derive(Debug, thiserror::Error)]
pub enum CategoryServiceError {
    /// Category name already exists
    #[error("Category name already exists: {0}")]
    DuplicateName(String),

    /// Category slug already exists
    #[error("Category slug already exists: {0}")]
    DuplicateSlug(String),

    /// Category not found
    #[error("Category not found: {0}")]
    NotFound(String),

    /// Cannot delete default category
    #[error("Cannot delete the default category")]
    CannotDeleteDefault,

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Parent category not found
    #[error("Parent category not found: {0}")]
    ParentNotFound(i64),

    /// Circular reference detected
    #[error("Circular reference detected: category cannot be its own ancestor")]
    CircularReference,

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

/// Category service for managing blog categories
pub struct CategoryService {
    repo: Arc<dyn CategoryRepository>,
    cache: Arc<Cache>,
    pool: DynDatabasePool,
    cache_ttl: Duration,
}

impl CategoryService {
    /// Create a new category service
    pub fn new(
        repo: Arc<dyn CategoryRepository>,
        cache: Arc<Cache>,
        pool: DynDatabasePool,
    ) -> Self {
        Self {
            repo,
            cache,
            pool,
            cache_ttl: Duration::from_secs(CATEGORY_CACHE_TTL_SECS),
        }
    }

    /// Create a new category service with custom cache TTL
    pub fn with_cache_ttl(
        repo: Arc<dyn CategoryRepository>,
        cache: Arc<Cache>,
        pool: DynDatabasePool,
        cache_ttl: Duration,
    ) -> Self {
        Self {
            repo,
            cache,
            pool,
            cache_ttl,
        }
    }

    /// Create a new category
    ///
    /// # Arguments
    /// * `input` - Category creation input
    ///
    /// # Returns
    /// The created category
    ///
    /// # Errors
    /// - `DuplicateName` if a category with the same name already exists (Requirement 2.5)
    /// - `DuplicateSlug` if a category with the same slug already exists
    /// - `ParentNotFound` if the specified parent category doesn't exist
    ///
    /// Satisfies requirements:
    /// - 2.1: WHEN 用户创建分类 THEN Category_Service SHALL 创建分类记录并支持设置父分类
    /// - 2.5: IF 分类名称已存�?THEN Category_Service SHALL 返回重复错误
    pub async fn create(&self, input: CreateCategoryInput) -> Result<Category, CategoryServiceError> {
        // Validate input
        self.validate_create_input(&input)?;

        // Check name uniqueness (Requirement 2.5)
        if self.repo.exists_by_name(&input.name).await.context("Failed to check name uniqueness")? {
            return Err(CategoryServiceError::DuplicateName(input.name));
        }

        // Generate slug if not provided
        let slug = input.slug.unwrap_or_else(|| generate_slug(&input.name));

        // Check slug uniqueness
        if self.repo.exists_by_slug(&slug).await.context("Failed to check slug uniqueness")? {
            return Err(CategoryServiceError::DuplicateSlug(slug));
        }

        // Validate parent exists if specified (Requirement 2.1)
        if let Some(parent_id) = input.parent_id {
            if self.repo.get_by_id(parent_id).await.context("Failed to get parent category")?.is_none() {
                return Err(CategoryServiceError::ParentNotFound(parent_id));
            }
        }

        // Create category
        let category = Category::new(
            slug,
            input.name,
            input.description,
            input.parent_id,
            input.sort_order.unwrap_or(0),
        );

        let created = self.repo.create(&category).await.context("Failed to create category")?;

        // Invalidate cache
        self.invalidate_cache().await?;

        Ok(created)
    }

    /// Get category by ID
    ///
    /// # Arguments
    /// * `id` - Category ID
    ///
    /// # Returns
    /// The category if found, None otherwise
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Category>, CategoryServiceError> {
        // Try cache first
        let cache_key = format!("{}{}", CACHE_KEY_CATEGORY_BY_ID, id);
        if let Some(category) = self.cache.get::<Category>(&cache_key).await.ok().flatten() {
            return Ok(Some(category));
        }

        // Get from database
        let category = self.repo.get_by_id(id).await.context("Failed to get category by ID")?;

        // Cache the result
        if let Some(ref cat) = category {
            let _ = self.cache.set(&cache_key, cat, self.cache_ttl).await;
        }

        Ok(category)
    }

    /// Get category by slug
    ///
    /// # Arguments
    /// * `slug` - Category slug
    ///
    /// # Returns
    /// The category if found, None otherwise
    pub async fn get_by_slug(&self, slug: &str) -> Result<Option<Category>, CategoryServiceError> {
        // Try cache first
        let cache_key = format!("{}{}", CACHE_KEY_CATEGORY_BY_SLUG, slug);
        if let Some(category) = self.cache.get::<Category>(&cache_key).await.ok().flatten() {
            return Ok(Some(category));
        }

        // Get from database
        let category = self.repo.get_by_slug(slug).await.context("Failed to get category by slug")?;

        // Cache the result
        if let Some(ref cat) = category {
            let _ = self.cache.set(&cache_key, cat, self.cache_ttl).await;
        }

        Ok(category)
    }

    /// Get category tree (hierarchical structure)
    ///
    /// Returns all categories organized in a tree structure.
    ///
    /// Satisfies requirement 2.3: WHEN 用户请求某分类下的文�?THEN Category_Service SHALL 返回该分类及其子分类下的所有文�?
    pub async fn list_tree(&self) -> Result<Vec<CategoryTree>, CategoryServiceError> {
        // Try cache first
        if let Some(tree) = self.cache.get::<Vec<CategoryTree>>(CACHE_KEY_CATEGORY_TREE).await.ok().flatten() {
            return Ok(tree);
        }

        // Get from database
        let tree = self.repo.list_tree().await.context("Failed to get category tree")?;

        // Cache the result
        let _ = self.cache.set(CACHE_KEY_CATEGORY_TREE, &tree, self.cache_ttl).await;

        Ok(tree)
    }

    /// List all categories (flat list)
    pub async fn list(&self) -> Result<Vec<Category>, CategoryServiceError> {
        // Try cache first
        if let Some(list) = self.cache.get::<Vec<Category>>(CACHE_KEY_CATEGORY_LIST).await.ok().flatten() {
            return Ok(list);
        }

        // Get from database
        let list = self.repo.list().await.context("Failed to list categories")?;

        // Cache the result
        let _ = self.cache.set(CACHE_KEY_CATEGORY_LIST, &list, self.cache_ttl).await;

        Ok(list)
    }

    /// Get all descendant category IDs (including the given category)
    ///
    /// This is useful for querying articles in a category and all its subcategories.
    ///
    /// Satisfies requirement 2.3: WHEN 用户请求某分类下的文�?THEN Category_Service SHALL 返回该分类及其子分类下的所有文�?
    pub async fn get_all_descendants(&self, id: i64) -> Result<Vec<i64>, CategoryServiceError> {
        self.repo.get_all_descendants(id).await.context("Failed to get descendants").map_err(Into::into)
    }

    /// Update a category
    ///
    /// # Arguments
    /// * `id` - Category ID to update
    /// * `input` - Update input
    ///
    /// # Returns
    /// The updated category
    ///
    /// # Errors
    /// - `NotFound` if the category doesn't exist
    /// - `DuplicateName` if the new name already exists (Requirement 2.5)
    /// - `DuplicateSlug` if the new slug already exists
    /// - `CircularReference` if the new parent would create a cycle
    ///
    /// Satisfies requirement 2.5: IF 分类名称已存�?THEN Category_Service SHALL 返回重复错误
    pub async fn update(&self, id: i64, input: UpdateCategoryInput) -> Result<Category, CategoryServiceError> {
        // Get existing category
        let mut category = self.repo.get_by_id(id)
            .await
            .context("Failed to get category")?
            .ok_or_else(|| CategoryServiceError::NotFound(format!("Category with ID {} not found", id)))?;

        // Check name uniqueness if name is being changed (Requirement 2.5)
        if let Some(ref new_name) = input.name {
            if new_name != &category.name {
                if self.repo.exists_by_name(new_name).await.context("Failed to check name uniqueness")? {
                    return Err(CategoryServiceError::DuplicateName(new_name.clone()));
                }
                category.name = new_name.clone();
            }
        }

        // Check slug uniqueness if slug is being changed
        if let Some(ref new_slug) = input.slug {
            if new_slug != &category.slug {
                if self.repo.exists_by_slug(new_slug).await.context("Failed to check slug uniqueness")? {
                    return Err(CategoryServiceError::DuplicateSlug(new_slug.clone()));
                }
                category.slug = new_slug.clone();
            }
        }

        // Update description if provided
        if let Some(ref new_description) = input.description {
            category.description = new_description.clone();
        }

        // Update parent if provided
        if let Some(new_parent_id) = input.parent_id {
            // Validate parent exists if specified
            if let Some(parent_id) = new_parent_id {
                if self.repo.get_by_id(parent_id).await.context("Failed to get parent category")?.is_none() {
                    return Err(CategoryServiceError::ParentNotFound(parent_id));
                }
                // Check for circular reference
                if self.would_create_cycle(id, parent_id).await? {
                    return Err(CategoryServiceError::CircularReference);
                }
            }
            category.parent_id = new_parent_id;
        }

        // Update sort order if provided
        if let Some(new_sort_order) = input.sort_order {
            category.sort_order = new_sort_order;
        }

        // Save changes
        let updated = self.repo.update(&category).await.context("Failed to update category")?;

        // Invalidate cache
        self.invalidate_cache().await?;

        Ok(updated)
    }

    /// Delete a category
    ///
    /// When a category is deleted, all articles in that category are moved
    /// to the default "uncategorized" category.
    ///
    /// # Arguments
    /// * `id` - Category ID to delete
    ///
    /// # Errors
    /// - `NotFound` if the category doesn't exist
    /// - `CannotDeleteDefault` if trying to delete the default category
    ///
    /// Satisfies requirement 2.4: WHEN 用户删除分类 THEN Category_Service SHALL 将该分类下的文章移至默认分类
    pub async fn delete(&self, id: i64) -> Result<(), CategoryServiceError> {
        // Get the category to delete
        let category = self.repo.get_by_id(id)
            .await
            .context("Failed to get category")?
            .ok_or_else(|| CategoryServiceError::NotFound(format!("Category with ID {} not found", id)))?;

        // Cannot delete the default category
        if category.is_default() {
            return Err(CategoryServiceError::CannotDeleteDefault);
        }

        // Get the default category
        let default_category = self.repo.get_default()
            .await
            .context("Failed to get default category")?
            .ok_or_else(|| CategoryServiceError::NotFound("Default category not found".to_string()))?;

        // Get all descendant category IDs (including this one)
        let descendant_ids = self.repo.get_all_descendants(id)
            .await
            .context("Failed to get descendant categories")?;

        // Move articles from this category and all descendants to default category (Requirement 2.4)
        self.move_articles_to_category(&descendant_ids, default_category.id).await?;

        // Update children to have no parent (or move to default parent)
        // First, get direct children and update their parent to this category's parent
        let children = self.repo.get_children(id).await.context("Failed to get children")?;
        for mut child in children {
            child.parent_id = category.parent_id;
            self.repo.update(&child).await.context("Failed to update child category")?;
        }

        // Delete the category
        self.repo.delete(id).await.context("Failed to delete category")?;

        // Invalidate cache
        self.invalidate_cache().await?;

        Ok(())
    }

    /// Get the default category (uncategorized)
    pub async fn get_default(&self) -> Result<Option<Category>, CategoryServiceError> {
        self.repo.get_default().await.context("Failed to get default category").map_err(Into::into)
    }

    /// Check if a category name already exists
    ///
    /// Satisfies requirement 2.5: IF 分类名称已存�?THEN Category_Service SHALL 返回重复错误
    pub async fn exists_by_name(&self, name: &str) -> Result<bool, CategoryServiceError> {
        self.repo.exists_by_name(name).await.context("Failed to check name existence").map_err(Into::into)
    }

    /// Check if a category slug already exists
    pub async fn exists_by_slug(&self, slug: &str) -> Result<bool, CategoryServiceError> {
        self.repo.exists_by_slug(slug).await.context("Failed to check slug existence").map_err(Into::into)
    }

    // ========================================================================
    // Private helper methods
    // ========================================================================

    /// Validate category creation input
    fn validate_create_input(&self, input: &CreateCategoryInput) -> Result<(), CategoryServiceError> {
        if input.name.trim().is_empty() {
            return Err(CategoryServiceError::ValidationError("Category name cannot be empty".to_string()));
        }

        if let Some(ref slug) = input.slug {
            if slug.trim().is_empty() {
                return Err(CategoryServiceError::ValidationError("Category slug cannot be empty".to_string()));
            }
        }

        Ok(())
    }

    /// Check if setting parent_id would create a circular reference
    async fn would_create_cycle(&self, category_id: i64, new_parent_id: i64) -> Result<bool, CategoryServiceError> {
        // A category cannot be its own parent
        if category_id == new_parent_id {
            return Ok(true);
        }

        // Check if the new parent is a descendant of this category
        let descendants = self.repo.get_all_descendants(category_id)
            .await
            .context("Failed to get descendants")?;

        Ok(descendants.contains(&new_parent_id))
    }

    /// Move articles from specified categories to a target category
    ///
    /// This is called when deleting a category to move its articles to the default category.
    ///
    /// Satisfies requirement 2.4: WHEN 用户删除分类 THEN Category_Service SHALL 将该分类下的文章移至默认分类
    async fn move_articles_to_category(&self, from_category_ids: &[i64], to_category_id: i64) -> Result<(), CategoryServiceError> {
        use crate::config::DatabaseDriver;

        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                let pool = self.pool.as_sqlite().unwrap();
                for category_id in from_category_ids {
                    sqlx::query("UPDATE articles SET category_id = ? WHERE category_id = ?")
                        .bind(to_category_id)
                        .bind(category_id)
                        .execute(pool)
                        .await
                        .context("Failed to move articles to default category")?;
                }
            }
            DatabaseDriver::Mysql => {
                let pool = self.pool.as_mysql().unwrap();
                for category_id in from_category_ids {
                    sqlx::query("UPDATE articles SET category_id = ? WHERE category_id = ?")
                        .bind(to_category_id)
                        .bind(category_id)
                        .execute(pool)
                        .await
                        .context("Failed to move articles to default category")?;
                }
            }
        }

        Ok(())
    }

    /// Invalidate all category-related cache entries
    async fn invalidate_cache(&self) -> Result<(), CategoryServiceError> {
        // Delete pattern-based cache entries
        let _ = self.cache.delete_pattern(&format!("{}*", CACHE_KEY_CATEGORY_BY_ID)).await;
        let _ = self.cache.delete_pattern(&format!("{}*", CACHE_KEY_CATEGORY_BY_SLUG)).await;
        let _ = self.cache.delete(CACHE_KEY_CATEGORY_TREE).await;
        let _ = self.cache.delete(CACHE_KEY_CATEGORY_LIST).await;

        Ok(())
    }
}

/// Input for creating a new category
#[derive(Debug, Clone)]
pub struct CreateCategoryInput {
    /// Category name (required)
    pub name: String,
    /// URL-friendly slug (optional, generated from name if not provided)
    pub slug: Option<String>,
    /// Category description (optional)
    pub description: Option<String>,
    /// Parent category ID (optional, for hierarchical structure)
    pub parent_id: Option<i64>,
    /// Sort order within parent (optional, defaults to 0)
    pub sort_order: Option<i32>,
}

impl CreateCategoryInput {
    /// Create a new category input with just a name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            slug: None,
            description: None,
            parent_id: None,
            sort_order: None,
        }
    }

    /// Set the slug
    pub fn with_slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the parent category ID
    pub fn with_parent(mut self, parent_id: i64) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set the sort order
    pub fn with_sort_order(mut self, sort_order: i32) -> Self {
        self.sort_order = Some(sort_order);
        self
    }
}

/// Input for updating a category
#[derive(Debug, Clone, Default)]
pub struct UpdateCategoryInput {
    /// New name (optional)
    pub name: Option<String>,
    /// New slug (optional)
    pub slug: Option<String>,
    /// New description (optional)
    pub description: Option<Option<String>>,
    /// New parent ID (optional, Some(None) to make root)
    pub parent_id: Option<Option<i64>>,
    /// New sort order (optional)
    pub sort_order: Option<i32>,
}

impl UpdateCategoryInput {
    /// Create an empty update input
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the slug
    pub fn with_slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = Some(description);
        self
    }

    /// Set the parent ID
    pub fn with_parent(mut self, parent_id: Option<i64>) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set the sort order
    pub fn with_sort_order(mut self, sort_order: i32) -> Self {
        self.sort_order = Some(sort_order);
        self
    }
}

/// Generate a URL-friendly slug from a name
///
/// Converts the name to lowercase, replaces spaces and special characters
/// with hyphens, and removes consecutive hyphens.
pub fn generate_slug(name: &str) -> String {
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
    use crate::cache::create_cache;
    use crate::config::CacheConfig;
    use crate::db::repositories::SqlxCategoryRepository;
    use crate::db::{create_test_pool, migrations};
    use proptest::prelude::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    async fn setup_test_service() -> (DynDatabasePool, CategoryService) {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let repo = SqlxCategoryRepository::boxed(pool.clone());
        let cache = create_cache(&CacheConfig::default()).await.expect("Failed to create cache");
        let service = CategoryService::new(repo, cache, pool.clone());

        (pool, service)
    }

    // ========================================================================
    // Slug generation tests
    // ========================================================================

    #[test]
    fn test_generate_slug_simple() {
        assert_eq!(generate_slug("Hello World"), "hello-world");
    }

    #[test]
    fn test_generate_slug_with_special_chars() {
        assert_eq!(generate_slug("Hello, World!"), "hello-world");
    }

    #[test]
    fn test_generate_slug_with_multiple_spaces() {
        assert_eq!(generate_slug("Hello   World"), "hello-world");
    }

    #[test]
    fn test_generate_slug_with_underscores() {
        assert_eq!(generate_slug("hello_world"), "hello-world");
    }

    #[test]
    fn test_generate_slug_chinese() {
        // Chinese characters should be preserved
        let slug = generate_slug("技术分类");
        assert_eq!(slug, "技术分类");
    }

    #[test]
    fn test_generate_slug_mixed() {
        let slug = generate_slug("Tech 技术");
        assert_eq!(slug, "tech-技术");
    }

    // ========================================================================
    // Create category tests
    // ========================================================================

    #[tokio::test]
    async fn test_create_category_success() {
        let (_pool, service) = setup_test_service().await;

        let input = CreateCategoryInput::new("Test Category")
            .with_description("A test category");

        let category = service.create(input).await.expect("Failed to create category");

        assert!(category.id > 0);
        assert_eq!(category.name, "Test Category");
        assert_eq!(category.slug, "test-category");
        assert_eq!(category.description, Some("A test category".to_string()));
        assert!(category.parent_id.is_none());
    }

    #[tokio::test]
    async fn test_create_category_with_custom_slug() {
        let (_pool, service) = setup_test_service().await;

        let input = CreateCategoryInput::new("Test Category")
            .with_slug("custom-slug");

        let category = service.create(input).await.expect("Failed to create category");

        assert_eq!(category.slug, "custom-slug");
    }

    #[tokio::test]
    async fn test_create_category_with_parent() {
        let (_pool, service) = setup_test_service().await;

        // Create parent
        let parent_input = CreateCategoryInput::new("Parent Category");
        let parent = service.create(parent_input).await.expect("Failed to create parent");

        // Create child
        let child_input = CreateCategoryInput::new("Child Category")
            .with_parent(parent.id);
        let child = service.create(child_input).await.expect("Failed to create child");

        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[tokio::test]
    async fn test_create_category_duplicate_name_fails() {
        let (_pool, service) = setup_test_service().await;

        let input1 = CreateCategoryInput::new("Duplicate Name");
        service.create(input1).await.expect("Failed to create first category");

        let input2 = CreateCategoryInput::new("Duplicate Name");
        let result = service.create(input2).await;

        assert!(matches!(result, Err(CategoryServiceError::DuplicateName(_))));
    }

    #[tokio::test]
    async fn test_create_category_duplicate_slug_fails() {
        let (_pool, service) = setup_test_service().await;

        let input1 = CreateCategoryInput::new("Category One")
            .with_slug("same-slug");
        service.create(input1).await.expect("Failed to create first category");

        let input2 = CreateCategoryInput::new("Category Two")
            .with_slug("same-slug");
        let result = service.create(input2).await;

        assert!(matches!(result, Err(CategoryServiceError::DuplicateSlug(_))));
    }

    #[tokio::test]
    async fn test_create_category_empty_name_fails() {
        let (_pool, service) = setup_test_service().await;

        let input = CreateCategoryInput::new("");
        let result = service.create(input).await;

        assert!(matches!(result, Err(CategoryServiceError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_create_category_nonexistent_parent_fails() {
        let (_pool, service) = setup_test_service().await;

        let input = CreateCategoryInput::new("Orphan Category")
            .with_parent(99999);
        let result = service.create(input).await;

        assert!(matches!(result, Err(CategoryServiceError::ParentNotFound(_))));
    }

    // ========================================================================
    // Get category tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_by_id_success() {
        let (_pool, service) = setup_test_service().await;

        let input = CreateCategoryInput::new("Get By ID Test");
        let created = service.create(input).await.expect("Failed to create category");

        let found = service.get_by_id(created.id).await.expect("Failed to get category")
            .expect("Category not found");

        assert_eq!(found.id, created.id);
        assert_eq!(found.name, "Get By ID Test");
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let (_pool, service) = setup_test_service().await;

        let result = service.get_by_id(99999).await.expect("Failed to get category");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_slug_success() {
        let (_pool, service) = setup_test_service().await;

        let input = CreateCategoryInput::new("Get By Slug Test");
        service.create(input).await.expect("Failed to create category");

        let found = service.get_by_slug("get-by-slug-test").await.expect("Failed to get category")
            .expect("Category not found");

        assert_eq!(found.slug, "get-by-slug-test");
    }

    #[tokio::test]
    async fn test_get_by_slug_not_found() {
        let (_pool, service) = setup_test_service().await;

        let result = service.get_by_slug("nonexistent").await.expect("Failed to get category");
        assert!(result.is_none());
    }

    // ========================================================================
    // List and tree tests
    // ========================================================================

    #[tokio::test]
    async fn test_list_categories() {
        let (_pool, service) = setup_test_service().await;

        // Create some categories
        service.create(CreateCategoryInput::new("Category A")).await.expect("Failed to create");
        service.create(CreateCategoryInput::new("Category B")).await.expect("Failed to create");

        let categories = service.list().await.expect("Failed to list categories");

        // Should include default "uncategorized" plus our 2
        assert!(categories.len() >= 3);
    }

    #[tokio::test]
    async fn test_list_tree() {
        let (_pool, service) = setup_test_service().await;

        // Create a hierarchy
        let parent = service.create(CreateCategoryInput::new("Parent")).await.expect("Failed to create parent");
        service.create(CreateCategoryInput::new("Child 1").with_parent(parent.id)).await.expect("Failed to create child1");
        service.create(CreateCategoryInput::new("Child 2").with_parent(parent.id)).await.expect("Failed to create child2");

        let tree = service.list_tree().await.expect("Failed to get tree");

        // Find our parent in the tree
        let parent_tree = tree.iter().find(|t| t.category.name == "Parent");
        assert!(parent_tree.is_some());
        assert_eq!(parent_tree.unwrap().children.len(), 2);
    }

    #[tokio::test]
    async fn test_get_all_descendants() {
        let (_pool, service) = setup_test_service().await;

        // Create a hierarchy: root -> child -> grandchild
        let root = service.create(CreateCategoryInput::new("Root")).await.expect("Failed to create root");
        let child = service.create(CreateCategoryInput::new("Child").with_parent(root.id)).await.expect("Failed to create child");
        let grandchild = service.create(CreateCategoryInput::new("Grandchild").with_parent(child.id)).await.expect("Failed to create grandchild");

        let descendants = service.get_all_descendants(root.id).await.expect("Failed to get descendants");

        assert_eq!(descendants.len(), 3);
        assert!(descendants.contains(&root.id));
        assert!(descendants.contains(&child.id));
        assert!(descendants.contains(&grandchild.id));
    }

    // ========================================================================
    // Update category tests
    // ========================================================================

    #[tokio::test]
    async fn test_update_category_name() {
        let (_pool, service) = setup_test_service().await;

        let input = CreateCategoryInput::new("Original Name");
        let created = service.create(input).await.expect("Failed to create category");

        let update = UpdateCategoryInput::new().with_name("Updated Name");
        let updated = service.update(created.id, update).await.expect("Failed to update category");

        assert_eq!(updated.name, "Updated Name");
    }

    #[tokio::test]
    async fn test_update_category_slug() {
        let (_pool, service) = setup_test_service().await;

        let input = CreateCategoryInput::new("Test Category");
        let created = service.create(input).await.expect("Failed to create category");

        let update = UpdateCategoryInput::new().with_slug("new-slug");
        let updated = service.update(created.id, update).await.expect("Failed to update category");

        assert_eq!(updated.slug, "new-slug");
    }

    #[tokio::test]
    async fn test_update_category_duplicate_name_fails() {
        let (_pool, service) = setup_test_service().await;

        service.create(CreateCategoryInput::new("Existing Name")).await.expect("Failed to create first");
        let second = service.create(CreateCategoryInput::new("Second Category")).await.expect("Failed to create second");

        let update = UpdateCategoryInput::new().with_name("Existing Name");
        let result = service.update(second.id, update).await;

        assert!(matches!(result, Err(CategoryServiceError::DuplicateName(_))));
    }

    #[tokio::test]
    async fn test_update_category_not_found() {
        let (_pool, service) = setup_test_service().await;

        let update = UpdateCategoryInput::new().with_name("New Name");
        let result = service.update(99999, update).await;

        assert!(matches!(result, Err(CategoryServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_update_category_circular_reference_fails() {
        let (_pool, service) = setup_test_service().await;

        // Create parent -> child hierarchy
        let parent = service.create(CreateCategoryInput::new("Parent")).await.expect("Failed to create parent");
        let child = service.create(CreateCategoryInput::new("Child").with_parent(parent.id)).await.expect("Failed to create child");

        // Try to make parent a child of child (circular reference)
        let update = UpdateCategoryInput::new().with_parent(Some(child.id));
        let result = service.update(parent.id, update).await;

        assert!(matches!(result, Err(CategoryServiceError::CircularReference)));
    }

    #[tokio::test]
    async fn test_update_category_self_parent_fails() {
        let (_pool, service) = setup_test_service().await;

        let category = service.create(CreateCategoryInput::new("Self Parent")).await.expect("Failed to create");

        // Try to make category its own parent
        let update = UpdateCategoryInput::new().with_parent(Some(category.id));
        let result = service.update(category.id, update).await;

        assert!(matches!(result, Err(CategoryServiceError::CircularReference)));
    }

    // ========================================================================
    // Delete category tests
    // ========================================================================

    #[tokio::test]
    async fn test_delete_category_success() {
        let (_pool, service) = setup_test_service().await;

        let input = CreateCategoryInput::new("To Delete");
        let created = service.create(input).await.expect("Failed to create category");

        service.delete(created.id).await.expect("Failed to delete category");

        let found = service.get_by_id(created.id).await.expect("Failed to get category");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_category_not_found() {
        let (_pool, service) = setup_test_service().await;

        let result = service.delete(99999).await;
        assert!(matches!(result, Err(CategoryServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_default_category_fails() {
        let (_pool, service) = setup_test_service().await;

        let default = service.get_default().await.expect("Failed to get default")
            .expect("Default category not found");

        let result = service.delete(default.id).await;
        assert!(matches!(result, Err(CategoryServiceError::CannotDeleteDefault)));
    }

    #[tokio::test]
    async fn test_delete_category_children_reparented() {
        let (_pool, service) = setup_test_service().await;

        // Create grandparent -> parent -> child hierarchy
        let grandparent = service.create(CreateCategoryInput::new("Grandparent")).await.expect("Failed to create grandparent");
        let parent = service.create(CreateCategoryInput::new("Parent").with_parent(grandparent.id)).await.expect("Failed to create parent");
        let child = service.create(CreateCategoryInput::new("Child").with_parent(parent.id)).await.expect("Failed to create child");

        // Delete parent
        service.delete(parent.id).await.expect("Failed to delete parent");

        // Child should now have grandparent as parent
        let updated_child = service.get_by_id(child.id).await.expect("Failed to get child")
            .expect("Child not found");
        assert_eq!(updated_child.parent_id, Some(grandparent.id));
    }

    // ========================================================================
    // Cache tests
    // ========================================================================

    #[tokio::test]
    async fn test_cache_invalidation_on_create() {
        let (_pool, service) = setup_test_service().await;

        // Populate cache
        let _ = service.list().await;

        // Create new category
        service.create(CreateCategoryInput::new("New Category")).await.expect("Failed to create");

        // List should include new category (cache should be invalidated)
        let categories = service.list().await.expect("Failed to list");
        assert!(categories.iter().any(|c| c.name == "New Category"));
    }

    #[tokio::test]
    async fn test_cache_invalidation_on_update() {
        let (_pool, service) = setup_test_service().await;

        let created = service.create(CreateCategoryInput::new("Original")).await.expect("Failed to create");

        // Populate cache
        let _ = service.get_by_id(created.id).await;

        // Update category
        service.update(created.id, UpdateCategoryInput::new().with_name("Updated")).await.expect("Failed to update");

        // Get should return updated name
        let found = service.get_by_id(created.id).await.expect("Failed to get")
            .expect("Category not found");
        assert_eq!(found.name, "Updated");
    }

    #[tokio::test]
    async fn test_cache_invalidation_on_delete() {
        let (_pool, service) = setup_test_service().await;

        let created = service.create(CreateCategoryInput::new("To Delete")).await.expect("Failed to create");

        // Populate cache
        let _ = service.get_by_id(created.id).await;

        // Delete category
        service.delete(created.id).await.expect("Failed to delete");

        // Get should return None
        let found = service.get_by_id(created.id).await.expect("Failed to get");
        assert!(found.is_none());
    }

    // ========================================================================
    // Property-Based Tests
    // ========================================================================

    /// Counter for generating unique test data across property test iterations
    static PROPERTY_TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Generate a unique suffix for test data
    fn unique_suffix() -> u64 {
        PROPERTY_TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// Setup a fresh test service for property tests
    async fn setup_property_test_service() -> CategoryService {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let repo = SqlxCategoryRepository::boxed(pool.clone());
        let cache = create_cache(&CacheConfig::default()).await.expect("Failed to create cache");
        CategoryService::new(repo, cache, pool)
    }

    // ========================================================================
    // Property 5: 分类层级查询完整�?(Category Hierarchy Query Completeness)
    // For any category tree structure, querying a parent category's articles
    // should return articles from that category and all its subcategories,
    // with no omissions.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Validates: Requirements 2.3**
        ///
        /// Property 5: Category Hierarchy Query Completeness
        /// For any category tree structure, querying get_all_descendants() for a parent
        /// category should return all descendants including the parent itself, with no omissions.
        #[test]
        fn property_5_category_hierarchy_query_completeness(
            depth in 1..4usize,
            children_per_level in 1..3usize
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let service = setup_property_test_service().await;
                let suffix = unique_suffix();

                // Build a category tree with the specified depth and breadth
                // Track all created category IDs
                let mut all_category_ids: Vec<i64> = Vec::new();

                // Create root category
                let root_name = format!("Root_{}_{}", suffix, 0);
                let root_input = CreateCategoryInput::new(root_name);
                let root = service.create(root_input).await
                    .expect("Failed to create root category");
                all_category_ids.push(root.id);

                // Build tree level by level
                let mut current_level_ids = vec![root.id];

                for level in 1..depth {
                    let mut next_level_ids = Vec::new();

                    for (parent_idx, &parent_id) in current_level_ids.iter().enumerate() {
                        for child_idx in 0..children_per_level {
                            let child_name = format!("Cat_{}_L{}_P{}_C{}", suffix, level, parent_idx, child_idx);
                            let child_input = CreateCategoryInput::new(child_name)
                                .with_parent(parent_id);
                            let child = service.create(child_input).await
                                .expect("Failed to create child category");
                            all_category_ids.push(child.id);
                            next_level_ids.push(child.id);
                        }
                    }

                    current_level_ids = next_level_ids;
                }

                // Query descendants from root
                let descendants = service.get_all_descendants(root.id).await
                    .expect("Failed to get descendants");

                // Property: All created category IDs should be in descendants
                for &cat_id in &all_category_ids {
                    prop_assert!(
                        descendants.contains(&cat_id),
                        "Category ID {} should be in descendants but was not found. \
                         Expected {} categories, got {}. \
                         All IDs: {:?}, Descendants: {:?}",
                        cat_id,
                        all_category_ids.len(),
                        descendants.len(),
                        all_category_ids,
                        descendants
                    );
                }

                // Property: Descendants count should match created count
                prop_assert_eq!(
                    descendants.len(),
                    all_category_ids.len(),
                    "Descendant count should match created category count"
                );

                // Property: No duplicate IDs in descendants
                let mut unique_descendants = descendants.clone();
                unique_descendants.sort();
                unique_descendants.dedup();
                prop_assert_eq!(
                    unique_descendants.len(),
                    descendants.len(),
                    "Descendants should not contain duplicates"
                );

                Ok(())
            });
            result?;
        }
    }

    // ========================================================================
    // Property 6: 分类名称唯一�?(Category Name Uniqueness)
    // For any existing category name, attempting to create a category with
    // the same name should return a duplicate error.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Validates: Requirements 2.5**
        ///
        /// Property 6: Category Name Uniqueness
        /// For any existing category name, attempting to create a category with
        /// the same name should return a DuplicateName error.
        #[test]
        fn property_6_category_name_uniqueness(
            name_base in "[a-zA-Z]{3,15}",
            description1 in proptest::option::of("[a-zA-Z ]{5,30}"),
            description2 in proptest::option::of("[a-zA-Z ]{5,30}")
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let service = setup_property_test_service().await;
                let suffix = unique_suffix();

                // Create a unique category name for this iteration
                let category_name = format!("{}_{}", name_base, suffix);

                // Create the first category with this name
                let input1 = CreateCategoryInput::new(category_name.clone());
                let input1 = if let Some(desc) = description1 {
                    input1.with_description(desc)
                } else {
                    input1
                };

                let first_category = service.create(input1).await
                    .expect("First category creation should succeed");

                // Verify the category was created with the correct name
                prop_assert_eq!(
                    &first_category.name,
                    &category_name,
                    "Created category should have the specified name"
                );

                // Attempt to create a second category with the same name
                let input2 = CreateCategoryInput::new(category_name.clone());
                let input2 = if let Some(desc) = description2 {
                    input2.with_description(desc)
                } else {
                    input2
                };

                let second_result = service.create(input2).await;

                // Property: Second creation should fail with DuplicateName error
                prop_assert!(
                    matches!(second_result, Err(CategoryServiceError::DuplicateName(ref n)) if n == &category_name),
                    "Creating a category with duplicate name should return DuplicateName error. \
                     Got: {:?}",
                    second_result
                );

                // Property: exists_by_name should return true for the existing name
                let exists = service.exists_by_name(&category_name).await
                    .expect("exists_by_name should not error");
                prop_assert!(
                    exists,
                    "exists_by_name should return true for existing category name"
                );

                // Property: The original category should still be retrievable
                let retrieved = service.get_by_id(first_category.id).await
                    .expect("get_by_id should not error")
                    .expect("Original category should still exist");
                prop_assert_eq!(
                    &retrieved.name,
                    &category_name,
                    "Original category should be unchanged after duplicate attempt"
                );

                Ok(())
            });
            result?;
        }
    }
}
