//! Hook contract documentation generator.
//!
//! Reads the [`HookRegistry`] and produces a Markdown document describing every
//! hook grouped by category.

use crate::plugin::hook_registry::{HookRegistry, HookType, HookScope};

/// Category display names (Chinese) keyed by the internal category id returned
/// from `HookRegistry::by_category()`.
fn category_display_name(cat: &str) -> &str {
    match cat {
        "article" => "Article 钩子",
        "comment" => "Comment 钩子",
        "user" => "User 钩子",
        "content_processing" => "Content Processing 钩子",
        "api" => "API 钩子",
        "system" => "System 钩子",
        "plugin" => "Plugin 钩子",
        "frontend" => "Frontend 钩子",
        "navigation" => "Navigation 钩子",
        other => return other, // fallback — caller must handle lifetime
    }
}

/// Render a `serde_json::Value` object as a Markdown table of field→type rows.
/// Returns `None` when the value is `null` or not an object.
fn schema_table(schema: &serde_json::Value) -> Option<String> {
    let obj = schema.as_object()?;
    if obj.is_empty() {
        return None;
    }
    let mut out = String::from("| 字段 | 类型 |\n|------|------|\n");
    for (field, typ) in obj {
        let type_str = match typ {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        out.push_str(&format!("| {} | {} |\n", field, type_str));
    }
    Some(out)
}

fn type_label(t: &HookType) -> &'static str {
    match t {
        HookType::Filter => "Filter",
        HookType::Action => "Action",
    }
}

fn scope_label(s: &HookScope) -> &'static str {
    match s {
        HookScope::Backend => "backend",
        HookScope::Frontend => "frontend",
        HookScope::Both => "both",
    }
}

/// Generate a complete Markdown document describing all hooks in the registry.
///
/// The output follows the format specified in the design document: hooks are
/// grouped by category, and each entry contains name, type, description,
/// trigger point, input/output schema, scope, and available-since version.
pub fn generate_hook_docs(registry: &HookRegistry) -> String {
    let mut doc = String::new();

    // Header
    doc.push_str(&format!(
        "# Noteva 钩子数据契约\n\n> 自动生成于 hook-registry.json v{}\n\n",
        registry.version
    ));

    // Group by category and sort category names for deterministic output
    let categories = registry.by_category();
    let mut cat_names: Vec<&String> = categories.keys().collect();
    cat_names.sort();

    for cat in cat_names {
        let hooks = &categories[cat];
        let display = category_display_name(cat);
        doc.push_str(&format!("## {}\n\n", display));

        for hook in hooks {
            doc.push_str(&format!(
                "### {} ({})\n\n",
                hook.name,
                type_label(&hook.hook_type)
            ));
            doc.push_str(&format!("- **描述**: {}\n", hook.description));
            doc.push_str(&format!("- **触发位置**: {}\n", hook.trigger_point));
            doc.push_str(&format!("- **作用域**: {}\n", scope_label(&hook.scope)));
            doc.push_str(&format!(
                "- **可用版本**: {}\n",
                hook.available_since
            ));
            doc.push('\n');

            // Input schema
            doc.push_str("**输入数据**:\n");
            match schema_table(&hook.input_schema) {
                Some(table) => doc.push_str(&table),
                None => doc.push_str("无\n"),
            }
            doc.push('\n');

            // Output schema
            doc.push_str("**输出数据**:\n");
            match &hook.output_schema {
                Some(val) => match schema_table(val) {
                    Some(table) => doc.push_str(&table),
                    None => doc.push_str("无\n"),
                },
                None => doc.push_str("无\n"),
            }
            doc.push_str("\n---\n\n");
        }
    }

    doc
}
