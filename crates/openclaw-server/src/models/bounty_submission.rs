//! Bounty submission model for Protocol M task marketplace.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

/// Status of a bounty submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "submission_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SubmissionStatus {
    /// Submission is pending review/verification.
    Pending,
    /// Submission has been approved and reward paid.
    Approved,
    /// Submission was rejected.
    Rejected,
}

/// Represents a submission for a bounty.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountySubmission {
    /// Unique identifier for this submission.
    pub id: Uuid,
    /// ID of the bounty this submission is for.
    pub bounty_id: Uuid,
    /// DID of the agent/user who submitted the work.
    pub submitter_did: String,
    /// SHA-256 hash of the submitted artifact.
    pub artifact_hash: String,
    /// Full SignatureEnvelopeV1 JSON for the submission.
    pub signature_envelope: serde_json::Value,
    /// Optional execution receipt for test-based bounties.
    pub execution_receipt: Option<serde_json::Value>,
    /// Current status of the submission.
    pub status: SubmissionStatus,
    /// When this submission was created.
    pub created_at: DateTime<Utc>,
    /// Reference to the registered artifact in ClawdHub (set on approval).
    pub artifact_id: Option<Uuid>,
}

/// Data required to create a new bounty submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewBountySubmission {
    pub bounty_id: Uuid,
    pub submitter_did: String,
    pub artifact_hash: String,
    pub signature_envelope: serde_json::Value,
    pub execution_receipt: Option<serde_json::Value>,
}

impl BountySubmission {
    /// Check if the submission is pending review.
    pub fn is_pending(&self) -> bool {
        self.status == SubmissionStatus::Pending
    }

    /// Check if the submission has been approved.
    pub fn is_approved(&self) -> bool {
        self.status == SubmissionStatus::Approved
    }

    /// Check if the submission was rejected.
    pub fn is_rejected(&self) -> bool {
        self.status == SubmissionStatus::Rejected
    }

    /// Check if the submission has an execution receipt.
    pub fn has_execution_receipt(&self) -> bool {
        self.execution_receipt.is_some()
    }

    /// Get the harness hash from the execution receipt, if present.
    pub fn execution_harness_hash(&self) -> Option<&str> {
        self.execution_receipt
            .as_ref()
            .and_then(|r| r.get("harness_hash"))
            .and_then(|v| v.as_str())
    }

    /// Check if all tests passed in the execution receipt.
    pub fn all_tests_passed(&self) -> Option<bool> {
        self.execution_receipt
            .as_ref()
            .and_then(|r| r.get("all_tests_passed"))
            .and_then(|v| v.as_bool())
    }

    /// Get the test results from the execution receipt, if present.
    pub fn test_results(&self) -> Option<&serde_json::Value> {
        self.execution_receipt
            .as_ref()
            .and_then(|r| r.get("test_results"))
    }

    /// Check if the submission has a registered artifact.
    pub fn has_artifact(&self) -> bool {
        self.artifact_id.is_some()
    }

    /// Get the artifact ID if registered.
    pub fn artifact_id(&self) -> Option<Uuid> {
        self.artifact_id
    }
}

impl NewBountySubmission {
    /// Create a new submission with an execution receipt (for test-based bounties).
    pub fn with_execution_receipt(
        bounty_id: Uuid,
        submitter_did: String,
        artifact_hash: String,
        signature_envelope: serde_json::Value,
        execution_receipt: serde_json::Value,
    ) -> Self {
        Self {
            bounty_id,
            submitter_did,
            artifact_hash,
            signature_envelope,
            execution_receipt: Some(execution_receipt),
        }
    }

    /// Create a new submission without an execution receipt.
    pub fn without_execution_receipt(
        bounty_id: Uuid,
        submitter_did: String,
        artifact_hash: String,
        signature_envelope: serde_json::Value,
    ) -> Self {
        Self {
            bounty_id,
            submitter_did,
            artifact_hash,
            signature_envelope,
            execution_receipt: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submission_status_serialization() {
        assert_eq!(
            serde_json::to_string(&SubmissionStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&SubmissionStatus::Approved).unwrap(),
            "\"approved\""
        );
        assert_eq!(
            serde_json::to_string(&SubmissionStatus::Rejected).unwrap(),
            "\"rejected\""
        );
    }

    #[test]
    fn test_submission_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<SubmissionStatus>("\"pending\"").unwrap(),
            SubmissionStatus::Pending
        );
        assert_eq!(
            serde_json::from_str::<SubmissionStatus>("\"approved\"").unwrap(),
            SubmissionStatus::Approved
        );
        assert_eq!(
            serde_json::from_str::<SubmissionStatus>("\"rejected\"").unwrap(),
            SubmissionStatus::Rejected
        );
    }

    #[test]
    fn test_new_submission_with_execution_receipt() {
        let bounty_id = Uuid::new_v4();
        let submission = NewBountySubmission::with_execution_receipt(
            bounty_id,
            "did:key:z6MkTest".to_string(),
            "abc123def456".to_string(),
            serde_json::json!({"version": "1.0", "signature": "..."}),
            serde_json::json!({"harness_hash": "sha256:test", "all_tests_passed": true}),
        );

        assert_eq!(submission.bounty_id, bounty_id);
        assert_eq!(submission.submitter_did, "did:key:z6MkTest");
        assert!(submission.execution_receipt.is_some());
    }

    #[test]
    fn test_new_submission_without_execution_receipt() {
        let bounty_id = Uuid::new_v4();
        let submission = NewBountySubmission::without_execution_receipt(
            bounty_id,
            "did:key:z6MkTest".to_string(),
            "abc123def456".to_string(),
            serde_json::json!({"version": "1.0", "signature": "..."}),
        );

        assert_eq!(submission.bounty_id, bounty_id);
        assert!(submission.execution_receipt.is_none());
    }

    #[test]
    fn test_submission_status_helpers() {
        let now = Utc::now();
        let submission = BountySubmission {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            artifact_hash: "abc123".to_string(),
            signature_envelope: serde_json::json!({}),
            execution_receipt: None,
            status: SubmissionStatus::Pending,
            created_at: now,
            artifact_id: None,
        };

        assert!(submission.is_pending());
        assert!(!submission.is_approved());
        assert!(!submission.is_rejected());
        assert!(!submission.has_execution_receipt());
        assert!(!submission.has_artifact());
    }

    #[test]
    fn test_execution_receipt_helpers() {
        let now = Utc::now();
        let submission = BountySubmission {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            artifact_hash: "abc123".to_string(),
            signature_envelope: serde_json::json!({}),
            execution_receipt: Some(serde_json::json!({
                "harness_hash": "sha256:testharness",
                "all_tests_passed": true,
                "test_results": {
                    "total": 10,
                    "passed": 10,
                    "failed": 0
                }
            })),
            status: SubmissionStatus::Pending,
            created_at: now,
            artifact_id: None,
        };

        assert!(submission.has_execution_receipt());
        assert_eq!(
            submission.execution_harness_hash(),
            Some("sha256:testharness")
        );
        assert_eq!(submission.all_tests_passed(), Some(true));
        assert!(submission.test_results().is_some());
    }

    #[test]
    fn test_submission_without_execution_receipt_returns_none() {
        let now = Utc::now();
        let submission = BountySubmission {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            artifact_hash: "abc123".to_string(),
            signature_envelope: serde_json::json!({}),
            execution_receipt: None,
            status: SubmissionStatus::Pending,
            created_at: now,
            artifact_id: None,
        };

        assert_eq!(submission.execution_harness_hash(), None);
        assert_eq!(submission.all_tests_passed(), None);
        assert!(submission.test_results().is_none());
    }

    #[test]
    fn test_submission_with_artifact_id() {
        let now = Utc::now();
        let artifact_id = Uuid::new_v4();
        let submission = BountySubmission {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            artifact_hash: "abc123".to_string(),
            signature_envelope: serde_json::json!({}),
            execution_receipt: None,
            status: SubmissionStatus::Approved,
            created_at: now,
            artifact_id: Some(artifact_id),
        };

        assert!(submission.has_artifact());
        assert_eq!(submission.artifact_id(), Some(artifact_id));
    }

    #[test]
    fn test_submission_without_artifact_id() {
        let now = Utc::now();
        let submission = BountySubmission {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            artifact_hash: "abc123".to_string(),
            signature_envelope: serde_json::json!({}),
            execution_receipt: None,
            status: SubmissionStatus::Pending,
            created_at: now,
            artifact_id: None,
        };

        assert!(!submission.has_artifact());
        assert_eq!(submission.artifact_id(), None);
    }
}
