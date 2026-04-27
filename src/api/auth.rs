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

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};
use crate::config::DatabaseDriver;
use crate::db::DynDatabasePool;
use crate::services::user::{LoginInput, RegisterInput, UserServiceError};
use axum::{
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

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
    pub status: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub totp_enabled: bool,
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
            totp_enabled: user.totp_enabled,
            created_at: user.created_at.to_rfc3339(),
        }
    }
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
        .route("/has-admin", get(has_admin))
}

/// GET /api/v1/auth/has-admin - Check if admin exists
///
/// Returns whether the system has at least one admin user.
/// Used for first-time setup flow.
async fn has_admin(State(state): State<AppState>) -> Result<Json<HasAdminResponse>, ApiError> {
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
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Check if admin already exists - block registration if true (security measure)
    let is_first = state
        .user_service
        .is_first_user()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if !is_first {
        return Err(ApiError::forbidden(
            "Admin account already exists, registration is closed",
        ));
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
    let ip_addr = Some(addr.ip().to_string());
    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(String::from);
    let session = state
        .user_service
        .login(login_input, ip_addr, ua)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Detect if behind HTTPS
    let is_secure = headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .map(|p| p == "https")
        .unwrap_or(false);
    let secure_flag = if is_secure { "; Secure" } else { "" };

    // Generate CSRF token
    let csrf_token = crate::api::middleware::generate_csrf_token();

    // Set session cookie
    let session_cookie = format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        session.id,
        7 * 24 * 60 * 60, // 7 days
        secure_flag,
    );
    // CSRF cookie (NOT httpOnly — JS needs to read it)
    let csrf_cookie = format!(
        "csrf_token={}; Path=/; SameSite=Lax; Max-Age={}{}",
        csrf_token,
        7 * 24 * 60 * 60,
        secure_flag,
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&session_cookie)
            .map_err(|_| ApiError::internal_error("Failed to build session cookie"))?,
    );
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&csrf_cookie)
            .map_err(|_| ApiError::internal_error("Failed to build CSRF cookie"))?,
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
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    // Extract IP address and User-Agent
    let ip_address = Some(addr.ip().to_string());
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(String::from);

    // Check IP rate limit (10 requests per minute)
    if let Some(ip) = ip_address.as_ref().and_then(|s| s.parse().ok()) {
        if state.rate_limiter.is_ip_limited(ip).await {
            log_login_attempt(
                &state.pool,
                &body.username_or_email,
                ip_address.as_deref(),
                user_agent.as_deref(),
                false,
                Some("IP rate limit exceeded"),
            )
            .await;
            return Err(ApiError::with_details(
                "RATE_LIMIT",
                "Too many requests, please try again later",
                serde_json::json!({"retry_after": 60}),
            ));
        }
        state.rate_limiter.record_ip_request(ip).await;
    }

    // Check username rate limit (progressive tiers)
    if let Some(lockout_secs) = state
        .rate_limiter
        .check_username_limit(&body.username_or_email)
        .await
    {
        log_login_attempt(
            &state.pool,
            &body.username_or_email,
            ip_address.as_deref(),
            user_agent.as_deref(),
            false,
            Some("Username rate limit exceeded"),
        )
        .await;
        return Err(ApiError::with_details(
            "RATE_LIMIT",
            "Too many failed login attempts, please try again later",
            serde_json::json!({"retry_after": lockout_secs}),
        ));
    }

    let input = LoginInput::new(body.username_or_email.clone(), body.password);

    let user = state
        .user_service
        .authenticate_login(&input, ip_address.clone())
        .await
        .map_err(|e| {
            // Record failed attempt
            let username = body.username_or_email.clone();
            let ip = ip_address.clone();
            let ua = user_agent.clone();
            let pool = state.pool.clone();
            let limiter = state.rate_limiter.clone();

            // Determine error type before moving
            let is_banned = matches!(&e, UserServiceError::AuthenticationError(msg) if msg.contains("banned") || msg.contains("封禁"));
            let is_auth_error = matches!(&e, UserServiceError::AuthenticationError(_));

            tokio::spawn(async move {
                limiter.record_failed_attempt(&username).await;
                let reason = if is_banned {
                    "User banned"
                } else if is_auth_error {
                    "Invalid credentials"
                } else {
                    "Unknown error"
                };
                log_login_attempt(&pool, &username, ip.as_deref(), ua.as_deref(), false, Some(reason)).await;
            });

            match e {
                UserServiceError::AuthenticationError(msg) => {
                    // Check if it's a banned user error
                    if msg.contains("banned") || msg.contains("封禁") {
                        ApiError::with_details("USER_BANNED", msg, serde_json::json!({}))
                    } else {
                        ApiError::unauthorized("Invalid username or password")
                    }
                }
                _ => ApiError::internal_error("Login failed"),
            }
        })?;

    // Check if 2FA is enabled — return challenge instead of full login
    if user.totp_enabled {
        let challenge_token = crate::api::middleware::generate_csrf_token();
        {
            let mut challenges = state.two_factor_challenges.write().await;
            let now = Instant::now();
            challenges.retain(|_, challenge| challenge.expires_at > now);
            challenges.insert(
                challenge_token.clone(),
                crate::api::middleware::TwoFactorLoginChallenge {
                    user_id: user.id,
                    expires_at: now + Duration::from_secs(5 * 60),
                    ip_address: ip_address.clone(),
                    user_agent: user_agent.clone(),
                },
            );
        }

        // Clear failed attempts (password was correct)
        state
            .rate_limiter
            .clear_username_attempts(&body.username_or_email)
            .await;
        log_login_attempt(
            &state.pool,
            &body.username_or_email,
            ip_address.as_deref(),
            user_agent.as_deref(),
            true,
            Some("2FA challenge issued"),
        )
        .await;

        // Don't set cookies yet — user must complete 2FA first
        return Ok((
            StatusCode::ACCEPTED,
            Json(crate::api::two_factor::TwoFactorChallengeResponse {
                requires_2fa: true,
                challenge_token,
            }),
        )
            .into_response());
    }

    // Clear failed attempts on successful login
    state
        .rate_limiter
        .clear_username_attempts(&body.username_or_email)
        .await;

    // Log successful login
    log_login_attempt(
        &state.pool,
        &body.username_or_email,
        ip_address.as_deref(),
        user_agent.as_deref(),
        true,
        None,
    )
    .await;

    // Detect if behind HTTPS (for Secure flag on cookies)
    let is_secure = headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .map(|p| p == "https")
        .unwrap_or(false);
    let secure_flag = if is_secure { "; Secure" } else { "" };

    // Generate CSRF token
    let csrf_token = crate::api::middleware::generate_csrf_token();

    let session = state
        .user_service
        .create_login_session(&user, ip_address, user_agent)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Set session cookie (httpOnly for security)
    let session_cookie = format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        session.id,
        7 * 24 * 60 * 60, // 7 days
        secure_flag,
    );
    // CSRF cookie (NOT httpOnly — JS needs to read it)
    let csrf_cookie = format!(
        "csrf_token={}; Path=/; SameSite=Lax; Max-Age={}{}",
        csrf_token,
        7 * 24 * 60 * 60,
        secure_flag,
    );

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&session_cookie)
            .map_err(|_| ApiError::internal_error("Failed to build session cookie"))?,
    );
    response_headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&csrf_cookie)
            .map_err(|_| ApiError::internal_error("Failed to build CSRF cookie"))?,
    );

    Ok((
        response_headers,
        Json(AuthResponse {
            user: user.into(),
            token: session.id,
        }),
    )
        .into_response())
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
        .logout(token, Some(_user.0.id))
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Clear the session cookie and CSRF cookie
    let clear_session = "session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
    let clear_csrf = "csrf_token=; Path=/; SameSite=Lax; Max-Age=0";
    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::SET_COOKIE, HeaderValue::from_static(clear_session));
    response_headers.append(header::SET_COOKIE, HeaderValue::from_static(clear_csrf));

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
        return Err(ApiError::validation_error(
            "Password must be at least 8 characters",
        ));
    }

    // Verify current password
    let is_valid = verify_password(&body.current_password, &user.0.password_hash)
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if !is_valid {
        return Err(ApiError::validation_error("Current password is incorrect"));
    }

    // Hash new password
    let new_hash =
        hash_password(&body.new_password).map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Update user
    let user_id = user.0.id;
    let mut updated_user = user.0;
    updated_user.password_hash = new_hash;

    state
        .user_service
        .update_user(updated_user)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Hook: user_password_change
    state.hook_manager.trigger(
        "user_password_change",
        serde_json::json!({ "user_id": user_id }),
    );

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Helper Functions for Security
// ============================================================================

/// Log login attempt to database for security auditing
async fn log_login_attempt(
    pool: &DynDatabasePool,
    username: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    success: bool,
    failure_reason: Option<&str>,
) {
    let success_int = if success { 1 } else { 0 };

    let result: Result<(), sqlx::Error> = match pool.driver() {
        DatabaseDriver::Sqlite => {
            sqlx::query(
                "INSERT INTO login_logs (username, ip_address, user_agent, success, failure_reason) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(username)
            .bind(ip_address)
            .bind(user_agent)
            .bind(success_int)
            .bind(failure_reason)
            .execute(pool.as_sqlite().expect("expected sqlite pool"))
            .await
            .map(|_| ())
        }
        DatabaseDriver::Mysql => {
            sqlx::query(
                "INSERT INTO login_logs (username, ip_address, user_agent, success, failure_reason) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(username)
            .bind(ip_address)
            .bind(user_agent)
            .bind(success_int)
            .bind(failure_reason)
            .execute(pool.as_mysql().expect("expected mysql pool"))
            .await
            .map(|_| ())
        }
    };

    if let Err(e) = result {
        tracing::warn!("Failed to log login attempt: {}", e);
    }
}
