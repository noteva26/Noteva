//! Session repository
//!
//! Database operations for user sessions.
//!
//! This module provides:
//! - `SessionRepository` trait defining the interface for session data access
//! - `SqlxSessionRepository` implementing the trait for SQLite and MySQL
//!
//! Satisfies requirements:
//! - 4.3: WHEN 用户登录 THEN User_Service SHALL 验证凭据并返回会话令牌
//! - 4.4: WHEN 会话令牌过期 THEN User_Service SHALL 要求用户重新登录
//! - 4.7: WHILE 用户已登录 THEN User_Service SHALL 维护用户会话状态

use crate::config::DatabaseDriver;
use crate::db::DynDatabasePool;
use crate::models::Session;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{MySqlPool, Row, SqlitePool};
use std::sync::Arc;

/// Session repository trait
#[async_trait]
pub trait SessionRepository: Send + Sync {
    /// Create a new session
    async fn create(&self, session: &Session) -> Result<Session>;

    /// Get session by ID (token)
    async fn get_by_id(&self, id: &str) -> Result<Option<Session>>;

    /// Delete a session
    async fn delete(&self, id: &str) -> Result<()>;

    /// Delete all sessions for a user
    async fn delete_by_user(&self, user_id: i64) -> Result<()>;

    /// Delete expired sessions
    async fn delete_expired(&self) -> Result<i64>;
}

/// SQLx-based session repository implementation
///
/// Supports both SQLite and MySQL databases.
pub struct SqlxSessionRepository {
    pool: DynDatabasePool,
}

impl SqlxSessionRepository {
    /// Create a new SQLx session repository
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }

    /// Create a boxed repository for use with dependency injection
    pub fn boxed(pool: DynDatabasePool) -> Arc<dyn SessionRepository> {
        Arc::new(Self::new(pool))
    }
}

#[async_trait]
impl SessionRepository for SqlxSessionRepository {
    async fn create(&self, session: &Session) -> Result<Session> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                create_session_sqlite(self.pool.as_sqlite().unwrap(), session).await
            }
            DatabaseDriver::Mysql => {
                create_session_mysql(self.pool.as_mysql().unwrap(), session).await
            }
        }
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<Session>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                get_session_by_id_sqlite(self.pool.as_sqlite().unwrap(), id).await
            }
            DatabaseDriver::Mysql => {
                get_session_by_id_mysql(self.pool.as_mysql().unwrap(), id).await
            }
        }
    }

    async fn delete(&self, id: &str) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                delete_session_sqlite(self.pool.as_sqlite().unwrap(), id).await
            }
            DatabaseDriver::Mysql => delete_session_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
    }

    async fn delete_by_user(&self, user_id: i64) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                delete_sessions_by_user_sqlite(self.pool.as_sqlite().unwrap(), user_id).await
            }
            DatabaseDriver::Mysql => {
                delete_sessions_by_user_mysql(self.pool.as_mysql().unwrap(), user_id).await
            }
        }
    }

    async fn delete_expired(&self) -> Result<i64> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => {
                delete_expired_sessions_sqlite(self.pool.as_sqlite().unwrap()).await
            }
            DatabaseDriver::Mysql => {
                delete_expired_sessions_mysql(self.pool.as_mysql().unwrap()).await
            }
        }
    }
}

// ============================================================================
// SQLite implementations
// ============================================================================

async fn create_session_sqlite(pool: &SqlitePool, session: &Session) -> Result<Session> {
    sqlx::query(
        r#"
        INSERT INTO sessions (id, user_id, expires_at, created_at)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&session.id)
    .bind(session.user_id)
    .bind(session.expires_at)
    .bind(session.created_at)
    .execute(pool)
    .await
    .context("Failed to create session")?;

    Ok(session.clone())
}

async fn get_session_by_id_sqlite(pool: &SqlitePool, id: &str) -> Result<Option<Session>> {
    let row = sqlx::query(
        r#"
        SELECT id, user_id, expires_at, created_at
        FROM sessions
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get session by ID")?;

    match row {
        Some(row) => Ok(Some(row_to_session_sqlite(&row)?)),
        None => Ok(None),
    }
}

async fn delete_session_sqlite(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete session")?;

    Ok(())
}

async fn delete_sessions_by_user_sqlite(pool: &SqlitePool, user_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .context("Failed to delete sessions by user")?;

    Ok(())
}

async fn delete_expired_sessions_sqlite(pool: &SqlitePool) -> Result<i64> {
    let now = Utc::now();
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
        .bind(now)
        .execute(pool)
        .await
        .context("Failed to delete expired sessions")?;

    Ok(result.rows_affected() as i64)
}

fn row_to_session_sqlite(row: &sqlx::sqlite::SqliteRow) -> Result<Session> {
    Ok(Session {
        id: row.get("id"),
        user_id: row.get("user_id"),
        expires_at: row.get("expires_at"),
        created_at: row.get("created_at"),
    })
}

// ============================================================================
// MySQL implementations
// ============================================================================

async fn create_session_mysql(pool: &MySqlPool, session: &Session) -> Result<Session> {
    sqlx::query(
        r#"
        INSERT INTO sessions (id, user_id, expires_at, created_at)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&session.id)
    .bind(session.user_id)
    .bind(session.expires_at)
    .bind(session.created_at)
    .execute(pool)
    .await
    .context("Failed to create session")?;

    Ok(session.clone())
}

async fn get_session_by_id_mysql(pool: &MySqlPool, id: &str) -> Result<Option<Session>> {
    let row = sqlx::query(
        r#"
        SELECT id, user_id, expires_at, created_at
        FROM sessions
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get session by ID")?;

    match row {
        Some(row) => Ok(Some(row_to_session_mysql(&row)?)),
        None => Ok(None),
    }
}

async fn delete_session_mysql(pool: &MySqlPool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete session")?;

    Ok(())
}

async fn delete_sessions_by_user_mysql(pool: &MySqlPool, user_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .context("Failed to delete sessions by user")?;

    Ok(())
}

async fn delete_expired_sessions_mysql(pool: &MySqlPool) -> Result<i64> {
    let now = Utc::now();
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
        .bind(now)
        .execute(pool)
        .await
        .context("Failed to delete expired sessions")?;

    Ok(result.rows_affected() as i64)
}

fn row_to_session_mysql(row: &sqlx::mysql::MySqlRow) -> Result<Session> {
    let expires_at: DateTime<Utc> = row.get("expires_at");
    let created_at: DateTime<Utc> = row.get("created_at");

    Ok(Session {
        id: row.get("id"),
        user_id: row.get("user_id"),
        expires_at,
        created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_test_pool, migrations};
    use chrono::Duration;
    use uuid::Uuid;

    async fn setup_test_repo() -> (DynDatabasePool, SqlxSessionRepository) {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");
        let repo = SqlxSessionRepository::new(pool.clone());
        (pool, repo)
    }

    fn create_test_session(user_id: i64, expires_in_days: i64) -> Session {
        let now = Utc::now();
        Session {
            id: Uuid::new_v4().to_string(),
            user_id,
            expires_at: now + Duration::days(expires_in_days),
            created_at: now,
        }
    }

    // Helper to create a test user for foreign key constraint
    async fn create_test_user(pool: &DynDatabasePool, id: i64) {
        let now = Utc::now();
        if let Some(sqlite_pool) = pool.as_sqlite() {
            sqlx::query(
                r#"
                INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(id)
            .bind(format!("user{}", id))
            .bind(format!("user{}@example.com", id))
            .bind("hash")
            .bind("author")
            .bind(now)
            .bind(now)
            .execute(sqlite_pool)
            .await
            .expect("Failed to create test user");
        }
    }

    #[tokio::test]
    async fn test_create_session() {
        let (pool, repo) = setup_test_repo().await;
        create_test_user(&pool, 1).await;

        let session = create_test_session(1, 7);
        let created = repo.create(&session).await.expect("Failed to create session");

        assert_eq!(created.id, session.id);
        assert_eq!(created.user_id, 1);
    }

    #[tokio::test]
    async fn test_get_session_by_id() {
        let (pool, repo) = setup_test_repo().await;
        create_test_user(&pool, 1).await;

        let session = create_test_session(1, 7);
        repo.create(&session).await.expect("Failed to create session");

        let found = repo
            .get_by_id(&session.id)
            .await
            .expect("Failed to get session")
            .expect("Session not found");

        assert_eq!(found.id, session.id);
        assert_eq!(found.user_id, 1);
    }

    #[tokio::test]
    async fn test_get_session_by_id_not_found() {
        let (_pool, repo) = setup_test_repo().await;

        let found = repo
            .get_by_id("nonexistent-session-id")
            .await
            .expect("Failed to get session");

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_session() {
        let (pool, repo) = setup_test_repo().await;
        create_test_user(&pool, 1).await;

        let session = create_test_session(1, 7);
        repo.create(&session).await.expect("Failed to create session");

        repo.delete(&session.id)
            .await
            .expect("Failed to delete session");

        let found = repo
            .get_by_id(&session.id)
            .await
            .expect("Failed to get session");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_sessions_by_user() {
        let (pool, repo) = setup_test_repo().await;
        create_test_user(&pool, 1).await;
        create_test_user(&pool, 2).await;

        // Create multiple sessions for user 1
        let session1 = create_test_session(1, 7);
        let session2 = create_test_session(1, 7);
        let session3 = create_test_session(2, 7); // Different user

        repo.create(&session1).await.expect("Failed to create session");
        repo.create(&session2).await.expect("Failed to create session");
        repo.create(&session3).await.expect("Failed to create session");

        // Delete all sessions for user 1
        repo.delete_by_user(1)
            .await
            .expect("Failed to delete sessions by user");

        // User 1's sessions should be gone
        assert!(repo.get_by_id(&session1.id).await.unwrap().is_none());
        assert!(repo.get_by_id(&session2.id).await.unwrap().is_none());

        // User 2's session should still exist
        assert!(repo.get_by_id(&session3.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_delete_expired_sessions() {
        let (pool, repo) = setup_test_repo().await;
        create_test_user(&pool, 1).await;

        // Create an expired session (negative days = past)
        let now = Utc::now();
        let expired_session = Session {
            id: Uuid::new_v4().to_string(),
            user_id: 1,
            expires_at: now - Duration::days(1), // Expired yesterday
            created_at: now - Duration::days(8),
        };

        // Create a valid session
        let valid_session = create_test_session(1, 7);

        repo.create(&expired_session)
            .await
            .expect("Failed to create expired session");
        repo.create(&valid_session)
            .await
            .expect("Failed to create valid session");

        // Delete expired sessions
        let deleted_count = repo
            .delete_expired()
            .await
            .expect("Failed to delete expired sessions");

        assert_eq!(deleted_count, 1);

        // Expired session should be gone
        assert!(repo.get_by_id(&expired_session.id).await.unwrap().is_none());

        // Valid session should still exist
        assert!(repo.get_by_id(&valid_session.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_session_expiration_check() {
        let now = Utc::now();

        let expired_session = Session {
            id: "expired".to_string(),
            user_id: 1,
            expires_at: now - Duration::hours(1),
            created_at: now - Duration::days(8),
        };

        let valid_session = Session {
            id: "valid".to_string(),
            user_id: 1,
            expires_at: now + Duration::hours(1),
            created_at: now,
        };

        assert!(expired_session.is_expired());
        assert!(!valid_session.is_expired());
    }
}
