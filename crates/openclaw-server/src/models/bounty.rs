//! Bounty model for Protocol M task marketplace.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

/// How a bounty's completion is verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "bounty_closure_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BountyClosureType {
    /// Automated verification via test harness.
    /// Requires `eval_harness_hash` in metadata.
    Tests,
    /// Verification by multiple reviewers.
    /// Requires `reviewer_count` and `min_reviewer_rep` in metadata.
    Quorum,
    /// Manual approval by the bounty requester.
    Requester,
}

/// Possible states of a bounty in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "bounty_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BountyStatus {
    /// Bounty is awaiting operator approval (for high-value bounties).
    PendingApproval,
    /// Bounty is open and accepting submissions.
    Open,
    /// Someone has accepted the bounty and is working on it.
    InProgress,
    /// Bounty has been completed and reward paid out.
    Completed,
    /// Bounty was cancelled by the poster.
    Cancelled,
}

/// Represents a bounty in the Protocol M marketplace.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Bounty {
    /// Unique identifier for this bounty.
    pub id: Uuid,
    /// DID of the agent/user who posted the bounty.
    pub poster_did: String,
    /// Title of the bounty.
    pub title: String,
    /// Detailed description of the task.
    pub description: String,
    /// Amount of M-credits offered as reward.
    pub reward_credits: BigDecimal,
    /// How bounty completion is verified.
    pub closure_type: BountyClosureType,
    /// Current status of the bounty.
    pub status: BountyStatus,
    /// Additional closure-type specific configuration.
    pub metadata: serde_json::Value,
    /// When this bounty was created.
    pub created_at: DateTime<Utc>,
    /// When this bounty was last updated.
    pub updated_at: DateTime<Utc>,
    /// Optional deadline for bounty completion.
    pub deadline: Option<DateTime<Utc>>,
}

/// Data required to create a new bounty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewBounty {
    pub poster_did: String,
    pub title: String,
    pub description: String,
    pub reward_credits: BigDecimal,
    pub closure_type: BountyClosureType,
    pub metadata: serde_json::Value,
    pub deadline: Option<DateTime<Utc>>,
}

impl Bounty {
    /// Check if the bounty is pending operator approval.
    pub fn is_pending_approval(&self) -> bool {
        self.status == BountyStatus::PendingApproval
    }

    /// Check if the bounty is open for submissions.
    pub fn is_open(&self) -> bool {
        self.status == BountyStatus::Open
    }

    /// Check if the bounty is currently being worked on.
    pub fn is_in_progress(&self) -> bool {
        self.status == BountyStatus::InProgress
    }

    /// Check if the bounty has been completed.
    pub fn is_completed(&self) -> bool {
        self.status == BountyStatus::Completed
    }

    /// Check if the bounty was cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.status == BountyStatus::Cancelled
    }

    /// Check if the bounty is still active (open or in progress).
    pub fn is_active(&self) -> bool {
        self.is_open() || self.is_in_progress()
    }

    /// Check if the bounty has passed its deadline.
    pub fn is_expired(&self) -> bool {
        match self.deadline {
            Some(deadline) => Utc::now() > deadline,
            None => false,
        }
    }

    /// Check if the bounty uses test-based verification.
    pub fn uses_tests(&self) -> bool {
        self.closure_type == BountyClosureType::Tests
    }

    /// Check if the bounty uses quorum-based verification.
    pub fn uses_quorum(&self) -> bool {
        self.closure_type == BountyClosureType::Quorum
    }

    /// Check if the bounty uses requester-based verification.
    pub fn uses_requester(&self) -> bool {
        self.closure_type == BountyClosureType::Requester
    }

    /// Get the eval harness hash for test-based bounties.
    pub fn eval_harness_hash(&self) -> Option<&str> {
        self.metadata
            .get("eval_harness_hash")
            .and_then(|v| v.as_str())
    }

    /// Get the required reviewer count for quorum-based bounties.
    pub fn reviewer_count(&self) -> Option<i64> {
        self.metadata
            .get("reviewer_count")
            .and_then(|v| v.as_i64())
    }

    /// Get the minimum reviewer reputation for quorum-based bounties.
    pub fn min_reviewer_rep(&self) -> Option<i64> {
        self.metadata
            .get("min_reviewer_rep")
            .and_then(|v| v.as_i64())
    }
}

impl NewBounty {
    /// Create a new test-based bounty.
    pub fn test_bounty(
        poster_did: String,
        title: String,
        description: String,
        reward_credits: BigDecimal,
        eval_harness_hash: String,
        deadline: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            poster_did,
            title,
            description,
            reward_credits,
            closure_type: BountyClosureType::Tests,
            metadata: serde_json::json!({
                "eval_harness_hash": eval_harness_hash
            }),
            deadline,
        }
    }

    /// Create a new quorum-based bounty.
    pub fn quorum_bounty(
        poster_did: String,
        title: String,
        description: String,
        reward_credits: BigDecimal,
        reviewer_count: i64,
        min_reviewer_rep: i64,
        deadline: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            poster_did,
            title,
            description,
            reward_credits,
            closure_type: BountyClosureType::Quorum,
            metadata: serde_json::json!({
                "reviewer_count": reviewer_count,
                "min_reviewer_rep": min_reviewer_rep
            }),
            deadline,
        }
    }

    /// Create a new requester-based bounty (manual approval).
    pub fn requester_bounty(
        poster_did: String,
        title: String,
        description: String,
        reward_credits: BigDecimal,
        deadline: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            poster_did,
            title,
            description,
            reward_credits,
            closure_type: BountyClosureType::Requester,
            metadata: serde_json::json!({}),
            deadline,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_closure_type_serialization() {
        assert_eq!(
            serde_json::to_string(&BountyClosureType::Tests).unwrap(),
            "\"tests\""
        );
        assert_eq!(
            serde_json::to_string(&BountyClosureType::Quorum).unwrap(),
            "\"quorum\""
        );
        assert_eq!(
            serde_json::to_string(&BountyClosureType::Requester).unwrap(),
            "\"requester\""
        );
    }

    #[test]
    fn test_closure_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<BountyClosureType>("\"tests\"").unwrap(),
            BountyClosureType::Tests
        );
        assert_eq!(
            serde_json::from_str::<BountyClosureType>("\"quorum\"").unwrap(),
            BountyClosureType::Quorum
        );
        assert_eq!(
            serde_json::from_str::<BountyClosureType>("\"requester\"").unwrap(),
            BountyClosureType::Requester
        );
    }

    #[test]
    fn test_status_serialization() {
        assert_eq!(
            serde_json::to_string(&BountyStatus::Open).unwrap(),
            "\"open\""
        );
        assert_eq!(
            serde_json::to_string(&BountyStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&BountyStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&BountyStatus::Cancelled).unwrap(),
            "\"cancelled\""
        );
    }

    #[test]
    fn test_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<BountyStatus>("\"open\"").unwrap(),
            BountyStatus::Open
        );
        assert_eq!(
            serde_json::from_str::<BountyStatus>("\"in_progress\"").unwrap(),
            BountyStatus::InProgress
        );
        assert_eq!(
            serde_json::from_str::<BountyStatus>("\"completed\"").unwrap(),
            BountyStatus::Completed
        );
        assert_eq!(
            serde_json::from_str::<BountyStatus>("\"cancelled\"").unwrap(),
            BountyStatus::Cancelled
        );
    }

    #[test]
    fn test_new_test_bounty() {
        let bounty = NewBounty::test_bounty(
            "did:key:z6MkTest".to_string(),
            "Fix bug".to_string(),
            "Please fix the bug".to_string(),
            BigDecimal::from_str("100.00000000").unwrap(),
            "sha256:abc123".to_string(),
            None,
        );

        assert_eq!(bounty.poster_did, "did:key:z6MkTest");
        assert_eq!(bounty.title, "Fix bug");
        assert_eq!(bounty.closure_type, BountyClosureType::Tests);
        assert_eq!(
            bounty.metadata.get("eval_harness_hash").unwrap(),
            "sha256:abc123"
        );
    }

    #[test]
    fn test_new_quorum_bounty() {
        let bounty = NewBounty::quorum_bounty(
            "did:key:z6MkTest".to_string(),
            "Review code".to_string(),
            "Please review".to_string(),
            BigDecimal::from_str("50.00000000").unwrap(),
            3,
            100,
            None,
        );

        assert_eq!(bounty.closure_type, BountyClosureType::Quorum);
        assert_eq!(bounty.metadata.get("reviewer_count").unwrap(), 3);
        assert_eq!(bounty.metadata.get("min_reviewer_rep").unwrap(), 100);
    }

    #[test]
    fn test_new_requester_bounty() {
        let bounty = NewBounty::requester_bounty(
            "did:key:z6MkTest".to_string(),
            "Design task".to_string(),
            "Please design".to_string(),
            BigDecimal::from_str("200.00000000").unwrap(),
            None,
        );

        assert_eq!(bounty.closure_type, BountyClosureType::Requester);
        assert!(bounty.metadata.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_bounty_status_helpers() {
        let now = Utc::now();
        let bounty = Bounty {
            id: Uuid::new_v4(),
            poster_did: "did:key:z6MkTest".to_string(),
            title: "Test".to_string(),
            description: "Test description".to_string(),
            reward_credits: BigDecimal::from_str("100.00000000").unwrap(),
            closure_type: BountyClosureType::Tests,
            status: BountyStatus::Open,
            metadata: serde_json::json!({"eval_harness_hash": "sha256:test"}),
            created_at: now,
            updated_at: now,
            deadline: None,
        };

        assert!(bounty.is_open());
        assert!(!bounty.is_in_progress());
        assert!(!bounty.is_completed());
        assert!(!bounty.is_cancelled());
        assert!(bounty.is_active());
    }

    #[test]
    fn test_bounty_expiry() {
        let now = Utc::now();
        let past = now - chrono::Duration::hours(1);
        let future = now + chrono::Duration::hours(1);

        // No deadline - not expired
        let bounty_no_deadline = Bounty {
            id: Uuid::new_v4(),
            poster_did: "did:key:z6MkTest".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            reward_credits: BigDecimal::from_str("100.00000000").unwrap(),
            closure_type: BountyClosureType::Tests,
            status: BountyStatus::Open,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            deadline: None,
        };
        assert!(!bounty_no_deadline.is_expired());

        // Past deadline - expired
        let bounty_expired = Bounty {
            deadline: Some(past),
            ..bounty_no_deadline.clone()
        };
        assert!(bounty_expired.is_expired());

        // Future deadline - not expired
        let bounty_not_expired = Bounty {
            deadline: Some(future),
            ..bounty_no_deadline
        };
        assert!(!bounty_not_expired.is_expired());
    }

    #[test]
    fn test_bounty_metadata_helpers() {
        let now = Utc::now();

        // Test bounty with eval harness
        let test_bounty = Bounty {
            id: Uuid::new_v4(),
            poster_did: "did:key:z6MkTest".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            reward_credits: BigDecimal::from_str("100.00000000").unwrap(),
            closure_type: BountyClosureType::Tests,
            status: BountyStatus::Open,
            metadata: serde_json::json!({"eval_harness_hash": "sha256:abc123"}),
            created_at: now,
            updated_at: now,
            deadline: None,
        };
        assert!(test_bounty.uses_tests());
        assert_eq!(test_bounty.eval_harness_hash(), Some("sha256:abc123"));
        assert_eq!(test_bounty.reviewer_count(), None);

        // Quorum bounty with reviewer settings
        let quorum_bounty = Bounty {
            closure_type: BountyClosureType::Quorum,
            metadata: serde_json::json!({"reviewer_count": 5, "min_reviewer_rep": 200}),
            ..test_bounty
        };
        assert!(quorum_bounty.uses_quorum());
        assert_eq!(quorum_bounty.reviewer_count(), Some(5));
        assert_eq!(quorum_bounty.min_reviewer_rep(), Some(200));
        assert_eq!(quorum_bounty.eval_harness_hash(), None);
    }
}
