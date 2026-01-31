//! User repository
//!
//! Database operations for users.
//!
//! This module provides:
//! - `UserRepository` trait defining the interface for user data access
//! - `SqlxUserRepository` implementing the trait for SQLite and MySQL
//!
//! Satisfies requirements:
//! - 4.2: WHEN 用户提交注册信息 THEN User_Service SHALL 验证邮箱唯一性并创建账户

use crate::config::DatabaseDriver;
use crate::db::DynDatabasePool;
use crate::models::{User, UserRole, UserStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{MySqlPool, Row, SqlitePool};
use std::str::FromStr;
use std::sync::Arc;

/// User repository trait
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Create a new user
    async fn create(&self, user: &User) -> Result<User>;

    /// Get user by ID
    async fn get_by_id(&self, id: i64) -> Result<Option<User>>;

    /// Get user by username
    async fn get_by_username(&self, username: &str) -> Result<Option<User>>;

    /// Get user by email
    async fn get_by_email(&self, email: &str) -> Result<Option<User>>;

    /// Update a user
    async fn update(&self, user: &User) -> Result<User>;

    /// Delete a user
    async fn delete(&self, id: i64) -> Result<()>;

    /// Count total users
    async fn count(&self) -> Result<i64>;

    /// List all users with pagination
    async fn list(&self, page: i64, per_page: i64) -> Result<(Vec<User>, i64)>;
}

/// SQLx-based user repository implementation
///
/// Supports both SQLite and MySQL databases.
pub struct SqlxUserRepository {
    pool: DynDatabasePool,
}

impl SqlxUserRepository {
    /// Create a new SQLx user repository
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }

    /// Create a boxed repository for use with dependency injection
    pub fn boxed(pool: DynDatabasePool) -> Arc<dyn UserRepository> {
        Arc::new(Self::new(pool))
    }
}

#[async_trait]
impl UserRepository for SqlxUserRepository {
    async fn create(&self, user: &User) -> Result<User> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => create_user_sqlite(self.pool.as_sqlite().unwrap(), user).await,
            DatabaseDriver::Mysql => create_user_mysql(self.pool.as_mysql().unwrap(), user).await,
        }
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<User>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => get_user_by_id_sqlite(self.pool.as_sqlite().unwrap(), id).await,
            DatabaseDriver::Mysql => get_user_by_id_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
    }

    async fn get_by_username(&self, username: &str) -> Result<Option<User>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_user_by_username_sqlite(self.pool.as_sqlite().unwrap(), username).await
            }
            DatabaseDriver::Mysql => {
                get_user_by_username_mysql(self.pool.as_mysql().unwrap(), username).await
            }
        }
    }

    async fn get_by_email(&self, email: &str) -> Result<Option<User>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_user_by_email_sqlite(self.pool.as_sqlite().unwrap(), email).await
            }
            DatabaseDriver::Mysql => {
                get_user_by_email_mysql(self.pool.as_mysql().unwrap(), email).await
            }
        }
    }

    async fn update(&self, user: &User) -> Result<User> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => update_user_sqlite(self.pool.as_sqlite().unwrap(), user).await,
            DatabaseDriver::Mysql => update_user_mysql(self.pool.as_mysql().unwrap(), user).await,
        }
    }

    async fn delete(&self, id: i64) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => delete_user_sqlite(self.pool.as_sqlite().unwrap(), id).await,
            DatabaseDriver::Mysql => delete_user_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
    }

    async fn count(&self) -> Result<i64> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => count_users_sqlite(self.pool.as_sqlite().unwrap()).await,
            DatabaseDriver::Mysql => count_users_mysql(self.pool.as_mysql().unwrap()).await,
        }
    }

    async fn list(&self, page: i64, per_page: i64) -> Result<(Vec<User>, i64)> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => list_users_sqlite(self.pool.as_sqlite().unwrap(), page, per_page).await,
            DatabaseDriver::Mysql => list_users_mysql(self.pool.as_mysql().unwrap(), page, per_page).await,
        }
    }
}

// ============================================================================
// SQLite implementations
// ============================================================================

async fn create_user_sqlite(pool: &SqlitePool, user: &User) -> Result<User> {
    let now = Utc::now();
    let role_str = user.role.to_string();
    let status_str = user.status.to_string();

    let result = sqlx::query(
        r#"
        INSERT INTO users (username, email, password_hash, role, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&user.username)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&role_str)
    .bind(&status_str)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create user")?;

    let id = result.last_insert_rowid();

    Ok(User {
        id,
        username: user.username.clone(),
        email: user.email.clone(),
        password_hash: user.password_hash.clone(),
        role: user.role,
        status: user.status,
        created_at: now,
        updated_at: now,
    })
}

async fn get_user_by_id_sqlite(pool: &SqlitePool, id: i64) -> Result<Option<User>> {
    let row = sqlx::query(
        r#"
        SELECT id, username, email, password_hash, role, status, created_at, updated_at
        FROM users
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get user by ID")?;

    match row {
        Some(row) => Ok(Some(row_to_user_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn get_user_by_username_sqlite(pool: &SqlitePool, username: &str) -> Result<Option<User>> {
    let row = sqlx::query(
        r#"
        SELECT id, username, email, password_hash, role, status, created_at, updated_at
        FROM users
        WHERE username = ?
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .context("Failed to get user by username")?;

    match row {
        Some(row) => Ok(Some(row_to_user_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn get_user_by_email_sqlite(pool: &SqlitePool, email: &str) -> Result<Option<User>> {
    let row = sqlx::query(
        r#"
        SELECT id, username, email, password_hash, role, status, created_at, updated_at
        FROM users
        WHERE email = ?
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .context("Failed to get user by email")?;

    match row {
        Some(row) => Ok(Some(row_to_user_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn update_user_sqlite(pool: &SqlitePool, user: &User) -> Result<User> {
    let now = Utc::now();
    let role_str = user.role.to_string();
    let status_str = user.status.to_string();

    sqlx::query(
        r#"
        UPDATE users
        SET username = ?, email = ?, password_hash = ?, role = ?, status = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&user.username)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&role_str)
    .bind(&status_str)
    .bind(now)
    .bind(user.id)
    .execute(pool)
    .await
    .context("Failed to update user")?;

    // Return the updated user
    get_user_by_id_sqlite(pool, user.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User not found after update"))
}

async fn delete_user_sqlite(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete user")?;

    Ok(())
}

async fn count_users_sqlite(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM users")
        .fetch_one(pool)
        .await
        .context("Failed to count users")?;

    Ok(row.get("count"))
}

fn row_to_user_sqlite(row: &sqlx::sqlite::SqliteRow) -> Result<User> {
    let role_str: String = row.get("role");
    let role = UserRole::from_str(&role_str)
        .with_context(|| format!("Invalid role in database: {}", role_str))?;

    let status_str: Option<String> = row.try_get("status").ok();
    let status = status_str
        .map(|s| UserStatus::from_str(&s).unwrap_or(UserStatus::Active))
        .unwrap_or(UserStatus::Active);

    Ok(User {
        id: row.get("id"),
        username: row.get("username"),
        email: row.get("email"),
        password_hash: row.get("password_hash"),
        role,
        status,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

async fn list_users_sqlite(pool: &SqlitePool, page: i64, per_page: i64) -> Result<(Vec<User>, i64)> {
    let offset = (page - 1) * per_page;
    
    let rows = sqlx::query(
        r#"
        SELECT id, username, email, password_hash, role, status, created_at, updated_at
        FROM users
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list users")?;

    let mut users = Vec::new();
    for row in rows {
        users.push(row_to_user_sqlite(&row)?);
    }

    let total = count_users_sqlite(pool).await?;

    Ok((users, total))
}

// ============================================================================
// MySQL implementations
// ============================================================================

async fn create_user_mysql(pool: &MySqlPool, user: &User) -> Result<User> {
    let now = Utc::now();
    let role_str = user.role.to_string();
    let status_str = user.status.to_string();

    let result = sqlx::query(
        r#"
        INSERT INTO users (username, email, password_hash, role, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&user.username)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&role_str)
    .bind(&status_str)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create user")?;

    let id = result.last_insert_id() as i64;

    Ok(User {
        id,
        username: user.username.clone(),
        email: user.email.clone(),
        password_hash: user.password_hash.clone(),
        role: user.role,
        status: user.status,
        created_at: now,
        updated_at: now,
    })
}

async fn get_user_by_id_mysql(pool: &MySqlPool, id: i64) -> Result<Option<User>> {
    let row = sqlx::query(
        r#"
        SELECT id, username, email, password_hash, role, status, created_at, updated_at
        FROM users
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get user by ID")?;

    match row {
        Some(row) => Ok(Some(row_to_user_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn get_user_by_username_mysql(pool: &MySqlPool, username: &str) -> Result<Option<User>> {
    let row = sqlx::query(
        r#"
        SELECT id, username, email, password_hash, role, status, created_at, updated_at
        FROM users
        WHERE username = ?
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .context("Failed to get user by username")?;

    match row {
        Some(row) => Ok(Some(row_to_user_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn get_user_by_email_mysql(pool: &MySqlPool, email: &str) -> Result<Option<User>> {
    let row = sqlx::query(
        r#"
        SELECT id, username, email, password_hash, role, status, created_at, updated_at
        FROM users
        WHERE email = ?
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .context("Failed to get user by email")?;

    match row {
        Some(row) => Ok(Some(row_to_user_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn update_user_mysql(pool: &MySqlPool, user: &User) -> Result<User> {
    let now = Utc::now();
    let role_str = user.role.to_string();
    let status_str = user.status.to_string();

    sqlx::query(
        r#"
        UPDATE users
        SET username = ?, email = ?, password_hash = ?, role = ?, status = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&user.username)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&role_str)
    .bind(&status_str)
    .bind(now)
    .bind(user.id)
    .execute(pool)
    .await
    .context("Failed to update user")?;

    // Return the updated user
    get_user_by_id_mysql(pool, user.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User not found after update"))
}

async fn delete_user_mysql(pool: &MySqlPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete user")?;

    Ok(())
}

async fn count_users_mysql(pool: &MySqlPool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM users")
        .fetch_one(pool)
        .await
        .context("Failed to count users")?;

    Ok(row.get("count"))
}

fn row_to_user_mysql(row: &sqlx::mysql::MySqlRow) -> Result<User> {
    let role_str: String = row.get("role");
    let role = UserRole::from_str(&role_str)
        .with_context(|| format!("Invalid role in database: {}", role_str))?;

    let status_str: Option<String> = row.try_get("status").ok();
    let status = status_str
        .map(|s| UserStatus::from_str(&s).unwrap_or(UserStatus::Active))
        .unwrap_or(UserStatus::Active);

    Ok(User {
        id: row.get("id"),
        username: row.get("username"),
        email: row.get("email"),
        password_hash: row.get("password_hash"),
        role,
        status,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

async fn list_users_mysql(pool: &MySqlPool, page: i64, per_page: i64) -> Result<(Vec<User>, i64)> {
    let offset = (page - 1) * per_page;
    
    let rows = sqlx::query(
        r#"
        SELECT id, username, email, password_hash, role, status, created_at, updated_at
        FROM users
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list users")?;

    let mut users = Vec::new();
    for row in rows {
        users.push(row_to_user_mysql(&row)?);
    }

    let total = count_users_mysql(pool).await?;

    Ok((users, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_test_pool, migrations};
    use crate::services::password::hash_password;

    async fn setup_test_repo() -> (DynDatabasePool, SqlxUserRepository) {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");
        let repo = SqlxUserRepository::new(pool.clone());
        (pool, repo)
    }

    fn create_test_user(username: &str, email: &str) -> User {
        User::new(
            username.to_string(),
            email.to_string(),
            hash_password("test_password").expect("Failed to hash password"),
            UserRole::Author,
        )
    }

    #[tokio::test]
    async fn test_create_user() {
        let (_pool, repo) = setup_test_repo().await;
        let user = create_test_user("testuser", "test@example.com");

        let created = repo.create(&user).await.expect("Failed to create user");

        assert!(created.id > 0);
        assert_eq!(created.username, "testuser");
        assert_eq!(created.email, "test@example.com");
        assert_eq!(created.role, UserRole::Author);
    }

    #[tokio::test]
    async fn test_get_user_by_id() {
        let (_pool, repo) = setup_test_repo().await;
        let user = create_test_user("testuser", "test@example.com");
        let created = repo.create(&user).await.expect("Failed to create user");

        let found = repo
            .get_by_id(created.id)
            .await
            .expect("Failed to get user")
            .expect("User not found");

        assert_eq!(found.id, created.id);
        assert_eq!(found.username, "testuser");
    }

    #[tokio::test]
    async fn test_get_user_by_id_not_found() {
        let (_pool, repo) = setup_test_repo().await;

        let found = repo.get_by_id(999).await.expect("Failed to get user");

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_get_user_by_username() {
        let (_pool, repo) = setup_test_repo().await;
        let user = create_test_user("findme", "findme@example.com");
        repo.create(&user).await.expect("Failed to create user");

        let found = repo
            .get_by_username("findme")
            .await
            .expect("Failed to get user")
            .expect("User not found");

        assert_eq!(found.username, "findme");
    }

    #[tokio::test]
    async fn test_get_user_by_username_not_found() {
        let (_pool, repo) = setup_test_repo().await;

        let found = repo
            .get_by_username("nonexistent")
            .await
            .expect("Failed to get user");

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_get_user_by_email() {
        let (_pool, repo) = setup_test_repo().await;
        let user = create_test_user("emailuser", "unique@example.com");
        repo.create(&user).await.expect("Failed to create user");

        let found = repo
            .get_by_email("unique@example.com")
            .await
            .expect("Failed to get user")
            .expect("User not found");

        assert_eq!(found.email, "unique@example.com");
    }

    #[tokio::test]
    async fn test_get_user_by_email_not_found() {
        let (_pool, repo) = setup_test_repo().await;

        let found = repo
            .get_by_email("nonexistent@example.com")
            .await
            .expect("Failed to get user");

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_update_user() {
        let (_pool, repo) = setup_test_repo().await;
        let user = create_test_user("updateme", "update@example.com");
        let mut created = repo.create(&user).await.expect("Failed to create user");

        created.username = "updated_username".to_string();
        created.role = UserRole::Editor;

        let updated = repo.update(&created).await.expect("Failed to update user");

        assert_eq!(updated.username, "updated_username");
        assert_eq!(updated.role, UserRole::Editor);
        assert!(updated.updated_at >= created.created_at);
    }

    #[tokio::test]
    async fn test_delete_user() {
        let (_pool, repo) = setup_test_repo().await;
        let user = create_test_user("deleteme", "delete@example.com");
        let created = repo.create(&user).await.expect("Failed to create user");

        repo.delete(created.id).await.expect("Failed to delete user");

        let found = repo.get_by_id(created.id).await.expect("Failed to get user");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_count_users() {
        let (_pool, repo) = setup_test_repo().await;

        // Initially no users
        let count = repo.count().await.expect("Failed to count users");
        assert_eq!(count, 0);

        // Create some users
        repo.create(&create_test_user("user1", "user1@example.com"))
            .await
            .expect("Failed to create user");
        repo.create(&create_test_user("user2", "user2@example.com"))
            .await
            .expect("Failed to create user");
        repo.create(&create_test_user("user3", "user3@example.com"))
            .await
            .expect("Failed to create user");

        let count = repo.count().await.expect("Failed to count users");
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_unique_username_constraint() {
        let (_pool, repo) = setup_test_repo().await;
        let user1 = create_test_user("duplicate", "user1@example.com");
        let user2 = create_test_user("duplicate", "user2@example.com");

        repo.create(&user1).await.expect("Failed to create first user");
        let result = repo.create(&user2).await;

        assert!(result.is_err(), "Should fail due to duplicate username");
    }

    #[tokio::test]
    async fn test_unique_email_constraint() {
        let (_pool, repo) = setup_test_repo().await;
        let user1 = create_test_user("user1", "duplicate@example.com");
        let user2 = create_test_user("user2", "duplicate@example.com");

        repo.create(&user1).await.expect("Failed to create first user");
        let result = repo.create(&user2).await;

        assert!(result.is_err(), "Should fail due to duplicate email");
    }

    #[tokio::test]
    async fn test_create_user_with_admin_role() {
        let (_pool, repo) = setup_test_repo().await;
        let user = User::new(
            "admin".to_string(),
            "admin@example.com".to_string(),
            hash_password("admin_password").expect("Failed to hash password"),
            UserRole::Admin,
        );

        let created = repo.create(&user).await.expect("Failed to create admin user");

        assert_eq!(created.role, UserRole::Admin);
    }

    #[tokio::test]
    async fn test_password_hash_stored_correctly() {
        let (_pool, repo) = setup_test_repo().await;
        let password = "my_secure_password";
        let hash = hash_password(password).expect("Failed to hash password");
        let user = User::new(
            "hashtest".to_string(),
            "hashtest@example.com".to_string(),
            hash.clone(),
            UserRole::Author,
        );

        let created = repo.create(&user).await.expect("Failed to create user");
        let found = repo
            .get_by_id(created.id)
            .await
            .expect("Failed to get user")
            .expect("User not found");

        // Verify the hash is stored correctly
        assert_eq!(found.password_hash, hash);
        assert!(found.password_hash.starts_with("$argon2id$"));
    }
}
