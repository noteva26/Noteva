//! Markdown rendering service
//!
//! This module provides Markdown to HTML conversion with syntax highlighting
//! for code blocks. It uses pulldown-cmark for Markdown parsing and syntect
//! for syntax highlighting.
//!
//! # Example
//!
//! ```
//! use noteva::services::markdown::MarkdownRenderer;
//!
//! let renderer = MarkdownRenderer::new();
//! let html = renderer.render("# Hello World\n\nThis is **bold** text.");
//! assert!(html.contains("<h1>"));
//! assert!(html.contains("<strong>"));
//! ```

use pulldown_cmark::{html, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::sync::Arc;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use regex::Regex;

use crate::plugin::{ShortcodeManager, ShortcodeContext, HookManager, hook_names};
use crate::services::emoji;

/// Options for rendering markdown with shortcodes
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// Context for shortcode rendering
    pub shortcode_context: ShortcodeContext,
    /// Whether to process shortcodes (default: true)
    pub process_shortcodes: bool,
}

/// A single entry in the table of contents.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TocEntry {
    /// Heading level (1-6)
    pub level: u32,
    /// Plain text content of the heading
    pub text: String,
    /// URL-friendly anchor id (e.g. "hello-world")
    pub id: String,
}

/// A thread-safe Markdown renderer with syntax highlighting support.
///
/// The renderer supports common Markdown features including:
/// - Headings (h1-h6)
/// - Lists (ordered and unordered)
/// - Links and images
/// - Blockquotes
/// - Code blocks with syntax highlighting
/// - Inline code
/// - Bold, italic, and strikethrough text
/// - Tables
/// - Task lists
/// - Smart punctuation
/// - Shortcodes (via ShortcodeManager integration)
/// - Hook integration for plugins
#[derive(Clone)]
pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme_set: Arc<ThemeSet>,
    theme_name: String,
    shortcode_manager: Option<Arc<ShortcodeManager>>,
    hook_manager: Option<Arc<HookManager>>,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    /// Creates a new MarkdownRenderer with default syntax definitions and themes.
    ///
    /// Uses the "base16-ocean.dark" theme by default for syntax highlighting.
    pub fn new() -> Self {
        Self::with_theme("base16-ocean.dark")
    }

    /// Creates a new MarkdownRenderer with a specific theme.
    ///
    /// # Arguments
    ///
    /// * `theme_name` - The name of the syntect theme to use for highlighting.
    ///                  Falls back to "base16-ocean.dark" if the theme is not found.
    pub fn with_theme(theme_name: &str) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        // Validate theme exists, fall back to default if not
        let validated_theme = if theme_set.themes.contains_key(theme_name) {
            theme_name.to_string()
        } else {
            "base16-ocean.dark".to_string()
        };

        Self {
            syntax_set,
            theme_set: Arc::new(theme_set),
            theme_name: validated_theme,
            shortcode_manager: None,
            hook_manager: None,
        }
    }

    /// Creates a new MarkdownRenderer with a shortcode manager.
    ///
    /// # Arguments
    ///
    /// * `shortcode_manager` - The shortcode manager to use for processing shortcodes.
    pub fn with_shortcode_manager(shortcode_manager: Arc<ShortcodeManager>) -> Self {
        let mut renderer = Self::new();
        renderer.shortcode_manager = Some(shortcode_manager);
        renderer
    }

    /// Creates a new MarkdownRenderer with shortcode manager and hook manager.
    pub fn with_managers(
        shortcode_manager: Arc<ShortcodeManager>,
        hook_manager: Arc<HookManager>,
    ) -> Self {
        let mut renderer = Self::new();
        renderer.shortcode_manager = Some(shortcode_manager);
        renderer.hook_manager = Some(hook_manager);
        renderer
    }

    /// Set the shortcode manager for this renderer.
    pub fn set_shortcode_manager(&mut self, shortcode_manager: Arc<ShortcodeManager>) {
        self.shortcode_manager = Some(shortcode_manager);
    }

    /// Set the hook manager for this renderer.
    pub fn set_hook_manager(&mut self, hook_manager: Arc<HookManager>) {
        self.hook_manager = Some(hook_manager);
    }

    /// Trigger a hook if hook manager is available
    fn trigger_hook(&self, name: &str, data: serde_json::Value) -> serde_json::Value {
        if let Some(ref manager) = self.hook_manager {
            manager.trigger(name, data)
        } else {
            data
        }
    }

    /// Renders Markdown text to HTML.
    ///
    /// # Arguments
    ///
    /// * `markdown` - The Markdown text to render.
    ///
    /// # Returns
    ///
    /// The rendered HTML string.
    ///
    /// # Features
    ///
    /// - Code blocks with language hints are syntax highlighted
    /// - Code blocks without language hints are rendered as plain code
    /// - All standard Markdown features are supported
    ///
    /// # Hooks
    /// - `markdown_before_parse` - Triggered before parsing, can modify content
    /// - `markdown_after_parse` - Triggered after parsing, can modify HTML output
    pub fn render(&self, markdown: &str) -> String {
        // Trigger markdown_before_parse hook
        let hook_data = self.trigger_hook(
            hook_names::MARKDOWN_BEFORE_PARSE,
            serde_json::json!({ "content": markdown })
        );
        
        // Get potentially modified content from hook
        let content = hook_data
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or(markdown);

        // Configure parser options
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
        options.insert(Options::ENABLE_FOOTNOTES);

        // Pre-process: protect math expressions from markdown parsing
        let (content, math_placeholders) = Self::extract_math_expressions(content);

        let parser = Parser::new_ext(&content, options);

        // Process events, handling code blocks specially for syntax highlighting
        let events = self.process_events(parser);

        // Render to HTML
        let mut html_output = String::new();
        html::push_html(&mut html_output, events.into_iter());

        // Post-process: restore math expressions as KaTeX-ready elements
        let html_output = Self::restore_math_expressions(&html_output, &math_placeholders);

        // Trigger markdown_after_parse hook
        let hook_data = self.trigger_hook(
            hook_names::MARKDOWN_AFTER_PARSE,
            serde_json::json!({ "html": html_output })
        );
        
        // Get potentially modified HTML from hook
        let html_after_hook = hook_data
            .get("html")
            .and_then(|v| v.as_str())
            .unwrap_or(&html_output)
            .to_string();
        
        // Process emoji (shortcodes and Unicode)
        emoji::process_all_emoji(&html_after_hook)
    }

    /// Extract a table of contents from markdown content.
    ///
    /// Parses the markdown and collects all headings with their level, text,
    /// and a generated anchor id. This can be used to build a TOC sidebar.
    pub fn extract_toc(&self, markdown: &str) -> Vec<TocEntry> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_FOOTNOTES);

        let parser = Parser::new_ext(markdown, options);

        let mut toc = Vec::new();
        let mut in_heading = false;
        let mut heading_level: u32 = 0;
        let mut heading_text = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    in_heading = true;
                    heading_level = level as u32;
                    heading_text.clear();
                }
                Event::Text(text) if in_heading => {
                    heading_text.push_str(&text);
                }
                Event::Code(code) if in_heading => {
                    heading_text.push_str(&code);
                }
                Event::End(TagEnd::Heading(_)) => {
                    in_heading = false;
                    let text = heading_text.trim().to_string();
                    if !text.is_empty() {
                        let id = Self::heading_to_id(&text);
                        toc.push(TocEntry { level: heading_level, text, id });
                    }
                }
                _ => {}
            }
        }

        toc
    }

    /// Convert heading text to a URL-friendly anchor id.
    fn heading_to_id(text: &str) -> String {
        text.chars()
            .map(|c| {
                if c.is_alphanumeric() { c.to_ascii_lowercase() }
                else if c == ' ' || c == '-' || c == '_' { '-' }
                else if c > '\x7f' { c } // keep CJK characters
                else { '-' }
            })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Renders Markdown text to HTML with shortcode processing.
    ///
    /// # Arguments
    ///
    /// * `markdown` - The Markdown text to render.
    /// * `options` - Render options including shortcode context.
    ///
    /// # Returns
    ///
    /// The rendered HTML string with shortcodes processed.
    ///
    /// # Processing Order
    ///
    /// 1. Shortcodes are processed first (before markdown parsing)
    /// 2. Markdown is then rendered to HTML
    ///
    /// This order allows shortcodes to output markdown that will be rendered.
    pub fn render_with_options(&self, markdown: &str, options: &RenderOptions) -> String {
        let content = if options.process_shortcodes {
            // Process shortcodes first
            if let Some(ref manager) = self.shortcode_manager {
                manager.render(markdown, &options.shortcode_context)
            } else {
                markdown.to_string()
            }
        } else {
            markdown.to_string()
        };

        // Then render markdown
        self.render(&content)
    }

    /// Renders Markdown text to HTML with shortcode processing using default context.
    ///
    /// Convenience method that uses default shortcode context.
    pub fn render_with_shortcodes(&self, markdown: &str) -> String {
        self.render_with_options(markdown, &RenderOptions {
            shortcode_context: ShortcodeContext::default(),
            process_shortcodes: true,
        })
    }

    /// Renders Markdown text to HTML with shortcode processing for a specific article.
    ///
    /// # Arguments
    ///
    /// * `markdown` - The Markdown text to render.
    /// * `article_id` - The ID of the article being rendered.
    /// * `user_id` - Optional user ID if the viewer is logged in.
    pub fn render_article(&self, markdown: &str, article_id: i64, user_id: Option<i64>) -> String {
        self.render_with_options(markdown, &RenderOptions {
            shortcode_context: ShortcodeContext {
                article_id: Some(article_id),
                user_id,
                is_preview: false,
                ..Default::default()
            },
            process_shortcodes: true,
        })
    }

    /// Renders Markdown text to HTML in preview mode.
    ///
    /// Preview mode may affect how certain shortcodes render (e.g., hiding
    /// premium content placeholders).
    pub fn render_preview(&self, markdown: &str) -> String {
        self.render_with_options(markdown, &RenderOptions {
            shortcode_context: ShortcodeContext {
                is_preview: true,
                ..Default::default()
            },
            process_shortcodes: true,
        })
    }

    /// Processes parser events, applying syntax highlighting to code blocks.
    fn process_events<'a>(&self, parser: Parser<'a>) -> Vec<Event<'a>> {
        let mut events = Vec::new();
        let mut in_code_block = false;
        let mut code_lang: Option<String> = None;
        let mut code_content = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    code_content.clear();
                    code_lang = match kind {
                        CodeBlockKind::Fenced(lang) => {
                            let lang_str = lang.to_string();
                            if lang_str.is_empty() {
                                None
                            } else {
                                Some(lang_str)
                            }
                        }
                        CodeBlockKind::Indented => None,
                    };
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code_block = false;

                    // Mermaid: output as <div class="mermaid"> instead of code block
                    let highlighted = if code_lang.as_deref() == Some("mermaid") {
                        format!(
                            "<div class=\"mermaid\">{}</div>",
                            html_escape(&code_content)
                        )
                    } else if let Some(ref lang) = code_lang {
                        self.highlight_code(&code_content, lang)
                    } else {
                        self.plain_code_block(&code_content)
                    };

                    events.push(Event::Html(highlighted.into()));
                    code_lang = None;
                }
                Event::Text(text) if in_code_block => {
                    code_content.push_str(&text);
                }
                _ => {
                    events.push(event);
                }
            }
        }

        events
    }

    /// Applies syntax highlighting to a code block.
    ///
    /// # Arguments
    ///
    /// * `code` - The code content to highlight.
    /// * `lang` - The language hint for syntax highlighting.
    ///
    /// # Returns
    ///
    /// HTML string with syntax highlighting applied, or plain code block
    /// if the language is not recognized.
    fn highlight_code(&self, code: &str, lang: &str) -> String {
        // Try to find syntax definition for the language
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .or_else(|| self.syntax_set.find_syntax_by_extension(lang));

        match syntax {
            Some(syntax) => {
                let theme = &self.theme_set.themes[&self.theme_name];
                match highlighted_html_for_string(code, &self.syntax_set, syntax, theme) {
                    Ok(html) => html,
                    Err(_) => self.plain_code_block(code),
                }
            }
            None => {
                // Language not recognized, render as plain code with language class
                self.plain_code_block_with_lang(code, lang)
            }
        }
    }

    /// Renders a plain code block without syntax highlighting.
    fn plain_code_block(&self, code: &str) -> String {
        format!(
            "<pre><code>{}</code></pre>",
            html_escape(code)
        )
    }

    /// Renders a plain code block with a language class.
    fn plain_code_block_with_lang(&self, code: &str, lang: &str) -> String {
        format!(
            "<pre><code class=\"language-{}\">{}</code></pre>",
            html_escape(lang),
            html_escape(code)
        )
    }

    /// Extract math expressions ($..$ and $$..$$) and replace with placeholders
    /// to prevent pulldown-cmark from mangling them.
    fn extract_math_expressions(content: &str) -> (String, Vec<(String, bool)>) {
        let mut placeholders: Vec<(String, bool)> = Vec::new();

        // First, protect code blocks and inline code from math extraction
        // Replace them with temporary markers, extract math, then restore
        let code_block_re = Regex::new(r"(?s)```.*?```").unwrap();
        let inline_code_re = Regex::new(r"`[^`]+`").unwrap();

        let mut code_markers: Vec<(String, String)> = Vec::new();
        let mut protected = content.to_string();

        // Protect fenced code blocks
        for m in code_block_re.find_iter(content) {
            let marker = format!("\x01CODEBLOCK_{}\x01", code_markers.len());
            code_markers.push((marker.clone(), m.as_str().to_string()));
            protected = protected.replacen(m.as_str(), &marker, 1);
        }
        // Protect inline code
        let snapshot = protected.clone();
        for m in inline_code_re.find_iter(&snapshot) {
            let marker = format!("\x01INLINECODE_{}\x01", code_markers.len());
            code_markers.push((marker.clone(), m.as_str().to_string()));
            protected = protected.replacen(m.as_str(), &marker, 1);
        }

        let mut result = protected;

        // Block math: $$...$$
        let block_re = Regex::new(r"\$\$([\s\S]+?)\$\$").unwrap();
        let mut offset: i64 = 0;
        let snapshot = result.clone();
        let captures: Vec<_> = block_re.captures_iter(&snapshot).collect();
        for cap in &captures {
            let m = cap.get(0).unwrap();
            let expr = cap.get(1).unwrap().as_str().to_string();
            let idx = placeholders.len();
            let placeholder = format!("\x00MATH_BLOCK_{}\x00", idx);
            placeholders.push((expr, true));
            let start = (m.start() as i64 + offset) as usize;
            let end = (m.end() as i64 + offset) as usize;
            let old_len = end - start;
            result.replace_range(start..end, &placeholder);
            offset += placeholder.len() as i64 - old_len as i64;
        }

        // Inline math: $..$ (single line, not inside code)
        let inline_re = Regex::new(r"\$([^\$\n]+?)\$").unwrap();
        let snapshot = result.clone();
        offset = 0;
        let captures: Vec<_> = inline_re.captures_iter(&snapshot).collect();
        for cap in &captures {
            let m = cap.get(0).unwrap();
            let expr = cap.get(1).unwrap().as_str().to_string();
            let idx = placeholders.len();
            let placeholder = format!("\x00MATH_INLINE_{}\x00", idx);
            placeholders.push((expr, false));
            let start = (m.start() as i64 + offset) as usize;
            let end = (m.end() as i64 + offset) as usize;
            let old_len = end - start;
            result.replace_range(start..end, &placeholder);
            offset += placeholder.len() as i64 - old_len as i64;
        }

        // Restore code blocks and inline code
        for (marker, original) in code_markers.iter().rev() {
            result = result.replace(marker, original);
        }

        (result, placeholders)
    }

    /// Restore math placeholders with KaTeX-ready HTML elements.
    fn restore_math_expressions(html: &str, placeholders: &[(String, bool)]) -> String {
        let mut result = html.to_string();
        for (idx, (expr, is_block)) in placeholders.iter().enumerate() {
            let tag = if *is_block { "MATH_BLOCK" } else { "MATH_INLINE" };
            let placeholder = format!("\x00{}_{}\x00", tag, idx);
            // The placeholder may have been HTML-escaped by pulldown-cmark
            let escaped_placeholder = placeholder
                .replace('\x00', "&#0;");
            let replacement = if *is_block {
                format!("<div class=\"math-block\">{}</div>", html_escape(expr))
            } else {
                format!("<span class=\"math-inline\">{}</span>", html_escape(expr))
            };
            result = result.replace(&placeholder, &replacement);
            result = result.replace(&escaped_placeholder, &replacement);
        }
        result
    }
}

/// Escapes HTML special characters in a string.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_renderer() {
        let renderer = MarkdownRenderer::new();
        assert_eq!(renderer.theme_name, "base16-ocean.dark");
    }

    #[test]
    fn test_with_valid_theme() {
        let renderer = MarkdownRenderer::with_theme("InspiredGitHub");
        assert_eq!(renderer.theme_name, "InspiredGitHub");
    }

    #[test]
    fn test_with_invalid_theme_falls_back() {
        let renderer = MarkdownRenderer::with_theme("nonexistent-theme");
        assert_eq!(renderer.theme_name, "base16-ocean.dark");
    }

    #[test]
    fn test_render_heading() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("# Heading 1");
        assert!(html.contains("<h1>"));
        assert!(html.contains("Heading 1"));
        assert!(html.contains("</h1>"));
    }

    #[test]
    fn test_render_multiple_headings() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6");
        assert!(html.contains("<h1>"));
        assert!(html.contains("<h2>"));
        assert!(html.contains("<h3>"));
        assert!(html.contains("<h4>"));
        assert!(html.contains("<h5>"));
        assert!(html.contains("<h6>"));
    }

    #[test]
    fn test_render_bold() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("This is **bold** text.");
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_render_italic() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("This is *italic* text.");
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_render_strikethrough() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("This is ~~strikethrough~~ text.");
        assert!(html.contains("<del>strikethrough</del>"));
    }

    #[test]
    fn test_render_unordered_list() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("- Item 1\n- Item 2\n- Item 3");
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>"));
        assert!(html.contains("Item 1"));
        assert!(html.contains("Item 2"));
        assert!(html.contains("Item 3"));
        assert!(html.contains("</ul>"));
    }

    #[test]
    fn test_render_ordered_list() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("1. First\n2. Second\n3. Third");
        assert!(html.contains("<ol>"));
        assert!(html.contains("<li>"));
        assert!(html.contains("First"));
        assert!(html.contains("Second"));
        assert!(html.contains("Third"));
        assert!(html.contains("</ol>"));
    }

    #[test]
    fn test_render_link() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("[Example](https://example.com)");
        assert!(html.contains("<a href=\"https://example.com\">Example</a>"));
    }

    #[test]
    fn test_render_image() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("![Alt text](https://example.com/image.png)");
        assert!(html.contains("<img"));
        assert!(html.contains("src=\"https://example.com/image.png\""));
        assert!(html.contains("alt=\"Alt text\""));
    }

    #[test]
    fn test_render_blockquote() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("> This is a quote");
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("This is a quote"));
        assert!(html.contains("</blockquote>"));
    }

    #[test]
    fn test_render_inline_code() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("Use `code` here");
        assert!(html.contains("<code>code</code>"));
    }

    #[test]
    fn test_render_code_block_without_language() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("```\nlet x = 1;\n```");
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code>"));
        assert!(html.contains("let x = 1;"));
    }

    #[test]
    fn test_render_code_block_with_rust() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("```rust\nfn main() {\n    println!(\"Hello\");\n}\n```");
        // Should contain syntax highlighting (pre tag with style)
        assert!(html.contains("<pre"));
        // Syntect generates styled spans
        assert!(html.contains("style="));
    }

    #[test]
    fn test_render_code_block_with_unknown_language() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("```unknownlang\nsome code\n```");
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code"));
        assert!(html.contains("language-unknownlang"));
        assert!(html.contains("some code"));
    }

    #[test]
    fn test_render_table() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("| A | B |\n|---|---|\n| 1 | 2 |");
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>"));
        assert!(html.contains("<td>"));
        assert!(html.contains("</table>"));
    }

    #[test]
    fn test_render_task_list() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("- [x] Done\n- [ ] Todo");
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("checked"));
        assert!(html.contains("Done"));
        assert!(html.contains("Todo"));
    }

    #[test]
    fn test_render_smart_punctuation() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("\"Hello\" -- world...");
        // Smart punctuation converts -- to en-dash and ... to ellipsis
        assert!(html.contains("–") || html.contains("&ndash;") || html.contains("--"));
    }

    #[test]
    fn test_html_escape_in_code() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("```\n<script>alert('xss')</script>\n```");
        // Should escape HTML in code blocks
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;") || html.contains("&lt;"));
    }

    #[test]
    fn test_render_empty_input() {
        let renderer = MarkdownRenderer::new();
        let html = renderer.render("");
        assert!(html.is_empty());
    }

    #[test]
    fn test_render_complex_document() {
        let renderer = MarkdownRenderer::new();
        let markdown = r#"
# Title

This is a **bold** and *italic* paragraph.

## Code Example

```rust
fn hello() {
    println!("Hello, world!");
}
```

### List

- Item 1
- Item 2

> A quote

[Link](https://example.com)
"#;
        let html = renderer.render(markdown);
        assert!(html.contains("<h1>"));
        assert!(html.contains("<h2>"));
        assert!(html.contains("<h3>"));
        assert!(html.contains("<strong>"));
        assert!(html.contains("<em>"));
        assert!(html.contains("<pre"));
        assert!(html.contains("<ul>"));
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("<a href="));
    }

    #[test]
    fn test_html_escape_function() {
        assert_eq!(html_escape("<>&\"'"), "&lt;&gt;&amp;&quot;&#x27;");
    }

    #[test]
    fn test_renderer_is_clone() {
        let renderer = MarkdownRenderer::new();
        let cloned = renderer.clone();
        assert_eq!(renderer.theme_name, cloned.theme_name);
    }

    #[test]
    fn test_renderer_default() {
        let renderer = MarkdownRenderer::default();
        assert_eq!(renderer.theme_name, "base16-ocean.dark");
    }

    #[test]
    fn test_render_with_shortcode_manager() {
        use crate::plugin::shortcode::builtins;
        
        let mut shortcode_manager = ShortcodeManager::new();
        builtins::register_builtins(&mut shortcode_manager);
        
        let renderer = MarkdownRenderer::with_shortcode_manager(Arc::new(shortcode_manager));
        
        let markdown = r#"# Title

[note type="info"]This is an info note[/note]

Some **bold** text."#;
        
        let html = renderer.render_with_shortcodes(markdown);
        
        assert!(html.contains("<h1>"));
        assert!(html.contains("shortcode-note"));
        assert!(html.contains("shortcode-note-info"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_render_article_context() {
        let mut shortcode_manager = ShortcodeManager::new();
        shortcode_manager.register("article-id", |_sc, ctx| {
            format!("Article ID: {:?}", ctx.article_id)
        });
        
        let renderer = MarkdownRenderer::with_shortcode_manager(Arc::new(shortcode_manager));
        
        let markdown = "The article is [article-id][/article-id]";
        let html = renderer.render_article(markdown, 42, Some(1));
        
        assert!(html.contains("Article ID: Some(42)"));
    }

    #[test]
    fn test_render_preview_mode() {
        let mut shortcode_manager = ShortcodeManager::new();
        shortcode_manager.register("preview-check", |_sc, ctx| {
            if ctx.is_preview {
                "PREVIEW MODE".to_string()
            } else {
                "NORMAL MODE".to_string()
            }
        });
        
        let renderer = MarkdownRenderer::with_shortcode_manager(Arc::new(shortcode_manager));
        
        let markdown = "[preview-check][/preview-check]";
        let html = renderer.render_preview(markdown);
        
        assert!(html.contains("PREVIEW MODE"));
    }

    #[test]
    fn test_render_without_shortcode_processing() {
        let mut shortcode_manager = ShortcodeManager::new();
        shortcode_manager.register("test", |_sc, _ctx| "PROCESSED".to_string());
        
        let renderer = MarkdownRenderer::with_shortcode_manager(Arc::new(shortcode_manager));
        
        let markdown = "[test][/test]";
        let html = renderer.render_with_options(markdown, &RenderOptions {
            process_shortcodes: false,
            ..Default::default()
        });
        
        // Shortcode should not be processed
        assert!(html.contains("[test]"));
        assert!(!html.contains("PROCESSED"));
    }

    #[test]
    fn test_render_footnotes() {
        let renderer = MarkdownRenderer::new();
        let markdown = "This has a footnote[^1].\n\n[^1]: Footnote content here.";
        let html = renderer.render(markdown);
        // pulldown-cmark generates footnote references and definitions
        assert!(html.contains("footnote"));
    }

    #[test]
    fn test_render_mermaid_block() {
        let renderer = MarkdownRenderer::new();
        let markdown = "```mermaid\ngraph TD;\n  A-->B;\n```";
        let html = renderer.render(markdown);
        assert!(html.contains("<div class=\"mermaid\">"));
        assert!(html.contains("A--&gt;B;"));
        // Should NOT be a <pre><code> block
        assert!(!html.contains("<pre><code"));
    }

    #[test]
    fn test_render_inline_math() {
        let renderer = MarkdownRenderer::new();
        let markdown = "The formula $E=mc^2$ is famous.";
        let html = renderer.render(markdown);
        assert!(html.contains("<span class=\"math-inline\">"));
        assert!(html.contains("E=mc^2"));
    }

    #[test]
    fn test_render_block_math() {
        let renderer = MarkdownRenderer::new();
        let markdown = "Here is a formula:\n\n$$\n\\sum_{i=1}^n i = \\frac{n(n+1)}{2}\n$$\n\nDone.";
        let html = renderer.render(markdown);
        assert!(html.contains("<div class=\"math-block\">"));
    }

    #[test]
    fn test_math_does_not_affect_code_blocks() {
        let renderer = MarkdownRenderer::new();
        let markdown = "```\n$not math$\n```";
        let html = renderer.render(markdown);
        // Inside code block, $ should not be treated as math
        assert!(!html.contains("math-inline"));
    }
}
