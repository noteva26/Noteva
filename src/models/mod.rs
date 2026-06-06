//! Data models
//!
//! This module contains all data structures used throughout the Noteva blog system.
//! Models represent:
//! - Database entities (Article, Category, Tag, User, Session, Comment, Page, NavItem)
//! - API request/response types
//! - Internal data transfer objects

mod about;
mod article;
mod category;
mod comment;
mod friend_link;
mod nav_item;
mod page;
mod session;
mod tag;
mod user;

pub use about::{AboutProfile, AboutSocialLink, AboutTimelineItem};
pub use article::{
    Article, ArticleSortBy, ArticleStatus, CreateArticleInput, ListParams, PagedResult,
    UpdateArticleInput,
};
pub use category::{Category, CategoryTree, CreateCategoryInput, UpdateCategoryInput};
pub use comment::{
    Comment, CommentStatus, CommentWithMeta, CreateCommentInput, Like, LikeTargetType,
};
pub use friend_link::{
    CreateFriendLinkInput, FriendLink, FriendLinkOrderItem, FriendLinkStatus,
    UpdateFriendLinkInput, UpdateFriendLinkOrderInput,
};
pub use nav_item::{
    CreateNavItemInput, NavItem, NavItemTree, NavItemType, NavOrderItem, UpdateNavItemInput,
    UpdateNavOrderInput,
};
pub use page::{CreatePageInput, Page, PageStatus, UpdatePageInput};
pub use session::Session;
pub use tag::{Tag, TagWithCount};
pub use user::{CreateUserInput, UpdateUserInput, User, UserRole, UserStatus};
