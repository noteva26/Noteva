//! Page model for custom pages (like WordPress pages)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Page status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageStatus {
    Draft,
    Published,
}

impl Default for PageStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl std::fmt::Display for PageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Published => write!(f, "published"),
        }
    }
}

impl std::str::FromStr for PageStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "published" => Ok(Self::Published),
            _ => Err(anyhow::anyhow!("Invalid page status: {}", s)),
        }
    }
}

/// Custom page model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub content_html: String,
    pub status: PageStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Page {
    pub fn new(slug: String, title: String, content: String, content_html: String) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            slug,
            title,
            content,
            content_html,
            status: PageStatus::Draft,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Input for creating a page
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePageInput {
    pub slug: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub status: Option<String>,
}

/// Input for updating a page
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePageInput {
    pub slug: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub status: Option<String>,
}
