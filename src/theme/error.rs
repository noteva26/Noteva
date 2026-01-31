//! Theme engine error types

use thiserror::Error;

/// Theme-specific errors
#[derive(Debug, Error)]
pub enum ThemeError {
    /// Theme not found
    #[error("Theme not found: {0}")]
    NotFound(String),

    /// Template rendering error
    #[error("Template error: {0}")]
    TemplateError(String),

    /// Invalid theme metadata (theme.toml)
    #[error("Invalid theme metadata: {0}")]
    InvalidMetadata(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
