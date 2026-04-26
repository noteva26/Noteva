//! WASM Worker Process
//!
//! Isolated subprocess for executing WASM plugin hooks.
//! This process is spawned by the main Noteva server and stays alive as a
//! persistent worker, processing multiple requests over its lifetime.
//! If the WASM code crashes (trap, OOM, etc.), only this subprocess dies —
//! the main server process is completely unaffected.
//!
//! Protocol (via stdin/stdout, one JSON per line):
//!   Input (JSON line):
//!     {
//!       "wasm_path": "...",
//!       "func_name": "...",
//!       "input": "base64...",
//!       "permissions": ["network", "storage"],
//!       "plugin_id": "ai-summary",
//!       "plugin_data": { "key1": "value1", ... }
//!     }
//!     or: { "cmd": "shutdown" }
//!
//!   Output (JSON line):
//!     { "success": true, "output": "base64...", "storage_ops": [...] }
//!     or: { "success": false, "error": "..." }
//!
//! Host functions provided to WASM plugins:
//!   - host_http_request(...)  -> result_ptr   (requires "network" permission)
//!   - host_storage_get(...)   -> result_ptr   (requires "storage" permission)
//!   - host_storage_set(...)   -> i32          (requires "storage" permission)
//!   - host_storage_delete(...) -> i32         (requires "storage" permission)
//!   - host_log(...)           -> ()           (no permission required)

use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use std::sync::{Arc, Mutex};

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use wasmtime::AsContextMut;

const MAX_WASM_MEMORY_BYTES: usize = 16 * 1024 * 1024;
const MAX_PLUGIN_INPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_HOST_ARG_BYTES: usize = 4 * 1024 * 1024;
const MAX_HOST_RETURN_BYTES: usize = 4 * 1024 * 1024;
const MAX_HTTP_METHOD_BYTES: usize = 16;
const MAX_HTTP_URL_BYTES: usize = 2 * 1024;
const MAX_HTTP_HEADERS_BYTES: usize = 64 * 1024;
const MAX_HTTP_BODY_BYTES: usize = 4 * 1024 * 1024;
const MAX_HTTP_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_STORAGE_OPS: usize = 100;
const MAX_STORAGE_KEY_BYTES: usize = 256;
const MAX_STORAGE_VALUE_BYTES: usize = 64 * 1024;
const MAX_META_OPS: usize = 100;
const MAX_META_VALUE_BYTES: usize = 64 * 1024;
const MAX_DB_OPS: usize = 50;
const MAX_SQL_BYTES: usize = 16 * 1024;
const MAX_DB_PARAMS_BYTES: usize = 64 * 1024;
const MAX_LOG_BYTES: usize = 8 * 1024;
const MAX_LOG_OPS: usize = 100;

/// A storage operation recorded during WASM execution.
/// These are returned to the main process for actual database execution.
#[derive(Clone)]
enum StorageOp {
    Set { key: String, value: String },
    Delete { key: String },
}

/// A meta update operation collected from host_update_article_meta calls.
struct MetaOp {
    article_id: i64,
    data: String, // JSON string
}

/// A database operation recorded during WASM execution.
/// Returned to the main process for actual database execution.
#[derive(Clone)]
enum DbOp {
    /// SELECT query returning JSON rows
    Query { sql: String, params: String },
    /// INSERT/UPDATE/DELETE returning affected rows
    Execute { sql: String, params: String },
}

/// Shared state between host functions and the main execution.
struct WorkerState {
    has_network: bool,
    has_storage: bool,
    has_database: bool,
    has_read_articles: bool,
    has_read_comments: bool,
    has_write_articles: bool,
    plugin_id: String,
    /// Pre-loaded plugin data (read-only cache from main process)
    plugin_data: std::collections::HashMap<String, String>,
    /// Pre-loaded articles data (for host_query_articles and host_get_article)
    articles_json: String,
    /// Pre-loaded comments data keyed by article_id (for host_get_comments)
    comments_json: String,
    /// Collected storage write operations (returned to main process)
    storage_ops: Arc<Mutex<Vec<StorageOp>>>,
    /// Collected meta update operations (returned to main process)
    meta_ops: Arc<Mutex<Vec<MetaOp>>>,
    /// Collected database operations (returned to main process)
    db_ops: Arc<Mutex<Vec<DbOp>>>,
    /// Per-store resource limits applied by Wasmtime.
    limits: wasmtime::StoreLimits,
    /// Number of host_log calls emitted during this request.
    log_count: usize,
}

/// Cached WASM engine + compiled modules for reuse across requests.
struct ModuleCache {
    engine: wasmtime::Engine,
    modules: HashMap<String, wasmtime::Module>,
}

impl ModuleCache {
    fn new() -> Result<Self, String> {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        let engine =
            wasmtime::Engine::new(&config).map_err(|e| format!("Engine creation failed: {}", e))?;
        Ok(Self {
            engine,
            modules: HashMap::new(),
        })
    }

    fn get_or_compile(&mut self, wasm_path: &str) -> Result<wasmtime::Module, String> {
        if let Some(module) = self.modules.get(wasm_path) {
            return Ok(module.clone());
        }
        let wasm_bytes =
            std::fs::read(wasm_path).map_err(|e| format!("Failed to read WASM file: {}", e))?;
        let module = wasmtime::Module::new(&self.engine, &wasm_bytes)
            .map_err(|e| format!("Failed to compile WASM: {}", e))?;
        self.modules.insert(wasm_path.to_string(), module.clone());
        Ok(module)
    }
}

fn main() {
    let mut cache = match ModuleCache::new() {
        Ok(c) => c,
        Err(e) => {
            let result = serde_json::json!({ "success": false, "error": format!("Worker init failed: {}", e) });
            println!("{}", result);
            return;
        }
    };

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    // Persistent loop: read one JSON request per line, process, respond
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break, // stdin closed, exit gracefully
        };

        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let _ = writeln!(
                    stdout_lock,
                    "{}",
                    serde_json::json!({ "success": false, "error": format!("Invalid JSON: {}", e) })
                );
                let _ = stdout_lock.flush();
                continue;
            }
        };

        // Shutdown command
        if request.get("cmd").and_then(|v| v.as_str()) == Some("shutdown") {
            break;
        }

        let result = handle_request(&request, &mut cache);
        let _ = writeln!(stdout_lock, "{}", result);
        let _ = stdout_lock.flush();
    }
}

fn handle_request(request: &serde_json::Value, cache: &mut ModuleCache) -> serde_json::Value {
    let wasm_path = match request.get("wasm_path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return serde_json::json!({ "success": false, "error": "Missing 'wasm_path'" }),
    };
    let func_name = match request.get("func_name").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => return serde_json::json!({ "success": false, "error": "Missing 'func_name'" }),
    };
    let input_b64 = request.get("input").and_then(|v| v.as_str()).unwrap_or("");
    let input_bytes = match base64_decode(input_b64) {
        Ok(b) => b,
        Err(e) => {
            return serde_json::json!({ "success": false, "error": format!("Failed to decode input: {}", e) })
        }
    };

    // Parse permissions
    let permissions: Vec<String> = request
        .get("permissions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let has_network = permissions.iter().any(|p| p == "network");
    let has_storage = permissions.iter().any(|p| p == "storage");
    let has_database = permissions.iter().any(|p| p == "database" || p == "db");
    let has_read_articles = permissions
        .iter()
        .any(|p| p == "read_articles" || p == "read-articles");
    let has_read_comments = permissions
        .iter()
        .any(|p| p == "read_comments" || p == "read-comments");
    let has_write_articles = permissions
        .iter()
        .any(|p| p == "write_articles" || p == "write-articles");

    let plugin_id = request
        .get("plugin_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let plugin_data: std::collections::HashMap<String, String> = request
        .get("plugin_data")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let articles_json = request
        .get("articles")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "[]".to_string());

    let comments_json = request
        .get("comments")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let storage_ops = Arc::new(Mutex::new(Vec::new()));
    let meta_ops = Arc::new(Mutex::new(Vec::new()));
    let db_ops = Arc::new(Mutex::new(Vec::new()));

    let state = WorkerState {
        has_network,
        has_storage,
        has_database,
        has_read_articles,
        has_read_comments,
        has_write_articles,
        plugin_id,
        plugin_data,
        articles_json,
        comments_json,
        storage_ops: storage_ops.clone(),
        meta_ops: meta_ops.clone(),
        db_ops: db_ops.clone(),
        limits: wasmtime::StoreLimitsBuilder::new()
            .memory_size(MAX_WASM_MEMORY_BYTES)
            .table_elements(10_000)
            .instances(2)
            .tables(4)
            .memories(1)
            .trap_on_grow_failure(true)
            .build(),
        log_count: 0,
    };

    match execute_wasm(wasm_path, func_name, &input_bytes, state, cache) {
        Ok(output) => {
            let output_b64 = base64_encode(&output);
            let ops: Vec<serde_json::Value> = storage_ops
                .lock()
                .unwrap()
                .iter()
                .map(|op| match op {
                    StorageOp::Set { key, value } => serde_json::json!({
                        "op": "set", "key": key, "value": value
                    }),
                    StorageOp::Delete { key } => serde_json::json!({
                        "op": "delete", "key": key
                    }),
                })
                .collect();

            let mops: Vec<serde_json::Value> = meta_ops
                .lock()
                .unwrap()
                .iter()
                .map(|op| {
                    serde_json::json!({
                        "article_id": op.article_id,
                        "data": op.data,
                    })
                })
                .collect();

            let dops: Vec<serde_json::Value> = db_ops
                .lock()
                .unwrap()
                .iter()
                .map(|op| match op {
                    DbOp::Query { sql, params } => serde_json::json!({
                        "op": "query", "sql": sql, "params": params
                    }),
                    DbOp::Execute { sql, params } => serde_json::json!({
                        "op": "execute", "sql": sql, "params": params
                    }),
                })
                .collect();

            serde_json::json!({
                "success": true,
                "output": output_b64,
                "storage_ops": ops,
                "meta_ops": mops,
                "db_ops": dops,
            })
        }
        Err(e) => {
            // Clean up WASM error: extract first line, strip backtrace noise
            let full_err = format!("{}", e);
            let clean_err = full_err
                .lines()
                .next()
                .unwrap_or(&full_err)
                .trim_start_matches("error while executing at wasm backtrace:")
                .trim();
            let clean_err = if clean_err.is_empty() {
                // If first line was just the backtrace header, try to get the root cause
                full_err
                    .lines()
                    .find(|l| {
                        l.contains("Caused by")
                            || l.contains("wasm trap")
                            || l.contains("unreachable")
                    })
                    .unwrap_or("unknown WASM error")
                    .trim()
            } else {
                clean_err
            };
            serde_json::json!({ "success": false, "error": format!("WASM execution failed: {}", clean_err) })
        }
    }
}

fn do_http_request(method: &str, url: &str, headers_json: &str, body: &[u8]) -> Vec<u8> {
    if body.len() > MAX_HTTP_BODY_BYTES {
        return http_response_json(413, "Request body too large");
    }

    let url = match validate_http_url(url) {
        Ok(url) => url,
        Err(e) => return http_response_json(400, &e),
    };

    let method = method.to_ascii_uppercase();
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
    {
        Ok(client) => client,
        Err(e) => return http_response_json(0, &format!("Client creation failed: {}", e)),
    };

    let mut request_builder = match method.as_str() {
        "GET" => client.get(url.clone()),
        "POST" => client.post(url.clone()),
        "PUT" => client.put(url.clone()),
        "DELETE" => client.delete(url.clone()),
        "PATCH" => client.patch(url),
        _ => return http_response_json(400, "Unsupported method"),
    };

    if !headers_json.is_empty() {
        if let Ok(headers) = serde_json::from_str::<serde_json::Value>(headers_json) {
            if let Some(obj) = headers.as_object() {
                for (key, value) in obj {
                    if !is_allowed_outbound_header(key) {
                        continue;
                    }
                    if let Some(val_str) = value.as_str() {
                        request_builder = request_builder.header(key.as_str(), val_str);
                    }
                }
            }
        }
    }

    if !body.is_empty() && method != "GET" {
        request_builder = request_builder.body(body.to_vec());
    }

    match request_builder.send() {
        Ok(response) => {
            let status = response.status().as_u16();
            let mut limited = Vec::new();
            let mut reader = response.take((MAX_HTTP_RESPONSE_BYTES + 1) as u64);
            if reader.read_to_end(&mut limited).is_err() {
                return http_response_json(0, "Failed to read response body");
            }
            if limited.len() > MAX_HTTP_RESPONSE_BYTES {
                return http_response_json(413, "Response body too large");
            }
            let body_text = String::from_utf8_lossy(&limited).into_owned();
            http_response_json(status, &body_text)
        }
        Err(e) => http_response_json(0, &format!("Request failed: {}", e)),
    }
}

fn http_response_json(status: u16, body: &str) -> Vec<u8> {
    serde_json::json!({
        "status": status,
        "body": body,
    })
    .to_string()
    .into_bytes()
}

fn validate_http_url(url: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| "Invalid URL".to_string())?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err("Only http and https URLs are allowed".to_string()),
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("Credentials in URL are not allowed".to_string());
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "URL host is required".to_string())?;
    let normalized_host = host.trim_end_matches('.').to_ascii_lowercase();

    if is_blocked_host_name(&normalized_host) {
        return Err("Local or private hosts are not allowed".to_string());
    }

    if let Ok(ip) = normalized_host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err("Local or private IP addresses are not allowed".to_string());
        }
        return Ok(parsed);
    }

    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "URL port is required".to_string())?;
    let resolved = (normalized_host.as_str(), port)
        .to_socket_addrs()
        .map_err(|_| "URL host could not be resolved".to_string())?;

    let mut resolved_any = false;
    for addr in resolved {
        resolved_any = true;
        if is_blocked_ip(addr.ip()) {
            return Err("URL host resolves to a local or private address".to_string());
        }
    }
    if !resolved_any {
        return Err("URL host could not be resolved".to_string());
    }

    Ok(parsed)
}

fn is_blocked_host_name(host: &str) -> bool {
    host == "localhost"
        || host.ends_with(".localhost")
        || host == "metadata.google.internal"
        || host.ends_with(".local")
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_blocked_ipv4(ip),
        IpAddr::V6(ip) => is_blocked_ipv6(ip),
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
}

fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_blocked_ipv4(mapped);
    }

    let segments = ip.segments();
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
}

fn is_allowed_outbound_header(name: &str) -> bool {
    !matches!(
        name.to_ascii_lowercase().as_str(),
        "host"
            | "content-length"
            | "connection"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn execute_wasm(
    wasm_path: &str,
    func_name: &str,
    input: &[u8],
    state: WorkerState,
    cache: &mut ModuleCache,
) -> Result<Vec<u8>, String> {
    if input.len() > MAX_PLUGIN_INPUT_BYTES {
        return Err("Input data exceeds plugin limit".to_string());
    }

    let module = cache.get_or_compile(wasm_path)?;
    let engine = &cache.engine;

    let mut store = wasmtime::Store::new(engine, state);
    store.limiter(|state| &mut state.limits);
    store
        .set_fuel(100_000_000)
        .map_err(|e| format!("Failed to set fuel: {}", e))?;

    let mut linker = wasmtime::Linker::new(engine);

    // ---- host_http_request (network permission) ----
    linker
        .func_wrap(
            "env",
            "host_http_request",
            |mut caller: wasmtime::Caller<'_, WorkerState>,
             method_ptr: i32,
             method_len: i32,
             url_ptr: i32,
             url_len: i32,
             headers_ptr: i32,
             headers_len: i32,
             body_ptr: i32,
             body_len: i32|
             -> i32 {
                if !caller.data().has_network {
                    return 0;
                }

                let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return 0,
                };
                let mem_data = memory.data(&caller);

                let method = read_wasm_string_limited(
                    mem_data,
                    method_ptr,
                    method_len,
                    MAX_HTTP_METHOD_BYTES,
                );
                let url = read_wasm_string_limited(mem_data, url_ptr, url_len, MAX_HTTP_URL_BYTES);
                let headers = read_wasm_string_limited(
                    mem_data,
                    headers_ptr,
                    headers_len,
                    MAX_HTTP_HEADERS_BYTES,
                );
                let body =
                    read_wasm_bytes_limited(mem_data, body_ptr, body_len, MAX_HTTP_BODY_BYTES);

                let (method, url, headers) = match (method, url, headers) {
                    (Some(m), Some(u), Some(h)) => (m, u, h),
                    _ => return 0,
                };
                let body = body.unwrap_or_default();

                let response_bytes = do_http_request(&method, &url, &headers, &body);
                write_to_wasm_memory(&mut caller, &response_bytes)
            },
        )
        .map_err(|e| format!("Failed to register host_http_request: {}", e))?;

    // ---- host_storage_get (storage permission) ----
    linker
        .func_wrap(
            "env",
            "host_storage_get",
            |mut caller: wasmtime::Caller<'_, WorkerState>, key_ptr: i32, key_len: i32| -> i32 {
                if !caller.data().has_storage {
                    return 0;
                }

                let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return 0,
                };
                let mem_data = memory.data(&caller);
                let key = match read_wasm_string_limited(
                    mem_data,
                    key_ptr,
                    key_len,
                    MAX_STORAGE_KEY_BYTES,
                ) {
                    Some(k) => k,
                    None => return 0,
                };

                let value = caller.data().plugin_data.get(&key).cloned();

                match value {
                    Some(val) => {
                        let response = serde_json::json!({"found": true, "value": val});
                        write_to_wasm_memory(&mut caller, response.to_string().as_bytes())
                    }
                    None => {
                        let response = serde_json::json!({"found": false, "value": ""});
                        write_to_wasm_memory(&mut caller, response.to_string().as_bytes())
                    }
                }
            },
        )
        .map_err(|e| format!("Failed to register host_storage_get: {}", e))?;

    // ---- host_storage_set (storage permission) ----
    linker
        .func_wrap(
            "env",
            "host_storage_set",
            |mut caller: wasmtime::Caller<'_, WorkerState>,
             key_ptr: i32,
             key_len: i32,
             value_ptr: i32,
             value_len: i32|
             -> i32 {
                if !caller.data().has_storage {
                    return 0;
                }

                let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return 0,
                };
                let mem_data = memory.data(&caller);
                let key = match read_wasm_string_limited(
                    mem_data,
                    key_ptr,
                    key_len,
                    MAX_STORAGE_KEY_BYTES,
                ) {
                    Some(k) => k,
                    None => return 0,
                };
                let value = match read_wasm_string_limited(
                    mem_data,
                    value_ptr,
                    value_len,
                    MAX_STORAGE_VALUE_BYTES,
                ) {
                    Some(v) => v,
                    None => return 0,
                };

                caller
                    .data_mut()
                    .plugin_data
                    .insert(key.clone(), value.clone());

                if let Ok(mut ops) = caller.data().storage_ops.lock() {
                    if ops.len() >= MAX_STORAGE_OPS {
                        return 0;
                    }
                    ops.push(StorageOp::Set { key, value });
                }
                1
            },
        )
        .map_err(|e| format!("Failed to register host_storage_set: {}", e))?;

    // ---- host_storage_delete (storage permission) ----
    linker
        .func_wrap(
            "env",
            "host_storage_delete",
            |mut caller: wasmtime::Caller<'_, WorkerState>, key_ptr: i32, key_len: i32| -> i32 {
                if !caller.data().has_storage {
                    return 0;
                }

                let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return 0,
                };
                let mem_data = memory.data(&caller);
                let key = match read_wasm_string_limited(
                    mem_data,
                    key_ptr,
                    key_len,
                    MAX_STORAGE_KEY_BYTES,
                ) {
                    Some(k) => k,
                    None => return 0,
                };

                caller.data_mut().plugin_data.remove(&key);

                if let Ok(mut ops) = caller.data().storage_ops.lock() {
                    if ops.len() >= MAX_STORAGE_OPS {
                        return 0;
                    }
                    ops.push(StorageOp::Delete { key });
                }
                1
            },
        )
        .map_err(|e| format!("Failed to register host_storage_delete: {}", e))?;

    // ---- host_log (no permission required) ----
    linker
        .func_wrap(
            "env",
            "host_log",
            |mut caller: wasmtime::Caller<'_, WorkerState>,
             level_ptr: i32,
             level_len: i32,
             msg_ptr: i32,
             msg_len: i32| {
                if caller.data().log_count >= MAX_LOG_OPS {
                    return;
                }
                caller.data_mut().log_count += 1;

                let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return,
                };
                let mem_data = memory.data(&caller);
                let level =
                    read_wasm_string_limited(mem_data, level_ptr, level_len, MAX_HTTP_METHOD_BYTES)
                        .unwrap_or_default();
                let msg = read_wasm_string_limited(mem_data, msg_ptr, msg_len, MAX_LOG_BYTES)
                    .unwrap_or_default();
                let plugin_id = &caller.data().plugin_id;

                eprintln!("[wasm:{}][{}] {}", plugin_id, level, msg);
            },
        )
        .map_err(|e| format!("Failed to register host_log: {}", e))?;

    // ---- host_query_articles (read_articles permission) ----
    linker
        .func_wrap(
            "env",
            "host_query_articles",
            |mut caller: wasmtime::Caller<'_, WorkerState>,
             _filter_ptr: i32,
             _filter_len: i32|
             -> i32 {
                if !caller.data().has_read_articles {
                    return 0;
                }

                let json_bytes = caller.data().articles_json.as_bytes().to_vec();
                write_to_wasm_memory(&mut caller, &json_bytes)
            },
        )
        .map_err(|e| format!("Failed to register host_query_articles: {}", e))?;

    // ---- host_get_article (read_articles permission) ----
    linker
        .func_wrap(
            "env",
            "host_get_article",
            |mut caller: wasmtime::Caller<'_, WorkerState>, article_id: i32| -> i32 {
                if !caller.data().has_read_articles {
                    return 0;
                }

                let articles_str = caller.data().articles_json.clone();
                let articles: Vec<serde_json::Value> =
                    serde_json::from_str(&articles_str).unwrap_or_default();

                let article = articles
                    .into_iter()
                    .find(|a| a.get("id").and_then(|v| v.as_i64()) == Some(article_id as i64));

                match article {
                    Some(a) => {
                        let json = a.to_string();
                        write_to_wasm_memory(&mut caller, json.as_bytes())
                    }
                    None => 0,
                }
            },
        )
        .map_err(|e| format!("Failed to register host_get_article: {}", e))?;

    // ---- host_get_comments (read_comments permission) ----
    linker
        .func_wrap(
            "env",
            "host_get_comments",
            |mut caller: wasmtime::Caller<'_, WorkerState>, article_id: i32| -> i32 {
                if !caller.data().has_read_comments {
                    return 0;
                }

                let comments_str = caller.data().comments_json.clone();
                let comments_map: serde_json::Value =
                    serde_json::from_str(&comments_str).unwrap_or(serde_json::json!({}));

                let key = article_id.to_string();
                let article_comments = comments_map
                    .get(&key)
                    .cloned()
                    .unwrap_or(serde_json::json!([]));

                let json = article_comments.to_string();
                write_to_wasm_memory(&mut caller, json.as_bytes())
            },
        )
        .map_err(|e| format!("Failed to register host_get_comments: {}", e))?;

    // ---- host_update_article_meta (write_articles permission) ----
    linker
        .func_wrap(
            "env",
            "host_update_article_meta",
            |mut caller: wasmtime::Caller<'_, WorkerState>,
             article_id: i32,
             data_ptr: i32,
             data_len: i32|
             -> i32 {
                if !caller.data().has_write_articles {
                    return 0;
                }

                let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return 0,
                };
                let mem_data = memory.data(&caller);
                let data_json = match read_wasm_string_limited(
                    mem_data,
                    data_ptr,
                    data_len,
                    MAX_META_VALUE_BYTES,
                ) {
                    Some(s) => s,
                    None => return 0,
                };

                if serde_json::from_str::<serde_json::Value>(&data_json).is_err() {
                    return 0;
                }

                if let Ok(mut ops) = caller.data().meta_ops.lock() {
                    if ops.len() >= MAX_META_OPS {
                        return 0;
                    }
                    ops.push(MetaOp {
                        article_id: article_id as i64,
                        data: data_json,
                    });
                }
                1
            },
        )
        .map_err(|e| format!("Failed to register host_update_article_meta: {}", e))?;

    // ---- host_hmac_sha256 (no special permission — crypto primitive) ----
    linker
        .func_wrap(
            "env",
            "host_hmac_sha256",
            |mut caller: wasmtime::Caller<'_, WorkerState>,
             key_ptr: i32,
             key_len: i32,
             data_ptr: i32,
             data_len: i32|
             -> i32 {
                let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return 0,
                };
                let mem_data = memory.data(&caller);
                let key = match read_wasm_bytes(mem_data, key_ptr, key_len) {
                    Some(k) => k,
                    None => return 0,
                };
                let data = match read_wasm_bytes(mem_data, data_ptr, data_len) {
                    Some(d) => d,
                    None => return 0,
                };

                type HmacSha256 = Hmac<Sha256>;
                let mut mac = match HmacSha256::new_from_slice(&key) {
                    Ok(m) => m,
                    Err(_) => return 0,
                };
                mac.update(&data);
                let result = mac.finalize().into_bytes();

                // Return hex-encoded result as a string
                let hex = result
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>();
                write_to_wasm_memory(&mut caller, hex.as_bytes())
            },
        )
        .map_err(|e| format!("Failed to register host_hmac_sha256: {}", e))?;

    // ---- host_sha256 (no special permission — crypto primitive) ----
    linker
        .func_wrap(
            "env",
            "host_sha256",
            |mut caller: wasmtime::Caller<'_, WorkerState>, data_ptr: i32, data_len: i32| -> i32 {
                let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return 0,
                };
                let mem_data = memory.data(&caller);
                let data = match read_wasm_bytes(mem_data, data_ptr, data_len) {
                    Some(d) => d,
                    None => return 0,
                };

                let hash = Sha256::digest(&data);
                let hex = hash
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>();
                write_to_wasm_memory(&mut caller, hex.as_bytes())
            },
        )
        .map_err(|e| format!("Failed to register host_sha256: {}", e))?;

    // ---- host_db_query (database permission) ----
    // Plugin calls host_db_query(sql_ptr, sql_len, params_ptr, params_len) -> result_ptr
    // Returns JSON string of rows, or empty on error/no permission.
    linker
        .func_wrap(
            "env",
            "host_db_query",
            |mut caller: wasmtime::Caller<'_, WorkerState>,
             sql_ptr: i32,
             sql_len: i32,
             params_ptr: i32,
             params_len: i32|
             -> i32 {
                if !caller.data().has_database {
                    return 0;
                }

                let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return 0,
                };
                let mem_data = memory.data(&caller);
                let sql = match read_wasm_string_limited(mem_data, sql_ptr, sql_len, MAX_SQL_BYTES)
                {
                    Some(s) => s,
                    None => return 0,
                };
                let params =
                    read_wasm_string_limited(mem_data, params_ptr, params_len, MAX_DB_PARAMS_BYTES)
                        .unwrap_or_else(|| "[]".to_string());

                // Collect the operation — actual execution happens in the bridge
                if let Ok(mut ops) = caller.data().db_ops.lock() {
                    if ops.len() >= MAX_DB_OPS {
                        return 0;
                    }
                    ops.push(DbOp::Query { sql, params });
                }

                // Return a placeholder — the bridge will replace with real results
                // For now, return empty array as the WASM side gets results asynchronously
                let placeholder = b"[]";
                write_to_wasm_memory(&mut caller, placeholder)
            },
        )
        .map_err(|e| format!("Failed to register host_db_query: {}", e))?;

    // ---- host_db_execute (database permission) ----
    // Plugin calls host_db_execute(sql_ptr, sql_len, params_ptr, params_len) -> i32
    // Returns 1 on success (op queued), 0 on no permission.
    linker
        .func_wrap(
            "env",
            "host_db_execute",
            |mut caller: wasmtime::Caller<'_, WorkerState>,
             sql_ptr: i32,
             sql_len: i32,
             params_ptr: i32,
             params_len: i32|
             -> i32 {
                if !caller.data().has_database {
                    return 0;
                }

                let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return 0,
                };
                let mem_data = memory.data(&caller);
                let sql = match read_wasm_string_limited(mem_data, sql_ptr, sql_len, MAX_SQL_BYTES)
                {
                    Some(s) => s,
                    None => return 0,
                };
                let params =
                    read_wasm_string_limited(mem_data, params_ptr, params_len, MAX_DB_PARAMS_BYTES)
                        .unwrap_or_else(|| "[]".to_string());

                if let Ok(mut ops) = caller.data().db_ops.lock() {
                    if ops.len() >= MAX_DB_OPS {
                        return 0;
                    }
                    ops.push(DbOp::Execute { sql, params });
                }
                1
            },
        )
        .map_err(|e| format!("Failed to register host_db_execute: {}", e))?;

    // ---- WASI stubs (for wasm32-wasip1 compiled modules) ----
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_write",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i32, _: i32, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_read",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i32, _: i32, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_close",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_seek",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i64, _: i32, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_fdstat_get",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1",
        "proc_exit",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32| {},
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_sizes_get",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_get",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i32| -> i32 { 0 },
    );

    // Instantiate
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("Instantiation failed: {}", e))?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| "No memory export".to_string())?;

    let func = instance
        .get_func(&mut store, func_name)
        .ok_or_else(|| format!("Function '{}' not found", func_name))?;

    // Data-passing protocol: allocate -> write -> call(ptr, len) -> read result
    let output = if let Some(alloc_func) = instance.get_func(&mut store, "allocate") {
        if let Ok(alloc_typed) = alloc_func.typed::<i32, i32>(&store) {
            let input_len = input.len() as i32;
            let input_ptr = alloc_typed
                .call(&mut store, input_len)
                .map_err(|e| format!("allocate failed: {}", e))?;
            if input_ptr < 0 {
                return Err("WASM allocate returned an invalid pointer".to_string());
            }

            let ptr = input_ptr as usize;
            let mem = memory.data_mut(&mut store);
            let input_end = ptr
                .checked_add(input.len())
                .ok_or_else(|| "Input data exceeds WASM memory".to_string())?;
            if input_end > mem.len() {
                return Err("Input data exceeds WASM memory".to_string());
            }
            mem[ptr..input_end].copy_from_slice(input);

            if let Ok(typed_func) = func.typed::<(i32, i32), i32>(&store) {
                let result_ptr = typed_func
                    .call(&mut store, (input_ptr, input_len))
                    .map_err(|e| format!("Function call failed: {}", e))?;

                if result_ptr > 0 {
                    let mem_data = memory.data(&store);
                    let rp = result_ptr as usize;
                    if let Some(header_end) = rp.checked_add(4).filter(|end| *end <= mem_data.len())
                    {
                        let result_len = u32::from_le_bytes([
                            mem_data[rp],
                            mem_data[rp + 1],
                            mem_data[rp + 2],
                            mem_data[rp + 3],
                        ]) as usize;
                        if result_len <= MAX_HOST_RETURN_BYTES {
                            if let Some(result_end) = header_end
                                .checked_add(result_len)
                                .filter(|end| *end <= mem_data.len())
                            {
                                mem_data[header_end..result_end].to_vec()
                            } else {
                                vec![]
                            }
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            } else {
                if let Ok(typed_func) = func.typed::<(), i32>(&store) {
                    let _ = typed_func.call(&mut store, ());
                }
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        if let Ok(typed_func) = func.typed::<(), i32>(&store) {
            let _ = typed_func.call(&mut store, ());
        }
        vec![]
    };

    Ok(output)
}

/// Write data to WASM memory via the allocate function.
/// Returns the pointer (with 4-byte length prefix) or 0 on failure.
fn write_to_wasm_memory(caller: &mut wasmtime::Caller<'_, WorkerState>, data: &[u8]) -> i32 {
    if data.len() > MAX_HOST_RETURN_BYTES {
        return 0;
    }

    let alloc_func = match caller.get_export("allocate") {
        Some(wasmtime::Extern::Func(f)) => f,
        _ => return 0,
    };
    let memory = match caller.get_export("memory") {
        Some(wasmtime::Extern::Memory(mem)) => mem,
        _ => return 0,
    };

    let alloc_typed = match alloc_func.typed::<i32, i32>(caller.as_context_mut()) {
        Ok(f) => f,
        Err(_) => return 0,
    };

    let total_len = match 4usize.checked_add(data.len()) {
        Some(total_len) => total_len,
        None => return 0,
    };
    let alloc_len = match i32::try_from(total_len) {
        Ok(len) => len,
        Err(_) => return 0,
    };
    let ptr = match alloc_typed.call(caller.as_context_mut(), alloc_len) {
        Ok(p) if p >= 0 => p as usize,
        Err(_) => return 0,
        _ => return 0,
    };

    let mem_data = memory.data_mut(caller.as_context_mut());
    let end = match ptr.checked_add(total_len) {
        Some(end) if end <= mem_data.len() => end,
        _ => return 0,
    };

    let len_bytes = (data.len() as u32).to_le_bytes();
    mem_data[ptr..ptr + 4].copy_from_slice(&len_bytes);
    mem_data[ptr + 4..end].copy_from_slice(data);

    ptr as i32
}

fn read_wasm_string_limited(mem: &[u8], ptr: i32, len: i32, max_len: usize) -> Option<String> {
    let bytes = read_wasm_range(mem, ptr, len, max_len)?;
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

fn read_wasm_bytes(mem: &[u8], ptr: i32, len: i32) -> Option<Vec<u8>> {
    read_wasm_bytes_limited(mem, ptr, len, MAX_HOST_ARG_BYTES)
}

fn read_wasm_bytes_limited(mem: &[u8], ptr: i32, len: i32, max_len: usize) -> Option<Vec<u8>> {
    read_wasm_range(mem, ptr, len, max_len).map(|bytes| bytes.to_vec())
}

fn read_wasm_range(mem: &[u8], ptr: i32, len: i32, max_len: usize) -> Option<&[u8]> {
    if ptr < 0 || len < 0 {
        return None;
    }
    let start = ptr as usize;
    let length = len as usize;
    if length > max_len {
        return None;
    }
    let end = start.checked_add(length)?;
    if end > mem.len() {
        return None;
    }
    Some(&mem[start..end])
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim_end_matches('=');
    let mut result = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for c in input.bytes() {
        let val = decode_b64_char(c);
        if val == u32::MAX {
            return Err(format!("Invalid base64 char: {}", c as char));
        }
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            result.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(result)
}

fn decode_b64_char(c: u8) -> u32 {
    match c {
        b'A'..=b'Z' => (c - b'A') as u32,
        b'a'..=b'z' => (c - b'a' + 26) as u32,
        b'0'..=b'9' => (c - b'0' + 52) as u32,
        b'+' => 62,
        b'/' => 63,
        _ => u32::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_public_literal_http_urls() {
        assert!(validate_http_url("https://8.8.8.8/v1").is_ok());
        assert!(validate_http_url("http://1.1.1.1/json").is_ok());
    }

    #[test]
    fn rejects_local_and_private_http_urls() {
        assert!(validate_http_url("http://localhost:8080").is_err());
        assert!(validate_http_url("http://127.0.0.1:8080").is_err());
        assert!(validate_http_url("http://10.0.0.5").is_err());
        assert!(validate_http_url("http://169.254.169.254/latest/meta-data").is_err());
        assert!(validate_http_url("http://[::1]/").is_err());
    }

    #[test]
    fn rejects_non_http_urls_and_url_credentials() {
        assert!(validate_http_url("file:///etc/passwd").is_err());
        assert!(validate_http_url("https://user:pass@8.8.8.8/").is_err());
    }

    #[test]
    fn reads_wasm_memory_with_bounds() {
        let mem = b"abcdef";
        assert_eq!(
            read_wasm_string_limited(mem, 1, 3, 8).as_deref(),
            Some("bcd")
        );
        assert!(read_wasm_bytes_limited(mem, -1, 1, 8).is_none());
        assert!(read_wasm_bytes_limited(mem, 1, 6, 8).is_none());
        assert!(read_wasm_bytes_limited(mem, 1, 3, 2).is_none());
    }

    #[test]
    fn filters_unsafe_outbound_headers() {
        assert!(is_allowed_outbound_header("authorization"));
        assert!(!is_allowed_outbound_header("host"));
        assert!(!is_allowed_outbound_header("transfer-encoding"));
    }
}
