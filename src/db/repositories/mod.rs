//! Database repositories
//!
//! Repository pattern implementations for database access.
//! Each repository handles CRUD operations for a specific entity.

pub mod article;
pub mod category;
pub mod comment;
pub mod nav_item;
pub mod page;
pub mod session;
pub mod settings;
pub mod tag;
pub mod user;

pub use article::{ArticleRepository, SqlxArticleRepository};
pub use category::{CategoryRepository, SqlxCategoryRepository};
pub use comment::{CommentRepository, CommentRepositoryImpl};
pub use nav_item::{NavItemRepository, SqlxNavItemRepository};
pub use page::{PageRepository, SqlxPageRepository};
pub use session::{SessionRepository, SqlxSessionRepository};
pub use settings::{Setting, SettingsRepository, SqlxSettingsRepository};
pub use tag::{TagRepository, SqlxTagRepository};
pub use user::{SqlxUserRepository, UserRepository};
