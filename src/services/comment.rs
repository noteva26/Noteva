//! Comment service

use std::sync::Arc;
use anyhow::Result;
use serde_json::json;
use crate::db::repositories::CommentRepository;
use crate::models::{CommentWithMeta, CreateCommentInput, LikeTargetType};
use crate::plugin::{HookManager, hook_names};

/// Comment service
pub struct CommentService {
    repo: Arc<dyn CommentRepository>,
    hook_manager: Option<Arc<HookManager>>,
}

impl CommentService {
    pub fn new(repo: Arc<dyn CommentRepository>) -> Self {
        Self { 
            repo,
            hook_manager: None,
        }
    }

    pub fn with_hooks(repo: Arc<dyn CommentRepository>, hook_manager: Arc<HookManager>) -> Self {
        Self {
            repo,
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

    /// Create a comment
    /// 
    /// # Hooks
    /// - `comment_before_create` - Triggered before creating, can modify input
    /// - `comment_after_create` - Triggered after creating, receives created comment
    pub async fn create(
        &self,
        mut input: CreateCommentInput,
        user_id: Option<i64>,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<crate::models::Comment> {
        // Trigger comment_before_create hook
        let hook_data = self.trigger_hook(
            hook_names::COMMENT_BEFORE_CREATE,
            json!({
                "article_id": input.article_id,
                "parent_id": input.parent_id,
                "content": input.content,
                "nickname": input.nickname,
                "email": input.email,
                "user_id": user_id,
            })
        );
        
        // Apply hook modifications
        if let Some(content) = hook_data.get("content").and_then(|v| v.as_str()) {
            input.content = content.to_string();
        }

        let comment = self.repo.create(input, user_id, ip, user_agent).await?;

        // Trigger comment_after_create hook
        self.trigger_hook(
            hook_names::COMMENT_AFTER_CREATE,
            json!({
                "id": comment.id,
                "article_id": comment.article_id,
                "parent_id": comment.parent_id,
                "content": comment.content,
                "nickname": comment.nickname,
                "user_id": comment.user_id,
            })
        );

        Ok(comment)
    }

    /// Get comments for an article
    pub async fn get_by_article(&self, article_id: i64, fingerprint: Option<&str>) -> Result<Vec<CommentWithMeta>> {
        self.repo.get_by_article(article_id, fingerprint).await
    }

    /// Delete a comment
    /// 
    /// # Hooks
    /// - `comment_before_delete` - Triggered before deleting
    /// - `comment_after_delete` - Triggered after deleting
    pub async fn delete(&self, id: i64) -> Result<bool> {
        // Trigger comment_before_delete hook
        self.trigger_hook(
            hook_names::COMMENT_BEFORE_DELETE,
            json!({ "id": id })
        );

        let result = self.repo.delete(id).await?;

        // Trigger comment_after_delete hook
        self.trigger_hook(
            hook_names::COMMENT_AFTER_DELETE,
            json!({ "id": id, "success": result })
        );

        Ok(result)
    }

    /// Like an article or comment
    pub async fn like(
        &self,
        target_type: LikeTargetType,
        target_id: i64,
        user_id: Option<i64>,
        fingerprint: Option<String>,
    ) -> Result<bool> {
        self.repo.add_like(target_type, target_id, user_id, fingerprint).await
    }

    /// Unlike an article or comment
    pub async fn unlike(
        &self,
        target_type: LikeTargetType,
        target_id: i64,
        user_id: Option<i64>,
        fingerprint: Option<String>,
    ) -> Result<bool> {
        self.repo.remove_like(target_type, target_id, user_id, fingerprint).await
    }

    /// Check if liked
    pub async fn is_liked(
        &self,
        target_type: LikeTargetType,
        target_id: i64,
        user_id: Option<i64>,
        fingerprint: Option<&str>,
    ) -> Result<bool> {
        self.repo.is_liked(target_type, target_id, user_id, fingerprint).await
    }

    /// Increment view count
    pub async fn increment_view(&self, article_id: i64) -> Result<()> {
        self.repo.increment_view(article_id).await
    }
}

/// Generate fingerprint from IP and User-Agent
pub fn generate_fingerprint(ip: &str, user_agent: &str) -> String {
    let data = format!("{}:{}", ip, user_agent);
    format!("{:x}", md5::compute(data))
}
