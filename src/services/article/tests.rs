    use super::*;
    use crate::cache::create_cache;
    use crate::config::CacheConfig;
    use crate::db::repositories::{SqlxArticleRepository, SqlxTagRepository};
    use crate::db::{create_test_pool, migrations, DynDatabasePool};
    use crate::models::ArticleStatus;

    async fn setup_test_service() -> (DynDatabasePool, ArticleService) {
        let pool = create_test_pool()
            .await
            .expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let article_repo = SqlxArticleRepository::boxed(pool.clone());
        let tag_repo = SqlxTagRepository::boxed(pool.clone());
        let cache = create_cache(&CacheConfig::default())
            .await
            .expect("Failed to create cache");
        let markdown_renderer = MarkdownRenderer::new();

        let service = ArticleService::new(article_repo, tag_repo, cache, markdown_renderer);

        (pool, service)
    }

    /// Helper to create a test user
    async fn create_test_user(pool: &sqlx::SqlitePool) -> i64 {
        let result = sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind("testuser")
        .bind("test@example.com")
        .bind("hash123")
        .bind("author")
        .execute(pool)
        .await
        .expect("Failed to create test user");
        result.last_insert_rowid()
    }

    // ========================================================================
    // Slug generation tests
    // ========================================================================

    #[test]
    fn test_generate_slug_simple() {
        assert_eq!(generate_slug("Hello World"), "hello-world");
    }

    #[test]
    fn test_generate_slug_with_special_chars() {
        assert_eq!(generate_slug("Hello, World!"), "hello-world");
    }

    #[test]
    fn test_generate_slug_with_multiple_spaces() {
        assert_eq!(generate_slug("Hello   World"), "hello-world");
    }

    #[test]
    fn test_generate_slug_with_underscores() {
        assert_eq!(generate_slug("hello_world"), "hello-world");
    }

    #[test]
    fn test_generate_slug_chinese() {
        // Chinese characters should be preserved
        let slug = generate_slug("技术文章");
        assert_eq!(slug, "技术文章");
    }

    #[test]
    fn test_generate_slug_mixed() {
        let slug = generate_slug("Tech 技术");
        assert_eq!(slug, "tech-技术");
    }

    // ========================================================================
    // Validation tests (Requirement 1.7)
    // ========================================================================

    #[tokio::test]
    async fn test_create_article_empty_title_fails() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "test-slug".to_string(),
            "".to_string(), // Empty title
            "Some content".to_string(),
            author_id,
            1,
        );

        let result = service.create(input, None).await;
        assert!(matches!(
            result,
            Err(ArticleServiceError::ValidationError(_))
        ));
    }

    #[tokio::test]
    async fn test_create_article_whitespace_title_fails() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "test-slug".to_string(),
            "   ".to_string(), // Whitespace-only title
            "Some content".to_string(),
            author_id,
            1,
        );

        let result = service.create(input, None).await;
        assert!(matches!(
            result,
            Err(ArticleServiceError::ValidationError(_))
        ));
    }

    #[tokio::test]
    async fn test_create_article_empty_content_fails() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "test-slug".to_string(),
            "Valid Title".to_string(),
            "".to_string(), // Empty content
            author_id,
            1,
        );

        let result = service.create(input, None).await;
        assert!(matches!(
            result,
            Err(ArticleServiceError::ValidationError(_))
        ));
    }

    #[tokio::test]
    async fn test_create_article_whitespace_content_fails() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "test-slug".to_string(),
            "Valid Title".to_string(),
            "   \n\t  ".to_string(), // Whitespace-only content
            author_id,
            1,
        );

        let result = service.create(input, None).await;
        assert!(matches!(
            result,
            Err(ArticleServiceError::ValidationError(_))
        ));
    }

    // ========================================================================
    // Create article tests (Requirement 1.1)
    // ========================================================================

    #[tokio::test]
    async fn test_create_article_success() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "my-first-article".to_string(),
            "My First Article".to_string(),
            "# Hello\n\nThis is my first article.".to_string(),
            author_id,
            1,
        );

        let article = service
            .create(input, None)
            .await
            .expect("Failed to create article");

        assert!(article.id > 0);
        assert_eq!(article.title, "My First Article");
        assert_eq!(article.slug, "my-first-article");
        assert!(article.content_html.contains("<h1>"));
        assert_eq!(article.status, ArticleStatus::Draft);
    }

    #[tokio::test]
    async fn test_create_article_generates_slug_from_title() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "".to_string(), // Empty slug - should be generated
            "Auto Generated Slug".to_string(),
            "Content here".to_string(),
            author_id,
            1,
        );

        let article = service
            .create(input, None)
            .await
            .expect("Failed to create article");

        assert_eq!(article.slug, "auto-generated-slug");
    }

    #[tokio::test]
    async fn test_create_article_duplicate_slug_fails() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        // Create first article
        let input1 = CreateArticleInput::new(
            "duplicate-slug".to_string(),
            "First Article".to_string(),
            "Content".to_string(),
            author_id,
            1,
        );
        service
            .create(input1, None)
            .await
            .expect("Failed to create first article");

        // Try to create second article with same slug
        let input2 = CreateArticleInput::new(
            "duplicate-slug".to_string(),
            "Second Article".to_string(),
            "Content".to_string(),
            author_id,
            1,
        );
        let result = service.create(input2, None).await;

        assert!(matches!(result, Err(ArticleServiceError::DuplicateSlug(_))));
    }

    #[tokio::test]
    async fn test_create_article_with_published_status() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "published-article".to_string(),
            "Published Article".to_string(),
            "Content".to_string(),
            author_id,
            1,
        )
        .with_status(ArticleStatus::Published);

        let article = service
            .create(input, None)
            .await
            .expect("Failed to create article");

        assert_eq!(article.status, ArticleStatus::Published);
        assert!(article.published_at.is_some());
    }

    // ========================================================================
    // Get article tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_by_id_success() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "get-by-id-test".to_string(),
            "Get By ID Test".to_string(),
            "Content".to_string(),
            author_id,
            1,
        );
        let created = service
            .create(input, None)
            .await
            .expect("Failed to create article");

        let found = service
            .get_by_id(created.id)
            .await
            .expect("Failed to get article")
            .expect("Article not found");

        assert_eq!(found.id, created.id);
        assert_eq!(found.title, "Get By ID Test");
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let (_pool, service) = setup_test_service().await;

        let result = service
            .get_by_id(99999)
            .await
            .expect("Failed to get article");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_slug_success() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "get-by-slug-test".to_string(),
            "Get By Slug Test".to_string(),
            "Content".to_string(),
            author_id,
            1,
        );
        service
            .create(input, None)
            .await
            .expect("Failed to create article");

        let found = service
            .get_by_slug("get-by-slug-test")
            .await
            .expect("Failed to get article")
            .expect("Article not found");

        assert_eq!(found.slug, "get-by-slug-test");
    }

    #[tokio::test]
    async fn test_get_by_slug_not_found() {
        let (_pool, service) = setup_test_service().await;

        let result = service
            .get_by_slug("nonexistent")
            .await
            .expect("Failed to get article");
        assert!(result.is_none());
    }

    // ========================================================================
    // List articles tests (Requirement 1.2)
    // ========================================================================

    #[tokio::test]
    async fn test_list_articles() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        // Create some articles
        for i in 1..=5 {
            let input = CreateArticleInput::new(
                format!("article-{}", i),
                format!("Article {}", i),
                "Content".to_string(),
                author_id,
                1,
            );
            service
                .create(input, None)
                .await
                .expect("Failed to create article");
        }

        let params = ListParams::new(1, 10);
        let result = service.list(&params).await.expect("Failed to list articles");

        assert_eq!(result.total, 5);
        assert_eq!(result.items.len(), 5);
    }

    #[tokio::test]
    async fn test_list_articles_pagination() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        // Create 10 articles
        for i in 1..=10 {
            let input = CreateArticleInput::new(
                format!("article-{}", i),
                format!("Article {}", i),
                "Content".to_string(),
                author_id,
                1,
            );
            service
                .create(input, None)
                .await
                .expect("Failed to create article");
        }

        // Get first page
        let params = ListParams::new(1, 3);
        let result = service.list(&params).await.expect("Failed to list articles");

        assert_eq!(result.total, 10);
        assert_eq!(result.items.len(), 3);
        assert_eq!(result.page, 1);
        assert!(result.has_next());
    }

    #[tokio::test]
    async fn test_list_published_articles() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        // Create mix of draft and published articles
        for i in 1..=3 {
            let input = CreateArticleInput::new(
                format!("draft-{}", i),
                format!("Draft {}", i),
                "Content".to_string(),
                author_id,
                1,
            );
            service
                .create(input, None)
                .await
                .expect("Failed to create draft");
        }

        for i in 1..=2 {
            let input = CreateArticleInput::new(
                format!("published-{}", i),
                format!("Published {}", i),
                "Content".to_string(),
                author_id,
                1,
            )
            .with_status(ArticleStatus::Published);
            service
                .create(input, None)
                .await
                .expect("Failed to create published");
        }

        let params = ListParams::new(1, 10);
        let result = service
            .list_published(&params)
            .await
            .expect("Failed to list published articles");

        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);
        for article in &result.items {
            assert_eq!(article.status, ArticleStatus::Published);
        }
    }

    // ========================================================================
    // Update article tests (Requirement 1.3)
    // ========================================================================

    #[tokio::test]
    async fn test_update_article_success() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "update-test".to_string(),
            "Original Title".to_string(),
            "Original content".to_string(),
            author_id,
            1,
        );
        let created = service
            .create(input, None)
            .await
            .expect("Failed to create article");

        let update_input = UpdateArticleInput::new()
            .with_title("Updated Title".to_string())
            .with_content("Updated content".to_string());

        let updated = service
            .update(created.id, update_input, None)
            .await
            .expect("Failed to update article");

        assert_eq!(updated.title, "Updated Title");
        assert!(updated.content_html.contains("Updated content"));
    }

    #[tokio::test]
    async fn test_update_article_not_found() {
        let (_pool, service) = setup_test_service().await;

        let update_input = UpdateArticleInput::new().with_title("New Title".to_string());

        let result = service.update(99999, update_input, None).await;
        assert!(matches!(result, Err(ArticleServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_update_article_empty_title_fails() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "update-validation-test".to_string(),
            "Original Title".to_string(),
            "Content".to_string(),
            author_id,
            1,
        );
        let created = service
            .create(input, None)
            .await
            .expect("Failed to create article");

        let update_input = UpdateArticleInput::new().with_title("".to_string());

        let result = service.update(created.id, update_input, None).await;
        assert!(matches!(
            result,
            Err(ArticleServiceError::ValidationError(_))
        ));
    }

    #[tokio::test]
    async fn test_update_article_duplicate_slug_fails() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        // Create two articles
        let input1 = CreateArticleInput::new(
            "first-article".to_string(),
            "First Article".to_string(),
            "Content".to_string(),
            author_id,
            1,
        );
        service
            .create(input1, None)
            .await
            .expect("Failed to create first article");

        let input2 = CreateArticleInput::new(
            "second-article".to_string(),
            "Second Article".to_string(),
            "Content".to_string(),
            author_id,
            1,
        );
        let second = service
            .create(input2, None)
            .await
            .expect("Failed to create second article");

        // Try to update second article with first article's slug
        let update_input = UpdateArticleInput::new().with_slug("first-article".to_string());

        let result = service.update(second.id, update_input, None).await;
        assert!(matches!(result, Err(ArticleServiceError::DuplicateSlug(_))));
    }

    // ========================================================================
    // Delete article tests (Requirement 1.4)
    // ========================================================================

    #[tokio::test]
    async fn test_delete_article_success() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "to-delete".to_string(),
            "To Delete".to_string(),
            "Content".to_string(),
            author_id,
            1,
        );
        let created = service
            .create(input, None)
            .await
            .expect("Failed to create article");

        service
            .delete(created.id)
            .await
            .expect("Failed to delete article");

        let found = service
            .get_by_id(created.id)
            .await
            .expect("Failed to get article");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_article_not_found() {
        let (_pool, service) = setup_test_service().await;

        let result = service.delete(99999).await;
        assert!(matches!(result, Err(ArticleServiceError::NotFound(_))));
    }

    // ========================================================================
    // Markdown rendering tests (Requirement 1.6)
    // ========================================================================

    #[tokio::test]
    async fn test_render_markdown() {
        let (_pool, service) = setup_test_service().await;

        let markdown = "# Hello\n\nThis is **bold** and *italic*.";
        let html = service.render_markdown(markdown);

        assert!(html.contains("<h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[tokio::test]
    async fn test_create_article_renders_markdown() {
        let (pool, service) = setup_test_service().await;
        let sqlite_pool = pool.as_sqlite().unwrap();
        let author_id = create_test_user(sqlite_pool).await;

        let input = CreateArticleInput::new(
            "markdown-test".to_string(),
            "Markdown Test".to_string(),
            "# Heading\n\n- Item 1\n- Item 2".to_string(),
            author_id,
            1,
        );

        let article = service
            .create(input, None)
            .await
            .expect("Failed to create article");

        assert!(article.content_html.contains("<h1>"));
        assert!(article.content_html.contains("<ul>"));
        assert!(article.content_html.contains("<li>"));
    }

    // ========================================================================
    // Property-Based Tests
    // ========================================================================

    use proptest::prelude::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Counter for generating unique test data across property test iterations
    static PROPERTY_TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Generate a unique suffix for test data
    fn unique_suffix() -> u64 {
        PROPERTY_TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// Setup a fresh test service for property tests
    async fn setup_property_test_service() -> (DynDatabasePool, ArticleService) {
        let pool = create_test_pool()
            .await
            .expect("Failed to create test pool");
        migrations::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let article_repo = SqlxArticleRepository::boxed(pool.clone());
        let tag_repo = SqlxTagRepository::boxed(pool.clone());
        let cache = create_cache(&CacheConfig::default())
            .await
            .expect("Failed to create cache");
        let markdown_renderer = MarkdownRenderer::new();

        let service = ArticleService::new(article_repo, tag_repo, cache, markdown_renderer);

        (pool, service)
    }

    /// Helper to create a test user for property tests
    async fn create_property_test_user(pool: &sqlx::SqlitePool, suffix: u64) -> i64 {
        let result = sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind(format!("testuser_{}", suffix))
        .bind(format!("test_{}@example.com", suffix))
        .bind("hash123")
        .bind("author")
        .execute(pool)
        .await
        .expect("Failed to create test user");
        result.last_insert_rowid()
    }

    // ========================================================================
    // Property 1: 文章 CRUD 往返一致�?(Article CRUD Round-trip Consistency)
    // For any valid article data, creating an article and then retrieving it
    // by ID should return the same data (title, content, category, etc.).
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 1: 文章 CRUD 往返一致�?
        /// **Validates: Requirements 1.1, 1.3**
        ///
        /// Property 1: Article CRUD Round-trip Consistency
        /// For any valid article data, creating an article and then retrieving it
        /// by ID should return the same data (title, content, category, etc.).
        #[test]
        fn property_1_article_crud_roundtrip(
            title_base in "[a-zA-Z]{5,30}",
            content_base in "[a-zA-Z0-9 ]{10,100}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let (pool, service) = setup_property_test_service().await;
                let suffix = unique_suffix();
                let sqlite_pool = pool.as_sqlite().unwrap();

                // Create a test user
                let author_id = create_property_test_user(sqlite_pool, suffix).await;

                // Create unique article data
                let title = format!("{}_{}", title_base, suffix);
                let content = format!("{}_{}", content_base, suffix);
                let slug = format!("article-{}", suffix);

                // Create article
                let input = CreateArticleInput::new(
                    slug.clone(),
                    title.clone(),
                    content.clone(),
                    author_id,
                    1, // Default category
                );

                let created = service.create(input, None).await
                    .expect("create should succeed");

                // Property: Created article should have a valid ID
                prop_assert!(created.id > 0, "Article ID should be positive");

                // Retrieve article by ID
                let retrieved = service.get_by_id(created.id).await
                    .expect("get_by_id should succeed")
                    .expect("Article should exist");

                // Property: Retrieved article should match created article
                prop_assert_eq!(
                    retrieved.id, created.id,
                    "Article ID should match"
                );
                prop_assert_eq!(
                    &retrieved.title, &title,
                    "Article title should match. Expected '{}', got '{}'",
                    title, retrieved.title
                );
                prop_assert_eq!(
                    &retrieved.content, &content,
                    "Article content should match"
                );
                prop_assert_eq!(
                    &retrieved.slug, &slug,
                    "Article slug should match"
                );
                prop_assert_eq!(
                    retrieved.author_id, author_id,
                    "Article author_id should match"
                );
                prop_assert_eq!(
                    retrieved.category_id, 1,
                    "Article category_id should match"
                );

                // Property: content_html should be generated
                prop_assert!(
                    !retrieved.content_html.is_empty(),
                    "Article content_html should not be empty"
                );

                Ok(())
            });
            result?;
        }
    }

    // ========================================================================
    // Property 2: 文章列表分页排序 (Article List Pagination and Sorting)
    // For any set of articles and pagination parameters, the returned list
    // should satisfy: count <= per_page, sorted by published_at DESC, correct total.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 2: 文章列表分页排序
        /// **Validates: Requirements 1.2**
        ///
        /// Property 2: Article List Pagination and Sorting
        /// For any set of articles and pagination parameters, the returned list
        /// should satisfy: count <= per_page, sorted by created_at DESC, correct total.
        #[test]
        fn property_2_article_list_pagination_sorting(
            article_count in 1..10usize,
            page in 1..5u32,
            per_page in 1..10u32
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let (pool, service) = setup_property_test_service().await;
                let suffix = unique_suffix();
                let sqlite_pool = pool.as_sqlite().unwrap();

                // Create a test user
                let author_id = create_property_test_user(sqlite_pool, suffix).await;

                // Create multiple articles
                for i in 0..article_count {
                    let input = CreateArticleInput::new(
                        format!("article-{}-{}", suffix, i),
                        format!("Article {} {}", suffix, i),
                        format!("Content for article {}", i),
                        author_id,
                        1,
                    );
                    service.create(input, None).await
                        .expect("create should succeed");
                    
                    // Small delay to ensure different timestamps
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }

                // List articles with pagination
                let params = ListParams::new(page, per_page);
                let result = service.list(&params).await
                    .expect("list should succeed");

                // Property: Total count should match the number of articles created
                prop_assert_eq!(
                    result.total as usize, article_count,
                    "Total count should match. Expected {}, got {}",
                    article_count, result.total
                );

                // Property: Items count should not exceed per_page
                prop_assert!(
                    result.items.len() <= per_page as usize,
                    "Items count ({}) should not exceed per_page ({})",
                    result.items.len(), per_page
                );

                // Property: Items should be sorted by created_at DESC
                for i in 1..result.items.len() {
                    prop_assert!(
                        result.items[i - 1].created_at >= result.items[i].created_at,
                        "Articles should be sorted by created_at DESC. \
                         Article at index {} has created_at {:?} but comes before \
                         article at index {} with created_at {:?}",
                        i - 1, result.items[i - 1].created_at,
                        i, result.items[i].created_at
                    );
                }

                // Property: Page number should be correct
                prop_assert_eq!(
                    result.page, page,
                    "Page number should match"
                );

                // Property: Per page should be correct
                prop_assert_eq!(
                    result.per_page, per_page,
                    "Per page should match"
                );

                Ok(())
            });
            result?;
        }
    }

    // ========================================================================
    // Property 3: 空内容文章验�?(Empty Content Article Validation)
    // For any article input with empty or whitespace-only title or content,
    // the system should reject creation and return a validation error.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 3: 空内容文章验�?
        /// **Validates: Requirements 1.7**
        ///
        /// Property 3: Empty Content Article Validation
        /// For any article input with empty or whitespace-only title or content,
        /// the system should reject creation and return a validation error.
        #[test]
        fn property_3_empty_content_validation(
            whitespace_type in prop_oneof![
                Just(""),
                Just(" "),
                Just("  "),
                Just("\t"),
                Just("\n"),
                Just("   \t\n  ")
            ],
            valid_text in "[a-zA-Z]{5,20}",
            empty_field in prop_oneof![Just("title"), Just("content"), Just("both")]
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let (pool, service) = setup_property_test_service().await;
                let suffix = unique_suffix();
                let sqlite_pool = pool.as_sqlite().unwrap();

                // Create a test user
                let author_id = create_property_test_user(sqlite_pool, suffix).await;

                // Determine title and content based on which field should be empty
                let (title, content) = match empty_field.as_ref() {
                    "title" => (whitespace_type.to_string(), format!("{}_{}", valid_text, suffix)),
                    "content" => (format!("{}_{}", valid_text, suffix), whitespace_type.to_string()),
                    "both" => (whitespace_type.to_string(), whitespace_type.to_string()),
                    _ => unreachable!(),
                };

                let input = CreateArticleInput::new(
                    format!("slug-{}", suffix),
                    title,
                    content,
                    author_id,
                    1,
                );

                let result = service.create(input, None).await;

                // Property: Creation should fail with ValidationError
                prop_assert!(
                    matches!(result, Err(ArticleServiceError::ValidationError(_))),
                    "Creating article with empty {} should return ValidationError. Got: {:?}",
                    empty_field, result
                );

                Ok(())
            });
            result?;
        }
    }

    // ========================================================================
    // Property 4: Markdown 渲染一致�?(Markdown Rendering Consistency)
    // For any valid Markdown text, the rendered HTML should contain
    // corresponding structural elements (headings -> h1-h6, lists -> ul/ol,
    // code blocks -> pre/code).
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 4: Markdown 渲染一致�?
        /// **Validates: Requirements 1.6**
        ///
        /// Property 4: Markdown Rendering Consistency
        /// For any valid Markdown text, the rendered HTML should contain
        /// corresponding structural elements.
        #[test]
        fn property_4_markdown_rendering_consistency(
            heading_level in 1..7u8,
            heading_text in "[a-zA-Z]{3,15}",
            list_items in proptest::collection::vec("[a-zA-Z]{3,10}", 1..5),
            use_ordered_list in proptest::bool::ANY,
            code_content in "[a-zA-Z0-9]{5,20}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let (_pool, service) = setup_property_test_service().await;

                // Build markdown with heading
                let heading_prefix = "#".repeat(heading_level as usize);
                let heading_md = format!("{} {}\n\n", heading_prefix, heading_text);

                // Build markdown with list
                let list_md = if use_ordered_list {
                    list_items.iter()
                        .enumerate()
                        .map(|(i, item)| format!("{}. {}", i + 1, item))
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    list_items.iter()
                        .map(|item| format!("- {}", item))
                        .collect::<Vec<_>>()
                        .join("\n")
                };

                // Build markdown with code block
                let code_md = format!("\n\n```\n{}\n```", code_content);

                // Combine all markdown
                let full_markdown = format!("{}{}{}", heading_md, list_md, code_md);

                // Render markdown
                let html = service.render_markdown(&full_markdown);

                // Property: HTML should contain the appropriate heading tag
                let expected_heading_tag = format!("<h{}>", heading_level);
                prop_assert!(
                    html.contains(&expected_heading_tag),
                    "HTML should contain {} for heading level {}. HTML: {}",
                    expected_heading_tag, heading_level, html
                );

                // Property: HTML should contain the heading text
                prop_assert!(
                    html.contains(&heading_text),
                    "HTML should contain heading text '{}'. HTML: {}",
                    heading_text, html
                );

                // Property: HTML should contain the appropriate list tag
                if use_ordered_list {
                    prop_assert!(
                        html.contains("<ol>"),
                        "HTML should contain <ol> for ordered list. HTML: {}",
                        html
                    );
                } else {
                    prop_assert!(
                        html.contains("<ul>"),
                        "HTML should contain <ul> for unordered list. HTML: {}",
                        html
                    );
                }

                // Property: HTML should contain list items
                prop_assert!(
                    html.contains("<li>"),
                    "HTML should contain <li> for list items. HTML: {}",
                    html
                );

                // Property: HTML should contain code block elements
                prop_assert!(
                    html.contains("<pre>") || html.contains("<pre "),
                    "HTML should contain <pre> for code block. HTML: {}",
                    html
                );
                prop_assert!(
                    html.contains("<code>") || html.contains("<code "),
                    "HTML should contain <code> for code block. HTML: {}",
                    html
                );

                Ok(())
            });
            result?;
        }
    }

    // ========================================================================
    // Property 22: 缓存失效一致�?(Cache Invalidation Consistency)
    // For any data modification operation (create, update, delete),
    // related cache entries should be invalidated, and subsequent queries
    // should return the latest data.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 22: 缓存失效一致�?
        /// **Validates: Requirements 1.5, 9.3**
        ///
        /// Property 22: Cache Invalidation Consistency
        /// For any data modification operation (create, update, delete),
        /// related cache entries should be invalidated, and subsequent queries
        /// should return the latest data.
        #[test]
        fn property_22_cache_invalidation_consistency(
            original_title in "[a-zA-Z]{5,20}",
            updated_title in "[a-zA-Z]{5,20}",
            original_content in "[a-zA-Z0-9 ]{10,50}",
            updated_content in "[a-zA-Z0-9 ]{10,50}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let (pool, service) = setup_property_test_service().await;
                let suffix = unique_suffix();
                let sqlite_pool = pool.as_sqlite().unwrap();

                // Create a test user
                let author_id = create_property_test_user(sqlite_pool, suffix).await;

                // Create unique article data
                let title = format!("{}_{}", original_title, suffix);
                let content = format!("{}_{}", original_content, suffix);
                let slug = format!("cache-test-{}", suffix);

                // Create article
                let input = CreateArticleInput::new(
                    slug.clone(),
                    title.clone(),
                    content.clone(),
                    author_id,
                    1,
                );

                let created = service.create(input, None).await
                    .expect("create should succeed");

                // First get - should populate cache
                let first_get = service.get_by_id(created.id).await
                    .expect("get_by_id should succeed")
                    .expect("Article should exist");

                prop_assert_eq!(
                    first_get.title, title,
                    "First get should return original title"
                );

                // Update article
                let new_title = format!("{}_{}", updated_title, suffix);
                let new_content = format!("{}_{}", updated_content, suffix);
                let update_input = UpdateArticleInput::new()
                    .with_title(new_title.clone())
                    .with_content(new_content.clone());

                let updated = service.update(created.id, update_input, None).await
                    .expect("update should succeed");

                prop_assert_eq!(
                    &updated.title, &new_title,
                    "Update should return new title"
                );

                // Second get - should return updated data (cache should be invalidated)
                let second_get = service.get_by_id(created.id).await
                    .expect("get_by_id should succeed")
                    .expect("Article should exist");

                // Property: After update, get should return the updated data
                prop_assert_eq!(
                    &second_get.title, &new_title,
                    "After update, get should return new title. Expected '{}', got '{}'",
                    new_title, second_get.title
                );
                prop_assert_eq!(
                    &second_get.content, &new_content,
                    "After update, get should return new content"
                );

                // Property: content_html should be re-rendered
                prop_assert!(
                    second_get.content_html.contains(new_content.as_str()) || !second_get.content_html.is_empty(),
                    "After update, content_html should be updated"
                );

                // Test cache invalidation on delete
                service.delete(created.id).await
                    .expect("delete should succeed");

                // After delete, get should return None
                let after_delete = service.get_by_id(created.id).await
                    .expect("get_by_id should succeed");

                prop_assert!(
                    after_delete.is_none(),
                    "After delete, get should return None"
                );

                Ok(())
            });
            result?;
        }
    }