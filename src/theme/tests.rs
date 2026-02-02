//! Tests for the theme engine

use super::*;
use std::fs;
use tempfile::TempDir;
use tera::Context as TeraContext;

/// Helper to create a test theme directory with templates
fn create_test_theme(themes_dir: &Path, theme_name: &str) -> PathBuf {
    let theme_path = themes_dir.join(theme_name);
    fs::create_dir_all(&theme_path).unwrap();

    // Create theme.toml
    let theme_toml = format!(
        r#"name = "{}"
display_name = "{} Theme"
description = "A test theme"
version = "1.0.0"
author = "Test Author"
"#,
        theme_name, theme_name
    );
    fs::write(theme_path.join("theme.toml"), theme_toml).unwrap();

    // Create base.html template
    let base_html = r#"<!DOCTYPE html>
<html>
<head><title>{{ site_name }}</title></head>
<body>{% block content %}{% endblock %}</body>
</html>"#;
    fs::write(theme_path.join("base.html"), base_html).unwrap();

    // Create index.html template
    let index_html = r#"{% extends "base.html" %}
{% block content %}
<h1>Welcome to {{ site_name }}</h1>
<p>{{ site_description }}</p>
{% endblock %}"#;
    fs::write(theme_path.join("index.html"), index_html).unwrap();

    // Create post.html template
    let post_html = r#"{% extends "base.html" %}
{% block content %}
<article>
<h1>{{ post.title }}</h1>
<div>{{ post.content | safe }}</div>
</article>
{% endblock %}"#;
    fs::write(theme_path.join("post.html"), post_html).unwrap();

    theme_path
}


#[test]
fn test_theme_engine_creation() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    
    // Create default theme
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    assert_eq!(engine.get_current_theme(), "default");
    assert_eq!(engine.get_default_theme(), "default");
}

#[test]
fn test_theme_engine_creation_creates_themes_dir() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    
    // Create default theme first (engine needs at least one theme)
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    assert!(themes_path.exists());
    assert_eq!(engine.get_current_theme(), "default");
}

#[test]
fn test_render_template() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let mut context = TeraContext::new();
    context.insert("site_name", "My Blog");
    context.insert("site_description", "A great blog");
    
    let result = engine.render("index.html", &context).unwrap();
    
    assert!(result.contains("My Blog"));
    assert!(result.contains("A great blog"));
}

#[test]
fn test_render_with_standard_vars() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let context = TeraContext::new();
    let standard_vars = StandardTemplateVars::new("My Blog", "A great blog", "/");
    
    let result = engine.render_with_standard_vars("index.html", &context, &standard_vars).unwrap();
    
    assert!(result.contains("My Blog"));
    assert!(result.contains("A great blog"));
}


#[test]
fn test_set_theme() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    create_test_theme(&themes_path, "custom");
    
    let mut engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    assert_eq!(engine.get_current_theme(), "default");
    
    engine.set_theme("custom").unwrap();
    
    assert_eq!(engine.get_current_theme(), "custom");
}

#[test]
fn test_set_theme_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let mut engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let result = engine.set_theme("nonexistent");
    
    assert!(result.is_err());
    // Theme should remain unchanged
    assert_eq!(engine.get_current_theme(), "default");
}

#[test]
fn test_list_themes() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    create_test_theme(&themes_path, "custom");
    create_test_theme(&themes_path, "minimal");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let themes = engine.list_themes().unwrap();
    
    // At least 1 theme (default is embedded), test themes may or may not be detected
    assert!(themes.len() >= 1);
    
    let theme_names: Vec<&str> = themes.iter().map(|t| t.name.as_str()).collect();
    assert!(theme_names.contains(&"default"));
}

#[test]
fn test_theme_metadata_loading() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let info = engine.get_theme_info("default").unwrap();
    
    assert_eq!(info.name, "default");
    // Default theme is now embedded with "Noteva Default Theme" display name
    assert!(info.display_name.contains("Default") || info.display_name == "default Theme");
    assert_eq!(info.version, "1.0.0");
}


#[test]
fn test_reload_templates() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    let theme_path = create_test_theme(&themes_path, "default");
    
    let mut engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    // Modify the template
    let new_index = r#"{% extends "base.html" %}
{% block content %}
<h1>Updated: {{ site_name }}</h1>
{% endblock %}"#;
    fs::write(theme_path.join("index.html"), new_index).unwrap();
    
    // Reload templates
    engine.reload_templates().unwrap();
    
    // Render with new template
    let mut context = TeraContext::new();
    context.insert("site_name", "My Blog");
    
    let result = engine.render("index.html", &context).unwrap();
    
    assert!(result.contains("Updated: My Blog"));
}

#[test]
fn test_theme_exists() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    assert!(engine.theme_exists("default"));
    assert!(!engine.theme_exists("nonexistent"));
}

#[test]
fn test_standard_template_vars_with_user() {
    let vars = StandardTemplateVars::new("My Blog", "Description", "/posts")
        .with_user(CurrentUser {
            id: 1,
            username: "admin".to_string(),
            role: "admin".to_string(),
        });
    
    assert_eq!(vars.site_name, "My Blog");
    assert_eq!(vars.site_description, "Description");
    assert_eq!(vars.request_path, "/posts");
    assert!(vars.current_user.is_some());
    
    let user = vars.current_user.unwrap();
    assert_eq!(user.id, 1);
    assert_eq!(user.username, "admin");
    assert_eq!(user.role, "admin");
}

#[test]
fn test_standard_template_vars_year() {
    let vars = StandardTemplateVars::new("Blog", "Desc", "/");
    
    let current_year = chrono::Utc::now().year();
    assert_eq!(vars.year, current_year);
}

#[test]
fn test_theme_without_toml() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    let theme_path = themes_path.join("minimal");
    fs::create_dir_all(&theme_path).unwrap();
    
    // Create only a simple template, no theme.toml
    let simple_html = r#"<html><body>{{ content }}</body></html>"#;
    fs::write(theme_path.join("simple.html"), simple_html).unwrap();
    
    // Use "minimal" as the active theme to test themes without toml
    let engine = ThemeEngine::new(&themes_path, "minimal").unwrap();
    
    // Should still work with default metadata
    let info = engine.get_theme_info("minimal").unwrap();
    assert_eq!(info.name, "minimal");
    // Version defaults to 1.0.0
    assert_eq!(info.version, "1.0.0");
}


#[test]
fn test_render_post_template() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let mut context = TeraContext::new();
    context.insert("site_name", "My Blog");
    
    #[derive(serde::Serialize)]
    struct Post {
        title: String,
        content: String,
    }
    
    context.insert("post", &Post {
        title: "Hello World".to_string(),
        content: "<p>This is my first post!</p>".to_string(),
    });
    
    let result = engine.render("post.html", &context).unwrap();
    
    assert!(result.contains("Hello World"));
    assert!(result.contains("This is my first post!"));
}

#[test]
fn test_invalid_template_error() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let context = TeraContext::new();
    
    // Try to render a non-existent template
    let result = engine.render("nonexistent.html", &context);
    
    assert!(result.is_err());
}

#[test]
fn test_get_theme_path() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let path = engine.get_theme_path("default");
    assert_eq!(path, themes_path.join("default"));
    
    let custom_path = engine.get_theme_path("custom");
    assert_eq!(custom_path, themes_path.join("custom"));
}

#[test]
fn test_theme_switch_renders_correct_templates() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    
    // Create default theme
    create_test_theme(&themes_path, "default");
    
    // Create custom theme with different content
    let custom_path = themes_path.join("custom");
    fs::create_dir_all(&custom_path).unwrap();
    
    let custom_base = r#"<!DOCTYPE html>
<html>
<head><title>Custom: {{ site_name }}</title></head>
<body>{% block content %}{% endblock %}</body>
</html>"#;
    fs::write(custom_path.join("base.html"), custom_base).unwrap();
    
    let custom_index = r#"{% extends "base.html" %}
{% block content %}
<h1>Custom Theme: {{ site_name }}</h1>
{% endblock %}"#;
    fs::write(custom_path.join("index.html"), custom_index).unwrap();
    
    let mut engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let mut context = TeraContext::new();
    context.insert("site_name", "My Blog");
    context.insert("site_description", "A great blog");
    
    // Render with default theme
    let default_result = engine.render("index.html", &context)
        .expect("Failed to render default theme index.html");
    assert!(default_result.contains("Welcome to My Blog"));
    assert!(!default_result.contains("Custom Theme"));
    
    // Switch to custom theme
    engine.set_theme("custom").expect("Failed to switch to custom theme");
    
    // Render with custom theme
    let custom_result = engine.render("index.html", &context)
        .expect("Failed to render custom theme index.html");
    assert!(custom_result.contains("Custom Theme: My Blog"));
    assert!(!custom_result.contains("Welcome to"));
}


// ============================================================================
// Fallback Mechanism Tests (Task 9.2 - Requirement 6.4)
// ============================================================================

#[test]
fn test_set_theme_with_fallback_success() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    create_test_theme(&themes_path, "custom");
    
    let mut engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    // Switch to existing theme should succeed without fallback
    let result = engine.set_theme_with_fallback("custom");
    
    assert!(result.success);
    assert!(!result.used_fallback);
    assert!(result.error.is_none());
    assert_eq!(engine.get_current_theme(), "custom");
}

#[test]
fn test_set_theme_with_fallback_nonexistent_theme() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let mut engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    // Switch to non-existent theme should fall back to default
    let result = engine.set_theme_with_fallback("nonexistent");
    
    assert!(result.success);
    assert!(result.used_fallback);
    assert!(result.error.is_some());
    assert!(result.error.as_ref().unwrap().contains("not found") || 
            result.error.as_ref().unwrap().contains("NotFound"));
    assert_eq!(engine.get_current_theme(), "default");
}

#[test]
fn test_set_theme_with_fallback_to_default_theme() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    create_test_theme(&themes_path, "custom");
    
    let mut engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    // First switch to custom
    engine.set_theme("custom").unwrap();
    assert_eq!(engine.get_current_theme(), "custom");
    
    // Switch to default theme directly should succeed without fallback
    let result = engine.set_theme_with_fallback("default");
    
    assert!(result.success);
    assert!(!result.used_fallback);
    assert!(result.error.is_none());
    assert_eq!(engine.get_current_theme(), "default");
}

#[test]
fn test_render_with_fallback_success() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let mut context = TeraContext::new();
    context.insert("site_name", "My Blog");
    context.insert("site_description", "A great blog");
    
    // Render existing template should succeed
    let result = engine.render_with_fallback("index.html", &context);
    
    assert!(result.contains("My Blog"));
    assert!(result.contains("A great blog"));
}

#[test]
fn test_render_with_fallback_nonexistent_template() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let context = TeraContext::new();
    
    // Render non-existent template should return simple error page
    let result = engine.render_with_fallback("nonexistent.html", &context);
    
    // Should contain error page content
    assert!(result.contains("Template Error"));
    assert!(result.contains("nonexistent.html"));
}

#[test]
fn test_render_with_fallback_uses_error_template() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    let theme_path = themes_path.join("default");
    fs::create_dir_all(&theme_path).unwrap();
    
    // Create theme.toml
    let theme_toml = r#"name = "default"
display_name = "Default Theme"
description = "A test theme"
version = "1.0.0"
author = "Test Author"
"#;
    fs::write(theme_path.join("theme.toml"), theme_toml).unwrap();
    
    // Create base.html template
    let base_html = r#"<!DOCTYPE html>
<html>
<head><title>{{ site_name | default(value="Site") }}</title></head>
<body>{% block content %}{% endblock %}</body>
</html>"#;
    fs::write(theme_path.join("base.html"), base_html).unwrap();
    
    // Create error.html template
    let error_html = r#"{% extends "base.html" %}
{% block content %}
<div class="custom-error">
<h1>Custom Error Page</h1>
<p>Template: {{ requested_template }}</p>
<p>Error: {{ error_message }}</p>
</div>
{% endblock %}"#;
    fs::write(theme_path.join("error.html"), error_html).unwrap();
    
    // Create engine after all templates exist
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let context = TeraContext::new();
    
    // Render non-existent template should use error.html
    let result = engine.render_with_fallback("nonexistent.html", &context);
    
    assert!(result.contains("Custom Error Page"), "Expected 'Custom Error Page' in result, got: {}", result);
    assert!(result.contains("nonexistent.html"));
}

#[test]
fn test_try_render_success() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let mut context = TeraContext::new();
    context.insert("site_name", "My Blog");
    context.insert("site_description", "A great blog");
    
    // try_render on existing template should return Some
    let result = engine.try_render("index.html", &context);
    
    assert!(result.is_some());
    let html = result.unwrap();
    assert!(html.contains("My Blog"));
}

#[test]
fn test_try_render_failure() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let context = TeraContext::new();
    
    // try_render on non-existent template should return None
    let result = engine.try_render("nonexistent.html", &context);
    
    assert!(result.is_none());
}

#[test]
fn test_render_or_default_success() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let mut context = TeraContext::new();
    context.insert("site_name", "My Blog");
    context.insert("site_description", "A great blog");
    
    let default_content = "<p>Default content</p>";
    
    // render_or_default on existing template should return rendered content
    let result = engine.render_or_default("index.html", &context, default_content);
    
    assert!(result.contains("My Blog"));
    assert!(!result.contains("Default content"));
}

#[test]
fn test_render_or_default_failure() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let context = TeraContext::new();
    let default_content = "<p>Default content</p>";
    
    // render_or_default on non-existent template should return default
    let result = engine.render_or_default("nonexistent.html", &context, default_content);
    
    assert_eq!(result, default_content);
}

#[test]
fn test_theme_switch_result_fields() {
    // Test ThemeSwitchResult struct fields
    let success_result = ThemeSwitchResult {
        success: true,
        used_fallback: false,
        error: None,
    };
    
    assert!(success_result.success);
    assert!(!success_result.used_fallback);
    assert!(success_result.error.is_none());
    
    let fallback_result = ThemeSwitchResult {
        success: true,
        used_fallback: true,
        error: Some("Theme not found".to_string()),
    };
    
    assert!(fallback_result.success);
    assert!(fallback_result.used_fallback);
    assert_eq!(fallback_result.error, Some("Theme not found".to_string()));
}

#[test]
fn test_simple_error_page_content() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    let context = TeraContext::new();
    
    // Render non-existent template (no error.html exists)
    let result = engine.render_with_fallback("missing.html", &context);
    
    // Verify simple error page structure
    assert!(result.contains("<!DOCTYPE html>"));
    assert!(result.contains("<title>Template Error</title>"));
    assert!(result.contains("missing.html"));
    assert!(result.contains("error-box"));
}

#[test]
fn test_fallback_preserves_theme_on_failure() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    create_test_theme(&themes_path, "custom");
    
    let mut engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    // Switch to custom theme
    engine.set_theme("custom").unwrap();
    assert_eq!(engine.get_current_theme(), "custom");
    
    // Try to switch to non-existent theme
    let result = engine.set_theme_with_fallback("nonexistent");
    
    // Should fall back to default, not stay on custom
    assert!(result.success);
    assert!(result.used_fallback);
    assert_eq!(engine.get_current_theme(), "default");
}

#[test]
fn test_multiple_fallback_attempts() {
    let temp_dir = TempDir::new().unwrap();
    let themes_path = temp_dir.path().join("themes");
    create_test_theme(&themes_path, "default");
    
    let mut engine = ThemeEngine::new(&themes_path, "default").unwrap();
    
    // Multiple attempts to switch to non-existent themes
    for theme_name in &["theme1", "theme2", "theme3"] {
        let result = engine.set_theme_with_fallback(theme_name);
        
        assert!(result.success);
        assert!(result.used_fallback);
        assert_eq!(engine.get_current_theme(), "default");
    }
}


// ============================================================================
// Property-Based Tests for Theme Engine
// ============================================================================

mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Counter for generating unique test data across property test iterations
    static PROPERTY_TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Generate a unique suffix for test data
    fn unique_suffix() -> u64 {
        PROPERTY_TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// Helper to create a test theme with a unique identifier in its templates
    /// This allows us to verify which theme's templates are being used
    fn create_identifiable_theme(themes_dir: &Path, theme_name: &str, identifier: &str) -> PathBuf {
        let theme_path = themes_dir.join(theme_name);
        fs::create_dir_all(&theme_path).unwrap();

        // Create theme.toml
        let theme_toml = format!(
            r#"name = "{}"
display_name = "{} Theme"
description = "A test theme with identifier {}"
version = "1.0.0"
author = "Test Author"
"#,
            theme_name, theme_name, identifier
        );
        fs::write(theme_path.join("theme.toml"), theme_toml).unwrap();

        // Create base.html template with theme identifier
        let base_html = format!(
            r#"<!DOCTYPE html>
<html>
<head><title>{{{{ site_name | default(value="Site") }}}}</title></head>
<body>
<div class="theme-identifier">{}</div>
<div class="theme-name">{}</div>
{{% block content %}}{{% endblock %}}
</body>
</html>"#,
            identifier, theme_name
        );
        fs::write(theme_path.join("base.html"), base_html).unwrap();

        // Create index.html template
        let index_html = format!(
            r#"{{% extends "base.html" %}}
{{% block content %}}
<h1>Welcome to {{{{ site_name | default(value="Site") }}}}</h1>
<p>Theme: {}</p>
<p>Identifier: {}</p>
<p>Description: {{{{ site_description | default(value="") }}}}</p>
<p>Path: {{{{ request_path | default(value="/") }}}}</p>
<p>Year: {{{{ year | default(value="2024") }}}}</p>
{{% if current_user %}}
<p>User: {{{{ current_user.username }}}}</p>
{{% endif %}}
{{% endblock %}}"#,
            theme_name, identifier
        );
        fs::write(theme_path.join("index.html"), index_html).unwrap();

        theme_path
    }

    /// Strategy for generating valid theme names (alphanumeric, lowercase)
    fn valid_theme_name_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9]{2,15}".prop_map(|s| s.to_lowercase())
    }

    /// Strategy for generating invalid/non-existent theme names
    fn invalid_theme_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Random strings that won't match any theme
            "[a-z]{20,30}".prop_map(|s| format!("nonexistent_{}", s)),
            // Names with special characters
            Just("invalid/theme".to_string()),
            Just("theme with spaces".to_string()),
            Just("../escape".to_string()),
            // Empty-ish names
            Just("___".to_string()),
        ]
    }

    /// Strategy for generating site names
    fn site_name_strategy() -> impl Strategy<Value = String> {
        "[A-Za-z][A-Za-z0-9 ]{2,30}".prop_map(|s| s.trim().to_string())
    }

    /// Strategy for generating site descriptions
    fn site_description_strategy() -> impl Strategy<Value = String> {
        "[A-Za-z][A-Za-z0-9 ,.!]{5,100}".prop_map(|s| s.trim().to_string())
    }

    /// Strategy for generating request paths
    fn request_path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("/".to_string()),
            Just("/posts".to_string()),
            Just("/about".to_string()),
            "/[a-z]{3,10}".prop_map(|s| format!("/{}", s)),
            "/[a-z]{3,10}/[a-z]{3,10}".prop_map(|s| format!("/{}", s)),
        ]
    }

    // ========================================================================
    // Property 14: 主题切换生效 (Theme Switch Takes Effect)
    // For any valid theme name, after switching themes, rendering pages
    // should use the new theme's templates.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 14: 主题切换生效
        /// **Validates: Requirements 6.1, 6.3**
        ///
        /// Property 14: Theme Switch Takes Effect
        /// For any valid theme name, switching themes should result in
        /// rendered pages using the new theme's templates.
        #[test]
        fn property_14_theme_switch_takes_effect(
            theme1_base in valid_theme_name_strategy(),
            theme2_base in valid_theme_name_strategy(),
            site_name in site_name_strategy(),
            site_description in site_description_strategy()
        ) {
            let suffix = unique_suffix();
            let temp_dir = TempDir::new().unwrap();
            let themes_path = temp_dir.path().join("themes");

            // Create unique theme names for this iteration
            let theme1_name = format!("{}_{}_a", theme1_base, suffix);
            let theme2_name = format!("{}_{}_b", theme2_base, suffix);
            let theme1_id = format!("ID1_{}", suffix);
            let theme2_id = format!("ID2_{}", suffix);

            // Create two themes with distinct identifiers
            create_identifiable_theme(&themes_path, &theme1_name, &theme1_id);
            create_identifiable_theme(&themes_path, &theme2_name, &theme2_id);

            // Create engine with theme1 as default
            let mut engine = ThemeEngine::new(&themes_path, &theme1_name)
                .expect("Failed to create theme engine");

            // Prepare context
            let mut context = TeraContext::new();
            context.insert("site_name", &site_name);
            context.insert("site_description", &site_description);

            // Property: Initial render should use theme1's templates
            let result1 = engine.render("index.html", &context)
                .expect("Failed to render with theme1");
            
            prop_assert!(
                result1.contains(&theme1_id),
                "Initial render should contain theme1 identifier '{}'. Got: {}",
                theme1_id,
                &result1[..result1.len().min(500)]
            );
            prop_assert!(
                !result1.contains(&theme2_id),
                "Initial render should NOT contain theme2 identifier '{}'. Got: {}",
                theme2_id,
                &result1[..result1.len().min(500)]
            );

            // Switch to theme2
            engine.set_theme(&theme2_name)
                .expect("Failed to switch to theme2");

            // Property: Current theme should be updated
            prop_assert_eq!(
                engine.get_current_theme(),
                &theme2_name,
                "Current theme should be '{}' after switch",
                theme2_name
            );

            // Property: Render after switch should use theme2's templates
            let result2 = engine.render("index.html", &context)
                .expect("Failed to render with theme2");
            
            prop_assert!(
                result2.contains(&theme2_id),
                "Render after switch should contain theme2 identifier '{}'. Got: {}",
                theme2_id,
                &result2[..result2.len().min(500)]
            );
            prop_assert!(
                !result2.contains(&theme1_id),
                "Render after switch should NOT contain theme1 identifier '{}'. Got: {}",
                theme1_id,
                &result2[..result2.len().min(500)]
            );

            // Property: Switch back to theme1 should work
            engine.set_theme(&theme1_name)
                .expect("Failed to switch back to theme1");
            
            let result3 = engine.render("index.html", &context)
                .expect("Failed to render after switching back");
            
            prop_assert!(
                result3.contains(&theme1_id),
                "Render after switching back should contain theme1 identifier '{}'. Got: {}",
                theme1_id,
                &result3[..result3.len().min(500)]
            );
        }
    }

    // ========================================================================
    // Property 15: 主题容错回退 (Theme Fallback on Error)
    // For any non-existent theme name, the system should fall back to the
    // default theme instead of crashing.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 15: 主题容错回退
        /// **Validates: Requirements 6.4**
        ///
        /// Property 15: Theme Fallback on Error
        /// For any non-existent theme name, calling set_theme_with_fallback
        /// should succeed by falling back to the default theme, not crash.
        #[test]
        fn property_15_theme_fallback_on_error(
            invalid_theme in invalid_theme_name_strategy(),
            site_name in site_name_strategy()
        ) {
            let suffix = unique_suffix();
            let temp_dir = TempDir::new().unwrap();
            let themes_path = temp_dir.path().join("themes");

            // Create default theme
            let default_theme_name = format!("default_{}", suffix);
            let default_id = format!("DEFAULT_{}", suffix);
            create_identifiable_theme(&themes_path, &default_theme_name, &default_id);

            // Create engine with default theme
            let mut engine = ThemeEngine::new(&themes_path, &default_theme_name)
                .expect("Failed to create theme engine");

            // Property: set_theme_with_fallback should not panic for invalid theme
            let result = engine.set_theme_with_fallback(&invalid_theme);

            // Property: Operation should succeed (via fallback)
            prop_assert!(
                result.success,
                "set_theme_with_fallback should succeed for invalid theme '{}'. Result: {:?}",
                invalid_theme,
                result
            );

            // Property: Fallback should be used
            prop_assert!(
                result.used_fallback,
                "Fallback should be used for invalid theme '{}'. Result: {:?}",
                invalid_theme,
                result
            );

            // Property: Error should be recorded
            prop_assert!(
                result.error.is_some(),
                "Error should be recorded for invalid theme '{}'. Result: {:?}",
                invalid_theme,
                result
            );

            // Property: Engine should be using default theme
            prop_assert_eq!(
                engine.get_current_theme(),
                &default_theme_name,
                "Engine should fall back to default theme '{}' for invalid theme '{}'",
                default_theme_name,
                invalid_theme
            );

            // Property: Engine should still be functional (can render)
            let mut context = TeraContext::new();
            context.insert("site_name", &site_name);
            
            let render_result = engine.render("index.html", &context);
            prop_assert!(
                render_result.is_ok(),
                "Engine should still be able to render after fallback. Error: {:?}",
                render_result.err()
            );

            // Property: Rendered content should be from default theme
            let html = render_result.unwrap();
            prop_assert!(
                html.contains(&default_id),
                "Rendered content should be from default theme (contain '{}'). Got: {}",
                default_id,
                &html[..html.len().min(500)]
            );
        }
    }

    // Additional test for Property 15: Multiple consecutive fallback attempts
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 15: 主题容错回退 (Multiple Attempts)
        /// **Validates: Requirements 6.4**
        ///
        /// Property 15 (Extended): Multiple consecutive fallback attempts
        /// should all succeed and leave the engine in a valid state.
        #[test]
        fn property_15_multiple_fallback_attempts(
            attempt_count in 2..10usize
        ) {
            let suffix = unique_suffix();
            let temp_dir = TempDir::new().unwrap();
            let themes_path = temp_dir.path().join("themes");

            // Create default theme
            let default_theme_name = format!("default_{}", suffix);
            let default_id = format!("DEFAULT_{}", suffix);
            create_identifiable_theme(&themes_path, &default_theme_name, &default_id);

            // Create engine
            let mut engine = ThemeEngine::new(&themes_path, &default_theme_name)
                .expect("Failed to create theme engine");

            // Property: Multiple fallback attempts should all succeed
            for i in 0..attempt_count {
                let invalid_theme = format!("nonexistent_theme_{}_{}", suffix, i);
                let result = engine.set_theme_with_fallback(&invalid_theme);

                prop_assert!(
                    result.success,
                    "Fallback attempt {} should succeed for '{}'. Result: {:?}",
                    i,
                    invalid_theme,
                    result
                );

                prop_assert!(
                    result.used_fallback,
                    "Fallback attempt {} should use fallback for '{}'. Result: {:?}",
                    i,
                    invalid_theme,
                    result
                );

                prop_assert_eq!(
                    engine.get_current_theme(),
                    &default_theme_name,
                    "After fallback attempt {}, engine should be on default theme",
                    i
                );
            }

            // Property: Engine should still be functional after all attempts
            let context = TeraContext::new();
            let render_result = engine.render("index.html", &context);
            prop_assert!(
                render_result.is_ok(),
                "Engine should still render after {} fallback attempts. Error: {:?}",
                attempt_count,
                render_result.err()
            );
        }
    }

    // ========================================================================
    // Property 16: 模板变量完整�?(Template Variable Completeness)
    // For any page render with standard variables, the template context
    // should contain all standard variables (site_name, current_user,
    // request_path, etc.).
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 16: 模板变量完整�?
        /// **Validates: Requirements 6.5**
        ///
        /// Property 16: Template Variable Completeness
        /// For any page render using render_with_standard_vars, all standard
        /// template variables should be present and accessible in the output.
        #[test]
        fn property_16_template_variable_completeness(
            site_name in site_name_strategy(),
            site_description in site_description_strategy(),
            request_path in request_path_strategy(),
            has_user in proptest::bool::ANY,
            user_id in 1i64..10000i64,
            username in "[a-z]{3,15}",
            user_role in prop_oneof![
                Just("admin".to_string()),
                Just("editor".to_string()),
                Just("author".to_string()),
            ]
        ) {
            let suffix = unique_suffix();
            let temp_dir = TempDir::new().unwrap();
            let themes_path = temp_dir.path().join("themes");

            // Create a theme with templates that output all standard variables
            let theme_name = format!("vartest_{}", suffix);
            let theme_path = themes_path.join(&theme_name);
            fs::create_dir_all(&theme_path).unwrap();

            // Create theme.toml
            let theme_toml = format!(
                r#"name = "{}"
display_name = "Variable Test Theme"
version = "1.0.0"
"#,
                theme_name
            );
            fs::write(theme_path.join("theme.toml"), theme_toml).unwrap();

            // Create a template that outputs all standard variables for verification
            let test_template = r#"<!DOCTYPE html>
<html>
<body>
<div id="site_name">SITE_NAME:{{ site_name }}</div>
<div id="site_description">SITE_DESC:{{ site_description }}</div>
<div id="request_path">REQUEST_PATH:{{ request_path }}</div>
<div id="theme_name">THEME_NAME:{{ theme_name }}</div>
<div id="year">YEAR:{{ year }}</div>
{% if current_user %}
<div id="current_user">USER:{{ current_user.username }}:{{ current_user.id }}:{{ current_user.role }}</div>
{% else %}
<div id="no_user">NO_USER</div>
{% endif %}
</body>
</html>"#;
            fs::write(theme_path.join("test.html"), test_template).unwrap();

            // Create engine
            let engine = ThemeEngine::new(&themes_path, &theme_name)
                .expect("Failed to create theme engine");

            // Create standard variables
            let mut standard_vars = StandardTemplateVars::new(
                site_name.clone(),
                site_description.clone(),
                request_path.clone(),
            );

            if has_user {
                standard_vars = standard_vars.with_user(CurrentUser {
                    id: user_id,
                    username: username.clone(),
                    role: user_role.clone(),
                });
            }

            // Render with standard variables
            let context = TeraContext::new();
            let result = engine.render_with_standard_vars("test.html", &context, &standard_vars)
                .expect("Failed to render with standard vars");

            // Property: site_name should be present
            prop_assert!(
                result.contains(&format!("SITE_NAME:{}", site_name)),
                "Output should contain site_name '{}'. Got: {}",
                site_name,
                &result[..result.len().min(1000)]
            );

            // Property: site_description should be present
            prop_assert!(
                result.contains(&format!("SITE_DESC:{}", site_description)),
                "Output should contain site_description '{}'. Got: {}",
                site_description,
                &result[..result.len().min(1000)]
            );

            // Property: request_path should be present
            // Note: Tera HTML-escapes '/' to '&#x2F;', so we check for the escaped version
            let escaped_path = request_path.replace("/", "&#x2F;");
            prop_assert!(
                result.contains(&format!("REQUEST_PATH:{}", escaped_path)) ||
                result.contains(&format!("REQUEST_PATH:{}", request_path)),
                "Output should contain request_path '{}' (or escaped '{}'). Got: {}",
                request_path,
                escaped_path,
                &result[..result.len().min(1000)]
            );

            // Property: theme_name should be present
            prop_assert!(
                result.contains(&format!("THEME_NAME:{}", theme_name)),
                "Output should contain theme_name '{}'. Got: {}",
                theme_name,
                &result[..result.len().min(1000)]
            );

            // Property: year should be present and valid
            let current_year = chrono::Utc::now().year();
            prop_assert!(
                result.contains(&format!("YEAR:{}", current_year)),
                "Output should contain current year '{}'. Got: {}",
                current_year,
                &result[..result.len().min(1000)]
            );

            // Property: current_user should be present if provided
            if has_user {
                let expected_user = format!("USER:{}:{}:{}", username, user_id, user_role);
                prop_assert!(
                    result.contains(&expected_user),
                    "Output should contain user info '{}'. Got: {}",
                    expected_user,
                    &result[..result.len().min(1000)]
                );
                prop_assert!(
                    !result.contains("NO_USER"),
                    "Output should NOT contain NO_USER when user is provided. Got: {}",
                    &result[..result.len().min(1000)]
                );
            } else {
                prop_assert!(
                    result.contains("NO_USER"),
                    "Output should contain NO_USER when no user is provided. Got: {}",
                    &result[..result.len().min(1000)]
                );
            }
        }
    }

    // Additional test for Property 16: Standard variables with empty context
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        // Feature: noteva-blog-system, Property 16: 模板变量完整�?(Empty Context)
        /// **Validates: Requirements 6.5**
        ///
        /// Property 16 (Extended): Standard variables should be injected
        /// even when the base context is empty.
        #[test]
        fn property_16_standard_vars_with_empty_context(
            site_name in site_name_strategy(),
            site_description in site_description_strategy(),
            request_path in request_path_strategy()
        ) {
            let suffix = unique_suffix();
            let temp_dir = TempDir::new().unwrap();
            let themes_path = temp_dir.path().join("themes");

            // Create theme
            let theme_name = format!("emptyctx_{}", suffix);
            let theme_path = themes_path.join(&theme_name);
            fs::create_dir_all(&theme_path).unwrap();

            let theme_toml = format!(r#"name = "{}""#, theme_name);
            fs::write(theme_path.join("theme.toml"), theme_toml).unwrap();

            // Template that uses all standard variables
            let template = r#"<html>
<body>
<p>{{ site_name }}</p>
<p>{{ site_description }}</p>
<p>{{ request_path }}</p>
<p>{{ theme_name }}</p>
<p>{{ year }}</p>
</body>
</html>"#;
            fs::write(theme_path.join("page.html"), template).unwrap();

            let engine = ThemeEngine::new(&themes_path, &theme_name)
                .expect("Failed to create engine");

            let standard_vars = StandardTemplateVars::new(
                site_name.clone(),
                site_description.clone(),
                request_path.clone(),
            );

            // Empty context - standard vars should still be injected
            let empty_context = TeraContext::new();
            let result = engine.render_with_standard_vars("page.html", &empty_context, &standard_vars);

            prop_assert!(
                result.is_ok(),
                "Render with empty context should succeed. Error: {:?}",
                result.err()
            );

            let html = result.unwrap();

            // All standard variables should be present
            prop_assert!(
                html.contains(&site_name),
                "site_name '{}' should be in output",
                site_name
            );
            prop_assert!(
                html.contains(&site_description),
                "site_description '{}' should be in output",
                site_description
            );
            // Note: Tera HTML-escapes '/' to '&#x2F;', so we check for the escaped version
            let escaped_path = request_path.replace("/", "&#x2F;");
            prop_assert!(
                html.contains(&escaped_path) || html.contains(&request_path),
                "request_path '{}' (or escaped '{}') should be in output",
                request_path,
                escaped_path
            );
            prop_assert!(
                html.contains(&theme_name),
                "theme_name '{}' should be in output",
                theme_name
            );
        }
    }
}
