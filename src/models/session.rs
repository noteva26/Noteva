//! Session model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Session entity for user authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID (token)
    pub id: String,
    /// Associated user ID
    pub user_id: i64,
    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Session {
    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }
}
