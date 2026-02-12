//! Plugin system
//!
//! This module provides the plugin ecosystem for Noteva:
//! - Plugin loading and management
//! - Hook system for plugin integration
//! - Shortcode parsing and rendering
//! - WASM-based plugin execution (optional)

pub mod loader;
pub mod hooks;
pub mod shortcode;
pub mod wasm_bridge;

// Re-export commonly used types
pub use loader::{Plugin, PluginManager, PluginMetadata, PluginRequirements, PluginHooks, 
                 check_version_requirement, VersionCheckResult, NOTEVA_VERSION};
pub use hooks::{HookManager, hook_names};
pub use shortcode::{ShortcodeManager, Shortcode, ShortcodeContext};

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use wasmtime::*;

/// Default memory limit for plugins (16 MB)
pub const DEFAULT_MEMORY_LIMIT_BYTES: u64 = 16 * 1024 * 1024;

/// Default fuel limit (instruction count proxy)
pub const DEFAULT_FUEL_LIMIT: u64 = 1_000_000;

/// Default execution timeout
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Available permissions that plugins can request
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Read access to articles
    ReadArticles,
    /// Write access to articles
    WriteArticles,
    /// Read access to comments
    ReadComments,
    /// Write access to comments
    WriteComments,
    /// Read access to configuration
    ReadConfig,
    /// Write access to configuration
    WriteConfig,
    /// Network access (HTTP requests)
    Network,
    /// File system read access
    FileSystemRead,
    /// File system write access
    FileSystemWrite,
}

impl Permission {
    /// Parse permission from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "read_articles" | "read-articles" => Some(Permission::ReadArticles),
            "write_articles" | "write-articles" => Some(Permission::WriteArticles),
            "read_comments" | "read-comments" => Some(Permission::ReadComments),
            "write_comments" | "write-comments" => Some(Permission::WriteComments),
            "read_config" | "read-config" => Some(Permission::ReadConfig),
            "write_config" | "write-config" => Some(Permission::WriteConfig),
            "network" => Some(Permission::Network),
            "fs_read" | "fs-read" | "filesystem_read" => Some(Permission::FileSystemRead),
            "fs_write" | "fs-write" | "filesystem_write" => Some(Permission::FileSystemWrite),
            _ => None,
        }
    }

    /// Convert permission to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::ReadArticles => "read_articles",
            Permission::WriteArticles => "write_articles",
            Permission::ReadComments => "read_comments",
            Permission::WriteComments => "write_comments",
            Permission::ReadConfig => "read_config",
            Permission::WriteConfig => "write_config",
            Permission::Network => "network",
            Permission::FileSystemRead => "fs_read",
            Permission::FileSystemWrite => "fs_write",
        }
    }
}

/// Resource limits for plugin execution
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory in bytes
    pub memory_limit_bytes: u64,
    /// Maximum fuel (instruction count proxy)
    pub fuel_limit: u64,
    /// Execution timeout
    pub timeout: Duration,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_limit_bytes: DEFAULT_MEMORY_LIMIT_BYTES,
            fuel_limit: DEFAULT_FUEL_LIMIT,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

/// Plugin manifest declaring metadata and permissions
#[derive(Debug, Clone)]
pub struct PluginManifest {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: Option<String>,
    /// Plugin author
    pub author: Option<String>,
    /// Declared permissions
    pub permissions: Vec<Permission>,
}

impl Default for PluginManifest {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            version: "0.0.0".to_string(),
            description: None,
            author: None,
            permissions: vec![],
        }
    }
}


/// Error types for plugin operations
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin execution timed out after {0:?}")]
    Timeout(Duration),

    #[error("Plugin exceeded memory limit of {0} bytes")]
    MemoryLimitExceeded(u64),

    #[error("Plugin ran out of fuel (instruction limit exceeded)")]
    FuelExhausted,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid plugin manifest: {0}")]
    InvalidManifest(String),

    #[error("Plugin not found: {0}")]
    NotFound(u64),

    #[error("WASM error: {0}")]
    WasmError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result of a plugin execution
#[derive(Debug)]
pub struct ExecutionResult {
    /// Output data from the plugin
    pub output: Vec<u8>,
    /// Fuel consumed during execution
    pub fuel_consumed: u64,
    /// Whether the execution completed successfully
    pub success: bool,
}

/// Internal state for a loaded plugin
struct LoadedPlugin {
    /// Plugin manifest
    manifest: PluginManifest,
    /// Compiled WASM module
    module: Module,
    /// Resource limits for this plugin
    limits: ResourceLimits,
    /// Pre-created store (reusable)
    store: Store<()>,
    /// Pre-created instance
    instance: Option<Instance>,
}

/// Plugin runtime for executing WASM plugins
pub struct PluginRuntime {
    /// Wasmtime engine with fuel consumption enabled
    engine: Engine,
    /// Loaded plugins indexed by handle
    plugins: HashMap<u64, LoadedPlugin>,
    /// Counter for generating unique handles
    next_handle: AtomicU64,
    /// Default resource limits
    default_limits: ResourceLimits,
    /// Allowed permissions (whitelist)
    allowed_permissions: Vec<Permission>,
}

impl PluginRuntime {
    /// Create a new plugin runtime with default settings
    pub fn new() -> Result<Self> {
        Self::with_limits(ResourceLimits::default())
    }

    /// Create a new plugin runtime with custom resource limits
    pub fn with_limits(limits: ResourceLimits) -> Result<Self> {
        let mut config = Config::new();
        // Enable fuel consumption for instruction limiting
        config.consume_fuel(true);
        // Enable epoch interruption for timeout handling
        config.epoch_interruption(true);

        let engine = Engine::new(&config)
            .map_err(|e| anyhow!("Failed to create WASM engine: {}", e))?;

        Ok(Self {
            engine,
            plugins: HashMap::new(),
            next_handle: AtomicU64::new(1),
            default_limits: limits,
            allowed_permissions: vec![
                Permission::ReadArticles,
                Permission::ReadConfig,
            ],
        })
    }

    /// Set allowed permissions for plugins
    pub fn set_allowed_permissions(&mut self, permissions: Vec<Permission>) {
        self.allowed_permissions = permissions;
    }

    /// Validate plugin permissions against allowed permissions
    fn validate_permissions(&self, manifest: &PluginManifest) -> Result<(), PluginError> {
        for perm in &manifest.permissions {
            if !self.allowed_permissions.contains(perm) {
                return Err(PluginError::PermissionDenied(format!(
                    "Permission '{}' is not allowed",
                    perm.as_str()
                )));
            }
        }
        Ok(())
    }

    /// Check if a plugin has a specific permission
    pub fn has_permission(&self, handle: PluginHandle, permission: &Permission) -> bool {
        self.plugins
            .get(&handle.0)
            .map(|p| p.manifest.permissions.contains(permission))
            .unwrap_or(false)
    }

    /// Load a plugin from WASM bytes with a manifest
    pub fn load_plugin_bytes(
        &mut self,
        wasm_bytes: &[u8],
        manifest: PluginManifest,
    ) -> Result<PluginHandle, PluginError> {
        self.load_plugin_bytes_with_limits(wasm_bytes, manifest, self.default_limits.clone())
    }

    /// Load a plugin from WASM bytes with custom limits
    pub fn load_plugin_bytes_with_limits(
        &mut self,
        wasm_bytes: &[u8],
        manifest: PluginManifest,
        limits: ResourceLimits,
    ) -> Result<PluginHandle, PluginError> {
        // Validate permissions
        self.validate_permissions(&manifest)?;

        // Compile the WASM module
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| PluginError::WasmError(e.to_string()))?;

        // Pre-create store and instance so we don't instantiate on every call
        let mut store = Store::new(&self.engine, ());
        store.set_fuel(limits.fuel_limit)
            .map_err(|e| PluginError::WasmError(e.to_string()))?;
        store.epoch_deadline_trap();

        let linker = Linker::new(&self.engine);
        let instance = linker.instantiate(&mut store, &module)
            .map_err(|e| PluginError::WasmError(format!("Failed to instantiate WASM module: {}", e)))?;

        let handle_id = self.next_handle.fetch_add(1, Ordering::SeqCst);
        let handle = PluginHandle(handle_id);

        self.plugins.insert(
            handle_id,
            LoadedPlugin {
                manifest,
                module,
                limits,
                store,
                instance: Some(instance),
            },
        );

        Ok(handle)
    }

    /// Load a plugin from file
    pub async fn load_plugin(&mut self, path: &Path) -> Result<PluginHandle, PluginError> {
        let wasm_bytes = tokio::fs::read(path).await?;
        
        // For now, use a default manifest - in production, this would be read from the plugin
        let manifest = PluginManifest {
            name: path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            ..Default::default()
        };

        self.load_plugin_bytes(&wasm_bytes, manifest)
    }

    /// Unload a plugin
    pub async fn unload_plugin(&mut self, handle: PluginHandle) -> Result<(), PluginError> {
        self.plugins
            .remove(&handle.0)
            .ok_or(PluginError::NotFound(handle.0))?;
        Ok(())
    }


    /// Execute a plugin function with resource limits
    /// 
    /// Data passing protocol:
    /// 1. Host calls plugin's `allocate(size) -> ptr` to get a memory buffer
    /// 2. Host writes JSON input bytes into that buffer
    /// 3. Host calls `func_name(ptr, len) -> result_ptr`
    /// 4. Result is encoded as: first 4 bytes = result length, followed by result JSON bytes
    /// 5. If result_ptr is 0, no output data (function succeeded with no modifications)
    /// 
    /// For plugins without allocate/memory, falls back to simple () -> i32 calling convention.
    pub fn execute_with_limits(
        &mut self,
        handle: PluginHandle,
        func_name: &str,
        input: &[u8],
    ) -> Result<ExecutionResult, PluginError> {
        let plugin = self
            .plugins
            .get_mut(&handle.0)
            .ok_or(PluginError::NotFound(handle.0))?;

        let instance = plugin.instance
            .ok_or_else(|| PluginError::WasmError("Plugin not instantiated".to_string()))?;

        // Refuel the store for this execution
        let _ = plugin.store.set_fuel(plugin.limits.fuel_limit);

        // Get memory for passing data
        let memory = instance
            .get_memory(&mut plugin.store, "memory")
            .ok_or_else(|| PluginError::WasmError("Plugin has no memory export".to_string()))?;

        // Check memory limit
        let current_memory = memory.data_size(&plugin.store) as u64;
        if current_memory > plugin.limits.memory_limit_bytes {
            return Err(PluginError::MemoryLimitExceeded(plugin.limits.memory_limit_bytes));
        }

        // Get the function to call
        let func = instance
            .get_func(&mut plugin.store, func_name)
            .ok_or_else(|| PluginError::WasmError(format!("Function '{}' not found", func_name)))?;

        let start_fuel = plugin.store.get_fuel().unwrap_or(0);

        // Try the data-passing protocol: func(ptr, len) -> result_ptr
        let output = if let Some(alloc_func) = instance.get_func(&mut plugin.store, "allocate") {
            if let Ok(alloc_typed) = alloc_func.typed::<i32, i32>(&plugin.store) {
                let input_len = input.len() as i32;
                let input_ptr = alloc_typed.call(&mut plugin.store, input_len)
                    .map_err(|e| PluginError::WasmError(format!("allocate failed: {}", e)))?;

                // Write input data into WASM memory
                let ptr = input_ptr as usize;
                let mem = memory.data_mut(&mut plugin.store);
                if ptr + input.len() <= mem.len() {
                    mem[ptr..ptr + input.len()].copy_from_slice(input);
                } else {
                    return Err(PluginError::WasmError("Input data exceeds WASM memory".to_string()));
                }

                // Call the hook function with (ptr, len) -> result_ptr
                if let Ok(typed_func) = func.typed::<(i32, i32), i32>(&plugin.store) {
                    let result_ptr = typed_func.call(&mut plugin.store, (input_ptr, input_len))
                        .map_err(|e| PluginError::WasmError(e.to_string()))?;

                    if result_ptr > 0 {
                        let mem_data = memory.data(&plugin.store);
                        let rp = result_ptr as usize;
                        if rp + 4 <= mem_data.len() {
                            let result_len = u32::from_le_bytes([
                                mem_data[rp], mem_data[rp + 1],
                                mem_data[rp + 2], mem_data[rp + 3],
                            ]) as usize;
                            if rp + 4 + result_len <= mem_data.len() {
                                mem_data[rp + 4..rp + 4 + result_len].to_vec()
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
                    // Fallback to simple call
                    if let Ok(typed_func) = func.typed::<(), i32>(&plugin.store) {
                        let _ = typed_func.call(&mut plugin.store, ());
                    }
                    vec![]
                }
            } else {
                if let Ok(typed_func) = func.typed::<(), i32>(&plugin.store) {
                    let _ = typed_func.call(&mut plugin.store, ());
                }
                vec![]
            }
        } else {
            // No allocate function — use simple () -> i32 calling convention
            if let Ok(typed_func) = func.typed::<(), i32>(&plugin.store) {
                let _ = typed_func.call(&mut plugin.store, ());
            }
            vec![]
        };

        let end_fuel = plugin.store.get_fuel().unwrap_or(0);
        let fuel_consumed = start_fuel.saturating_sub(end_fuel);

        Ok(ExecutionResult {
            output,
            fuel_consumed,
            success: true,
        })
    }

    /// Execute a plugin with timeout (async wrapper)
    pub async fn execute_with_timeout(
        &mut self,
        handle: PluginHandle,
        func_name: &str,
        input: &[u8],
        timeout: Duration,
    ) -> Result<ExecutionResult, PluginError> {
        let handle_copy = handle;
        let func_name = func_name.to_string();
        let input = input.to_vec();

        // Use tokio timeout for async timeout handling
        match tokio::time::timeout(timeout, async {
            self.execute_with_limits(handle_copy, &func_name, &input)
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(PluginError::Timeout(timeout)),
        }
    }

    /// Call a hook on all plugins
    pub async fn call_hook(&mut self, hook: &str, data: &[u8]) -> Result<Vec<u8>> {
        let mut results = Vec::new();
        
        let handle_ids: Vec<(u64, Duration)> = self.plugins.iter()
            .map(|(&id, p)| (id, p.limits.timeout))
            .collect();

        for (handle_id, timeout) in handle_ids {
            let handle = PluginHandle(handle_id);
            
            match self.execute_with_timeout(
                handle,
                &format!("hook_{}", hook),
                data,
                timeout,
            )
            .await
            {
                Ok(result) => {
                    results.extend(result.output);
                }
                Err(PluginError::WasmError(_)) => {
                    continue;
                }
                Err(e) => {
                    if let Some(plugin) = self.plugins.get(&handle_id) {
                        tracing::warn!(
                            "Plugin {} failed to handle hook {}: {}",
                            plugin.manifest.name,
                            hook,
                            e
                        );
                    }
                }
            }
        }

        Ok(results)
    }

    /// List loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .iter()
            .map(|(&handle_id, plugin)| PluginInfo {
                handle: PluginHandle(handle_id),
                name: plugin.manifest.name.clone(),
                version: plugin.manifest.version.clone(),
                description: plugin.manifest.description.clone(),
                author: plugin.manifest.author.clone(),
                permissions: plugin
                    .manifest
                    .permissions
                    .iter()
                    .map(|p| p.as_str().to_string())
                    .collect(),
            })
            .collect()
    }

    /// Get plugin info by handle
    pub fn get_plugin_info(&self, handle: PluginHandle) -> Option<PluginInfo> {
        self.plugins.get(&handle.0).map(|plugin| PluginInfo {
            handle,
            name: plugin.manifest.name.clone(),
            version: plugin.manifest.version.clone(),
            description: plugin.manifest.description.clone(),
            author: plugin.manifest.author.clone(),
            permissions: plugin
                .manifest
                .permissions
                .iter()
                .map(|p| p.as_str().to_string())
                .collect(),
        })
    }

    /// Get resource limits for a plugin
    pub fn get_plugin_limits(&self, handle: PluginHandle) -> Option<ResourceLimits> {
        self.plugins.get(&handle.0).map(|p| p.limits.clone())
    }

    /// Update resource limits for a plugin
    pub fn set_plugin_limits(
        &mut self,
        handle: PluginHandle,
        limits: ResourceLimits,
    ) -> Result<(), PluginError> {
        let plugin = self
            .plugins
            .get_mut(&handle.0)
            .ok_or(PluginError::NotFound(handle.0))?;
        plugin.limits = limits;
        Ok(())
    }
}

impl Default for PluginRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create plugin runtime")
    }
}

/// Handle to a loaded plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginHandle(pub u64);

/// Information about a plugin
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Plugin handle
    pub handle: PluginHandle,
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: Option<String>,
    /// Plugin author
    pub author: Option<String>,
    /// Declared permissions
    pub permissions: Vec<String>,
}

#[cfg(test)]
mod tests;
