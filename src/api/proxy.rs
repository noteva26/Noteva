//! Plugin proxy API
//!
//! Provides a secure proxy for plugins to call external APIs without exposing API keys.

use axum::{
    extract::State,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use reqwest;

use crate::api::{AppState, ApiError};

/// Plugin proxy request
#[derive(Debug, Deserialize)]
pub struct ProxyRequest {
    /// Plugin ID
    pub plugin_id: String,
    /// Target URL
    pub url: String,
    /// HTTP method (GET, POST, PUT, DELETE)
    #[serde(default = "default_method")]
    pub method: String,
    /// Request headers (can contain {{variable}} placeholders)
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body (can contain {{variable}} placeholders)
    pub body: Option<serde_json::Value>,
}

fn default_method() -> String {
    "GET".to_string()
}

/// Plugin proxy response
#[derive(Debug, Serialize)]
pub struct ProxyResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: serde_json::Value,
}

/// POST /api/v1/plugins/proxy - Proxy plugin requests to external APIs
///
/// This endpoint allows plugins to make HTTP requests to external APIs
/// without exposing sensitive credentials (like API keys) to the frontend.
///
/// The proxy:
/// 1. Reads the plugin's full settings from the database (including secret fields)
/// 2. Replaces {{variable}} placeholders in URL, headers, and body with actual values
/// 3. Forwards the request to the external API
/// 4. Returns the response to the frontend
pub async fn proxy_request(
    State(state): State<AppState>,
    Json(req): Json<ProxyRequest>,
) -> Result<Json<ProxyResponse>, ApiError> {
    // Get plugin manager
    let plugin_manager = state.plugin_manager.read().await;
    
    // Get plugin
    let plugin = plugin_manager.get(&req.plugin_id)
        .ok_or_else(|| ApiError::not_found(format!("Plugin not found: {}", req.plugin_id)))?;
    
    // Check if plugin is enabled
    if !plugin.enabled {
        return Err(ApiError::validation_error("Plugin is not enabled"));
    }
    
    // Get plugin settings (includes secret fields)
    let settings = &plugin.settings;
    
    // Replace variables in URL
    let url = replace_variables(&req.url, settings)?;
    
    // Replace variables in headers
    let mut headers = HashMap::new();
    for (key, value) in &req.headers {
        let replaced_value = replace_variables(value, settings)?;
        headers.insert(key.clone(), replaced_value);
    }
    
    // Replace variables in body
    let body = if let Some(body_value) = &req.body {
        Some(replace_variables_in_json(body_value, settings)?)
    } else {
        None
    };
    
    // Create HTTP client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ApiError::internal_error(format!("Failed to create HTTP client: {}", e)))?;
    
    // Build request
    let method = req.method.to_uppercase();
    let mut request_builder = match method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        _ => return Err(ApiError::validation_error(format!("Unsupported HTTP method: {}", method))),
    };
    
    // Add headers
    for (key, value) in headers {
        request_builder = request_builder.header(&key, &value);
    }
    
    // Add body for POST/PUT/PATCH
    if let Some(body_value) = body {
        if method == "POST" || method == "PUT" || method == "PATCH" {
            request_builder = request_builder.json(&body_value);
        }
    }
    
    // Send request
    let response = request_builder
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to send request: {}", e)))?;
    
    // Extract response status
    let status = response.status().as_u16();
    
    // Extract response headers
    let mut response_headers = HashMap::new();
    for (key, value) in response.headers() {
        if let Ok(value_str) = value.to_str() {
            response_headers.insert(key.to_string(), value_str.to_string());
        }
    }
    
    // Extract response body
    let body_text = response
        .text()
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to read response body: {}", e)))?;
    
    // Try to parse as JSON, fallback to string
    let body_json = serde_json::from_str(&body_text)
        .unwrap_or_else(|_| serde_json::json!(body_text));
    
    Ok(Json(ProxyResponse {
        status,
        headers: response_headers,
        body: body_json,
    }))
}

/// Replace {{variable}} placeholders in a string with values from settings
fn replace_variables(text: &str, settings: &HashMap<String, serde_json::Value>) -> Result<String, ApiError> {
    let mut result = text.to_string();
    
    // Find all {{variable}} patterns
    let re = regex::Regex::new(r"\{\{(\w+)\}\}")
        .map_err(|e| ApiError::internal_error(format!("Regex error: {}", e)))?;
    
    for cap in re.captures_iter(text) {
        let var_name = &cap[1];
        let placeholder = &cap[0];
        
        // Get value from settings
        let value = settings.get(var_name)
            .ok_or_else(|| ApiError::validation_error(format!("Variable not found in settings: {}", var_name)))?;
        
        // Convert value to string
        let value_str = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => value.to_string(),
        };
        
        result = result.replace(placeholder, &value_str);
    }
    
    Ok(result)
}

/// Replace {{variable}} placeholders in JSON recursively
fn replace_variables_in_json(
    value: &serde_json::Value,
    settings: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, ApiError> {
    match value {
        serde_json::Value::String(s) => {
            let replaced = replace_variables(s, settings)?;
            Ok(serde_json::Value::String(replaced))
        }
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(k.clone(), replace_variables_in_json(v, settings)?);
            }
            Ok(serde_json::Value::Object(new_map))
        }
        serde_json::Value::Array(arr) => {
            let mut new_arr = Vec::new();
            for item in arr {
                new_arr.push(replace_variables_in_json(item, settings)?);
            }
            Ok(serde_json::Value::Array(new_arr))
        }
        _ => Ok(value.clone()),
    }
}
