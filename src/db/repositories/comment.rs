//! Comment repository

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool, MySqlPool};

use crate::config::DatabaseDriver;
use crate::db::DynDatabasePool;
use crate::models::{Comment, CommentStatus, CommentWithMeta, CreateCommentInput, Like, LikeTargetType};

/// Comment repository trait
#[async_trait]
pub trait CommentRepository: Send + Sync {
    /// Create a new comment
    async fn create(&self, input: CreateCommentInput, user_id: Option<i64>, ip: Option<String>, ua: Option<String>) -> Result<Comment>;
    
    /// Create a new comment with specific status
    async fn create_with_status(&self, input: CreateCommentInput, user_id: Option<i64>, ip: Option<String>, ua: Option<String>, status: CommentStatus) -> Result<Comment>;
    
    /// Get a comment by ID
    async fn get_by_id(&self, id: i64) -> Result<Option<Comment>>;
    
    /// Get comments for an article
    async fn get_by_article(&self, article_id: i64, fingerprint: Option<&str>) -> Result<Vec<CommentWithMeta>>;
    
    /// Get pending comments for moderation
    async fn list_pending(&self, page: i64, per_page: i64) -> Result<(Vec<CommentWithMeta>, i64)>;
    
    /// Delete a comment
    async fn delete(&self, id: i64) -> Result<bool>;
    
    /// Update comment status
    async fn update_status(&self, id: i64, status: CommentStatus) -> Result<bool>;
    
    /// Add a like
    async fn add_like(&self, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<String>) -> Result<bool>;
    
    /// Remove a like
    async fn remove_like(&self, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<String>) -> Result<bool>;
    
    /// Check if liked
    async fn is_liked(&self, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<&str>) -> Result<bool>;
    
    /// Increment article view count
    async fn increment_view(&self, article_id: i64) -> Result<()>;
}

/// Comment repository implementation
pub struct CommentRepositoryImpl {
    pool: DynDatabasePool,
}

impl CommentRepositoryImpl {
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CommentRepository for CommentRepositoryImpl {
    async fn create(&self, input: CreateCommentInput, user_id: Option<i64>, ip: Option<String>, ua: Option<String>) -> Result<Comment> {
        self.create_with_status(input, user_id, ip, ua, CommentStatus::Approved).await
    }
    
    async fn create_with_status(&self, input: CreateCommentInput, user_id: Option<i64>, ip: Option<String>, ua: Option<String>, status: CommentStatus) -> Result<Comment> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => create_sqlite(self.pool.as_sqlite().unwrap(), input, user_id, ip, ua, status).await,
            DatabaseDriver::Mysql => create_mysql(self.pool.as_mysql().unwrap(), input, user_id, ip, ua, status).await,
        }
    }
    
    async fn get_by_id(&self, id: i64) -> Result<Option<Comment>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => get_by_id_sqlite(self.pool.as_sqlite().unwrap(), id).await,
            DatabaseDriver::Mysql => get_by_id_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
    }
    
    async fn get_by_article(&self, article_id: i64, fingerprint: Option<&str>) -> Result<Vec<CommentWithMeta>> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => get_by_article_sqlite(self.pool.as_sqlite().unwrap(), article_id, fingerprint).await,
            DatabaseDriver::Mysql => get_by_article_mysql(self.pool.as_mysql().unwrap(), article_id, fingerprint).await,
        }
    }
    
    async fn list_pending(&self, page: i64, per_page: i64) -> Result<(Vec<CommentWithMeta>, i64)> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => list_pending_sqlite(self.pool.as_sqlite().unwrap(), page, per_page).await,
            DatabaseDriver::Mysql => list_pending_mysql(self.pool.as_mysql().unwrap(), page, per_page).await,
        }
    }
    
    async fn delete(&self, id: i64) -> Result<bool> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => delete_sqlite(self.pool.as_sqlite().unwrap(), id).await,
            DatabaseDriver::Mysql => delete_mysql(self.pool.as_mysql().unwrap(), id).await,
        }
    }
    
    async fn update_status(&self, id: i64, status: CommentStatus) -> Result<bool> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => update_status_sqlite(self.pool.as_sqlite().unwrap(), id, status).await,
            DatabaseDriver::Mysql => update_status_mysql(self.pool.as_mysql().unwrap(), id, status).await,
        }
    }
    
    async fn add_like(&self, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<String>) -> Result<bool> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => add_like_sqlite(self.pool.as_sqlite().unwrap(), target_type, target_id, user_id, fingerprint).await,
            DatabaseDriver::Mysql => add_like_mysql(self.pool.as_mysql().unwrap(), target_type, target_id, user_id, fingerprint).await,
        }
    }
    
    async fn remove_like(&self, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<String>) -> Result<bool> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => remove_like_sqlite(self.pool.as_sqlite().unwrap(), target_type, target_id, user_id, fingerprint).await,
            DatabaseDriver::Mysql => remove_like_mysql(self.pool.as_mysql().unwrap(), target_type, target_id, user_id, fingerprint).await,
        }
    }
    
    async fn is_liked(&self, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<&str>) -> Result<bool> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => is_liked_sqlite(self.pool.as_sqlite().unwrap(), target_type, target_id, user_id, fingerprint).await,
            DatabaseDriver::Mysql => is_liked_mysql(self.pool.as_mysql().unwrap(), target_type, target_id, user_id, fingerprint).await,
        }
    }
    
    async fn increment_view(&self, article_id: i64) -> Result<()> {
        match self.pool.driver() {
            DatabaseDriver::Sqlite => increment_view_sqlite(self.pool.as_sqlite().unwrap(), article_id).await,
            DatabaseDriver::Mysql => increment_view_mysql(self.pool.as_mysql().unwrap(), article_id).await,
        }
    }
}

// SQLite implementations
async fn create_sqlite(pool: &SqlitePool, input: CreateCommentInput, user_id: Option<i64>, ip: Option<String>, ua: Option<String>, status: CommentStatus) -> Result<Comment> {
    let now = Utc::now();
    let result = sqlx::query(
        r#"INSERT INTO comments (article_id, user_id, parent_id, nickname, email, content, status, ip_address, user_agent, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(input.article_id)
    .bind(user_id)
    .bind(input.parent_id)
    .bind(&input.nickname)
    .bind(&input.email)
    .bind(&input.content)
    .bind(status.to_string())
    .bind(&ip)
    .bind(&ua)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    
    let id = result.last_insert_rowid();
    
    // Only update article comment count if approved
    if status == CommentStatus::Approved {
        sqlx::query("UPDATE articles SET comment_count = comment_count + 1 WHERE id = ?")
            .bind(input.article_id)
            .execute(pool)
            .await?;
    }
    
    Ok(Comment {
        id,
        article_id: input.article_id,
        user_id,
        parent_id: input.parent_id,
        nickname: input.nickname,
        email: input.email,
        content: input.content,
        status,
        ip_address: ip,
        user_agent: ua,
        created_at: now,
        updated_at: now,
    })
}

async fn get_by_id_sqlite(pool: &SqlitePool, id: i64) -> Result<Option<Comment>> {
    let row = sqlx::query(
        "SELECT * FROM comments WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    
    Ok(row.map(|r| Comment {
        id: r.get("id"),
        article_id: r.get("article_id"),
        user_id: r.get("user_id"),
        parent_id: r.get("parent_id"),
        nickname: r.get("nickname"),
        email: r.get("email"),
        content: r.get("content"),
        status: r.get::<String, _>("status").parse().unwrap_or_default(),
        ip_address: r.get("ip_address"),
        user_agent: r.get("user_agent"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }))
}

async fn get_by_article_sqlite(pool: &SqlitePool, article_id: i64, fingerprint: Option<&str>) -> Result<Vec<CommentWithMeta>> {
    let rows = sqlx::query(
        r#"SELECT c.*, u.username,
           (SELECT COUNT(*) FROM likes WHERE target_type = 'comment' AND target_id = c.id) as like_count
           FROM comments c 
           LEFT JOIN users u ON c.user_id = u.id
           WHERE c.article_id = ? AND c.status = 'approved'
           ORDER BY c.created_at ASC"#
    )
    .bind(article_id)
    .fetch_all(pool)
    .await?;
    
    let mut comments: Vec<CommentWithMeta> = Vec::new();
    let mut replies_map: std::collections::HashMap<i64, Vec<CommentWithMeta>> = std::collections::HashMap::new();
    
    for row in rows {
        let id: i64 = row.get("id");
        let parent_id: Option<i64> = row.get("parent_id");
        let email: Option<String> = row.get("email");
        let like_count: i64 = row.get("like_count");
        let nickname: Option<String> = row.get("nickname");
        let username: Option<String> = row.try_get("username").ok();
        
        // Use username if logged in user, otherwise use nickname
        let display_name = username.or(nickname);
        
        let is_liked = if let Some(fp) = fingerprint {
            is_liked_sqlite(pool, LikeTargetType::Comment, id, None, Some(fp)).await.unwrap_or(false)
        } else {
            false
        };
        
        let comment = CommentWithMeta {
            id,
            article_id: row.get("article_id"),
            user_id: row.get("user_id"),
            parent_id,
            nickname: display_name,
            email: email.clone(),
            content: row.get("content"),
            status: row.get::<String, _>("status").parse().unwrap_or_default(),
            created_at: row.get("created_at"),
            avatar_url: CommentWithMeta::gravatar_url(&email),
            like_count,
            is_liked,
            replies: Vec::new(),
        };
        
        if let Some(pid) = parent_id {
            replies_map.entry(pid).or_default().push(comment);
        } else {
            comments.push(comment);
        }
    }
    
    // Attach replies to parent comments
    for comment in &mut comments {
        if let Some(replies) = replies_map.remove(&comment.id) {
            comment.replies = replies;
        }
    }
    
    Ok(comments)
}

async fn list_pending_sqlite(pool: &SqlitePool, page: i64, per_page: i64) -> Result<(Vec<CommentWithMeta>, i64)> {
    let offset = (page - 1) * per_page;
    
    // Get total count
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM comments WHERE status = 'pending'"
    )
    .fetch_one(pool)
    .await?;
    
    let rows = sqlx::query(
        r#"SELECT c.*, u.username, a.title as article_title
           FROM comments c 
           LEFT JOIN users u ON c.user_id = u.id
           LEFT JOIN articles a ON c.article_id = a.id
           WHERE c.status = 'pending'
           ORDER BY c.created_at DESC
           LIMIT ? OFFSET ?"#
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    
    let comments: Vec<CommentWithMeta> = rows.iter().map(|row| {
        let email: Option<String> = row.get("email");
        let nickname: Option<String> = row.get("nickname");
        let username: Option<String> = row.try_get("username").ok();
        let display_name = username.or(nickname);
        
        CommentWithMeta {
            id: row.get("id"),
            article_id: row.get("article_id"),
            user_id: row.get("user_id"),
            parent_id: row.get("parent_id"),
            nickname: display_name,
            email: email.clone(),
            content: row.get("content"),
            status: row.get::<String, _>("status").parse().unwrap_or_default(),
            created_at: row.get("created_at"),
            avatar_url: CommentWithMeta::gravatar_url(&email),
            like_count: 0,
            is_liked: false,
            replies: Vec::new(),
        }
    }).collect();
    
    Ok((comments, total))
}

async fn delete_sqlite(pool: &SqlitePool, id: i64) -> Result<bool> {
    // Get article_id first
    let row = sqlx::query("SELECT article_id FROM comments WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    
    if let Some(row) = row {
        let article_id: i64 = row.get("article_id");
        
        let result = sqlx::query("DELETE FROM comments WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        
        if result.rows_affected() > 0 {
            // Update article comment count
            sqlx::query("UPDATE articles SET comment_count = MAX(0, comment_count - 1) WHERE id = ?")
                .bind(article_id)
                .execute(pool)
                .await?;
            return Ok(true);
        }
    }
    
    Ok(false)
}

async fn update_status_sqlite(pool: &SqlitePool, id: i64, status: CommentStatus) -> Result<bool> {
    // Get current status and article_id
    let row = sqlx::query("SELECT article_id, status FROM comments WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    
    if let Some(row) = row {
        let article_id: i64 = row.get("article_id");
        let old_status: String = row.get("status");
        let old_status: CommentStatus = old_status.parse().unwrap_or_default();
        
        let result = sqlx::query("UPDATE comments SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status.to_string())
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await?;
        
        if result.rows_affected() > 0 {
            // Update article comment count based on status change
            if old_status != CommentStatus::Approved && status == CommentStatus::Approved {
                // Approving a pending/rejected comment
                sqlx::query("UPDATE articles SET comment_count = comment_count + 1 WHERE id = ?")
                    .bind(article_id)
                    .execute(pool)
                    .await?;
            } else if old_status == CommentStatus::Approved && status != CommentStatus::Approved {
                // Rejecting an approved comment
                sqlx::query("UPDATE articles SET comment_count = MAX(0, comment_count - 1) WHERE id = ?")
                    .bind(article_id)
                    .execute(pool)
                    .await?;
            }
            return Ok(true);
        }
    }
    
    Ok(false)
}

async fn add_like_sqlite(pool: &SqlitePool, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<String>) -> Result<bool> {
    let result = if let Some(uid) = user_id {
        sqlx::query("INSERT OR IGNORE INTO likes (target_type, target_id, user_id) VALUES (?, ?, ?)")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(uid)
            .execute(pool)
            .await?
    } else if let Some(fp) = fingerprint {
        sqlx::query("INSERT OR IGNORE INTO likes (target_type, target_id, fingerprint) VALUES (?, ?, ?)")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(fp)
            .execute(pool)
            .await?
    } else {
        return Ok(false);
    };
    
    if result.rows_affected() > 0 {
        // Update like count
        match target_type {
            LikeTargetType::Article => {
                sqlx::query("UPDATE articles SET like_count = like_count + 1 WHERE id = ?")
                    .bind(target_id)
                    .execute(pool)
                    .await?;
            }
            LikeTargetType::Comment => {}
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn remove_like_sqlite(pool: &SqlitePool, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<String>) -> Result<bool> {
    let result = if let Some(uid) = user_id {
        sqlx::query("DELETE FROM likes WHERE target_type = ? AND target_id = ? AND user_id = ?")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(uid)
            .execute(pool)
            .await?
    } else if let Some(fp) = fingerprint {
        sqlx::query("DELETE FROM likes WHERE target_type = ? AND target_id = ? AND fingerprint = ?")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(fp)
            .execute(pool)
            .await?
    } else {
        return Ok(false);
    };
    
    if result.rows_affected() > 0 {
        match target_type {
            LikeTargetType::Article => {
                sqlx::query("UPDATE articles SET like_count = MAX(0, like_count - 1) WHERE id = ?")
                    .bind(target_id)
                    .execute(pool)
                    .await?;
            }
            LikeTargetType::Comment => {}
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn is_liked_sqlite(pool: &SqlitePool, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<&str>) -> Result<bool> {
    let count: i64 = if let Some(uid) = user_id {
        sqlx::query_scalar("SELECT COUNT(*) FROM likes WHERE target_type = ? AND target_id = ? AND user_id = ?")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(uid)
            .fetch_one(pool)
            .await?
    } else if let Some(fp) = fingerprint {
        sqlx::query_scalar("SELECT COUNT(*) FROM likes WHERE target_type = ? AND target_id = ? AND fingerprint = ?")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(fp)
            .fetch_one(pool)
            .await?
    } else {
        return Ok(false);
    };
    Ok(count > 0)
}

async fn increment_view_sqlite(pool: &SqlitePool, article_id: i64) -> Result<()> {
    sqlx::query("UPDATE articles SET view_count = view_count + 1 WHERE id = ?")
        .bind(article_id)
        .execute(pool)
        .await?;
    Ok(())
}

// MySQL implementations (similar to SQLite)
async fn create_mysql(pool: &MySqlPool, input: CreateCommentInput, user_id: Option<i64>, ip: Option<String>, ua: Option<String>, status: CommentStatus) -> Result<Comment> {
    let now = Utc::now();
    let result = sqlx::query(
        r#"INSERT INTO comments (article_id, user_id, parent_id, nickname, email, content, status, ip_address, user_agent, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(input.article_id)
    .bind(user_id)
    .bind(input.parent_id)
    .bind(&input.nickname)
    .bind(&input.email)
    .bind(&input.content)
    .bind(status.to_string())
    .bind(&ip)
    .bind(&ua)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    
    let id = result.last_insert_id() as i64;
    
    // Only update article comment count if approved
    if status == CommentStatus::Approved {
        sqlx::query("UPDATE articles SET comment_count = comment_count + 1 WHERE id = ?")
            .bind(input.article_id)
            .execute(pool)
            .await?;
    }
    
    Ok(Comment {
        id,
        article_id: input.article_id,
        user_id,
        parent_id: input.parent_id,
        nickname: input.nickname,
        email: input.email,
        content: input.content,
        status,
        ip_address: ip,
        user_agent: ua,
        created_at: now,
        updated_at: now,
    })
}

async fn get_by_id_mysql(pool: &MySqlPool, id: i64) -> Result<Option<Comment>> {
    let row = sqlx::query(
        "SELECT * FROM comments WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    
    Ok(row.map(|r| Comment {
        id: r.get("id"),
        article_id: r.get("article_id"),
        user_id: r.get("user_id"),
        parent_id: r.get("parent_id"),
        nickname: r.get("nickname"),
        email: r.get("email"),
        content: r.get("content"),
        status: r.get::<String, _>("status").parse().unwrap_or_default(),
        ip_address: r.get("ip_address"),
        user_agent: r.get("user_agent"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }))
}

async fn get_by_article_mysql(pool: &MySqlPool, article_id: i64, fingerprint: Option<&str>) -> Result<Vec<CommentWithMeta>> {
    let rows = sqlx::query(
        r#"SELECT c.*, u.username,
           (SELECT COUNT(*) FROM likes WHERE target_type = 'comment' AND target_id = c.id) as like_count
           FROM comments c 
           LEFT JOIN users u ON c.user_id = u.id
           WHERE c.article_id = ? AND c.status = 'approved'
           ORDER BY c.created_at ASC"#
    )
    .bind(article_id)
    .fetch_all(pool)
    .await?;
    
    let mut comments: Vec<CommentWithMeta> = Vec::new();
    let mut replies_map: std::collections::HashMap<i64, Vec<CommentWithMeta>> = std::collections::HashMap::new();
    
    for row in rows {
        let id: i64 = row.get("id");
        let parent_id: Option<i64> = row.get("parent_id");
        let email: Option<String> = row.get("email");
        let like_count: i64 = row.get("like_count");
        let nickname: Option<String> = row.get("nickname");
        let username: Option<String> = row.try_get("username").ok();
        
        // Use username if logged in user, otherwise use nickname
        let display_name = username.or(nickname);
        
        let is_liked = if let Some(fp) = fingerprint {
            is_liked_mysql(pool, LikeTargetType::Comment, id, None, Some(fp)).await.unwrap_or(false)
        } else {
            false
        };
        
        let comment = CommentWithMeta {
            id,
            article_id: row.get("article_id"),
            user_id: row.get("user_id"),
            parent_id,
            nickname: display_name,
            email: email.clone(),
            content: row.get("content"),
            status: row.get::<String, _>("status").parse().unwrap_or_default(),
            created_at: row.get("created_at"),
            avatar_url: CommentWithMeta::gravatar_url(&email),
            like_count,
            is_liked,
            replies: Vec::new(),
        };
        
        if let Some(pid) = parent_id {
            replies_map.entry(pid).or_default().push(comment);
        } else {
            comments.push(comment);
        }
    }
    
    for comment in &mut comments {
        if let Some(replies) = replies_map.remove(&comment.id) {
            comment.replies = replies;
        }
    }
    
    Ok(comments)
}

async fn list_pending_mysql(pool: &MySqlPool, page: i64, per_page: i64) -> Result<(Vec<CommentWithMeta>, i64)> {
    let offset = (page - 1) * per_page;
    
    // Get total count
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM comments WHERE status = 'pending'"
    )
    .fetch_one(pool)
    .await?;
    
    let rows = sqlx::query(
        r#"SELECT c.*, u.username, a.title as article_title
           FROM comments c 
           LEFT JOIN users u ON c.user_id = u.id
           LEFT JOIN articles a ON c.article_id = a.id
           WHERE c.status = 'pending'
           ORDER BY c.created_at DESC
           LIMIT ? OFFSET ?"#
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    
    let comments: Vec<CommentWithMeta> = rows.iter().map(|row| {
        let email: Option<String> = row.get("email");
        let nickname: Option<String> = row.get("nickname");
        let username: Option<String> = row.try_get("username").ok();
        let display_name = username.or(nickname);
        
        CommentWithMeta {
            id: row.get("id"),
            article_id: row.get("article_id"),
            user_id: row.get("user_id"),
            parent_id: row.get("parent_id"),
            nickname: display_name,
            email: email.clone(),
            content: row.get("content"),
            status: row.get::<String, _>("status").parse().unwrap_or_default(),
            created_at: row.get("created_at"),
            avatar_url: CommentWithMeta::gravatar_url(&email),
            like_count: 0,
            is_liked: false,
            replies: Vec::new(),
        }
    }).collect();
    
    Ok((comments, total))
}

async fn delete_mysql(pool: &MySqlPool, id: i64) -> Result<bool> {
    let row = sqlx::query("SELECT article_id FROM comments WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    
    if let Some(row) = row {
        let article_id: i64 = row.get("article_id");
        
        let result = sqlx::query("DELETE FROM comments WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        
        if result.rows_affected() > 0 {
            sqlx::query("UPDATE articles SET comment_count = GREATEST(0, comment_count - 1) WHERE id = ?")
                .bind(article_id)
                .execute(pool)
                .await?;
            return Ok(true);
        }
    }
    
    Ok(false)
}

async fn update_status_mysql(pool: &MySqlPool, id: i64, status: CommentStatus) -> Result<bool> {
    // Get current status and article_id
    let row = sqlx::query("SELECT article_id, status FROM comments WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    
    if let Some(row) = row {
        let article_id: i64 = row.get("article_id");
        let old_status: String = row.get("status");
        let old_status: CommentStatus = old_status.parse().unwrap_or_default();
        
        let result = sqlx::query("UPDATE comments SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status.to_string())
            .bind(Utc::now())
            .bind(id)
            .execute(pool)
            .await?;
        
        if result.rows_affected() > 0 {
            // Update article comment count based on status change
            if old_status != CommentStatus::Approved && status == CommentStatus::Approved {
                // Approving a pending/rejected comment
                sqlx::query("UPDATE articles SET comment_count = comment_count + 1 WHERE id = ?")
                    .bind(article_id)
                    .execute(pool)
                    .await?;
            } else if old_status == CommentStatus::Approved && status != CommentStatus::Approved {
                // Rejecting an approved comment
                sqlx::query("UPDATE articles SET comment_count = GREATEST(0, comment_count - 1) WHERE id = ?")
                    .bind(article_id)
                    .execute(pool)
                    .await?;
            }
            return Ok(true);
        }
    }
    
    Ok(false)
}

async fn add_like_mysql(pool: &MySqlPool, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<String>) -> Result<bool> {
    let result = if let Some(uid) = user_id {
        sqlx::query("INSERT IGNORE INTO likes (target_type, target_id, user_id) VALUES (?, ?, ?)")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(uid)
            .execute(pool)
            .await?
    } else if let Some(fp) = fingerprint {
        sqlx::query("INSERT IGNORE INTO likes (target_type, target_id, fingerprint) VALUES (?, ?, ?)")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(fp)
            .execute(pool)
            .await?
    } else {
        return Ok(false);
    };
    
    if result.rows_affected() > 0 {
        match target_type {
            LikeTargetType::Article => {
                sqlx::query("UPDATE articles SET like_count = like_count + 1 WHERE id = ?")
                    .bind(target_id)
                    .execute(pool)
                    .await?;
            }
            LikeTargetType::Comment => {}
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn remove_like_mysql(pool: &MySqlPool, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<String>) -> Result<bool> {
    let result = if let Some(uid) = user_id {
        sqlx::query("DELETE FROM likes WHERE target_type = ? AND target_id = ? AND user_id = ?")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(uid)
            .execute(pool)
            .await?
    } else if let Some(fp) = fingerprint {
        sqlx::query("DELETE FROM likes WHERE target_type = ? AND target_id = ? AND fingerprint = ?")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(fp)
            .execute(pool)
            .await?
    } else {
        return Ok(false);
    };
    
    if result.rows_affected() > 0 {
        match target_type {
            LikeTargetType::Article => {
                sqlx::query("UPDATE articles SET like_count = GREATEST(0, like_count - 1) WHERE id = ?")
                    .bind(target_id)
                    .execute(pool)
                    .await?;
            }
            LikeTargetType::Comment => {}
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn is_liked_mysql(pool: &MySqlPool, target_type: LikeTargetType, target_id: i64, user_id: Option<i64>, fingerprint: Option<&str>) -> Result<bool> {
    let count: i64 = if let Some(uid) = user_id {
        sqlx::query_scalar("SELECT COUNT(*) FROM likes WHERE target_type = ? AND target_id = ? AND user_id = ?")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(uid)
            .fetch_one(pool)
            .await?
    } else if let Some(fp) = fingerprint {
        sqlx::query_scalar("SELECT COUNT(*) FROM likes WHERE target_type = ? AND target_id = ? AND fingerprint = ?")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(fp)
            .fetch_one(pool)
            .await?
    } else {
        return Ok(false);
    };
    Ok(count > 0)
}

async fn increment_view_mysql(pool: &MySqlPool, article_id: i64) -> Result<()> {
    sqlx::query("UPDATE articles SET view_count = view_count + 1 WHERE id = ?")
        .bind(article_id)
        .execute(pool)
        .await?;
    Ok(())
}
