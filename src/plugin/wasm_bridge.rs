//! WASM Plugin Bridge
//!
//! Connects the WASM PluginRuntime with the HookManager,
//! enabling WASM plugins to participate in backend hooks.
//!
//! WASM execution is isolated in persistent subprocess workers (`wasm-worker`)
//! to prevent WASM traps from crashing the main server process. Workers are
//! pooled and reused across requests, with compiled WASM modules cached in
//! each worker for fast subsequent invocations.

use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::RwLock;
use tracing::{debug, warn, error};
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

/// Sanitize plugin log output to prevent leaking sensitive information.
///
/// Masks:
/// - Full URLs (API endpoints) → shows only scheme + host
/// - API keys / tokens (common patterns like sk-xxx, Bearer xxx)
/// - Email addresses
fn sanitize_plugin_log(line: &str) -> String {
    let mut result = line.to_string();

    // Mask full URLs: keep scheme + host, hide path/query
    // Match http(s)://host.domain/anything...
    let mut sanitized = String::new();
    let mut rest = result.as_str();
    while let Some(start) = rest.find("http://").or_else(|| rest.find("https://")) {
        sanitized.push_str(&rest[..start]);
        let url_start = &rest[start..];
        // Find the scheme
        let scheme_end = url_start.find("://").unwrap() + 3;
        let after_scheme = &url_start[scheme_end..];
        // Find end of host (first / or space or end)
        let host_end = after_scheme.find(|c: char| c == '/' || c == '?' || c == ' ' || c == '"' || c == '\'' || c == ')')
            .unwrap_or(after_scheme.len());
        let host = &after_scheme[..host_end];
        sanitized.push_str(&url_start[..scheme_end]);
        sanitized.push_str(host);
        sanitized.push_str("/***");
        // Skip past the full URL
        let url_end = scheme_end + host_end;
        let remaining = &url_start[url_end..];
        let url_tail = remaining.find(|c: char| c == ' ' || c == '"' || c == '\'' || c == ')' || c == ']' || c == '}')
            .unwrap_or(remaining.len());
        rest = &remaining[url_tail..];
    }
    sanitized.push_str(rest);
    result = sanitized;

    // Mask common API key patterns: sk-xxx, key-xxx, Bearer xxx
    let key_patterns = ["sk-", "key-", "Key-", "KEY-"];
    for pat in &key_patterns {
        while let Some(pos) = result.find(pat) {
            let end = result[pos..].find(|c: char| c == '"' || c == '\'' || c == ' ' || c == ',' || c == '}')
                .map(|e| pos + e)
                .unwrap_or(result.len());
            let visible = (pos + pat.len()).min(pos + pat.len() + 4).min(end);
            let masked = format!("{}{}***", pat, &result[pos + pat.len()..visible]);
            result.replace_range(pos..end, &masked);
        }
    }

    // Mask "Bearer <token>" → "Bearer ***"
    if let Some(pos) = result.find("Bearer ") {
        let token_start = pos + 7;
        let token_end = result[token_start..].find(|c: char| c == '"' || c == '\'' || c == ' ' || c == ',')
            .map(|e| token_start + e)
            .unwrap_or(result.len());
        if token_end > token_start {
            result.replace_range(token_start..token_end, "***");
        }
    }

    result
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

/// A persistent wasm-worker subprocess with stdin/stdout handles.
struct Worker {
    child: std::process::Child,
    stdin: std::io::BufWriter<std::process::ChildStdin>,
    stdout: std::io::BufReader<std::process::ChildStdout>,
}

impl Worker {
    fn spawn() -> Option<Self> {
        use std::process::{Command, Stdio};
        use std::io::{BufWriter, BufReader};

        let worker_exe = find_worker_exe();
        let mut child = Command::new(&worker_exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ok()?;

        let stdin = BufWriter::new(child.stdin.take()?);
        let stdout = BufReader::new(child.stdout.take()?);

        // Spawn a thread to drain stderr (host_log output) so it doesn't block
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                use std::io::{BufRead, BufReader};
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let sanitized = sanitize_plugin_log(&line);
                        // Parse plugin log level from format: [wasm:plugin_id][level] message
                        if sanitized.contains("[error]") {
                            tracing::error!("{}", sanitized);
                        } else if sanitized.contains("[warn]") {
                            tracing::warn!("{}", sanitized);
                        } else {
                            tracing::debug!("{}", sanitized);
                        }
                    }
                }
            });
        }

        Some(Self { child, stdin, stdout })
    }

    fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn send_request(&mut self, request: &serde_json::Value) -> Option<serde_json::Value> {
        use std::io::{Write, BufRead};

        let request_str = request.to_string();
        // Write request as a single line
        if self.stdin.write_all(request_str.as_bytes()).is_err() { return None; }
        if self.stdin.write_all(b"\n").is_err() { return None; }
        if self.stdin.flush().is_err() { return None; }

        // Read one line of response
        let mut response_line = String::new();
        if self.stdout.read_line(&mut response_line).is_err() { return None; }
        if response_line.is_empty() { return None; }

        serde_json::from_str(response_line.trim()).ok()
    }

    fn shutdown(mut self) {
        let _ = self.send_request(&serde_json::json!({"cmd": "shutdown"}));
        let _ = self.child.wait();
    }
}

/// Pool of persistent wasm-worker subprocesses.
/// Workers are reused across hook invocations to avoid spawn + compile overhead.
pub struct WorkerPool {
    workers: StdMutex<Vec<Worker>>,
    pool_size: usize,
}

impl WorkerPool {
    pub fn new(pool_size: usize) -> Self {
        let mut workers = Vec::with_capacity(pool_size);
        for _ in 0..pool_size {
            if let Some(w) = Worker::spawn() {
                workers.push(w);
            }
        }
        let actual = workers.len();
        if actual < pool_size {
            tracing::warn!("WorkerPool: only spawned {}/{} workers", actual, pool_size);
        } else {
            tracing::debug!("WorkerPool: spawned {} persistent workers", actual);
        }
        Self { workers: StdMutex::new(workers), pool_size }
    }

    /// Take an available worker from the pool (or spawn a new one).
    fn acquire(&self) -> Option<Worker> {
        let mut pool = self.workers.lock().ok()?;
        // Try to get an existing live worker
        while let Some(mut w) = pool.pop() {
            if w.is_alive() {
                return Some(w);
            }
            // Dead worker, drop it and try next
            let _ = w.child.kill();
        }
        drop(pool);
        // Pool empty, spawn a fresh one
        Worker::spawn()
    }

    /// Return a worker to the pool after use.
    fn release(&self, mut worker: Worker) {
        if !worker.is_alive() { return; }
        if let Ok(mut pool) = self.workers.lock() {
            if pool.len() < self.pool_size {
                pool.push(worker);
                return;
            }
        }
        // Pool full, shut down the extra worker
        worker.shutdown();
    }

    /// Execute a request on a pooled worker with timeout.
    fn execute(&self, request: &serde_json::Value, timeout: std::time::Duration) -> SubprocessResult {
        let empty = SubprocessResult { output: None, storage_ops: vec![], meta_ops: vec![] };

        let mut worker = match self.acquire() {
            Some(w) => w,
            None => {
                tracing::error!("WorkerPool: failed to acquire worker");
                return empty;
            }
        };

        // For requests with timeout, we use a thread to enforce it
        let request_clone = request.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        // We need to move the worker into the thread and get it back
        let handle = std::thread::spawn(move || {
            let response = worker.send_request(&request_clone);
            let _ = tx.send(());
            (worker, response)
        });

        match rx.recv_timeout(timeout) {
            Ok(()) => {
                // Thread completed within timeout
                match handle.join() {
                    Ok((worker, response)) => {
                        self.release(worker);
                        match response {
                            Some(resp) => parse_subprocess_response(&resp),
                            None => empty,
                        }
                    }
                    Err(_) => empty,
                }
            }
            Err(_) => {
                // Timeout — the thread is still blocked on I/O.
                // We can't easily kill the thread, but the worker will be dropped
                // when the thread eventually finishes (or the process exits).
                tracing::warn!("WorkerPool: request timed out after {:?}", timeout);
                // Don't join — let the thread finish in background
                // The worker won't be returned to pool (it's consumed by the thread)
                empty
            }
        }
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        if let Ok(mut pool) = self.workers.lock() {
            for worker in pool.drain(..) {
                worker.shutdown();
            }
        }
    }
}

/// Global worker pool, initialized once.
static WORKER_POOL: once_cell::sync::Lazy<WorkerPool> = once_cell::sync::Lazy::new(|| {
    WorkerPool::new(4)
});

/// In-memory cache for plugin data, keyed by plugin_id.
/// Avoids querying the database on every hook invocation.
/// Invalidated when storage ops are executed for a plugin.
static PLUGIN_DATA_CACHE: once_cell::sync::Lazy<StdMutex<std::collections::HashMap<String, (std::time::Instant, std::collections::HashMap<String, String>)>>> =
    once_cell::sync::Lazy::new(|| StdMutex::new(std::collections::HashMap::new()));

/// Plugin data cache TTL (5 minutes)
const PLUGIN_DATA_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(300);

/// Result from wasm-worker subprocess execution.
struct SubprocessResult {
    /// Hook output bytes (if any)
    output: Option<Vec<u8>>,
    /// Storage operations to execute on the database
    storage_ops: Vec<(String, String, Option<String>)>, // (op, key, value)
    /// Meta update operations to execute on articles
    meta_ops: Vec<(i64, String)>, // (article_id, data_json)
}

/// Parse the JSON response from a wasm-worker into a SubprocessResult.
fn parse_subprocess_response(response: &serde_json::Value) -> SubprocessResult {
    let empty = SubprocessResult { output: None, storage_ops: vec![], meta_ops: vec![] };

    if response.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let raw_err = response.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
        // Clean up WASM backtrace noise — only show the first meaningful line
        let clean_err = if let Some(pos) = raw_err.find("\n") {
            raw_err[..pos].trim()
        } else {
            raw_err
        };
        // Strip redundant prefixes
        let clean_err = clean_err
            .trim_start_matches("WASM execution failed: ")
            .trim_start_matches("Function call failed: ");
        tracing::warn!("Plugin WASM execution failed: {}", clean_err);
        return empty;
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

/// Execute a WASM hook function via a pooled persistent worker.
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
    let input_b64 = base64_encode(input_bytes);

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

    if let Some(arts) = articles {
        request["articles"] = Value::Array(arts.clone());
    }
    if let Some(cmts) = comments {
        request["comments"] = cmts.clone();
    }

    // Adjust timeout based on context
    let timeout = if func_name.contains("plugin_activate") {
        std::time::Duration::from_secs(300)
    } else if permissions.iter().any(|p| p == "network") {
        std::time::Duration::from_secs(30)
    } else {
        std::time::Duration::from_secs(5)
    };

    WORKER_POOL.execute(&request, timeout)
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
                                Ok(_) => tracing::debug!("Storage set OK: {}:{}", pid, key),
                                Err(e) => tracing::error!("Storage set FAILED: {}:{} - {}", pid, key, e),
                            }
                        }
                    }
                    "delete" => {
                        match tokio::task::block_in_place(|| {
                            handle.block_on(repo.delete(pid, key))
                        }) {
                            Ok(_) => tracing::debug!("Storage delete OK: {}:{}", pid, key),
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
                    Ok(_) => tracing::debug!("Meta update OK: article {} by plugin {}", article_id, plugin_id),
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
/// Uses an in-memory cache to avoid querying the DB on every hook invocation.
fn load_plugin_data_sync(
    pool: &DynDatabasePool,
    plugin_id: &str,
) -> std::collections::HashMap<String, String> {
    // Check cache first
    if let Ok(cache) = PLUGIN_DATA_CACHE.lock() {
        if let Some((cached_at, data)) = cache.get(plugin_id) {
            if cached_at.elapsed() < PLUGIN_DATA_CACHE_TTL {
                return data.clone();
            }
        }
    }

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
        Ok(entries) => {
            let result: std::collections::HashMap<String, String> =
                entries.into_iter().map(|e| (e.key, e.value)).collect();
            // Update cache
            if let Ok(mut cache) = PLUGIN_DATA_CACHE.lock() {
                cache.insert(plugin_id.to_string(), (std::time::Instant::now(), result.clone()));
            }
            result
        }
        Err(e) => {
            tracing::warn!("Failed to load plugin data for '{}': {}", plugin_id, e);
            std::collections::HashMap::new()
        }
    }
}

/// Invalidate the plugin data cache for a specific plugin.
fn invalidate_plugin_data_cache(plugin_id: &str) {
    if let Ok(mut cache) = PLUGIN_DATA_CACHE.lock() {
        cache.remove(plugin_id);
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
        debug!("{}", w);
    }

    if backend_hooks.is_empty() {
        warn!("Plugin '{}' has backend.wasm but no hooks.backend declared, skipping", plugin_id);
        return Ok(());
    }

    debug!("Loading WASM module for plugin '{}'...", plugin_id);

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

    debug!("WASM module loaded for plugin '{}' (handle: {:?}), registering {} backend hooks",
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
                tracing::debug!("WASM hook '{}' triggered for plugin '{}'", hook_fn_name, pid);

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
                    invalidate_plugin_data_cache(&pid);
                    tracing::debug!("Plugin '{}' executed {} storage ops", pid, result.storage_ops.len());
                }

                // Execute any meta update operations the plugin requested
                if !result.meta_ops.is_empty() {
                    execute_meta_ops(&pool, &pid, &result.meta_ops);
                    tracing::debug!("Plugin '{}' executed {} meta ops", pid, result.meta_ops.len());
                }

                // Return hook output
                result.output.and_then(|bytes| {
                    serde_json::from_slice::<Value>(&bytes).ok()
                })
            },
            PRIORITY_DEFAULT,
            Some(plugin_id.clone()),
        );

        debug!("  Registered WASM hook: {} -> {}", hook_name, hook_fn_name_log);
    }

    {
        let mut reg = registry.write().await;
        reg.entries.insert(plugin_id.clone(), WasmPluginEntry {
            handle, _hooks: backend_hooks,
        });
    }

    debug!("WASM plugin '{}' fully loaded and hooked (subprocess isolation)", plugin_id);
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
        debug!("WASM plugin '{}' unloaded", plugin_id);
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
        invalidate_plugin_data_cache(plugin_id);
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
