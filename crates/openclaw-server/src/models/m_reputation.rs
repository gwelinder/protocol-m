//! M-reputation model for Protocol M reputation tracking.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a reputation record for a DID in Protocol M.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MReputation {
    /// DID (Decentralized Identifier) of the agent/user.
    pub did: String,
    /// Total reputation score after decay.
    pub total_rep: BigDecimal,
    /// Current decay factor (0.99^months since start).
    pub decay_factor: BigDecimal,
    /// When reputation was last updated.
    pub last_updated: DateTime<Utc>,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
}

/// Data required to create a new reputation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMReputation {
    pub did: String,
    pub total_rep: BigDecimal,
    pub decay_factor: BigDecimal,
}

impl MReputation {
    /// Check if this reputation record has any accumulated reputation.
    pub fn has_reputation(&self) -> bool {
        self.total_rep > BigDecimal::from(0)
    }

    /// Get the effective reputation after applying time decay.
    /// Decay is 0.99 per month since last_updated.
    pub fn effective_reputation(&self, now: DateTime<Utc>) -> BigDecimal {
        let months_elapsed = self.months_since_last_update(now);
        if months_elapsed == 0 {
            return self.total_rep.clone();
        }

        // Apply 0.99^months decay using f64 calculation
        let decay_rate = 0.99_f64.powi(months_elapsed as i32);
        let decay = BigDecimal::try_from(decay_rate).unwrap_or_else(|_| BigDecimal::from(1));
        &self.total_rep * decay
    }

    /// Calculate months elapsed since last update.
    fn months_since_last_update(&self, now: DateTime<Utc>) -> u32 {
        let duration = now.signed_duration_since(self.last_updated);
        let days = duration.num_days().max(0) as u32;
        // Approximate months as 30 days
        days / 30
    }
}

impl NewMReputation {
    /// Create a new reputation record with initial values.
    pub fn new(did: String, initial_rep: BigDecimal) -> Self {
        Self {
            did,
            total_rep: initial_rep,
            decay_factor: BigDecimal::from(1),
        }
    }

    /// Create a new reputation record with zero reputation.
    pub fn zero(did: String) -> Self {
        Self {
            did,
            total_rep: BigDecimal::from(0),
            decay_factor: BigDecimal::from(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn create_test_reputation() -> MReputation {
        let now = Utc::now();
        MReputation {
            did: "did:key:z6MkTest".to_string(),
            total_rep: BigDecimal::from_str("100.00000000").unwrap(),
            decay_factor: BigDecimal::from(1),
            last_updated: now,
            created_at: now,
        }
    }

    #[test]
    fn test_has_reputation_positive() {
        let rep = create_test_reputation();
        assert!(rep.has_reputation());
    }

    #[test]
    fn test_has_reputation_zero() {
        let mut rep = create_test_reputation();
        rep.total_rep = BigDecimal::from(0);
        assert!(!rep.has_reputation());
    }

    #[test]
    fn test_effective_reputation_no_decay() {
        let rep = create_test_reputation();
        let now = Utc::now();
        let effective = rep.effective_reputation(now);
        assert_eq!(effective, rep.total_rep);
    }

    #[test]
    fn test_effective_reputation_with_decay() {
        let mut rep = create_test_reputation();
        // Set last_updated to 60 days ago (2 months)
        rep.last_updated = Utc::now() - chrono::Duration::days(60);
        let effective = rep.effective_reputation(Utc::now());
        // After 2 months: 100 * 0.99 * 0.99 = 98.01
        let expected = BigDecimal::from_str("98.01000000").unwrap();
        assert_eq!(effective.round(8), expected);
    }

    #[test]
    fn test_new_m_reputation() {
        let rep = NewMReputation::new(
            "did:key:z6MkTest".to_string(),
            BigDecimal::from_str("50.00000000").unwrap(),
        );
        assert_eq!(rep.did, "did:key:z6MkTest");
        assert_eq!(rep.total_rep, BigDecimal::from_str("50.00000000").unwrap());
        assert_eq!(rep.decay_factor, BigDecimal::from(1));
    }

    #[test]
    fn test_new_m_reputation_zero() {
        let rep = NewMReputation::zero("did:key:z6MkNew".to_string());
        assert_eq!(rep.did, "did:key:z6MkNew");
        assert_eq!(rep.total_rep, BigDecimal::from(0));
    }

    #[test]
    fn test_serialization() {
        let rep = create_test_reputation();
        let json = serde_json::to_string(&rep).unwrap();
        assert!(json.contains("did:key:z6MkTest"));
        assert!(json.contains("total_rep"));
        assert!(json.contains("decay_factor"));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{
            "did": "did:key:z6MkTest",
            "total_rep": "100.00000000",
            "decay_factor": "1.00000000",
            "last_updated": "2026-01-31T12:00:00Z",
            "created_at": "2026-01-31T12:00:00Z"
        }"#;
        let rep: MReputation = serde_json::from_str(json).unwrap();
        assert_eq!(rep.did, "did:key:z6MkTest");
        assert_eq!(rep.total_rep, BigDecimal::from_str("100.00000000").unwrap());
    }
}
