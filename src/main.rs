//! Noteva - A lightweight modern blog system

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use noteva::{
    api::{self, AppState, middleware::RequestStats},
    cache::create_cache,
    config::Config,
    db::{
        self,
        repositories::{
            SqlxCommentRepository, SqlxArticleRepository, SqlxCategoryRepository,
            SqlxNavItemRepository, SqlxPageRepository, SqlxSessionRepository,
            SqlxSettingsRepository, SqlxTagRepository, SqlxUserRepository,
            SettingsRepository,
        },
    },
    plugin::{PluginManager, HookManager, ShortcodeManager, shortcode::builtins, hook_registry::HookRegistry},
    services::{
        article::ArticleService,
        category::CategoryService,
        comment::CommentService,
        markdown::MarkdownRenderer,
        nav_item::NavItemService,
        page::PageService,
        settings::SettingsService,
        tag::TagService,
        user::UserService,
    },
    theme::ThemeEngine,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "noteva=info,tower_http=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Noteva blog system...");

    // Load configuration
    let config = Config::load(Path::new("config.yml"))?;
    tracing::debug!("Configuration loaded");

    // Initialize database
    let pool = db::create_pool(&config.database).await?;
    tracing::info!("Database connected: {:?}", config.database.driver);

    // Run migrations
    db::migrations::run_migrations(&pool).await?;
    tracing::debug!("Database migrations completed");

    // Initialize cache
    let cache = create_cache(&config.cache).await?;
    tracing::debug!("Cache initialized");

    // Initialize plugin system (before services, so shortcodes are available)
    let mut plugin_manager = PluginManager::new(Path::new("plugins"), Path::new("data"), pool.clone());
    if let Err(e) = plugin_manager.init().await {
        tracing::warn!("Failed to initialize plugins: {}", e);
    }
    
    let hook_manager = Arc::new(HookManager::new(HookRegistry::load_embedded()));
    
    let mut shortcode_manager = ShortcodeManager::new();
    builtins::register_builtins(&mut shortcode_manager);
    let shortcode_manager_arc = Arc::new(shortcode_manager);
    tracing::debug!("Plugin system initialized");

    // Create markdown renderer with shortcode and hook support
    let markdown_renderer = MarkdownRenderer::with_managers(
        shortcode_manager_arc.clone(),
        hook_manager.clone(),
    );

    // Create repositories
    let user_repo = SqlxUserRepository::boxed(pool.clone());
    let session_repo = Arc::new(SqlxSessionRepository::new(pool.clone()));
    let category_repo = Arc::new(SqlxCategoryRepository::new(pool.clone()));
    let tag_repo = Arc::new(SqlxTagRepository::new(pool.clone()));
    let article_repo = Arc::new(SqlxArticleRepository::new(pool.clone()));
    let settings_repo = SqlxSettingsRepository::new(pool.clone());
    let page_repo = SqlxPageRepository::boxed(pool.clone());
    let nav_repo = SqlxNavItemRepository::boxed(pool.clone());

    // Initialize services with hook support
    let user_service = Arc::new(UserService::new(user_repo.clone(), session_repo));
    let category_service = Arc::new(CategoryService::new(
        category_repo,
        cache.clone(),
        pool.clone(),
    ));
    let tag_service = Arc::new(TagService::new(tag_repo.clone(), cache.clone()));
    let settings_service = Arc::new(SettingsService::from_sqlx(settings_repo));
    let article_service = Arc::new(ArticleService::with_hooks(
        article_repo,
        tag_repo,
        cache.clone(),
        markdown_renderer,
        hook_manager.clone(),
    ));
    let page_service = Arc::new(PageService::new(page_repo, cache.clone()));
    let nav_service = Arc::new(NavItemService::new(nav_repo, cache.clone()));

    // Create comment service with hooks and settings support
    let comment_repo = Arc::new(SqlxCommentRepository::new(pool.clone()));
    let settings_repo_for_comment = Arc::new(SqlxSettingsRepository::new(pool.clone()));
    let comment_service = Arc::new(
        CommentService::with_hooks(comment_repo, cache.clone(), hook_manager.clone())
            .with_settings(settings_repo_for_comment),
    );

    // Initialize default navigation items
    nav_service.init_defaults().await?;
    tracing::debug!("Navigation initialized");

    // Initialize theme engine - read active theme from database
    let active_theme = {
        let settings_repo_for_theme = SqlxSettingsRepository::new(pool.clone());
        settings_repo_for_theme
            .get("active_theme")
            .await
            .ok()
            .flatten()
            .map(|s| s.value)
            .unwrap_or_else(|| config.theme.active.clone())
    };
    let mut theme_engine = ThemeEngine::new(&config.theme.path, "default")?;
    // If active theme is not default, switch to it
    if active_theme != "default" {
        let result = theme_engine.set_theme_with_fallback(&active_theme);
        if result.used_fallback {
            tracing::warn!("Active theme '{}' not available, using default", active_theme);
        }
    }
    tracing::info!("Theme engine initialized, current: {}, default: {}", theme_engine.get_current_theme(), "default");

    // Initialize WASM plugin runtime
    let wasm_runtime = match noteva::plugin::PluginRuntime::new() {
        Ok(mut runtime) => {
            // Allow common permissions for WASM plugins
            runtime.set_allowed_permissions(vec![
                noteva::plugin::Permission::ReadArticles,
                noteva::plugin::Permission::WriteArticles,
                noteva::plugin::Permission::ReadConfig,
                noteva::plugin::Permission::WriteConfig,
                noteva::plugin::Permission::ReadComments,
                noteva::plugin::Permission::WriteComments,
                noteva::plugin::Permission::Network,
                noteva::plugin::Permission::Storage,
            ]);
            tracing::debug!("WASM plugin runtime initialized");
            Arc::new(tokio::sync::RwLock::new(runtime))
        }
        Err(e) => {
            tracing::warn!("Failed to initialize WASM runtime: {}, WASM plugins disabled", e);
            Arc::new(tokio::sync::RwLock::new(noteva::plugin::PluginRuntime::default()))
        }
    };

    // Initialize WASM plugin registry and load WASM plugins for enabled plugins
    let wasm_registry = Arc::new(tokio::sync::RwLock::new(
        noteva::plugin::wasm_bridge::WasmPluginRegistry::new(),
    ));
    
    // Load WASM modules for all enabled plugins at startup
    // WASM execution is isolated in subprocess (wasm-worker) — safe on all platforms
    noteva::plugin::wasm_bridge::load_all_wasm_plugins(
        &plugin_manager,
        &wasm_runtime,
        &hook_manager,
        &wasm_registry,
        &pool,
    ).await;

    // Build application state
    let request_stats = Arc::new(RequestStats::new());
    
    let rate_limiter = Arc::new(noteva::services::LoginRateLimiter::new());
    
    let state = AppState {
        pool: pool.clone(),
        user_service,
        user_repo,
        article_service,
        category_service,
        tag_service,
        settings_service,
        comment_service,
        theme_engine: Arc::new(std::sync::RwLock::new(theme_engine)),
        upload_config: Arc::new(config.upload.clone()),
        page_service,
        nav_service,
        plugin_manager: Arc::new(tokio::sync::RwLock::new(plugin_manager)),
        hook_manager: hook_manager.clone(),
        shortcode_manager: shortcode_manager_arc,
        request_stats,
        rate_limiter: rate_limiter.clone(),
        wasm_runtime: wasm_runtime.clone(),
        wasm_registry: wasm_registry.clone(),
        store_url: config.store_url.clone(),
    };
    
    // Start rate limiter cleanup task (runs every 5 minutes)
    {
        let limiter = rate_limiter.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                limiter.cleanup().await;
            }
        });
    }

    // Trigger system_init hook
    hook_manager.trigger(
        noteva::plugin::hook_names::SYSTEM_INIT,
        serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "theme": config.theme.active,
        })
    );

    // Build router
    let app = api::build_router(state, &config.server.cors_origin);

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
