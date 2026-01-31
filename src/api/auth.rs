//! Authentication API endpoints
//!
//! Handles HTTP requests for user authentication:
//! - POST /api/v1/auth/register - User registration
//! - POST /api/v1/auth/login - User login
//! - POST /api/v1/auth/logout - User logout
//! - GET /api/v1/auth/me - Get current user
//!
//! Satisfies requirements:
//! - 4.1: First user becomes admin
//! - 4.2: User registration
//! - 4.3: User login

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};
use crate::services::user::{LoginInput, RegisterInput, UserServiceError};

/// Request body for user registration
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// Request body for user login
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username_or_email: String,
    pub password: String,
}

/// Response for successful authentication
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub token: String,
}

/// Response for user info
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub created_at: String,
}

impl From<crate::models::User> for UserResponse {
    fn from(user: crate::models::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role.to_string(),
            created_at: user.created_at.to_rfc3339(),
        }
    }
}

/// Build the auth router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(get_current_user))
}

/// Build protected auth routes (requires auth middleware)
pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/logout", post(logout))
        .route("/me", get(get_current_user))
}

/// Build public auth routes (no auth required)
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

/// POST /api/v1/auth/register - User registration
///
/// Satisfies requirements:
/// - 4.1: First user becomes admin
/// - 4.2: User registration
async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let password = body.password.clone();
    let input = RegisterInput::new(body.username, body.email, body.password);

    let user = state
        .user_service
        .register(input)
        .await
        .map_err(|e| match e {
            UserServiceError::ValidationError(msg) => ApiError::validation_error(msg),
            UserServiceError::UserExists(msg) => {
                ApiError::with_details("CONFLICT", msg, serde_json::json!({}))
            }
            _ => ApiError::internal_error(e.to_string()),
        })?;

    // Create session for the new user
    let login_input = LoginInput::new(&user.username, &password);
    let session = state
        .user_service
        .login(login_input)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Set session cookie
    let cookie = format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        session.id,
        7 * 24 * 60 * 60 // 7 days
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).unwrap(),
    );

    Ok((
        StatusCode::CREATED,
        headers,
        Json(AuthResponse {
            user: user.into(),
            token: session.id,
        }),
    ))
}

/// POST /api/v1/auth/login - User login
///
/// Satisfies requirement 4.3: User login
async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let input = LoginInput::new(body.username_or_email, body.password);

    let session = state
        .user_service
        .login(input)
        .await
        .map_err(|e| match e {
            UserServiceError::AuthenticationError(_) => {
                ApiError::unauthorized("Invalid username or password")
            }
            _ => ApiError::internal_error(e.to_string()),
        })?;

    let user = state
        .user_service
        .validate_session(&session.id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::internal_error("Session validation failed"))?;

    // Set session cookie (httpOnly for security)
    let cookie = format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        session.id,
        7 * 24 * 60 * 60 // 7 days
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).unwrap(),
    );

    Ok((
        headers,
        Json(AuthResponse {
            user: user.into(),
            token: session.id,
        }),
    ))
}

/// POST /api/v1/auth/logout - User logout
///
/// Requires authentication.
async fn logout(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    // Extract token from cookie or Authorization header
    let token = headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .find(|c| c.trim().starts_with("session="))
                .map(|c| c.trim().strip_prefix("session=").unwrap_or(""))
        })
        .or_else(|| {
            headers
                .get(header::AUTHORIZATION)
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
        })
        .ok_or_else(|| ApiError::unauthorized("Missing authentication token"))?;

    state
        .user_service
        .logout(token)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Clear the session cookie
    let clear_cookie = "session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_static(clear_cookie),
    );

    Ok((StatusCode::NO_CONTENT, response_headers))
}

/// GET /api/v1/auth/me - Get current user
///
/// Requires authentication.
async fn get_current_user(user: AuthenticatedUser) -> Json<UserResponse> {
    Json(user.0.into())
}
