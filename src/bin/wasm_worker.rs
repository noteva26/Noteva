//! WASM Worker Process
//!
//! Isolated subprocess for executing WASM plugin hooks.
//! This process is spawned by the main Noteva server to execute WASM code safely.
//! If the WASM code crashes (trap, OOM, etc.), only this subprocess dies —
//! the main server process is completely unaffected.
//!
//! Protocol (via stdin/stdout):
//!   Input (JSON):
//!     {
//!       "wasm_path": "...",
//!       "func_name": "...",
//!       "input": "base64...",
//!       "permissions": ["network", "storage"],
//!       "plugin_id": "ai-summary",
//!       "plugin_data": { "key1": "value1", ... }
//!     }
//!
//!   Output (JSON):
//!     { "success": true, "output": "base64...", "storage_ops": [...] }
//!     or: { "success": false, "error": "..." }
//!
//! Host functions provided to WASM plugins:
//!   - host_http_request(...)  -> result_ptr   (requires "network" permission)
//!   - host_storage_get(...)   -> result_ptr   (requires "storage" permission)
//!   - host_storage_set(...)   -> i32          (requires "storage" permission)
//!   - host_storage_delete(...) -> i32         (requires "storage" permission)
//!   - host_log(...)           -> ()           (no permission required)

use std::io::{self, Read};
use std::sync::{Arc, Mutex};
use wasmtime::AsContextMut;

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

/// Shared state between host functions and the main execution.
struct WorkerState {
    has_network: bool,
    has_storage: bool,
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
}


fn main() {
    let mut input_str = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input_str) {
        print_error(&format!("Failed to read stdin: {}", e));
        return;
    }

    let request: serde_json::Value = match serde_json::from_str(&input_str) {
        Ok(v) => v,
        Err(e) => {
            print_error(&format!("Invalid JSON input: {}", e));
            return;
        }
    };

    let wasm_path = match request.get("wasm_path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => { print_error("Missing 'wasm_path' field"); return; }
    };
    let func_name = match request.get("func_name").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => { print_error("Missing 'func_name' field"); return; }
    };
    let input_b64 = request.get("input").and_then(|v| v.as_str()).unwrap_or("");
    let input_bytes = match base64_decode(input_b64) {
        Ok(b) => b,
        Err(e) => { print_error(&format!("Failed to decode input: {}", e)); return; }
    };

    // Parse permissions
    let permissions: Vec<String> = request
        .get("permissions")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let has_network = permissions.iter().any(|p| p == "network");
    let has_storage = permissions.iter().any(|p| p == "storage");
    let has_read_articles = permissions.iter().any(|p| p == "read_articles" || p == "read-articles");
    let has_read_comments = permissions.iter().any(|p| p == "read_comments" || p == "read-comments");
    let has_write_articles = permissions.iter().any(|p| p == "write_articles" || p == "write-articles");

    // Parse plugin_id
    let plugin_id = request.get("plugin_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Parse pre-loaded plugin data
    let plugin_data: std::collections::HashMap<String, String> = request
        .get("plugin_data")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    // Parse pre-loaded articles data
    let articles_json = request.get("articles")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "[]".to_string());

    // Parse pre-loaded comments data
    let comments_json = request.get("comments")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let storage_ops = Arc::new(Mutex::new(Vec::new()));
    let meta_ops = Arc::new(Mutex::new(Vec::new()));

    let state = WorkerState {
        has_network,
        has_storage,
        has_read_articles,
        has_read_comments,
        has_write_articles,
        plugin_id,
        plugin_data,
        articles_json,
        comments_json,
        storage_ops: storage_ops.clone(),
        meta_ops: meta_ops.clone(),
    };

    match execute_wasm(wasm_path, func_name, &input_bytes, state) {
        Ok(output) => {
            let output_b64 = base64_encode(&output);
            // Collect storage ops for main process
            let ops: Vec<serde_json::Value> = storage_ops.lock().unwrap().iter().map(|op| {
                match op {
                    StorageOp::Set { key, value } => serde_json::json!({
                        "op": "set", "key": key, "value": value
                    }),
                    StorageOp::Delete { key } => serde_json::json!({
                        "op": "delete", "key": key
                    }),
                }
            }).collect();

            // Collect meta ops for main process
            let mops: Vec<serde_json::Value> = meta_ops.lock().unwrap().iter().map(|op| {
                serde_json::json!({
                    "article_id": op.article_id,
                    "data": op.data,
                })
            }).collect();

            let result = serde_json::json!({
                "success": true,
                "output": output_b64,
                "storage_ops": ops,
                "meta_ops": mops,
            });
            println!("{}", result);
        }
        Err(e) => {
            print_error(&format!("WASM execution failed: {}", e));
        }
    }
}

fn print_error(msg: &str) {
    let result = serde_json::json!({ "success": false, "error": msg });
    println!("{}", result);
}

/// Perform a blocking HTTP request.
fn do_http_request(method: &str, url: &str, headers_json: &str, body: &[u8]) -> Vec<u8> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new());

    let mut req = match method.to_uppercase().as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "PATCH" => client.patch(url),
        _ => {
            let err = serde_json::json!({"status": 0, "body": format!("Unsupported method: {}", method)});
            return err.to_string().into_bytes();
        }
    };

    if !headers_json.is_empty() {
        if let Ok(headers) = serde_json::from_str::<serde_json::Value>(headers_json) {
            if let Some(obj) = headers.as_object() {
                for (key, value) in obj {
                    if let Some(val_str) = value.as_str() {
                        req = req.header(key.as_str(), val_str);
                    }
                }
            }
        }
    }

    if !body.is_empty() && method.to_uppercase() != "GET" {
        req = req.body(body.to_vec());
    }

    match req.send() {
        Ok(resp) => {
            let status = resp.status().as_u16();
            match resp.bytes() {
                Ok(bytes) => {
                    let body_str = if bytes.len() > 1_048_576 {
                        String::from_utf8_lossy(&bytes[..1_048_576]).to_string()
                    } else {
                        String::from_utf8_lossy(&bytes).to_string()
                    };
                    serde_json::json!({"status": status, "body": body_str}).to_string().into_bytes()
                }
                Err(e) => serde_json::json!({"status": status, "body": format!("Failed to read body: {}", e)}).to_string().into_bytes(),
            }
        }
        Err(e) => serde_json::json!({"status": 0, "body": format!("Request failed: {}", e)}).to_string().into_bytes(),
    }
}

fn execute_wasm(wasm_path: &str, func_name: &str, input: &[u8], state: WorkerState) -> Result<Vec<u8>, String> {
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);

    let engine = wasmtime::Engine::new(&config)
        .map_err(|e| format!("Engine creation failed: {}", e))?;

    let wasm_bytes = std::fs::read(wasm_path)
        .map_err(|e| format!("Failed to read WASM file: {}", e))?;

    let module = wasmtime::Module::new(&engine, &wasm_bytes)
        .map_err(|e| format!("Failed to compile WASM: {}", e))?;

    let mut store = wasmtime::Store::new(&engine, state);
    store.set_fuel(100_000_000).map_err(|e| format!("Failed to set fuel: {}", e))?;

    let mut linker = wasmtime::Linker::new(&engine);

    // ---- host_http_request (network permission) ----
    linker.func_wrap(
        "env", "host_http_request",
        |mut caller: wasmtime::Caller<'_, WorkerState>,
         method_ptr: i32, method_len: i32,
         url_ptr: i32, url_len: i32,
         headers_ptr: i32, headers_len: i32,
         body_ptr: i32, body_len: i32| -> i32 {

            if !caller.data().has_network { return 0; }

                let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return 0,
                };
                let mem_data = memory.data(&caller);

                let method = read_wasm_string(mem_data, method_ptr, method_len);
                let url = read_wasm_string(mem_data, url_ptr, url_len);
                let headers = read_wasm_string(mem_data, headers_ptr, headers_len);
                let body = read_wasm_bytes(mem_data, body_ptr, body_len);

                let (method, url, headers) = match (method, url, headers) {
                    (Some(m), Some(u), Some(h)) => (m, u, h),
                    _ => return 0,
                };
                let body = body.unwrap_or_default();

                let response_bytes = do_http_request(&method, &url, &headers, &body);
                write_to_wasm_memory(&mut caller, &response_bytes)
            },
        ).map_err(|e| format!("Failed to register host_http_request: {}", e))?;

    // ---- host_storage_get (storage permission) ----
    linker.func_wrap(
        "env", "host_storage_get",
        |mut caller: wasmtime::Caller<'_, WorkerState>,
         key_ptr: i32, key_len: i32| -> i32 {

            if !caller.data().has_storage { return 0; }

            let memory = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(mem)) => mem,
                    _ => return 0,
                };
                let mem_data = memory.data(&caller);
                let key = match read_wasm_string(mem_data, key_ptr, key_len) {
                    Some(k) => k,
                    None => return 0,
                };

                // Look up in pre-loaded plugin data
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
        ).map_err(|e| format!("Failed to register host_storage_get: {}", e))?;

    // ---- host_storage_set (storage permission) ----
    linker.func_wrap(
        "env", "host_storage_set",
        |mut caller: wasmtime::Caller<'_, WorkerState>,
         key_ptr: i32, key_len: i32,
         value_ptr: i32, value_len: i32| -> i32 {

            if !caller.data().has_storage { return 0; }

            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return 0,
            };
            let mem_data = memory.data(&caller);
            let key = match read_wasm_string(mem_data, key_ptr, key_len) {
                Some(k) => k,
                None => return 0,
            };
            let value = match read_wasm_string(mem_data, value_ptr, value_len) {
                Some(v) => v,
                None => return 0,
                };

                // Update local cache so subsequent gets see the new value
                caller.data_mut().plugin_data.insert(key.clone(), value.clone());

                // Record operation for main process to execute
                if let Ok(mut ops) = caller.data().storage_ops.lock() {
                    ops.push(StorageOp::Set { key, value });
                }
                1 // success
            },
        ).map_err(|e| format!("Failed to register host_storage_set: {}", e))?;

    // ---- host_storage_delete (storage permission) ----
    linker.func_wrap(
        "env", "host_storage_delete",
        |mut caller: wasmtime::Caller<'_, WorkerState>,
         key_ptr: i32, key_len: i32| -> i32 {

            if !caller.data().has_storage { return 0; }

            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return 0,
            };
            let mem_data = memory.data(&caller);
            let key = match read_wasm_string(mem_data, key_ptr, key_len) {
                Some(k) => k,
                None => return 0,
            };

            caller.data_mut().plugin_data.remove(&key);

            if let Ok(mut ops) = caller.data().storage_ops.lock() {
                ops.push(StorageOp::Delete { key });
            }
            1 // success
        },
    ).map_err(|e| format!("Failed to register host_storage_delete: {}", e))?;

    // ---- host_log (no permission required) ----
    linker.func_wrap(
        "env", "host_log",
        |mut caller: wasmtime::Caller<'_, WorkerState>,
         level_ptr: i32, level_len: i32,
         msg_ptr: i32, msg_len: i32| {

            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return,
            };
            let mem_data = memory.data(&caller);
            let level = read_wasm_string(mem_data, level_ptr, level_len).unwrap_or_default();
            let msg = read_wasm_string(mem_data, msg_ptr, msg_len).unwrap_or_default();
            let plugin_id = &caller.data().plugin_id;

            eprintln!("[wasm:{}][{}] {}", plugin_id, level, msg);
        },
    ).map_err(|e| format!("Failed to register host_log: {}", e))?;

    // ---- host_query_articles (read_articles permission) ----
    linker.func_wrap(
        "env", "host_query_articles",
        |mut caller: wasmtime::Caller<'_, WorkerState>,
         _filter_ptr: i32, _filter_len: i32| -> i32 {

            if !caller.data().has_read_articles { return 0; }

            // Return pre-loaded articles JSON
            let json_bytes = caller.data().articles_json.as_bytes().to_vec();
            write_to_wasm_memory(&mut caller, &json_bytes)
        },
    ).map_err(|e| format!("Failed to register host_query_articles: {}", e))?;

    // ---- host_get_article (read_articles permission) ----
    // Returns a single article by ID from the pre-loaded articles data.
    // Input: article ID as i32. Returns JSON string pointer or 0 if not found.
    linker.func_wrap(
        "env", "host_get_article",
        |mut caller: wasmtime::Caller<'_, WorkerState>,
         article_id: i32| -> i32 {

            if !caller.data().has_read_articles { return 0; }

            let articles_str = caller.data().articles_json.clone();
            let articles: Vec<serde_json::Value> = serde_json::from_str(&articles_str).unwrap_or_default();

            let article = articles.into_iter().find(|a| {
                a.get("id").and_then(|v| v.as_i64()) == Some(article_id as i64)
            });

            match article {
                Some(a) => {
                    let json = a.to_string();
                    write_to_wasm_memory(&mut caller, json.as_bytes())
                }
                None => 0,
            }
        },
    ).map_err(|e| format!("Failed to register host_get_article: {}", e))?;

    // ---- host_get_comments (read_comments permission) ----
    // Returns comments for a given article ID from pre-loaded comments data.
    // Input: article ID as i32. Returns JSON array string pointer or 0.
    linker.func_wrap(
        "env", "host_get_comments",
        |mut caller: wasmtime::Caller<'_, WorkerState>,
         article_id: i32| -> i32 {

            if !caller.data().has_read_comments { return 0; }

            let comments_str = caller.data().comments_json.clone();
            let comments_map: serde_json::Value = serde_json::from_str(&comments_str).unwrap_or(serde_json::json!({}));

            let key = article_id.to_string();
            let article_comments = comments_map.get(&key)
                .cloned()
                .unwrap_or(serde_json::json!([]));

            let json = article_comments.to_string();
            write_to_wasm_memory(&mut caller, json.as_bytes())
        },
    ).map_err(|e| format!("Failed to register host_get_comments: {}", e))?;

    // ---- host_update_article_meta (write_articles permission) ----
    // Updates the meta JSON for a given article. Input: article_id (i32),
    // data_ptr/data_len pointing to a JSON string with the plugin's metadata.
    // The main process will merge this under the plugin_id namespace.
    linker.func_wrap(
        "env", "host_update_article_meta",
        |mut caller: wasmtime::Caller<'_, WorkerState>,
         article_id: i32,
         data_ptr: i32, data_len: i32| -> i32 {

            if !caller.data().has_write_articles { return 0; }

            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(mem)) => mem,
                _ => return 0,
            };
            let mem_data = memory.data(&caller);
            let data_json = match read_wasm_string(mem_data, data_ptr, data_len) {
                Some(s) => s,
                None => return 0,
            };

            // Validate it's valid JSON
            if serde_json::from_str::<serde_json::Value>(&data_json).is_err() {
                return 0;
            }

            if let Ok(mut ops) = caller.data().meta_ops.lock() {
                ops.push(MetaOp {
                    article_id: article_id as i64,
                    data: data_json,
                });
            }
            1 // success
        },
    ).map_err(|e| format!("Failed to register host_update_article_meta: {}", e))?;

    // ---- WASI stubs (for wasm32-wasip1 compiled modules) ----
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1", "fd_write",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i32, _: i32, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1", "fd_read",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i32, _: i32, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1", "fd_close",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1", "fd_seek",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i64, _: i32, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1", "fd_fdstat_get",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1", "proc_exit",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32| {},
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1", "environ_sizes_get",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i32| -> i32 { 0 },
    );
    let _ = linker.func_wrap(
        "wasi_snapshot_preview1", "environ_get",
        |_: wasmtime::Caller<'_, WorkerState>, _: i32, _: i32| -> i32 { 0 },
    );

    // Instantiate
    let instance = linker.instantiate(&mut store, &module)
        .map_err(|e| format!("Instantiation failed: {}", e))?;

    let memory = instance.get_memory(&mut store, "memory")
        .ok_or_else(|| "No memory export".to_string())?;

    let func = instance.get_func(&mut store, func_name)
        .ok_or_else(|| format!("Function '{}' not found", func_name))?;

    // Data-passing protocol: allocate -> write -> call(ptr, len) -> read result
    let output = if let Some(alloc_func) = instance.get_func(&mut store, "allocate") {
        if let Ok(alloc_typed) = alloc_func.typed::<i32, i32>(&store) {
            let input_len = input.len() as i32;
            let input_ptr = alloc_typed.call(&mut store, input_len)
                .map_err(|e| format!("allocate failed: {}", e))?;

            let ptr = input_ptr as usize;
            let mem = memory.data_mut(&mut store);
            if ptr + input.len() > mem.len() {
                return Err("Input data exceeds WASM memory".to_string());
            }
            mem[ptr..ptr + input.len()].copy_from_slice(input);

            if let Ok(typed_func) = func.typed::<(i32, i32), i32>(&store) {
                let result_ptr = typed_func.call(&mut store, (input_ptr, input_len))
                    .map_err(|e| format!("Function call failed: {}", e))?;

                if result_ptr > 0 {
                    let mem_data = memory.data(&store);
                    let rp = result_ptr as usize;
                    if rp + 4 <= mem_data.len() {
                        let result_len = u32::from_le_bytes([
                            mem_data[rp], mem_data[rp + 1], mem_data[rp + 2], mem_data[rp + 3],
                        ]) as usize;
                        if rp + 4 + result_len <= mem_data.len() {
                            mem_data[rp + 4..rp + 4 + result_len].to_vec()
                        } else { vec![] }
                    } else { vec![] }
                } else { vec![] }
            } else {
                if let Ok(typed_func) = func.typed::<(), i32>(&store) {
                    let _ = typed_func.call(&mut store, ());
                }
                vec![]
            }
        } else { vec![] }
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
    let alloc_func = match caller.get_export("allocate") {
        Some(wasmtime::Extern::Func(f)) => f,
        _ => return 0,
    };
    let memory = match caller.get_export("memory") {
        Some(wasmtime::Extern::Memory(mem)) => mem,
        _ => return 0,
    };

    let total_len = 4 + data.len();
    let alloc_typed = match alloc_func.typed::<i32, i32>(caller.as_context_mut()) {
        Ok(f) => f,
        Err(_) => return 0,
    };
    let result_ptr = match alloc_typed.call(&mut *caller, total_len as i32) {
        Ok(ptr) => ptr,
        Err(_) => return 0,
    };
    if result_ptr <= 0 { return 0; }

    let rp = result_ptr as usize;
    let mem_data = memory.data_mut(&mut *caller);
    if rp + total_len > mem_data.len() { return 0; }

    let len_bytes = (data.len() as u32).to_le_bytes();
    mem_data[rp..rp + 4].copy_from_slice(&len_bytes);
    mem_data[rp + 4..rp + 4 + data.len()].copy_from_slice(data);

    result_ptr
}

fn read_wasm_string(mem: &[u8], ptr: i32, len: i32) -> Option<String> {
    if len <= 0 { return Some(String::new()); }
    let p = ptr as usize;
    let l = len as usize;
    if p + l > mem.len() { return None; }
    String::from_utf8(mem[p..p + l].to_vec()).ok()
}

fn read_wasm_bytes(mem: &[u8], ptr: i32, len: i32) -> Option<Vec<u8>> {
    if len <= 0 { return Some(vec![]); }
    let p = ptr as usize;
    let l = len as usize;
    if p + l > mem.len() { return None; }
    Some(mem[p..p + l].to_vec())
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
