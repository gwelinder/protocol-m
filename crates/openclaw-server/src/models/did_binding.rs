//! DID binding model for linking DIDs to user accounts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Represents a binding between a DID and a user account.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DidBinding {
    /// Unique identifier for this binding record.
    pub id: Uuid,
    /// User ID that this DID is bound to.
    pub user_id: Uuid,
    /// DID of the bound identity (did:key:z6Mk...).
    pub did: String,
    /// When this binding was created.
    pub created_at: DateTime<Utc>,
    /// When this binding was revoked (null if still active).
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Data required to create a new DID binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDidBinding {
    pub user_id: Uuid,
    pub did: String,
}

impl DidBinding {
    /// Returns true if this binding is currently active (not revoked).
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_active_when_not_revoked() {
        let binding = DidBinding {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            did: "did:key:z6MkTest...".to_string(),
            created_at: Utc::now(),
            revoked_at: None,
        };
        assert!(binding.is_active());
    }

    #[test]
    fn test_is_active_when_revoked() {
        let binding = DidBinding {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            did: "did:key:z6MkTest...".to_string(),
            created_at: Utc::now(),
            revoked_at: Some(Utc::now()),
        };
        assert!(!binding.is_active());
    }
}
