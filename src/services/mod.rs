//! Services layer - Business logic
//!
//! This module contains all business logic services for the Noteva blog system.
//! Services are responsible for:
//! - Implementing business rules
//! - Coordinating between repositories and cache
//! - Handling validation and error cases

pub mod article;
pub mod category;
pub mod comment;
pub mod email;
pub mod emoji;
pub mod markdown;
pub mod nav_item;
pub mod page;
pub mod password;
pub mod rate_limiter;
pub mod settings;
pub mod tag;
pub mod user;

pub use article::{ArticleService, ArticleServiceError, generate_slug as generate_article_slug};
pub use category::{
    CategoryService, CategoryServiceError, CreateCategoryInput, UpdateCategoryInput,
    generate_slug,
};
pub use comment::{CommentService, generate_fingerprint};
pub use email::{EmailService, generate_verification_code};
pub use emoji::{process_all_emoji, process_shortcodes, process_unicode_emoji};
pub use markdown::{MarkdownRenderer, TocEntry};
pub use nav_item::NavItemService;
pub use page::PageService;
pub use password::{hash_password, verify_password};
pub use rate_limiter::LoginRateLimiter;
pub use settings::{SettingsService, SettingsServiceError, SiteSettings};
pub use tag::{generate_tag_slug, TagService, TagServiceError};
pub use user::{LoginInput, RegisterInput, UserService, UserServiceError};
