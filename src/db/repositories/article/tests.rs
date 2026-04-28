use super::*;
use crate::db::repositories::tag::{SqlxTagRepository, TagRepository};
use crate::db::{create_test_pool, migrations};
use crate::models::{ArticleSortBy, ListParams, PagedResult, Tag};

async fn setup_test_repo() -> (DynDatabasePool, SqlxArticleRepository) {
    let pool = create_test_pool()
        .await
        .expect("Failed to create test pool");
    migrations::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");
    let repo = SqlxArticleRepository::new(pool.clone());
    (pool, repo)
}

/// Helper to create a user for article tests
async fn create_test_user(pool: &SqlitePool) -> i64 {
    let result =
        sqlx::query("INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)")
            .bind("testuser")
            .bind("test@example.com")
            .bind("hash123")
            .bind("author")
            .execute(pool)
            .await
            .expect("Failed to create test user");
    result.last_insert_rowid()
}

/// Helper to create a category for article tests
async fn create_test_category(pool: &SqlitePool, slug: &str) -> i64 {
    let result = sqlx::query("INSERT INTO categories (slug, name, sort_order) VALUES (?, ?, ?)")
        .bind(slug)
        .bind(format!("Category {}", slug))
        .bind(0)
        .execute(pool)
        .await
        .expect("Failed to create test category");
    result.last_insert_rowid()
}

fn create_test_input(
    slug: &str,
    title: &str,
    author_id: i64,
    category_id: i64,
) -> CreateArticleInput {
    CreateArticleInput {
        slug: slug.to_string(),
        title: title.to_string(),
        content: format!("Content for {}", title),
        content_html: Some(format!("<p>Content for {}</p>", title)),
        author_id,
        category_id,
        status: None,
        scheduled_at: None,
    }
}

#[tokio::test]
async fn test_create_article() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    let input = create_test_input("test-article", "Test Article", user_id, category_id);
    let created = repo.create(&input).await.expect("Failed to create article");

    assert!(created.id > 0);
    assert_eq!(created.slug, "test-article");
    assert_eq!(created.title, "Test Article");
    assert_eq!(created.status, ArticleStatus::Draft);
    assert!(created.published_at.is_none());
}

#[tokio::test]
async fn test_create_published_article() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    let mut input = create_test_input(
        "published-article",
        "Published Article",
        user_id,
        category_id,
    );
    input.status = Some(ArticleStatus::Published);

    let created = repo.create(&input).await.expect("Failed to create article");

    assert_eq!(created.status, ArticleStatus::Published);
    assert!(created.published_at.is_some());
}

#[tokio::test]
async fn test_get_article_by_id() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    let input = create_test_input("get-by-id", "Get By ID", user_id, category_id);
    let created = repo.create(&input).await.expect("Failed to create article");

    let found = repo
        .get_by_id(created.id)
        .await
        .expect("Failed to get article")
        .expect("Article not found");

    assert_eq!(found.id, created.id);
    assert_eq!(found.slug, "get-by-id");
    assert_eq!(found.title, "Get By ID");
}

#[tokio::test]
async fn test_get_article_by_id_not_found() {
    let (_pool, repo) = setup_test_repo().await;

    let found = repo.get_by_id(99999).await.expect("Failed to get article");

    assert!(found.is_none());
}

#[tokio::test]
async fn test_get_article_by_slug() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    let input = create_test_input("unique-slug", "Unique Slug", user_id, category_id);
    repo.create(&input).await.expect("Failed to create article");

    let found = repo
        .get_by_slug("unique-slug")
        .await
        .expect("Failed to get article")
        .expect("Article not found");

    assert_eq!(found.slug, "unique-slug");
}

#[tokio::test]
async fn test_get_article_by_slug_not_found() {
    let (_pool, repo) = setup_test_repo().await;

    let found = repo
        .get_by_slug("nonexistent")
        .await
        .expect("Failed to get article");

    assert!(found.is_none());
}

#[tokio::test]
async fn test_list_articles() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    // Create some articles
    for i in 1..=3 {
        let input = create_test_input(
            &format!("article-{}", i),
            &format!("Article {}", i),
            user_id,
            category_id,
        );
        repo.create(&input).await.expect("Failed to create article");
    }

    let articles = repo
        .list(0, 10, ArticleSortBy::default())
        .await
        .expect("Failed to list articles");

    assert_eq!(articles.len(), 3);
}

#[tokio::test]
async fn test_list_articles_pagination() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    // Create 5 articles
    for i in 1..=5 {
        let input = create_test_input(
            &format!("article-{}", i),
            &format!("Article {}", i),
            user_id,
            category_id,
        );
        repo.create(&input).await.expect("Failed to create article");
    }

    // Get first page (2 items)
    let page1 = repo
        .list(0, 2, ArticleSortBy::default())
        .await
        .expect("Failed to list articles");
    assert_eq!(page1.len(), 2);

    // Get second page (2 items)
    let page2 = repo
        .list(2, 2, ArticleSortBy::default())
        .await
        .expect("Failed to list articles");
    assert_eq!(page2.len(), 2);

    // Get third page (1 item)
    let page3 = repo
        .list(4, 2, ArticleSortBy::default())
        .await
        .expect("Failed to list articles");
    assert_eq!(page3.len(), 1);
}

#[tokio::test]
async fn test_count_articles() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    // Initially 0
    let count = repo.count().await.expect("Failed to count articles");
    assert_eq!(count, 0);

    // Create 3 articles
    for i in 1..=3 {
        let input = create_test_input(
            &format!("article-{}", i),
            &format!("Article {}", i),
            user_id,
            category_id,
        );
        repo.create(&input).await.expect("Failed to create article");
    }

    let count = repo.count().await.expect("Failed to count articles");
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_update_article() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    let input = create_test_input("to-update", "To Update", user_id, category_id);
    let created = repo.create(&input).await.expect("Failed to create article");

    let update_input = UpdateArticleInput::new()
        .with_title("Updated Title".to_string())
        .with_content("Updated content".to_string());

    let updated = repo
        .update(created.id, &update_input)
        .await
        .expect("Failed to update article");

    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.content, "Updated content");
    assert_eq!(updated.slug, "to-update"); // Unchanged
}

#[tokio::test]
async fn test_update_article_can_clear_thumbnail() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    let input = create_test_input("thumbnail-test", "Thumbnail Test", user_id, category_id);
    let created = repo.create(&input).await.expect("Failed to create article");

    let mut set_thumbnail = UpdateArticleInput::new();
    set_thumbnail.thumbnail = Some(Some("/uploads/cover.png".to_string()));
    let updated = repo
        .update(created.id, &set_thumbnail)
        .await
        .expect("Failed to set thumbnail");
    assert_eq!(updated.thumbnail.as_deref(), Some("/uploads/cover.png"));

    let mut clear_thumbnail = UpdateArticleInput::new();
    clear_thumbnail.thumbnail = Some(None);
    let updated = repo
        .update(created.id, &clear_thumbnail)
        .await
        .expect("Failed to clear thumbnail");
    assert!(updated.thumbnail.is_none());
}

#[tokio::test]
async fn test_update_article_status_to_published() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    let input = create_test_input("draft-article", "Draft Article", user_id, category_id);
    let created = repo.create(&input).await.expect("Failed to create article");
    assert_eq!(created.status, ArticleStatus::Draft);
    assert!(created.published_at.is_none());

    let update_input = UpdateArticleInput::new().with_status(ArticleStatus::Published);

    let updated = repo
        .update(created.id, &update_input)
        .await
        .expect("Failed to update article");

    assert_eq!(updated.status, ArticleStatus::Published);
    assert!(updated.published_at.is_some());
}

#[tokio::test]
async fn test_delete_article() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    let input = create_test_input("to-delete", "To Delete", user_id, category_id);
    let created = repo.create(&input).await.expect("Failed to create article");

    repo.delete(created.id)
        .await
        .expect("Failed to delete article");

    let found = repo
        .get_by_id(created.id)
        .await
        .expect("Failed to get article");
    assert!(found.is_none());
}

#[tokio::test]
async fn test_list_articles_by_category() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category1_id = create_test_category(sqlite_pool, "cat1").await;
    let category2_id = create_test_category(sqlite_pool, "cat2").await;

    // Create articles in different categories
    for i in 1..=3 {
        let input = create_test_input(
            &format!("cat1-article-{}", i),
            &format!("Cat1 Article {}", i),
            user_id,
            category1_id,
        );
        repo.create(&input).await.expect("Failed to create article");
    }
    for i in 1..=2 {
        let input = create_test_input(
            &format!("cat2-article-{}", i),
            &format!("Cat2 Article {}", i),
            user_id,
            category2_id,
        );
        repo.create(&input).await.expect("Failed to create article");
    }

    let cat1_articles = repo
        .list_by_category(category1_id, 0, 10, ArticleSortBy::default())
        .await
        .expect("Failed to list articles");
    assert_eq!(cat1_articles.len(), 3);

    let cat2_articles = repo
        .list_by_category(category2_id, 0, 10, ArticleSortBy::default())
        .await
        .expect("Failed to list articles");
    assert_eq!(cat2_articles.len(), 2);
}

#[tokio::test]
async fn test_list_articles_by_tag() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    // Create tag repository
    let tag_repo = SqlxTagRepository::new(pool.clone());

    // Create a tag
    let tag = Tag::new("rust".to_string(), "Rust".to_string());
    let created_tag = tag_repo.create(&tag).await.expect("Failed to create tag");

    // Create articles
    let input1 = create_test_input("article-1", "Article 1", user_id, category_id);
    let article1 = repo
        .create(&input1)
        .await
        .expect("Failed to create article");

    let input2 = create_test_input("article-2", "Article 2", user_id, category_id);
    let article2 = repo
        .create(&input2)
        .await
        .expect("Failed to create article");

    let input3 = create_test_input("article-3", "Article 3", user_id, category_id);
    repo.create(&input3)
        .await
        .expect("Failed to create article");

    // Associate tag with articles 1 and 2
    tag_repo
        .add_to_article(created_tag.id, article1.id)
        .await
        .expect("Failed to add tag");
    tag_repo
        .add_to_article(created_tag.id, article2.id)
        .await
        .expect("Failed to add tag");

    let tagged_articles = repo
        .list_by_tag(created_tag.id, 0, 10, ArticleSortBy::default())
        .await
        .expect("Failed to list articles");
    assert_eq!(tagged_articles.len(), 2);
}

#[tokio::test]
async fn test_list_published_articles() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    // Create draft articles
    for i in 1..=2 {
        let input = create_test_input(
            &format!("draft-{}", i),
            &format!("Draft {}", i),
            user_id,
            category_id,
        );
        repo.create(&input).await.expect("Failed to create article");
    }

    // Create published articles
    for i in 1..=3 {
        let mut input = create_test_input(
            &format!("published-{}", i),
            &format!("Published {}", i),
            user_id,
            category_id,
        );
        input.status = Some(ArticleStatus::Published);
        repo.create(&input).await.expect("Failed to create article");
    }

    let published = repo
        .list_published(0, 10, ArticleSortBy::default())
        .await
        .expect("Failed to list published articles");
    assert_eq!(published.len(), 3);

    // All should be published
    for article in &published {
        assert_eq!(article.status, ArticleStatus::Published);
    }
}

#[tokio::test]
async fn test_list_articles_by_status() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    let draft = create_test_input("draft-status", "Draft Status", user_id, category_id);
    repo.create(&draft).await.expect("Failed to create draft");

    let mut published =
        create_test_input("published-status", "Published Status", user_id, category_id);
    published.status = Some(ArticleStatus::Published);
    repo.create(&published)
        .await
        .expect("Failed to create published article");

    let mut archived =
        create_test_input("archived-status", "Archived Status", user_id, category_id);
    archived.status = Some(ArticleStatus::Archived);
    repo.create(&archived)
        .await
        .expect("Failed to create archived article");

    let drafts = repo
        .list_by_status(ArticleStatus::Draft, 0, 10, ArticleSortBy::default())
        .await
        .expect("Failed to list drafts");
    let archived = repo
        .list_by_status(ArticleStatus::Archived, 0, 10, ArticleSortBy::default())
        .await
        .expect("Failed to list archived articles");

    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].status, ArticleStatus::Draft);
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].status, ArticleStatus::Archived);
    assert_eq!(
        repo.count_by_status(ArticleStatus::Draft)
            .await
            .expect("Failed to count drafts"),
        1
    );
    assert_eq!(
        repo.count_by_status(ArticleStatus::Archived)
            .await
            .expect("Failed to count archived articles"),
        1
    );
}

#[tokio::test]
async fn test_update_article_scheduled_at_set_clear_and_publish() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;
    let scheduled_at = Utc::now() + chrono::Duration::hours(1);

    let mut input = create_test_input("scheduled", "Scheduled", user_id, category_id);
    input.scheduled_at = Some(scheduled_at);
    let created = repo.create(&input).await.expect("Failed to create article");
    assert_eq!(created.scheduled_at, Some(scheduled_at));

    let cleared = repo
        .update(
            created.id,
            &UpdateArticleInput::new().with_scheduled_at(None),
        )
        .await
        .expect("Failed to clear scheduled_at");
    assert!(cleared.scheduled_at.is_none());

    let rescheduled = repo
        .update(
            created.id,
            &UpdateArticleInput::new().with_scheduled_at(Some(scheduled_at)),
        )
        .await
        .expect("Failed to reschedule article");
    assert_eq!(rescheduled.scheduled_at, Some(scheduled_at));

    let published = repo
        .update(
            created.id,
            &UpdateArticleInput::new().with_status(ArticleStatus::Published),
        )
        .await
        .expect("Failed to publish scheduled article");
    assert_eq!(published.status, ArticleStatus::Published);
    assert!(published.scheduled_at.is_none());
}

#[tokio::test]
async fn test_list_published_ordered_by_published_at_desc() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    // Create published articles with small delays to ensure different timestamps
    for i in 1..=3 {
        let mut input = create_test_input(
            &format!("published-{}", i),
            &format!("Published {}", i),
            user_id,
            category_id,
        );
        input.status = Some(ArticleStatus::Published);
        repo.create(&input).await.expect("Failed to create article");
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    let published = repo
        .list_published(0, 10, ArticleSortBy::default())
        .await
        .expect("Failed to list published articles");
    assert_eq!(published.len(), 3);

    // Should be ordered by published_at DESC (newest first)
    for i in 0..published.len() - 1 {
        let current = published[i].published_at.unwrap();
        let next = published[i + 1].published_at.unwrap();
        assert!(
            current >= next,
            "Articles should be ordered by published_at DESC"
        );
    }
}

#[tokio::test]
async fn test_count_published() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    // Create draft articles
    for i in 1..=2 {
        let input = create_test_input(
            &format!("draft-{}", i),
            &format!("Draft {}", i),
            user_id,
            category_id,
        );
        repo.create(&input).await.expect("Failed to create article");
    }

    // Create published articles
    for i in 1..=3 {
        let mut input = create_test_input(
            &format!("published-{}", i),
            &format!("Published {}", i),
            user_id,
            category_id,
        );
        input.status = Some(ArticleStatus::Published);
        repo.create(&input).await.expect("Failed to create article");
    }

    let count = repo
        .count_published()
        .await
        .expect("Failed to count published");
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_count_by_category() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category1_id = create_test_category(sqlite_pool, "cat1").await;
    let category2_id = create_test_category(sqlite_pool, "cat2").await;

    // Create articles in different categories
    for i in 1..=3 {
        let input = create_test_input(
            &format!("cat1-article-{}", i),
            &format!("Cat1 Article {}", i),
            user_id,
            category1_id,
        );
        repo.create(&input).await.expect("Failed to create article");
    }
    for i in 1..=2 {
        let input = create_test_input(
            &format!("cat2-article-{}", i),
            &format!("Cat2 Article {}", i),
            user_id,
            category2_id,
        );
        repo.create(&input).await.expect("Failed to create article");
    }

    let count1 = repo
        .count_by_category(category1_id)
        .await
        .expect("Failed to count");
    assert_eq!(count1, 3);

    let count2 = repo
        .count_by_category(category2_id)
        .await
        .expect("Failed to count");
    assert_eq!(count2, 2);
}

#[tokio::test]
async fn test_count_by_tag() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    // Create tag repository
    let tag_repo = SqlxTagRepository::new(pool.clone());

    // Create a tag
    let tag = Tag::new("rust".to_string(), "Rust".to_string());
    let created_tag = tag_repo.create(&tag).await.expect("Failed to create tag");

    // Create articles and associate with tag
    for i in 1..=3 {
        let input = create_test_input(
            &format!("article-{}", i),
            &format!("Article {}", i),
            user_id,
            category_id,
        );
        let article = repo.create(&input).await.expect("Failed to create article");
        tag_repo
            .add_to_article(created_tag.id, article.id)
            .await
            .expect("Failed to add tag");
    }

    let count = repo
        .count_by_tag(created_tag.id)
        .await
        .expect("Failed to count");
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_exists_by_slug() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    // Initially doesn't exist
    let exists = repo
        .exists_by_slug("test-slug")
        .await
        .expect("Failed to check");
    assert!(!exists);

    // Create article
    let input = create_test_input("test-slug", "Test Slug", user_id, category_id);
    repo.create(&input).await.expect("Failed to create article");

    // Now exists
    let exists = repo
        .exists_by_slug("test-slug")
        .await
        .expect("Failed to check");
    assert!(exists);
}

#[tokio::test]
async fn test_exists_by_slug_excluding() {
    let (pool, repo) = setup_test_repo().await;
    let sqlite_pool = pool.as_sqlite().unwrap();
    let user_id = create_test_user(sqlite_pool).await;
    let category_id = create_test_category(sqlite_pool, "test-cat").await;

    // Create two articles
    let input1 = create_test_input("slug-1", "Article 1", user_id, category_id);
    let article1 = repo
        .create(&input1)
        .await
        .expect("Failed to create article");

    let input2 = create_test_input("slug-2", "Article 2", user_id, category_id);
    let article2 = repo
        .create(&input2)
        .await
        .expect("Failed to create article");

    // slug-1 exists when excluding article2
    let exists = repo
        .exists_by_slug_excluding("slug-1", article2.id)
        .await
        .expect("Failed to check");
    assert!(exists);

    // slug-1 doesn't exist when excluding article1 (itself)
    let exists = repo
        .exists_by_slug_excluding("slug-1", article1.id)
        .await
        .expect("Failed to check");
    assert!(!exists);
}

#[tokio::test]
async fn test_article_status_conversion() {
    assert_eq!(ArticleStatus::Draft.as_str(), "draft");
    assert_eq!(ArticleStatus::Published.as_str(), "published");
    assert_eq!(ArticleStatus::Archived.as_str(), "archived");

    assert_eq!(ArticleStatus::from_str("draft"), Some(ArticleStatus::Draft));
    assert_eq!(
        ArticleStatus::from_str("published"),
        Some(ArticleStatus::Published)
    );
    assert_eq!(
        ArticleStatus::from_str("archived"),
        Some(ArticleStatus::Archived)
    );
    assert_eq!(ArticleStatus::from_str("DRAFT"), Some(ArticleStatus::Draft)); // Case insensitive
    assert_eq!(ArticleStatus::from_str("invalid"), None);
}

#[tokio::test]
async fn test_list_params() {
    let params = ListParams::new(1, 10);
    assert_eq!(params.offset(), 0);
    assert_eq!(params.limit(), 10);

    let params = ListParams::new(2, 10);
    assert_eq!(params.offset(), 10);

    let params = ListParams::new(3, 5);
    assert_eq!(params.offset(), 10);
    assert_eq!(params.limit(), 5);

    // Edge cases
    let params = ListParams::new(0, 10); // Page 0 should become 1
    assert_eq!(params.page, 1);
    assert_eq!(params.offset(), 0);

    let params = ListParams::new(1, 200); // per_page clamped to 100
    assert_eq!(params.per_page, 100);
}

#[tokio::test]
async fn test_paged_result() {
    let params = ListParams::new(1, 10);
    let items = vec![1, 2, 3, 4, 5];
    let result = PagedResult::new(items, 25, &params);

    assert_eq!(result.len(), 5);
    assert_eq!(result.total, 25);
    assert_eq!(result.page, 1);
    assert_eq!(result.per_page, 10);
    assert_eq!(result.total_pages(), 3);
    assert!(result.has_next());
    assert!(!result.has_prev());

    let params = ListParams::new(2, 10);
    let items = vec![6, 7, 8, 9, 10];
    let result = PagedResult::new(items, 25, &params);
    assert!(result.has_next());
    assert!(result.has_prev());

    let params = ListParams::new(3, 10);
    let items = vec![21, 22, 23, 24, 25];
    let result = PagedResult::new(items, 25, &params);
    assert!(!result.has_next());
    assert!(result.has_prev());
}
