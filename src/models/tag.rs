//! Tag model
//!
//! This module defines the Tag entity and related types for the Noteva blog system.
//!
//! Satisfies requirements:
//! - 3.1: WHEN 用户为文章添加标签 THEN Tag_Service SHALL 创建或复用已有标签并建立关联
//! - 3.4: THE Tag_Service SHALL 提供标签云功能，按使用频率排序

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Tag entity representing a tag in the blog system.
///
/// Tags are used to categorize articles across different categories,
/// enabling cross-category content discovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    /// Unique identifier
    pub id: i64,
    /// URL-friendly slug
    pub slug: String,
    /// Tag name
    pub name: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Tag {
    /// Create a new Tag with the given parameters.
    ///
    /// The ID will be set to 0 and should be assigned by the database.
    pub fn new(slug: String, name: String) -> Self {
        Self {
            id: 0, // Will be set by the database
            slug,
            name,
            created_at: Utc::now(),
        }
    }
}

/// Tag with article count for tag cloud functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagWithCount {
    /// The tag itself
    #[serde(flatten)]
    pub tag: Tag,
    /// Number of articles with this tag
    pub article_count: i64,
}

impl TagWithCount {
    /// Create a new TagWithCount
    pub fn new(tag: Tag, article_count: i64) -> Self {
        Self { tag, article_count }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_new() {
        let tag = Tag::new("rust-programming".to_string(), "Rust Programming".to_string());

        assert_eq!(tag.id, 0);
        assert_eq!(tag.slug, "rust-programming");
        assert_eq!(tag.name, "Rust Programming");
    }

    #[test]
    fn test_tag_with_count_new() {
        let tag = Tag::new("rust".to_string(), "Rust".to_string());
        let tag_with_count = TagWithCount::new(tag.clone(), 42);

        assert_eq!(tag_with_count.tag, tag);
        assert_eq!(tag_with_count.article_count, 42);
    }
}
