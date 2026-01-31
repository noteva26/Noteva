//! Category model
//!
//! This module defines the Category entity and related types for the Noteva blog system.
//!
//! Satisfies requirements:
//! - 2.1: WHEN 用户创建分类 THEN Category_Service SHALL 创建分类记录并支持设置父分类
//! - 2.3: WHEN 用户请求某分类下的文章 THEN Category_Service SHALL 返回该分类及其子分类下的所有文章

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Category entity representing a hierarchical category in the blog system.
///
/// Categories support parent-child relationships for organizing articles
/// into a tree structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Category {
    /// Unique identifier
    pub id: i64,
    /// URL-friendly slug
    pub slug: String,
    /// Category name
    pub name: String,
    /// Category description
    pub description: Option<String>,
    /// Parent category ID (for hierarchical structure)
    pub parent_id: Option<i64>,
    /// Sort order within parent
    pub sort_order: i32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Category {
    /// Create a new Category with the given parameters.
    ///
    /// The ID will be set to 0 and should be assigned by the database.
    pub fn new(
        slug: String,
        name: String,
        description: Option<String>,
        parent_id: Option<i64>,
        sort_order: i32,
    ) -> Self {
        Self {
            id: 0, // Will be set by the database
            slug,
            name,
            description,
            parent_id,
            sort_order,
            created_at: Utc::now(),
        }
    }

    /// Check if this is a root category (no parent)
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Check if this category is the default "uncategorized" category
    pub fn is_default(&self) -> bool {
        self.slug == "uncategorized"
    }
}

/// Category with its children for tree representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryTree {
    /// The category itself
    #[serde(flatten)]
    pub category: Category,
    /// Child categories
    pub children: Vec<CategoryTree>,
}

impl CategoryTree {
    /// Create a new CategoryTree from a category with no children
    pub fn new(category: Category) -> Self {
        Self {
            category,
            children: Vec::new(),
        }
    }

    /// Create a CategoryTree with children
    pub fn with_children(category: Category, children: Vec<CategoryTree>) -> Self {
        Self { category, children }
    }

    /// Get the total count of this category and all descendants
    pub fn total_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.total_count()).sum::<usize>()
    }

    /// Flatten the tree into a list of categories (depth-first)
    pub fn flatten(&self) -> Vec<&Category> {
        let mut result = vec![&self.category];
        for child in &self.children {
            result.extend(child.flatten());
        }
        result
    }

    /// Get all descendant IDs (not including self)
    pub fn descendant_ids(&self) -> Vec<i64> {
        let mut ids = Vec::new();
        for child in &self.children {
            ids.push(child.category.id);
            ids.extend(child.descendant_ids());
        }
        ids
    }
}

/// Input for creating a new category
#[derive(Debug, Clone)]
pub struct CreateCategoryInput {
    /// URL-friendly slug
    pub slug: String,
    /// Category name
    pub name: String,
    /// Category description
    pub description: Option<String>,
    /// Parent category ID
    pub parent_id: Option<i64>,
    /// Sort order within parent
    pub sort_order: Option<i32>,
}

/// Input for updating a category
#[derive(Debug, Clone, Default)]
pub struct UpdateCategoryInput {
    /// New slug (optional)
    pub slug: Option<String>,
    /// New name (optional)
    pub name: Option<String>,
    /// New description (optional)
    pub description: Option<Option<String>>,
    /// New parent ID (optional)
    pub parent_id: Option<Option<i64>>,
    /// New sort order (optional)
    pub sort_order: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_new() {
        let category = Category::new(
            "test-category".to_string(),
            "Test Category".to_string(),
            Some("A test category".to_string()),
            None,
            0,
        );

        assert_eq!(category.id, 0);
        assert_eq!(category.slug, "test-category");
        assert_eq!(category.name, "Test Category");
        assert_eq!(category.description, Some("A test category".to_string()));
        assert!(category.parent_id.is_none());
        assert_eq!(category.sort_order, 0);
    }

    #[test]
    fn test_category_is_root() {
        let root = Category::new("root".to_string(), "Root".to_string(), None, None, 0);
        let child = Category::new("child".to_string(), "Child".to_string(), None, Some(1), 0);

        assert!(root.is_root());
        assert!(!child.is_root());
    }

    #[test]
    fn test_category_is_default() {
        let default = Category::new(
            "uncategorized".to_string(),
            "Uncategorized".to_string(),
            None,
            None,
            0,
        );
        let other = Category::new("other".to_string(), "Other".to_string(), None, None, 0);

        assert!(default.is_default());
        assert!(!other.is_default());
    }

    #[test]
    fn test_category_tree_new() {
        let category = Category::new("test".to_string(), "Test".to_string(), None, None, 0);
        let tree = CategoryTree::new(category.clone());

        assert_eq!(tree.category, category);
        assert!(tree.children.is_empty());
    }

    #[test]
    fn test_category_tree_total_count() {
        let root = Category::new("root".to_string(), "Root".to_string(), None, None, 0);
        let child1 = Category::new("child1".to_string(), "Child 1".to_string(), None, Some(1), 0);
        let child2 = Category::new("child2".to_string(), "Child 2".to_string(), None, Some(1), 1);
        let grandchild = Category::new(
            "grandchild".to_string(),
            "Grandchild".to_string(),
            None,
            Some(2),
            0,
        );

        let tree = CategoryTree::with_children(
            root,
            vec![
                CategoryTree::with_children(child1, vec![CategoryTree::new(grandchild)]),
                CategoryTree::new(child2),
            ],
        );

        assert_eq!(tree.total_count(), 4);
    }

    #[test]
    fn test_category_tree_flatten() {
        let mut root = Category::new("root".to_string(), "Root".to_string(), None, None, 0);
        root.id = 1;
        let mut child1 = Category::new("child1".to_string(), "Child 1".to_string(), None, Some(1), 0);
        child1.id = 2;
        let mut child2 = Category::new("child2".to_string(), "Child 2".to_string(), None, Some(1), 1);
        child2.id = 3;

        let tree = CategoryTree::with_children(
            root,
            vec![CategoryTree::new(child1), CategoryTree::new(child2)],
        );

        let flattened = tree.flatten();
        assert_eq!(flattened.len(), 3);
        assert_eq!(flattened[0].id, 1);
        assert_eq!(flattened[1].id, 2);
        assert_eq!(flattened[2].id, 3);
    }

    #[test]
    fn test_category_tree_descendant_ids() {
        let mut root = Category::new("root".to_string(), "Root".to_string(), None, None, 0);
        root.id = 1;
        let mut child1 = Category::new("child1".to_string(), "Child 1".to_string(), None, Some(1), 0);
        child1.id = 2;
        let mut grandchild = Category::new(
            "grandchild".to_string(),
            "Grandchild".to_string(),
            None,
            Some(2),
            0,
        );
        grandchild.id = 3;

        let tree = CategoryTree::with_children(
            root,
            vec![CategoryTree::with_children(
                child1,
                vec![CategoryTree::new(grandchild)],
            )],
        );

        let descendant_ids = tree.descendant_ids();
        assert_eq!(descendant_ids, vec![2, 3]);
    }
}
