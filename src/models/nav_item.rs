//! Navigation item model for custom navigation

use serde::{Deserialize, Serialize};

/// Navigation item type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NavItemType {
    /// Built-in pages (home, archives, categories, tags)
    Builtin,
    /// Custom page (links to a Page)
    Page,
    /// External URL
    External,
}

impl Default for NavItemType {
    fn default() -> Self {
        Self::Builtin
    }
}

impl std::fmt::Display for NavItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Builtin => write!(f, "builtin"),
            Self::Page => write!(f, "page"),
            Self::External => write!(f, "external"),
        }
    }
}

impl std::str::FromStr for NavItemType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "builtin" => Ok(Self::Builtin),
            "page" => Ok(Self::Page),
            "external" => Ok(Self::External),
            _ => Err(anyhow::anyhow!("Invalid nav item type: {}", s)),
        }
    }
}

/// Navigation item model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavItem {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub title: String,
    pub nav_type: NavItemType,
    /// For builtin: "home", "archives", "categories", "tags"
    /// For page: page slug
    /// For external: full URL
    pub target: String,
    pub open_new_tab: bool,
    pub sort_order: i32,
    pub visible: bool,
}

impl NavItem {
    pub fn new(title: String, nav_type: NavItemType, target: String) -> Self {
        Self {
            id: 0,
            parent_id: None,
            title,
            nav_type,
            target,
            open_new_tab: false,
            sort_order: 0,
            visible: true,
        }
    }
}

/// Navigation item with children (tree structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavItemTree {
    #[serde(flatten)]
    pub item: NavItem,
    pub children: Vec<NavItemTree>,
}

impl NavItemTree {
    pub fn new(item: NavItem) -> Self {
        Self {
            item,
            children: Vec::new(),
        }
    }

    pub fn with_children(item: NavItem, children: Vec<NavItemTree>) -> Self {
        Self { item, children }
    }
}

/// Input for creating a nav item
#[derive(Debug, Clone, Deserialize)]
pub struct CreateNavItemInput {
    pub parent_id: Option<i64>,
    pub title: String,
    pub nav_type: String,
    pub target: String,
    #[serde(default)]
    pub open_new_tab: bool,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default = "default_visible")]
    pub visible: bool,
}

fn default_visible() -> bool {
    true
}

/// Input for updating a nav item
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateNavItemInput {
    pub parent_id: Option<Option<i64>>,
    pub title: Option<String>,
    pub nav_type: Option<String>,
    pub target: Option<String>,
    pub open_new_tab: Option<bool>,
    pub sort_order: Option<i32>,
    pub visible: Option<bool>,
}

/// Input for batch updating nav items order
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateNavOrderInput {
    pub items: Vec<NavOrderItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NavOrderItem {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub sort_order: i32,
}
