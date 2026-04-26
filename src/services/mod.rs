//! Services layer - Business logic
//!
//! This module contains all business logic services for the Noteva blog system.
//! Services are responsible for:
//! - Implementing business rules
//! - Coordinating between repositories and cache
//! - Handling validation and error cases

pub mod article;
pub mod backup;
pub mod category;
pub mod comment;
pub mod email;
pub mod emoji;
pub mod locale;
pub mod markdown;
pub mod nav_item;
pub mod page;
pub mod password;
pub mod rate_limiter;
pub mod settings;
pub mod tag;
pub mod user;

pub use article::{generate_slug as generate_article_slug, ArticleService, ArticleServiceError};
pub use category::{
    generate_slug, CategoryService, CategoryServiceError, CreateCategoryInput, UpdateCategoryInput,
};
pub use comment::{generate_fingerprint, CommentService};
pub use email::{generate_verification_code, EmailService};
pub use emoji::{process_all_emoji, process_shortcodes, process_unicode_emoji};
pub use markdown::{MarkdownRenderer, TocEntry};
pub use nav_item::NavItemService;
pub use page::PageService;
pub use password::{hash_password, verify_password};
pub use rate_limiter::LoginRateLimiter;
pub use settings::{SettingsService, SettingsServiceError, SiteSettings};
pub use tag::{generate_tag_slug, TagService, TagServiceError};
pub use user::{LoginInput, RegisterInput, UserService, UserServiceError};
