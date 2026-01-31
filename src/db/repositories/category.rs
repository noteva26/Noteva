//! Category repository
//!
//! Database operations for categories.
//!
//! This module provides:
//! - `CategoryRepository` trait defining the interface for category data access
//! - `SqlxCategoryRepository` implementing the trait for SQLite and MySQL
//!
//! Satisfies requirements:
//! - 2.1: WHEN 用户创建分类 THEN Category_Service SHALL 创建分类记录并支持设置父分类
//! - 2.3: WHEN 用户请求某分类下的文章 THEN Category_Service SHALL 返回该分类及其子分类下的所有文章

use crate::config::DatabaseDriver;
use crate::db::DynDatabasePool;
use crate::models::{Category, CategoryTree};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{MySqlPool, Row, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;

/// Category repository trait
#[async_trait]
pub trait CategoryRepository: Send + Sync {
    /// Create a new category
    async fn create(&self, category: &Category) -> Result<Category>;

    /// Get category by ID
    async fn get_by_id(&self, id: i64) -> Result<Option<Category>>;

    /// Get category by slug
    async fn get_by_slug(&self, slug: &str) -> Result<Option<Category>>;

    /// Get category by name
    async fn get_by_name(&self, name: &str) -> Result<Option<Category>>;

    /// List all categories (flat list)
    async fn list(&self) -> Result<Vec<Category>>;

    /// List all categories as a tree structure
    async fn list_tree(&self) -> Result<Vec<CategoryTree>>;

    /// Get direct children of a category
    async fn get_children(&self, parent_id: i64) -> Result<Vec<Category>>;

    /// Get all descendants of a category (recursive)
    /// Returns all category IDs including the given category ID
    async fn get_all_descendants(&self, id: i64) -> Result<Vec<i64>>;

    /// Update a category
    async fn update(&self, category: &Category) -> Result<Category>;

    /// Delete a category
    async fn delete(&self, id: i64) -> Result<()>;

    /// Check if a category name already exists
    async fn exists_by_name(&self, name: &str) -> Result<bool>;

    /// Check if a category slug already exists
    async fn exists_by_slug(&self, slug: &str) -> Result<bool>;

    /// Get the default category (uncategorized)
    async fn get_default(&self) -> Result<Option<Category>>;
}

/// SQLx-based category repository implementation
///
/// Supports both SQLite and MySQL databases.
pub struct SqlxCategoryRepository {
    pool: DynDatabasePool,
}

impl SqlxCategoryRepository {
    /// Create a new SQLx category repository
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }

    /// Create a boxed repository for use with dependency injection
    pub fn boxed(pool: DynDatabasePool) -> Arc<dyn CategoryRepository> {
        Arc::new(Self::new(pool))
    }
}

#[async_trait]
impl CategoryRepository for SqlxCategoryRepository {
    async fn create(&self, category: &Category) -> Result<Category> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                create_category_sqlite(self.pool.as_sqlite().unwrap(), category).await
            }
            DatabaseDriver::Mysql => {
                create_category_mysql(self.pool.as_mysql().unwrap(), category).await
            }
        }
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<Category>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_category_by_id_sqlite(self.pool.as_sqlite().unwrap(), id).await
            }
            DatabaseDriver::Mysql => {
                get_category_by_id_mysql(self.pool.as_mysql().unwrap(), id).await
            }
        }
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Option<Category>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_category_by_slug_sqlite(self.pool.as_sqlite().unwrap(), slug).await
            }
            DatabaseDriver::Mysql => {
                get_category_by_slug_mysql(self.pool.as_mysql().unwrap(), slug).await
            }
        }
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<Category>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_category_by_name_sqlite(self.pool.as_sqlite().unwrap(), name).await
            }
            DatabaseDriver::Mysql => {
                get_category_by_name_mysql(self.pool.as_mysql().unwrap(), name).await
            }
        }
    }

    async fn list(&self) -> Result<Vec<Category>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => list_categories_sqlite(self.pool.as_sqlite().unwrap()).await,
            DatabaseDriver::Mysql => list_categories_mysql(self.pool.as_mysql().unwrap()).await,
        }
    }

    async fn list_tree(&self) -> Result<Vec<CategoryTree>> {
        // Get all categories and build tree in application layer
        let categories = self.list().await?;
        Ok(build_category_tree(categories))
    }

    async fn get_children(&self, parent_id: i64) -> Result<Vec<Category>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_children_sqlite(self.pool.as_sqlite().unwrap(), parent_id).await
            }
            DatabaseDriver::Mysql => {
                get_children_mysql(self.pool.as_mysql().unwrap(), parent_id).await
            }
        }
    }

    async fn get_all_descendants(&self, id: i64) -> Result<Vec<i64>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_all_descendants_sqlite(self.pool.as_sqlite().unwrap(), id).await
            }
            DatabaseDriver::Mysql => {
                get_all_descendants_mysql(self.pool.as_mysql().unwrap(), id).await
            }
        }
    }

    async fn update(&self, category: &Category) -> Result<Category> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                update_category_sqlite(self.pool.as_sqlite().unwrap(), category).await
            }
            DatabaseDriver::Mysql => {
                update_category_mysql(self.pool.as_mysql().unwrap(), category).await
            }
        }
    }

    async fn delete(&self, id: i64) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => delete_category_sqlite(self.pool.as_sqlite().unwrap(), id).await,
            DatabaseDriver::Mysql => delete_category_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
    }

    async fn exists_by_name(&self, name: &str) -> Result<bool> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                exists_by_name_sqlite(self.pool.as_sqlite().unwrap(), name).await
            }
            DatabaseDriver::Mysql => {
                exists_by_name_mysql(self.pool.as_mysql().unwrap(), name).await
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

    async fn get_default(&self) -> Result<Option<Category>> {
        self.get_by_slug("uncategorized").await
    }
}

// ============================================================================
// Tree building helper
// ============================================================================

/// Build a category tree from a flat list of categories
fn build_category_tree(categories: Vec<Category>) -> Vec<CategoryTree> {
    // Create a map of id -> category
    let mut category_map: HashMap<i64, Category> = HashMap::new();
    for cat in categories {
        category_map.insert(cat.id, cat);
    }

    // Create a map of parent_id -> children
    let mut children_map: HashMap<Option<i64>, Vec<i64>> = HashMap::new();
    for (id, cat) in &category_map {
        children_map
            .entry(cat.parent_id)
            .or_default()
            .push(*id);
    }

    // Sort children by sort_order
    for children in children_map.values_mut() {
        children.sort_by(|a, b| {
            let cat_a = category_map.get(a).unwrap();
            let cat_b = category_map.get(b).unwrap();
            cat_a.sort_order.cmp(&cat_b.sort_order)
        });
    }

    // Build tree recursively starting from root categories (parent_id = None)
    fn build_subtree(
        parent_id: Option<i64>,
        category_map: &HashMap<i64, Category>,
        children_map: &HashMap<Option<i64>, Vec<i64>>,
    ) -> Vec<CategoryTree> {
        let Some(child_ids) = children_map.get(&parent_id) else {
            return Vec::new();
        };

        child_ids
            .iter()
            .filter_map(|id| {
                let category = category_map.get(id)?.clone();
                let children = build_subtree(Some(*id), category_map, children_map);
                Some(CategoryTree::with_children(category, children))
            })
            .collect()
    }

    build_subtree(None, &category_map, &children_map)
}

// ============================================================================
// SQLite implementations
// ============================================================================

async fn create_category_sqlite(pool: &SqlitePool, category: &Category) -> Result<Category> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"
        INSERT INTO categories (slug, name, description, parent_id, sort_order, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&category.slug)
    .bind(&category.name)
    .bind(&category.description)
    .bind(category.parent_id)
    .bind(category.sort_order)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create category")?;

    let id = result.last_insert_rowid();

    Ok(Category {
        id,
        slug: category.slug.clone(),
        name: category.name.clone(),
        description: category.description.clone(),
        parent_id: category.parent_id,
        sort_order: category.sort_order,
        created_at: now,
    })
}

async fn get_category_by_id_sqlite(pool: &SqlitePool, id: i64) -> Result<Option<Category>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, description, parent_id, sort_order, created_at
        FROM categories
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get category by ID")?;

    match row {
        Some(row) => Ok(Some(row_to_category_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn get_category_by_slug_sqlite(pool: &SqlitePool, slug: &str) -> Result<Option<Category>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, description, parent_id, sort_order, created_at
        FROM categories
        WHERE slug = ?
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .context("Failed to get category by slug")?;

    match row {
        Some(row) => Ok(Some(row_to_category_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn get_category_by_name_sqlite(pool: &SqlitePool, name: &str) -> Result<Option<Category>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, description, parent_id, sort_order, created_at
        FROM categories
        WHERE name = ?
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .context("Failed to get category by name")?;

    match row {
        Some(row) => Ok(Some(row_to_category_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn list_categories_sqlite(pool: &SqlitePool) -> Result<Vec<Category>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, name, description, parent_id, sort_order, created_at
        FROM categories
        ORDER BY sort_order, name
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to list categories")?;

    let mut categories = Vec::new();
    for row in rows {
        categories.push(row_to_category_sqlite(&row)?);
    }

    Ok(categories)
}

async fn get_children_sqlite(pool: &SqlitePool, parent_id: i64) -> Result<Vec<Category>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, name, description, parent_id, sort_order, created_at
        FROM categories
        WHERE parent_id = ?
        ORDER BY sort_order, name
        "#,
    )
    .bind(parent_id)
    .fetch_all(pool)
    .await
    .context("Failed to get children categories")?;

    let mut categories = Vec::new();
    for row in rows {
        categories.push(row_to_category_sqlite(&row)?);
    }

    Ok(categories)
}

/// Get all descendants using recursive CTE (SQLite)
async fn get_all_descendants_sqlite(pool: &SqlitePool, id: i64) -> Result<Vec<i64>> {
    let rows = sqlx::query(
        r#"
        WITH RECURSIVE descendants AS (
            -- Base case: the category itself
            SELECT id FROM categories WHERE id = ?
            UNION ALL
            -- Recursive case: children of current level
            SELECT c.id
            FROM categories c
            INNER JOIN descendants d ON c.parent_id = d.id
        )
        SELECT id FROM descendants
        "#,
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .context("Failed to get all descendants")?;

    let mut ids = Vec::new();
    for row in rows {
        ids.push(row.get("id"));
    }

    Ok(ids)
}

async fn update_category_sqlite(pool: &SqlitePool, category: &Category) -> Result<Category> {
    sqlx::query(
        r#"
        UPDATE categories
        SET slug = ?, name = ?, description = ?, parent_id = ?, sort_order = ?
        WHERE id = ?
        "#,
    )
    .bind(&category.slug)
    .bind(&category.name)
    .bind(&category.description)
    .bind(category.parent_id)
    .bind(category.sort_order)
    .bind(category.id)
    .execute(pool)
    .await
    .context("Failed to update category")?;

    // Return the updated category
    get_category_by_id_sqlite(pool, category.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Category not found after update"))
}

async fn delete_category_sqlite(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete category")?;

    Ok(())
}

async fn exists_by_name_sqlite(pool: &SqlitePool, name: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM categories WHERE name = ?")
        .bind(name)
        .fetch_one(pool)
        .await
        .context("Failed to check category name existence")?;

    let count: i64 = row.get("count");
    Ok(count > 0)
}

async fn exists_by_slug_sqlite(pool: &SqlitePool, slug: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM categories WHERE slug = ?")
        .bind(slug)
        .fetch_one(pool)
        .await
        .context("Failed to check category slug existence")?;

    let count: i64 = row.get("count");
    Ok(count > 0)
}

fn row_to_category_sqlite(row: &sqlx::sqlite::SqliteRow) -> Result<Category> {
    Ok(Category {
        id: row.get("id"),
        slug: row.get("slug"),
        name: row.get("name"),
        description: row.get("description"),
        parent_id: row.get("parent_id"),
        sort_order: row.get("sort_order"),
        created_at: row.get("created_at"),
    })
}

// ============================================================================
// MySQL implementations
// ============================================================================

async fn create_category_mysql(pool: &MySqlPool, category: &Category) -> Result<Category> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"
        INSERT INTO categories (slug, name, description, parent_id, sort_order, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&category.slug)
    .bind(&category.name)
    .bind(&category.description)
    .bind(category.parent_id)
    .bind(category.sort_order)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create category")?;

    let id = result.last_insert_id() as i64;

    Ok(Category {
        id,
        slug: category.slug.clone(),
        name: category.name.clone(),
        description: category.description.clone(),
        parent_id: category.parent_id,
        sort_order: category.sort_order,
        created_at: now,
    })
}

async fn get_category_by_id_mysql(pool: &MySqlPool, id: i64) -> Result<Option<Category>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, description, parent_id, sort_order, created_at
        FROM categories
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get category by ID")?;

    match row {
        Some(row) => Ok(Some(row_to_category_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn get_category_by_slug_mysql(pool: &MySqlPool, slug: &str) -> Result<Option<Category>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, description, parent_id, sort_order, created_at
        FROM categories
        WHERE slug = ?
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .context("Failed to get category by slug")?;

    match row {
        Some(row) => Ok(Some(row_to_category_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn get_category_by_name_mysql(pool: &MySqlPool, name: &str) -> Result<Option<Category>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, name, description, parent_id, sort_order, created_at
        FROM categories
        WHERE name = ?
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .context("Failed to get category by name")?;

    match row {
        Some(row) => Ok(Some(row_to_category_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn list_categories_mysql(pool: &MySqlPool) -> Result<Vec<Category>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, name, description, parent_id, sort_order, created_at
        FROM categories
        ORDER BY sort_order, name
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to list categories")?;

    let mut categories = Vec::new();
    for row in rows {
        categories.push(row_to_category_mysql(&row)?);
    }

    Ok(categories)
}

async fn get_children_mysql(pool: &MySqlPool, parent_id: i64) -> Result<Vec<Category>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, name, description, parent_id, sort_order, created_at
        FROM categories
        WHERE parent_id = ?
        ORDER BY sort_order, name
        "#,
    )
    .bind(parent_id)
    .fetch_all(pool)
    .await
    .context("Failed to get children categories")?;

    let mut categories = Vec::new();
    for row in rows {
        categories.push(row_to_category_mysql(&row)?);
    }

    Ok(categories)
}

/// Get all descendants using recursive CTE (MySQL 8.0+)
async fn get_all_descendants_mysql(pool: &MySqlPool, id: i64) -> Result<Vec<i64>> {
    let rows = sqlx::query(
        r#"
        WITH RECURSIVE descendants AS (
            -- Base case: the category itself
            SELECT id FROM categories WHERE id = ?
            UNION ALL
            -- Recursive case: children of current level
            SELECT c.id
            FROM categories c
            INNER JOIN descendants d ON c.parent_id = d.id
        )
        SELECT id FROM descendants
        "#,
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .context("Failed to get all descendants")?;

    let mut ids = Vec::new();
    for row in rows {
        ids.push(row.get("id"));
    }

    Ok(ids)
}

async fn update_category_mysql(pool: &MySqlPool, category: &Category) -> Result<Category> {
    sqlx::query(
        r#"
        UPDATE categories
        SET slug = ?, name = ?, description = ?, parent_id = ?, sort_order = ?
        WHERE id = ?
        "#,
    )
    .bind(&category.slug)
    .bind(&category.name)
    .bind(&category.description)
    .bind(category.parent_id)
    .bind(category.sort_order)
    .bind(category.id)
    .execute(pool)
    .await
    .context("Failed to update category")?;

    // Return the updated category
    get_category_by_id_mysql(pool, category.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Category not found after update"))
}

async fn delete_category_mysql(pool: &MySqlPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete category")?;

    Ok(())
}

async fn exists_by_name_mysql(pool: &MySqlPool, name: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM categories WHERE name = ?")
        .bind(name)
        .fetch_one(pool)
        .await
        .context("Failed to check category name existence")?;

    let count: i64 = row.get("count");
    Ok(count > 0)
}

async fn exists_by_slug_mysql(pool: &MySqlPool, slug: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM categories WHERE slug = ?")
        .bind(slug)
        .fetch_one(pool)
        .await
        .context("Failed to check category slug existence")?;

    let count: i64 = row.get("count");
    Ok(count > 0)
}

fn row_to_category_mysql(row: &sqlx::mysql::MySqlRow) -> Result<Category> {
    Ok(Category {
        id: row.get("id"),
        slug: row.get("slug"),
        name: row.get("name"),
        description: row.get("description"),
        parent_id: row.get("parent_id"),
        sort_order: row.get("sort_order"),
        created_at: row.get("created_at"),
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_test_pool, migrations};

    async fn setup_test_repo() -> (DynDatabasePool, SqlxCategoryRepository) {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");
        let repo = SqlxCategoryRepository::new(pool.clone());
        (pool, repo)
    }

    fn create_test_category(slug: &str, name: &str, parent_id: Option<i64>) -> Category {
        Category::new(
            slug.to_string(),
            name.to_string(),
            Some(format!("Description for {}", name)),
            parent_id,
            0,
        )
    }

    #[tokio::test]
    async fn test_create_category() {
        let (_pool, repo) = setup_test_repo().await;
        let category = create_test_category("test-category", "Test Category", None);

        let created = repo.create(&category).await.expect("Failed to create category");

        assert!(created.id > 0);
        assert_eq!(created.slug, "test-category");
        assert_eq!(created.name, "Test Category");
        assert!(created.parent_id.is_none());
    }

    #[tokio::test]
    async fn test_create_category_with_parent() {
        let (_pool, repo) = setup_test_repo().await;
        
        // Create parent category
        let parent = create_test_category("parent", "Parent Category", None);
        let created_parent = repo.create(&parent).await.expect("Failed to create parent");

        // Create child category
        let child = create_test_category("child", "Child Category", Some(created_parent.id));
        let created_child = repo.create(&child).await.expect("Failed to create child");

        assert_eq!(created_child.parent_id, Some(created_parent.id));
    }

    #[tokio::test]
    async fn test_get_category_by_id() {
        let (_pool, repo) = setup_test_repo().await;
        let category = create_test_category("get-by-id", "Get By ID", None);
        let created = repo.create(&category).await.expect("Failed to create category");

        let found = repo
            .get_by_id(created.id)
            .await
            .expect("Failed to get category")
            .expect("Category not found");

        assert_eq!(found.id, created.id);
        assert_eq!(found.slug, "get-by-id");
    }

    #[tokio::test]
    async fn test_get_category_by_id_not_found() {
        let (_pool, repo) = setup_test_repo().await;

        let found = repo.get_by_id(99999).await.expect("Failed to get category");

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_get_category_by_slug() {
        let (_pool, repo) = setup_test_repo().await;
        let category = create_test_category("unique-slug", "Unique Slug", None);
        repo.create(&category).await.expect("Failed to create category");

        let found = repo
            .get_by_slug("unique-slug")
            .await
            .expect("Failed to get category")
            .expect("Category not found");

        assert_eq!(found.slug, "unique-slug");
    }

    #[tokio::test]
    async fn test_get_category_by_slug_not_found() {
        let (_pool, repo) = setup_test_repo().await;

        let found = repo
            .get_by_slug("nonexistent")
            .await
            .expect("Failed to get category");

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_get_category_by_name() {
        let (_pool, repo) = setup_test_repo().await;
        let category = create_test_category("name-test", "Unique Name", None);
        repo.create(&category).await.expect("Failed to create category");

        let found = repo
            .get_by_name("Unique Name")
            .await
            .expect("Failed to get category")
            .expect("Category not found");

        assert_eq!(found.name, "Unique Name");
    }

    #[tokio::test]
    async fn test_list_categories() {
        let (_pool, repo) = setup_test_repo().await;

        // Create some categories
        repo.create(&create_test_category("cat1", "Category 1", None))
            .await
            .expect("Failed to create category");
        repo.create(&create_test_category("cat2", "Category 2", None))
            .await
            .expect("Failed to create category");

        let categories = repo.list().await.expect("Failed to list categories");

        // Should include the default "uncategorized" category plus our 2
        assert!(categories.len() >= 3);
    }

    #[tokio::test]
    async fn test_list_tree() {
        let (_pool, repo) = setup_test_repo().await;

        // Create a hierarchy: root -> child1, child2 -> grandchild
        let root = repo
            .create(&create_test_category("root", "Root", None))
            .await
            .expect("Failed to create root");
        let child1 = repo
            .create(&create_test_category("child1", "Child 1", Some(root.id)))
            .await
            .expect("Failed to create child1");
        repo.create(&create_test_category("child2", "Child 2", Some(root.id)))
            .await
            .expect("Failed to create child2");
        repo.create(&create_test_category("grandchild", "Grandchild", Some(child1.id)))
            .await
            .expect("Failed to create grandchild");

        let tree = repo.list_tree().await.expect("Failed to list tree");

        // Find our root in the tree
        let root_tree = tree.iter().find(|t| t.category.slug == "root");
        assert!(root_tree.is_some());

        let root_tree = root_tree.unwrap();
        assert_eq!(root_tree.children.len(), 2);

        // Find child1 and verify it has grandchild
        let child1_tree = root_tree.children.iter().find(|t| t.category.slug == "child1");
        assert!(child1_tree.is_some());
        assert_eq!(child1_tree.unwrap().children.len(), 1);
    }

    #[tokio::test]
    async fn test_get_children() {
        let (_pool, repo) = setup_test_repo().await;

        // Create parent and children
        let parent = repo
            .create(&create_test_category("parent", "Parent", None))
            .await
            .expect("Failed to create parent");
        repo.create(&create_test_category("child1", "Child 1", Some(parent.id)))
            .await
            .expect("Failed to create child1");
        repo.create(&create_test_category("child2", "Child 2", Some(parent.id)))
            .await
            .expect("Failed to create child2");

        let children = repo
            .get_children(parent.id)
            .await
            .expect("Failed to get children");

        assert_eq!(children.len(), 2);
    }

    #[tokio::test]
    async fn test_get_all_descendants() {
        let (_pool, repo) = setup_test_repo().await;

        // Create a hierarchy: root -> child -> grandchild
        let root = repo
            .create(&create_test_category("root", "Root", None))
            .await
            .expect("Failed to create root");
        let child = repo
            .create(&create_test_category("child", "Child", Some(root.id)))
            .await
            .expect("Failed to create child");
        let grandchild = repo
            .create(&create_test_category("grandchild", "Grandchild", Some(child.id)))
            .await
            .expect("Failed to create grandchild");

        let descendants = repo
            .get_all_descendants(root.id)
            .await
            .expect("Failed to get descendants");

        // Should include root, child, and grandchild
        assert_eq!(descendants.len(), 3);
        assert!(descendants.contains(&root.id));
        assert!(descendants.contains(&child.id));
        assert!(descendants.contains(&grandchild.id));
    }

    #[tokio::test]
    async fn test_get_all_descendants_leaf() {
        let (_pool, repo) = setup_test_repo().await;

        // Create a leaf category (no children)
        let leaf = repo
            .create(&create_test_category("leaf", "Leaf", None))
            .await
            .expect("Failed to create leaf");

        let descendants = repo
            .get_all_descendants(leaf.id)
            .await
            .expect("Failed to get descendants");

        // Should only include the leaf itself
        assert_eq!(descendants.len(), 1);
        assert!(descendants.contains(&leaf.id));
    }

    #[tokio::test]
    async fn test_update_category() {
        let (_pool, repo) = setup_test_repo().await;
        let category = create_test_category("update-me", "Update Me", None);
        let mut created = repo.create(&category).await.expect("Failed to create category");

        created.name = "Updated Name".to_string();
        created.description = Some("Updated description".to_string());

        let updated = repo.update(&created).await.expect("Failed to update category");

        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.description, Some("Updated description".to_string()));
    }

    #[tokio::test]
    async fn test_delete_category() {
        let (_pool, repo) = setup_test_repo().await;
        let category = create_test_category("delete-me", "Delete Me", None);
        let created = repo.create(&category).await.expect("Failed to create category");

        repo.delete(created.id).await.expect("Failed to delete category");

        let found = repo.get_by_id(created.id).await.expect("Failed to get category");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_exists_by_name() {
        let (_pool, repo) = setup_test_repo().await;
        let category = create_test_category("exists-test", "Exists Test", None);
        repo.create(&category).await.expect("Failed to create category");

        let exists = repo
            .exists_by_name("Exists Test")
            .await
            .expect("Failed to check existence");
        assert!(exists);

        let not_exists = repo
            .exists_by_name("Does Not Exist")
            .await
            .expect("Failed to check existence");
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_exists_by_slug() {
        let (_pool, repo) = setup_test_repo().await;
        let category = create_test_category("slug-exists", "Slug Exists", None);
        repo.create(&category).await.expect("Failed to create category");

        let exists = repo
            .exists_by_slug("slug-exists")
            .await
            .expect("Failed to check existence");
        assert!(exists);

        let not_exists = repo
            .exists_by_slug("slug-not-exists")
            .await
            .expect("Failed to check existence");
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_get_default_category() {
        let (_pool, repo) = setup_test_repo().await;

        let default = repo
            .get_default()
            .await
            .expect("Failed to get default category")
            .expect("Default category not found");

        assert_eq!(default.slug, "uncategorized");
    }

    #[tokio::test]
    async fn test_unique_slug_constraint() {
        let (_pool, repo) = setup_test_repo().await;
        let cat1 = create_test_category("duplicate-slug", "Category 1", None);
        let cat2 = create_test_category("duplicate-slug", "Category 2", None);

        repo.create(&cat1).await.expect("Failed to create first category");
        let result = repo.create(&cat2).await;

        assert!(result.is_err(), "Should fail due to duplicate slug");
    }

    #[tokio::test]
    async fn test_category_sort_order() {
        let (_pool, repo) = setup_test_repo().await;

        // Create categories with different sort orders
        let mut cat1 = create_test_category("sort-c", "Sort C", None);
        cat1.sort_order = 2;
        let mut cat2 = create_test_category("sort-a", "Sort A", None);
        cat2.sort_order = 0;
        let mut cat3 = create_test_category("sort-b", "Sort B", None);
        cat3.sort_order = 1;

        repo.create(&cat1).await.expect("Failed to create cat1");
        repo.create(&cat2).await.expect("Failed to create cat2");
        repo.create(&cat3).await.expect("Failed to create cat3");

        let categories = repo.list().await.expect("Failed to list categories");

        // Find our categories and verify order
        let our_cats: Vec<_> = categories
            .iter()
            .filter(|c| c.slug.starts_with("sort-"))
            .collect();

        assert_eq!(our_cats.len(), 3);
        // Should be ordered by sort_order
        assert_eq!(our_cats[0].slug, "sort-a");
        assert_eq!(our_cats[1].slug, "sort-b");
        assert_eq!(our_cats[2].slug, "sort-c");
    }

    #[tokio::test]
    async fn test_build_category_tree_empty() {
        let tree = build_category_tree(Vec::new());
        assert!(tree.is_empty());
    }

    #[tokio::test]
    async fn test_build_category_tree_single() {
        let mut cat = Category::new("single".to_string(), "Single".to_string(), None, None, 0);
        cat.id = 1;

        let tree = build_category_tree(vec![cat]);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].category.slug, "single");
        assert!(tree[0].children.is_empty());
    }

    #[tokio::test]
    async fn test_build_category_tree_hierarchy() {
        let mut root = Category::new("root".to_string(), "Root".to_string(), None, None, 0);
        root.id = 1;
        let mut child1 = Category::new("child1".to_string(), "Child 1".to_string(), None, Some(1), 0);
        child1.id = 2;
        let mut child2 = Category::new("child2".to_string(), "Child 2".to_string(), None, Some(1), 1);
        child2.id = 3;
        let mut grandchild = Category::new("grandchild".to_string(), "Grandchild".to_string(), None, Some(2), 0);
        grandchild.id = 4;

        let tree = build_category_tree(vec![root, child1, child2, grandchild]);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].category.slug, "root");
        assert_eq!(tree[0].children.len(), 2);
        
        // child1 should have grandchild
        let child1_tree = tree[0].children.iter().find(|c| c.category.slug == "child1").unwrap();
        assert_eq!(child1_tree.children.len(), 1);
        assert_eq!(child1_tree.children[0].category.slug, "grandchild");
    }
}
