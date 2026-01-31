//! Password hashing module
//!
//! This module provides secure password hashing and verification using Argon2id,
//! which is the recommended variant for password hashing.
//!
//! # Security
//!
//! - Uses Argon2id variant (hybrid of Argon2i and Argon2d)
//! - Uses secure default parameters from the argon2 crate
//! - Generates random salt for each password hash
//!
//! Satisfies requirement 4.6: THE User_Service SHALL 使用安全的密码哈希算法存储密码

use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Hash a password using Argon2id with secure defaults.
///
/// # Arguments
///
/// * `password` - The plaintext password to hash
///
/// # Returns
///
/// The password hash as a PHC string format (includes algorithm, parameters, salt, and hash)
///
/// # Errors
///
/// Returns an error if password hashing fails
///
/// # Example
///
/// ```ignore
/// use noteva::services::password::hash_password;
///
/// let hash = hash_password("my_secure_password")?;
/// assert!(hash.starts_with("$argon2id$"));
/// ```
///
/// Satisfies requirement 4.6: THE User_Service SHALL 使用安全的密码哈希算法存储密码
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))
        .context("Password hashing failed")?;

    Ok(password_hash.to_string())
}

/// Verify a password against a stored hash.
///
/// # Arguments
///
/// * `password` - The plaintext password to verify
/// * `hash` - The stored password hash (PHC string format)
///
/// # Returns
///
/// `true` if the password matches the hash, `false` otherwise
///
/// # Errors
///
/// Returns an error if the hash format is invalid
///
/// # Example
///
/// ```ignore
/// use noteva::services::password::{hash_password, verify_password};
///
/// let hash = hash_password("my_password")?;
/// assert!(verify_password("my_password", &hash)?);
/// assert!(!verify_password("wrong_password", &hash)?);
/// ```
///
/// Satisfies requirement 4.6: THE User_Service SHALL 使用安全的密码哈希算法存储密码
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| anyhow::anyhow!("Invalid password hash format: {}", e))
        .context("Failed to parse password hash")?;

    let argon2 = Argon2::default();

    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(anyhow::anyhow!("Password verification failed: {}", e))
            .context("Password verification error"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_produces_argon2id_hash() {
        let password = "test_password_123";
        let hash = hash_password(password).expect("Failed to hash password");

        // Verify it's an Argon2id hash
        assert!(hash.starts_with("$argon2id$"), "Hash should use Argon2id");
    }

    #[test]
    fn test_hash_password_produces_different_hashes() {
        let password = "same_password";
        let hash1 = hash_password(password).expect("Failed to hash password");
        let hash2 = hash_password(password).expect("Failed to hash password");

        // Different salts should produce different hashes
        assert_ne!(hash1, hash2, "Same password should produce different hashes due to random salt");
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "correct_password";
        let hash = hash_password(password).expect("Failed to hash password");

        let result = verify_password(password, &hash).expect("Verification should not error");
        assert!(result, "Correct password should verify successfully");
    }

    #[test]
    fn test_verify_password_incorrect() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let hash = hash_password(password).expect("Failed to hash password");

        let result = verify_password(wrong_password, &hash).expect("Verification should not error");
        assert!(!result, "Wrong password should not verify");
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let result = verify_password("password", "invalid_hash_format");
        assert!(result.is_err(), "Invalid hash format should return error");
    }

    #[test]
    fn test_hash_password_empty_password() {
        // Empty passwords should still hash successfully (validation is done elsewhere)
        let hash = hash_password("").expect("Failed to hash empty password");
        assert!(hash.starts_with("$argon2id$"));

        let result = verify_password("", &hash).expect("Verification should not error");
        assert!(result, "Empty password should verify against its hash");
    }

    #[test]
    fn test_hash_password_unicode() {
        let password = "密码测试🔐";
        let hash = hash_password(password).expect("Failed to hash unicode password");

        let result = verify_password(password, &hash).expect("Verification should not error");
        assert!(result, "Unicode password should verify successfully");
    }

    #[test]
    fn test_hash_password_long_password() {
        let password = "a".repeat(1000);
        let hash = hash_password(&password).expect("Failed to hash long password");

        let result = verify_password(&password, &hash).expect("Verification should not error");
        assert!(result, "Long password should verify successfully");
    }

    #[test]
    fn test_password_hash_not_equal_to_password() {
        let password = "my_secret_password";
        let hash = hash_password(password).expect("Failed to hash password");

        assert_ne!(password, hash, "Hash should not equal the original password");
        assert!(!hash.contains(password), "Hash should not contain the original password");
    }
}
