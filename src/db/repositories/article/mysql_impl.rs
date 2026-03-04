//! MySQL implementations for article repository
use super::*;

// ============================================================================
// MySQL-specific implementations
// (Functions that differ from SQLite or cross-reference other _mysql fns)
// ============================================================================

pub(super) async fn create_article_mysql(pool: &MySqlPool, input: &CreateArticleInput) -> Result<Article> {
    let now = Utc::now();
    let status = input.status.unwrap_or_default();
    let published_at = if status == ArticleStatus::Published {
        Some(now)
    } else {
        None
    };
    let content_html = input.content_html.clone().unwrap_or_default();

    let result = sqlx::query(
        r#"
        INSERT INTO articles (slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, thumbnail, is_pinned, pin_order, scheduled_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&input.slug)
    .bind(&input.title)
    .bind(&input.content)
    .bind(&content_html)
    .bind(input.author_id)
    .bind(input.category_id)
    .bind(status.as_str())
    .bind(published_at)
    .bind(now)
    .bind(now)
    .bind::<Option<&str>>(None)
    .bind(false)
    .bind(0)
    .bind(input.scheduled_at)
    .execute(pool)
    .await
    .context("Failed to create article")?;

    let id = result.last_insert_id() as i64;

    Ok(Article {
        id,
        slug: input.slug.clone(),
        title: input.title.clone(),
        content: input.content.clone(),
        content_html,
        author_id: input.author_id,
        category_id: input.category_id,
        status,
        published_at,
        created_at: now,
        updated_at: now,
        view_count: 0,
        like_count: 0,
        comment_count: 0,
        thumbnail: None,
        is_pinned: false,
        pin_order: 0,
        meta: serde_json::json!({}),
        scheduled_at: input.scheduled_at,
    })
}

pub(super) async fn get_article_by_id_mysql(pool: &MySqlPool, id: i64) -> Result<Option<Article>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to get article by ID")?;

    match row {
        Some(row) => Ok(Some(row_to_article_mysql(&row)?)),
        None => Ok(None),
    }
}

pub(super) async fn get_article_by_slug_mysql(pool: &MySqlPool, slug: &str) -> Result<Option<Article>> {
    let row = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE slug = ?
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .context("Failed to get article by slug")?;

    match row {
        Some(row) => Ok(Some(row_to_article_mysql(&row)?)),
        None => Ok(None),
    }
}

pub(super) async fn list_articles_mysql(pool: &MySqlPool, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list articles")?;

    rows.iter().map(row_to_article_mysql).collect()
}

pub(super) async fn update_article_mysql(pool: &MySqlPool, id: i64, input: &UpdateArticleInput) -> Result<Article> {
    let existing = get_article_by_id_mysql(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Article not found"))?;

    let now = Utc::now();
    let new_slug = input.slug.as_ref().unwrap_or(&existing.slug);
    let new_title = input.title.as_ref().unwrap_or(&existing.title);
    let new_content = input.content.as_ref().unwrap_or(&existing.content);
    let new_content_html = input.content_html.as_ref().unwrap_or(&existing.content_html);
    let new_category_id = input.category_id.unwrap_or(existing.category_id);
    let new_status = input.status.unwrap_or(existing.status);
    let new_thumbnail = input.thumbnail.clone().or(existing.thumbnail.clone());
    let new_is_pinned = input.is_pinned.unwrap_or(existing.is_pinned);
    let new_pin_order = input.pin_order.unwrap_or(existing.pin_order);
    let new_scheduled_at = if input.scheduled_at.is_some() { input.scheduled_at } else { existing.scheduled_at };

    let new_published_at = if new_status == ArticleStatus::Published && existing.status != ArticleStatus::Published {
        Some(now)
    } else if new_status != ArticleStatus::Published {
        None
    } else {
        existing.published_at
    };

    sqlx::query(
        r#"
        UPDATE articles
        SET slug = ?, title = ?, content = ?, content_html = ?, category_id = ?, status = ?, published_at = ?, updated_at = ?, thumbnail = ?, is_pinned = ?, pin_order = ?, scheduled_at = ?
        WHERE id = ?
        "#,
    )
    .bind(new_slug)
    .bind(new_title)
    .bind(new_content)
    .bind(new_content_html)
    .bind(new_category_id)
    .bind(new_status.as_str())
    .bind(new_published_at)
    .bind(now)
    .bind(&new_thumbnail)
    .bind(new_is_pinned)
    .bind(new_pin_order)
    .bind(new_scheduled_at)
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to update article")?;

    get_article_by_id_mysql(pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Article not found after update"))
}

pub(super) async fn list_articles_by_category_mysql(pool: &MySqlPool, category_id: i64, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE category_id = ?
        ORDER BY is_pinned DESC, pin_order ASC, COALESCE(published_at, created_at) DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(category_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list articles by category")?;

    rows.iter().map(row_to_article_mysql).collect()
}

pub(super) async fn list_articles_by_tag_mysql(pool: &MySqlPool, tag_id: i64, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT a.id, a.slug, a.title, a.content, a.content_html, a.author_id, a.category_id, a.status, a.published_at, a.created_at, a.updated_at, a.view_count, a.like_count, a.comment_count, a.thumbnail, a.is_pinned, a.pin_order
        FROM articles a
        INNER JOIN article_tags at ON a.id = at.article_id
        WHERE at.tag_id = ?
        ORDER BY a.is_pinned DESC, a.pin_order ASC, COALESCE(a.published_at, a.created_at) DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(tag_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list articles by tag")?;

    rows.iter().map(row_to_article_mysql).collect()
}

pub(super) async fn list_published_articles_mysql(pool: &MySqlPool, offset: i64, limit: i64) -> Result<Vec<Article>> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE status = 'published'
        ORDER BY is_pinned DESC, pin_order ASC, published_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list published articles")?;

    rows.iter().map(row_to_article_mysql).collect()
}

pub(super) async fn search_articles_mysql(pool: &MySqlPool, keyword: &str, offset: i64, limit: i64, published_only: bool) -> Result<Vec<Article>> {
    let search_pattern = format!("%{}%", keyword);

    let query = if published_only {
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE status = 'published' AND (title LIKE ? OR content LIKE ?)
        ORDER BY is_pinned DESC, pin_order ASC, published_at DESC
        LIMIT ? OFFSET ?
        "#
    } else {
        r#"
        SELECT id, slug, title, content, content_html, author_id, category_id, status, published_at, created_at, updated_at, view_count, like_count, comment_count, thumbnail, is_pinned, pin_order
        FROM articles
        WHERE title LIKE ? OR content LIKE ?
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#
    };

    let rows = sqlx::query(query)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to search articles")?;

    rows.iter().map(row_to_article_mysql).collect()
}
