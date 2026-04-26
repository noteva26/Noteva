//! Page service

use crate::cache::{Cache, CacheLayer};
use crate::db::repositories::PageRepository;
use crate::models::{Page, PageStatus};
use crate::plugin::HookManager;
use crate::services::MarkdownRenderer;
use anyhow::{Context, Result};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

/// Default cache TTL for pages (1 hour - pages rarely change)
const PAGE_CACHE_TTL_SECS: u64 = 3600;

/// Cache key prefixes
const CACHE_KEY_PAGE_BY_ID: &str = "page:id:";
const CACHE_KEY_PAGE_BY_SLUG: &str = "page:slug:";
const CACHE_KEY_PAGE_LIST: &str = "page:list";
const CACHE_KEY_PAGE_LIST_PUBLISHED: &str = "page:list:published";

pub struct PageService {
    repo: Arc<dyn PageRepository>,
    markdown: MarkdownRenderer,
    cache: Arc<Cache>,
    cache_ttl: Duration,
    hook_manager: Option<Arc<HookManager>>,
}

impl PageService {
    pub fn new(repo: Arc<dyn PageRepository>, cache: Arc<Cache>) -> Self {
        Self {
            repo,
            markdown: MarkdownRenderer::new(),
            cache,
            cache_ttl: Duration::from_secs(PAGE_CACHE_TTL_SECS),
            hook_manager: None,
        }
    }

    pub fn with_hooks(
        repo: Arc<dyn PageRepository>,
        cache: Arc<Cache>,
        hook_manager: Arc<HookManager>,
    ) -> Self {
        Self {
            repo,
            markdown: MarkdownRenderer::new(),
            cache,
            cache_ttl: Duration::from_secs(PAGE_CACHE_TTL_SECS),
            hook_manager: Some(hook_manager),
        }
    }

    fn trigger_hook(&self, name: &str, data: serde_json::Value) -> serde_json::Value {
        if let Some(ref manager) = self.hook_manager {
            manager.trigger(name, data.clone())
        } else {
            data
        }
    }

    pub async fn create(
        &self,
        slug: String,
        title: String,
        content: String,
        status: Option<String>,
    ) -> Result<Page> {
        // Hook: page_before_create
        let hook_data = self.trigger_hook(
            "page_before_create",
            json!({ "slug": slug, "title": title }),
        );
        let slug = hook_data["slug"].as_str().map(String::from).unwrap_or(slug);
        let title = hook_data["title"]
            .as_str()
            .map(String::from)
            .unwrap_or(title);

        // Check slug uniqueness
        if self.repo.exists_by_slug(&slug).await? {
            anyhow::bail!("Page with slug '{}' already exists", slug);
        }

        let content_html = self.markdown.render(&content);
        let mut page = Page::new(slug, title, content, content_html);
        if let Some(s) = status {
            page.status = s.parse().unwrap_or(PageStatus::Draft);
        }

        let created = self
            .repo
            .create(&page)
            .await
            .context("Failed to create page")?;

        // Invalidate cache
        self.invalidate_cache().await?;

        // Hook: page_after_create
        self.trigger_hook(
            "page_after_create",
            json!({ "id": created.id, "slug": created.slug, "title": created.title }),
        );

        Ok(created)
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<Page>> {
        // Try cache first
        let cache_key = format!("{}{}", CACHE_KEY_PAGE_BY_ID, id);
        if let Ok(Some(page)) = self.cache.get::<Page>(&cache_key).await {
            return Ok(Some(page));
        }

        // Get from database
        let page = self.repo.get_by_id(id).await?;

        // Cache the result
        if let Some(ref p) = page {
            let _ = self.cache.set(&cache_key, p, self.cache_ttl).await;
        }

        Ok(page)
    }

    pub async fn get_by_slug(&self, slug: &str) -> Result<Option<Page>> {
        // Try cache first
        let cache_key = format!("{}{}", CACHE_KEY_PAGE_BY_SLUG, slug);
        if let Ok(Some(page)) = self.cache.get::<Page>(&cache_key).await {
            return Ok(Some(page));
        }

        // Get from database
        let page = self.repo.get_by_slug(slug).await?;

        // Cache the result
        if let Some(ref p) = page {
            let _ = self.cache.set(&cache_key, p, self.cache_ttl).await;
        }

        Ok(page)
    }

    pub async fn get_published_by_slug(&self, slug: &str) -> Result<Option<Page>> {
        let page = self.repo.get_by_slug(slug).await?;
        Ok(page.filter(|p| p.status == PageStatus::Published))
    }

    pub async fn list(&self) -> Result<Vec<Page>> {
        // Try cache first
        if let Ok(Some(pages)) = self.cache.get::<Vec<Page>>(CACHE_KEY_PAGE_LIST).await {
            return Ok(pages);
        }

        // Get from database
        let pages = self.repo.list().await?;

        // Cache the result
        let _ = self
            .cache
            .set(CACHE_KEY_PAGE_LIST, &pages, self.cache_ttl)
            .await;

        Ok(pages)
    }

    pub async fn list_published(&self) -> Result<Vec<Page>> {
        // Try cache first
        if let Ok(Some(pages)) = self
            .cache
            .get::<Vec<Page>>(CACHE_KEY_PAGE_LIST_PUBLISHED)
            .await
        {
            return Ok(pages);
        }

        // Get from database
        let pages = self.repo.list_published().await?;

        // Cache the result
        let _ = self
            .cache
            .set(CACHE_KEY_PAGE_LIST_PUBLISHED, &pages, self.cache_ttl)
            .await;

        Ok(pages)
    }

    pub async fn update(
        &self,
        id: i64,
        slug: Option<String>,
        title: Option<String>,
        content: Option<String>,
        status: Option<String>,
    ) -> Result<Page> {
        let mut page = self
            .repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        // Hook: page_before_update
        self.trigger_hook(
            "page_before_update",
            json!({ "id": id, "slug": page.slug, "title": page.title }),
        );

        let old_slug = page.slug.clone();

        if let Some(new_slug) = slug {
            if new_slug != page.slug && self.repo.exists_by_slug(&new_slug).await? {
                anyhow::bail!("Page with slug '{}' already exists", new_slug);
            }
            page.slug = new_slug;
        }

        if let Some(new_title) = title {
            page.title = new_title;
        }

        if let Some(new_content) = content {
            page.content_html = self.markdown.render(&new_content);
            page.content = new_content;
        }

        if let Some(new_status) = status {
            page.status = new_status.parse().unwrap_or(PageStatus::Draft);
        }

        let updated = self.repo.update(&page).await?;

        // Invalidate cache
        self.invalidate_page_cache(id, &old_slug).await?;
        if page.slug != old_slug {
            self.invalidate_page_cache(id, &page.slug).await?;
        }

        // Hook: page_after_update
        self.trigger_hook(
            "page_after_update",
            json!({ "id": id, "slug": updated.slug, "title": updated.title }),
        );

        Ok(updated)
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        let page = self
            .repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        // Hook: page_before_delete
        self.trigger_hook(
            "page_before_delete",
            json!({ "id": id, "slug": page.slug, "title": page.title }),
        );

        self.repo.delete(id).await?;

        // Invalidate cache
        self.invalidate_page_cache(id, &page.slug).await?;

        // Hook: page_after_delete
        self.trigger_hook(
            "page_after_delete",
            json!({ "id": id, "slug": page.slug, "title": page.title }),
        );

        Ok(())
    }

    /// Invalidate cache for a specific page
    async fn invalidate_page_cache(&self, id: i64, slug: &str) -> Result<()> {
        // Delete page by ID cache
        let id_key = format!("{}{}", CACHE_KEY_PAGE_BY_ID, id);
        let _ = self.cache.delete(&id_key).await;

        // Delete page by slug cache
        let slug_key = format!("{}{}", CACHE_KEY_PAGE_BY_SLUG, slug);
        let _ = self.cache.delete(&slug_key).await;

        // Invalidate list caches
        self.invalidate_cache().await?;

        Ok(())
    }

    /// Invalidate all page list caches
    async fn invalidate_cache(&self) -> Result<()> {
        let _ = self.cache.delete(CACHE_KEY_PAGE_LIST).await;
        let _ = self.cache.delete(CACHE_KEY_PAGE_LIST_PUBLISHED).await;
        Ok(())
    }

    /// Ensure pages declared by a plugin or theme exist in the database.
    ///
    /// For each (slug, title) pair, creates a published Page if no page with
    /// that slug exists yet. Existing pages are never overwritten.
    /// The `source` field records the origin (e.g. "plugin:friendlinks", "theme:fusion").
    pub async fn ensure_pages(&self, pages: &[(String, String)], source: &str) -> Result<usize> {
        let mut created = 0usize;
        for (slug, title) in pages {
            if self.repo.exists_by_slug(slug).await? {
                tracing::debug!("Page '{}' already exists, skipping auto-creation", slug);
                continue;
            }
            let page = Page::new_auto(slug.clone(), title.clone(), source.to_string());
            self.repo
                .create(&page)
                .await
                .with_context(|| format!("Failed to auto-create page '{}'", slug))?;
            tracing::info!("Auto-created page '{}' (source: {})", slug, source);
            created += 1;
        }
        if created > 0 {
            self.invalidate_cache().await?;
        }
        Ok(created)
    }
}
