//! Article service
//!
//! Implements business logic for article management:
//! - Create, read, update, delete articles
//! - Markdown rendering
//! - Cache invalidation
//! - Validation
//! - Tag associations
//!
//! Satisfies requirements:
//! - 1.1: WHEN 用户提交新文�?THEN Article_Manager SHALL 创建文章记录并生成唯一标识�?
//! - 1.3: WHEN 用户更新文章内容 THEN Article_Manager SHALL 保存更改并更新修改时间戳
//! - 1.4: WHEN 用户删除文章 THEN Article_Manager SHALL 将文章标记为已删除或永久移除
//! - 1.5: WHEN 文章被创建或更新 THEN Article_Manager SHALL 使相关缓存失�?
//! - 1.7: IF 文章标题或内容为�?THEN Article_Manager SHALL 返回验证错误并拒绝保�?

use crate::cache::{Cache, CacheLayer};
use crate::db::repositories::{ArticleRepository, TagRepository};
use crate::models::{
    Article, ArticleSortBy, CreateArticleInput, ListParams, PagedResult, UpdateArticleInput,
};
use crate::plugin::{hook_names, HookManager};
use crate::services::markdown::MarkdownRenderer;
use anyhow::Context;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

/// Default cache TTL for single articles (1 hour)
const ARTICLE_CACHE_TTL_SECS: u64 = 3600;

/// Cache TTL for article lists (10 minutes - lists should refresh faster)
const ARTICLE_LIST_CACHE_TTL_SECS: u64 = 600;

/// Cache key prefixes
const CACHE_KEY_ARTICLE_BY_ID: &str = "article:id:";
const CACHE_KEY_ARTICLE_BY_SLUG: &str = "article:slug:";
const CACHE_KEY_ARTICLE_LIST: &str = "articles:list";

/// Error types for article service operations
#[derive(Debug, thiserror::Error)]
pub enum ArticleServiceError {
    /// Article not found
    #[error("Article not found: {0}")]
    NotFound(String),

    /// Validation error (Requirement 1.7)
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Duplicate slug
    #[error("Article slug already exists: {0}")]
    DuplicateSlug(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

/// Article service for managing blog articles
///
/// Provides business logic for article operations including:
/// - CRUD operations with validation
/// - Markdown rendering
/// - Cache management
/// - Tag associations
/// - Hook triggering for plugin integration
pub struct ArticleService {
    repo: Arc<dyn ArticleRepository>,
    tag_repo: Arc<dyn TagRepository>,
    cache: Arc<Cache>,
    markdown_renderer: MarkdownRenderer,
    cache_ttl: Duration,
    hook_manager: Option<Arc<HookManager>>,
}

impl ArticleService {
    /// Create a new article service
    ///
    /// # Arguments
    /// * `repo` - Article repository for database operations
    /// * `tag_repo` - Tag repository for tag associations
    /// * `cache` - Cache layer for caching
    /// * `markdown_renderer` - Markdown renderer for content conversion
    pub fn new(
        repo: Arc<dyn ArticleRepository>,
        tag_repo: Arc<dyn TagRepository>,
        cache: Arc<Cache>,
        markdown_renderer: MarkdownRenderer,
    ) -> Self {
        Self {
            repo,
            tag_repo,
            cache,
            markdown_renderer,
            cache_ttl: Duration::from_secs(ARTICLE_CACHE_TTL_SECS),
            hook_manager: None,
        }
    }

    /// Create a new article service with custom cache TTL
    pub fn with_cache_ttl(
        repo: Arc<dyn ArticleRepository>,
        tag_repo: Arc<dyn TagRepository>,
        cache: Arc<Cache>,
        markdown_renderer: MarkdownRenderer,
        cache_ttl: Duration,
    ) -> Self {
        Self {
            repo,
            tag_repo,
            cache,
            markdown_renderer,
            cache_ttl,
            hook_manager: None,
        }
    }

    /// Create a new article service with hook manager
    pub fn with_hooks(
        repo: Arc<dyn ArticleRepository>,
        tag_repo: Arc<dyn TagRepository>,
        cache: Arc<Cache>,
        markdown_renderer: MarkdownRenderer,
        hook_manager: Arc<HookManager>,
    ) -> Self {
        Self {
            repo,
            tag_repo,
            cache,
            markdown_renderer,
            cache_ttl: Duration::from_secs(ARTICLE_CACHE_TTL_SECS),
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

    /// Create a new article
    ///
    /// # Arguments
    /// * `input` - Article creation input
    /// * `tag_ids` - Optional list of tag IDs to associate with the article
    ///
    /// # Returns
    /// The created article
    ///
    /// # Errors
    /// - `ValidationError` if title or content is empty (Requirement 1.7)
    /// - `DuplicateSlug` if the slug already exists
    ///
    /// # Hooks
    /// - `article_before_create` - Triggered before creating, can modify input data
    /// - `article_after_create` - Triggered after creating, receives created article
    ///
    /// Satisfies requirements:
    /// - 1.1: WHEN 用户提交新文章 THEN Article_Manager SHALL 创建文章记录并生成唯一标识
    /// - 1.5: WHEN 文章被创建或更新 THEN Article_Manager SHALL 使相关缓存失效
    /// - 1.7: IF 文章标题或内容为空 THEN Article_Manager SHALL 返回验证错误并拒绝保存
    pub async fn create(
        &self,
        mut input: CreateArticleInput,
        tag_ids: Option<Vec<i64>>,
    ) -> Result<Article, ArticleServiceError> {
        // Trigger article_before_create hook
        let hook_data = self.trigger_hook(
            hook_names::ARTICLE_BEFORE_CREATE,
            json!({
                "title": input.title,
                "slug": input.slug,
                "content": input.content,
                "author_id": input.author_id,
                "category_id": input.category_id,
                "tag_ids": tag_ids,
            }),
        );

        // Apply hook modifications
        if let Some(title) = hook_data.get("title").and_then(|v| v.as_str()) {
            input.title = title.to_string();
        }
        if let Some(content) = hook_data.get("content").and_then(|v| v.as_str()) {
            input.content = content.to_string();
        }
        if let Some(slug) = hook_data.get("slug").and_then(|v| v.as_str()) {
            input.slug = slug.to_string();
        }

        // Validate input (Requirement 1.7)
        self.validate_create_input(&input)?;

        // Generate slug if empty
        if input.slug.trim().is_empty() {
            input.slug = generate_slug(&input.title);
        }

        // Check slug uniqueness
        if self
            .repo
            .exists_by_slug(&input.slug)
            .await
            .context("Failed to check slug uniqueness")?
        {
            return Err(ArticleServiceError::DuplicateSlug(input.slug));
        }

        // Render markdown to HTML with shortcode processing
        let _content_html = self
            .markdown_renderer
            .render_article(&input.content, 0, None);

        // Trigger article_content_filter hook
        let filter_data = self.trigger_hook(
            hook_names::ARTICLE_CONTENT_FILTER,
            json!({
                "content": &input.content,
                "article_id": 0,
            }),
        );

        let filtered_content = filter_data
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or(&input.content);

        let final_content_html = self
            .markdown_renderer
            .render_article(filtered_content, 0, None);
        input.content_html = Some(final_content_html);

        // Create article
        let article = self
            .repo
            .create(&input)
            .await
            .context("Failed to create article")?;

        // Associate tags if provided
        if let Some(ids) = tag_ids {
            for tag_id in ids {
                let _ = self.tag_repo.add_to_article(tag_id, article.id).await;
            }
        }

        // Invalidate cache (Requirement 1.5)
        self.invalidate_list_cache().await?;

        // Trigger article_after_create hook
        self.trigger_hook(
            hook_names::ARTICLE_AFTER_CREATE,
            json!({
                "id": article.id,
                "title": article.title,
                "slug": article.slug,
                "content": article.content,
                "author_id": article.author_id,
                "category_id": article.category_id,
                "status": format!("{:?}", article.status),
            }),
        );

        Ok(article)
    }

    /// Get article by ID
    ///
    /// # Arguments
    /// * `id` - Article ID
    ///
    /// # Returns
    /// The article if found, None otherwise
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Article>, ArticleServiceError> {
        // Try cache first
        let cache_key = format!("{}{}", CACHE_KEY_ARTICLE_BY_ID, id);
        if let Some(article) = self.cache.get::<Article>(&cache_key).await.ok().flatten() {
            return Ok(Some(article));
        }

        // Get from database
        let article = self
            .repo
            .get_by_id(id)
            .await
            .context("Failed to get article by ID")?;

        // Cache the result
        if let Some(ref art) = article {
            let _ = self.cache.set(&cache_key, art, self.cache_ttl).await;
        }

        Ok(article)
    }

    /// Get article by slug
    ///
    /// # Arguments
    /// * `slug` - Article slug
    ///
    /// # Returns
    /// The article if found, None otherwise
    pub async fn get_by_slug(&self, slug: &str) -> Result<Option<Article>, ArticleServiceError> {
        // Try cache first
        let cache_key = format!("{}{}", CACHE_KEY_ARTICLE_BY_SLUG, slug);
        if let Some(article) = self.cache.get::<Article>(&cache_key).await.ok().flatten() {
            return Ok(Some(article));
        }

        // Get from database
        let article = self
            .repo
            .get_by_slug(slug)
            .await
            .context("Failed to get article by slug")?;

        // Cache the result
        if let Some(ref art) = article {
            let _ = self.cache.set(&cache_key, art, self.cache_ttl).await;
        }

        Ok(article)
    }

    /// List articles with pagination
    ///
    /// Returns all articles (any status) ordered by created_at DESC.
    ///
    /// # Arguments
    /// * `params` - Pagination parameters
    ///
    /// # Returns
    /// Paginated result of articles
    ///
    /// Satisfies requirement 1.2: WHEN 用户请求文章列表 THEN Article_Manager SHALL 返回分页的文章列表，支持按时间排�?
    pub async fn list(
        &self,
        params: &ListParams,
        sort_by: ArticleSortBy,
    ) -> Result<PagedResult<Article>, ArticleServiceError> {
        let offset = params.offset();
        let limit = params.limit();

        let articles = self
            .repo
            .list(offset, limit, sort_by)
            .await
            .context("Failed to list articles")?;

        let total = self
            .repo
            .count()
            .await
            .context("Failed to count articles")?;

        Ok(PagedResult::new(articles, total, params))
    }

    /// List only published articles with pagination
    ///
    /// Returns only published articles ordered by published_at DESC.
    ///
    /// # Arguments
    /// * `params` - Pagination parameters
    ///
    /// # Returns
    /// Paginated result of published articles
    pub async fn list_published(
        &self,
        params: &ListParams,
        sort_by: ArticleSortBy,
    ) -> Result<PagedResult<Article>, ArticleServiceError> {
        let offset = params.offset();
        let limit = params.limit();

        // Try cache first (include sort variant in cache key)
        let cache_key = format!(
            "{}:published:{}:{}:{}",
            CACHE_KEY_ARTICLE_LIST,
            offset,
            limit,
            sort_by.cache_key()
        );
        if let Ok(Some(cached)) = self.cache.get::<PagedResult<Article>>(&cache_key).await {
            return Ok(cached);
        }

        let articles = self
            .repo
            .list_published(offset, limit, sort_by)
            .await
            .context("Failed to list published articles")?;

        let total = self
            .repo
            .count_published()
            .await
            .context("Failed to count published articles")?;

        let result = PagedResult::new(articles, total, params);

        // Cache the result (lists use shorter TTL for freshness)
        let _ = self
            .cache
            .set(
                &cache_key,
                &result,
                Duration::from_secs(ARTICLE_LIST_CACHE_TTL_SECS),
            )
            .await;

        Ok(result)
    }

    /// List articles by category with pagination
    ///
    /// # Arguments
    /// * `category_id` - Category ID to filter by
    /// * `params` - Pagination parameters
    ///
    /// # Returns
    /// Paginated result of articles in the category
    pub async fn list_by_category(
        &self,
        category_id: i64,
        params: &ListParams,
        sort_by: ArticleSortBy,
    ) -> Result<PagedResult<Article>, ArticleServiceError> {
        let offset = params.offset();
        let limit = params.limit();

        let articles = self
            .repo
            .list_by_category(category_id, offset, limit, sort_by)
            .await
            .context("Failed to list articles by category")?;

        let total = self
            .repo
            .count_by_category(category_id)
            .await
            .context("Failed to count articles by category")?;

        Ok(PagedResult::new(articles, total, params))
    }

    /// List articles by tag with pagination
    ///
    /// # Arguments
    /// * `tag_id` - Tag ID to filter by
    /// * `params` - Pagination parameters
    ///
    /// # Returns
    /// Paginated result of articles with the tag
    pub async fn list_by_tag(
        &self,
        tag_id: i64,
        params: &ListParams,
        sort_by: ArticleSortBy,
    ) -> Result<PagedResult<Article>, ArticleServiceError> {
        let offset = params.offset();
        let limit = params.limit();

        let articles = self
            .repo
            .list_by_tag(tag_id, offset, limit, sort_by)
            .await
            .context("Failed to list articles by tag")?;

        let total = self
            .repo
            .count_by_tag(tag_id)
            .await
            .context("Failed to count articles by tag")?;

        Ok(PagedResult::new(articles, total, params))
    }

    /// Search articles by keyword
    ///
    /// Searches in article title and content.
    ///
    /// # Arguments
    /// * `keyword` - Search keyword
    /// * `params` - Pagination parameters
    /// * `published_only` - If true, only search published articles
    ///
    /// # Returns
    /// Paginated result of matching articles
    pub async fn search(
        &self,
        keyword: &str,
        params: &ListParams,
        published_only: bool,
        sort_by: ArticleSortBy,
    ) -> Result<PagedResult<Article>, ArticleServiceError> {
        let offset = params.offset();
        let limit = params.limit();

        let articles = self
            .repo
            .search(keyword, offset, limit, published_only, sort_by)
            .await
            .context("Failed to search articles")?;

        let total = self
            .repo
            .count_search(keyword, published_only)
            .await
            .context("Failed to count search results")?;

        Ok(PagedResult::new(articles, total, params))
    }

    /// Update an article
    ///
    /// # Arguments
    /// * `id` - Article ID to update
    /// * `input` - Update input
    /// * `tag_ids` - Optional new list of tag IDs (replaces existing associations)
    ///
    /// # Returns
    /// The updated article
    ///
    /// # Errors
    /// - `NotFound` if the article doesn't exist
    /// - `ValidationError` if title or content is empty (Requirement 1.7)
    /// - `DuplicateSlug` if the new slug already exists
    ///
    /// # Hooks
    /// - `article_before_update` - Triggered before updating, can modify input data
    /// - `article_after_update` - Triggered after updating, receives updated article
    ///
    /// Satisfies requirements:
    /// - 1.3: WHEN 用户更新文章内容 THEN Article_Manager SHALL 保存更改并更新修改时间戳
    /// - 1.5: WHEN 文章被创建或更新 THEN Article_Manager SHALL 使相关缓存失效
    /// - 1.7: IF 文章标题或内容为空 THEN Article_Manager SHALL 返回验证错误并拒绝保存
    pub async fn update(
        &self,
        id: i64,
        mut input: UpdateArticleInput,
        tag_ids: Option<Vec<i64>>,
    ) -> Result<Article, ArticleServiceError> {
        // Get existing article
        let existing = self
            .repo
            .get_by_id(id)
            .await
            .context("Failed to get article")?
            .ok_or_else(|| {
                ArticleServiceError::NotFound(format!("Article with ID {} not found", id))
            })?;

        // Trigger article_before_update hook
        let hook_data = self.trigger_hook(
            hook_names::ARTICLE_BEFORE_UPDATE,
            json!({
                "id": id,
                "title": input.title,
                "slug": input.slug,
                "content": input.content,
                "status": input.status.as_ref().map(|s| format!("{:?}", s)),
                "existing": {
                    "title": existing.title,
                    "slug": existing.slug,
                    "status": format!("{:?}", existing.status),
                }
            }),
        );

        // Apply hook modifications
        if let Some(title) = hook_data.get("title").and_then(|v| v.as_str()) {
            input.title = Some(title.to_string());
        }
        if let Some(content) = hook_data.get("content").and_then(|v| v.as_str()) {
            input.content = Some(content.to_string());
        }

        // Validate input (Requirement 1.7)
        self.validate_update_input(&input, &existing)?;

        // Check slug uniqueness if slug is being changed
        if let Some(ref new_slug) = input.slug {
            if new_slug != &existing.slug {
                if self
                    .repo
                    .exists_by_slug_excluding(new_slug, id)
                    .await
                    .context("Failed to check slug uniqueness")?
                {
                    return Err(ArticleServiceError::DuplicateSlug(new_slug.clone()));
                }
            }
        }

        // Re-render markdown if content is being updated (with shortcode processing)
        if let Some(ref content) = input.content {
            // Trigger article_content_filter hook
            let filter_data = self.trigger_hook(
                hook_names::ARTICLE_CONTENT_FILTER,
                json!({
                    "content": content,
                    "article_id": id,
                }),
            );

            let filtered_content = filter_data
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or(content);

            let content_html = self
                .markdown_renderer
                .render_article(filtered_content, id, None);
            input.content_html = Some(content_html);
        }

        // Update article
        let updated = self
            .repo
            .update(id, &input)
            .await
            .context("Failed to update article")?;

        // Update tag associations if provided
        if let Some(new_tag_ids) = tag_ids {
            // Remove all existing tag associations
            self.remove_all_tags(id).await?;

            // Add new tag associations
            for tag_id in new_tag_ids {
                let _ = self.tag_repo.add_to_article(tag_id, id).await;
            }
        }

        // Invalidate cache (Requirement 1.5)
        self.invalidate_article_cache(id, &existing.slug).await?;
        if let Some(ref new_slug) = input.slug {
            if new_slug != &existing.slug {
                self.invalidate_article_cache(id, new_slug).await?;
            }
        }

        // Trigger article_after_update hook
        self.trigger_hook(
            hook_names::ARTICLE_AFTER_UPDATE,
            json!({
                "id": updated.id,
                "title": updated.title,
                "slug": updated.slug,
                "status": format!("{:?}", updated.status),
            }),
        );

        // Trigger article_status_change hook if status actually changed
        let old_status = format!("{:?}", existing.status);
        let new_status = format!("{:?}", updated.status);
        if old_status != new_status {
            self.trigger_hook(
                hook_names::ARTICLE_STATUS_CHANGE,
                json!({
                    "id": updated.id,
                    "title": updated.title,
                    "slug": updated.slug,
                    "old_status": old_status,
                    "new_status": new_status,
                    "trigger": "manual",
                }),
            );
        }

        Ok(updated)
    }

    /// Delete an article
    ///
    /// # Arguments
    /// * `id` - Article ID to delete
    ///
    /// # Errors
    /// - `NotFound` if the article doesn't exist
    ///
    /// # Hooks
    /// - `article_before_delete` - Triggered before deleting
    /// - `article_after_delete` - Triggered after deleting
    ///
    /// Satisfies requirements:
    /// - 1.4: WHEN 用户删除文章 THEN Article_Manager SHALL 将文章标记为已删除或永久移除
    /// - 1.5: WHEN 文章被创建或更新 THEN Article_Manager SHALL 使相关缓存失效
    pub async fn delete(&self, id: i64) -> Result<(), ArticleServiceError> {
        // Get existing article for cache invalidation
        let existing = self
            .repo
            .get_by_id(id)
            .await
            .context("Failed to get article")?
            .ok_or_else(|| {
                ArticleServiceError::NotFound(format!("Article with ID {} not found", id))
            })?;

        // Trigger article_before_delete hook
        self.trigger_hook(
            hook_names::ARTICLE_BEFORE_DELETE,
            json!({
                "id": id,
                "title": existing.title,
                "slug": existing.slug,
            }),
        );

        // Delete article (tag associations are removed via CASCADE)
        self.repo
            .delete(id)
            .await
            .context("Failed to delete article")?;

        // Invalidate cache (Requirement 1.5)
        self.invalidate_article_cache(id, &existing.slug).await?;

        // Trigger article_after_delete hook
        self.trigger_hook(
            hook_names::ARTICLE_AFTER_DELETE,
            json!({
                "id": id,
                "title": existing.title,
                "slug": existing.slug,
            }),
        );

        Ok(())
    }

    /// Render markdown content to HTML
    ///
    /// # Arguments
    /// * `content` - Markdown content
    ///
    /// # Returns
    /// Rendered HTML string
    ///
    /// Satisfies requirement 1.6: WHEN 文章包含 Markdown 内容 THEN Article_Manager SHALL 将其解析�?HTML 进行渲染
    pub fn render_markdown(&self, content: &str) -> String {
        self.markdown_renderer.render(content)
    }

    /// Extract table of contents from markdown content.
    pub fn extract_toc(&self, content: &str) -> Vec<crate::services::markdown::TocEntry> {
        self.markdown_renderer.extract_toc(content)
    }

    /// Render markdown content to HTML with shortcode processing
    ///
    /// # Arguments
    /// * `content` - Markdown content
    /// * `article_id` - Optional article ID for shortcode context
    /// * `user_id` - Optional user ID for shortcode context
    ///
    /// # Returns
    /// Rendered HTML string with shortcodes processed
    pub fn render_markdown_with_shortcodes(
        &self,
        content: &str,
        article_id: Option<i64>,
        user_id: Option<i64>,
    ) -> String {
        self.markdown_renderer
            .render_article(content, article_id.unwrap_or(0), user_id)
    }

    /// Count total articles (all statuses)
    ///
    /// # Returns
    /// Total number of articles
    pub async fn count(&self) -> Result<i64, ArticleServiceError> {
        self.repo
            .count()
            .await
            .context("Failed to count articles")
            .map_err(Into::into)
    }

    /// Count published articles only
    ///
    /// # Returns
    /// Number of published articles
    pub async fn count_published(&self) -> Result<i64, ArticleServiceError> {
        self.repo
            .count_published()
            .await
            .context("Failed to count published articles")
            .map_err(Into::into)
    }

    /// Get adjacent (prev/next) published articles for navigation.
    /// Returns (prev_article, next_article) where prev is newer and next is older.
    pub async fn get_adjacent(
        &self,
        article_id: i64,
        published_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(Option<Article>, Option<Article>), ArticleServiceError> {
        self.repo
            .get_adjacent(article_id, published_at)
            .await
            .context("Failed to get adjacent articles")
            .map_err(Into::into)
    }

    /// Get monthly archive counts for published articles.
    /// Returns Vec of (month_string, count) sorted newest first.
    pub async fn get_archives_monthly(&self) -> Result<Vec<(String, i64)>, ArticleServiceError> {
        self.repo
            .get_archives_monthly()
            .await
            .context("Failed to get monthly archives")
            .map_err(Into::into)
    }

    /// Get related articles (same category, excluding self, published only).
    pub async fn get_related(
        &self,
        article_id: i64,
        category_id: i64,
        limit: i64,
    ) -> Result<Vec<Article>, ArticleServiceError> {
        self.repo
            .get_related(article_id, category_id, limit)
            .await
            .context("Failed to get related articles")
            .map_err(Into::into)
    }

    // ========================================================================
    // Private helper methods
    // ========================================================================

    /// Validate article creation input
    ///
    /// Satisfies requirement 1.7: IF 文章标题或内容为�?THEN Article_Manager SHALL 返回验证错误并拒绝保�?
    fn validate_create_input(&self, input: &CreateArticleInput) -> Result<(), ArticleServiceError> {
        // Title cannot be empty or whitespace-only
        if input.title.trim().is_empty() {
            return Err(ArticleServiceError::ValidationError(
                "Article title cannot be empty".to_string(),
            ));
        }

        // Content cannot be empty or whitespace-only
        if input.content.trim().is_empty() {
            return Err(ArticleServiceError::ValidationError(
                "Article content cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate article update input
    ///
    /// Satisfies requirement 1.7: IF 文章标题或内容为�?THEN Article_Manager SHALL 返回验证错误并拒绝保�?
    fn validate_update_input(
        &self,
        input: &UpdateArticleInput,
        existing: &Article,
    ) -> Result<(), ArticleServiceError> {
        // If title is being updated, it cannot be empty
        if let Some(ref title) = input.title {
            if title.trim().is_empty() {
                return Err(ArticleServiceError::ValidationError(
                    "Article title cannot be empty".to_string(),
                ));
            }
        }

        // If content is being updated, it cannot be empty
        if let Some(ref content) = input.content {
            if content.trim().is_empty() {
                return Err(ArticleServiceError::ValidationError(
                    "Article content cannot be empty".to_string(),
                ));
            }
        }

        // Check that the resulting article would have non-empty title and content
        let final_title = input.title.as_ref().unwrap_or(&existing.title);
        let final_content = input.content.as_ref().unwrap_or(&existing.content);

        if final_title.trim().is_empty() {
            return Err(ArticleServiceError::ValidationError(
                "Article title cannot be empty".to_string(),
            ));
        }

        if final_content.trim().is_empty() {
            return Err(ArticleServiceError::ValidationError(
                "Article content cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Remove all tag associations for an article
    async fn remove_all_tags(&self, article_id: i64) -> Result<(), ArticleServiceError> {
        self.tag_repo
            .remove_all_by_article(article_id)
            .await
            .context("Failed to remove all tags from article")?;
        Ok(())
    }

    /// Invalidate cache for a specific article
    ///
    /// Satisfies requirement 1.5: WHEN 文章被创建或更新 THEN Article_Manager SHALL 使相关缓存失�?
    pub async fn invalidate_article_cache(
        &self,
        id: i64,
        slug: &str,
    ) -> Result<(), ArticleServiceError> {
        // Delete article by ID cache
        let id_key = format!("{}{}", CACHE_KEY_ARTICLE_BY_ID, id);
        let _ = self.cache.delete(&id_key).await;

        // Delete article by slug cache
        let slug_key = format!("{}{}", CACHE_KEY_ARTICLE_BY_SLUG, slug);
        let _ = self.cache.delete(&slug_key).await;

        // Invalidate list caches
        self.invalidate_list_cache().await?;

        Ok(())
    }

    /// Invalidate all article list caches
    ///
    /// Satisfies requirement 1.5: WHEN 文章被创建或更新 THEN Article_Manager SHALL 使相关缓存失�?
    async fn invalidate_list_cache(&self) -> Result<(), ArticleServiceError> {
        let _ = self
            .cache
            .delete_pattern(&format!("{}*", CACHE_KEY_ARTICLE_LIST))
            .await;
        Ok(())
    }
}

/// Generate a URL-friendly slug from a title
///
/// Converts the title to lowercase, replaces spaces and special characters
/// with hyphens, and removes consecutive hyphens.
pub fn generate_slug(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else if c == ' ' || c == '_' || c == '-' {
                '-'
            } else if !c.is_ascii() {
                // For non-ASCII characters (like Chinese), keep them
                c
            } else {
                // Replace other ASCII special characters with hyphen
                '-'
            }
        })
        .collect();

    // Remove consecutive hyphens and trim hyphens from ends
    let mut result = String::new();
    let mut prev_hyphen = false;

    for c in slug.chars() {
        if c == '-' {
            if !prev_hyphen && !result.is_empty() {
                result.push(c);
                prev_hyphen = true;
            }
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }

    // Trim trailing hyphen
    result.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests;
