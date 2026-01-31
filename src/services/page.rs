//! Page service

use crate::db::repositories::PageRepository;
use crate::models::{Page, PageStatus};
use crate::services::MarkdownRenderer;
use anyhow::{Context, Result};
use std::sync::Arc;

pub struct PageService {
    repo: Arc<dyn PageRepository>,
    markdown: MarkdownRenderer,
}

impl PageService {
    pub fn new(repo: Arc<dyn PageRepository>) -> Self {
        Self {
            repo,
            markdown: MarkdownRenderer::new(),
        }
    }

    pub async fn create(&self, slug: String, title: String, content: String, status: Option<String>) -> Result<Page> {
        // Check slug uniqueness
        if self.repo.exists_by_slug(&slug).await? {
            anyhow::bail!("Page with slug '{}' already exists", slug);
        }

        let content_html = self.markdown.render(&content);
        let mut page = Page::new(slug, title, content, content_html);
        if let Some(s) = status {
            page.status = s.parse().unwrap_or(PageStatus::Draft);
        }

        self.repo.create(&page).await.context("Failed to create page")
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<Page>> {
        self.repo.get_by_id(id).await
    }

    pub async fn get_by_slug(&self, slug: &str) -> Result<Option<Page>> {
        self.repo.get_by_slug(slug).await
    }

    pub async fn get_published_by_slug(&self, slug: &str) -> Result<Option<Page>> {
        let page = self.repo.get_by_slug(slug).await?;
        Ok(page.filter(|p| p.status == PageStatus::Published))
    }

    pub async fn list(&self) -> Result<Vec<Page>> {
        self.repo.list().await
    }

    pub async fn list_published(&self) -> Result<Vec<Page>> {
        self.repo.list_published().await
    }

    pub async fn update(&self, id: i64, slug: Option<String>, title: Option<String>, content: Option<String>, status: Option<String>) -> Result<Page> {
        let mut page = self.repo.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Page not found"))?;

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

        self.repo.update(&page).await
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        self.repo.delete(id).await
    }
}
