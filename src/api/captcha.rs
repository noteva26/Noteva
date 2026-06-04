//! Public captcha configuration and verification helpers.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::api::middleware::{ApiError, AppState};

const PROVIDER_NONE: &str = "none";
const PROVIDER_TURNSTILE: &str = "turnstile";
const PROVIDER_HCAPTCHA: &str = "hcaptcha";

#[derive(Debug, Clone, Serialize)]
pub struct CaptchaConfigResponse {
    pub enabled: bool,
    pub provider: String,
    pub site_key: String,
}

#[derive(Debug, Deserialize)]
struct CaptchaVerifyResponse {
    success: bool,
}

pub async fn get_config(State(state): State<AppState>) -> Json<CaptchaConfigResponse> {
    Json(read_public_config(&state).await)
}

pub async fn read_public_config(state: &AppState) -> CaptchaConfigResponse {
    let provider = read_setting(state, "captcha_provider")
        .await
        .unwrap_or_else(|| PROVIDER_NONE.to_string())
        .trim()
        .to_ascii_lowercase();
    let site_key = read_setting(state, "captcha_site_key")
        .await
        .unwrap_or_default();
    let secret_key = read_setting(state, "captcha_secret_key")
        .await
        .unwrap_or_default();

    let enabled = matches!(provider.as_str(), PROVIDER_TURNSTILE | PROVIDER_HCAPTCHA)
        && !site_key.trim().is_empty()
        && !secret_key.trim().is_empty();

    CaptchaConfigResponse {
        enabled,
        provider: if enabled {
            provider
        } else {
            PROVIDER_NONE.to_string()
        },
        site_key: if enabled { site_key } else { String::new() },
    }
}

pub async fn verify_comment_token(
    state: &AppState,
    token: Option<&str>,
    remote_ip: Option<&str>,
) -> Result<(), ApiError> {
    let config = read_public_config(state).await;
    if !config.enabled {
        return Ok(());
    }

    let token = token
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::validation_error("Captcha token is required"))?;
    let secret = read_setting(state, "captcha_secret_key")
        .await
        .unwrap_or_default();

    let verify_url = match config.provider.as_str() {
        PROVIDER_TURNSTILE => "https://challenges.cloudflare.com/turnstile/v0/siteverify",
        PROVIDER_HCAPTCHA => "https://hcaptcha.com/siteverify",
        _ => return Ok(()),
    };

    let client = reqwest::Client::new();
    let mut form = vec![("secret", secret.as_str()), ("response", token)];
    if let Some(ip) = remote_ip.filter(|value| !value.trim().is_empty()) {
        form.push(("remoteip", ip));
    }

    let response = client
        .post(verify_url)
        .form(&form)
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("Captcha verification failed: {}", e)))?;

    let result = response
        .json::<CaptchaVerifyResponse>()
        .await
        .map_err(|e| ApiError::internal_error(format!("Invalid captcha response: {}", e)))?;

    if result.success {
        Ok(())
    } else {
        Err(ApiError::validation_error("Captcha verification failed"))
    }
}

async fn read_setting(state: &AppState, key: &str) -> Option<String> {
    state.settings_service.get(key).await.ok().flatten()
}
