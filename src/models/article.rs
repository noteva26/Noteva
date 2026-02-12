//! Article model
//!
//! This module provides:
//! - `Article` entity representing a blog article
//! - `ArticleStatus` enum for publication states
//! - Input types for creating and updating articles
//! - Pagination types for list queries
//!
//! Satisfies requirements:
//! - 1.1: WHEN 用户提交新文章 THEN Article_Manager SHALL 创建文章记录并生成唯一标识符
//! - 1.2: WHEN 用户请求文章列表 THEN Article_Manager SHALL 返回分页的文章列表，支持按时间排序

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Article entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    /// Unique identifier
    pub id: i64,
    /// URL-friendly slug
    pub slug: String,
    /// Article title
    pub title: String,
    /// Markdown content
    pub content: String,
    /// Rendered HTML content
    pub content_html: String,
    /// Author user ID
    pub author_id: i64,
    /// Category ID
    pub category_id: i64,
    /// Publication status
    pub status: ArticleStatus,
    /// Publication timestamp
    pub published_at: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// View count
    #[serde(default)]
    pub view_count: i64,
    /// Like count
    #[serde(default)]
    pub like_count: i64,
    /// Comment count
    #[serde(default)]
    pub comment_count: i64,
    /// Thumbnail image URL
    #[serde(default)]
    pub thumbnail: Option<String>,
    /// Whether the article is pinned
    #[serde(default)]
    pub is_pinned: bool,
    /// Pin order (lower = higher priority)
    #[serde(default)]
    pub pin_order: i32,
    /// Plugin-managed metadata (JSON object, keyed by plugin_id)
    #[serde(default = "default_meta")]
    pub meta: serde_json::Value,
}

fn default_meta() -> serde_json::Value {
    serde_json::json!({})
}

impl Article {
    /// Create a new article with the given parameters
    pub fn new(
        slug: String,
        title: String,
        content: String,
        content_html: String,
        author_id: i64,
        category_id: i64,
        status: ArticleStatus,
    ) -> Self {
        let now = Utc::now();
        let published_at = if status == ArticleStatus::Published {
            Some(now)
        } else {
            None
        };

        Self {
            id: 0, // Will be set by database
            slug,
            title,
            content,
            content_html,
            author_id,
            category_id,
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
            meta: serde_json::json!({}),
        }
    }
}

/// Article publication status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArticleStatus {
    /// Draft - not visible to public
    Draft,
    /// Published - visible to public
    Published,
    /// Archived - hidden but not deleted
    Archived,
}

impl Default for ArticleStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl ArticleStatus {
    /// Convert status to database string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ArticleStatus::Draft => "draft",
            ArticleStatus::Published => "published",
            ArticleStatus::Archived => "archived",
        }
    }

    /// Parse status from database string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "draft" => Some(ArticleStatus::Draft),
            "published" => Some(ArticleStatus::Published),
            "archived" => Some(ArticleStatus::Archived),
            _ => None,
        }
    }
}

impl std::fmt::Display for ArticleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Input for creating a new article
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateArticleInput {
    /// URL-friendly slug
    pub slug: String,
    /// Article title
    pub title: String,
    /// Markdown content
    pub content: String,
    /// Rendered HTML content (optional, can be generated)
    pub content_html: Option<String>,
    /// Author user ID
    pub author_id: i64,
    /// Category ID
    pub category_id: i64,
    /// Publication status (defaults to Draft)
    pub status: Option<ArticleStatus>,
}

impl CreateArticleInput {
    /// Create a new CreateArticleInput
    pub fn new(
        slug: String,
        title: String,
        content: String,
        author_id: i64,
        category_id: i64,
    ) -> Self {
        Self {
            slug,
            title,
            content,
            content_html: None,
            author_id,
            category_id,
            status: None,
        }
    }

    /// Set the content HTML
    pub fn with_content_html(mut self, content_html: String) -> Self {
        self.content_html = Some(content_html);
        self
    }

    /// Set the status
    pub fn with_status(mut self, status: ArticleStatus) -> Self {
        self.status = Some(status);
        self
    }
}

/// Input for updating an existing article
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateArticleInput {
    /// New slug (optional)
    pub slug: Option<String>,
    /// New title (optional)
    pub title: Option<String>,
    /// New markdown content (optional)
    pub content: Option<String>,
    /// New rendered HTML content (optional)
    pub content_html: Option<String>,
    /// New category ID (optional)
    pub category_id: Option<i64>,
    /// New status (optional)
    pub status: Option<ArticleStatus>,
    /// New thumbnail URL (optional)
    pub thumbnail: Option<String>,
    /// Whether the article is pinned (optional)
    pub is_pinned: Option<bool>,
    /// Pin order (optional)
    pub pin_order: Option<i32>,
}

impl UpdateArticleInput {
    /// Create a new empty UpdateArticleInput
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the slug
    pub fn with_slug(mut self, slug: String) -> Self {
        self.slug = Some(slug);
        self
    }

    /// Set the title
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the content
    pub fn with_content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    /// Set the content HTML
    pub fn with_content_html(mut self, content_html: String) -> Self {
        self.content_html = Some(content_html);
        self
    }

    /// Set the category ID
    pub fn with_category_id(mut self, category_id: i64) -> Self {
        self.category_id = Some(category_id);
        self
    }

    /// Set the status
    pub fn with_status(mut self, status: ArticleStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Check if any field is set
    pub fn has_changes(&self) -> bool {
        self.slug.is_some()
            || self.title.is_some()
            || self.content.is_some()
            || self.content_html.is_some()
            || self.category_id.is_some()
            || self.status.is_some()
            || self.thumbnail.is_some()
            || self.is_pinned.is_some()
            || self.pin_order.is_some()
    }
}

/// Pagination parameters for list queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListParams {
    /// Page number (1-indexed)
    pub page: u32,
    /// Number of items per page
    pub per_page: u32,
}

impl Default for ListParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 10,
        }
    }
}

impl ListParams {
    /// Create new pagination parameters
    pub fn new(page: u32, per_page: u32) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, 100),
        }
    }

    /// Calculate the offset for database queries
    pub fn offset(&self) -> i64 {
        ((self.page.saturating_sub(1)) * self.per_page) as i64
    }

    /// Get the limit for database queries
    pub fn limit(&self) -> i64 {
        self.per_page as i64
    }
}

/// Paginated result container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedResult<T> {
    /// Items in the current page
    pub items: Vec<T>,
    /// Total number of items across all pages
    pub total: i64,
    /// Current page number (1-indexed)
    pub page: u32,
    /// Number of items per page
    pub per_page: u32,
}

impl<T> PagedResult<T> {
    /// Create a new paginated result
    pub fn new(items: Vec<T>, total: i64, params: &ListParams) -> Self {
        Self {
            items,
            total,
            page: params.page,
            per_page: params.per_page,
        }
    }

    /// Calculate the total number of pages
    pub fn total_pages(&self) -> u32 {
        if self.per_page == 0 {
            return 0;
        }
        ((self.total as u32) + self.per_page - 1) / self.per_page
    }

    /// Check if there is a next page
    pub fn has_next(&self) -> bool {
        self.page < self.total_pages()
    }

    /// Check if there is a previous page
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }

    /// Check if the result is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of items in the current page
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<T> Default for PagedResult<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            page: 1,
            per_page: 10,
        }
    }
}
