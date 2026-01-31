//! User model
//!
//! This module defines the User entity and related types for the Noteva blog system.
//!
//! Satisfies requirements:
//! - 4.2: User registration and account management
//! - 4.6: Secure password storage (password_hash field)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// User entity representing a registered user in the system.
///
/// Users can have different roles (Admin, Editor, Author) which determine
/// their permissions within the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier
    pub id: i64,
    /// Username (unique)
    pub username: String,
    /// Email address (unique)
    pub email: String,
    /// Password hash (argon2)
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// User role
    pub role: UserRole,
    /// User status (active/banned)
    pub status: UserStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Create a new User with the given parameters.
    ///
    /// Note: The password should already be hashed before calling this function.
    /// Use `services::password::hash_password()` to hash the password.
    pub fn new(
        username: String,
        email: String,
        password_hash: String,
        role: UserRole,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: 0, // Will be set by the database
            username,
            email,
            password_hash,
            role,
            status: UserStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if the user is an administrator
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }

    /// Check if the user is an editor (or higher)
    pub fn is_editor(&self) -> bool {
        matches!(self.role, UserRole::Admin | UserRole::Editor)
    }

    /// Check if the user can edit the given content
    ///
    /// Admins and Editors can edit any content.
    /// Authors can only edit their own content.
    pub fn can_edit(&self, author_id: i64) -> bool {
        self.is_editor() || self.id == author_id
    }

    /// Check if the user is banned
    pub fn is_banned(&self) -> bool {
        self.status == UserStatus::Banned
    }

    /// Check if the user is active
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }
}

/// User role for authorization.
///
/// Roles determine what actions a user can perform:
/// - Admin: Full access to all features
/// - Editor: Can edit all content
/// - Author: Can only edit own content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Administrator - full access
    Admin,
    /// Editor - can edit all content
    Editor,
    /// Author - can only edit own content
    Author,
}

impl Default for UserRole {
    fn default() -> Self {
        Self::Author
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::Editor => write!(f, "editor"),
            UserRole::Author => write!(f, "author"),
        }
    }
}

impl FromStr for UserRole {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(UserRole::Admin),
            "editor" => Ok(UserRole::Editor),
            "author" => Ok(UserRole::Author),
            _ => Err(anyhow::anyhow!("Invalid user role: {}", s)),
        }
    }
}

/// User status for account state.
///
/// Status determines if a user can access the system:
/// - Active: Normal access
/// - Banned: Cannot login
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    /// Active - normal access
    Active,
    /// Banned - cannot login
    Banned,
}

impl Default for UserStatus {
    fn default() -> Self {
        Self::Active
    }
}

impl fmt::Display for UserStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserStatus::Active => write!(f, "active"),
            UserStatus::Banned => write!(f, "banned"),
        }
    }
}

impl FromStr for UserStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(UserStatus::Active),
            "banned" => Ok(UserStatus::Banned),
            _ => Err(anyhow::anyhow!("Invalid user status: {}", s)),
        }
    }
}

/// Input for creating a new user (before password hashing)
#[derive(Debug, Clone)]
pub struct CreateUserInput {
    /// Username
    pub username: String,
    /// Email address
    pub email: String,
    /// Plaintext password (will be hashed)
    pub password: String,
    /// User role (optional, defaults to Author)
    pub role: Option<UserRole>,
}

/// Input for updating a user
#[derive(Debug, Clone, Default)]
pub struct UpdateUserInput {
    /// New username (optional)
    pub username: Option<String>,
    /// New email (optional)
    pub email: Option<String>,
    /// New password (optional, will be hashed)
    pub password: Option<String>,
    /// New role (optional)
    pub role: Option<UserRole>,
    /// New status (optional)
    pub status: Option<UserStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_new() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hashed_password".to_string(),
            UserRole::Author,
        );

        assert_eq!(user.id, 0);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.role, UserRole::Author);
    }

    #[test]
    fn test_user_is_admin() {
        let admin = User::new("admin".to_string(), "admin@test.com".to_string(), "hash".to_string(), UserRole::Admin);
        let editor = User::new("editor".to_string(), "editor@test.com".to_string(), "hash".to_string(), UserRole::Editor);
        let author = User::new("author".to_string(), "author@test.com".to_string(), "hash".to_string(), UserRole::Author);

        assert!(admin.is_admin());
        assert!(!editor.is_admin());
        assert!(!author.is_admin());
    }

    #[test]
    fn test_user_is_editor() {
        let admin = User::new("admin".to_string(), "admin@test.com".to_string(), "hash".to_string(), UserRole::Admin);
        let editor = User::new("editor".to_string(), "editor@test.com".to_string(), "hash".to_string(), UserRole::Editor);
        let author = User::new("author".to_string(), "author@test.com".to_string(), "hash".to_string(), UserRole::Author);

        assert!(admin.is_editor());
        assert!(editor.is_editor());
        assert!(!author.is_editor());
    }

    #[test]
    fn test_user_can_edit() {
        let mut admin = User::new("admin".to_string(), "admin@test.com".to_string(), "hash".to_string(), UserRole::Admin);
        admin.id = 1;
        
        let mut author = User::new("author".to_string(), "author@test.com".to_string(), "hash".to_string(), UserRole::Author);
        author.id = 2;

        // Admin can edit anyone's content
        assert!(admin.can_edit(1));
        assert!(admin.can_edit(2));
        assert!(admin.can_edit(999));

        // Author can only edit own content
        assert!(author.can_edit(2));
        assert!(!author.can_edit(1));
        assert!(!author.can_edit(999));
    }

    #[test]
    fn test_user_role_display() {
        assert_eq!(UserRole::Admin.to_string(), "admin");
        assert_eq!(UserRole::Editor.to_string(), "editor");
        assert_eq!(UserRole::Author.to_string(), "author");
    }

    #[test]
    fn test_user_role_from_str() {
        assert_eq!(UserRole::from_str("admin").unwrap(), UserRole::Admin);
        assert_eq!(UserRole::from_str("ADMIN").unwrap(), UserRole::Admin);
        assert_eq!(UserRole::from_str("Editor").unwrap(), UserRole::Editor);
        assert_eq!(UserRole::from_str("author").unwrap(), UserRole::Author);
        assert!(UserRole::from_str("invalid").is_err());
    }

    #[test]
    fn test_user_role_default() {
        assert_eq!(UserRole::default(), UserRole::Author);
    }
}
