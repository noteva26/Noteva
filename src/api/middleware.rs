//! API middleware
//!
//! Contains middleware for:
//! - Authentication (Session token validation)
//! - Authorization (permission checking)
//!
//! Satisfies requirements:
//! - 4.3: Session token validation
//! - 5.4: Permission control for admin access

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::models::{User, UserRole};
use crate::services::user::UserService;
use crate::plugin::{PluginManager, HookManager, ShortcodeManager};

// ============================================================================
// Request Statistics
// ============================================================================

/// Lightweight request statistics using atomic operations (no locks)
pub struct RequestStats {
    /// Total number of requests processed
    total_requests: AtomicU64,
    /// Total response time in microseconds (for calculating average)
    total_response_time_us: AtomicU64,
    /// Application start time
    start_time: Instant,
}

impl RequestStats {
    /// Create new stats tracker
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_response_time_us: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }
    
    /// Record a request with its response time
    pub fn record(&self, duration_us: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_response_time_us.fetch_add(duration_us, Ordering::Relaxed);
    }
    
    /// Get total request count
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }
    
    /// Get average response time in microseconds
    pub fn avg_response_time_us(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        let total_time = self.total_response_time_us.load(Ordering::Relaxed);
        total_time as f64 / total as f64
    }
    
    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Default for RequestStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state containing shared services
#[derive(Clone)]
pub struct AppState {
    pub pool: crate::db::DynDatabasePool,
    pub user_service: Arc<UserService>,
    pub user_repo: Arc<dyn crate::db::repositories::UserRepository>,
    pub article_service: Arc<crate::services::article::ArticleService>,
    pub category_service: Arc<crate::services::category::CategoryService>,
    pub tag_service: Arc<crate::services::tag::TagService>,
    pub settings_service: Arc<crate::services::settings::SettingsService>,
    pub comment_service: Arc<crate::services::comment::CommentService>,
    pub theme_engine: Arc<RwLock<crate::theme::ThemeEngine>>,
    pub upload_config: Arc<crate::config::UploadConfig>,
    pub page_service: Arc<crate::services::page::PageService>,
    pub nav_service: Arc<crate::services::nav_item::NavItemService>,
    pub plugin_manager: Arc<RwLock<PluginManager>>,
    pub hook_manager: Arc<HookManager>,
    pub shortcode_manager: Arc<ShortcodeManager>,
    pub request_stats: Arc<RequestStats>,
}

/// Authenticated user extracted from request
#[derive(Debug, Clone)]
pub struct AuthenticatedUser(pub User);

/// Error response for API errors
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub error: ApiErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ApiErrorDetail {
                code: code.into(),
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            error: ApiErrorDetail {
                code: code.into(),
                message: message.into(),
                details: Some(details),
            },
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new("UNAUTHORIZED", message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new("FORBIDDEN", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("NOT_FOUND", message)
    }

    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new("VALIDATION_ERROR", message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.error.code.as_str() {
            "UNAUTHORIZED" => StatusCode::UNAUTHORIZED,
            "FORBIDDEN" => StatusCode::FORBIDDEN,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "VALIDATION_ERROR" => StatusCode::BAD_REQUEST,
            "CONFLICT" => StatusCode::CONFLICT,
            "USER_BANNED" => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}

/// Extract session token from request
fn extract_session_token(request: &Request) -> Option<String> {
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    if let Some(cookie_header) = request.headers().get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(token) = cookie.strip_prefix("session=") {
                    return Some(token.to_string());
                }
            }
        }
    }

    None
}

/// Authentication middleware
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let token = extract_session_token(&request)
        .ok_or_else(|| ApiError::unauthorized("Missing authentication token"))?;

    let user = state
        .user_service
        .validate_session(&token)
        .await
        .map_err(|e| ApiError::internal_error(format!("Session validation failed: {}", e)))?
        .ok_or_else(|| ApiError::unauthorized("Invalid or expired session"))?;

    request.extensions_mut().insert(AuthenticatedUser(user));
    Ok(next.run(request).await)
}

/// Optional authentication middleware
pub async fn optional_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    if let Some(token) = extract_session_token(&request) {
        if let Ok(Some(user)) = state.user_service.validate_session(&token).await {
            request.extensions_mut().insert(AuthenticatedUser(user));
        }
    }
    next.run(request).await
}

/// Admin authorization middleware
pub async fn require_admin(request: Request, next: Next) -> Result<Response, ApiError> {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .ok_or_else(|| ApiError::unauthorized("Authentication required"))?;

    if user.0.role != UserRole::Admin {
        return Err(ApiError::forbidden("Admin privileges required"));
    }

    Ok(next.run(request).await)
}

/// Editor authorization middleware
pub async fn require_editor(request: Request, next: Next) -> Result<Response, ApiError> {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .ok_or_else(|| ApiError::unauthorized("Authentication required"))?;

    if !user.0.is_editor() {
        return Err(ApiError::forbidden("Editor privileges required"));
    }

    Ok(next.run(request).await)
}

/// API hooks middleware
/// 
/// Triggers `api_request_before` before processing and `api_request_after` after processing.
/// This allows plugins to intercept, log, or modify API requests/responses.
pub async fn api_hooks_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    use crate::plugin::hook_names;
    
    // Extract request info for hooks
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let path = request.uri().path().to_string();
    
    // Trigger api_request_before hook
    let before_data = serde_json::json!({
        "method": method,
        "uri": uri,
        "path": path,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    state.hook_manager.trigger(hook_names::API_REQUEST_BEFORE, before_data);
    
    // Process the request
    let response = next.run(request).await;
    
    // Trigger api_request_after hook
    let status = response.status().as_u16();
    let after_data = serde_json::json!({
        "method": method,
        "uri": uri,
        "path": path,
        "status": status,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    state.hook_manager.trigger(hook_names::API_REQUEST_AFTER, after_data);
    
    response
}

/// Request statistics middleware
/// 
/// Records request count and response time for performance monitoring.
/// Uses atomic operations for minimal overhead.
pub async fn request_stats_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    
    // Process the request
    let response = next.run(request).await;
    
    // Record stats (microseconds for precision)
    let duration_us = start.elapsed().as_micros() as u64;
    state.request_stats.record(duration_us);
    
    response
}

/// Extract authenticated user from request extensions
pub fn get_authenticated_user(request: &Request) -> Option<&User> {
    request.extensions().get::<AuthenticatedUser>().map(|au| &au.0)
}


// ============================================================================
// HTTP Cache Headers
// ============================================================================

/// Cache control configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub static_max_age: u32,
    pub api_max_age: u32,
    pub stale_while_revalidate: Option<u32>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            static_max_age: 31536000,
            api_max_age: 300,
            stale_while_revalidate: Some(3600),
        }
    }
}

/// Generate ETag from content
pub fn generate_etag(content: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("\"{}\"", hasher.finish())
}

/// Generate weak ETag from content
pub fn generate_weak_etag(content: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("W/\"{}\"", hasher.finish())
}

/// Check if ETags match
pub fn etag_matches(request_etag: Option<&str>, response_etag: &str) -> bool {
    match request_etag {
        Some(etag) => {
            let normalized_request = etag.trim_start_matches("W/");
            let normalized_response = response_etag.trim_start_matches("W/");
            normalized_request == normalized_response
        }
        None => false,
    }
}

/// Build Cache-Control header for static assets
pub fn cache_control_static(max_age: u32, immutable: bool) -> String {
    if immutable {
        format!("public, max-age={}, immutable", max_age)
    } else {
        format!("public, max-age={}", max_age)
    }
}

/// Build Cache-Control header for API responses
pub fn cache_control_api(max_age: u32, stale_while_revalidate: Option<u32>) -> String {
    match stale_while_revalidate {
        Some(swr) => format!("public, max-age={}, stale-while-revalidate={}", max_age, swr),
        None => format!("public, max-age={}", max_age),
    }
}

/// Build Cache-Control header for private content
pub fn cache_control_private(max_age: u32) -> String {
    format!("private, max-age={}", max_age)
}

/// Build Cache-Control header for no-cache content
pub fn cache_control_no_cache() -> String {
    "no-cache, no-store, must-revalidate".to_string()
}

/// Middleware to add cache headers to static assets
pub async fn add_static_cache_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let cache_control = cache_control_static(31536000, true);
    response.headers_mut().insert(header::CACHE_CONTROL, cache_control.parse().unwrap());
    response
}

/// Middleware to add cache headers to API responses
pub async fn add_api_cache_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    if response.status().is_success() {
        let cache_control = cache_control_api(300, Some(3600));
        response.headers_mut().insert(header::CACHE_CONTROL, cache_control.parse().unwrap());
    }
    response
}

/// Response wrapper with ETag support
#[derive(Debug)]
pub struct CachedResponse<T> {
    pub data: T,
    pub etag: String,
}

impl<T: Serialize> CachedResponse<T> {
    pub fn new(data: T) -> Self {
        let json = serde_json::to_vec(&data).unwrap_or_default();
        let etag = generate_etag(&json);
        Self { data, etag }
    }

    pub fn with_etag(data: T, etag: String) -> Self {
        Self { data, etag }
    }
}

impl<T: Serialize> IntoResponse for CachedResponse<T> {
    fn into_response(self) -> Response {
        let json = serde_json::to_vec(&self.data).unwrap_or_default();
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ETAG, &self.etag)
            .header(header::CACHE_CONTROL, cache_control_api(300, Some(3600)))
            .body(axum::body::Body::from(json))
            .unwrap()
    }
}

/// Check If-None-Match header
pub fn check_if_none_match(request: &Request, etag: &str) -> Option<Response> {
    if let Some(if_none_match) = request.headers().get(header::IF_NONE_MATCH) {
        if let Ok(if_none_match_str) = if_none_match.to_str() {
            if etag_matches(Some(if_none_match_str), etag) {
                return Some(
                    Response::builder()
                        .status(StatusCode::NOT_MODIFIED)
                        .header(header::ETAG, etag)
                        .body(axum::body::Body::empty())
                        .unwrap()
                );
            }
        }
    }
    None
}


// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};

    fn create_request_with_auth(token: &str) -> Request<Body> {
        Request::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap()
    }

    fn create_request_with_cookie(token: &str) -> Request<Body> {
        Request::builder()
            .uri("/test")
            .header(header::COOKIE, format!("session={}", token))
            .body(Body::empty())
            .unwrap()
    }

    #[test]
    fn test_extract_session_token_from_bearer() {
        let request = create_request_with_auth("test-token-123");
        assert_eq!(extract_session_token(&request), Some("test-token-123".to_string()));
    }

    #[test]
    fn test_extract_session_token_from_cookie() {
        let request = create_request_with_cookie("test-token-456");
        assert_eq!(extract_session_token(&request), Some("test-token-456".to_string()));
    }

    #[test]
    fn test_extract_session_token_bearer_priority() {
        let request = Request::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, "Bearer bearer-token")
            .header(header::COOKIE, "session=cookie-token")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_session_token(&request), Some("bearer-token".to_string()));
    }

    #[test]
    fn test_extract_session_token_none() {
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
        assert!(extract_session_token(&request).is_none());
    }

    #[test]
    fn test_extract_session_token_invalid_bearer() {
        let request = Request::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, "Basic invalid")
            .body(Body::empty())
            .unwrap();
        assert!(extract_session_token(&request).is_none());
    }

    #[test]
    fn test_api_error_unauthorized() {
        let error = ApiError::unauthorized("Test message");
        assert_eq!(error.error.code, "UNAUTHORIZED");
    }

    #[test]
    fn test_api_error_forbidden() {
        let error = ApiError::forbidden("Access denied");
        assert_eq!(error.error.code, "FORBIDDEN");
    }

    #[test]
    fn test_api_error_with_details() {
        let details = serde_json::json!({"field": "username"});
        let error = ApiError::with_details("VALIDATION_ERROR", "Invalid", details.clone());
        assert_eq!(error.error.details, Some(details));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::models::UserRole;
    use proptest::prelude::*;

    fn role_strategy() -> impl Strategy<Value = UserRole> {
        prop_oneof![Just(UserRole::Admin), Just(UserRole::Editor), Just(UserRole::Author)]
    }

    fn non_admin_role_strategy() -> impl Strategy<Value = UserRole> {
        prop_oneof![Just(UserRole::Editor), Just(UserRole::Author)]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Property 13: Permission Control** - **Validates: Requirements 5.4**
        #[test]
        fn property_13_non_admin_denied_admin_access(role in non_admin_role_strategy()) {
            let user = User {
                id: 1, username: "testuser".to_string(), email: "test@example.com".to_string(),
                password_hash: "hash".to_string(), role,
                status: crate::models::UserStatus::Active,
                display_name: None, avatar: None,
                created_at: chrono::Utc::now(), updated_at: chrono::Utc::now(),
            };
            prop_assert!(!user.is_admin());
        }

        #[test]
        fn property_13_admin_has_admin_access(_dummy in 0..20i32) {
            let user = User {
                id: 1, username: "admin".to_string(), email: "admin@example.com".to_string(),
                password_hash: "hash".to_string(), role: UserRole::Admin,
                status: crate::models::UserStatus::Active,
                display_name: None, avatar: None,
                created_at: chrono::Utc::now(), updated_at: chrono::Utc::now(),
            };
            prop_assert!(user.is_admin());
        }

        #[test]
        fn property_13_editor_privileges(role in role_strategy()) {
            let user = User {
                id: 1, username: "testuser".to_string(), email: "test@example.com".to_string(),
                password_hash: "hash".to_string(), role,
                status: crate::models::UserStatus::Active,
                display_name: None, avatar: None,
                created_at: chrono::Utc::now(), updated_at: chrono::Utc::now(),
            };
            let expected = matches!(role, UserRole::Admin | UserRole::Editor);
            prop_assert_eq!(user.is_editor(), expected);
        }

        #[test]
        fn property_13_author_can_edit_own_content(user_id in 1i64..100, content_author_id in 1i64..100) {
            let user = User {
                id: user_id, username: "author".to_string(), email: "author@example.com".to_string(),
                password_hash: "hash".to_string(), role: UserRole::Author,
                status: crate::models::UserStatus::Active,
                display_name: None, avatar: None,
                created_at: chrono::Utc::now(), updated_at: chrono::Utc::now(),
            };
            prop_assert_eq!(user.can_edit(content_author_id), user_id == content_author_id);
        }

        #[test]
        fn property_13_admin_editor_can_edit_any_content(user_id in 1i64..100, content_author_id in 1i64..100, is_admin in prop::bool::ANY) {
            let role = if is_admin { UserRole::Admin } else { UserRole::Editor };
            let user = User {
                id: user_id, username: "editor".to_string(), email: "editor@example.com".to_string(),
                password_hash: "hash".to_string(), role,
                status: crate::models::UserStatus::Active,
                display_name: None, avatar: None,
                created_at: chrono::Utc::now(), updated_at: chrono::Utc::now(),
            };
            prop_assert!(user.can_edit(content_author_id));
        }
    }
}


#[cfg(test)]
mod cache_header_tests {
    use super::*;

    #[test]
    fn test_generate_etag_deterministic() {
        let content = b"Hello, World!";
        assert_eq!(generate_etag(content), generate_etag(content));
    }

    #[test]
    fn test_generate_etag_different_content() {
        assert_ne!(generate_etag(b"Hello"), generate_etag(b"World"));
    }

    #[test]
    fn test_generate_weak_etag_format() {
        let etag = generate_weak_etag(b"test");
        assert!(etag.starts_with("W/\"") && etag.ends_with("\""));
    }

    #[test]
    fn test_etag_matches_exact() {
        assert!(etag_matches(Some("\"12345\""), "\"12345\""));
        assert!(!etag_matches(Some("\"54321\""), "\"12345\""));
    }

    #[test]
    fn test_etag_matches_weak() {
        assert!(etag_matches(Some("W/\"12345\""), "W/\"12345\""));
        assert!(etag_matches(Some("\"12345\""), "W/\"12345\""));
    }

    #[test]
    fn test_etag_matches_none() {
        assert!(!etag_matches(None, "\"12345\""));
    }

    #[test]
    fn test_cache_control_static_immutable() {
        let header = cache_control_static(31536000, true);
        assert!(header.contains("public") && header.contains("immutable"));
    }

    #[test]
    fn test_cache_control_static_mutable() {
        let header = cache_control_static(3600, false);
        assert!(header.contains("public") && !header.contains("immutable"));
    }

    #[test]
    fn test_cache_control_api_with_swr() {
        let header = cache_control_api(300, Some(3600));
        assert!(header.contains("stale-while-revalidate=3600"));
    }

    #[test]
    fn test_cache_control_api_without_swr() {
        let header = cache_control_api(300, None);
        assert!(!header.contains("stale-while-revalidate"));
    }

    #[test]
    fn test_cache_control_private() {
        let header = cache_control_private(600);
        assert!(header.contains("private") && !header.contains("public"));
    }

    #[test]
    fn test_cache_control_no_cache() {
        let header = cache_control_no_cache();
        assert!(header.contains("no-cache") && header.contains("no-store"));
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.static_max_age, 31536000);
        assert_eq!(config.api_max_age, 300);
    }
}

#[cfg(test)]
mod cache_header_property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        /// **Property 25: HTTP Cache Headers** - **Validates: Requirements 10.5**
        #[test]
        fn property_25_etag_determinism(content in prop::collection::vec(any::<u8>(), 0..100)) {
            prop_assert_eq!(generate_etag(&content), generate_etag(&content));
        }

        #[test]
        fn property_25_etag_format(content in prop::collection::vec(any::<u8>(), 0..100)) {
            let etag = generate_etag(&content);
            prop_assert!(etag.starts_with("\"") && etag.ends_with("\"") && etag.len() > 2);
        }

        #[test]
        fn property_25_weak_etag_format(content in prop::collection::vec(any::<u8>(), 0..100)) {
            let etag = generate_weak_etag(&content);
            prop_assert!(etag.starts_with("W/\"") && etag.ends_with("\""));
        }

        #[test]
        fn property_25_cache_control_max_age(max_age in 0u32..=31536000u32) {
            let header = cache_control_static(max_age, false);
            let expected = format!("max-age={}", max_age);
            prop_assert!(header.contains(&expected));
        }

        #[test]
        fn property_25_static_cache_public(max_age in 1u32..=31536000u32, immutable in prop::bool::ANY) {
            let header = cache_control_static(max_age, immutable);
            prop_assert!(header.contains("public"));
            prop_assert_eq!(header.contains("immutable"), immutable);
        }

        #[test]
        fn property_25_api_cache_swr(max_age in 1u32..=3600u32, swr in prop::option::of(1u32..=86400u32)) {
            let header = cache_control_api(max_age, swr);
            let expected = format!("max-age={}", max_age);
            prop_assert!(header.contains(&expected));
            prop_assert_eq!(header.contains("stale-while-revalidate"), swr.is_some());
        }

        #[test]
        fn property_25_private_cache_isolation(max_age in 1u32..=3600u32) {
            let header = cache_control_private(max_age);
            prop_assert!(header.contains("private") && !header.contains("public"));
        }

        #[test]
        fn property_25_etag_matching(hash in "[0-9a-f]{8,16}") {
            let etag = format!("\"{}\"", hash);
            prop_assert!(etag_matches(Some(&etag), &etag));
            prop_assert!(!etag_matches(None, &etag));
        }
    }
}
