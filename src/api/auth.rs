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
    routing::{get, post, put},
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
    pub verification_code: Option<String>,
}

/// Request body for sending verification code
#[derive(Debug, Deserialize)]
pub struct SendCodeRequest {
    pub email: String,
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
    pub status: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub created_at: String,
}

impl From<crate::models::User> for UserResponse {
    fn from(user: crate::models::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role.to_string(),
            status: user.status.to_string(),
            display_name: user.display_name,
            avatar: user.avatar,
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
        .route("/profile", put(update_profile))
        .route("/password", put(change_password))
        .route("/send-code", post(send_verification_code))
        .route("/has-admin", get(has_admin))
}

/// Build protected auth routes (requires auth middleware)
pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/logout", post(logout))
        .route("/me", get(get_current_user))
        .route("/profile", put(update_profile))
        .route("/password", put(change_password))
}

/// Build public auth routes (no auth required)
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/send-code", post(send_verification_code))
        .route("/has-admin", get(has_admin))
}

/// GET /api/v1/auth/has-admin - Check if admin exists
///
/// Returns whether the system has at least one admin user.
/// Used for first-time setup flow.
async fn has_admin(
    State(state): State<AppState>,
) -> Result<Json<HasAdminResponse>, ApiError> {
    let is_first = state
        .user_service
        .is_first_user()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    
    Ok(Json(HasAdminResponse {
        has_admin: !is_first,
    }))
}

/// Response for has-admin check
#[derive(Debug, Serialize)]
pub struct HasAdminResponse {
    pub has_admin: bool,
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
    use crate::db::repositories::SqlxSettingsRepository;
    use crate::services::EmailService;
    use std::sync::Arc;

    // Check if email verification is enabled
    let settings_repo = Arc::new(SqlxSettingsRepository::new(state.pool.clone()));
    let email_service = EmailService::new(settings_repo.clone());
    
    if email_service.is_verification_enabled().await {
        // Verify the code
        let code = body.verification_code.as_ref()
            .ok_or_else(|| ApiError::validation_error("Verification code is required"))?;
        
        // Check code in database
        let valid = verify_email_code(&state.pool, &body.email, code).await
            .map_err(|e| ApiError::internal_error(e.to_string()))?;
        
        if !valid {
            return Err(ApiError::validation_error("Invalid or expired verification code"));
        }
        
        // Delete used code
        delete_email_code(&state.pool, &body.email).await
            .map_err(|e| ApiError::internal_error(e.to_string()))?;
    }

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
            UserServiceError::AuthenticationError(msg) => {
                // Check if it's a banned user error
                if msg.contains("banned") || msg.contains("封禁") {
                    ApiError::with_details("USER_BANNED", msg, serde_json::json!({}))
                } else {
                    ApiError::unauthorized("Invalid username or password")
                }
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

/// Request body for updating profile
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub avatar: Option<String>,
}

/// PUT /api/v1/auth/profile - Update current user's profile
async fn update_profile(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let mut current_user = user.0;
    
    // Update fields
    if let Some(display_name) = body.display_name {
        current_user.display_name = if display_name.trim().is_empty() {
            None
        } else {
            Some(display_name.trim().to_string())
        };
    }
    if let Some(avatar) = body.avatar {
        current_user.avatar = if avatar.trim().is_empty() {
            None
        } else {
            Some(avatar.trim().to_string())
        };
    }
    
    let updated = state
        .user_service
        .update_user(current_user)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    
    Ok(Json(updated.into()))
}

/// Request body for changing password
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// PUT /api/v1/auth/password - Change current user's password
async fn change_password(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::services::password::{hash_password, verify_password};
    
    // Validate new password
    if body.new_password.len() < 8 {
        return Err(ApiError::validation_error("Password must be at least 8 characters"));
    }
    
    // Verify current password
    let is_valid = verify_password(&body.current_password, &user.0.password_hash)
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    
    if !is_valid {
        return Err(ApiError::validation_error("Current password is incorrect"));
    }
    
    // Hash new password
    let new_hash = hash_password(&body.new_password)
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    
    // Update user
    let mut updated_user = user.0;
    updated_user.password_hash = new_hash;
    
    state
        .user_service
        .update_user(updated_user)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/auth/send-code - Send verification code to email
async fn send_verification_code(
    State(state): State<AppState>,
    Json(body): Json<SendCodeRequest>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::SqlxSettingsRepository;
    use crate::services::{EmailService, generate_verification_code};
    use std::sync::Arc;

    // Validate email format
    if !body.email.contains('@') {
        return Err(ApiError::validation_error("Invalid email format"));
    }

    // Check if email verification is enabled
    let settings_repo = Arc::new(SqlxSettingsRepository::new(state.pool.clone()));
    let email_service = EmailService::new(settings_repo);
    
    if !email_service.is_verification_enabled().await {
        return Err(ApiError::validation_error("Email verification is not enabled"));
    }

    // Check if email is already registered
    if let Some(_) = state.user_repo.get_by_email(&body.email).await
        .map_err(|e| ApiError::internal_error(e.to_string()))? 
    {
        return Err(ApiError::validation_error("Email is already registered"));
    }

    // Generate and save verification code
    let code = generate_verification_code();
    save_email_code(&state.pool, &body.email, &code).await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Send email
    email_service.send_verification_code(&body.email, &code).await
        .map_err(|e| ApiError::internal_error(format!("Failed to send email: {}", e)))?;

    Ok(Json(serde_json::json!({ "message": "Verification code sent" })))
}

// Helper functions for email verification codes

use crate::db::DynDatabasePool;
use crate::config::DatabaseDriver;
use anyhow::Result;
use chrono::{Duration, Utc};

async fn save_email_code(pool: &DynDatabasePool, email: &str, code: &str) -> Result<()> {
    let expires_at = Utc::now() + Duration::minutes(10);
    
    // Delete any existing codes for this email first
    delete_email_code(pool, email).await?;
    
    match pool.driver() {
        DatabaseDriver::Sqlite => {
            sqlx::query(
                "INSERT INTO email_verifications (email, code, expires_at, created_at) VALUES (?, ?, ?, ?)"
            )
            .bind(email)
            .bind(code)
            .bind(expires_at)
            .bind(Utc::now())
            .execute(pool.as_sqlite().unwrap())
            .await?;
        }
        DatabaseDriver::Mysql => {
            sqlx::query(
                "INSERT INTO email_verifications (email, code, expires_at, created_at) VALUES (?, ?, ?, ?)"
            )
            .bind(email)
            .bind(code)
            .bind(expires_at)
            .bind(Utc::now())
            .execute(pool.as_mysql().unwrap())
            .await?;
        }
    }
    Ok(())
}

async fn verify_email_code(pool: &DynDatabasePool, email: &str, code: &str) -> Result<bool> {
    let now = Utc::now();
    
    let count: i64 = match pool.driver() {
        DatabaseDriver::Sqlite => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM email_verifications WHERE email = ? AND code = ? AND expires_at > ?"
            )
            .bind(email)
            .bind(code)
            .bind(now)
            .fetch_one(pool.as_sqlite().unwrap())
            .await?
        }
        DatabaseDriver::Mysql => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM email_verifications WHERE email = ? AND code = ? AND expires_at > ?"
            )
            .bind(email)
            .bind(code)
            .bind(now)
            .fetch_one(pool.as_mysql().unwrap())
            .await?
        }
    };
    
    Ok(count > 0)
}

async fn delete_email_code(pool: &DynDatabasePool, email: &str) -> Result<()> {
    match pool.driver() {
        DatabaseDriver::Sqlite => {
            sqlx::query("DELETE FROM email_verifications WHERE email = ?")
                .bind(email)
                .execute(pool.as_sqlite().unwrap())
                .await?;
        }
        DatabaseDriver::Mysql => {
            sqlx::query("DELETE FROM email_verifications WHERE email = ?")
                .bind(email)
                .execute(pool.as_mysql().unwrap())
                .await?;
        }
    }
    Ok(())
}
