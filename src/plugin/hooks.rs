//! Backend hook system
//!
//! Provides a way for plugins to hook into various lifecycle events.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, warn};

use super::hook_registry::{HookRegistry, HookType};

/// Hook callback type
pub type HookCallback = Arc<dyn Fn(&mut Value) -> Option<Value> + Send + Sync>;

/// Async hook callback type
pub type AsyncHookCallback = Arc<
    dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<Value>> + Send>>
        + Send
        + Sync,
>;

/// Hook priority (lower = earlier)
pub const PRIORITY_EARLY: i32 = -100;
pub const PRIORITY_DEFAULT: i32 = 0;
pub const PRIORITY_LATE: i32 = 100;

/// Registered hook handler
struct HookHandler {
    callback: HookCallback,
    priority: i32,
    plugin_id: Option<String>,
}

/// Hook manager for backend hooks
pub struct HookManager {
    /// Registered hooks (hook_name -> handlers)
    hooks: RwLock<HashMap<String, Vec<HookHandler>>>,
    /// Hook registry — single source of truth for hook definitions
    registry: HookRegistry,
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new(HookRegistry::load_embedded())
    }
}

impl HookManager {
    /// Create a new hook manager with the given registry
    pub fn new(registry: HookRegistry) -> Self {
        Self {
            hooks: RwLock::new(HashMap::new()),
            registry,
        }
    }

    /// Register a hook handler
    pub fn register<F>(&self, name: &str, callback: F, priority: i32, plugin_id: Option<String>)
    where
        F: Fn(&mut Value) -> Option<Value> + Send + Sync + 'static,
    {
        let Ok(mut hooks) = self.hooks.write() else {
            tracing::error!("Hook registry lock poisoned, cannot register");
            return;
        };
        let handlers = hooks.entry(name.to_string()).or_insert_with(Vec::new);

        handlers.push(HookHandler {
            callback: Arc::new(callback),
            priority,
            plugin_id,
        });

        // Sort by priority
        handlers.sort_by_key(|h| h.priority);

        debug!(
            "Registered hook handler for '{}' with priority {}",
            name, priority
        );
    }

    /// Unregister all hooks for a plugin
    pub fn unregister_plugin(&self, plugin_id: &str) {
        let Ok(mut hooks) = self.hooks.write() else {
            tracing::error!("Hook registry lock poisoned, cannot unregister");
            return;
        };
        for handlers in hooks.values_mut() {
            handlers.retain(|h| h.plugin_id.as_deref() != Some(plugin_id));
        }
    }

    /// Trigger a hook — automatically dispatches to filter or action based on registry type.
    /// Unknown hooks are treated as actions with a warning.
    pub fn trigger(&self, name: &str, data: Value) -> Value {
        match self.registry.get_type(name) {
            Some(HookType::Filter) => self.trigger_filter(name, data),
            Some(HookType::Action) => {
                self.trigger_action(name, data.clone());
                data
            }
            None => {
                warn!("Hook '{}' not found in registry, treating as action", name);
                self.trigger_action(name, data.clone());
                data
            }
        }
    }

    /// Filter trigger: chain handler return values so each replaces the data for the next.
    pub fn trigger_filter(&self, name: &str, mut data: Value) -> Value {
        // Warn if the registry says this is actually an action
        if let Some(HookType::Action) = self.registry.get_type(name) {
            warn!(
                "trigger_filter called on action hook '{}', falling back to action semantics",
                name
            );
            self.trigger_action(name, data.clone());
            return data;
        }

        let Ok(hooks) = self.hooks.read() else {
            tracing::error!("Hook registry lock poisoned in trigger_filter");
            return data;
        };
        if let Some(handlers) = hooks.get(name) {
            for handler in handlers {
                if let Some(result) = (handler.callback)(&mut data) {
                    data = result;
                }
            }
        }
        data
    }

    /// Action trigger: run all handlers but ignore their return values.
    pub fn trigger_action(&self, name: &str, mut data: Value) {
        // Warn if the registry says this is actually a filter
        if let Some(HookType::Filter) = self.registry.get_type(name) {
            warn!(
                "trigger_action called on filter hook '{}', executing as action (return values ignored)",
                name
            );
        }

        let Ok(hooks) = self.hooks.read() else {
            tracing::error!("Hook registry lock poisoned in trigger_action");
            return;
        };
        if let Some(handlers) = hooks.get(name) {
            for handler in handlers {
                let _ = (handler.callback)(&mut data);
            }
        }
    }

    /// Check if a hook has any handlers
    pub fn has_handlers(&self, name: &str) -> bool {
        let Ok(hooks) = self.hooks.read() else {
            return false;
        };
        hooks.get(name).map_or(false, |h| !h.is_empty())
    }

    /// Get a reference to the hook registry
    pub fn registry(&self) -> &HookRegistry {
        &self.registry
    }
}

/// Available backend hooks (only hooks with actual trigger points)
pub mod hook_names {
    // Article hooks - triggered in src/services/article.rs and src/api/articles.rs
    pub const ARTICLE_BEFORE_CREATE: &str = "article_before_create";
    pub const ARTICLE_AFTER_CREATE: &str = "article_after_create";
    pub const ARTICLE_BEFORE_UPDATE: &str = "article_before_update";
    pub const ARTICLE_AFTER_UPDATE: &str = "article_after_update";
    pub const ARTICLE_BEFORE_DELETE: &str = "article_before_delete";
    pub const ARTICLE_AFTER_DELETE: &str = "article_after_delete";
    pub const ARTICLE_BEFORE_DISPLAY: &str = "article_before_display";
    pub const ARTICLE_VIEW: &str = "article_view";
    pub const ARTICLE_STATUS_CHANGE: &str = "article_status_change";

    // Comment hooks - triggered in src/services/comment.rs and src/api/comments.rs
    pub const COMMENT_BEFORE_CREATE: &str = "comment_before_create";
    pub const COMMENT_AFTER_CREATE: &str = "comment_after_create";
    pub const COMMENT_BEFORE_DELETE: &str = "comment_before_delete";
    pub const COMMENT_AFTER_DELETE: &str = "comment_after_delete";
    pub const COMMENT_BEFORE_DISPLAY: &str = "comment_before_display";

    // User hooks - triggered in src/services/user.rs
    pub const USER_LOGIN_BEFORE: &str = "user_login_before";
    pub const USER_LOGIN_AFTER: &str = "user_login_after";
    pub const USER_LOGIN_FAILED: &str = "user_login_failed";
    pub const USER_LOGOUT: &str = "user_logout";
    pub const USER_REGISTER_BEFORE: &str = "user_register_before";
    pub const USER_REGISTER_AFTER: &str = "user_register_after";

    // Content processing hooks - triggered in src/services/markdown.rs
    pub const MARKDOWN_BEFORE_PARSE: &str = "markdown_before_parse";
    pub const MARKDOWN_AFTER_PARSE: &str = "markdown_after_parse";
    pub const EXCERPT_GENERATE: &str = "excerpt_generate"; // triggered in src/services/article.rs

    // API hooks - triggered in src/api/middleware.rs
    pub const API_REQUEST_BEFORE: &str = "api_request_before";
    pub const API_REQUEST_AFTER: &str = "api_request_after";

    // System hooks - triggered in various places
    pub const SYSTEM_INIT: &str = "system_init"; // src/main.rs
    pub const CACHE_CLEAR: &str = "cache_clear"; // src/cache/mod.rs
    pub const THEME_SWITCH: &str = "theme_switch"; // src/theme/mod.rs
    pub const THEME_ACTIVATE: &str = "theme_activate"; // src/api/admin.rs
    pub const PLUGIN_ACTIVATE: &str = "plugin_activate"; // src/api/plugins.rs
    pub const PLUGIN_DEACTIVATE: &str = "plugin_deactivate"; // src/api/plugins.rs
    pub const PLUGIN_ACTION: &str = "plugin_action"; // src/api/plugins.rs
    pub const PLUGIN_DESTROY: &str = "plugin_destroy"; // src/api/plugins.rs
    pub const PLUGIN_UPGRADE: &str = "plugin_upgrade"; // src/api/plugins.rs

    // Content filter hooks - triggered in services
    pub const ARTICLE_CONTENT_FILTER: &str = "article_content_filter"; // src/services/article.rs
    pub const ARTICLE_EXCERPT_FILTER: &str = "article_excerpt_filter"; // src/services/article.rs
    pub const COMMENT_CONTENT_FILTER: &str = "comment_content_filter"; // src/services/comment.rs

    // Page hooks - triggered in src/services/page.rs
    pub const PAGE_BEFORE_CREATE: &str = "page_before_create";
    pub const PAGE_AFTER_CREATE: &str = "page_after_create";
    pub const PAGE_BEFORE_UPDATE: &str = "page_before_update";
    pub const PAGE_AFTER_UPDATE: &str = "page_after_update";
    pub const PAGE_BEFORE_DELETE: &str = "page_before_delete";
    pub const PAGE_AFTER_DELETE: &str = "page_after_delete";

    // Taxonomy hooks - triggered in src/api/admin/taxonomy.rs
    pub const CATEGORY_AFTER_CREATE: &str = "category_after_create";
    pub const CATEGORY_AFTER_DELETE: &str = "category_after_delete";
    pub const TAG_AFTER_CREATE: &str = "tag_after_create";
    pub const TAG_AFTER_DELETE: &str = "tag_after_delete";

    // Comment moderation hooks - triggered in src/services/comment.rs
    pub const COMMENT_APPROVE: &str = "comment_approve";
    pub const COMMENT_REJECT: &str = "comment_reject";

    // User behavior hooks - triggered in src/services/user.rs and src/api/auth.rs
    pub const USER_PROFILE_UPDATE: &str = "user_profile_update";
    pub const USER_PASSWORD_CHANGE: &str = "user_password_change";

    // Settings hooks - triggered in src/api/admin/settings.rs
    pub const SETTINGS_BEFORE_SAVE: &str = "settings_before_save";
    pub const SETTINGS_AFTER_SAVE: &str = "settings_after_save";

    // Frontend hooks - triggered by frontend SDK
    pub const SEO_META_TAGS: &str = "seo_meta_tags"; // frontend
    pub const ADMIN_MENU: &str = "admin_menu"; // frontend
    pub const ADMIN_DASHBOARD: &str = "admin_dashboard"; // frontend

    // Navigation hooks - triggered in src/api/nav.rs
    pub const NAV_ITEMS_FILTER: &str = "nav_items_filter"; // src/api/nav.rs

    // Upload hooks - triggered in src/api/upload.rs
    pub const IMAGE_UPLOAD_FILTER: &str = "image_upload_filter"; // src/api/upload.rs
    pub const FILE_UPLOAD_FILTER: &str = "file_upload_filter"; // src/api/upload.rs

    // SEO hooks - triggered in src/api/seo.rs
    pub const FEED_FILTER: &str = "feed_filter"; // src/api/seo.rs
    pub const SITEMAP_FILTER: &str = "sitemap_filter"; // src/api/seo.rs

    // Article list hook - triggered in src/api/articles.rs
    pub const ARTICLE_LIST_FILTER: &str = "article_list_filter"; // src/api/articles.rs

    // Cron hooks - triggered in src/main.rs
    pub const CRON_REGISTER: &str = "cron_register"; // src/main.rs (system_init)
    pub const CRON_TICK: &str = "cron_tick"; // src/main.rs (every 60s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Helper: create a minimal HookRegistry for tests
    fn test_registry() -> HookRegistry {
        HookRegistry::load_embedded()
    }

    #[test]
    fn test_hook_registration_and_trigger() {
        let manager = HookManager::new(test_registry());

        // Use a known filter hook so trigger() applies filter semantics
        let hook = "article_before_create";

        // Register a hook that returns modified data
        manager.register(
            hook,
            |data| {
                let mut cloned = data.clone();
                if let Some(obj) = cloned.as_object_mut() {
                    obj.insert("modified".to_string(), json!(true));
                }
                Some(cloned)
            },
            PRIORITY_DEFAULT,
            None,
        );

        // Trigger the hook
        let input = json!({"original": true});
        let output = manager.trigger(hook, input);

        assert_eq!(output["original"], json!(true));
        assert_eq!(output["modified"], json!(true));
    }

    #[test]
    fn test_hook_priority() {
        let manager = HookManager::new(test_registry());

        // Use a known filter hook
        let hook = "article_before_create";

        // Register hooks with different priorities
        manager.register(
            hook,
            |data| {
                let mut cloned = data.clone();
                if let Some(arr) = cloned.as_array_mut() {
                    arr.push(json!("late"));
                }
                Some(cloned)
            },
            PRIORITY_LATE,
            None,
        );

        manager.register(
            hook,
            |data| {
                let mut cloned = data.clone();
                if let Some(arr) = cloned.as_array_mut() {
                    arr.push(json!("early"));
                }
                Some(cloned)
            },
            PRIORITY_EARLY,
            None,
        );

        manager.register(
            hook,
            |data| {
                let mut cloned = data.clone();
                if let Some(arr) = cloned.as_array_mut() {
                    arr.push(json!("default"));
                }
                Some(cloned)
            },
            PRIORITY_DEFAULT,
            None,
        );

        let output = manager.trigger(hook, json!([]));

        // Should be in order: early, default, late
        assert_eq!(output, json!(["early", "default", "late"]));
    }

    #[test]
    fn test_registry_accessor() {
        let manager = HookManager::new(test_registry());
        // The registry should contain hooks from hook-registry.json
        assert!(manager.registry().contains("article_before_create"));
    }
}
