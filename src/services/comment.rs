//! Comment service

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use serde_json::json;
use crate::cache::{Cache, CacheLayer};
use crate::db::repositories::{CommentRepository, SettingsRepository};
use crate::models::{CommentStatus, CommentWithMeta, CreateCommentInput, LikeTargetType};
use crate::plugin::{HookManager, hook_names};

/// Default cache TTL for comments (5 minutes - comments change frequently)
const COMMENT_CACHE_TTL_SECS: u64 = 300;

/// Cache key prefixes
const CACHE_KEY_COMMENT_BY_ARTICLE: &str = "comment:article:";

/// Comment service
pub struct CommentService {
    repo: Arc<dyn CommentRepository>,
    settings_repo: Option<Arc<dyn SettingsRepository>>,
    hook_manager: Option<Arc<HookManager>>,
    cache: Arc<Cache>,
    cache_ttl: Duration,
}

impl CommentService {
    pub fn new(repo: Arc<dyn CommentRepository>, cache: Arc<Cache>) -> Self {
        Self { 
            repo,
            settings_repo: None,
            hook_manager: None,
            cache,
            cache_ttl: Duration::from_secs(COMMENT_CACHE_TTL_SECS),
        }
    }

    pub fn with_hooks(repo: Arc<dyn CommentRepository>, cache: Arc<Cache>, hook_manager: Arc<HookManager>) -> Self {
        Self {
            repo,
            settings_repo: None,
            hook_manager: Some(hook_manager),
            cache,
            cache_ttl: Duration::from_secs(COMMENT_CACHE_TTL_SECS),
        }
    }

    pub fn with_settings(mut self, settings_repo: Arc<dyn SettingsRepository>) -> Self {
        self.settings_repo = Some(settings_repo);
        self
    }

    /// Check if login is required to comment
    pub async fn check_require_login(&self) -> Result<bool> {
        if let Some(ref settings_repo) = self.settings_repo {
            if let Ok(Some(setting)) = settings_repo.get("require_login_to_comment").await {
                return Ok(setting.value == "true");
            }
        }
        Ok(false)
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

        // Determine comment status based on moderation settings
        let status = self.determine_comment_status(&input.content).await;

        let comment = self.repo.create_with_status(input.clone(), user_id, ip, user_agent, status).await?;

        // Invalidate cache - CRITICAL: must clear comment cache for this article
        let cache_key = format!("{}{}", CACHE_KEY_COMMENT_BY_ARTICLE, input.article_id);
        let _ = self.cache.delete(&cache_key).await;

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
                "status": comment.status.to_string(),
            })
        );

        Ok(comment)
    }

    /// Determine comment status based on moderation settings
    async fn determine_comment_status(&self, content: &str) -> CommentStatus {
        if let Some(ref settings_repo) = self.settings_repo {
            // Check if global moderation is enabled
            if let Ok(Some(setting)) = settings_repo.get("comment_moderation").await {
                if setting.value == "true" {
                    return CommentStatus::Pending;
                }
            }
            
            // Check for moderation keywords
            if let Ok(Some(setting)) = settings_repo.get("moderation_keywords").await {
                if !setting.value.is_empty() {
                    let keywords: Vec<&str> = setting.value.split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    
                    let content_lower = content.to_lowercase();
                    for keyword in keywords {
                        if content_lower.contains(&keyword.to_lowercase()) {
                            return CommentStatus::Pending;
                        }
                    }
                }
            }
        }
        
        CommentStatus::Approved
    }

    /// Get comments for an article
    pub async fn get_by_article(&self, article_id: i64, fingerprint: Option<&str>) -> Result<Vec<CommentWithMeta>> {
        // Try cache first
        let cache_key = format!("{}{}", CACHE_KEY_COMMENT_BY_ARTICLE, article_id);
        if let Ok(Some(comments)) = self.cache.get::<Vec<CommentWithMeta>>(&cache_key).await {
            return Ok(comments);
        }

        // Get from database
        let comments = self.repo.get_by_article(article_id, fingerprint).await?;

        // Cache the result
        let _ = self.cache.set(&cache_key, &comments, self.cache_ttl).await;

        Ok(comments)
    }

    /// Get pending comments for moderation
    pub async fn list_pending(&self, page: i64, per_page: i64) -> Result<(Vec<CommentWithMeta>, i64)> {
        self.repo.list_pending(page, per_page).await
    }

    /// Approve a comment
    pub async fn approve(&self, id: i64) -> Result<bool> {
        let result = self.repo.update_status(id, CommentStatus::Approved).await?;
        
        // Invalidate all comment caches - CRITICAL: we don't know which article without extra query
        let _ = self.cache.delete_pattern(&format!("{}*", CACHE_KEY_COMMENT_BY_ARTICLE)).await;
        
        Ok(result)
    }

    /// Reject a comment (delete it)
    pub async fn reject(&self, id: i64) -> Result<bool> {
        let result = self.repo.delete(id).await?;
        
        // Invalidate all comment caches - CRITICAL: we don't know which article without extra query
        let _ = self.cache.delete_pattern(&format!("{}*", CACHE_KEY_COMMENT_BY_ARTICLE)).await;
        
        Ok(result)
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

        // Invalidate all comment caches - CRITICAL: we don't know which article this comment belongs to
        let _ = self.cache.delete_pattern(&format!("{}*", CACHE_KEY_COMMENT_BY_ARTICLE)).await;

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
