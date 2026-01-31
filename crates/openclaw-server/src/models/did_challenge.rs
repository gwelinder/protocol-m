//! DID challenge model for secure DID binding flow.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Represents a challenge for secure DID binding.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DidChallenge {
    /// Unique identifier for this challenge record.
    pub id: Uuid,
    /// User ID requesting the challenge.
    pub user_id: Uuid,
    /// Random challenge bytes encoded as hex.
    pub challenge: String,
    /// When this challenge expires.
    pub expires_at: DateTime<Utc>,
    /// When this challenge was used (null if not yet used).
    pub used_at: Option<DateTime<Utc>>,
    /// When this challenge was created.
    pub created_at: DateTime<Utc>,
}

/// Data required to create a new DID challenge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDidChallenge {
    pub user_id: Uuid,
    pub challenge: String,
    pub expires_at: DateTime<Utc>,
}

impl DidChallenge {
    /// Returns true if this challenge is still valid (not expired and not used).
    pub fn is_valid(&self) -> bool {
        self.used_at.is_none() && self.expires_at > Utc::now()
    }

    /// Returns true if this challenge has been used.
    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }

    /// Returns true if this challenge has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_challenge(expires_at: DateTime<Utc>, used_at: Option<DateTime<Utc>>) -> DidChallenge {
        DidChallenge {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            challenge: "0123456789abcdef".to_string(),
            expires_at,
            used_at,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_is_valid_when_not_expired_and_not_used() {
        let challenge = make_challenge(Utc::now() + Duration::minutes(10), None);
        assert!(challenge.is_valid());
    }

    #[test]
    fn test_is_valid_when_expired() {
        let challenge = make_challenge(Utc::now() - Duration::minutes(1), None);
        assert!(!challenge.is_valid());
    }

    #[test]
    fn test_is_valid_when_used() {
        let challenge = make_challenge(Utc::now() + Duration::minutes(10), Some(Utc::now()));
        assert!(!challenge.is_valid());
    }

    #[test]
    fn test_is_used() {
        let unused = make_challenge(Utc::now() + Duration::minutes(10), None);
        assert!(!unused.is_used());

        let used = make_challenge(Utc::now() + Duration::minutes(10), Some(Utc::now()));
        assert!(used.is_used());
    }

    #[test]
    fn test_is_expired() {
        let not_expired = make_challenge(Utc::now() + Duration::minutes(10), None);
        assert!(!not_expired.is_expired());

        let expired = make_challenge(Utc::now() - Duration::minutes(1), None);
        assert!(expired.is_expired());
    }
}
