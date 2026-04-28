//! Comment repository

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{MySqlPool, Row, SqlitePool};

use crate::db::DynDatabasePool;
use crate::models::{Comment, CommentStatus, CommentWithMeta, CreateCommentInput, LikeTargetType};

/// Comment repository trait
#[async_trait]
pub trait CommentRepository: Send + Sync {
    /// Create a new comment
    async fn create(
        &self,
        input: CreateCommentInput,
        user_id: Option<i64>,
        ip: Option<String>,
        ua: Option<String>,
    ) -> Result<Comment>;

    /// Create a new comment with specific status
    async fn create_with_status(
        &self,
        input: CreateCommentInput,
        user_id: Option<i64>,
        ip: Option<String>,
        ua: Option<String>,
        status: CommentStatus,
    ) -> Result<Comment>;

    /// Get a comment by ID
    async fn get_by_id(&self, id: i64) -> Result<Option<Comment>>;

    /// Get comments for an article
    async fn get_by_article(
        &self,
        article_id: i64,
        fingerprint: Option<&str>,
    ) -> Result<Vec<CommentWithMeta>>;

    /// Get pending comments for moderation
    async fn list_pending(&self, page: i64, per_page: i64) -> Result<(Vec<CommentWithMeta>, i64)>;

    /// List all comments with optional status filter (for admin management)
    async fn list_all(
        &self,
        status: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<CommentWithMeta>, i64)>;

    /// Delete a comment
    async fn delete(&self, id: i64) -> Result<bool>;

    /// Update comment status
    async fn update_status(&self, id: i64, status: CommentStatus) -> Result<bool>;

    /// Add a like
    async fn add_like(
        &self,
        target_type: LikeTargetType,
        target_id: i64,
        user_id: Option<i64>,
        fingerprint: Option<String>,
    ) -> Result<bool>;

    /// Remove a like
    async fn remove_like(
        &self,
        target_type: LikeTargetType,
        target_id: i64,
        user_id: Option<i64>,
        fingerprint: Option<String>,
    ) -> Result<bool>;

    /// Check if liked
    async fn is_liked(
        &self,
        target_type: LikeTargetType,
        target_id: i64,
        user_id: Option<i64>,
        fingerprint: Option<&str>,
    ) -> Result<bool>;

    /// Increment article view count
    async fn increment_view(&self, article_id: i64) -> Result<()>;

    /// Get recent approved comments across all articles
    async fn list_recent(&self, limit: i64) -> Result<Vec<CommentWithMeta>>;
}

/// Comment repository implementation
pub struct SqlxCommentRepository {
    pool: DynDatabasePool,
}

impl SqlxCommentRepository {
    pub fn new(pool: DynDatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CommentRepository for SqlxCommentRepository {
    async fn create(
        &self,
        input: CreateCommentInput,
        user_id: Option<i64>,
        ip: Option<String>,
        ua: Option<String>,
    ) -> Result<Comment> {
        self.create_with_status(input, user_id, ip, ua, CommentStatus::Approved)
            .await
    }

    async fn create_with_status(
        &self,
        input: CreateCommentInput,
        user_id: Option<i64>,
        ip: Option<String>,
        ua: Option<String>,
        status: CommentStatus,
    ) -> Result<Comment> {
        dispatch!(self, create, input, user_id, ip, ua, status)
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<Comment>> {
        dispatch!(self, get_by_id, id)
    }

    async fn get_by_article(
        &self,
        article_id: i64,
        fingerprint: Option<&str>,
    ) -> Result<Vec<CommentWithMeta>> {
        dispatch!(self, get_by_article, article_id, fingerprint)
    }

    async fn list_pending(&self, page: i64, per_page: i64) -> Result<(Vec<CommentWithMeta>, i64)> {
        dispatch!(self, list_pending, page, per_page)
    }

    async fn list_all(
        &self,
        status: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<CommentWithMeta>, i64)> {
        dispatch!(self, list_all, status, page, per_page)
    }

    async fn delete(&self, id: i64) -> Result<bool> {
        dispatch!(self, delete, id)
    }

    async fn update_status(&self, id: i64, status: CommentStatus) -> Result<bool> {
        dispatch!(self, update_status, id, status)
    }

    async fn add_like(
        &self,
        target_type: LikeTargetType,
        target_id: i64,
        user_id: Option<i64>,
        fingerprint: Option<String>,
    ) -> Result<bool> {
        dispatch!(self, add_like, target_type, target_id, user_id, fingerprint)
    }

    async fn remove_like(
        &self,
        target_type: LikeTargetType,
        target_id: i64,
        user_id: Option<i64>,
        fingerprint: Option<String>,
    ) -> Result<bool> {
        dispatch!(
            self,
            remove_like,
            target_type,
            target_id,
            user_id,
            fingerprint
        )
    }

    async fn is_liked(
        &self,
        target_type: LikeTargetType,
        target_id: i64,
        user_id: Option<i64>,
        fingerprint: Option<&str>,
    ) -> Result<bool> {
        dispatch!(self, is_liked, target_type, target_id, user_id, fingerprint)
    }

    async fn increment_view(&self, article_id: i64) -> Result<()> {
        dispatch!(self, increment_view, article_id)
    }

    async fn list_recent(&self, limit: i64) -> Result<Vec<CommentWithMeta>> {
        dispatch!(self, list_recent, limit)
    }
}

// SQLite implementations
async fn create_sqlite(
    pool: &SqlitePool,
    input: CreateCommentInput,
    user_id: Option<i64>,
    ip: Option<String>,
    ua: Option<String>,
    status: CommentStatus,
) -> Result<Comment> {
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
    let row = sqlx::query("SELECT * FROM comments WHERE id = ?")
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

async fn get_by_article_sqlite(
    pool: &SqlitePool,
    article_id: i64,
    fingerprint: Option<&str>,
) -> Result<Vec<CommentWithMeta>> {
    // First get the article author_id
    let author_id: Option<i64> = sqlx::query_scalar("SELECT author_id FROM articles WHERE id = ?")
        .bind(article_id)
        .fetch_optional(pool)
        .await?;

    let rows = sqlx::query(
        r#"SELECT c.*, u.username, u.role as user_role, u.avatar as user_avatar, u.display_name as user_display_name,
           (SELECT COUNT(*) FROM likes WHERE target_type = 'comment' AND target_id = c.id) as like_count
           FROM comments c 
           LEFT JOIN users u ON c.user_id = u.id
           WHERE c.article_id = ? AND c.status = 'approved'
           ORDER BY c.created_at ASC"#
    )
    .bind(article_id)
    .fetch_all(pool)
    .await?;

    let mut all_comments: Vec<CommentWithMeta> = Vec::new();
    let mut id_to_index: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();

    for row in rows {
        let id: i64 = row.get("id");
        let parent_id: Option<i64> = row.get("parent_id");
        let email: Option<String> = row.get("email");
        let like_count: i64 = row.get("like_count");
        let nickname: Option<String> = row.get("nickname");
        let username: Option<String> = row.try_get("username").ok();
        let user_id: Option<i64> = row.get("user_id");
        let user_role: Option<String> = row.try_get("user_role").ok();
        let user_avatar: Option<String> = row.try_get("user_avatar").ok().flatten();
        let user_display_name: Option<String> = row.try_get("user_display_name").ok().flatten();

        // Use user's display_name first, then username, then nickname
        let display_name = user_display_name.or(username).or(nickname);

        // Check if this comment is from the article author or an admin
        let is_author =
            user_id.is_some() && (user_id == author_id || user_role.as_deref() == Some("admin"));

        // Use user's avatar if set, otherwise use Gravatar
        let avatar_url = user_avatar
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| CommentWithMeta::gravatar_url(&email));
        let is_liked = if let Some(fp) = fingerprint {
            is_liked_sqlite(pool, LikeTargetType::Comment, id, None, Some(fp))
                .await
                .unwrap_or(false)
        } else {
            false
        };

        let comment = CommentWithMeta {
            id,
            article_id: row.get("article_id"),
            user_id,
            parent_id,
            nickname: display_name,
            email: email.clone(),
            content: row.get("content"),
            status: row.get::<String, _>("status").parse().unwrap_or_default(),
            created_at: row.get("created_at"),
            avatar_url,
            like_count,
            is_liked,
            is_author,
            replies: Vec::new(),
        };

        let idx = all_comments.len();
        id_to_index.insert(id, idx);
        all_comments.push(comment);
    }

    // Build tree: iterate in reverse so children are attached before parents
    let comments = build_comment_tree(all_comments, &id_to_index);

    Ok(comments)
}

/// Build a nested comment tree from a flat list.
/// Supports arbitrary depth nesting.
fn build_comment_tree(
    mut all_comments: Vec<CommentWithMeta>,
    id_to_index: &std::collections::HashMap<i64, usize>,
) -> Vec<CommentWithMeta> {
    // Process in reverse order so deeper children are moved first
    for i in (0..all_comments.len()).rev() {
        if let Some(pid) = all_comments[i].parent_id {
            if let Some(&parent_idx) = id_to_index.get(&pid) {
                if parent_idx != i {
                    let child = all_comments[i].clone();
                    all_comments[parent_idx].replies.push(child);
                    // Mark for removal by setting a sentinel
                    all_comments[i].parent_id = Some(-1);
                }
            }
        }
    }
    // Keep only root comments (parent_id is None) — the ones with Some(-1) were moved
    all_comments
        .into_iter()
        .filter(|c| c.parent_id.is_none())
        .collect()
}

async fn list_pending_sqlite(
    pool: &SqlitePool,
    page: i64,
    per_page: i64,
) -> Result<(Vec<CommentWithMeta>, i64)> {
    let offset = (page - 1) * per_page;

    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE status = 'pending'")
        .fetch_one(pool)
        .await?;

    let rows = sqlx::query(
        r#"SELECT c.*, u.username, a.title as article_title
           FROM comments c 
           LEFT JOIN users u ON c.user_id = u.id
           LEFT JOIN articles a ON c.article_id = a.id
           WHERE c.status = 'pending'
           ORDER BY c.created_at DESC
           LIMIT ? OFFSET ?"#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let comments: Vec<CommentWithMeta> = rows
        .iter()
        .map(|row| {
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
                is_author: false,
                replies: Vec::new(),
            }
        })
        .collect();

    Ok((comments, total))
}

async fn list_all_sqlite(
    pool: &SqlitePool,
    status: Option<&str>,
    page: i64,
    per_page: i64,
) -> Result<(Vec<CommentWithMeta>, i64)> {
    let offset = (page - 1) * per_page;

    let total: i64 = if let Some(s) = status {
        sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE status = ?")
            .bind(s)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM comments")
            .fetch_one(pool)
            .await?
    };

    let rows = if let Some(s) = status {
        sqlx::query(
            r#"SELECT c.*, u.username, a.title as article_title
               FROM comments c 
               LEFT JOIN users u ON c.user_id = u.id
               LEFT JOIN articles a ON c.article_id = a.id
               WHERE c.status = ?
               ORDER BY c.created_at DESC
               LIMIT ? OFFSET ?"#,
        )
        .bind(s)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT c.*, u.username, a.title as article_title
               FROM comments c 
               LEFT JOIN users u ON c.user_id = u.id
               LEFT JOIN articles a ON c.article_id = a.id
               ORDER BY c.created_at DESC
               LIMIT ? OFFSET ?"#,
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };

    let comments: Vec<CommentWithMeta> = rows
        .iter()
        .map(|row| {
            let email: Option<String> = row.get("email");
            let nickname: Option<String> = row.get("nickname");
            let username: Option<String> = row.try_get("username").ok();
            let display_name = username.or(nickname);
            let _article_title: Option<String> = row.try_get("article_title").ok().flatten();

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
                is_author: false,
                replies: Vec::new(),
            }
        })
        .collect();

    Ok((comments, total))
}

async fn list_recent_sqlite(pool: &SqlitePool, limit: i64) -> Result<Vec<CommentWithMeta>> {
    let rows = sqlx::query(
        r#"SELECT c.*, u.username, a.title as article_title, a.slug as article_slug
           FROM comments c
           LEFT JOIN users u ON c.user_id = u.id
           INNER JOIN articles a ON c.article_id = a.id
           WHERE c.status = 'approved' AND a.status = 'published'
           ORDER BY c.created_at DESC
           LIMIT ?"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let comments: Vec<CommentWithMeta> = rows
        .iter()
        .map(|row| {
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
                is_author: false,
                replies: Vec::new(),
            }
        })
        .collect();

    Ok(comments)
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
            sqlx::query(
                "UPDATE articles SET comment_count = MAX(0, comment_count - 1) WHERE id = ?",
            )
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
                sqlx::query(
                    "UPDATE articles SET comment_count = MAX(0, comment_count - 1) WHERE id = ?",
                )
                .bind(article_id)
                .execute(pool)
                .await?;
            }
            return Ok(true);
        }
    }

    Ok(false)
}

async fn add_like_sqlite(
    pool: &SqlitePool,
    target_type: LikeTargetType,
    target_id: i64,
    user_id: Option<i64>,
    fingerprint: Option<String>,
) -> Result<bool> {
    if !like_target_exists_sqlite(pool, &target_type, target_id).await? {
        return Ok(false);
    }

    let result = if let Some(uid) = user_id {
        sqlx::query(
            "INSERT OR IGNORE INTO likes (target_type, target_id, user_id) VALUES (?, ?, ?)",
        )
        .bind(target_type.to_string())
        .bind(target_id)
        .bind(uid)
        .execute(pool)
        .await?
    } else if let Some(fp) = fingerprint {
        sqlx::query(
            "INSERT OR IGNORE INTO likes (target_type, target_id, fingerprint) VALUES (?, ?, ?)",
        )
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

async fn remove_like_sqlite(
    pool: &SqlitePool,
    target_type: LikeTargetType,
    target_id: i64,
    user_id: Option<i64>,
    fingerprint: Option<String>,
) -> Result<bool> {
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

async fn is_liked_sqlite(
    pool: &SqlitePool,
    target_type: LikeTargetType,
    target_id: i64,
    user_id: Option<i64>,
    fingerprint: Option<&str>,
) -> Result<bool> {
    let count: i64 = if let Some(uid) = user_id {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM likes WHERE target_type = ? AND target_id = ? AND user_id = ?",
        )
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
    let result = sqlx::query("UPDATE articles SET view_count = view_count + 1 WHERE id = ?")
        .bind(article_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        anyhow::bail!("Article not found: {}", article_id);
    }
    Ok(())
}

async fn like_target_exists_sqlite(
    pool: &SqlitePool,
    target_type: &LikeTargetType,
    target_id: i64,
) -> Result<bool> {
    let count: i64 = match target_type {
        LikeTargetType::Article => {
            sqlx::query_scalar("SELECT COUNT(*) FROM articles WHERE id = ?")
                .bind(target_id)
                .fetch_one(pool)
                .await?
        }
        LikeTargetType::Comment => {
            sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE id = ?")
                .bind(target_id)
                .fetch_one(pool)
                .await?
        }
    };
    Ok(count > 0)
}

// MySQL implementations (similar to SQLite)
async fn create_mysql(
    pool: &MySqlPool,
    input: CreateCommentInput,
    user_id: Option<i64>,
    ip: Option<String>,
    ua: Option<String>,
    status: CommentStatus,
) -> Result<Comment> {
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
    let row = sqlx::query("SELECT * FROM comments WHERE id = ?")
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

async fn get_by_article_mysql(
    pool: &MySqlPool,
    article_id: i64,
    fingerprint: Option<&str>,
) -> Result<Vec<CommentWithMeta>> {
    // First get the article author_id
    let author_id: Option<i64> = sqlx::query_scalar("SELECT author_id FROM articles WHERE id = ?")
        .bind(article_id)
        .fetch_optional(pool)
        .await?;

    let rows = sqlx::query(
        r#"SELECT c.*, u.username, u.role as user_role,
           (SELECT COUNT(*) FROM likes WHERE target_type = 'comment' AND target_id = c.id) as like_count
           FROM comments c 
           LEFT JOIN users u ON c.user_id = u.id
           WHERE c.article_id = ? AND c.status = 'approved'
           ORDER BY c.created_at ASC"#
    )
    .bind(article_id)
    .fetch_all(pool)
    .await?;

    let mut all_comments: Vec<CommentWithMeta> = Vec::new();
    let mut id_to_index: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();

    for row in rows {
        let id: i64 = row.get("id");
        let parent_id: Option<i64> = row.get("parent_id");
        let email: Option<String> = row.get("email");
        let like_count: i64 = row.get("like_count");
        let nickname: Option<String> = row.get("nickname");
        let username: Option<String> = row.try_get("username").ok();
        let user_id: Option<i64> = row.get("user_id");
        let user_role: Option<String> = row.try_get("user_role").ok();

        // Use username if logged in user, otherwise use nickname
        let display_name = username.or(nickname);

        // Check if this comment is from the article author or an admin
        let is_author =
            user_id.is_some() && (user_id == author_id || user_role.as_deref() == Some("admin"));

        let is_liked = if let Some(fp) = fingerprint {
            is_liked_mysql(pool, LikeTargetType::Comment, id, None, Some(fp))
                .await
                .unwrap_or(false)
        } else {
            false
        };

        let comment = CommentWithMeta {
            id,
            article_id: row.get("article_id"),
            user_id,
            parent_id,
            nickname: display_name,
            email: email.clone(),
            content: row.get("content"),
            status: row.get::<String, _>("status").parse().unwrap_or_default(),
            created_at: row.get("created_at"),
            avatar_url: CommentWithMeta::gravatar_url(&email),
            like_count,
            is_liked,
            is_author,
            replies: Vec::new(),
        };

        let idx = all_comments.len();
        id_to_index.insert(id, idx);
        all_comments.push(comment);
    }

    // Build tree with arbitrary depth nesting
    let comments = build_comment_tree(all_comments, &id_to_index);

    Ok(comments)
}

async fn list_pending_mysql(
    pool: &MySqlPool,
    page: i64,
    per_page: i64,
) -> Result<(Vec<CommentWithMeta>, i64)> {
    let offset = (page - 1) * per_page;

    // Get total count
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE status = 'pending'")
        .fetch_one(pool)
        .await?;

    let rows = sqlx::query(
        r#"SELECT c.*, u.username, a.title as article_title
           FROM comments c 
           LEFT JOIN users u ON c.user_id = u.id
           LEFT JOIN articles a ON c.article_id = a.id
           WHERE c.status = 'pending'
           ORDER BY c.created_at DESC
           LIMIT ? OFFSET ?"#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let comments: Vec<CommentWithMeta> = rows
        .iter()
        .map(|row| {
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
                is_author: false,
                replies: Vec::new(),
            }
        })
        .collect();

    Ok((comments, total))
}

async fn list_all_mysql(
    pool: &MySqlPool,
    status: Option<&str>,
    page: i64,
    per_page: i64,
) -> Result<(Vec<CommentWithMeta>, i64)> {
    let offset = (page - 1) * per_page;

    let total: i64 = if let Some(s) = status {
        sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE status = ?")
            .bind(s)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM comments")
            .fetch_one(pool)
            .await?
    };

    let rows = if let Some(s) = status {
        sqlx::query(
            r#"SELECT c.*, u.username, a.title as article_title
               FROM comments c 
               LEFT JOIN users u ON c.user_id = u.id
               LEFT JOIN articles a ON c.article_id = a.id
               WHERE c.status = ?
               ORDER BY c.created_at DESC
               LIMIT ? OFFSET ?"#,
        )
        .bind(s)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT c.*, u.username, a.title as article_title
               FROM comments c 
               LEFT JOIN users u ON c.user_id = u.id
               LEFT JOIN articles a ON c.article_id = a.id
               ORDER BY c.created_at DESC
               LIMIT ? OFFSET ?"#,
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };

    let comments: Vec<CommentWithMeta> = rows
        .iter()
        .map(|row| {
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
                is_author: false,
                replies: Vec::new(),
            }
        })
        .collect();

    Ok((comments, total))
}

async fn list_recent_mysql(pool: &MySqlPool, limit: i64) -> Result<Vec<CommentWithMeta>> {
    let rows = sqlx::query(
        r#"SELECT c.*, u.username, a.title as article_title, a.slug as article_slug
           FROM comments c
           LEFT JOIN users u ON c.user_id = u.id
           INNER JOIN articles a ON c.article_id = a.id
           WHERE c.status = 'approved' AND a.status = 'published'
           ORDER BY c.created_at DESC
           LIMIT ?"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let comments: Vec<CommentWithMeta> = rows
        .iter()
        .map(|row| {
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
                is_author: false,
                replies: Vec::new(),
            }
        })
        .collect();

    Ok(comments)
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
            sqlx::query(
                "UPDATE articles SET comment_count = GREATEST(0, comment_count - 1) WHERE id = ?",
            )
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

async fn add_like_mysql(
    pool: &MySqlPool,
    target_type: LikeTargetType,
    target_id: i64,
    user_id: Option<i64>,
    fingerprint: Option<String>,
) -> Result<bool> {
    if !like_target_exists_mysql(pool, &target_type, target_id).await? {
        return Ok(false);
    }

    let result = if let Some(uid) = user_id {
        sqlx::query("INSERT IGNORE INTO likes (target_type, target_id, user_id) VALUES (?, ?, ?)")
            .bind(target_type.to_string())
            .bind(target_id)
            .bind(uid)
            .execute(pool)
            .await?
    } else if let Some(fp) = fingerprint {
        sqlx::query(
            "INSERT IGNORE INTO likes (target_type, target_id, fingerprint) VALUES (?, ?, ?)",
        )
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

async fn remove_like_mysql(
    pool: &MySqlPool,
    target_type: LikeTargetType,
    target_id: i64,
    user_id: Option<i64>,
    fingerprint: Option<String>,
) -> Result<bool> {
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
                sqlx::query(
                    "UPDATE articles SET like_count = GREATEST(0, like_count - 1) WHERE id = ?",
                )
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

async fn is_liked_mysql(
    pool: &MySqlPool,
    target_type: LikeTargetType,
    target_id: i64,
    user_id: Option<i64>,
    fingerprint: Option<&str>,
) -> Result<bool> {
    let count: i64 = if let Some(uid) = user_id {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM likes WHERE target_type = ? AND target_id = ? AND user_id = ?",
        )
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
    let result = sqlx::query("UPDATE articles SET view_count = view_count + 1 WHERE id = ?")
        .bind(article_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        anyhow::bail!("Article not found: {}", article_id);
    }
    Ok(())
}

async fn like_target_exists_mysql(
    pool: &MySqlPool,
    target_type: &LikeTargetType,
    target_id: i64,
) -> Result<bool> {
    let count: i64 = match target_type {
        LikeTargetType::Article => {
            sqlx::query_scalar("SELECT COUNT(*) FROM articles WHERE id = ?")
                .bind(target_id)
                .fetch_one(pool)
                .await?
        }
        LikeTargetType::Comment => {
            sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE id = ?")
                .bind(target_id)
                .fetch_one(pool)
                .await?
        }
    };
    Ok(count > 0)
}
