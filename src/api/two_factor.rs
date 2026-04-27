//! Two-Factor Authentication (TOTP) API endpoints
//!
//! Provides optional TOTP-based 2FA for admin accounts:
//! - POST /api/v1/auth/2fa/setup - Generate TOTP secret + QR code
//! - POST /api/v1/auth/2fa/enable - Verify code and enable 2FA
//! - POST /api/v1/auth/2fa/disable - Disable 2FA (requires password + code)
//! - POST /api/v1/auth/2fa/verify - Verify 2FA code during login
//! - GET  /api/v1/auth/2fa/status - Check if 2FA is enabled

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};
use crate::services::password::verify_password;
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use totp_rs::{Algorithm, Secret, TOTP};

/// Build the 2FA router (all routes require auth)
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/setup", post(setup_2fa))
        .route("/enable", post(enable_2fa))
        .route("/disable", post(disable_2fa))
        .route("/status", get(get_2fa_status))
}

/// Build the public 2FA router (for login verification)
pub fn public_router() -> Router<AppState> {
    Router::new().route("/verify", post(verify_2fa))
}

// ============================================================================
// Request/Response types
// ============================================================================

/// Response for 2FA setup (QR code + secret)
#[derive(Debug, Serialize)]
pub struct Setup2FAResponse {
    /// Base32-encoded TOTP secret for manual entry
    pub secret: String,
    /// QR code as data URI (PNG base64)
    pub qr_code: String,
}

/// Request to enable 2FA (verify first code)
#[derive(Debug, Deserialize)]
pub struct Enable2FARequest {
    /// TOTP code to verify setup is correct
    pub code: String,
}

/// Request to disable 2FA
#[derive(Debug, Deserialize)]
pub struct Disable2FARequest {
    /// Current password for security
    pub password: String,
    /// Current TOTP code
    pub code: String,
}

/// Request to verify 2FA during login
#[derive(Debug, Deserialize)]
pub struct Verify2FARequest {
    /// Challenge token from login response
    pub challenge_token: String,
    /// TOTP code
    pub code: String,
}

/// Response for 2FA status check
#[derive(Debug, Serialize)]
pub struct TwoFactorStatusResponse {
    pub enabled: bool,
}

/// Response for login that requires 2FA
#[derive(Debug, Serialize)]
pub struct TwoFactorChallengeResponse {
    pub requires_2fa: bool,
    pub challenge_token: String,
}

// ============================================================================
// Helper: create TOTP instance from secret
// ============================================================================

fn create_totp(secret_base32: &str, account_name: &str) -> Result<TOTP, ApiError> {
    let secret = Secret::Encoded(secret_base32.to_string());
    TOTP::new(
        Algorithm::SHA1,
        6,  // digits
        1,  // skew (allows ±1 time step)
        30, // step (30 seconds)
        secret
            .to_bytes()
            .map_err(|e| ApiError::internal_error(format!("Invalid secret: {}", e)))?,
        Some("Noteva".to_string()),
        account_name.to_string(),
    )
    .map_err(|e| ApiError::internal_error(format!("Failed to create TOTP: {}", e)))
}

// ============================================================================
// Endpoints
// ============================================================================

/// POST /api/v1/auth/2fa/setup - Generate TOTP secret and QR code
///
/// Generates a new secret but does NOT enable 2FA yet.
/// User must verify with a code via /2fa/enable.
async fn setup_2fa(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Setup2FAResponse>, ApiError> {
    // Generate random secret
    let secret = Secret::generate_secret();
    let secret_base32 = secret.to_encoded().to_string();

    // Create TOTP for QR code generation
    let totp = create_totp(&secret_base32, &user.0.username)?;

    // Generate QR code as data URI
    let qr_code = totp
        .get_qr_base64()
        .map_err(|e| ApiError::internal_error(format!("Failed to generate QR code: {}", e)))?;
    let qr_data_uri = format!("data:image/png;base64,{}", qr_code);

    // Store the secret temporarily on the user (not yet enabled)
    let mut updated_user = user.0;
    updated_user.totp_secret = Some(secret_base32.clone());
    // Keep totp_enabled as-is (don't enable yet)

    state
        .user_service
        .update_user(updated_user)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(Setup2FAResponse {
        secret: secret_base32,
        qr_code: qr_data_uri,
    }))
}

/// POST /api/v1/auth/2fa/enable - Verify code and enable 2FA
///
/// User must provide a valid TOTP code to confirm setup is working.
async fn enable_2fa(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<Enable2FARequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let secret = user
        .0
        .totp_secret
        .as_deref()
        .ok_or_else(|| ApiError::validation_error("2FA not set up. Call /2fa/setup first."))?;

    let totp = create_totp(secret, &user.0.username)?;

    // Verify the code
    let code = body.code.trim();
    if !totp
        .check_current(code)
        .map_err(|e| ApiError::internal_error(format!("TOTP check error: {}", e)))?
    {
        return Err(ApiError::validation_error(
            "Invalid verification code. Please try again.",
        ));
    }

    // Enable 2FA
    let mut updated_user = user.0;
    updated_user.totp_enabled = true;

    state
        .user_service
        .update_user(updated_user)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Two-factor authentication enabled successfully"
    })))
}

/// POST /api/v1/auth/2fa/disable - Disable 2FA
///
/// Requires both current password and a valid TOTP code for security.
async fn disable_2fa(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<Disable2FARequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !user.0.totp_enabled {
        return Err(ApiError::validation_error("2FA is not currently enabled."));
    }

    // Verify password
    let is_valid = verify_password(&body.password, &user.0.password_hash)
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    if !is_valid {
        return Err(ApiError::validation_error("Password is incorrect."));
    }

    // Verify TOTP code
    let secret = user
        .0
        .totp_secret
        .as_deref()
        .ok_or_else(|| ApiError::internal_error("2FA enabled but no secret stored"))?;
    let totp = create_totp(secret, &user.0.username)?;

    let code = body.code.trim();
    if !totp
        .check_current(code)
        .map_err(|e| ApiError::internal_error(format!("TOTP check error: {}", e)))?
    {
        return Err(ApiError::validation_error("Invalid verification code."));
    }

    // Disable 2FA and clear secret
    let mut updated_user = user.0;
    updated_user.totp_enabled = false;
    updated_user.totp_secret = None;

    state
        .user_service
        .update_user(updated_user)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Two-factor authentication disabled"
    })))
}

/// GET /api/v1/auth/2fa/status - Check if 2FA is enabled
async fn get_2fa_status(user: AuthenticatedUser) -> Json<TwoFactorStatusResponse> {
    Json(TwoFactorStatusResponse {
        enabled: user.0.totp_enabled,
    })
}

/// POST /api/v1/auth/2fa/verify - Verify 2FA code during login
///
/// Called after login returns `requires_2fa: true`.
/// Takes the challenge_token and TOTP code, returns a real session.
async fn verify_2fa(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Verify2FARequest>,
) -> Result<impl IntoResponse, ApiError> {
    let challenge = {
        let mut challenges = state.two_factor_challenges.write().await;
        let now = Instant::now();
        challenges.retain(|_, challenge| challenge.expires_at > now);
        challenges.get(&body.challenge_token).cloned()
    }
    .ok_or_else(|| ApiError::unauthorized("Invalid or expired challenge token"))?;

    let user = state
        .user_service
        .get_by_id(challenge.user_id)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?
        .ok_or_else(|| ApiError::unauthorized("Invalid or expired challenge token"))?;

    if !user.totp_enabled {
        return Err(ApiError::validation_error(
            "2FA is not enabled for this account",
        ));
    }

    let secret = user
        .totp_secret
        .as_deref()
        .ok_or_else(|| ApiError::internal_error("2FA enabled but no secret stored"))?;

    let totp = create_totp(secret, &user.username)?;

    let code = body.code.trim();
    if !totp
        .check_current(code)
        .map_err(|e| ApiError::internal_error(format!("TOTP check error: {}", e)))?
    {
        return Err(ApiError::unauthorized("Invalid verification code"));
    }

    {
        let mut challenges = state.two_factor_challenges.write().await;
        match challenges.get(&body.challenge_token) {
            Some(current) if current.user_id == user.id && current.expires_at > Instant::now() => {
                challenges.remove(&body.challenge_token);
            }
            _ => return Err(ApiError::unauthorized("Invalid or expired challenge token")),
        }
    }

    let session = state
        .user_service
        .create_login_session(&user, challenge.ip_address, challenge.user_agent)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // Detect HTTPS
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
        7 * 24 * 60 * 60,
        secure_flag,
    );
    let csrf_cookie = format!(
        "csrf_token={}; Path=/; SameSite=Lax; Max-Age={}{}",
        csrf_token,
        7 * 24 * 60 * 60,
        secure_flag,
    );

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&session_cookie).expect("session cookie"),
    );
    response_headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&csrf_cookie).expect("csrf cookie"),
    );

    let user_response = crate::api::auth::UserResponse::from(user);

    Ok((
        response_headers,
        Json(crate::api::auth::AuthResponse {
            user: user_response,
            token: session.id,
        }),
    ))
}
