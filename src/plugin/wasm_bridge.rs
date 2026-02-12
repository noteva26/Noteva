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
use super::loader::{Plugin, PluginManager};

/// Mapping from plugin_id to its WASM handle
#[derive(Debug, Default)]
pub struct WasmPluginRegistry {
    /// Map of plugin_id -> (PluginHandle, list of registered hook names)
    entries: std::collections::HashMap<String, WasmPluginEntry>,
}

#[derive(Debug)]
struct WasmPluginEntry {
    handle: PluginHandle,
    _hooks: Vec<String>,
}

impl WasmPluginRegistry {
    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    /// Check if a plugin has a loaded WASM module
    pub fn has_plugin(&self, plugin_id: &str) -> bool {
        self.entries.contains_key(plugin_id)
    }

    /// Get the handle for a plugin
    pub fn get_handle(&self, plugin_id: &str) -> Option<PluginHandle> {
        self.entries.get(plugin_id).map(|e| e.handle)
    }
}

/// Find the wasm-worker executable path.
/// Looks next to the current executable first, then falls back to PATH.
fn find_worker_exe() -> String {
    if let Ok(current_exe) = std::env::current_exe() {
        let dir = current_exe.parent().unwrap_or(std::path::Path::new("."));
        let worker_name = if cfg!(windows) { "wasm-worker.exe" } else { "wasm-worker" };
        let worker_path = dir.join(worker_name);
        if worker_path.exists() {
            return worker_path.to_string_lossy().to_string();
        }
    }
    // Fallback: assume it's in PATH
    if cfg!(windows) { "wasm-worker.exe".to_string() } else { "wasm-worker".to_string() }
}

/// Execute a WASM hook function via subprocess isolation.
///
/// This spawns the `wasm-worker` process, passes the WASM file path,
/// function name, and input data via stdin (JSON), and reads the result
/// from stdout. If the subprocess crashes, the main process is unaffected.
fn execute_wasm_subprocess(
    wasm_path: &str,
    func_name: &str,
    input_bytes: &[u8],
) -> Option<Vec<u8>> {
    use std::process::{Command, Stdio};
    use std::io::Write;

    let worker_exe = find_worker_exe();
    let input_b64 = base64_encode(input_bytes);

    let request = serde_json::json!({
        "wasm_path": wasm_path,
        "func_name": func_name,
        "input": input_b64,
    });
    let request_str = request.to_string();

    // Spawn subprocess with timeout
    let mut child = match Command::new(&worker_exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to spawn wasm-worker: {}", e);
            return None;
        }
    };

    // Write request to stdin
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(request_str.as_bytes()) {
            tracing::error!("Failed to write to wasm-worker stdin: {}", e);
            let _ = child.kill();
            return None;
        }
        // stdin is dropped here, closing the pipe
    }

    // Wait for result with timeout (5 seconds)
    let result = match wait_with_timeout(&mut child, std::time::Duration::from_secs(5)) {
        Ok(output) => output,
        Err(e) => {
            tracing::warn!("wasm-worker timed out or failed: {}", e);
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };

    // Parse response
    let stdout_str = String::from_utf8_lossy(&result.stdout);
    let response: Value = match serde_json::from_str(&stdout_str) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("wasm-worker returned invalid JSON: {} (output: {})", e, stdout_str);
            return None;
        }
    };

    if response.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let err = response.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
        tracing::warn!("wasm-worker execution failed: {}", err);
        return None;
    }

    // Decode output
    let output_b64 = response.get("output").and_then(|v| v.as_str()).unwrap_or("");
    match base64_decode(output_b64) {
        Ok(bytes) => {
            if bytes.is_empty() { None } else { Some(bytes) }
        }
        Err(e) => {
            tracing::warn!("Failed to decode wasm-worker output: {}", e);
            None
        }
    }
}

/// Wait for a child process with a timeout.
fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: std::time::Duration,
) -> Result<std::process::Output, String> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process exited
                let mut stdout = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    use std::io::Read;
                    let _ = out.read_to_end(&mut stdout);
                }
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr: Vec::new(),
                });
            }
            Ok(None) => {
                // Still running
                if start.elapsed() > timeout {
                    return Err("Timeout".to_string());
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => {
                return Err(format!("Wait error: {}", e));
            }
        }
    }
}

/// Load a WASM module for a plugin and register its backend hooks.
///
/// This is called when a plugin with a `backend.wasm` file is enabled.
/// Hook execution is isolated via subprocess — WASM crashes cannot affect the server.
pub async fn load_wasm_plugin(
    plugin: &Plugin,
    runtime: &Arc<RwLock<PluginRuntime>>,
    hook_manager: &Arc<HookManager>,
    registry: &Arc<RwLock<WasmPluginRegistry>>,
) -> Result<(), PluginError> {
    let wasm_path = plugin.path.join("backend.wasm");
    if !wasm_path.exists() {
        return Ok(());
    }

    let plugin_id = plugin.metadata.id.clone();
    let backend_hooks = plugin.metadata.hooks.backend.clone();

    if backend_hooks.is_empty() {
        warn!(
            "Plugin '{}' has backend.wasm but no hooks.backend declared in plugin.json, skipping",
            plugin_id
        );
        return Ok(());
    }

    info!("Loading WASM module for plugin '{}'...", plugin_id);

    // Read the WASM bytes
    let wasm_bytes = tokio::fs::read(&wasm_path).await.map_err(|e| {
        PluginError::WasmError(format!("Failed to read {}: {}", wasm_path.display(), e))
    })?;

    // Build manifest from plugin metadata
    let manifest = PluginManifest {
        name: plugin.metadata.name.clone(),
        version: plugin.metadata.version.clone(),
        description: Some(plugin.metadata.description.clone()),
        author: Some(plugin.metadata.author.clone()),
        permissions: plugin
            .metadata
            .permissions
            .iter()
            .filter_map(|p| Permission::from_str(p))
            .collect(),
    };

    // Load into runtime (for metadata tracking / status API)
    let handle = {
        let mut rt = runtime.write().await;
        rt.load_plugin_bytes(&wasm_bytes, manifest)?
    };

    info!(
        "WASM module loaded for plugin '{}' (handle: {:?}), registering {} backend hooks",
        plugin_id, handle, backend_hooks.len()
    );

    // Get the absolute path to the WASM file for subprocess use
    let wasm_abs_path = std::fs::canonicalize(&wasm_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| wasm_path.to_string_lossy().to_string());

    // Register each declared backend hook — execution goes through subprocess
    for hook_name in &backend_hooks {
        let wasm_file = wasm_abs_path.clone();
        let hook_fn_name = format!("hook_{}", hook_name);
        let hook_fn_name_log = hook_fn_name.clone();
        let pid = plugin_id.clone();

        hook_manager.register(
            hook_name,
            move |data: &mut Value| {
                let input_bytes = serde_json::to_vec(data).unwrap_or_default();
                let wasm_file = wasm_file.clone();
                let hook_fn = hook_fn_name.clone();
                let _plugin_id = pid.clone();

                // Execute WASM in a subprocess — completely isolated from main process.
                // If the WASM crashes, only the subprocess dies.
                match execute_wasm_subprocess(&wasm_file, &hook_fn, &input_bytes) {
                    Some(output_bytes) => {
                        match serde_json::from_slice::<Value>(&output_bytes) {
                            Ok(modified) => Some(modified),
                            Err(_) => None,
                        }
                    }
                    None => None,
                }
            },
            PRIORITY_DEFAULT,
            Some(plugin_id.clone()),
        );

        info!("  Registered WASM hook: {} -> {}", hook_name, hook_fn_name_log);
    }

    // Record in registry
    {
        let mut reg = registry.write().await;
        reg.entries.insert(
            plugin_id.clone(),
            WasmPluginEntry {
                handle,
                _hooks: backend_hooks,
            },
        );
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
            warn!("Failed to unload WASM module for plugin '{}': {}", plugin_id, e);
        }

        info!("WASM plugin '{}' unloaded", plugin_id);
    }

    Ok(())
}

/// Scan all enabled plugins and load any that have backend.wasm files.
/// Called during startup.
pub async fn load_all_wasm_plugins(
    plugin_manager: &PluginManager,
    runtime: &Arc<RwLock<PluginRuntime>>,
    hook_manager: &Arc<HookManager>,
    registry: &Arc<RwLock<WasmPluginRegistry>>,
) {
    let enabled_plugins: Vec<Plugin> = plugin_manager
        .get_enabled()
        .into_iter()
        .cloned()
        .collect();

    for plugin in &enabled_plugins {
        let wasm_path = plugin.path.join("backend.wasm");
        if wasm_path.exists() {
            if let Err(e) = load_wasm_plugin(plugin, runtime, hook_manager, registry).await {
                error!(
                    "Failed to load WASM plugin '{}': {}",
                    plugin.metadata.id, e
                );
            }
        }
    }
}

// Simple base64 encode/decode — avoids adding an external dependency

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
    if input.is_empty() {
        return Ok(vec![]);
    }
    let mut result = Vec::new();
    let chars: Vec<u8> = input.bytes().filter(|&b| b != b'\n' && b != b'\r').collect();
    if chars.len() % 4 != 0 {
        return Err("Invalid base64 length".to_string());
    }
    for chunk in chars.chunks(4) {
        let vals: Vec<u32> = chunk.iter().map(|&c| decode_b64_char(c)).collect();
        if vals.iter().any(|&v| v == 255) {
            return Err("Invalid base64 character".to_string());
        }
        let triple = (vals[0] << 18) | (vals[1] << 12) | (vals[2] << 6) | vals[3];
        result.push(((triple >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' {
            result.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk[3] != b'=' {
            result.push((triple & 0xFF) as u8);
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
        b'=' => 0,
        _ => 255,
    }
}
