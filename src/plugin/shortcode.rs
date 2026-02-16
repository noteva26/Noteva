//! Shortcode parser and renderer
//!
//! Parses and renders shortcodes like [name attr="value"]content[/name]

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Parsed shortcode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcode {
    /// Shortcode name
    pub name: String,
    /// Attributes
    pub attrs: HashMap<String, String>,
    /// Inner content (between opening and closing tags)
    pub content: String,
    /// Original matched string
    pub original: String,
}

/// Shortcode handler function type
pub type ShortcodeHandler = Box<dyn Fn(&Shortcode, &ShortcodeContext) -> String + Send + Sync>;

/// Context passed to shortcode handlers
#[derive(Debug, Clone, Default)]
pub struct ShortcodeContext {
    /// Current article ID (if rendering article content)
    pub article_id: Option<i64>,
    /// Current user ID (if logged in)
    pub user_id: Option<i64>,
    /// Whether this is a preview render
    pub is_preview: bool,
    /// Additional context data
    pub data: HashMap<String, String>,
}

/// Shortcode manager
pub struct ShortcodeManager {
    /// Registered handlers (name -> handler)
    handlers: HashMap<String, ShortcodeHandler>,
}

impl Default for ShortcodeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShortcodeManager {
    /// Create a new shortcode manager
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }
    
    /// Register a shortcode handler
    pub fn register<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&Shortcode, &ShortcodeContext) -> String + Send + Sync + 'static,
    {
        debug!("Registered shortcode: [{}]", name);
        self.handlers.insert(name.to_string(), Box::new(handler));
    }
    
    /// Unregister a shortcode handler
    pub fn unregister(&mut self, name: &str) {
        self.handlers.remove(name);
    }
    
    /// Check if a shortcode is registered
    pub fn has_handler(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }

    /// Parse shortcodes from content
    pub fn parse(&self, content: &str) -> Vec<Shortcode> {
        let mut shortcodes = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let len = chars.len();
        let mut i = 0;
        
        while i < len {
            // Look for opening bracket
            if chars[i] == '[' {
                if let Some((shortcode, end_pos)) = self.parse_shortcode_at(&chars, i) {
                    shortcodes.push(shortcode);
                    i = end_pos;
                    continue;
                }
            }
            i += 1;
        }
        
        shortcodes
    }
    
    /// Try to parse a shortcode starting at position
    fn parse_shortcode_at(&self, chars: &[char], start: usize) -> Option<(Shortcode, usize)> {
        let len = chars.len();
        if start >= len || chars[start] != '[' {
            return None;
        }
        
        // Find the end of opening tag
        let mut i = start + 1;
        
        // Skip whitespace
        while i < len && chars[i].is_whitespace() {
            i += 1;
        }
        
        // Parse name
        let name_start = i;
        while i < len && (chars[i].is_alphanumeric() || chars[i] == '-' || chars[i] == '_') {
            i += 1;
        }
        
        if i == name_start {
            return None; // No name found
        }
        
        let name: String = chars[name_start..i].iter().collect();
        
        // Parse attributes until ] or /]
        let mut attrs = HashMap::new();
        let mut is_self_closing = false;
        
        while i < len && chars[i] != ']' {
            // Skip whitespace
            while i < len && chars[i].is_whitespace() {
                i += 1;
            }
            
            if i >= len {
                return None;
            }
            
            // Check for self-closing
            if chars[i] == '/' {
                is_self_closing = true;
                i += 1;
                continue;
            }
            
            if chars[i] == ']' {
                break;
            }
            
            // Parse attribute name
            let attr_name_start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '-' || chars[i] == '_') {
                i += 1;
            }
            
            if i == attr_name_start {
                i += 1; // Skip unknown char
                continue;
            }
            
            let attr_name: String = chars[attr_name_start..i].iter().collect();
            
            // Skip whitespace and =
            while i < len && (chars[i].is_whitespace() || chars[i] == '=') {
                i += 1;
            }
            
            // Parse attribute value (quoted)
            if i < len && (chars[i] == '"' || chars[i] == '\'') {
                let quote = chars[i];
                i += 1;
                let value_start = i;
                while i < len && chars[i] != quote {
                    i += 1;
                }
                let attr_value: String = chars[value_start..i].iter().collect();
                attrs.insert(attr_name, attr_value);
                if i < len {
                    i += 1; // Skip closing quote
                }
            }
        }
        
        if i >= len {
            return None;
        }
        
        i += 1; // Skip ]
        
        let opening_tag_end = i;
        
        if is_self_closing {
            let original: String = chars[start..opening_tag_end].iter().collect();
            return Some((Shortcode {
                name,
                attrs,
                content: String::new(),
                original,
            }, opening_tag_end));
        }
        
        // Find closing tag [/name]
        let closing_tag = format!("[/{}]", name);
        let content_start = i;
        
        // Search for closing tag
        while i < len {
            if chars[i] == '[' && chars.get(i + 1) == Some(&'/') {
                let remaining: String = chars[i..].iter().collect();
                if remaining.starts_with(&closing_tag) {
                    let content: String = chars[content_start..i].iter().collect();
                    let end_pos = i + closing_tag.len();
                    let original: String = chars[start..end_pos].iter().collect();
                    
                    return Some((Shortcode {
                        name,
                        attrs,
                        content,
                        original,
                    }, end_pos));
                }
            }
            i += 1;
        }
        
        None // No closing tag found
    }
    
    /// Render all shortcodes in content
    pub fn render(&self, content: &str, context: &ShortcodeContext) -> String {
        let mut result = content.to_string();
        let shortcodes = self.parse(content);
        
        for shortcode in shortcodes {
            if let Some(handler) = self.handlers.get(&shortcode.name) {
                let rendered = handler(&shortcode, context);
                result = result.replace(&shortcode.original, &rendered);
            }
            // If no handler, leave shortcode as-is
        }
        
        result
    }
    
    /// Render shortcodes with a custom fallback for unknown shortcodes
    pub fn render_with_fallback<F>(&self, content: &str, context: &ShortcodeContext, fallback: F) -> String
    where
        F: Fn(&Shortcode) -> String,
    {
        let mut result = content.to_string();
        let shortcodes = self.parse(content);
        
        for shortcode in shortcodes {
            let rendered = if let Some(handler) = self.handlers.get(&shortcode.name) {
                handler(&shortcode, context)
            } else {
                fallback(&shortcode)
            };
            result = result.replace(&shortcode.original, &rendered);
        }
        
        result
    }
}

/// Built-in shortcodes
pub mod builtins {
    use super::*;
    
    /// Register built-in shortcodes
    pub fn register_builtins(manager: &mut ShortcodeManager) {
        // [note type="info"]content[/note] - Note/callout box
        manager.register("note", |shortcode, _ctx| {
            let note_type = shortcode.attrs.get("type").map_or("info", |s| s.as_str());
            let class = match note_type {
                "warning" => "shortcode-note shortcode-note-warning",
                "error" | "danger" => "shortcode-note shortcode-note-error",
                "success" => "shortcode-note shortcode-note-success",
                _ => "shortcode-note shortcode-note-info",
            };
            format!(
                r#"<div class="{}">{}</div>"#,
                class,
                shortcode.content
            )
        });
        
        // [collapse title="Click to expand"]content[/collapse] - Collapsible section
        manager.register("collapse", |shortcode, _ctx| {
            let title = shortcode.attrs.get("title").map_or("Details", |s| s.as_str());
            format!(
                r#"<details class="shortcode-collapse">
<summary>{}</summary>
<div class="shortcode-collapse-content">{}</div>
</details>"#,
                html_escape(title),
                shortcode.content
            )
        });
        
        // [button url="..." target="_blank"]text[/button] - Button
        manager.register("button", |shortcode, _ctx| {
            let url = shortcode.attrs.get("url").map_or("#", |s| s.as_str());
            let target = shortcode.attrs.get("target").map_or("_self", |s| s.as_str());
            let style = shortcode.attrs.get("style").map_or("primary", |s| s.as_str());
            format!(
                r#"<a href="{}" target="{}" class="shortcode-button shortcode-button-{}">{}</a>"#,
                html_escape(url),
                html_escape(target),
                style,
                shortcode.content
            )
        });
        
        // [code lang="rust"]code[/code] - Code block with language
        manager.register("code", |shortcode, _ctx| {
            let lang = shortcode.attrs.get("lang").map_or("", |s| s.as_str());
            format!(
                r#"<pre><code class="language-{}">{}</code></pre>"#,
                html_escape(lang),
                html_escape(&shortcode.content)
            )
        });
        
        // [quote author="Name" source="Book"]quote[/quote] - Quote with attribution
        manager.register("quote", |shortcode, _ctx| {
            let author = shortcode.attrs.get("author");
            let source = shortcode.attrs.get("source");
            
            let mut footer = String::new();
            if let Some(a) = author {
                footer.push_str(&format!("— {}", html_escape(a)));
            }
            if let Some(s) = source {
                footer.push_str(&format!(", <cite>{}</cite>", html_escape(s)));
            }
            
            if footer.is_empty() {
                format!(r#"<blockquote class="shortcode-quote">{}</blockquote>"#, shortcode.content)
            } else {
                format!(
                    r#"<blockquote class="shortcode-quote">{}<footer>{}</footer></blockquote>"#,
                    shortcode.content, footer
                )
            }
        });
        
        // [video url="..." /] or [video src="..." poster="..." /] - Video embed
        manager.register("video", |shortcode, _ctx| {
            let url = shortcode.attrs.get("src")
                .or(shortcode.attrs.get("url"))
                .map_or("", |s| s.as_str());
            let width = shortcode.attrs.get("width").map_or("100%", |s| s.as_str());
            let poster = shortcode.attrs.get("poster");
            
            // Detect video type
            if url.contains("youtube.com") || url.contains("youtu.be") {
                let video_id = extract_youtube_id(url);
                format!(
                    r#"<div class="shortcode-video"><iframe width="{}" height="315" src="https://www.youtube.com/embed/{}" frameborder="0" allowfullscreen></iframe></div>"#,
                    width, video_id
                )
            } else if url.contains("bilibili.com") {
                let bvid = extract_bilibili_id(url);
                format!(
                    r#"<div class="shortcode-video"><iframe width="{}" height="315" src="//player.bilibili.com/player.html?bvid={}" frameborder="0" allowfullscreen></iframe></div>"#,
                    width, bvid
                )
            } else {
                let poster_attr = poster.map_or(String::new(), |p| format!(r#" poster="{}""#, html_escape(p)));
                let is_hls = url.ends_with(".m3u8");
                let data_hls = if is_hls { r#" data-hls="true""# } else { "" };
                format!(
                    r#"<div class="shortcode-video"><video src="{}" width="{}"{}{} controls playsinline></video></div>"#,
                    html_escape(url), width, poster_attr, data_hls
                )
            }
        });
        
        // [audio src="..." /] - Audio player
        manager.register("audio", |shortcode, _ctx| {
            let url = shortcode.attrs.get("src")
                .or(shortcode.attrs.get("url"))
                .map_or("", |s| s.as_str());
            let title = shortcode.attrs.get("title");
            let is_hls = url.ends_with(".m3u8");
            let data_hls = if is_hls { r#" data-hls="true""# } else { "" };
            
            if let Some(t) = title {
                format!(
                    r#"<div class="shortcode-audio"><p class="shortcode-audio-title">{}</p><audio src="{}" controls preload="metadata"{}></audio></div>"#,
                    html_escape(t), html_escape(url), data_hls
                )
            } else {
                format!(
                    r#"<div class="shortcode-audio"><audio src="{}" controls preload="metadata"{}></audio></div>"#,
                    html_escape(url), data_hls
                )
            }
        });
        
        // [hide-until-reply]content[/hide-until-reply] - Hidden content until user replies
        // Note: The actual unlock logic is handled by the frontend plugin
        manager.register("hide-until-reply", |shortcode, ctx| {
            let placeholder = shortcode.attrs.get("placeholder")
                .map_or("回复后可见", |s| s.as_str());
            let article_id = ctx.article_id.unwrap_or(0);
            
            format!(
                r#"<div class="noteva-hidden-content" data-article-id="{}">
  <div class="noteva-placeholder">
    <span class="noteva-placeholder-icon">🔒</span>
    <span class="noteva-placeholder-text">{}</span>
  </div>
  <template class="noteva-hidden-template">{}</template>
</div>"#,
                article_id,
                html_escape(placeholder),
                shortcode.content
            )
        });

        // [file name="doc.pdf" size="1.2 MB" url="/uploads/xxx.pdf" /] - File attachment card
        manager.register("file", |shortcode, _ctx| {
            let name = shortcode.attrs.get("name").map_or("file", |s| s.as_str());
            let url = shortcode.attrs.get("url").map_or("#", |s| s.as_str());
            let size = shortcode.attrs.get("size").map_or("", |s| s.as_str());
            let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
            let icon = match ext.as_str() {
                "pdf" => "pdf",
                "doc" | "docx" => "word",
                "xls" | "xlsx" | "csv" => "excel",
                "ppt" | "pptx" => "ppt",
                "zip" | "rar" | "7z" | "tar" | "gz" => "archive",
                "mp3" | "wav" | "flac" | "ogg" => "audio",
                "mp4" | "mkv" | "avi" | "mov" => "video",
                "exe" | "msi" => "exe",
                "md" | "txt" | "json" | "xml" | "yml" | "yaml" => "text",
                "html" | "css" | "js" | "ts" | "rs" | "py" | "php" => "code",
                _ => "file",
            };
            let size_html = if size.is_empty() {
                String::new()
            } else {
                format!(r#"<span class="shortcode-file-size">{}</span>"#, html_escape(size))
            };
            format!(
                r#"<a href="{}" download class="shortcode-file" data-file-type="{}"><span class="shortcode-file-icon" data-icon="{}"></span><span class="shortcode-file-info"><span class="shortcode-file-name">{}</span>{}</span><span class="shortcode-file-download">↓</span></a>"#,
                html_escape(url), html_escape(&ext), icon, html_escape(name), size_html
            )
        });
    }
    
    /// HTML escape helper
    fn html_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
    
    /// Extract YouTube video ID from URL
    fn extract_youtube_id(url: &str) -> &str {
        if let Some(pos) = url.find("v=") {
            let start = pos + 2;
            let end = url[start..].find('&').map_or(url.len(), |p| start + p);
            &url[start..end]
        } else if let Some(pos) = url.find("youtu.be/") {
            let start = pos + 9;
            let end = url[start..].find('?').map_or(url.len(), |p| start + p);
            &url[start..end]
        } else {
            ""
        }
    }
    
    /// Extract Bilibili video ID from URL
    fn extract_bilibili_id(url: &str) -> &str {
        if let Some(pos) = url.find("/BV") {
            let start = pos + 1;
            let end = url[start..].find(['/', '?']).map_or(url.len(), |p| start + p);
            &url[start..end]
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_shortcode() {
        let manager = ShortcodeManager::new();
        let content = r#"Hello [note type="warning"]This is a warning[/note] world"#;
        let shortcodes = manager.parse(content);
        
        assert_eq!(shortcodes.len(), 1);
        assert_eq!(shortcodes[0].name, "note");
        assert_eq!(shortcodes[0].attrs.get("type"), Some(&"warning".to_string()));
        assert_eq!(shortcodes[0].content, "This is a warning");
    }
    
    #[test]
    fn test_parse_self_closing() {
        let manager = ShortcodeManager::new();
        let content = r#"Check this [video url="https://youtube.com/watch?v=123" /] out"#;
        let shortcodes = manager.parse(content);
        
        assert_eq!(shortcodes.len(), 1);
        assert_eq!(shortcodes[0].name, "video");
        assert_eq!(shortcodes[0].attrs.get("url"), Some(&"https://youtube.com/watch?v=123".to_string()));
        assert!(shortcodes[0].content.is_empty());
    }
    
    #[test]
    fn test_render_shortcode() {
        let mut manager = ShortcodeManager::new();
        manager.register("upper", |sc, _| sc.content.to_uppercase());
        
        let content = "Hello [upper]world[/upper]!";
        let ctx = ShortcodeContext::default();
        let result = manager.render(content, &ctx);
        
        assert_eq!(result, "Hello WORLD!");
    }
}
