//! Backend hook system
//!
//! Provides a way for plugins to hook into various lifecycle events.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde_json::Value;
use tracing::debug;

/// Hook callback type
pub type HookCallback = Arc<dyn Fn(&mut Value) -> Option<Value> + Send + Sync>;

/// Async hook callback type
pub type AsyncHookCallback = Arc<dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<Value>> + Send>> + Send + Sync>;

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
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HookManager {
    /// Create a new hook manager
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register a hook handler
    pub fn register<F>(&self, name: &str, callback: F, priority: i32, plugin_id: Option<String>)
    where
        F: Fn(&mut Value) -> Option<Value> + Send + Sync + 'static,
    {
        let mut hooks = self.hooks.write().unwrap();
        let handlers = hooks.entry(name.to_string()).or_insert_with(Vec::new);
        
        handlers.push(HookHandler {
            callback: Arc::new(callback),
            priority,
            plugin_id,
        });
        
        // Sort by priority
        handlers.sort_by_key(|h| h.priority);
        
        debug!("Registered hook handler for '{}' with priority {}", name, priority);
    }
    
    /// Unregister all hooks for a plugin
    pub fn unregister_plugin(&self, plugin_id: &str) {
        let mut hooks = self.hooks.write().unwrap();
        for handlers in hooks.values_mut() {
            handlers.retain(|h| h.plugin_id.as_deref() != Some(plugin_id));
        }
    }
    
    /// Trigger a hook and return modified data
    pub fn trigger(&self, name: &str, mut data: Value) -> Value {
        let hooks = self.hooks.read().unwrap();
        
        if let Some(handlers) = hooks.get(name) {
            for handler in handlers {
                if let Some(result) = (handler.callback)(&mut data) {
                    data = result;
                }
            }
        }
        
        data
    }
    
    /// Check if a hook has any handlers
    pub fn has_handlers(&self, name: &str) -> bool {
        let hooks = self.hooks.read().unwrap();
        hooks.get(name).map_or(false, |h| !h.is_empty())
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
    pub const EXCERPT_GENERATE: &str = "excerpt_generate";  // triggered in src/services/article.rs
    
    // API hooks - triggered in src/api/middleware.rs
    pub const API_REQUEST_BEFORE: &str = "api_request_before";
    pub const API_REQUEST_AFTER: &str = "api_request_after";
    
    // System hooks - triggered in various places
    pub const SYSTEM_INIT: &str = "system_init";           // src/main.rs
    pub const CACHE_CLEAR: &str = "cache_clear";           // src/cache/mod.rs
    pub const THEME_SWITCH: &str = "theme_switch";         // src/theme/mod.rs
    pub const PLUGIN_ACTIVATE: &str = "plugin_activate";   // src/api/plugins.rs
    pub const PLUGIN_DEACTIVATE: &str = "plugin_deactivate"; // src/api/plugins.rs
    
    // Content filter hooks - triggered in services
    pub const ARTICLE_CONTENT_FILTER: &str = "article_content_filter";     // src/services/article.rs
    pub const ARTICLE_EXCERPT_FILTER: &str = "article_excerpt_filter";     // src/services/article.rs
    pub const COMMENT_CONTENT_FILTER: &str = "comment_content_filter";     // src/services/comment.rs
    
    // Frontend hooks - triggered by frontend SDK
    pub const SEO_META_TAGS: &str = "seo_meta_tags";                       // frontend
    pub const ADMIN_MENU: &str = "admin_menu";                             // frontend
    pub const ADMIN_DASHBOARD: &str = "admin_dashboard";                   // frontend
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_hook_registration_and_trigger() {
        let manager = HookManager::new();
        
        // Register a hook that modifies data
        manager.register("test_hook", |data| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("modified".to_string(), json!(true));
            }
            None
        }, PRIORITY_DEFAULT, None);
        
        // Trigger the hook
        let input = json!({"original": true});
        let output = manager.trigger("test_hook", input);
        
        assert_eq!(output["original"], json!(true));
        assert_eq!(output["modified"], json!(true));
    }
    
    #[test]
    fn test_hook_priority() {
        let manager = HookManager::new();
        
        // Register hooks with different priorities
        manager.register("priority_test", |data| {
            if let Some(arr) = data.as_array_mut() {
                arr.push(json!("late"));
            }
            None
        }, PRIORITY_LATE, None);
        
        manager.register("priority_test", |data| {
            if let Some(arr) = data.as_array_mut() {
                arr.push(json!("early"));
            }
            None
        }, PRIORITY_EARLY, None);
        
        manager.register("priority_test", |data| {
            if let Some(arr) = data.as_array_mut() {
                arr.push(json!("default"));
            }
            None
        }, PRIORITY_DEFAULT, None);
        
        let output = manager.trigger("priority_test", json!([]));
        
        // Should be in order: early, default, late
        assert_eq!(output, json!(["early", "default", "late"]));
    }
}
