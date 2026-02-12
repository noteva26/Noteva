//! Theme engine
//!
//! This module provides template rendering using Tera.
//! Features:
//! - Theme loading and switching
//! - Template hot-reload
//! - Standard template variables
//! - Fallback to default theme

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tera::{Context as TeraContext, Tera};
use tracing;

use crate::plugin::HookManager;
use crate::plugin::loader::{check_version_requirement, NOTEVA_VERSION};

mod error;

pub use error::ThemeError;

/// Theme engine for rendering templates
pub struct ThemeEngine {
    /// Tera template engine instance
    tera: Tera,
    /// Path to themes directory
    themes_path: PathBuf,
    /// Currently active theme name
    current_theme: String,
    /// Default theme name (fallback)
    default_theme: String,
    /// Cached theme metadata
    theme_cache: HashMap<String, ThemeInfo>,
    /// Hook manager for triggering theme_switch hook
    hook_manager: Option<Arc<HookManager>>,
}

/// Result of a theme switch operation with fallback support
/// 
/// This struct provides detailed information about what happened during
/// a theme switch operation, including whether a fallback was used.
#[derive(Debug, Clone)]
pub struct ThemeSwitchResult {
    /// Whether the theme switch was successful (either directly or via fallback)
    pub success: bool,
    /// Whether the fallback theme was used instead of the requested theme
    pub used_fallback: bool,
    /// Error message if the original theme switch failed (before fallback)
    pub error: Option<String>,
}

impl ThemeEngine {
    /// Create a new theme engine
    ///
    /// # Arguments
    /// * `themes_path` - Path to the themes directory
    /// * `default_theme` - Name of the default theme to use
    ///
    /// # Returns
    /// A new ThemeEngine instance or an error if initialization fails
    pub fn new(themes_path: &Path, default_theme: &str) -> Result<Self> {
        let themes_path = themes_path.to_path_buf();
        
        // Ensure themes directory exists
        if !themes_path.exists() {
            fs::create_dir_all(&themes_path)
                .with_context(|| format!("Failed to create themes directory: {:?}", themes_path))?;
        }
        
        // Ensure default theme directory exists
        let default_theme_path = themes_path.join(default_theme);
        if !default_theme_path.exists() {
            fs::create_dir_all(&default_theme_path)
                .with_context(|| format!("Failed to create default theme directory: {:?}", default_theme_path))?;
        }
        
        // Note: For the default theme, we use embedded metadata instead of creating theme.json
        // This keeps the themes/default directory clean (only dist/ folder)

        let mut engine = Self {
            tera: Tera::default(),
            themes_path,
            current_theme: default_theme.to_string(),
            default_theme: default_theme.to_string(),
            theme_cache: HashMap::new(),
            hook_manager: None,
        };

        // Load templates for the default theme (may be empty for embedded themes)
        // This won't fail if there are no .html templates - embedded themes serve static files
        if let Err(e) = engine.load_theme_templates(default_theme) {
            tracing::warn!("No templates found for theme '{}': {} (this is OK for embedded themes)", default_theme, e);
        }
        
        // Cache theme metadata
        engine.refresh_theme_cache()?;

        Ok(engine)
    }
    
    /// Set the hook manager for triggering theme_switch hook
    pub fn with_hooks(mut self, hook_manager: Arc<HookManager>) -> Self {
        self.hook_manager = Some(hook_manager);
        self
    }
    
    /// Set the hook manager (for existing instances)
    pub fn set_hook_manager(&mut self, hook_manager: Arc<HookManager>) {
        self.hook_manager = Some(hook_manager);
    }
    
    /// Trigger theme_switch hook
    fn trigger_theme_switch_hook(&self, old_theme: &str, new_theme: &str) {
        if let Some(ref hook_manager) = self.hook_manager {
            let data = serde_json::json!({
                "old_theme": old_theme,
                "new_theme": new_theme,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            hook_manager.trigger(crate::plugin::hook_names::THEME_SWITCH, data);
        }
    }

    /// Load templates for a specific theme
    fn load_theme_templates(&mut self, theme_name: &str) -> Result<()> {
        let theme_path = self.themes_path.join(theme_name);
        
        if !theme_path.exists() {
            return Err(ThemeError::NotFound(theme_name.to_string()).into());
        }

        // Try to load from dist/ directory first (for built themes like Next.js/Nuxt)
        let dist_path = theme_path.join("dist");
        let template_path = if dist_path.exists() && dist_path.is_dir() {
            dist_path
        } else {
            theme_path.clone()
        };

        // Create a new Tera instance
        let mut tera = Tera::default();
        
        // Collect all templates first
        let mut templates: Vec<(String, String)> = Vec::new();
        self.collect_templates_from_dir(&template_path, &template_path, &mut templates)?;
        
        // Sort templates so base templates are loaded first
        templates.sort_by(|a, b| {
            let a_is_base = a.0 == "base.html" || a.0.ends_with("/base.html");
            let b_is_base = b.0 == "base.html" || b.0.ends_with("/base.html");
            b_is_base.cmp(&a_is_base)
        });
        
        // Add all templates
        for (name, content) in templates {
            tera.add_raw_template(&name, &content)
                .map_err(|e| ThemeError::TemplateError(format!("Failed to add template {}: {}", name, e)))?;
        }
        
        // Build inheritance chains after adding all templates
        tera.build_inheritance_chains()
            .map_err(|e| ThemeError::TemplateError(format!("Failed to build template inheritance: {}", e)))?;
        
        self.tera = tera;
        Ok(())
    }

    /// Collect templates from a directory
    fn collect_templates_from_dir(&self, base_path: &Path, current_path: &Path, templates: &mut Vec<(String, String)>) -> Result<()> {
        if !current_path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(current_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                self.collect_templates_from_dir(base_path, &path, templates)?;
            } else if path.extension().map_or(false, |ext| ext == "html") {
                let relative_path = path.strip_prefix(base_path)
                    .map_err(|_| ThemeError::TemplateError("Failed to get relative path".to_string()))?;
                
                let template_name = relative_path
                    .to_string_lossy()
                    .replace('\\', "/");
                
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read template: {:?}", path))?;
                
                templates.push((template_name, content));
            }
        }
        
        Ok(())
    }

    /// Recursively add templates from a directory with relative names
    fn add_templates_from_dir(&self, tera: &mut Tera, base_path: &Path, current_path: &Path) -> Result<()> {
        if !current_path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(current_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                self.add_templates_from_dir(tera, base_path, &path)?;
            } else if path.extension().map_or(false, |ext| ext == "html") {
                // Get relative path from theme root
                let relative_path = path.strip_prefix(base_path)
                    .map_err(|_| ThemeError::TemplateError("Failed to get relative path".to_string()))?;
                
                // Convert to forward slashes for template name (cross-platform)
                let template_name = relative_path
                    .to_string_lossy()
                    .replace('\\', "/");
                
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read template: {:?}", path))?;
                
                tera.add_raw_template(&template_name, &content)
                    .map_err(|e| ThemeError::TemplateError(format!("Failed to add template {}: {}", template_name, e)))?;
            }
        }
        
        Ok(())
    }

    /// Refresh the theme metadata cache
    fn refresh_theme_cache(&mut self) -> Result<()> {
        self.theme_cache.clear();
        
        if !self.themes_path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.themes_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(theme_name) = path.file_name().and_then(|n| n.to_str()) {
                    match self.load_theme_metadata(theme_name) {
                        Ok(info) => {
                            self.theme_cache.insert(theme_name.to_string(), info);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load theme metadata for '{}': {}", theme_name, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Load theme metadata from theme.json or theme.toml
    fn load_theme_metadata(&self, theme_name: &str) -> Result<ThemeInfo> {
        let theme_json_path = self.themes_path.join(theme_name).join("theme.json");
        let theme_toml_path = self.themes_path.join(theme_name).join("theme.toml");
        
        // Try theme.json first (for ALL themes, including default)
        if theme_json_path.exists() {
            let content = fs::read_to_string(&theme_json_path)
                .with_context(|| format!("Failed to read theme.json: {:?}", theme_json_path))?;
            
            let metadata: ThemeJsonMetadata = serde_json::from_str(&content)
                .map_err(|e| ThemeError::InvalidMetadata(format!("theme '{}': {}", theme_name, e)))?;

            let requires_noteva = metadata.requires.as_ref()
                .map(|r| r.noteva.clone())
                .unwrap_or_default();
            let version_check = check_version_requirement(&requires_noteva, NOTEVA_VERSION);

            return Ok(ThemeInfo {
                name: metadata.short.unwrap_or_else(|| theme_name.to_string()),
                display_name: metadata.name,
                description: metadata.description,
                version: metadata.version,
                author: metadata.author,
                url: metadata.url,
                preview: metadata.preview,
                requires_noteva,
                compatible: version_check.compatible,
                compatibility_message: version_check.message,
                config: metadata.configuration,
            });
        }
        
        // Fall back to theme.toml
        if theme_toml_path.exists() {
            let content = fs::read_to_string(&theme_toml_path)
                .with_context(|| format!("Failed to read theme.toml: {:?}", theme_toml_path))?;
            
            let metadata: ThemeMetadata = toml::from_str(&content)
                .map_err(|e| ThemeError::InvalidMetadata(format!("theme '{}': {}", theme_name, e)))?;

            return Ok(ThemeInfo {
                name: metadata.theme.name,
                display_name: metadata.theme.display_name,
                description: metadata.theme.description,
                version: metadata.theme.version,
                author: metadata.theme.author,
                url: None,
                preview: None,
                requires_noteva: String::new(),
                compatible: true,
                compatibility_message: None,
                config: None,
            });
        }
        
        // Fallback: hardcoded metadata for default theme (when no theme.json on disk)
        if theme_name == self.default_theme {
            let version_check = check_version_requirement(">=0.0.8", NOTEVA_VERSION);
            return Ok(ThemeInfo {
                name: self.default_theme.clone(),
                display_name: "Noteva Default Theme".to_string(),
                description: Some("The default theme for Noteva blog system".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                author: Some("Noteva Team".to_string()),
                url: Some("https://github.com/noteva26/Noteva".to_string()),
                preview: Some("preview.png".to_string()),
                requires_noteva: ">=0.0.8".to_string(),
                compatible: version_check.compatible,
                compatibility_message: version_check.message,
                config: None,
            });
        }
        
        // Return default metadata if no config file exists
        Ok(ThemeInfo {
            name: theme_name.to_string(),
            display_name: theme_name.to_string(),
            description: None,
            version: "0.0.0".to_string(),
            author: None,
            url: None,
            preview: None,
            requires_noteva: String::new(),
            compatible: true,
            compatibility_message: None,
            config: None,
        })
    }

    /// Render a template with context
    ///
    /// # Arguments
    /// * `template` - Template name (e.g., "index.html", "post.html")
    /// * `context` - Tera context with template variables
    ///
    /// # Returns
    /// Rendered HTML string or an error
    pub fn render(&self, template: &str, context: &TeraContext) -> Result<String> {
        self.tera
            .render(template, context)
            .map_err(|e| {
                let mut error_msg = format!("Failed to render '{}': {}", template, e);
                let mut source = e.source();
                while let Some(s) = source {
                    error_msg.push_str(&format!("\n  Caused by: {}", s));
                    source = s.source();
                }
                ThemeError::TemplateError(error_msg).into()
            })
    }

    /// Render a template with standard variables automatically added
    ///
    /// # Arguments
    /// * `template` - Template name
    /// * `context` - Base context (standard variables will be added)
    /// * `standard_vars` - Standard template variables to inject
    ///
    /// # Returns
    /// Rendered HTML string or an error
    pub fn render_with_standard_vars(
        &self,
        template: &str,
        context: &TeraContext,
        standard_vars: &StandardTemplateVars,
    ) -> Result<String> {
        let mut full_context = context.clone();
        
        // Add standard variables
        full_context.insert("site_name", &standard_vars.site_name);
        full_context.insert("site_description", &standard_vars.site_description);
        full_context.insert("request_path", &standard_vars.request_path);
        full_context.insert("theme_name", &self.current_theme);
        full_context.insert("year", &standard_vars.year);
        
        if let Some(ref user) = standard_vars.current_user {
            full_context.insert("current_user", user);
        }

        self.render(template, &full_context)
    }

    /// Set the active theme
    ///
    /// # Arguments
    /// * `theme_name` - Name of the theme to switch to
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if the theme doesn't exist
    /// 
    /// Triggers `theme_switch` hook with old and new theme names
    pub fn set_theme(&mut self, theme_name: &str) -> Result<()> {
        let theme_path = self.themes_path.join(theme_name);
        
        if !theme_path.exists() {
            return Err(ThemeError::NotFound(theme_name.to_string()).into());
        }

        let old_theme = self.current_theme.clone();
        self.load_theme_templates(theme_name)?;
        self.current_theme = theme_name.to_string();
        
        // Trigger theme_switch hook
        if old_theme != theme_name {
            self.trigger_theme_switch_hook(&old_theme, theme_name);
        }
        
        Ok(())
    }

    /// Set theme with automatic fallback to default theme
    ///
    /// This method attempts to set the requested theme. If the theme doesn't exist
    /// or fails to load, it automatically falls back to the default theme.
    ///
    /// # Arguments
    /// * `theme_name` - Name of the theme to switch to
    ///
    /// # Returns
    /// A `ThemeSwitchResult` indicating whether the switch was successful,
    /// whether fallback was used, and any error that occurred.
    ///
    /// # Validates: Requirement 6.4
    pub fn set_theme_with_fallback(&mut self, theme_name: &str) -> ThemeSwitchResult {
        // If requesting the default theme, just try to set it directly
        if theme_name == self.default_theme {
            match self.set_theme(theme_name) {
                Ok(()) => {
                    return ThemeSwitchResult {
                        success: true,
                        used_fallback: false,
                        error: None,
                    };
                }
                Err(e) => {
                    tracing::error!("Failed to set default theme '{}': {}", theme_name, e);
                    return ThemeSwitchResult {
                        success: false,
                        used_fallback: false,
                        error: Some(e.to_string()),
                    };
                }
            }
        }

        // Try to set the requested theme
        match self.set_theme(theme_name) {
            Ok(()) => {
                ThemeSwitchResult {
                    success: true,
                    used_fallback: false,
                    error: None,
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                tracing::warn!(
                    "Theme '{}' not available ({}), falling back to default theme '{}'",
                    theme_name,
                    error_msg,
                    self.default_theme
                );

                // Try to fall back to default theme
                match self.set_theme(&self.default_theme.clone()) {
                    Ok(()) => {
                        ThemeSwitchResult {
                            success: true,
                            used_fallback: true,
                            error: Some(error_msg),
                        }
                    }
                    Err(fallback_err) => {
                        tracing::error!(
                            "Failed to fall back to default theme '{}': {}",
                            self.default_theme,
                            fallback_err
                        );
                        ThemeSwitchResult {
                            success: false,
                            used_fallback: true,
                            error: Some(format!(
                                "Original error: {}; Fallback error: {}",
                                error_msg, fallback_err
                            )),
                        }
                    }
                }
            }
        }
    }

    /// Render a template with fallback to error template or simple HTML
    ///
    /// This method attempts to render the requested template. If it fails,
    /// it tries to render a generic "error.html" template. If that also fails,
    /// it returns a simple HTML error page.
    ///
    /// # Arguments
    /// * `template` - Template name to render
    /// * `context` - Tera context with template variables
    ///
    /// # Returns
    /// Rendered HTML string (always succeeds, may return error page)
    pub fn render_with_fallback(&self, template: &str, context: &TeraContext) -> String {
        // Try to render the requested template
        match self.render(template, context) {
            Ok(html) => html,
            Err(e) => {
                tracing::warn!(
                    "Failed to render template '{}': {}, trying error template",
                    template,
                    e
                );

                // Try to render error.html template
                let mut error_context = context.clone();
                error_context.insert("error_message", &e.to_string());
                error_context.insert("requested_template", template);

                match self.render("error.html", &error_context) {
                    Ok(html) => html,
                    Err(error_template_err) => {
                        tracing::warn!(
                            "Failed to render error template: {}, returning simple HTML error page",
                            error_template_err
                        );

                        // Return a simple HTML error page as last resort
                        Self::simple_error_page(template, &e.to_string())
                    }
                }
            }
        }
    }

    /// Try to render a template, returning None on error
    ///
    /// This is a convenience method that converts render errors to None,
    /// useful when template rendering failure should be handled gracefully.
    ///
    /// # Arguments
    /// * `template` - Template name to render
    /// * `context` - Tera context with template variables
    ///
    /// # Returns
    /// Some(html) if rendering succeeds, None if it fails
    pub fn try_render(&self, template: &str, context: &TeraContext) -> Option<String> {
        match self.render(template, context) {
            Ok(html) => Some(html),
            Err(e) => {
                tracing::debug!("try_render failed for '{}': {}", template, e);
                None
            }
        }
    }

    /// Render a template or return default content on error
    ///
    /// This method attempts to render the template and returns the provided
    /// default string if rendering fails.
    ///
    /// # Arguments
    /// * `template` - Template name to render
    /// * `context` - Tera context with template variables
    /// * `default` - Default string to return on error
    ///
    /// # Returns
    /// Rendered HTML string or the default string
    pub fn render_or_default(&self, template: &str, context: &TeraContext, default: &str) -> String {
        match self.render(template, context) {
            Ok(html) => html,
            Err(e) => {
                tracing::debug!(
                    "render_or_default: template '{}' failed ({}), using default",
                    template,
                    e
                );
                default.to_string()
            }
        }
    }

    /// Generate a simple HTML error page
    ///
    /// This is used as a last resort when both the requested template
    /// and the error template fail to render.
    fn simple_error_page(template: &str, error: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Template Error</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 600px;
            margin: 50px auto;
            padding: 20px;
            background: #f5f5f5;
        }}
        .error-box {{
            background: white;
            border-left: 4px solid #e74c3c;
            padding: 20px;
            border-radius: 4px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        h1 {{ color: #e74c3c; margin-top: 0; }}
        code {{
            background: #f8f8f8;
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 0.9em;
        }}
        .details {{
            margin-top: 15px;
            padding-top: 15px;
            border-top: 1px solid #eee;
            color: #666;
            font-size: 0.9em;
        }}
    </style>
</head>
<body>
    <div class="error-box">
        <h1>Template Error</h1>
        <p>Failed to render template: <code>{}</code></p>
        <div class="details">
            <strong>Error:</strong> {}
        </div>
    </div>
</body>
</html>"#,
            template, error
        )
    }

    /// Get the current theme name
    pub fn get_current_theme(&self) -> &str {
        &self.current_theme
    }

    /// Get the default theme name
    pub fn get_default_theme(&self) -> &str {
        &self.default_theme
    }

    /// List available themes
    ///
    /// # Returns
    /// Vector of ThemeInfo for all available themes
    pub fn list_themes(&self) -> Vec<ThemeInfo> {
        let mut themes: Vec<ThemeInfo> = self.theme_cache.values().cloned().collect();
        themes.sort_by(|a, b| a.name.cmp(&b.name));
        themes
    }

    /// Refresh themes (rescan themes directory)
    ///
    /// This method rescans the themes directory and updates the theme cache.
    /// Useful when new themes are installed without restarting the server.
    pub fn refresh_themes(&mut self) -> Result<()> {
        self.refresh_theme_cache()
    }

    /// Reload templates (for hot-reload)
    ///
    /// This method reloads all templates from disk for the current theme.
    /// Useful during development or when theme files are modified.
    pub fn reload_templates(&mut self) -> Result<()> {
        self.load_theme_templates(&self.current_theme.clone())?;
        self.refresh_theme_cache()?;
        Ok(())
    }

    /// Check if a theme exists
    pub fn theme_exists(&self, theme_name: &str) -> bool {
        self.themes_path.join(theme_name).exists()
    }

    /// Get the path to a theme directory
    pub fn get_theme_path(&self, theme_name: &str) -> PathBuf {
        self.themes_path.join(theme_name)
    }

    /// Get theme info for a specific theme
    pub fn get_theme_info(&self, theme_name: &str) -> Option<&ThemeInfo> {
        self.theme_cache.get(theme_name)
    }

    /// Get the Tera instance (for advanced usage)
    pub fn tera(&self) -> &Tera {
        &self.tera
    }

    /// Get a mutable reference to the Tera instance
    pub fn tera_mut(&mut self) -> &mut Tera {
        &mut self.tera
    }
}

/// Theme metadata from theme.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeMetadata {
    /// Theme section
    pub theme: ThemeMetadataInner,
}

/// Inner theme metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeMetadataInner {
    /// Theme name (identifier)
    pub name: String,
    /// Display name for UI
    pub display_name: String,
    /// Theme description
    pub description: Option<String>,
    /// Theme version
    pub version: String,
    /// Theme author
    pub author: Option<String>,
}

/// Theme metadata from theme.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeJsonMetadata {
    /// Theme display name
    pub name: String,
    /// Short identifier
    pub short: Option<String>,
    /// Theme description
    pub description: Option<String>,
    /// Theme version
    pub version: String,
    /// Theme author
    pub author: Option<String>,
    /// Theme homepage URL
    pub url: Option<String>,
    /// Preview image filename
    pub preview: Option<String>,
    /// Required Noteva version
    pub requires: Option<ThemeRequirements>,
    /// Theme configuration
    pub configuration: Option<serde_json::Value>,
}

/// Theme requirements
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ThemeRequirements {
    /// Minimum Noteva version
    #[serde(default)]
    pub noteva: String,
}

/// Information about a theme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeInfo {
    /// Theme name (identifier)
    pub name: String,
    /// Theme display name
    pub display_name: String,
    /// Theme description
    pub description: Option<String>,
    /// Theme version
    pub version: String,
    /// Theme author
    pub author: Option<String>,
    /// Theme homepage URL
    pub url: Option<String>,
    /// Preview image filename
    pub preview: Option<String>,
    /// Required Noteva version
    pub requires_noteva: String,
    /// Whether compatible with current version
    pub compatible: bool,
    /// Compatibility message if not compatible
    pub compatibility_message: Option<String>,
    /// Theme configuration from theme.json
    pub config: Option<serde_json::Value>,
}

/// Standard template variables (Requirement 6.5)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardTemplateVars {
    /// Blog name
    pub site_name: String,
    /// Blog description
    pub site_description: String,
    /// Current logged-in user (optional)
    pub current_user: Option<CurrentUser>,
    /// Current request path
    pub request_path: String,
    /// Current year (for copyright)
    pub year: i32,
}

/// Current user information for templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    /// User ID
    pub id: i64,
    /// Username
    pub username: String,
    /// User role
    pub role: String,
}

impl StandardTemplateVars {
    /// Create new standard template variables
    pub fn new(
        site_name: impl Into<String>,
        site_description: impl Into<String>,
        request_path: impl Into<String>,
    ) -> Self {
        Self {
            site_name: site_name.into(),
            site_description: site_description.into(),
            current_user: None,
            request_path: request_path.into(),
            year: chrono::Utc::now().year(),
        }
    }

    /// Set the current user
    pub fn with_user(mut self, user: CurrentUser) -> Self {
        self.current_user = Some(user);
        self
    }
}

// Import chrono for year calculation
use chrono::Datelike;

#[cfg(test)]
mod tests;
