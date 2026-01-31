//! Dispute model for Protocol M bounty dispute resolution.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

/// Dispute window duration in days.
pub const DISPUTE_WINDOW_DAYS: i64 = 7;

/// Stake percentage required to initiate a dispute (10%).
pub const DISPUTE_STAKE_PERCENTAGE: f64 = 0.10;

/// Possible states of a dispute in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "dispute_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum DisputeStatus {
    /// Dispute is pending resolution.
    Pending,
    /// Dispute has been resolved.
    Resolved,
    /// Dispute window expired without resolution.
    Expired,
}

/// Possible resolution outcomes for a dispute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionOutcome {
    /// The original submission is upheld as valid.
    /// Initiator loses stake, submitter keeps reward.
    UpholdSubmission,
    /// The original submission is rejected as invalid.
    /// Submitter loses reward, poster gets escrow back, initiator gets stake back.
    RejectSubmission,
}

impl ResolutionOutcome {
    /// Parse a resolution outcome from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "uphold_submission" => Some(Self::UpholdSubmission),
            "reject_submission" => Some(Self::RejectSubmission),
            _ => None,
        }
    }

    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UpholdSubmission => "uphold_submission",
            Self::RejectSubmission => "reject_submission",
        }
    }
}

/// Represents a dispute against a bounty submission.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Dispute {
    /// Unique identifier for this dispute.
    pub id: Uuid,
    /// ID of the bounty being disputed.
    pub bounty_id: Uuid,
    /// ID of the submission being disputed.
    pub submission_id: Uuid,
    /// DID of the agent/user who initiated the dispute.
    pub initiator_did: String,
    /// Reason for the dispute.
    pub reason: String,
    /// Current status of the dispute.
    pub status: DisputeStatus,
    /// Amount staked by the initiator (10% of bounty reward).
    pub stake_amount: BigDecimal,
    /// Reference to the escrow hold for the stake.
    pub stake_escrow_id: Option<Uuid>,
    /// Resolution outcome (null until resolved).
    pub resolution_outcome: Option<String>,
    /// DID of the arbiter who resolved the dispute.
    pub resolver_did: Option<String>,
    /// When this dispute was created.
    pub created_at: DateTime<Utc>,
    /// When the dispute was resolved (null if still pending).
    pub resolved_at: Option<DateTime<Utc>>,
    /// Deadline for dispute resolution.
    pub dispute_deadline: DateTime<Utc>,
}

/// Data required to create a new dispute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDispute {
    pub bounty_id: Uuid,
    pub submission_id: Uuid,
    pub initiator_did: String,
    pub reason: String,
    pub stake_amount: BigDecimal,
    pub stake_escrow_id: Option<Uuid>,
    pub dispute_deadline: DateTime<Utc>,
}

impl Dispute {
    /// Check if the dispute is pending resolution.
    pub fn is_pending(&self) -> bool {
        self.status == DisputeStatus::Pending
    }

    /// Check if the dispute has been resolved.
    pub fn is_resolved(&self) -> bool {
        self.status == DisputeStatus::Resolved
    }

    /// Check if the dispute has expired.
    pub fn is_expired(&self) -> bool {
        self.status == DisputeStatus::Expired
    }

    /// Check if the dispute has passed its deadline.
    pub fn is_past_deadline(&self) -> bool {
        Utc::now() > self.dispute_deadline
    }

    /// Get the resolution outcome as an enum.
    pub fn resolution(&self) -> Option<ResolutionOutcome> {
        self.resolution_outcome
            .as_ref()
            .and_then(|s| ResolutionOutcome::from_str(s))
    }

    /// Check if the submission was upheld.
    pub fn was_upheld(&self) -> bool {
        self.resolution() == Some(ResolutionOutcome::UpholdSubmission)
    }

    /// Check if the submission was rejected.
    pub fn was_rejected(&self) -> bool {
        self.resolution() == Some(ResolutionOutcome::RejectSubmission)
    }
}

impl NewDispute {
    /// Create a new dispute with calculated deadline.
    pub fn new(
        bounty_id: Uuid,
        submission_id: Uuid,
        initiator_did: String,
        reason: String,
        stake_amount: BigDecimal,
        stake_escrow_id: Option<Uuid>,
    ) -> Self {
        let dispute_deadline = Utc::now() + Duration::days(DISPUTE_WINDOW_DAYS);
        Self {
            bounty_id,
            submission_id,
            initiator_did,
            reason,
            stake_amount,
            stake_escrow_id,
            dispute_deadline,
        }
    }

    /// Create a new dispute with a custom deadline.
    pub fn with_deadline(
        bounty_id: Uuid,
        submission_id: Uuid,
        initiator_did: String,
        reason: String,
        stake_amount: BigDecimal,
        stake_escrow_id: Option<Uuid>,
        dispute_deadline: DateTime<Utc>,
    ) -> Self {
        Self {
            bounty_id,
            submission_id,
            initiator_did,
            reason,
            stake_amount,
            stake_escrow_id,
            dispute_deadline,
        }
    }
}

/// Calculate the required stake amount for a dispute.
pub fn calculate_dispute_stake(bounty_reward: &BigDecimal) -> BigDecimal {
    use std::str::FromStr;
    let percentage = BigDecimal::from_str(&format!("{:.8}", DISPUTE_STAKE_PERCENTAGE)).unwrap();
    bounty_reward * &percentage
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_dispute_status_serialization() {
        assert_eq!(
            serde_json::to_string(&DisputeStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&DisputeStatus::Resolved).unwrap(),
            "\"resolved\""
        );
        assert_eq!(
            serde_json::to_string(&DisputeStatus::Expired).unwrap(),
            "\"expired\""
        );
    }

    #[test]
    fn test_dispute_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<DisputeStatus>("\"pending\"").unwrap(),
            DisputeStatus::Pending
        );
        assert_eq!(
            serde_json::from_str::<DisputeStatus>("\"resolved\"").unwrap(),
            DisputeStatus::Resolved
        );
        assert_eq!(
            serde_json::from_str::<DisputeStatus>("\"expired\"").unwrap(),
            DisputeStatus::Expired
        );
    }

    #[test]
    fn test_resolution_outcome_parsing() {
        assert_eq!(
            ResolutionOutcome::from_str("uphold_submission"),
            Some(ResolutionOutcome::UpholdSubmission)
        );
        assert_eq!(
            ResolutionOutcome::from_str("reject_submission"),
            Some(ResolutionOutcome::RejectSubmission)
        );
        assert_eq!(ResolutionOutcome::from_str("invalid"), None);
    }

    #[test]
    fn test_resolution_outcome_as_str() {
        assert_eq!(
            ResolutionOutcome::UpholdSubmission.as_str(),
            "uphold_submission"
        );
        assert_eq!(
            ResolutionOutcome::RejectSubmission.as_str(),
            "reject_submission"
        );
    }

    #[test]
    fn test_calculate_dispute_stake() {
        let reward = BigDecimal::from_str("100.00000000").unwrap();
        let stake = calculate_dispute_stake(&reward);
        let expected = BigDecimal::from_str("10.00000000").unwrap();
        assert_eq!(stake, expected);
    }

    #[test]
    fn test_calculate_dispute_stake_large_amount() {
        let reward = BigDecimal::from_str("5000.00000000").unwrap();
        let stake = calculate_dispute_stake(&reward);
        let expected = BigDecimal::from_str("500.00000000").unwrap();
        assert_eq!(stake, expected);
    }

    #[test]
    fn test_new_dispute_creates_deadline() {
        let bounty_id = Uuid::new_v4();
        let submission_id = Uuid::new_v4();
        let stake = BigDecimal::from_str("10.00000000").unwrap();

        let new_dispute = NewDispute::new(
            bounty_id,
            submission_id,
            "did:key:z6MkTest".to_string(),
            "Fraudulent submission".to_string(),
            stake,
            None,
        );

        // Deadline should be approximately 7 days from now
        let now = Utc::now();
        let expected_deadline = now + Duration::days(DISPUTE_WINDOW_DAYS);
        let diff = (new_dispute.dispute_deadline - expected_deadline).num_seconds().abs();
        assert!(diff < 2); // Allow 2 seconds of variance
    }

    #[test]
    fn test_dispute_status_helpers() {
        let now = Utc::now();
        let future = now + Duration::hours(1);

        let dispute = Dispute {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submission_id: Uuid::new_v4(),
            initiator_did: "did:key:z6MkTest".to_string(),
            reason: "Fraudulent".to_string(),
            status: DisputeStatus::Pending,
            stake_amount: BigDecimal::from_str("10.00000000").unwrap(),
            stake_escrow_id: Some(Uuid::new_v4()),
            resolution_outcome: None,
            resolver_did: None,
            created_at: now,
            resolved_at: None,
            dispute_deadline: future,
        };

        assert!(dispute.is_pending());
        assert!(!dispute.is_resolved());
        assert!(!dispute.is_expired());
        assert!(!dispute.is_past_deadline());
        assert!(dispute.resolution().is_none());
    }

    #[test]
    fn test_dispute_past_deadline() {
        let now = Utc::now();
        let past = now - Duration::hours(1);

        let dispute = Dispute {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submission_id: Uuid::new_v4(),
            initiator_did: "did:key:z6MkTest".to_string(),
            reason: "Fraudulent".to_string(),
            status: DisputeStatus::Pending,
            stake_amount: BigDecimal::from_str("10.00000000").unwrap(),
            stake_escrow_id: Some(Uuid::new_v4()),
            resolution_outcome: None,
            resolver_did: None,
            created_at: now - Duration::days(8),
            resolved_at: None,
            dispute_deadline: past,
        };

        assert!(dispute.is_past_deadline());
    }

    #[test]
    fn test_dispute_resolution_helpers() {
        let now = Utc::now();

        // Test upheld submission
        let dispute_upheld = Dispute {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submission_id: Uuid::new_v4(),
            initiator_did: "did:key:z6MkTest".to_string(),
            reason: "Fraudulent".to_string(),
            status: DisputeStatus::Resolved,
            stake_amount: BigDecimal::from_str("10.00000000").unwrap(),
            stake_escrow_id: Some(Uuid::new_v4()),
            resolution_outcome: Some("uphold_submission".to_string()),
            resolver_did: Some("did:key:z6MkArbiter".to_string()),
            created_at: now,
            resolved_at: Some(now),
            dispute_deadline: now + Duration::days(7),
        };

        assert!(dispute_upheld.is_resolved());
        assert!(dispute_upheld.was_upheld());
        assert!(!dispute_upheld.was_rejected());

        // Test rejected submission
        let dispute_rejected = Dispute {
            resolution_outcome: Some("reject_submission".to_string()),
            ..dispute_upheld
        };

        assert!(!dispute_rejected.was_upheld());
        assert!(dispute_rejected.was_rejected());
    }
}
