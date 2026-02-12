//! Shared API response types
//!
//! This module contains common response structures used across multiple API endpoints
//! to ensure consistency and reduce code duplication.

use serde::{Deserialize, Serialize};

use crate::services::markdown::TocEntry;

// ============================================================================
// Article Response Types
// ============================================================================

/// Full article response with all fields
/// Used in article detail endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct ArticleResponse {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub content_html: String,
    pub author_id: i64,
    pub category_id: i64,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub view_count: i64,
    pub like_count: i64,
    pub comment_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    pub is_pinned: bool,
    pub pin_order: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<CategoryInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<TagInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toc: Option<Vec<TocEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Simplified article response for list views
/// Used in category/tag article listings
#[derive(Debug, Serialize)]
pub struct ArticleSummary {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
}

/// Category info embedded in article response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryInfo {
    pub id: i64,
    pub slug: String,
    pub name: String,
}

/// Tag info embedded in article response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagInfo {
    pub id: i64,
    pub slug: String,
    pub name: String,
}

// ============================================================================
// Pagination Response Types
// ============================================================================

/// Paginated article list response (full articles)
#[derive(Debug, Serialize)]
pub struct PaginatedArticlesResponse {
    pub articles: Vec<ArticleResponse>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// Paginated article summary list response
#[derive(Debug, Serialize)]
pub struct PaginatedArticleSummaryResponse {
    pub articles: Vec<ArticleSummary>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

// ============================================================================
// Conversions
// ============================================================================

impl From<crate::models::Article> for ArticleResponse {
    fn from(article: crate::models::Article) -> Self {
        Self {
            id: article.id,
            slug: article.slug,
            title: article.title,
            content: article.content,
            content_html: article.content_html,
            author_id: article.author_id,
            category_id: article.category_id,
            status: article.status.to_string(),
            published_at: article.published_at.map(|dt| dt.to_rfc3339()),
            created_at: article.created_at.to_rfc3339(),
            updated_at: article.updated_at.to_rfc3339(),
            view_count: article.view_count,
            like_count: article.like_count,
            comment_count: article.comment_count,
            thumbnail: article.thumbnail,
            is_pinned: article.is_pinned,
            pin_order: article.pin_order,
            category: None,
            tags: None,
            toc: None,
            meta: if article.meta.as_object().map_or(true, |m| m.is_empty()) {
                None
            } else {
                Some(article.meta)
            },
        }
    }
}

impl From<crate::models::Article> for ArticleSummary {
    fn from(article: crate::models::Article) -> Self {
        Self {
            id: article.id,
            slug: article.slug,
            title: article.title,
            status: article.status.to_string(),
            published_at: article.published_at.map(|dt| dt.to_rfc3339()),
            created_at: article.created_at.to_rfc3339(),
        }
    }
}

impl ArticleResponse {
    /// Add category info to response
    pub fn with_category(mut self, category: Option<crate::models::Category>) -> Self {
        self.category = category.map(|c| CategoryInfo {
            id: c.id,
            slug: c.slug,
            name: c.name,
        });
        self
    }

    /// Add tags info to response
    pub fn with_tags(mut self, tags: Vec<crate::models::Tag>) -> Self {
        self.tags = Some(
            tags.into_iter()
                .map(|t| TagInfo {
                    id: t.id,
                    slug: t.slug,
                    name: t.name,
                })
                .collect(),
        );
        self
    }

    /// Add table of contents to response
    pub fn with_toc(mut self, toc: Vec<TocEntry>) -> Self {
        if !toc.is_empty() {
            self.toc = Some(toc);
        }
        self
    }
}
