//! Lightweight proof-of-work captcha state.
//!
//! The built-in captcha stores short-lived challenges and one-time tokens in
//! memory. A process restart invalidates both, which is acceptable for captcha
//! state and avoids adding database migrations for ephemeral data.

use chrono::{DateTime, Duration, Utc};
use getrandom::fill as fill_random;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tokio::sync::RwLock;

const DEFAULT_MAX_CHALLENGES_PER_MINUTE: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptchaPowDifficulty {
    Low,
    Normal,
    High,
}

impl CaptchaPowDifficulty {
    pub fn from_setting(value: Option<&str>) -> Self {
        match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("high") => Self::High,
            Some("low") => Self::Low,
            _ => Self::Normal,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
        }
    }

    pub fn leading_zero_bits(self) -> u8 {
        match self {
            Self::Low => 14,
            Self::Normal => 16,
            Self::High => 18,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptchaPowChallenge {
    pub id: String,
    pub action: String,
    pub nonce: String,
    pub algorithm: String,
    pub difficulty: String,
    pub leading_zero_bits: u8,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptchaPowToken {
    pub token: String,
    pub action: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct StoredChallenge {
    action: String,
    nonce: String,
    leading_zero_bits: u8,
    expires_at: DateTime<Utc>,
    client_ip: Option<String>,
}

#[derive(Debug, Clone)]
struct StoredToken {
    action: String,
    expires_at: DateTime<Utc>,
    client_ip: Option<String>,
}

#[derive(Debug, Default)]
pub struct CaptchaPowStore {
    challenges: RwLock<HashMap<String, StoredChallenge>>,
    tokens: RwLock<HashMap<String, StoredToken>>,
    challenge_requests: RwLock<HashMap<String, Vec<DateTime<Utc>>>>,
}

impl CaptchaPowStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn issue_challenge(
        &self,
        action: &str,
        difficulty: CaptchaPowDifficulty,
        ttl_seconds: u64,
        client_ip: Option<String>,
    ) -> Result<CaptchaPowChallenge, CaptchaPowError> {
        let action = normalize_action(action)?;
        if let Some(ref ip) = client_ip {
            self.check_rate_limit(ip).await?;
        }

        self.cleanup_expired().await;

        let id = random_hex(16)?;
        let nonce = random_hex(16)?;
        let expires_at = Utc::now() + Duration::seconds(clamp_ttl(ttl_seconds, 30, 600) as i64);
        let leading_zero_bits = difficulty.leading_zero_bits();

        self.challenges.write().await.insert(
            id.clone(),
            StoredChallenge {
                action: action.clone(),
                nonce: nonce.clone(),
                leading_zero_bits,
                expires_at,
                client_ip: client_ip.clone(),
            },
        );

        Ok(CaptchaPowChallenge {
            id,
            action,
            nonce,
            algorithm: "sha256".to_string(),
            difficulty: difficulty.as_str().to_string(),
            leading_zero_bits,
            expires_at,
        })
    }

    pub async fn verify_solution(
        &self,
        challenge_id: &str,
        action: &str,
        solution: &str,
        token_ttl_seconds: u64,
        client_ip: Option<String>,
    ) -> Result<CaptchaPowToken, CaptchaPowError> {
        let action = normalize_action(action)?;
        let challenge_id = challenge_id.trim();
        let solution = solution.trim();
        if challenge_id.is_empty() || solution.is_empty() {
            return Err(CaptchaPowError::InvalidSolution);
        }

        self.cleanup_expired().await;

        let challenge = self
            .challenges
            .write()
            .await
            .remove(challenge_id)
            .ok_or(CaptchaPowError::ChallengeNotFound)?;

        if challenge.expires_at <= Utc::now() {
            return Err(CaptchaPowError::ChallengeExpired);
        }

        if challenge.action != action {
            return Err(CaptchaPowError::ActionMismatch);
        }

        if !same_client(challenge.client_ip.as_deref(), client_ip.as_deref()) {
            return Err(CaptchaPowError::ClientMismatch);
        }

        if !solution_meets_difficulty(
            challenge_id,
            &challenge.nonce,
            &challenge.action,
            solution,
            challenge.leading_zero_bits,
        ) {
            return Err(CaptchaPowError::InvalidSolution);
        }

        let token = random_hex(32)?;
        let expires_at =
            Utc::now() + Duration::seconds(clamp_ttl(token_ttl_seconds, 60, 1800) as i64);

        self.tokens.write().await.insert(
            token.clone(),
            StoredToken {
                action: action.clone(),
                expires_at,
                client_ip,
            },
        );

        Ok(CaptchaPowToken {
            token,
            action,
            expires_at,
        })
    }

    pub async fn consume_token(
        &self,
        token: &str,
        action: &str,
        client_ip: Option<&str>,
    ) -> Result<(), CaptchaPowError> {
        let action = normalize_action(action)?;
        let token = token.trim();
        if token.is_empty() {
            return Err(CaptchaPowError::TokenRequired);
        }

        self.cleanup_expired().await;

        let stored = self
            .tokens
            .write()
            .await
            .remove(token)
            .ok_or(CaptchaPowError::TokenNotFound)?;

        if stored.expires_at <= Utc::now() {
            return Err(CaptchaPowError::TokenExpired);
        }

        if stored.action != action {
            return Err(CaptchaPowError::ActionMismatch);
        }

        if !same_client(stored.client_ip.as_deref(), client_ip) {
            return Err(CaptchaPowError::ClientMismatch);
        }

        Ok(())
    }

    async fn check_rate_limit(&self, client_ip: &str) -> Result<(), CaptchaPowError> {
        let now = Utc::now();
        let cutoff = now - Duration::minutes(1);
        let mut requests = self.challenge_requests.write().await;
        let entries = requests
            .entry(client_ip.to_string())
            .or_insert_with(Vec::new);
        entries.retain(|time| *time > cutoff);

        if entries.len() >= DEFAULT_MAX_CHALLENGES_PER_MINUTE {
            return Err(CaptchaPowError::RateLimited);
        }

        entries.push(now);
        Ok(())
    }

    async fn cleanup_expired(&self) {
        let now = Utc::now();
        self.challenges
            .write()
            .await
            .retain(|_, challenge| challenge.expires_at > now);
        self.tokens
            .write()
            .await
            .retain(|_, token| token.expires_at > now);
        self.challenge_requests.write().await.retain(|_, entries| {
            entries.retain(|time| *time > now - Duration::minutes(1));
            !entries.is_empty()
        });
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CaptchaPowError {
    #[error("Captcha action is invalid")]
    InvalidAction,
    #[error("Too many captcha challenges requested")]
    RateLimited,
    #[error("Captcha challenge was not found")]
    ChallengeNotFound,
    #[error("Captcha challenge has expired")]
    ChallengeExpired,
    #[error("Captcha solution is invalid")]
    InvalidSolution,
    #[error("Captcha token is required")]
    TokenRequired,
    #[error("Captcha token was not found")]
    TokenNotFound,
    #[error("Captcha token has expired")]
    TokenExpired,
    #[error("Captcha action does not match")]
    ActionMismatch,
    #[error("Captcha client does not match")]
    ClientMismatch,
    #[error("Failed to generate captcha secret")]
    Random,
}

fn normalize_action(action: &str) -> Result<String, CaptchaPowError> {
    let action = action.trim().to_ascii_lowercase();
    if action == "comment" {
        Ok(action)
    } else {
        Err(CaptchaPowError::InvalidAction)
    }
}

fn clamp_ttl(value: u64, min: u64, max: u64) -> u64 {
    value.clamp(min, max)
}

fn same_client(expected: Option<&str>, actual: Option<&str>) -> bool {
    match (expected, actual) {
        (Some(expected), Some(actual)) => expected == actual,
        (Some(_), None) => false,
        _ => true,
    }
}

fn random_hex(bytes_len: usize) -> Result<String, CaptchaPowError> {
    let mut bytes = vec![0u8; bytes_len];
    fill_random(&mut bytes).map_err(|_| CaptchaPowError::Random)?;
    Ok(hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn solution_meets_difficulty(
    challenge_id: &str,
    nonce: &str,
    action: &str,
    solution: &str,
    leading_zero_bits: u8,
) -> bool {
    let payload = format!("{challenge_id}:{nonce}:{action}:{solution}");
    let digest = Sha256::digest(payload.as_bytes());
    has_leading_zero_bits(&digest, leading_zero_bits)
}

fn has_leading_zero_bits(bytes: &[u8], bits: u8) -> bool {
    let mut remaining = bits;
    for byte in bytes {
        if remaining == 0 {
            return true;
        }

        if remaining >= 8 {
            if *byte != 0 {
                return false;
            }
            remaining -= 8;
            continue;
        }

        let mask = 0xff << (8 - remaining);
        return byte & mask == 0;
    }

    remaining == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_solution(id: &str, nonce: &str, action: &str, bits: u8) -> String {
        for candidate in 0u64.. {
            let candidate = candidate.to_string();
            if solution_meets_difficulty(id, nonce, action, &candidate, bits) {
                return candidate;
            }
        }
        unreachable!()
    }

    #[test]
    fn difficulty_rejects_bad_solution() {
        assert!(!solution_meets_difficulty(
            "id", "nonce", "comment", "0", 32
        ));
    }

    #[tokio::test]
    async fn token_can_only_be_consumed_once() {
        let store = CaptchaPowStore::new();
        let challenge = store
            .issue_challenge(
                "comment",
                CaptchaPowDifficulty::Low,
                120,
                Some("127.0.0.1".to_string()),
            )
            .await
            .unwrap();
        let solution = find_solution(
            &challenge.id,
            &challenge.nonce,
            &challenge.action,
            challenge.leading_zero_bits,
        );
        let token = store
            .verify_solution(
                &challenge.id,
                "comment",
                &solution,
                300,
                Some("127.0.0.1".to_string()),
            )
            .await
            .unwrap();

        assert!(store
            .consume_token(&token.token, "comment", Some("127.0.0.1"))
            .await
            .is_ok());
        assert!(matches!(
            store
                .consume_token(&token.token, "comment", Some("127.0.0.1"))
                .await,
            Err(CaptchaPowError::TokenNotFound)
        ));
    }

    #[tokio::test]
    async fn challenge_solution_cannot_be_replayed() {
        let store = CaptchaPowStore::new();
        let challenge = store
            .issue_challenge("comment", CaptchaPowDifficulty::Low, 120, None)
            .await
            .unwrap();
        let solution = find_solution(
            &challenge.id,
            &challenge.nonce,
            &challenge.action,
            challenge.leading_zero_bits,
        );

        assert!(store
            .verify_solution(&challenge.id, "comment", &solution, 300, None)
            .await
            .is_ok());
        assert!(matches!(
            store
                .verify_solution(&challenge.id, "comment", &solution, 300, None)
                .await,
            Err(CaptchaPowError::ChallengeNotFound)
        ));
    }

    #[tokio::test]
    async fn expired_challenge_cannot_be_verified() {
        let store = CaptchaPowStore::new();
        let challenge = store
            .issue_challenge("comment", CaptchaPowDifficulty::Low, 120, None)
            .await
            .unwrap();
        let solution = find_solution(
            &challenge.id,
            &challenge.nonce,
            &challenge.action,
            challenge.leading_zero_bits,
        );

        {
            let mut challenges = store.challenges.write().await;
            let stored = challenges.get_mut(&challenge.id).unwrap();
            stored.expires_at = Utc::now() - Duration::seconds(1);
        }

        assert!(matches!(
            store
                .verify_solution(&challenge.id, "comment", &solution, 300, None)
                .await,
            Err(CaptchaPowError::ChallengeNotFound | CaptchaPowError::ChallengeExpired)
        ));
    }

    #[tokio::test]
    async fn expired_token_cannot_be_consumed() {
        let store = CaptchaPowStore::new();
        let challenge = store
            .issue_challenge("comment", CaptchaPowDifficulty::Low, 120, None)
            .await
            .unwrap();
        let solution = find_solution(
            &challenge.id,
            &challenge.nonce,
            &challenge.action,
            challenge.leading_zero_bits,
        );
        let token = store
            .verify_solution(&challenge.id, "comment", &solution, 300, None)
            .await
            .unwrap();

        {
            let mut tokens = store.tokens.write().await;
            let stored = tokens.get_mut(&token.token).unwrap();
            stored.expires_at = Utc::now() - Duration::seconds(1);
        }

        assert!(matches!(
            store.consume_token(&token.token, "comment", None).await,
            Err(CaptchaPowError::TokenNotFound | CaptchaPowError::TokenExpired)
        ));
    }
}
