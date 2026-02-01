//! Comment model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Comment status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommentStatus {
    Pending,
    Approved,
    Spam,
}

impl Default for CommentStatus {
    fn default() -> Self {
        Self::Approved
    }
}

impl std::fmt::Display for CommentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Spam => write!(f, "spam"),
        }
    }
}

impl std::str::FromStr for CommentStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "spam" => Ok(Self::Spam),
            _ => Err(format!("Invalid comment status: {}", s)),
        }
    }
}

/// Comment entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: i64,
    pub article_id: i64,
    pub user_id: Option<i64>,
    pub parent_id: Option<i64>,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub content: String,
    pub status: CommentStatus,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Comment with additional info for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentWithMeta {
    pub id: i64,
    pub article_id: i64,
    pub user_id: Option<i64>,
    pub parent_id: Option<i64>,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub content: String,
    pub status: CommentStatus,
    pub created_at: DateTime<Utc>,
    pub avatar_url: String,
    pub like_count: i64,
    pub is_liked: bool,
    #[serde(default)]
    pub is_author: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub replies: Vec<CommentWithMeta>,
}

impl CommentWithMeta {
    /// Generate Gravatar URL from email
    pub fn gravatar_url(email: &Option<String>) -> String {
        match email {
            Some(e) if !e.is_empty() => {
                let hash = format!("{:x}", md5::compute(e.trim().to_lowercase()));
                format!("https://www.gravatar.com/avatar/{}?d=mp&s=80", hash)
            }
            _ => "https://www.gravatar.com/avatar/?d=mp&s=80".to_string(),
        }
    }
}

/// Input for creating a comment
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCommentInput {
    pub article_id: i64,
    pub parent_id: Option<i64>,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub content: String,
}

/// Like target type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LikeTargetType {
    Article,
    Comment,
}

impl std::fmt::Display for LikeTargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Article => write!(f, "article"),
            Self::Comment => write!(f, "comment"),
        }
    }
}

/// Like entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Like {
    pub id: i64,
    pub target_type: LikeTargetType,
    pub target_id: i64,
    pub user_id: Option<i64>,
    pub fingerprint: Option<String>,
    pub created_at: DateTime<Utc>,
}
