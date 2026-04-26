//! Plugin proxy API.
//!
//! The old generic proxy accepted an arbitrary frontend-provided URL and
//! substituted plugin secrets into URL/header/body templates. That pattern is
//! intentionally disabled because it lets public requests decide where backend
//! secrets are sent.

use axum::response::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::api::ApiError;

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

/// POST /api/v1/plugins/proxy - deprecated generic plugin proxy.
///
/// External requests that need secrets should go through a plugin-owned backend
/// hook/API handler, where the URL is controlled by plugin settings or plugin
/// code rather than by a public frontend payload.
pub async fn proxy_request(
    Json(_req): Json<ProxyRequest>,
) -> Result<Json<ProxyResponse>, ApiError> {
    Err(ApiError::validation_error(
        "Generic plugin proxy is disabled. Use plugin settings with a backend WASM API or hook instead.",
    ))
}
