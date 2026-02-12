//! WASM Worker Process
//!
//! Isolated subprocess for executing WASM plugin hooks.
//! This process is spawned by the main Noteva server to execute WASM code safely.
//! If the WASM code crashes (trap, OOM, etc.), only this subprocess dies —
//! the main server process is completely unaffected.
//!
//! Protocol (via stdin/stdout):
//!   Input (JSON):  { "wasm_path": "...", "func_name": "...", "input": "..." }
//!   Output (JSON): { "success": true, "output": "..." }
//!              or: { "success": false, "error": "..." }
//!
//! The "input" and "output" fields are base64-encoded bytes.

use std::io::{self, Read};

fn main() {
    // Read entire stdin
    let mut input_str = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input_str) {
        print_error(&format!("Failed to read stdin: {}", e));
        return;
    }

    // Parse request
    let request: serde_json::Value = match serde_json::from_str(&input_str) {
        Ok(v) => v,
        Err(e) => {
            print_error(&format!("Invalid JSON input: {}", e));
            return;
        }
    };

    let wasm_path = match request.get("wasm_path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            print_error("Missing 'wasm_path' field");
            return;
        }
    };

    let func_name = match request.get("func_name").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => {
            print_error("Missing 'func_name' field");
            return;
        }
    };

    let input_b64 = request.get("input").and_then(|v| v.as_str()).unwrap_or("");
    let input_bytes = match base64_decode(input_b64) {
        Ok(b) => b,
        Err(e) => {
            print_error(&format!("Failed to decode input: {}", e));
            return;
        }
    };

    // Load and execute WASM
    match execute_wasm(wasm_path, func_name, &input_bytes) {
        Ok(output) => {
            let output_b64 = base64_encode(&output);
            let result = serde_json::json!({
                "success": true,
                "output": output_b64,
            });
            println!("{}", result);
        }
        Err(e) => {
            print_error(&format!("WASM execution failed: {}", e));
        }
    }
}

fn print_error(msg: &str) {
    let result = serde_json::json!({
        "success": false,
        "error": msg,
    });
    println!("{}", result);
}

fn execute_wasm(wasm_path: &str, func_name: &str, input: &[u8]) -> Result<Vec<u8>, String> {
    // Create engine with fuel consumption
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);

    let engine = wasmtime::Engine::new(&config)
        .map_err(|e| format!("Engine creation failed: {}", e))?;

    // Load WASM module
    let wasm_bytes = std::fs::read(wasm_path)
        .map_err(|e| format!("Failed to read WASM file: {}", e))?;

    let module = wasmtime::Module::new(&engine, &wasm_bytes)
        .map_err(|e| format!("Failed to compile WASM: {}", e))?;

    // Create store with fuel limit
    let mut store = wasmtime::Store::new(&engine, ());
    store.set_fuel(1_000_000)
        .map_err(|e| format!("Failed to set fuel: {}", e))?;

    // Instantiate
    let linker = wasmtime::Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &module)
        .map_err(|e| format!("Instantiation failed: {}", e))?;

    // Get memory
    let memory = instance.get_memory(&mut store, "memory")
        .ok_or_else(|| "No memory export".to_string())?;

    // Get the target function
    let func = instance.get_func(&mut store, func_name)
        .ok_or_else(|| format!("Function '{}' not found", func_name))?;

    // Try data-passing protocol: allocate -> write -> call(ptr, len) -> read result
    let output = if let Some(alloc_func) = instance.get_func(&mut store, "allocate") {
        if let Ok(alloc_typed) = alloc_func.typed::<i32, i32>(&store) {
            let input_len = input.len() as i32;
            let input_ptr = alloc_typed.call(&mut store, input_len)
                .map_err(|e| format!("allocate failed: {}", e))?;

            // Write input data into WASM memory
            let ptr = input_ptr as usize;
            let mem = memory.data_mut(&mut store);
            if ptr + input.len() > mem.len() {
                return Err("Input data exceeds WASM memory".to_string());
            }
            mem[ptr..ptr + input.len()].copy_from_slice(input);

            // Call the hook function with (ptr, len) -> result_ptr
            if let Ok(typed_func) = func.typed::<(i32, i32), i32>(&store) {
                let result_ptr = typed_func.call(&mut store, (input_ptr, input_len))
                    .map_err(|e| format!("Function call failed: {}", e))?;

                if result_ptr > 0 {
                    let mem_data = memory.data(&store);
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
                    // result_ptr == 0 means no modifications
                    vec![]
                }
            } else {
                // Fallback: simple () -> i32
                if let Ok(typed_func) = func.typed::<(), i32>(&store) {
                    let _ = typed_func.call(&mut store, ());
                }
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        // No allocate — simple calling convention
        if let Ok(typed_func) = func.typed::<(), i32>(&store) {
            let _ = typed_func.call(&mut store, ());
        }
        vec![]
    };

    Ok(output)
}

// Simple base64 encode/decode (no external dependency needed)

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
