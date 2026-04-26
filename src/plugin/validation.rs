//! Plugin package and settings validation.
//!
//! The plugin runtime intentionally keeps the package format small and
//! convention-based. This module enforces those conventions before a plugin is
//! loaded, installed, or updated.

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::hook_registry::{validate_plugin_hooks, HookRegistry};
use super::loader::{PluginMetadata, PluginPageDeclaration};

pub const PLUGIN_SCHEMA_VERSION: u32 = 1;

const FRONTEND_HOOKS: &[&str] = &[
    "system_init",
    "body_end",
    "route_change",
    "content_render",
    "article_view",
    "comment_after_create",
    "seo_meta_tags",
    "api_request_before",
    "api_request_after",
    "api_error",
];

const STABLE_PERMISSIONS: &[&str] = &[
    "network",
    "storage",
    "read_articles",
    "read_comments",
    "write_articles",
];

pub fn validate_plugin_package_dir(plugin_dir: &Path) -> Result<PluginMetadata> {
    let manifest = load_plugin_manifest(plugin_dir)?;
    let expected_dir_name = plugin_dir.file_name().and_then(|name| name.to_str());
    validate_plugin_manifest(&manifest, expected_dir_name)?;
    validate_package_files(plugin_dir, &manifest)?;
    Ok(manifest)
}

pub fn load_plugin_manifest(plugin_dir: &Path) -> Result<PluginMetadata> {
    let path = plugin_dir.join("plugin.json");
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read plugin.json: {:?}", path))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse plugin.json: {:?}", path))
}

pub fn load_settings_schema(path: &Path) -> Result<Value> {
    crate::theme::validation::load_settings_schema(path)
}

pub fn validate_settings_schema(schema: &Value) -> Result<()> {
    crate::theme::validation::validate_settings_schema(schema)
}

pub fn validate_plugin_manifest(
    manifest: &PluginMetadata,
    expected_dir_name: Option<&str>,
) -> Result<()> {
    if manifest.schema != PLUGIN_SCHEMA_VERSION {
        return Err(anyhow!(
            "unsupported plugin schema {}; expected {}",
            manifest.schema,
            PLUGIN_SCHEMA_VERSION
        ));
    }

    validate_plugin_id(&manifest.id)?;
    if let Some(dir_name) = expected_dir_name {
        if dir_name != manifest.id {
            return Err(anyhow!(
                "plugin directory '{}' must match plugin id '{}'",
                dir_name,
                manifest.id
            ));
        }
    }

    validate_non_empty("name", &manifest.name, 80)?;
    validate_semver_like("version", &manifest.version)?;
    validate_non_empty("description", &manifest.description, 240)?;
    validate_non_empty("author", &manifest.author, 80)?;
    validate_non_empty("license", &manifest.license, 40)?;
    validate_repository(&manifest.repository)?;

    let requires_noteva = manifest.requires.noteva.trim();
    if requires_noteva.is_empty() {
        return Err(anyhow!("requires.noteva is required"));
    }
    for required_plugin in &manifest.requires.plugins {
        validate_plugin_id(required_plugin)?;
    }

    validate_hooks(manifest)?;
    validate_permissions(&manifest.permissions)?;
    validate_activate(&manifest.activate)?;

    for shortcode in &manifest.shortcodes {
        validate_shortcode(shortcode)?;
    }
    for page in &manifest.pages {
        validate_page(page)?;
    }

    if manifest.database {
        return Err(anyhow!(
            "plugin database API is experimental and cannot be declared in plugin schema v1"
        ));
    }

    Ok(())
}

fn validate_package_files(plugin_dir: &Path, manifest: &PluginMetadata) -> Result<()> {
    if !plugin_dir.join("plugin.json").is_file() {
        return Err(anyhow!("plugin package must contain plugin.json"));
    }

    if manifest.settings {
        let settings_path = plugin_dir.join("settings.json");
        if !settings_path.is_file() {
            return Err(anyhow!(
                "plugin declares settings=true but settings.json is missing"
            ));
        }
        let settings = load_settings_schema(&settings_path)?;
        validate_settings_schema(&settings)?;
    } else if plugin_dir.join("settings.json").exists() {
        return Err(anyhow!("plugin has settings.json but settings is not true"));
    }

    if !manifest.hooks.frontend.is_empty() && !plugin_dir.join("frontend.js").is_file() {
        return Err(anyhow!(
            "plugin declares frontend hooks but frontend.js is missing"
        ));
    }

    if !manifest.hooks.backend.is_empty() && !plugin_dir.join("backend.wasm").is_file() {
        return Err(anyhow!(
            "plugin declares backend hooks but backend.wasm is missing"
        ));
    }

    if manifest.api && !plugin_dir.join("backend.wasm").is_file() {
        return Err(anyhow!(
            "plugin declares api=true but backend.wasm is missing"
        ));
    }

    if manifest.hooks.editor.iter().any(|hook| hook == "toolbar")
        && !plugin_dir.join("editor.json").is_file()
    {
        return Err(anyhow!(
            "plugin declares editor toolbar hook but editor.json is missing"
        ));
    }

    Ok(())
}

fn validate_hooks(manifest: &PluginMetadata) -> Result<()> {
    let registry = HookRegistry::load_embedded();
    let backend_warnings =
        validate_plugin_hooks(&registry, &manifest.id, &manifest.hooks.backend, &[]);
    if let Some(warning) = backend_warnings.first() {
        return Err(anyhow!("invalid backend hook declaration: {}", warning));
    }

    let mut frontend_seen = HashSet::new();
    for hook in &manifest.hooks.frontend {
        if !frontend_seen.insert(hook) {
            return Err(anyhow!("duplicate frontend hook '{}'", hook));
        }
        if !FRONTEND_HOOKS.contains(&hook.as_str()) {
            return Err(anyhow!("unknown frontend hook '{}'", hook));
        }
    }

    let mut backend_seen = HashSet::new();
    for hook in &manifest.hooks.backend {
        if !backend_seen.insert(hook) {
            return Err(anyhow!("duplicate backend hook '{}'", hook));
        }
    }

    for hook in &manifest.hooks.editor {
        if hook != "toolbar" {
            return Err(anyhow!("unknown editor hook '{}'", hook));
        }
    }

    Ok(())
}

fn validate_permissions(permissions: &[String]) -> Result<()> {
    let mut seen = HashSet::new();
    for permission in permissions {
        if !seen.insert(permission) {
            return Err(anyhow!("duplicate permission '{}'", permission));
        }
        if !STABLE_PERMISSIONS.contains(&permission.as_str()) {
            return Err(anyhow!(
                "permission '{}' is not available in plugin schema v1",
                permission
            ));
        }
    }
    Ok(())
}

fn validate_activate(activate: &super::loader::ActivateConfig) -> Result<()> {
    if activate.interval_hours > 24 * 30 {
        return Err(anyhow!("activate.interval_hours cannot exceed 720"));
    }
    Ok(())
}

fn validate_page(page: &PluginPageDeclaration) -> Result<()> {
    validate_page_slug(&page.slug)?;
    validate_non_empty("page.title", &page.title, 80)?;
    Ok(())
}

pub fn collect_settings_fields<'a>(schema: &'a Value) -> HashMap<String, &'a Value> {
    let mut fields = HashMap::new();
    if let Some(sections) = schema.get("sections").and_then(Value::as_array) {
        for section in sections {
            if let Some(items) = section.get("fields").and_then(Value::as_array) {
                for field in items {
                    if let Some(id) = field.get("id").and_then(Value::as_str) {
                        fields.insert(id.to_string(), field);
                    }
                }
            }
        }
    }
    fields
}

pub fn coerce_plugin_setting_value(field: &Value, value: &Value) -> Result<Value> {
    let field_type = field
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("settings field type is required"))?;
    let field_id = field.get("id").and_then(Value::as_str).unwrap_or("unknown");

    if field
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && is_empty_setting_value(value)
    {
        return Err(anyhow!("setting '{}' is required", field_id));
    }

    match field_type {
        "text" | "textarea" | "color" => Ok(Value::String(coerce_string_setting(field, value)?)),
        "switch" => Ok(Value::Bool(coerce_bool(value)?)),
        "select" => {
            let selected = coerce_string(value)?;
            let options = field
                .get("options")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow!("select setting '{}' missing options", field_id))?;
            let valid = options.iter().any(|option| {
                option
                    .get("value")
                    .and_then(Value::as_str)
                    .map(|value| value == selected)
                    .unwrap_or(false)
            });
            if !valid {
                return Err(anyhow!("invalid value for select setting '{}'", field_id));
            }
            Ok(Value::String(selected))
        }
        "number" => {
            let number = coerce_number(value)?;
            if let Some(min) = field.get("min").and_then(Value::as_f64) {
                if number < min {
                    return Err(anyhow!(
                        "setting '{}' must be greater than or equal to {}",
                        field_id,
                        min
                    ));
                }
            }
            if let Some(max) = field.get("max").and_then(Value::as_f64) {
                if number > max {
                    return Err(anyhow!(
                        "setting '{}' must be less than or equal to {}",
                        field_id,
                        max
                    ));
                }
            }
            serde_json::Number::from_f64(number)
                .map(Value::Number)
                .ok_or_else(|| anyhow!("setting '{}' must be a finite number", field_id))
        }
        "array" => coerce_array_setting(field, value),
        _ => Err(anyhow!("unsupported setting type '{}'", field_type)),
    }
}

fn coerce_array_setting(field: &Value, value: &Value) -> Result<Value> {
    let field_id = field.get("id").and_then(Value::as_str).unwrap_or("unknown");
    let items = value
        .as_array()
        .ok_or_else(|| anyhow!("setting '{}' must be an array", field_id))?;
    let item_fields = field
        .get("itemFields")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("array setting '{}' missing itemFields", field_id))?;

    let item_fields = item_fields
        .iter()
        .filter_map(|item| {
            item.get("id")
                .and_then(Value::as_str)
                .map(|id| (id.to_string(), item))
        })
        .collect::<HashMap<_, _>>();

    let mut coerced_items = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let obj = item.as_object().ok_or_else(|| {
            anyhow!(
                "setting '{}' item {} must be an object",
                field_id,
                index + 1
            )
        })?;
        let mut coerced = serde_json::Map::new();

        for (item_field_id, item_field) in &item_fields {
            let value = obj.get(item_field_id).unwrap_or(&Value::Null);
            if item_field
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && is_empty_setting_value(value)
            {
                return Err(anyhow!(
                    "setting '{}' item {} field '{}' is required",
                    field_id,
                    index + 1,
                    item_field_id
                ));
            }
            if value.is_null() {
                continue;
            }
            let item_type = item_field
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("text");
            let coerced_value = match item_type {
                "number" => serde_json::Number::from_f64(coerce_number(value)?)
                    .map(Value::Number)
                    .ok_or_else(|| {
                        anyhow!("array item field '{}' must be finite", item_field_id)
                    })?,
                _ => Value::String(coerce_string(value)?),
            };
            coerced.insert(item_field_id.clone(), coerced_value);
        }

        for key in obj.keys() {
            if !item_fields.contains_key(key) {
                return Err(anyhow!("unknown array item field '{}.{}'", field_id, key));
            }
        }

        coerced_items.push(Value::Object(coerced));
    }

    Ok(Value::Array(coerced_items))
}

fn coerce_string_setting(field: &Value, value: &Value) -> Result<String> {
    let field_id = field.get("id").and_then(Value::as_str).unwrap_or("unknown");
    let text = coerce_string(value)?;
    if let Some(max) = field.get("maxLength").and_then(Value::as_u64) {
        if text.chars().count() as u64 > max {
            return Err(anyhow!(
                "setting '{}' must be at most {} characters",
                field_id,
                max
            ));
        }
    }
    Ok(text)
}

fn coerce_string(value: &Value) -> Result<String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Null => Ok(String::new()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        _ => Err(anyhow!("expected string-compatible setting value")),
    }
}

fn coerce_bool(value: &Value) -> Result<bool> {
    match value {
        Value::Bool(value) => Ok(*value),
        Value::String(value) if value == "true" => Ok(true),
        Value::String(value) if value == "false" => Ok(false),
        _ => Err(anyhow!("expected boolean setting value")),
    }
}

fn coerce_number(value: &Value) -> Result<f64> {
    match value {
        Value::Number(value) => value
            .as_f64()
            .ok_or_else(|| anyhow!("expected finite number setting value")),
        Value::String(value) => value
            .parse::<f64>()
            .map_err(|_| anyhow!("expected number setting value")),
        _ => Err(anyhow!("expected number setting value")),
    }
}

fn is_empty_setting_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.trim().is_empty(),
        Value::Array(value) => value.is_empty(),
        _ => false,
    }
}

fn validate_plugin_id(id: &str) -> Result<()> {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return Err(anyhow!("plugin id cannot be empty"));
    };

    let valid = id.len() <= 63
        && (first.is_ascii_lowercase() || first.is_ascii_digit())
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
    if valid {
        Ok(())
    } else {
        Err(anyhow!("plugin id must match ^[a-z0-9][a-z0-9-]{{0,62}}$"))
    }
}

fn validate_shortcode(shortcode: &str) -> Result<()> {
    if shortcode.is_empty()
        || shortcode.len() > 64
        || !shortcode
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(anyhow!(
            "shortcode must use lowercase letters, numbers, or hyphens"
        ));
    }
    Ok(())
}

fn validate_page_slug(slug: &str) -> Result<()> {
    if slug.is_empty()
        || slug.len() > 96
        || slug.contains("..")
        || slug.contains('/')
        || slug.contains('\\')
        || !slug
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(anyhow!(
            "page slug must use lowercase letters, numbers, or hyphens"
        ));
    }
    Ok(())
}

fn validate_repository(repository: &str) -> Result<()> {
    let repository = repository
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git");
    validate_non_empty("repository", repository, 240)?;

    if repository
        .strip_prefix("https://github.com/")
        .map(is_owner_repo)
        .unwrap_or(false)
        || is_owner_repo(repository)
    {
        Ok(())
    } else {
        Err(anyhow!(
            "repository must be a GitHub URL or owner/repo identifier"
        ))
    }
}

fn is_owner_repo(value: &str) -> bool {
    let mut parts = value.split('/');
    let Some(owner) = parts.next() else {
        return false;
    };
    let Some(repo) = parts.next() else {
        return false;
    };
    parts.next().is_none() && is_github_part(owner) && is_github_part(repo)
}

fn is_github_part(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

fn validate_semver_like(field: &str, value: &str) -> Result<()> {
    let core = value.split('-').next().unwrap_or(value);
    let parts: Vec<&str> = core.split('.').collect();
    let valid = parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()));

    if valid {
        Ok(())
    } else {
        Err(anyhow!("{} must use x.y.z version format", field))
    }
}

fn validate_non_empty(field: &str, value: &str, max_len: usize) -> Result<()> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("{} is required", field));
    }
    if value.chars().count() > max_len {
        return Err(anyhow!("{} must be at most {} characters", field, max_len));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_plugin_packages_validate() {
        let plugins_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins");
        let entries = fs::read_dir(&plugins_dir).expect("plugins directory should exist");

        for entry in entries {
            let entry = entry.expect("plugin directory entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                validate_plugin_package_dir(&path).unwrap_or_else(|error| {
                    panic!("invalid official plugin {:?}: {}", path, error)
                });
            }
        }
    }

    #[test]
    fn rejects_experimental_database_api_in_schema_v1() {
        let manifest = PluginMetadata {
            schema: PLUGIN_SCHEMA_VERSION,
            id: "db-plugin".to_string(),
            name: "DB Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A plugin that tries to use database API".to_string(),
            author: "Noteva Team".to_string(),
            repository: "noteva26/noteva-plugins".to_string(),
            license: "MIT".to_string(),
            requires: crate::plugin::loader::PluginRequirements {
                noteva: ">=0.2.7".to_string(),
                plugins: Vec::new(),
            },
            hooks: Default::default(),
            shortcodes: Vec::new(),
            permissions: Vec::new(),
            settings: false,
            database: true,
            api: false,
            activate: Default::default(),
            pages: Vec::new(),
        };

        assert!(validate_plugin_manifest(&manifest, None).is_err());
    }
}
