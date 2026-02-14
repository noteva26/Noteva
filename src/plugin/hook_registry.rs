//! Hook registry — the single source of truth for all hook definitions.
//!
//! The registry is loaded from `hook-registry.json` which is embedded at compile
//! time via `include_str!`. It provides query methods for looking up hook
//! definitions by name, type, scope, and category.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Hook type: filter (return value replaces data) or action (return value ignored).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookType {
    Filter,
    Action,
}

/// Hook scope: backend-only, frontend-only, or both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookScope {
    Backend,
    Frontend,
    Both,
}

/// A single hook definition from the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub hook_type: HookType,
    pub description: String,
    pub trigger_point: String,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub scope: HookScope,
    pub available_since: String,
}

/// The hook registry containing all hook definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRegistry {
    pub version: String,
    pub hooks: Vec<HookDefinition>,
    /// Internal index built after loading (name -> position in `hooks` vec).
    #[serde(skip)]
    index: HashMap<String, usize>,
}

impl HookRegistry {
    /// Load the registry from the compile-time embedded JSON.
    pub fn load_embedded() -> Self {
        let json = include_str!("../../hook-registry.json");
        let mut registry: HookRegistry =
            serde_json::from_str(json).expect("hook-registry.json is invalid — this is a build-time bug");
        registry.build_index();
        registry
    }

    /// Build the internal name→index lookup table.
    fn build_index(&mut self) {
        self.index = self
            .hooks
            .iter()
            .enumerate()
            .map(|(i, h)| (h.name.clone(), i))
            .collect();
    }

    /// Look up a hook definition by name.
    pub fn get(&self, name: &str) -> Option<&HookDefinition> {
        self.index.get(name).map(|&i| &self.hooks[i])
    }

    /// Check whether a hook name exists in the registry.
    pub fn contains(&self, name: &str) -> bool {
        self.index.contains_key(name)
    }

    /// Get the type of a hook by name.
    pub fn get_type(&self, name: &str) -> Option<&HookType> {
        self.get(name).map(|d| &d.hook_type)
    }

    /// Return all hook names.
    pub fn all_names(&self) -> Vec<&str> {
        self.hooks.iter().map(|h| h.name.as_str()).collect()
    }

    /// Return hooks matching the given scope.
    pub fn by_scope(&self, scope: &HookScope) -> Vec<&HookDefinition> {
        self.hooks
            .iter()
            .filter(|h| &h.scope == scope || h.scope == HookScope::Both)
            .collect()
    }

    /// Group hooks by category (inferred from the name prefix before the first `_`
    /// that is followed by a known action keyword, or simply the first segment).
    pub fn by_category(&self) -> HashMap<String, Vec<&HookDefinition>> {
        let mut map: HashMap<String, Vec<&HookDefinition>> = HashMap::new();
        for hook in &self.hooks {
            let category = infer_category(&hook.name);
            map.entry(category).or_default().push(hook);
        }
        map
    }
}

/// Infer a human-friendly category from a hook name.
///
/// Rules:
/// - Names starting with `article_` → "article"
/// - Names starting with `comment_` → "comment"
/// - Names starting with `user_`    → "user"
/// - Names starting with `markdown_` or `excerpt_` → "content_processing"
/// - Names starting with `api_`     → "api"
/// - Names starting with `system_` | `cache_` | `theme_` → "system"
/// - Names starting with `plugin_`  → "plugin"
/// - Names starting with `seo_` | `admin_` → "frontend"
/// - Names starting with `nav_`     → "navigation"
/// - Anything else                  → first segment before `_`
fn infer_category(name: &str) -> String {
    let prefix = name.split('_').next().unwrap_or(name);
    match prefix {
        "article" => "article".to_string(),
        "comment" => "comment".to_string(),
        "user" => "user".to_string(),
        "markdown" | "excerpt" => "content_processing".to_string(),
        "api" => "api".to_string(),
        "system" | "cache" | "theme" => "system".to_string(),
        "plugin" => "plugin".to_string(),
        "seo" | "admin" => "frontend".to_string(),
        "nav" => "navigation".to_string(),
        "image" => "upload".to_string(),
        other => other.to_string(),
    }
}

/// A warning produced when validating a plugin's declared hooks against the registry.
#[derive(Debug, Clone)]
pub struct HookValidationWarning {
    pub plugin_id: String,
    pub hook_name: String,
    /// `"unknown_hook"` or `"scope_mismatch"`
    pub reason: String,
}

impl std::fmt::Display for HookValidationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Plugin '{}': hook '{}' — {}",
            self.plugin_id, self.hook_name, self.reason
        )
    }
}

/// Validate the hooks declared in a plugin's `plugin.json` against the registry.
///
/// Returns a list of warnings (never blocks loading).
pub fn validate_plugin_hooks(
    registry: &HookRegistry,
    plugin_id: &str,
    backend_hooks: &[String],
    frontend_hooks: &[String],
) -> Vec<HookValidationWarning> {
    let mut warnings = Vec::new();

    for hook_name in backend_hooks {
        match registry.get(hook_name) {
            None => {
                warnings.push(HookValidationWarning {
                    plugin_id: plugin_id.to_string(),
                    hook_name: hook_name.clone(),
                    reason: "unknown_hook".to_string(),
                });
            }
            Some(def) => {
                if def.scope != HookScope::Backend && def.scope != HookScope::Both {
                    warnings.push(HookValidationWarning {
                        plugin_id: plugin_id.to_string(),
                        hook_name: hook_name.clone(),
                        reason: format!(
                            "scope_mismatch: declared in hooks.backend but registry scope is {:?}",
                            def.scope
                        ),
                    });
                }
            }
        }
    }

    for hook_name in frontend_hooks {
        match registry.get(hook_name) {
            None => {
                warnings.push(HookValidationWarning {
                    plugin_id: plugin_id.to_string(),
                    hook_name: hook_name.clone(),
                    reason: "unknown_hook".to_string(),
                });
            }
            Some(def) => {
                if def.scope != HookScope::Frontend && def.scope != HookScope::Both {
                    warnings.push(HookValidationWarning {
                        plugin_id: plugin_id.to_string(),
                        hook_name: hook_name.clone(),
                        reason: format!(
                            "scope_mismatch: declared in hooks.frontend but registry scope is {:?}",
                            def.scope
                        ),
                    });
                }
            }
        }
    }

    warnings
}

