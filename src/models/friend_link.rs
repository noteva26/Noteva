//! Friend link model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FriendLinkStatus {
    Pending,
    Approved,
    Rejected,
    Hidden,
}

impl Default for FriendLinkStatus {
    fn default() -> Self {
        Self::Approved
    }
}

impl std::fmt::Display for FriendLinkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Rejected => write!(f, "rejected"),
            Self::Hidden => write!(f, "hidden"),
        }
    }
}

impl std::str::FromStr for FriendLinkStatus {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "hidden" => Ok(Self::Hidden),
            _ => Err(anyhow::anyhow!("Invalid friend link status: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendLink {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub logo: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub sort_order: i32,
    pub status: FriendLinkStatus,
    pub is_recommended: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl FriendLink {
    pub fn new(name: String, url: String) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            name,
            url,
            logo: None,
            description: None,
            category: None,
            sort_order: 0,
            status: FriendLinkStatus::Approved,
            is_recommended: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateFriendLinkInput {
    pub name: String,
    pub url: String,
    pub logo: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub sort_order: Option<i32>,
    pub status: Option<String>,
    #[serde(default)]
    pub is_recommended: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateFriendLinkInput {
    pub name: Option<String>,
    pub url: Option<String>,
    pub logo: Option<Option<String>>,
    pub description: Option<Option<String>>,
    pub category: Option<Option<String>>,
    pub sort_order: Option<i32>,
    pub status: Option<String>,
    pub is_recommended: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateFriendLinkOrderInput {
    pub items: Vec<FriendLinkOrderItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FriendLinkOrderItem {
    pub id: i64,
    pub sort_order: i32,
}
