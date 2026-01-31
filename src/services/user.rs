//! User service
//!
//! Implements business logic for user management:
//! - User registration (first user becomes admin) - Requirement 4.1
//! - Login/logout - Requirements 4.3, 4.5
//! - Session management - Requirements 4.4, 4.7
//! - Password hashing - Requirement 4.6
//!
//! Satisfies requirements:
//! - 4.1: WHEN 第一个用户注册 THEN User_Service SHALL 自动将其设置为管理员角色
//! - 4.3: WHEN 用户登录 THEN User_Service SHALL 验证凭据并返回会话令牌
//! - 4.4: WHEN 会话令牌过期 THEN User_Service SHALL 要求用户重新登录
//! - 4.5: IF 登录凭据无效 THEN User_Service SHALL 返回认证错误
//! - 4.7: WHILE 用户已登录 THEN User_Service SHALL 维护用户会话状态

use crate::db::repositories::{SessionRepository, UserRepository};
use crate::models::{Session, User, UserRole};
use crate::plugin::{HookManager, hook_names};
use crate::services::password::{hash_password, verify_password};
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Default session expiration time in days
const DEFAULT_SESSION_EXPIRATION_DAYS: i64 = 7;

/// Error types for user service operations
#[derive(Debug, thiserror::Error)]
pub enum UserServiceError {
    /// Authentication failed (invalid credentials)
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// Validation error (invalid input)
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// User already exists
    #[error("User already exists: {0}")]
    UserExists(String),

    /// Session expired
    #[error("Session expired")]
    SessionExpired,

    /// Session not found
    #[error("Session not found")]
    SessionNotFound,

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

/// User service for managing users and authentication
pub struct UserService {
    user_repo: Arc<dyn UserRepository>,
    session_repo: Arc<dyn SessionRepository>,
    session_expiration_days: i64,
    hook_manager: Option<Arc<HookManager>>,
}

impl UserService {
    /// Create a new user service with the given repositories
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        session_repo: Arc<dyn SessionRepository>,
    ) -> Self {
        Self {
            user_repo,
            session_repo,
            session_expiration_days: DEFAULT_SESSION_EXPIRATION_DAYS,
            hook_manager: None,
        }
    }

    /// Create a new user service with custom session expiration
    pub fn with_session_expiration(
        user_repo: Arc<dyn UserRepository>,
        session_repo: Arc<dyn SessionRepository>,
        session_expiration_days: i64,
    ) -> Self {
        Self {
            user_repo,
            session_repo,
            session_expiration_days,
            hook_manager: None,
        }
    }

    /// Create a new user service with hook manager
    pub fn with_hooks(
        user_repo: Arc<dyn UserRepository>,
        session_repo: Arc<dyn SessionRepository>,
        hook_manager: Arc<HookManager>,
    ) -> Self {
        Self {
            user_repo,
            session_repo,
            session_expiration_days: DEFAULT_SESSION_EXPIRATION_DAYS,
            hook_manager: Some(hook_manager),
        }
    }

    /// Trigger a hook if hook manager is available
    fn trigger_hook(&self, name: &str, data: serde_json::Value) -> serde_json::Value {
        if let Some(ref manager) = self.hook_manager {
            manager.trigger(name, data)
        } else {
            data
        }
    }

    /// Register a new user
    ///
    /// If this is the first user in the system, they will automatically
    /// be assigned the Admin role (Requirement 4.1).
    ///
    /// # Arguments
    ///
    /// * `input` - Registration input containing username, email, and password
    ///
    /// # Returns
    ///
    /// The created user on success
    ///
    /// # Errors
    ///
    /// - `ValidationError` if username or email is empty
    /// - `UserExists` if username or email is already taken
    /// - `InternalError` for database errors
    ///
    /// Satisfies requirement 4.1: WHEN 第一个用户注册 THEN User_Service SHALL 自动将其设置为管理员角色
    /// 
    /// # Hooks
    /// - `user_register_before` - Triggered before registration, can modify input
    /// - `user_register_after` - Triggered after registration, receives created user
    pub async fn register(&self, input: RegisterInput) -> Result<User, UserServiceError> {
        // Trigger user_register_before hook
        let hook_data = self.trigger_hook(
            hook_names::USER_REGISTER_BEFORE,
            json!({
                "username": input.username,
                "email": input.email,
            })
        );
        
        // Check if hook wants to block registration
        if hook_data.get("blocked").and_then(|v| v.as_bool()).unwrap_or(false) {
            let reason = hook_data.get("reason").and_then(|v| v.as_str()).unwrap_or("Registration blocked");
            return Err(UserServiceError::ValidationError(reason.to_string()));
        }

        // Validate input
        self.validate_register_input(&input)?;

        // Check if username already exists
        if self
            .user_repo
            .get_by_username(&input.username)
            .await
            .context("Failed to check username")?
            .is_some()
        {
            return Err(UserServiceError::UserExists(format!(
                "Username '{}' is already taken",
                input.username
            )));
        }

        // Check if email already exists
        if self
            .user_repo
            .get_by_email(&input.email)
            .await
            .context("Failed to check email")?
            .is_some()
        {
            return Err(UserServiceError::UserExists(format!(
                "Email '{}' is already registered",
                input.email
            )));
        }

        // Determine role: first user becomes admin (Requirement 4.1)
        let is_first = self.is_first_user().await?;
        let role = if is_first {
            UserRole::Admin
        } else {
            UserRole::Author
        };

        // Hash password
        let password_hash =
            hash_password(&input.password).context("Failed to hash password")?;

        // Create user
        let user = User::new(input.username, input.email, password_hash, role);

        let created_user = self
            .user_repo
            .create(&user)
            .await
            .context("Failed to create user")?;

        // Trigger user_register_after hook
        self.trigger_hook(
            hook_names::USER_REGISTER_AFTER,
            json!({
                "id": created_user.id,
                "username": created_user.username,
                "email": created_user.email,
                "role": format!("{:?}", created_user.role),
            })
        );

        Ok(created_user)
    }

    /// Login with credentials
    ///
    /// Validates the provided credentials and creates a new session if valid.
    ///
    /// # Arguments
    ///
    /// * `input` - Login input containing username/email and password
    ///
    /// # Returns
    ///
    /// A new session on success
    ///
    /// # Errors
    ///
    /// - `AuthenticationError` if credentials are invalid (Requirement 4.5)
    /// - `InternalError` for database errors
    ///
    /// # Hooks
    /// - `user_login_before` - Triggered before login validation
    /// - `user_login_after` - Triggered after successful login
    /// - `user_login_failed` - Triggered when login fails
    ///
    /// Satisfies requirements:
    /// - 4.3: WHEN 用户登录 THEN User_Service SHALL 验证凭据并返回会话令牌
    /// - 4.5: IF 登录凭据无效 THEN User_Service SHALL 返回认证错误
    pub async fn login(&self, input: LoginInput) -> Result<Session, UserServiceError> {
        // Trigger user_login_before hook
        let hook_data = self.trigger_hook(
            hook_names::USER_LOGIN_BEFORE,
            json!({
                "username_or_email": input.username_or_email,
            })
        );
        
        // Check if hook wants to block login
        if hook_data.get("blocked").and_then(|v| v.as_bool()).unwrap_or(false) {
            let reason = hook_data.get("reason").and_then(|v| v.as_str()).unwrap_or("Login blocked");
            return Err(UserServiceError::AuthenticationError(reason.to_string()));
        }

        // Find user by username or email
        let user = self
            .find_user_by_username_or_email(&input.username_or_email)
            .await?
            .ok_or_else(|| {
                // Trigger user_login_failed hook
                self.trigger_hook(
                    hook_names::USER_LOGIN_FAILED,
                    json!({
                        "username_or_email": input.username_or_email,
                        "reason": "user_not_found",
                    })
                );
                UserServiceError::AuthenticationError("Invalid username or password".to_string())
            })?;

        // Verify password
        let password_valid = verify_password(&input.password, &user.password_hash)
            .context("Failed to verify password")?;

        if !password_valid {
            // Trigger user_login_failed hook
            self.trigger_hook(
                hook_names::USER_LOGIN_FAILED,
                json!({
                    "username_or_email": input.username_or_email,
                    "user_id": user.id,
                    "reason": "invalid_password",
                })
            );
            return Err(UserServiceError::AuthenticationError(
                "Invalid username or password".to_string(),
            ));
        }

        // Check if user is banned
        if user.is_banned() {
            // Trigger user_login_failed hook
            self.trigger_hook(
                hook_names::USER_LOGIN_FAILED,
                json!({
                    "username_or_email": input.username_or_email,
                    "user_id": user.id,
                    "reason": "user_banned",
                })
            );
            return Err(UserServiceError::AuthenticationError(
                "Your account has been banned. Please contact the administrator.".to_string(),
            ));
        }

        // Create session
        let session = self.create_session(user.id).await?;

        // Trigger user_login_after hook
        self.trigger_hook(
            hook_names::USER_LOGIN_AFTER,
            json!({
                "user_id": user.id,
                "username": user.username,
                "session_id": session.id,
            })
        );

        Ok(session)
    }

    /// Logout (invalidate session)
    ///
    /// Deletes the session from the database, effectively logging out the user.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID (token) to invalidate
    ///
    /// # Returns
    ///
    /// `Ok(())` on success
    ///
    /// # Errors
    ///
    /// - `InternalError` for database errors
    ///
    /// # Hooks
    /// - `user_logout` - Triggered when user logs out
    pub async fn logout(&self, session_id: &str) -> Result<(), UserServiceError> {
        // Trigger user_logout hook
        self.trigger_hook(
            hook_names::USER_LOGOUT,
            json!({
                "session_id": session_id,
            })
        );

        self.session_repo
            .delete(session_id)
            .await
            .context("Failed to delete session")?;

        Ok(())
    }

    /// Get user by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The user ID
    ///
    /// # Returns
    ///
    /// The user if found, `None` otherwise
    pub async fn get_by_id(&self, id: i64) -> Result<Option<User>, UserServiceError> {
        let user = self
            .user_repo
            .get_by_id(id)
            .await
            .context("Failed to get user by ID")?;

        Ok(user)
    }

    /// Update a user
    ///
    /// # Arguments
    ///
    /// * `user` - The user with updated fields
    ///
    /// # Returns
    ///
    /// The updated user
    pub async fn update_user(&self, user: User) -> Result<User, UserServiceError> {
        let updated = self
            .user_repo
            .update(&user)
            .await
            .context("Failed to update user")?;
        
        Ok(updated)
    }

    /// Validate session token and return the associated user
    ///
    /// Checks if the session exists and is not expired. If valid, returns
    /// the associated user.
    ///
    /// # Arguments
    ///
    /// * `token` - The session token to validate
    ///
    /// # Returns
    ///
    /// The user if the session is valid, `None` if the session doesn't exist
    /// or is expired.
    ///
    /// # Errors
    ///
    /// - `InternalError` for database errors
    ///
    /// Satisfies requirements:
    /// - 4.4: WHEN 会话令牌过期 THEN User_Service SHALL 要求用户重新登录
    /// - 4.7: WHILE 用户已登�?THEN User_Service SHALL 维护用户会话状�?
    pub async fn validate_session(&self, token: &str) -> Result<Option<User>, UserServiceError> {
        // Get session
        let session = match self
            .session_repo
            .get_by_id(token)
            .await
            .context("Failed to get session")?
        {
            Some(s) => s,
            None => return Ok(None),
        };

        // Check if session is expired (Requirement 4.4)
        if session.is_expired() {
            // Clean up expired session
            let _ = self.session_repo.delete(token).await;
            return Ok(None);
        }

        // Get user
        let user = self
            .user_repo
            .get_by_id(session.user_id)
            .await
            .context("Failed to get user")?;

        Ok(user)
    }

    /// Check if this is the first user (for auto-admin)
    ///
    /// # Returns
    ///
    /// `true` if no users exist in the system, `false` otherwise
    ///
    /// Satisfies requirement 4.1: WHEN 第一个用户注�?THEN User_Service SHALL 自动将其设置为管理员角色
    pub async fn is_first_user(&self) -> Result<bool, UserServiceError> {
        let count = self
            .user_repo
            .count()
            .await
            .context("Failed to count users")?;

        Ok(count == 0)
    }

    /// Get user by username
    pub async fn get_by_username(&self, username: &str) -> Result<Option<User>, UserServiceError> {
        let user = self
            .user_repo
            .get_by_username(username)
            .await
            .context("Failed to get user by username")?;

        Ok(user)
    }

    /// Get user by email
    pub async fn get_by_email(&self, email: &str) -> Result<Option<User>, UserServiceError> {
        let user = self
            .user_repo
            .get_by_email(email)
            .await
            .context("Failed to get user by email")?;

        Ok(user)
    }

    /// Delete all expired sessions
    ///
    /// This is a maintenance operation that should be called periodically
    /// to clean up expired sessions.
    ///
    /// # Returns
    ///
    /// The number of sessions deleted
    pub async fn cleanup_expired_sessions(&self) -> Result<i64, UserServiceError> {
        let count = self
            .session_repo
            .delete_expired()
            .await
            .context("Failed to delete expired sessions")?;

        Ok(count)
    }

    // ========================================================================
    // Private helper methods
    // ========================================================================

    /// Validate registration input
    fn validate_register_input(&self, input: &RegisterInput) -> Result<(), UserServiceError> {
        if input.username.trim().is_empty() {
            return Err(UserServiceError::ValidationError(
                "Username cannot be empty".to_string(),
            ));
        }

        if input.email.trim().is_empty() {
            return Err(UserServiceError::ValidationError(
                "Email cannot be empty".to_string(),
            ));
        }

        if input.password.is_empty() {
            return Err(UserServiceError::ValidationError(
                "Password cannot be empty".to_string(),
            ));
        }

        // Basic email format validation
        if !input.email.contains('@') {
            return Err(UserServiceError::ValidationError(
                "Invalid email format".to_string(),
            ));
        }

        Ok(())
    }

    /// Find user by username or email
    async fn find_user_by_username_or_email(
        &self,
        username_or_email: &str,
    ) -> Result<Option<User>, UserServiceError> {
        // Try to find by username first
        if let Some(user) = self
            .user_repo
            .get_by_username(username_or_email)
            .await
            .context("Failed to get user by username")?
        {
            return Ok(Some(user));
        }

        // Try to find by email
        let user = self
            .user_repo
            .get_by_email(username_or_email)
            .await
            .context("Failed to get user by email")?;

        Ok(user)
    }

    /// Create a new session for a user
    async fn create_session(&self, user_id: i64) -> Result<Session, UserServiceError> {
        let now = Utc::now();
        let session = Session {
            id: Uuid::new_v4().to_string(),
            user_id,
            expires_at: now + Duration::days(self.session_expiration_days),
            created_at: now,
        };

        let created = self
            .session_repo
            .create(&session)
            .await
            .context("Failed to create session")?;

        Ok(created)
    }
}

/// Input for user registration
#[derive(Debug, Clone)]
pub struct RegisterInput {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl RegisterInput {
    /// Create a new registration input
    pub fn new(username: impl Into<String>, email: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            email: email.into(),
            password: password.into(),
        }
    }
}

/// Input for user login
#[derive(Debug, Clone)]
pub struct LoginInput {
    pub username_or_email: String,
    pub password: String,
}

impl LoginInput {
    /// Create a new login input
    pub fn new(username_or_email: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username_or_email: username_or_email.into(),
            password: password.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repositories::{SqlxSessionRepository, SqlxUserRepository};
    use crate::db::{create_test_pool, migrations, DynDatabasePool};

    async fn setup_test_service() -> (DynDatabasePool, UserService) {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let user_repo = SqlxUserRepository::boxed(pool.clone());
        let session_repo = SqlxSessionRepository::boxed(pool.clone());
        let service = UserService::new(user_repo, session_repo);

        (pool, service)
    }

    // ========================================================================
    // Registration tests
    // ========================================================================

    #[tokio::test]
    async fn test_register_first_user_becomes_admin() {
        let (_pool, service) = setup_test_service().await;

        let input = RegisterInput::new("admin", "admin@example.com", "password123");
        let user = service.register(input).await.expect("Failed to register");

        // First user should be admin (Requirement 4.1)
        assert_eq!(user.role, UserRole::Admin);
        assert_eq!(user.username, "admin");
        assert_eq!(user.email, "admin@example.com");
    }

    #[tokio::test]
    async fn test_register_second_user_becomes_author() {
        let (_pool, service) = setup_test_service().await;

        // Register first user (admin)
        let input1 = RegisterInput::new("admin", "admin@example.com", "password123");
        service.register(input1).await.expect("Failed to register first user");

        // Register second user (should be author)
        let input2 = RegisterInput::new("author", "author@example.com", "password456");
        let user = service.register(input2).await.expect("Failed to register second user");

        assert_eq!(user.role, UserRole::Author);
    }

    #[tokio::test]
    async fn test_register_duplicate_username_fails() {
        let (_pool, service) = setup_test_service().await;

        let input1 = RegisterInput::new("testuser", "user1@example.com", "password123");
        service.register(input1).await.expect("Failed to register first user");

        let input2 = RegisterInput::new("testuser", "user2@example.com", "password456");
        let result = service.register(input2).await;

        assert!(matches!(result, Err(UserServiceError::UserExists(_))));
    }

    #[tokio::test]
    async fn test_register_duplicate_email_fails() {
        let (_pool, service) = setup_test_service().await;

        let input1 = RegisterInput::new("user1", "same@example.com", "password123");
        service.register(input1).await.expect("Failed to register first user");

        let input2 = RegisterInput::new("user2", "same@example.com", "password456");
        let result = service.register(input2).await;

        assert!(matches!(result, Err(UserServiceError::UserExists(_))));
    }

    #[tokio::test]
    async fn test_register_empty_username_fails() {
        let (_pool, service) = setup_test_service().await;

        let input = RegisterInput::new("", "test@example.com", "password123");
        let result = service.register(input).await;

        assert!(matches!(result, Err(UserServiceError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_register_empty_email_fails() {
        let (_pool, service) = setup_test_service().await;

        let input = RegisterInput::new("testuser", "", "password123");
        let result = service.register(input).await;

        assert!(matches!(result, Err(UserServiceError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_register_empty_password_fails() {
        let (_pool, service) = setup_test_service().await;

        let input = RegisterInput::new("testuser", "test@example.com", "");
        let result = service.register(input).await;

        assert!(matches!(result, Err(UserServiceError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_register_invalid_email_fails() {
        let (_pool, service) = setup_test_service().await;

        let input = RegisterInput::new("testuser", "invalid-email", "password123");
        let result = service.register(input).await;

        assert!(matches!(result, Err(UserServiceError::ValidationError(_))));
    }

    // ========================================================================
    // Login tests
    // ========================================================================

    #[tokio::test]
    async fn test_login_with_username_success() {
        let (_pool, service) = setup_test_service().await;

        // Register user
        let register_input = RegisterInput::new("testuser", "test@example.com", "password123");
        service.register(register_input).await.expect("Failed to register");

        // Login with username
        let login_input = LoginInput::new("testuser", "password123");
        let session = service.login(login_input).await.expect("Failed to login");

        assert!(!session.id.is_empty());
        assert!(!session.is_expired());
    }

    #[tokio::test]
    async fn test_login_with_email_success() {
        let (_pool, service) = setup_test_service().await;

        // Register user
        let register_input = RegisterInput::new("testuser", "test@example.com", "password123");
        service.register(register_input).await.expect("Failed to register");

        // Login with email
        let login_input = LoginInput::new("test@example.com", "password123");
        let session = service.login(login_input).await.expect("Failed to login");

        assert!(!session.id.is_empty());
        assert!(!session.is_expired());
    }

    #[tokio::test]
    async fn test_login_wrong_password_fails() {
        let (_pool, service) = setup_test_service().await;

        // Register user
        let register_input = RegisterInput::new("testuser", "test@example.com", "password123");
        service.register(register_input).await.expect("Failed to register");

        // Login with wrong password (Requirement 4.5)
        let login_input = LoginInput::new("testuser", "wrongpassword");
        let result = service.login(login_input).await;

        assert!(matches!(result, Err(UserServiceError::AuthenticationError(_))));
    }

    #[tokio::test]
    async fn test_login_nonexistent_user_fails() {
        let (_pool, service) = setup_test_service().await;

        // Login with nonexistent user (Requirement 4.5)
        let login_input = LoginInput::new("nonexistent", "password123");
        let result = service.login(login_input).await;

        assert!(matches!(result, Err(UserServiceError::AuthenticationError(_))));
    }

    // ========================================================================
    // Session validation tests
    // ========================================================================

    #[tokio::test]
    async fn test_validate_session_success() {
        let (_pool, service) = setup_test_service().await;

        // Register and login
        let register_input = RegisterInput::new("testuser", "test@example.com", "password123");
        let registered_user = service.register(register_input).await.expect("Failed to register");

        let login_input = LoginInput::new("testuser", "password123");
        let session = service.login(login_input).await.expect("Failed to login");

        // Validate session (Requirement 4.7)
        let user = service
            .validate_session(&session.id)
            .await
            .expect("Failed to validate session")
            .expect("User not found");

        assert_eq!(user.id, registered_user.id);
        assert_eq!(user.username, "testuser");
    }

    #[tokio::test]
    async fn test_validate_session_nonexistent_returns_none() {
        let (_pool, service) = setup_test_service().await;

        let result = service
            .validate_session("nonexistent-session-id")
            .await
            .expect("Failed to validate session");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_validate_expired_session_returns_none() {
        // Create service with very short session expiration
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let user_repo = SqlxUserRepository::boxed(pool.clone());
        let session_repo = SqlxSessionRepository::boxed(pool.clone());

        // Create service with -1 day expiration (already expired)
        let service = UserService::with_session_expiration(user_repo, session_repo, -1);

        // Register and login
        let register_input = RegisterInput::new("testuser", "test@example.com", "password123");
        service.register(register_input).await.expect("Failed to register");

        let login_input = LoginInput::new("testuser", "password123");
        let session = service.login(login_input).await.expect("Failed to login");

        // Session should be expired immediately (Requirement 4.4)
        let result = service
            .validate_session(&session.id)
            .await
            .expect("Failed to validate session");

        assert!(result.is_none());
    }

    // ========================================================================
    // Logout tests
    // ========================================================================

    #[tokio::test]
    async fn test_logout_invalidates_session() {
        let (_pool, service) = setup_test_service().await;

        // Register and login
        let register_input = RegisterInput::new("testuser", "test@example.com", "password123");
        service.register(register_input).await.expect("Failed to register");

        let login_input = LoginInput::new("testuser", "password123");
        let session = service.login(login_input).await.expect("Failed to login");

        // Logout
        service.logout(&session.id).await.expect("Failed to logout");

        // Session should no longer be valid
        let result = service
            .validate_session(&session.id)
            .await
            .expect("Failed to validate session");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_logout_nonexistent_session_succeeds() {
        let (_pool, service) = setup_test_service().await;

        // Logout with nonexistent session should not error
        let result = service.logout("nonexistent-session-id").await;
        assert!(result.is_ok());
    }

    // ========================================================================
    // Other tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_by_id() {
        let (_pool, service) = setup_test_service().await;

        let register_input = RegisterInput::new("testuser", "test@example.com", "password123");
        let registered = service.register(register_input).await.expect("Failed to register");

        let user = service
            .get_by_id(registered.id)
            .await
            .expect("Failed to get user")
            .expect("User not found");

        assert_eq!(user.id, registered.id);
        assert_eq!(user.username, "testuser");
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let (_pool, service) = setup_test_service().await;

        let result = service.get_by_id(999).await.expect("Failed to get user");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_is_first_user() {
        let (_pool, service) = setup_test_service().await;

        // Initially should be first user
        assert!(service.is_first_user().await.expect("Failed to check"));

        // Register a user
        let input = RegisterInput::new("testuser", "test@example.com", "password123");
        service.register(input).await.expect("Failed to register");

        // Now should not be first user
        assert!(!service.is_first_user().await.expect("Failed to check"));
    }

    #[tokio::test]
    async fn test_cleanup_expired_sessions() {
        // Create service with expired sessions
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let user_repo = SqlxUserRepository::boxed(pool.clone());
        let session_repo = SqlxSessionRepository::boxed(pool.clone());

        // Create service with -1 day expiration (already expired)
        let service = UserService::with_session_expiration(user_repo, session_repo, -1);

        // Register and login (creates expired session)
        let register_input = RegisterInput::new("testuser", "test@example.com", "password123");
        service.register(register_input).await.expect("Failed to register");

        let login_input = LoginInput::new("testuser", "password123");
        service.login(login_input).await.expect("Failed to login");

        // Cleanup expired sessions
        let count = service
            .cleanup_expired_sessions()
            .await
            .expect("Failed to cleanup");

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_multiple_sessions_per_user() {
        let (_pool, service) = setup_test_service().await;

        // Register user
        let register_input = RegisterInput::new("testuser", "test@example.com", "password123");
        service.register(register_input).await.expect("Failed to register");

        // Login multiple times (creates multiple sessions)
        let login_input1 = LoginInput::new("testuser", "password123");
        let session1 = service.login(login_input1).await.expect("Failed to login");

        let login_input2 = LoginInput::new("testuser", "password123");
        let session2 = service.login(login_input2).await.expect("Failed to login");

        // Both sessions should be valid
        assert!(service.validate_session(&session1.id).await.unwrap().is_some());
        assert!(service.validate_session(&session2.id).await.unwrap().is_some());

        // Sessions should be different
        assert_ne!(session1.id, session2.id);
    }

    #[tokio::test]
    async fn test_password_is_hashed() {
        let (_pool, service) = setup_test_service().await;

        let password = "my_secret_password";
        let register_input = RegisterInput::new("testuser", "test@example.com", password);
        let user = service.register(register_input).await.expect("Failed to register");

        // Password should be hashed, not stored in plaintext
        assert_ne!(user.password_hash, password);
        assert!(user.password_hash.starts_with("$argon2id$"));
    }
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::db::repositories::{SqlxSessionRepository, SqlxUserRepository};
    use crate::db::{create_test_pool, migrations};
    use crate::services::password::{hash_password, verify_password};
    use proptest::prelude::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Counter for generating unique usernames/emails across test iterations
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Helper to create a unique test service for each property test iteration
    async fn setup_property_test_service() -> UserService {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let user_repo = SqlxUserRepository::boxed(pool.clone());
        let session_repo = SqlxSessionRepository::boxed(pool.clone());
        UserService::new(user_repo, session_repo)
    }

    /// Helper to create a service with custom session expiration
    async fn setup_property_test_service_with_expiration(days: i64) -> UserService {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let user_repo = SqlxUserRepository::boxed(pool.clone());
        let session_repo = SqlxSessionRepository::boxed(pool.clone());
        UserService::with_session_expiration(user_repo, session_repo, days)
    }

    /// Generate a unique suffix for test data
    fn unique_suffix() -> u64 {
        TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    // ========================================================================
    // Property 9: 用户认证往�?(User Authentication Roundtrip)
    // For any valid credentials, login should return a token that validates
    // to the same user.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Validates: Requirements 4.3, 4.7**
        ///
        /// Property 9: User Authentication Roundtrip
        /// For any valid credentials, login should return a token that validates
        /// to the same user.
        #[test]
        fn property_9_user_auth_roundtrip(
            username in "[a-z]{3,10}",
            email_prefix in "[a-z]{3,10}",
            password in "[a-zA-Z0-9!@#$%^&*]{8,20}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let service = setup_property_test_service().await;
                let suffix = unique_suffix();

                // Create unique username and email for this iteration
                let unique_username = format!("{}_{}", username, suffix);
                let unique_email = format!("{}_{}@example.com", email_prefix, suffix);

                // Register user
                let register_input = RegisterInput::new(
                    unique_username.clone(),
                    unique_email.clone(),
                    password.clone(),
                );
                let registered_user = service.register(register_input).await
                    .expect("Registration should succeed");

                // Login with the same credentials
                let login_input = LoginInput::new(unique_username.clone(), password.clone());
                let session = service.login(login_input).await
                    .expect("Login should succeed with valid credentials");

                // Validate the session token
                let validated_user = service.validate_session(&session.id).await
                    .expect("Session validation should not error")
                    .expect("Session should be valid and return user");

                // The validated user should be the same as the registered user
                prop_assert_eq!(validated_user.id, registered_user.id);
                prop_assert_eq!(validated_user.username, registered_user.username);
                prop_assert_eq!(validated_user.email, registered_user.email);
                Ok(())
            });
            result?;
        }
    }

    // ========================================================================
    // Property 10: 密码安全存储 (Password Secure Storage)
    // For any password, the stored hash should differ from the original
    // and correct password should verify.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Validates: Requirements 4.6**
        ///
        /// Property 10: Password Secure Storage
        /// For any password, the stored hash should differ from the original
        /// and correct password should verify.
        #[test]
        fn property_10_password_secure_storage(
            password in "[a-zA-Z0-9!@#$%^&*()_+-=]{1,50}"
        ) {
            // Hash the password
            let hash = hash_password(&password)
                .expect("Password hashing should succeed");

            // Property 1: Hash should differ from original password
            prop_assert_ne!(&hash, &password, "Hash must differ from original password");

            // Property 2: Hash should be in Argon2id format (secure algorithm)
            prop_assert!(hash.starts_with("$argon2id$"), "Hash should use Argon2id algorithm");

            // Property 3: Hash should have sufficient length (Argon2 hashes are typically 90+ chars)
            prop_assert!(hash.len() > 80, "Hash should have sufficient length for security");

            // Property 4: Correct password should verify successfully
            let verify_result = verify_password(&password, &hash)
                .expect("Password verification should not error");
            prop_assert!(verify_result, "Correct password should verify successfully");

            // Property 5: Wrong password should NOT verify
            let wrong_password = format!("{}wrong", password);
            let wrong_verify_result = verify_password(&wrong_password, &hash)
                .expect("Password verification should not error");
            prop_assert!(!wrong_verify_result, "Wrong password should not verify");

            // Property 6: Same password hashed twice should produce different hashes (due to random salt)
            let hash2 = hash_password(&password)
                .expect("Second password hashing should succeed");
            prop_assert_ne!(&hash, &hash2, "Same password should produce different hashes due to random salt");
        }
    }

    // ========================================================================
    // Property 11: 无效凭据拒绝 (Invalid Credentials Rejection)
    // For any wrong password or nonexistent username, login should return
    // authentication error.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Validates: Requirements 4.5**
        ///
        /// Property 11: Invalid Credentials Rejection
        /// For any wrong password or nonexistent username, login should return
        /// authentication error.
        #[test]
        fn property_11_invalid_credentials_rejection(
            username in "[a-z]{3,10}",
            email_prefix in "[a-z]{3,10}",
            correct_password in "[a-zA-Z0-9]{8,20}",
            wrong_password in "[a-zA-Z0-9]{8,20}",
            nonexistent_username in "[a-z]{3,10}"
        ) {
            // Skip if passwords happen to be the same
            prop_assume!(correct_password != wrong_password);

            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let service = setup_property_test_service().await;
                let suffix = unique_suffix();

                // Create unique identifiers
                let unique_username = format!("{}_{}", username, suffix);
                let unique_email = format!("{}_{}@example.com", email_prefix, suffix);
                let unique_nonexistent = format!("nonexist_{}_{}", nonexistent_username, suffix);

                // Register a user
                let register_input = RegisterInput::new(
                    unique_username.clone(),
                    unique_email.clone(),
                    correct_password.clone(),
                );
                service.register(register_input).await
                    .expect("Registration should succeed");

                // Test 1: Wrong password should fail
                let wrong_password_input = LoginInput::new(
                    unique_username.clone(),
                    wrong_password.clone(),
                );
                let wrong_password_result = service.login(wrong_password_input).await;
                prop_assert!(
                    matches!(wrong_password_result, Err(UserServiceError::AuthenticationError(_))),
                    "Wrong password should return AuthenticationError"
                );

                // Test 2: Nonexistent username should fail
                let nonexistent_input = LoginInput::new(
                    unique_nonexistent.clone(),
                    correct_password.clone(),
                );
                let nonexistent_result = service.login(nonexistent_input).await;
                prop_assert!(
                    matches!(nonexistent_result, Err(UserServiceError::AuthenticationError(_))),
                    "Nonexistent username should return AuthenticationError"
                );
                Ok(())
            });
            result?;
        }
    }

    // ========================================================================
    // Property 12: 会话过期处理 (Session Expiration Handling)
    // For any expired session token, validation should return invalid
    // and require re-login.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Validates: Requirements 4.4**
        ///
        /// Property 12: Session Expiration Handling
        /// For any expired session token, validation should return invalid
        /// and require re-login.
        #[test]
        fn property_12_session_expiration_handling(
            username in "[a-z]{3,10}",
            email_prefix in "[a-z]{3,10}",
            password in "[a-zA-Z0-9]{8,20}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                // Create service with -1 day expiration (sessions are immediately expired)
                let service = setup_property_test_service_with_expiration(-1).await;
                let suffix = unique_suffix();

                // Create unique identifiers
                let unique_username = format!("{}_{}", username, suffix);
                let unique_email = format!("{}_{}@example.com", email_prefix, suffix);

                // Register user
                let register_input = RegisterInput::new(
                    unique_username.clone(),
                    unique_email.clone(),
                    password.clone(),
                );
                service.register(register_input).await
                    .expect("Registration should succeed");

                // Login (creates an already-expired session)
                let login_input = LoginInput::new(unique_username.clone(), password.clone());
                let session = service.login(login_input).await
                    .expect("Login should succeed");

                // The session should be expired
                prop_assert!(session.is_expired(), "Session should be expired");

                // Validating the expired session should return None
                let validation_result = service.validate_session(&session.id).await
                    .expect("Session validation should not error");
                prop_assert!(
                    validation_result.is_none(),
                    "Expired session validation should return None"
                );

                // User should be able to re-login and get a new session
                let relogin_input = LoginInput::new(unique_username.clone(), password.clone());
                let new_session = service.login(relogin_input).await
                    .expect("Re-login should succeed");

                // New session should be different from the old one
                prop_assert_ne!(
                    new_session.id, session.id,
                    "New session should have different ID"
                );
                Ok(())
            });
            result?;
        }
    }
}
