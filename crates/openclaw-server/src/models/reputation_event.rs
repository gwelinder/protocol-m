//! Reputation event model for event sourcing reputation changes.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

use crate::models::BountyClosureType;

/// Closure type weights for reputation calculation.
/// Tests = 1.5x (automated verification is most reliable)
/// Quorum = 1.2x (peer review provides good signal)
/// Requester = 1.0x (single approver is baseline)
pub const WEIGHT_TESTS: f64 = 1.5;
pub const WEIGHT_QUORUM: f64 = 1.2;
pub const WEIGHT_REQUESTER: f64 = 1.0;

/// Types of reputation events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "reputation_event_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReputationEventType {
    /// Earned from completing a bounty.
    BountyCompletion,
    /// Earned from reviewing/validating work (quorum).
    ReviewContribution,
    /// Admin adjustment (corrections, disputes).
    ManualAdjustment,
    /// Time-based decay event.
    Decay,
}

/// Represents an immutable reputation event in the ledger.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReputationEvent {
    /// Unique identifier for this event.
    pub id: Uuid,
    /// DID that received/lost the reputation.
    pub did: String,
    /// Type of reputation event.
    pub event_type: ReputationEventType,
    /// Base amount before weighting.
    pub base_amount: BigDecimal,
    /// Closure type weight applied.
    pub closure_type_weight: BigDecimal,
    /// Reviewer credibility weight if applicable.
    pub reviewer_weight: BigDecimal,
    /// Final weighted amount.
    pub weighted_amount: BigDecimal,
    /// Reason description for this reputation change.
    pub reason: String,
    /// Closure type that triggered this event.
    pub closure_type: Option<String>,
    /// Related bounty ID if applicable.
    pub bounty_id: Option<Uuid>,
    /// Related submission ID if applicable.
    pub submission_id: Option<Uuid>,
    /// Additional metadata.
    pub metadata: serde_json::Value,
    /// When this event occurred.
    pub created_at: DateTime<Utc>,
}

/// Data required to create a new reputation event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewReputationEvent {
    pub did: String,
    pub event_type: ReputationEventType,
    pub base_amount: BigDecimal,
    pub closure_type_weight: BigDecimal,
    pub reviewer_weight: BigDecimal,
    pub weighted_amount: BigDecimal,
    pub reason: String,
    pub closure_type: Option<String>,
    pub bounty_id: Option<Uuid>,
    pub submission_id: Option<Uuid>,
    pub metadata: serde_json::Value,
}

impl NewReputationEvent {
    /// Create a bounty completion reputation event.
    pub fn bounty_completion(
        did: String,
        base_amount: BigDecimal,
        closure_type: BountyClosureType,
        reviewer_credibility: Option<f64>,
        bounty_id: Uuid,
        submission_id: Option<Uuid>,
        reason: String,
    ) -> Self {
        let closure_type_weight = closure_type_to_weight(closure_type);
        let reviewer_weight = reviewer_credibility.unwrap_or(1.0);
        let weighted_amount = calculate_weighted_amount(&base_amount, closure_type_weight, reviewer_weight);

        Self {
            did,
            event_type: ReputationEventType::BountyCompletion,
            base_amount,
            closure_type_weight: BigDecimal::try_from(closure_type_weight).unwrap_or_else(|_| BigDecimal::from(1)),
            reviewer_weight: BigDecimal::try_from(reviewer_weight).unwrap_or_else(|_| BigDecimal::from(1)),
            weighted_amount,
            reason,
            closure_type: Some(closure_type_to_string(closure_type)),
            bounty_id: Some(bounty_id),
            submission_id,
            metadata: serde_json::json!({}),
        }
    }

    /// Create a review contribution reputation event (for quorum reviewers).
    pub fn review_contribution(
        did: String,
        base_amount: BigDecimal,
        reviewer_credibility: f64,
        bounty_id: Uuid,
        reason: String,
    ) -> Self {
        let closure_type_weight = WEIGHT_QUORUM;
        let weighted_amount = calculate_weighted_amount(&base_amount, closure_type_weight, reviewer_credibility);

        Self {
            did,
            event_type: ReputationEventType::ReviewContribution,
            base_amount,
            closure_type_weight: BigDecimal::try_from(closure_type_weight).unwrap_or_else(|_| BigDecimal::from(1)),
            reviewer_weight: BigDecimal::try_from(reviewer_credibility).unwrap_or_else(|_| BigDecimal::from(1)),
            weighted_amount,
            reason,
            closure_type: Some("quorum".to_string()),
            bounty_id: Some(bounty_id),
            submission_id: None,
            metadata: serde_json::json!({}),
        }
    }

    /// Create a manual adjustment event.
    pub fn manual_adjustment(
        did: String,
        amount: BigDecimal,
        reason: String,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            did,
            event_type: ReputationEventType::ManualAdjustment,
            base_amount: amount.clone(),
            closure_type_weight: BigDecimal::from(1),
            reviewer_weight: BigDecimal::from(1),
            weighted_amount: amount,
            reason,
            closure_type: None,
            bounty_id: None,
            submission_id: None,
            metadata,
        }
    }

    /// Create a decay event.
    pub fn decay(
        did: String,
        amount: BigDecimal,
        months_decayed: u32,
    ) -> Self {
        Self {
            did,
            event_type: ReputationEventType::Decay,
            base_amount: amount.clone(),
            closure_type_weight: BigDecimal::from(1),
            reviewer_weight: BigDecimal::from(1),
            weighted_amount: amount,
            reason: format!("Time decay: {} month(s)", months_decayed),
            closure_type: None,
            bounty_id: None,
            submission_id: None,
            metadata: serde_json::json!({ "months_decayed": months_decayed }),
        }
    }
}

/// Get the weight multiplier for a closure type.
pub fn closure_type_to_weight(closure_type: BountyClosureType) -> f64 {
    match closure_type {
        BountyClosureType::Tests => WEIGHT_TESTS,
        BountyClosureType::Quorum => WEIGHT_QUORUM,
        BountyClosureType::Requester => WEIGHT_REQUESTER,
    }
}

/// Convert closure type to string representation.
fn closure_type_to_string(closure_type: BountyClosureType) -> String {
    match closure_type {
        BountyClosureType::Tests => "tests".to_string(),
        BountyClosureType::Quorum => "quorum".to_string(),
        BountyClosureType::Requester => "requester".to_string(),
    }
}

/// Calculate the weighted reputation amount.
fn calculate_weighted_amount(base_amount: &BigDecimal, closure_type_weight: f64, reviewer_weight: f64) -> BigDecimal {
    use std::str::FromStr;
    let weight = closure_type_weight * reviewer_weight;
    // Use string parsing to avoid floating point precision issues
    let weight_decimal = BigDecimal::from_str(&format!("{:.8}", weight)).unwrap_or_else(|_| BigDecimal::from(1));
    (base_amount * weight_decimal).round(8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_event_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ReputationEventType::BountyCompletion).unwrap(),
            "\"bounty_completion\""
        );
        assert_eq!(
            serde_json::to_string(&ReputationEventType::ReviewContribution).unwrap(),
            "\"review_contribution\""
        );
        assert_eq!(
            serde_json::to_string(&ReputationEventType::ManualAdjustment).unwrap(),
            "\"manual_adjustment\""
        );
        assert_eq!(
            serde_json::to_string(&ReputationEventType::Decay).unwrap(),
            "\"decay\""
        );
    }

    #[test]
    fn test_event_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<ReputationEventType>("\"bounty_completion\"").unwrap(),
            ReputationEventType::BountyCompletion
        );
        assert_eq!(
            serde_json::from_str::<ReputationEventType>("\"decay\"").unwrap(),
            ReputationEventType::Decay
        );
    }

    #[test]
    fn test_closure_type_weights() {
        assert_eq!(closure_type_to_weight(BountyClosureType::Tests), 1.5);
        assert_eq!(closure_type_to_weight(BountyClosureType::Quorum), 1.2);
        assert_eq!(closure_type_to_weight(BountyClosureType::Requester), 1.0);
    }

    #[test]
    fn test_bounty_completion_event_tests() {
        let base = BigDecimal::from_str("10.00000000").unwrap();
        let event = NewReputationEvent::bounty_completion(
            "did:key:z6MkTest".to_string(),
            base.clone(),
            BountyClosureType::Tests,
            None,
            Uuid::new_v4(),
            None,
            "Completed test bounty".to_string(),
        );

        assert_eq!(event.event_type, ReputationEventType::BountyCompletion);
        assert_eq!(event.closure_type, Some("tests".to_string()));
        // 10 * 1.5 = 15
        assert_eq!(event.weighted_amount, BigDecimal::from_str("15.00000000").unwrap());
    }

    #[test]
    fn test_bounty_completion_event_quorum() {
        let base = BigDecimal::from_str("10.00000000").unwrap();
        let event = NewReputationEvent::bounty_completion(
            "did:key:z6MkTest".to_string(),
            base.clone(),
            BountyClosureType::Quorum,
            None,
            Uuid::new_v4(),
            None,
            "Completed quorum bounty".to_string(),
        );

        // 10 * 1.2 = 12
        assert_eq!(event.weighted_amount, BigDecimal::from_str("12.00000000").unwrap());
    }

    #[test]
    fn test_bounty_completion_event_requester() {
        let base = BigDecimal::from_str("10.00000000").unwrap();
        let event = NewReputationEvent::bounty_completion(
            "did:key:z6MkTest".to_string(),
            base.clone(),
            BountyClosureType::Requester,
            None,
            Uuid::new_v4(),
            None,
            "Completed requester bounty".to_string(),
        );

        // 10 * 1.0 = 10
        assert_eq!(event.weighted_amount, BigDecimal::from_str("10.00000000").unwrap());
    }

    #[test]
    fn test_bounty_completion_with_reviewer_weight() {
        let base = BigDecimal::from_str("10.00000000").unwrap();
        let event = NewReputationEvent::bounty_completion(
            "did:key:z6MkTest".to_string(),
            base.clone(),
            BountyClosureType::Quorum,
            Some(1.5), // High credibility reviewer
            Uuid::new_v4(),
            None,
            "Completed quorum bounty".to_string(),
        );

        // 10 * 1.2 * 1.5 = 18
        assert_eq!(event.weighted_amount, BigDecimal::from_str("18.00000000").unwrap());
    }

    #[test]
    fn test_review_contribution_event() {
        let base = BigDecimal::from_str("5.00000000").unwrap();
        let event = NewReputationEvent::review_contribution(
            "did:key:z6MkReviewer".to_string(),
            base.clone(),
            1.2, // Reviewer credibility
            Uuid::new_v4(),
            "Reviewed submission".to_string(),
        );

        assert_eq!(event.event_type, ReputationEventType::ReviewContribution);
        // 5 * 1.2 * 1.2 = 7.2
        assert_eq!(event.weighted_amount, BigDecimal::from_str("7.20000000").unwrap());
    }

    #[test]
    fn test_manual_adjustment_event() {
        let amount = BigDecimal::from_str("-10.00000000").unwrap();
        let event = NewReputationEvent::manual_adjustment(
            "did:key:z6MkTest".to_string(),
            amount.clone(),
            "Dispute resolution penalty".to_string(),
            serde_json::json!({"dispute_id": "123"}),
        );

        assert_eq!(event.event_type, ReputationEventType::ManualAdjustment);
        assert_eq!(event.weighted_amount, amount);
        assert!(event.closure_type.is_none());
    }

    #[test]
    fn test_decay_event() {
        let amount = BigDecimal::from_str("-5.00000000").unwrap();
        let event = NewReputationEvent::decay(
            "did:key:z6MkTest".to_string(),
            amount.clone(),
            2,
        );

        assert_eq!(event.event_type, ReputationEventType::Decay);
        assert_eq!(event.reason, "Time decay: 2 month(s)");
        assert_eq!(event.metadata.get("months_decayed").unwrap(), 2);
    }

    #[test]
    fn test_calculate_weighted_amount() {
        let base = BigDecimal::from_str("100.00000000").unwrap();

        // Tests closure type
        let weighted = calculate_weighted_amount(&base, 1.5, 1.0);
        assert_eq!(weighted, BigDecimal::from_str("150.00000000").unwrap());

        // Quorum with high credibility reviewer
        let weighted = calculate_weighted_amount(&base, 1.2, 1.5);
        assert_eq!(weighted, BigDecimal::from_str("180.00000000").unwrap());
    }
}
