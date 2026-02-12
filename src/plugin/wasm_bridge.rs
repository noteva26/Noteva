//! WASM Plugin Bridge
//!
//! Connects the WASM PluginRuntime with the HookManager,
//! enabling WASM plugins to participate in backend hooks.
//!
//! WASM execution is isolated in a subprocess (`wasm-worker`) to prevent
//! WASM traps from crashing the main server process. This is critical on
//! Windows where wasmtime's SEH signal handling causes unrecoverable aborts.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use serde_json::Value;

use super::{PluginRuntime, PluginHandle, PluginManifest, Permission, PluginError};
use super::hooks::{HookManager, PRIORITY_DEFAULT};
use super::hook_registry::validate_plugin_hooks;
use super::loader::{Plugin, PluginManager};
use crate::db::repositories::{PluginDataRepository, SqlxPluginDataRepository, ArticleRepository, SqlxArticleRepository, CommentRepository, SqlxCommentRepository};
use crate::db::DynDatabasePool;

/// Mapping from plugin_id to its WASM handle
#[derive(Debug, Default)]
pub struct WasmPluginRegistry {
    entries: std::collections::HashMap<String, WasmPluginEntry>,
}

#[derive(Debug)]
struct WasmPluginEntry {
    handle: PluginHandle,
    _hooks: Vec<String>,
}

impl WasmPluginRegistry {
    pub fn new() -> Self {
        Self { entries: std::collections::HashMap::new() }
    }

    pub fn has_plugin(&self, plugin_id: &str) -> bool {
        self.entries.contains_key(plugin_id)
    }

    pub fn get_handle(&self, plugin_id: &str) -> Option<PluginHandle> {
        self.entries.get(plugin_id).map(|e| e.handle)
    }
}

fn find_worker_exe() -> String {
    if let Ok(current_exe) = std::env::current_exe() {
        let dir = current_exe.parent().unwrap_or(std::path::Path::new("."));
        let worker_name = if cfg!(windows) { "wasm-worker.exe" } else { "wasm-worker" };
        let worker_path = dir.join(worker_name);
        if worker_path.exists() {
            return worker_path.to_string_lossy().to_string();
        }
    }
    if cfg!(windows) { "wasm-worker.exe".to_string() } else { "wasm-worker".to_string() }
}

/// Result from wasm-worker subprocess execution.
struct SubprocessResult {
    /// Hook output bytes (if any)
    output: Option<Vec<u8>>,
    /// Storage operations to execute on the database
    storage_ops: Vec<(String, String, Option<String>)>, // (op, key, value)
    /// Meta update operations to execute on articles
    meta_ops: Vec<(i64, String)>, // (article_id, data_json)
}

/// Execute a WASM hook function via subprocess isolation.
fn execute_wasm_subprocess(
    wasm_path: &str,
    func_name: &str,
    input_bytes: &[u8],
    permissions: &[String],
    plugin_id: &str,
    plugin_data: &std::collections::HashMap<String, String>,
    articles: Option<&Vec<Value>>,
    comments: Option<&Value>,
) -> SubprocessResult {
    use std::process::{Command, Stdio};
    use std::io::Write;

    let worker_exe = find_worker_exe();
    let input_b64 = base64_encode(input_bytes);

    // Convert plugin_data to JSON object
    let pd_json: serde_json::Map<String, Value> = plugin_data
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();

    let mut request = serde_json::json!({
        "wasm_path": wasm_path,
        "func_name": func_name,
        "input": input_b64,
        "permissions": permissions,
        "plugin_id": plugin_id,
        "plugin_data": pd_json,
    });

    // Attach articles data if provided (for host_query_articles)
    if let Some(arts) = articles {
        request["articles"] = Value::Array(arts.clone());
    }
    // Attach comments data if provided (for host_get_comments)
    if let Some(cmts) = comments {
        request["comments"] = cmts.clone();
    }
    let request_str = request.to_string();

    let empty_result = SubprocessResult { output: None, storage_ops: vec![], meta_ops: vec![] };

    let mut child = match Command::new(&worker_exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to spawn wasm-worker: {}", e);
            return empty_result;
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(request_str.as_bytes()) {
            tracing::error!("Failed to write to wasm-worker stdin: {}", e);
            let _ = child.kill();
            return empty_result;
        }
    }

    // Adjust timeout based on context:
    // - plugin_activate hooks may process many articles (batch), need longer timeout
    // - network-capable plugins need more time for API calls
    // - default is 5s for simple hooks
    let timeout = if func_name.contains("plugin_activate") {
        std::time::Duration::from_secs(300) // 5 minutes for batch operations
    } else if permissions.iter().any(|p| p == "network") {
        std::time::Duration::from_secs(30)
    } else {
        std::time::Duration::from_secs(5)
    };

    let result = match wait_with_timeout(&mut child, timeout) {
        Ok(output) => {
            // Log stderr from wasm-worker (contains host_log output)
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            if !stderr_str.is_empty() {
                for line in stderr_str.lines() {
                    tracing::info!("{}", line);
                }
            }
            output
        }
        Err(e) => {
            tracing::warn!("wasm-worker timed out or failed: {}", e);
            let _ = child.kill();
            let _ = child.wait();
            return empty_result;
        }
    };

    let stdout_str = String::from_utf8_lossy(&result.stdout);
    let response: Value = match serde_json::from_str(&stdout_str) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("wasm-worker returned invalid JSON: {} (output: {})", e, stdout_str);
            return empty_result;
        }
    };

    if response.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let err = response.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
        tracing::warn!("wasm-worker execution failed: {}", err);
        return empty_result;
    }

    // Parse storage_ops
    let storage_ops: Vec<(String, String, Option<String>)> = response
        .get("storage_ops")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().filter_map(|op| {
                let op_type = op.get("op")?.as_str()?.to_string();
                let key = op.get("key")?.as_str()?.to_string();
                let value = op.get("value").and_then(|v| v.as_str()).map(String::from);
                Some((op_type, key, value))
            }).collect()
        })
        .unwrap_or_default();

    // Parse meta_ops
    let meta_ops: Vec<(i64, String)> = response
        .get("meta_ops")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().filter_map(|op| {
                let article_id = op.get("article_id")?.as_i64()?;
                let data = op.get("data")?.as_str()?.to_string();
                Some((article_id, data))
            }).collect()
        })
        .unwrap_or_default();

    // Decode output
    let output_b64 = response.get("output").and_then(|v| v.as_str()).unwrap_or("");
    let output = match base64_decode(output_b64) {
        Ok(bytes) if !bytes.is_empty() => Some(bytes),
        _ => None,
    };

    SubprocessResult { output, storage_ops, meta_ops }
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: std::time::Duration,
) -> Result<std::process::Output, String> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    use std::io::Read;
                    let _ = out.read_to_end(&mut stdout);
                }
                if let Some(mut err) = child.stderr.take() {
                    use std::io::Read;
                    let _ = err.read_to_end(&mut stderr);
                }
                return Ok(std::process::Output { status, stdout, stderr });
            }
            Ok(None) => {
                if start.elapsed() > timeout { return Err("Timeout".to_string()); }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => return Err(format!("Wait error: {}", e)),
        }
    }
}

/// Execute pending storage operations from a wasm-worker result.
/// This runs in a blocking context since hook callbacks are synchronous.
fn execute_storage_ops(
    pool: &DynDatabasePool,
    plugin_id: &str,
    ops: &[(String, String, Option<String>)],
) {
    if ops.is_empty() { return; }

    let repo = SqlxPluginDataRepository::new(pool.clone());

    // Use a new tokio runtime for blocking execution inside sync hook callback
    // This is safe because wasm-worker hooks run in a thread pool, not on the tokio runtime
    let rt = match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // We're inside a tokio context, use block_in_place
            for (op, key, value) in ops {
                let repo = &repo;
                let pid = plugin_id;
                match op.as_str() {
                    "set" => {
                        if let Some(val) = value {
                            match tokio::task::block_in_place(|| {
                                handle.block_on(repo.set(pid, key, val))
                            }) {
                                Ok(_) => tracing::info!("Storage set OK: {}:{}", pid, key),
                                Err(e) => tracing::error!("Storage set FAILED: {}:{} - {}", pid, key, e),
                            }
                        }
                    }
                    "delete" => {
                        match tokio::task::block_in_place(|| {
                            handle.block_on(repo.delete(pid, key))
                        }) {
                            Ok(_) => tracing::info!("Storage delete OK: {}:{}", pid, key),
                            Err(e) => tracing::error!("Storage delete FAILED: {}:{} - {}", pid, key, e),
                        }
                    }
                    _ => {}
                }
            }
            return;
        }
        Err(_) => {
            // Not inside tokio, create a temporary runtime
            match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create runtime for storage ops: {}", e);
                    return;
                }
            }
        }
    };

    for (op, key, value) in ops {
        match op.as_str() {
            "set" => {
                if let Some(val) = value {
                    let _ = rt.block_on(repo.set(plugin_id, key, val));
                }
            }
            "delete" => {
                let _ = rt.block_on(repo.delete(plugin_id, key));
            }
            _ => {}
        }
    }
}

/// Execute meta update operations returned by the WASM subprocess.
fn execute_meta_ops(
    pool: &DynDatabasePool,
    plugin_id: &str,
    ops: &[(i64, String)],
) {
    if ops.is_empty() { return; }

    let repo = SqlxArticleRepository::new(pool.clone());

    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            for (article_id, data_json) in ops {
                let data: serde_json::Value = match serde_json::from_str(data_json) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("Invalid meta JSON for article {}: {}", article_id, e);
                        continue;
                    }
                };
                match tokio::task::block_in_place(|| {
                    handle.block_on(repo.update_meta(*article_id, plugin_id, &data))
                }) {
                    Ok(_) => tracing::info!("Meta update OK: article {} by plugin {}", article_id, plugin_id),
                    Err(e) => tracing::error!("Meta update FAILED: article {} - {}", article_id, e),
                }
            }
        }
        Err(_) => {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create runtime for meta ops: {}", e);
                    return;
                }
            };
            for (article_id, data_json) in ops {
                let data: serde_json::Value = match serde_json::from_str(data_json) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let _ = rt.block_on(repo.update_meta(*article_id, plugin_id, &data));
            }
        }
    }
}

/// Load pre-existing plugin data from the database for passing to wasm-worker.
fn load_plugin_data_sync(
    pool: &DynDatabasePool,
    plugin_id: &str,
) -> std::collections::HashMap<String, String> {
    let repo = SqlxPluginDataRepository::new(pool.clone());

    let data = match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            tokio::task::block_in_place(|| {
                handle.block_on(repo.get_all(plugin_id))
            })
        }
        Err(_) => {
            match tokio::runtime::Runtime::new() {
                Ok(rt) => rt.block_on(repo.get_all(plugin_id)),
                Err(_) => return std::collections::HashMap::new(),
            }
        }
    };

    match data {
        Ok(entries) => entries.into_iter().map(|e| (e.key, e.value)).collect(),
        Err(e) => {
            tracing::warn!("Failed to load plugin data for '{}': {}", plugin_id, e);
            std::collections::HashMap::new()
        }
    }
}

/// Load published articles from the database (for host_query_articles).
/// Returns simplified article data (id, title, slug, content, status).
fn load_articles_sync(
    pool: &DynDatabasePool,
) -> Vec<Value> {
    let repo = SqlxArticleRepository::new(pool.clone());

    let articles = match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            tokio::task::block_in_place(|| {
                handle.block_on(repo.list_published(0, 10000))
            })
        }
        Err(_) => {
            match tokio::runtime::Runtime::new() {
                Ok(rt) => rt.block_on(repo.list_published(0, 10000)),
                Err(_) => return vec![],
            }
        }
    };

    match articles {
        Ok(list) => list.iter().map(|a| {
            serde_json::json!({
                "id": a.id,
                "title": a.title,
                "slug": a.slug,
                "content": a.content,
                "status": format!("{:?}", a.status),
            })
        }).collect(),
        Err(e) => {
            tracing::warn!("Failed to load articles for WASM plugin: {}", e);
            vec![]
        }
    }
}

/// Load approved comments grouped by article_id (for host_get_comments).
/// Returns a map of article_id -> Vec<comment> as serde_json::Value.
fn load_comments_sync(
    pool: &DynDatabasePool,
) -> Value {
    let repo = SqlxCommentRepository::new(pool.clone());

    // Get all published article IDs first, then load comments for each
    let article_repo = SqlxArticleRepository::new(pool.clone());
    let articles = match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            tokio::task::block_in_place(|| {
                handle.block_on(article_repo.list_published(0, 10000))
            })
        }
        Err(_) => {
            match tokio::runtime::Runtime::new() {
                Ok(rt) => rt.block_on(article_repo.list_published(0, 10000)),
                Err(_) => return serde_json::json!({}),
            }
        }
    };

    let article_ids: Vec<i64> = match articles {
        Ok(list) => list.iter().map(|a| a.id).collect(),
        Err(_) => return serde_json::json!({}),
    };

    let mut map = serde_json::Map::new();
    for article_id in article_ids {
        let comments = match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                tokio::task::block_in_place(|| {
                    handle.block_on(repo.get_by_article(article_id, None))
                })
            }
            Err(_) => continue,
        };
        if let Ok(list) = comments {
            let arr: Vec<Value> = list.iter().map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "article_id": c.article_id,
                    "parent_id": c.parent_id,
                    "nickname": c.nickname,
                    "content": c.content,
                    "status": format!("{}", c.status),
                    "created_at": c.created_at.to_rfc3339(),
                })
            }).collect();
            if !arr.is_empty() {
                map.insert(article_id.to_string(), Value::Array(arr));
            }
        }
    }
    Value::Object(map)
}

/// Load a WASM module for a plugin and register its backend hooks.
pub async fn load_wasm_plugin(
    plugin: &Plugin,
    runtime: &Arc<RwLock<PluginRuntime>>,
    hook_manager: &Arc<HookManager>,
    registry: &Arc<RwLock<WasmPluginRegistry>>,
    pool: &DynDatabasePool,
) -> Result<(), PluginError> {
    let wasm_path = plugin.path.join("backend.wasm");
    if !wasm_path.exists() { return Ok(()); }

    let plugin_id = plugin.metadata.id.clone();
    let backend_hooks = plugin.metadata.hooks.backend.clone();
    let frontend_hooks = &plugin.metadata.hooks.frontend;

    // Validate declared hooks against the registry (warnings only, never blocks loading)
    let hook_registry = hook_manager.registry();
    let validation_warnings = validate_plugin_hooks(
        hook_registry,
        &plugin_id,
        &backend_hooks,
        frontend_hooks,
    );
    for w in &validation_warnings {
        warn!("{}", w);
    }

    if backend_hooks.is_empty() {
        warn!("Plugin '{}' has backend.wasm but no hooks.backend declared, skipping", plugin_id);
        return Ok(());
    }

    info!("Loading WASM module for plugin '{}'...", plugin_id);

    let wasm_bytes = tokio::fs::read(&wasm_path).await.map_err(|e| {
        PluginError::WasmError(format!("Failed to read {}: {}", wasm_path.display(), e))
    })?;

    let manifest = PluginManifest {
        name: plugin.metadata.name.clone(),
        version: plugin.metadata.version.clone(),
        description: Some(plugin.metadata.description.clone()),
        author: Some(plugin.metadata.author.clone()),
        permissions: plugin.metadata.permissions.iter()
            .filter_map(|p| Permission::from_str(p)).collect(),
    };

    let handle = {
        let mut rt = runtime.write().await;
        rt.load_plugin_bytes(&wasm_bytes, manifest)?
    };

    info!("WASM module loaded for plugin '{}' (handle: {:?}), registering {} backend hooks",
        plugin_id, handle, backend_hooks.len());

    let wasm_abs_path = std::fs::canonicalize(&wasm_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| wasm_path.to_string_lossy().to_string());

    let plugin_permissions: Vec<String> = plugin.metadata.permissions.clone();
    // Auto-add "storage" permission if not declared (all WASM plugins get basic storage)
    let mut perms_with_storage = plugin_permissions.clone();
    if !perms_with_storage.contains(&"storage".to_string()) {
        perms_with_storage.push("storage".to_string());
    }

    let plugin_settings: serde_json::Map<String, Value> = plugin.settings
        .iter().map(|(k, v)| (k.clone(), v.clone())).collect();

    let db_pool = pool.clone();

    for hook_name in &backend_hooks {
        let wasm_file = wasm_abs_path.clone();
        let hook_fn_name = format!("hook_{}", hook_name);
        let hook_fn_name_log = hook_fn_name.clone();
        let pid = plugin_id.clone();
        let perms = perms_with_storage.clone();
        let settings = plugin_settings.clone();
        let pool_clone = db_pool.clone();
        let has_read_articles = plugin_permissions.contains(&"read_articles".to_string());
        let has_read_comments = plugin_permissions.contains(&"read_comments".to_string());

        hook_manager.register(
            hook_name,
            move |data: &mut Value| {
                tracing::info!("WASM hook '{}' triggered for plugin '{}'", hook_fn_name, pid);

                // Inject plugin settings into hook data
                if let Value::Object(ref mut map) = data {
                    for (k, v) in &settings {
                        if !map.contains_key(k) {
                            map.insert(k.clone(), v.clone());
                        }
                    }
                }

                let input_bytes = serde_json::to_vec(data).unwrap_or_default();
                let pid = pid.clone();
                let perms = perms.clone();
                let wasm_file = wasm_file.clone();
                let hook_fn = hook_fn_name.clone();
                let pool = pool_clone.clone();

                // Load current plugin data for the worker
                let plugin_data = load_plugin_data_sync(&pool, &pid);

                // Load articles if plugin has read_articles permission
                let articles = if has_read_articles {
                    Some(load_articles_sync(&pool))
                } else {
                    None
                };

                // Load comments if plugin has read_comments permission
                let comments = if has_read_comments {
                    Some(load_comments_sync(&pool))
                } else {
                    None
                };

                // Execute WASM in subprocess
                let result = execute_wasm_subprocess(
                    &wasm_file, &hook_fn, &input_bytes, &perms, &pid, &plugin_data,
                    articles.as_ref(), comments.as_ref(),
                );

                // Execute any storage operations the plugin requested
                if !result.storage_ops.is_empty() {
                    execute_storage_ops(&pool, &pid, &result.storage_ops);
                    tracing::info!("Plugin '{}' executed {} storage ops", pid, result.storage_ops.len());
                }

                // Execute any meta update operations the plugin requested
                if !result.meta_ops.is_empty() {
                    execute_meta_ops(&pool, &pid, &result.meta_ops);
                    tracing::info!("Plugin '{}' executed {} meta ops", pid, result.meta_ops.len());
                }

                // Return hook output
                result.output.and_then(|bytes| {
                    serde_json::from_slice::<Value>(&bytes).ok()
                })
            },
            PRIORITY_DEFAULT,
            Some(plugin_id.clone()),
        );

        info!("  Registered WASM hook: {} -> {}", hook_name, hook_fn_name_log);
    }

    {
        let mut reg = registry.write().await;
        reg.entries.insert(plugin_id.clone(), WasmPluginEntry {
            handle, _hooks: backend_hooks,
        });
    }

    info!("WASM plugin '{}' fully loaded and hooked (subprocess isolation)", plugin_id);
    Ok(())
}

/// Unload a WASM plugin and remove its hook registrations.
pub async fn unload_wasm_plugin(
    plugin_id: &str,
    runtime: &Arc<RwLock<PluginRuntime>>,
    hook_manager: &Arc<HookManager>,
    registry: &Arc<RwLock<WasmPluginRegistry>>,
) -> Result<(), PluginError> {
    let entry = {
        let mut reg = registry.write().await;
        reg.entries.remove(plugin_id)
    };

    if let Some(entry) = entry {
        hook_manager.unregister_plugin(plugin_id);
        let mut rt = runtime.write().await;
        if let Err(e) = rt.unload_plugin(entry.handle).await {
            warn!("Failed to unload WASM module for '{}': {}", plugin_id, e);
        }
        info!("WASM plugin '{}' unloaded", plugin_id);
    }
    Ok(())
}

/// Scan all enabled plugins and load any that have backend.wasm files.
pub async fn load_all_wasm_plugins(
    plugin_manager: &PluginManager,
    runtime: &Arc<RwLock<PluginRuntime>>,
    hook_manager: &Arc<HookManager>,
    registry: &Arc<RwLock<WasmPluginRegistry>>,
    pool: &DynDatabasePool,
) {
    let enabled_plugins: Vec<Plugin> = plugin_manager.get_enabled().into_iter().cloned().collect();

    for plugin in &enabled_plugins {
        let wasm_path = plugin.path.join("backend.wasm");
        if wasm_path.exists() {
            if let Err(e) = load_wasm_plugin(plugin, runtime, hook_manager, registry, pool).await {
                error!("Failed to load WASM plugin '{}': {}", plugin.metadata.id, e);
            }
        }
    }
}

/// Execute a plugin's `handle_request` function via subprocess for custom API routes.
///
/// Returns `(status_code, content_type, body_bytes)` or None if execution failed.
pub fn execute_plugin_api_request(
    wasm_path: &str,
    plugin_id: &str,
    permissions: &[String],
    request_data: &serde_json::Value,
    pool: &DynDatabasePool,
) -> Option<(u16, String, Vec<u8>)> {
    let input_bytes = serde_json::to_vec(request_data).unwrap_or_default();

    let mut perms = permissions.to_vec();
    if !perms.contains(&"storage".to_string()) {
        perms.push("storage".to_string());
    }

    let plugin_data = load_plugin_data_sync(pool, plugin_id);

    let result = execute_wasm_subprocess(
        wasm_path,
        "handle_request",
        &input_bytes,
        &perms,
        plugin_id,
        &plugin_data,
        None,
        None,
    );

    // Execute storage ops if any
    if !result.storage_ops.is_empty() {
        execute_storage_ops(pool, plugin_id, &result.storage_ops);
    }
    if !result.meta_ops.is_empty() {
        execute_meta_ops(pool, plugin_id, &result.meta_ops);
    }

    // Parse response from WASM output
    let output = result.output?;
    let response: serde_json::Value = serde_json::from_slice(&output).ok()?;

    let status = response.get("status").and_then(|v| v.as_u64()).unwrap_or(200) as u16;
    let content_type = response.get("content_type")
        .and_then(|v| v.as_str())
        .unwrap_or("application/json")
        .to_string();
    let body = if let Some(b) = response.get("body").and_then(|v| v.as_str()) {
        b.as_bytes().to_vec()
    } else {
        response.get("body").map(|v| v.to_string().into_bytes()).unwrap_or_default()
    };

    Some((status, content_type, body))
}

// Base64 encode/decode

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 { result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char); }
        else { result.push('='); }
        if chunk.len() > 2 { result.push(CHARS[(triple & 0x3F) as usize] as char); }
        else { result.push('='); }
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    if input.is_empty() { return Ok(vec![]); }
    let mut result = Vec::new();
    let chars: Vec<u8> = input.bytes().filter(|&b| b != b'\n' && b != b'\r').collect();
    if chars.len() % 4 != 0 { return Err("Invalid base64 length".to_string()); }
    for chunk in chars.chunks(4) {
        let vals: Vec<u32> = chunk.iter().map(|&c| decode_b64_char(c)).collect();
        if vals.iter().any(|&v| v == 255) { return Err("Invalid base64 character".to_string()); }
        let triple = (vals[0] << 18) | (vals[1] << 12) | (vals[2] << 6) | vals[3];
        result.push(((triple >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' { result.push(((triple >> 8) & 0xFF) as u8); }
        if chunk[3] != b'=' { result.push((triple & 0xFF) as u8); }
    }
    Ok(result)
}

fn decode_b64_char(c: u8) -> u32 {
    match c {
        b'A'..=b'Z' => (c - b'A') as u32,
        b'a'..=b'z' => (c - b'a' + 26) as u32,
        b'0'..=b'9' => (c - b'0' + 52) as u32,
        b'+' => 62, b'/' => 63, b'=' => 0,
        _ => 255,
    }
}
