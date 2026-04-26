use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path};

use super::{ThemeJsonMetadata, ThemePageDeclaration};

pub const THEME_SCHEMA_VERSION: u32 = 1;
pub const THEME_SETTINGS_SCHEMA_VERSION: u32 = 1;

pub fn validate_theme_package_dir(theme_dir: &Path) -> Result<ThemeJsonMetadata> {
    let manifest = load_theme_manifest(theme_dir)?;
    validate_theme_manifest(&manifest, None)?;

    let entry = theme_dir.join("dist").join("index.html");
    if !entry.is_file() {
        return Err(anyhow!("theme package must contain dist/index.html"));
    }

    if let Some(preview) = &manifest.preview {
        validate_relative_asset_path("preview", preview)?;
        if !theme_dir.join(preview).is_file() {
            return Err(anyhow!("theme preview file does not exist: {}", preview));
        }
    }

    let settings_path = theme_dir.join("settings.json");
    if settings_path.exists() {
        let settings = load_settings_schema(&settings_path)?;
        validate_settings_schema(&settings)?;
    }

    Ok(manifest)
}

pub fn load_theme_manifest(theme_dir: &Path) -> Result<ThemeJsonMetadata> {
    let path = theme_dir.join("theme.json");
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read theme.json: {:?}", path))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse theme.json: {:?}", path))
}

pub fn load_settings_schema(path: &Path) -> Result<Value> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read settings.json: {:?}", path))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse settings.json: {:?}", path))
}

pub fn validate_theme_manifest(
    manifest: &ThemeJsonMetadata,
    expected_dir_name: Option<&str>,
) -> Result<()> {
    if manifest.schema != THEME_SCHEMA_VERSION {
        return Err(anyhow!(
            "unsupported theme schema {}; expected {}",
            manifest.schema,
            THEME_SCHEMA_VERSION
        ));
    }

    validate_non_empty("name", &manifest.name, 80)?;
    validate_theme_slug(&manifest.short)?;
    if let Some(dir_name) = expected_dir_name {
        if dir_name != manifest.short {
            return Err(anyhow!(
                "theme directory '{}' must match theme short '{}'",
                dir_name,
                manifest.short
            ));
        }
    }

    validate_semver_like("version", &manifest.version)?;
    validate_non_empty("author", &manifest.author, 80)?;
    validate_non_empty("description", &manifest.description, 240)?;
    validate_repository(&manifest.repository)?;

    let requires_noteva = manifest.requires.noteva.trim();
    if requires_noteva.is_empty() {
        return Err(anyhow!("requires.noteva is required"));
    }

    if let Some(preview) = &manifest.preview {
        validate_relative_asset_path("preview", preview)?;
    }

    for page in &manifest.pages {
        validate_theme_page(page)?;
    }

    Ok(())
}

pub fn validate_theme_slug(slug: &str) -> Result<()> {
    if !is_valid_theme_slug(slug) {
        return Err(anyhow!(
            "theme short must match ^[a-z0-9][a-z0-9-]{{0,62}}$"
        ));
    }
    Ok(())
}

pub fn is_valid_theme_slug(slug: &str) -> bool {
    let mut chars = slug.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    slug.len() <= 63
        && (first.is_ascii_lowercase() || first.is_ascii_digit())
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

pub fn validate_relative_asset_path(field: &str, value: &str) -> Result<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("{} cannot be empty", field));
    }
    if trimmed.contains('\\') {
        return Err(anyhow!("{} must use forward slashes", field));
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(anyhow!("{} must be a relative path", field));
    }

    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err(anyhow!("{} contains an unsafe path segment", field)),
        }
    }

    Ok(())
}

pub fn validate_settings_schema(schema: &Value) -> Result<()> {
    let obj = schema
        .as_object()
        .ok_or_else(|| anyhow!("settings.json must be an object"))?;

    reject_unknown_keys(obj.keys(), &["schema", "sections"], "settings.json")?;

    let schema_version = obj.get("schema").and_then(Value::as_u64).ok_or_else(|| {
        anyhow!(
            "settings.json schema must be {}",
            THEME_SETTINGS_SCHEMA_VERSION
        )
    })?;
    if schema_version != u64::from(THEME_SETTINGS_SCHEMA_VERSION) {
        return Err(anyhow!(
            "unsupported settings schema {}; expected {}",
            schema_version,
            THEME_SETTINGS_SCHEMA_VERSION
        ));
    }

    let sections = obj
        .get("sections")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("settings.json sections must be an array"))?;

    let mut section_ids = HashSet::new();
    let mut field_ids = HashSet::new();
    for section in sections {
        validate_settings_section(section, &mut section_ids, &mut field_ids)?;
    }

    Ok(())
}

pub fn coerce_setting_value(field: &Value, value: &Value) -> Result<String> {
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
        "text" | "textarea" | "color" => coerce_string_setting(field, value),
        "switch" => Ok(coerce_bool(value)?.to_string()),
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
            Ok(selected)
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
            Ok(number.to_string())
        }
        "array" => {
            if !value.is_array() {
                return Err(anyhow!("setting '{}' must be an array", field_id));
            }
            serde_json::to_string(value)
                .with_context(|| format!("failed to serialize setting '{}'", field_id))
        }
        _ => Err(anyhow!("unsupported setting type '{}'", field_type)),
    }
}

fn validate_settings_section(
    section: &Value,
    section_ids: &mut HashSet<String>,
    field_ids: &mut HashSet<String>,
) -> Result<()> {
    let obj = section
        .as_object()
        .ok_or_else(|| anyhow!("settings section must be an object"))?;
    reject_unknown_keys(
        obj.keys(),
        &["id", "title", "description", "fields"],
        "settings section",
    )?;

    let id = required_str(obj.get("id"), "section.id")?;
    validate_settings_id("section.id", id)?;
    if !section_ids.insert(id.to_string()) {
        return Err(anyhow!("duplicate settings section id '{}'", id));
    }

    validate_localized_text(obj.get("title"), "section.title", true)?;
    validate_localized_text(obj.get("description"), "section.description", false)?;

    let fields = obj
        .get("fields")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("section '{}' fields must be an array", id))?;
    for field in fields {
        validate_settings_field(field, field_ids)?;
    }

    Ok(())
}

fn validate_settings_field(field: &Value, field_ids: &mut HashSet<String>) -> Result<()> {
    let obj = field
        .as_object()
        .ok_or_else(|| anyhow!("settings field must be an object"))?;
    reject_unknown_keys(
        obj.keys(),
        &[
            "id",
            "type",
            "label",
            "description",
            "placeholder",
            "default",
            "required",
            "secret",
            "options",
            "min",
            "max",
            "step",
            "rows",
            "maxLength",
            "itemFields",
        ],
        "settings field",
    )?;

    let id = required_str(obj.get("id"), "field.id")?;
    validate_settings_id("field.id", id)?;
    if !field_ids.insert(id.to_string()) {
        return Err(anyhow!("duplicate settings field id '{}'", id));
    }

    let field_type = required_str(obj.get("type"), "field.type")?;
    validate_field_type(field_type)?;
    validate_localized_text(obj.get("label"), "field.label", true)?;
    validate_localized_text(obj.get("description"), "field.description", false)?;
    validate_localized_text(obj.get("placeholder"), "field.placeholder", false)?;

    if let Some(required) = obj.get("required") {
        require_bool(required, "field.required")?;
    }
    if let Some(secret) = obj.get("secret") {
        require_bool(secret, "field.secret")?;
    }
    if let Some(rows) = obj.get("rows") {
        if field_type != "textarea" {
            return Err(anyhow!("field.rows is only supported for textarea fields"));
        }
        let rows = rows
            .as_u64()
            .ok_or_else(|| anyhow!("field.rows must be a positive integer"))?;
        if rows == 0 || rows > 30 {
            return Err(anyhow!("field.rows must be between 1 and 30"));
        }
    }
    if let Some(max_length) = obj.get("maxLength") {
        if !matches!(field_type, "text" | "textarea" | "color") {
            return Err(anyhow!(
                "field.maxLength is only supported for text, textarea, and color fields"
            ));
        }
        let max_length = max_length
            .as_u64()
            .ok_or_else(|| anyhow!("field.maxLength must be a positive integer"))?;
        if max_length == 0 || max_length > 10000 {
            return Err(anyhow!("field.maxLength must be between 1 and 10000"));
        }
    }

    match field_type {
        "select" => validate_select_options(field)?,
        "number" => validate_number_field(field)?,
        "array" => validate_array_field(field)?,
        "color" => {
            if let Some(default) = obj.get("default") {
                validate_color_default(default)?;
            }
        }
        "switch" => {
            if let Some(default) = obj.get("default") {
                require_bool(default, "field.default")?;
            }
        }
        "text" | "textarea" => {
            if let Some(default) = obj.get("default") {
                require_string(default, "field.default")?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn validate_select_options(field: &Value) -> Result<()> {
    let field_id = field.get("id").and_then(Value::as_str).unwrap_or("unknown");
    let options = field
        .get("options")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("select field '{}' options must be an array", field_id))?;
    if options.is_empty() {
        return Err(anyhow!("select field '{}' must have options", field_id));
    }

    let mut values = HashSet::new();
    for option in options {
        let obj = option
            .as_object()
            .ok_or_else(|| anyhow!("select option must be an object"))?;
        reject_unknown_keys(obj.keys(), &["value", "label"], "select option")?;
        let value = required_str(obj.get("value"), "option.value")?;
        if value.trim().is_empty() {
            return Err(anyhow!("select option value cannot be empty"));
        }
        if !values.insert(value.to_string()) {
            return Err(anyhow!("duplicate select option value '{}'", value));
        }
        validate_localized_text(obj.get("label"), "option.label", true)?;
    }

    if let Some(default) = field.get("default") {
        let default = required_value_str(default, "field.default")?;
        if !values.contains(default) {
            return Err(anyhow!(
                "select field '{}' default is not in options",
                field_id
            ));
        }
    }

    Ok(())
}

fn validate_number_field(field: &Value) -> Result<()> {
    for key in ["min", "max", "step"] {
        if let Some(value) = field.get(key) {
            require_number(value, &format!("field.{}", key))?;
        }
    }

    if let Some(default) = field.get("default") {
        require_number(default, "field.default")?;
    }

    let min = field.get("min").and_then(Value::as_f64);
    let max = field.get("max").and_then(Value::as_f64);
    if let (Some(min), Some(max)) = (min, max) {
        if min > max {
            return Err(anyhow!("field.min cannot be greater than field.max"));
        }
    }

    Ok(())
}

fn validate_array_field(field: &Value) -> Result<()> {
    if let Some(default) = field.get("default") {
        if !default.is_array() {
            return Err(anyhow!("array field default must be an array"));
        }
    }

    let item_fields = field
        .get("itemFields")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("array field itemFields must be an array"))?;
    if item_fields.is_empty() {
        return Err(anyhow!("array field itemFields cannot be empty"));
    }

    let mut ids = HashSet::new();
    for item_field in item_fields {
        let obj = item_field
            .as_object()
            .ok_or_else(|| anyhow!("array item field must be an object"))?;
        reject_unknown_keys(
            obj.keys(),
            &["id", "label", "type", "placeholder", "required"],
            "array item field",
        )?;
        let id = required_str(obj.get("id"), "itemField.id")?;
        validate_settings_id("itemField.id", id)?;
        if !ids.insert(id.to_string()) {
            return Err(anyhow!("duplicate array item field id '{}'", id));
        }

        let field_type = required_str(obj.get("type"), "itemField.type")?;
        if field_type != "text" && field_type != "number" {
            return Err(anyhow!(
                "array item field '{}' only supports text or number",
                id
            ));
        }

        validate_localized_text(obj.get("label"), "itemField.label", true)?;
        validate_localized_text(obj.get("placeholder"), "itemField.placeholder", false)?;
        if let Some(required) = obj.get("required") {
            require_bool(required, "itemField.required")?;
        }
    }

    if let Some(default) = field.get("default").and_then(Value::as_array) {
        for (index, item) in default.iter().enumerate() {
            let obj = item.as_object().ok_or_else(|| {
                anyhow!("array field default item {} must be an object", index + 1)
            })?;
            for key in obj.keys() {
                if !ids.contains(key) {
                    return Err(anyhow!(
                        "array field default item {} contains unknown field '{}'",
                        index + 1,
                        key
                    ));
                }
            }
        }
    }

    Ok(())
}

fn validate_theme_page(page: &ThemePageDeclaration) -> Result<()> {
    validate_page_slug(&page.slug)?;
    validate_non_empty("page.title", &page.title, 80)?;
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

fn validate_settings_id(field: &str, value: &str) -> Result<()> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(anyhow!("{} cannot be empty", field));
    };

    let valid = value.len() <= 64
        && first.is_ascii_lowercase()
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_');
    if valid {
        Ok(())
    } else {
        Err(anyhow!("{} must match ^[a-z][a-z0-9_]{{0,63}}$", field))
    }
}

fn validate_field_type(field_type: &str) -> Result<()> {
    match field_type {
        "text" | "textarea" | "switch" | "select" | "number" | "color" | "array" => Ok(()),
        _ => Err(anyhow!("unsupported settings field type '{}'", field_type)),
    }
}

fn validate_color_default(value: &Value) -> Result<()> {
    let color = required_value_str(value, "field.default")?;
    let valid = color.len() == 7
        && color.starts_with('#')
        && color.chars().skip(1).all(|ch| ch.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(anyhow!("color default must be #RRGGBB"))
    }
}

fn validate_localized_text(value: Option<&Value>, field: &str, required: bool) -> Result<()> {
    let Some(value) = value else {
        if required {
            return Err(anyhow!("{} is required", field));
        }
        return Ok(());
    };

    match value {
        Value::String(text) => {
            if required && text.trim().is_empty() {
                Err(anyhow!("{} cannot be empty", field))
            } else {
                Ok(())
            }
        }
        Value::Object(map) => {
            if map.is_empty() {
                return Err(anyhow!("{} locale map cannot be empty", field));
            }
            for (locale, text) in map {
                if locale.trim().is_empty() {
                    return Err(anyhow!("{} locale key cannot be empty", field));
                }
                let text = text
                    .as_str()
                    .ok_or_else(|| anyhow!("{} locale value must be a string", field))?;
                if required && text.trim().is_empty() {
                    return Err(anyhow!("{} locale value cannot be empty", field));
                }
            }
            Ok(())
        }
        _ => Err(anyhow!("{} must be a string or locale map", field)),
    }
}

fn validate_non_empty(field: &str, value: &str, max_len: usize) -> Result<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("{} is required", field));
    }
    if trimmed.len() > max_len {
        return Err(anyhow!("{} must be at most {} characters", field, max_len));
    }
    Ok(())
}

fn reject_unknown_keys<'a>(
    keys: impl Iterator<Item = &'a String>,
    allowed: &[&str],
    context: &str,
) -> Result<()> {
    for key in keys {
        if !allowed.contains(&key.as_str()) {
            return Err(anyhow!("unknown key '{}' in {}", key, context));
        }
    }
    Ok(())
}

fn required_str<'a>(value: Option<&'a Value>, field: &str) -> Result<&'a str> {
    let value = value.ok_or_else(|| anyhow!("{} is required", field))?;
    required_value_str(value, field)
}

fn required_value_str<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    let text = value
        .as_str()
        .ok_or_else(|| anyhow!("{} must be a string", field))?;
    if text.trim().is_empty() {
        return Err(anyhow!("{} cannot be empty", field));
    }
    Ok(text)
}

fn require_string(value: &Value, field: &str) -> Result<()> {
    if value.is_string() {
        Ok(())
    } else {
        Err(anyhow!("{} must be a string", field))
    }
}

fn require_bool(value: &Value, field: &str) -> Result<()> {
    if value.is_boolean() {
        Ok(())
    } else {
        Err(anyhow!("{} must be a boolean", field))
    }
}

fn require_number(value: &Value, field: &str) -> Result<()> {
    if value.is_number() {
        Ok(())
    } else {
        Err(anyhow!("{} must be a number", field))
    }
}

fn coerce_string(value: &Value) -> Result<String> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Bool(boolean) => Ok(boolean.to_string()),
        Value::Null => Ok(String::new()),
        _ => Err(anyhow!("value must be string-compatible")),
    }
}

fn coerce_string_setting(field: &Value, value: &Value) -> Result<String> {
    let field_type = field.get("type").and_then(Value::as_str).unwrap_or("text");
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
    if field_type == "color" && !text.trim().is_empty() {
        validate_color_default(&Value::String(text.clone()))
            .with_context(|| format!("invalid color setting '{}'", field_id))?;
    }
    Ok(text)
}

fn coerce_bool(value: &Value) -> Result<bool> {
    match value {
        Value::Bool(boolean) => Ok(*boolean),
        Value::String(text) if text.eq_ignore_ascii_case("true") => Ok(true),
        Value::String(text) if text.eq_ignore_ascii_case("false") => Ok(false),
        _ => Err(anyhow!("value must be a boolean")),
    }
}

fn coerce_number(value: &Value) -> Result<f64> {
    match value {
        Value::Number(number) => number
            .as_f64()
            .ok_or_else(|| anyhow!("value must be a finite number")),
        Value::String(text) => text
            .parse::<f64>()
            .map_err(|_| anyhow!("value must be a number")),
        _ => Err(anyhow!("value must be a number")),
    }
}

fn is_empty_setting_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_theme_slug() {
        assert!(is_valid_theme_slug("default"));
        assert!(is_valid_theme_slug("my-theme-1"));
        assert!(!is_valid_theme_slug("MyTheme"));
        assert!(!is_valid_theme_slug("my_theme"));
        assert!(!is_valid_theme_slug(""));
    }

    #[test]
    fn validates_settings_schema() {
        let schema = json!({
            "schema": 1,
            "sections": [{
                "id": "appearance",
                "title": "Appearance",
                "fields": [{
                    "id": "show_toc",
                    "type": "switch",
                    "label": "Show TOC",
                    "default": true
                }]
            }]
        });

        validate_settings_schema(&schema).unwrap();
    }

    #[test]
    fn rejects_unsupported_settings_field() {
        let schema = json!({
            "schema": 1,
            "sections": [{
                "id": "appearance",
                "title": "Appearance",
                "fields": [{
                    "id": "hero_image",
                    "type": "image",
                    "label": "Hero image"
                }]
            }]
        });

        assert!(validate_settings_schema(&schema).is_err());
    }
}
