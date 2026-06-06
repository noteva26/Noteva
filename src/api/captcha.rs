//! Public captcha configuration and verification helpers.

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::api::middleware::{extract_client_ip, ApiError, AppState};
use crate::services::captcha_pow::{CaptchaPowChallenge, CaptchaPowDifficulty, CaptchaPowError};

const PROVIDER_NONE: &str = "none";
const PROVIDER_NOTEVA_POW: &str = "noteva_pow";
const PROVIDER_TURNSTILE: &str = "turnstile";
const PROVIDER_HCAPTCHA: &str = "hcaptcha";
const PROVIDER_CAP: &str = "cap";
const CAPTCHA_ACTION_COMMENT: &str = "comment";
const DEFAULT_CAP_BASE_URL: &str = "https://captcha.noteva.org";
const DEFAULT_CAP_SITE_KEY: &str = "4d0a333fd4";
const DEFAULT_CAP_SECRET_KEY: &str = "sk-P4HovV1v4cRlLpQiKvQC3bZv5i74UTHaIA0dWKWrLc";

#[derive(Debug, Clone, Serialize)]
pub struct CaptchaConfigResponse {
    pub enabled: bool,
    pub provider: String,
    pub site_key: String,
    pub cap_base_url: String,
    pub cap_endpoint: String,
    pub pow: Option<CaptchaPowConfigResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptchaPowConfigResponse {
    pub difficulty: String,
    pub leading_zero_bits: u8,
    pub challenge_ttl_seconds: u64,
    pub token_ttl_seconds: u64,
    pub auto_solve: bool,
}

#[derive(Debug, Deserialize)]
struct CaptchaVerifyResponse {
    success: bool,
}

#[derive(Debug, Serialize)]
struct CaptchaCapVerifyRequest<'a> {
    secret: &'a str,
    response: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct CaptchaChallengeRequest {
    pub action: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CaptchaChallengeResponse {
    pub challenge: CaptchaPowChallenge,
}

#[derive(Debug, Deserialize)]
pub struct CaptchaPowVerifyRequest {
    pub action: Option<String>,
    pub challenge_id: String,
    pub solution: String,
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CaptchaPowVerifyResponse {
    pub token: String,
    pub action: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_config(State(state): State<AppState>) -> Json<CaptchaConfigResponse> {
    Json(read_public_config(&state).await)
}

pub async fn read_public_config(state: &AppState) -> CaptchaConfigResponse {
    let configured_provider = read_setting(state, "captcha_provider")
        .await
        .unwrap_or_else(|| PROVIDER_CAP.to_string())
        .trim()
        .to_ascii_lowercase();
    let provider = if configured_provider == PROVIDER_NOTEVA_POW {
        PROVIDER_CAP.to_string()
    } else {
        configured_provider
    };
    let site_key = read_setting(state, "captcha_site_key")
        .await
        .unwrap_or_default();
    let secret_key = effective_cap_secret_key(
        &provider,
        &read_setting(state, "captcha_secret_key")
            .await
            .unwrap_or_default(),
    );
    let cap_base_url = read_setting(state, "captcha_cap_base_url")
        .await
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CAP_BASE_URL.to_string());

    let pow_config = read_pow_config(state).await;
    let provider_enabled = match provider.as_str() {
        PROVIDER_NOTEVA_POW => true,
        PROVIDER_CAP => {
            let effective_site_key = effective_cap_site_key(&site_key);
            !effective_site_key.is_empty()
                && !secret_key.trim().is_empty()
                && !normalize_cap_base_url(&cap_base_url).is_empty()
        }
        PROVIDER_TURNSTILE | PROVIDER_HCAPTCHA => {
            !site_key.trim().is_empty() && !secret_key.trim().is_empty()
        }
        _ => false,
    };

    let enabled = provider_enabled;

    let public_provider = if enabled {
        provider.clone()
    } else {
        PROVIDER_NONE.to_string()
    };
    let public_site_key = if enabled {
        if provider == PROVIDER_CAP {
            effective_cap_site_key(&site_key)
        } else {
            site_key
        }
    } else {
        String::new()
    };
    let public_cap_base_url = if enabled && provider == PROVIDER_CAP {
        normalize_cap_base_url(&cap_base_url)
    } else {
        String::new()
    };
    let public_cap_endpoint = if enabled && provider == PROVIDER_CAP {
        build_cap_widget_endpoint(&public_cap_base_url, &public_site_key)
    } else {
        String::new()
    };
    let public_pow = if enabled && provider == PROVIDER_NOTEVA_POW {
        Some(pow_config)
    } else {
        None
    };

    CaptchaConfigResponse {
        enabled,
        provider: public_provider,
        site_key: public_site_key,
        cap_base_url: public_cap_base_url,
        cap_endpoint: public_cap_endpoint,
        pow: public_pow,
    }
}

pub async fn create_challenge(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<CaptchaChallengeRequest>,
) -> Result<Json<CaptchaChallengeResponse>, ApiError> {
    let config = read_public_config(&state).await;
    if !config.enabled || config.provider != PROVIDER_NOTEVA_POW {
        return Err(ApiError::validation_error(
            "Built-in captcha is not enabled",
        ));
    }

    let pow = config.pow.unwrap_or_else(|| default_pow_config(None));
    let difficulty = CaptchaPowDifficulty::from_setting(Some(&pow.difficulty));
    let action = req.action.as_deref().unwrap_or(CAPTCHA_ACTION_COMMENT);
    let client_ip = extract_client_ip(&headers, addr);
    let challenge = state
        .captcha_pow_store
        .issue_challenge(
            action,
            difficulty,
            pow.challenge_ttl_seconds,
            Some(client_ip),
        )
        .await
        .map_err(pow_error_to_api_error)?;

    Ok(Json(CaptchaChallengeResponse { challenge }))
}

pub async fn verify_pow(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<CaptchaPowVerifyRequest>,
) -> Result<Json<CaptchaPowVerifyResponse>, ApiError> {
    let config = read_public_config(&state).await;
    if !config.enabled || config.provider != PROVIDER_NOTEVA_POW {
        return Err(ApiError::validation_error(
            "Built-in captcha is not enabled",
        ));
    }

    let pow = config.pow.unwrap_or_else(|| default_pow_config(None));
    let action = req.action.as_deref().unwrap_or(CAPTCHA_ACTION_COMMENT);
    let client_ip = extract_client_ip(&headers, addr);
    let token = state
        .captcha_pow_store
        .verify_solution(
            &req.challenge_id,
            action,
            &req.solution,
            pow.token_ttl_seconds,
            Some(client_ip),
        )
        .await
        .map_err(pow_error_to_api_error)?;

    Ok(Json(CaptchaPowVerifyResponse {
        token: token.token,
        action: token.action,
        expires_at: token.expires_at,
    }))
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

    if config.provider == PROVIDER_NOTEVA_POW {
        return state
            .captcha_pow_store
            .consume_token(token, CAPTCHA_ACTION_COMMENT, remote_ip)
            .await
            .map_err(pow_error_to_api_error);
    }

    let secret = effective_cap_secret_key(
        &config.provider,
        &read_setting(state, "captcha_secret_key")
            .await
            .unwrap_or_default(),
    );
    if secret.trim().is_empty() {
        return Err(ApiError::validation_error("Captcha secret key is required"));
    }

    if config.provider == PROVIDER_CAP {
        let cap_base_url = read_setting(state, "captcha_cap_base_url")
            .await
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_CAP_BASE_URL.to_string());
        return verify_cap_token(&cap_base_url, &config.site_key, &secret, token).await;
    }

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

async fn verify_cap_token(
    base_url: &str,
    site_key: &str,
    secret: &str,
    token: &str,
) -> Result<(), ApiError> {
    let verify_urls = build_cap_verify_urls(base_url, site_key);
    if verify_urls.is_empty() {
        return Err(ApiError::validation_error("Cap base URL is invalid"));
    }

    let client = reqwest::Client::new();
    let body = CaptchaCapVerifyRequest {
        secret: secret.trim(),
        response: token,
    };

    let mut last_error = None;
    for verify_url in verify_urls {
        let response = client
            .post(&verify_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::internal_error(format!("Captcha verification failed: {}", e)))?;

        let status = response.status();
        if matches!(
            status,
            StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
        ) {
            last_error = Some(format!("Cap verification endpoint returned {}", status));
            continue;
        }

        let result = response
            .json::<CaptchaVerifyResponse>()
            .await
            .map_err(|e| ApiError::internal_error(format!("Invalid captcha response: {}", e)))?;

        return if result.success {
            Ok(())
        } else {
            Err(ApiError::validation_error("Captcha verification failed"))
        };
    }

    Err(ApiError::internal_error(last_error.unwrap_or_else(|| {
        "Captcha verification failed".to_string()
    })))
}

async fn read_setting(state: &AppState, key: &str) -> Option<String> {
    state.settings_service.get(key).await.ok().flatten()
}

fn effective_cap_site_key(site_key: &str) -> String {
    let site_key = site_key.trim();
    if site_key.is_empty() {
        DEFAULT_CAP_SITE_KEY.to_string()
    } else {
        site_key.to_string()
    }
}

fn effective_cap_secret_key(provider: &str, secret_key: &str) -> String {
    let secret_key = secret_key.trim();
    if provider == PROVIDER_CAP && secret_key.is_empty() {
        DEFAULT_CAP_SECRET_KEY.to_string()
    } else {
        secret_key.to_string()
    }
}

fn normalize_cap_base_url(base_url: &str) -> String {
    let value = base_url.trim().trim_end_matches('/').to_string();
    if value.starts_with("http://") || value.starts_with("https://") {
        value
    } else {
        String::new()
    }
}

fn build_cap_widget_endpoint(base_url: &str, site_key: &str) -> String {
    let base_url = normalize_cap_base_url(base_url);
    let site_key = site_key.trim().trim_matches('/');
    if base_url.is_empty() || site_key.is_empty() {
        return String::new();
    }
    format!("{}/{}/", base_url, site_key)
}

fn build_cap_verify_urls(base_url: &str, site_key: &str) -> Vec<String> {
    let base_url = normalize_cap_base_url(base_url);
    if base_url.is_empty() {
        return Vec::new();
    }

    let mut urls = vec![format!("{}/siteverify", base_url)];
    let site_key = site_key.trim().trim_matches('/');
    if !site_key.is_empty() {
        urls.push(format!("{}/{}/siteverify", base_url, site_key));
    }
    urls
}

async fn read_pow_config(state: &AppState) -> CaptchaPowConfigResponse {
    let difficulty = read_setting(state, "captcha_pow_difficulty").await;
    let challenge_ttl_seconds = read_setting(state, "captcha_challenge_ttl_seconds")
        .await
        .and_then(|value| value.parse::<u64>().ok());
    let token_ttl_seconds = read_setting(state, "captcha_token_ttl_seconds")
        .await
        .and_then(|value| value.parse::<u64>().ok());
    let auto_solve = read_setting(state, "captcha_pow_auto_solve")
        .await
        .map(|value| value != "false")
        .unwrap_or(true);

    let mut config = default_pow_config(difficulty.as_deref());
    if let Some(seconds) = challenge_ttl_seconds {
        config.challenge_ttl_seconds = seconds.clamp(30, 600);
    }
    if let Some(seconds) = token_ttl_seconds {
        config.token_ttl_seconds = seconds.clamp(60, 1800);
    }
    config.auto_solve = auto_solve;
    config
}

fn default_pow_config(difficulty: Option<&str>) -> CaptchaPowConfigResponse {
    let difficulty = CaptchaPowDifficulty::from_setting(difficulty);
    CaptchaPowConfigResponse {
        difficulty: difficulty.as_str().to_string(),
        leading_zero_bits: difficulty.leading_zero_bits(),
        challenge_ttl_seconds: 120,
        token_ttl_seconds: 300,
        auto_solve: true,
    }
}

fn pow_error_to_api_error(error: CaptchaPowError) -> ApiError {
    match error {
        CaptchaPowError::RateLimited => {
            ApiError::new("RATE_LIMIT", "Too many captcha challenges requested")
        }
        CaptchaPowError::ChallengeNotFound
        | CaptchaPowError::ChallengeExpired
        | CaptchaPowError::InvalidSolution
        | CaptchaPowError::TokenRequired
        | CaptchaPowError::TokenNotFound
        | CaptchaPowError::TokenExpired
        | CaptchaPowError::ActionMismatch
        | CaptchaPowError::ClientMismatch
        | CaptchaPowError::InvalidAction => ApiError::validation_error(error.to_string()),
        CaptchaPowError::Random => ApiError::internal_error(error.to_string()),
    }
}
