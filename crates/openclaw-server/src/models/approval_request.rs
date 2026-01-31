//! Approval request model for Protocol M operator approval workflow.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

/// Default approval window duration in hours.
pub const APPROVAL_WINDOW_HOURS: i64 = 24;

/// Type of action requiring operator approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "approval_action_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ApprovalActionType {
    /// Delegating authority to another DID.
    Delegate,
    /// Spending credits beyond threshold.
    Spend,
}

impl ApprovalActionType {
    /// Parse an action type from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "delegate" => Some(Self::Delegate),
            "spend" => Some(Self::Spend),
            _ => None,
        }
    }

    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Delegate => "delegate",
            Self::Spend => "spend",
        }
    }
}

/// Status of an approval request in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "approval_request_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ApprovalRequestStatus {
    /// Awaiting operator approval.
    Pending,
    /// Approved by operator.
    Approved,
    /// Rejected by operator.
    Rejected,
    /// Approval window expired.
    Expired,
}

impl ApprovalRequestStatus {
    /// Parse a status from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }

    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
        }
    }
}

/// Represents an approval request for a high-value action.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApprovalRequest {
    /// Unique identifier for this request.
    pub id: Uuid,
    /// DID of the operator who must approve this request.
    pub operator_did: String,
    /// ID of the bounty (for spend actions, null for delegate).
    pub bounty_id: Option<Uuid>,
    /// Type of action requiring approval.
    pub action_type: ApprovalActionType,
    /// Amount of credits involved (for spend actions).
    pub amount: Option<BigDecimal>,
    /// Current status of the request.
    pub status: ApprovalRequestStatus,
    /// Additional metadata (e.g., delegate_to_did, reason).
    pub metadata: serde_json::Value,
    /// When this request was created.
    pub created_at: DateTime<Utc>,
    /// When the request was resolved (null if still pending).
    pub resolved_at: Option<DateTime<Utc>>,
    /// When the request expires.
    pub expires_at: DateTime<Utc>,
    /// DID of the agent/user who created the request.
    pub requester_did: String,
    /// Resolution reason (null until resolved).
    pub resolution_reason: Option<String>,
}

/// Data required to create a new approval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewApprovalRequest {
    pub operator_did: String,
    pub bounty_id: Option<Uuid>,
    pub action_type: ApprovalActionType,
    pub amount: Option<BigDecimal>,
    pub metadata: serde_json::Value,
    pub requester_did: String,
    pub expires_at: DateTime<Utc>,
}

impl ApprovalRequest {
    /// Check if the request is pending.
    pub fn is_pending(&self) -> bool {
        self.status == ApprovalRequestStatus::Pending
    }

    /// Check if the request has been approved.
    pub fn is_approved(&self) -> bool {
        self.status == ApprovalRequestStatus::Approved
    }

    /// Check if the request has been rejected.
    pub fn is_rejected(&self) -> bool {
        self.status == ApprovalRequestStatus::Rejected
    }

    /// Check if the request has expired.
    pub fn is_expired(&self) -> bool {
        self.status == ApprovalRequestStatus::Expired
    }

    /// Check if the request has passed its expiry time.
    pub fn is_past_expiry(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the request is still valid (pending and not past expiry).
    pub fn is_valid(&self) -> bool {
        self.is_pending() && !self.is_past_expiry()
    }

    /// Check if this is a delegate action.
    pub fn is_delegate_action(&self) -> bool {
        self.action_type == ApprovalActionType::Delegate
    }

    /// Check if this is a spend action.
    pub fn is_spend_action(&self) -> bool {
        self.action_type == ApprovalActionType::Spend
    }

    /// Get the delegate target DID from metadata (for delegate actions).
    pub fn delegate_to_did(&self) -> Option<&str> {
        self.metadata
            .get("delegate_to_did")
            .and_then(|v| v.as_str())
    }

    /// Get the description from metadata.
    pub fn description(&self) -> Option<&str> {
        self.metadata.get("description").and_then(|v| v.as_str())
    }
}

impl NewApprovalRequest {
    /// Create a new spend approval request.
    pub fn spend(
        operator_did: String,
        requester_did: String,
        bounty_id: Uuid,
        amount: BigDecimal,
        metadata: serde_json::Value,
    ) -> Self {
        let expires_at = Utc::now() + Duration::hours(APPROVAL_WINDOW_HOURS);
        Self {
            operator_did,
            bounty_id: Some(bounty_id),
            action_type: ApprovalActionType::Spend,
            amount: Some(amount),
            metadata,
            requester_did,
            expires_at,
        }
    }

    /// Create a new delegate approval request.
    pub fn delegate(
        operator_did: String,
        requester_did: String,
        delegate_to_did: String,
        metadata: serde_json::Value,
    ) -> Self {
        let expires_at = Utc::now() + Duration::hours(APPROVAL_WINDOW_HOURS);
        let mut meta = metadata;
        meta["delegate_to_did"] = serde_json::json!(delegate_to_did);
        Self {
            operator_did,
            bounty_id: None,
            action_type: ApprovalActionType::Delegate,
            amount: None,
            metadata: meta,
            requester_did,
            expires_at,
        }
    }

    /// Create a request with a custom expiry time.
    pub fn with_expiry(
        operator_did: String,
        requester_did: String,
        action_type: ApprovalActionType,
        bounty_id: Option<Uuid>,
        amount: Option<BigDecimal>,
        metadata: serde_json::Value,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            operator_did,
            bounty_id,
            action_type,
            amount,
            metadata,
            requester_did,
            expires_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_action_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ApprovalActionType::Delegate).unwrap(),
            "\"delegate\""
        );
        assert_eq!(
            serde_json::to_string(&ApprovalActionType::Spend).unwrap(),
            "\"spend\""
        );
    }

    #[test]
    fn test_action_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<ApprovalActionType>("\"delegate\"").unwrap(),
            ApprovalActionType::Delegate
        );
        assert_eq!(
            serde_json::from_str::<ApprovalActionType>("\"spend\"").unwrap(),
            ApprovalActionType::Spend
        );
    }

    #[test]
    fn test_action_type_from_str() {
        assert_eq!(
            ApprovalActionType::from_str("delegate"),
            Some(ApprovalActionType::Delegate)
        );
        assert_eq!(
            ApprovalActionType::from_str("spend"),
            Some(ApprovalActionType::Spend)
        );
        assert_eq!(ApprovalActionType::from_str("invalid"), None);
    }

    #[test]
    fn test_action_type_as_str() {
        assert_eq!(ApprovalActionType::Delegate.as_str(), "delegate");
        assert_eq!(ApprovalActionType::Spend.as_str(), "spend");
    }

    #[test]
    fn test_status_serialization() {
        assert_eq!(
            serde_json::to_string(&ApprovalRequestStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&ApprovalRequestStatus::Approved).unwrap(),
            "\"approved\""
        );
        assert_eq!(
            serde_json::to_string(&ApprovalRequestStatus::Rejected).unwrap(),
            "\"rejected\""
        );
        assert_eq!(
            serde_json::to_string(&ApprovalRequestStatus::Expired).unwrap(),
            "\"expired\""
        );
    }

    #[test]
    fn test_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<ApprovalRequestStatus>("\"pending\"").unwrap(),
            ApprovalRequestStatus::Pending
        );
        assert_eq!(
            serde_json::from_str::<ApprovalRequestStatus>("\"approved\"").unwrap(),
            ApprovalRequestStatus::Approved
        );
        assert_eq!(
            serde_json::from_str::<ApprovalRequestStatus>("\"rejected\"").unwrap(),
            ApprovalRequestStatus::Rejected
        );
        assert_eq!(
            serde_json::from_str::<ApprovalRequestStatus>("\"expired\"").unwrap(),
            ApprovalRequestStatus::Expired
        );
    }

    #[test]
    fn test_status_from_str() {
        assert_eq!(
            ApprovalRequestStatus::from_str("pending"),
            Some(ApprovalRequestStatus::Pending)
        );
        assert_eq!(
            ApprovalRequestStatus::from_str("approved"),
            Some(ApprovalRequestStatus::Approved)
        );
        assert_eq!(
            ApprovalRequestStatus::from_str("rejected"),
            Some(ApprovalRequestStatus::Rejected)
        );
        assert_eq!(
            ApprovalRequestStatus::from_str("expired"),
            Some(ApprovalRequestStatus::Expired)
        );
        assert_eq!(ApprovalRequestStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_new_spend_request() {
        let operator_did = "did:key:z6MkOperator".to_string();
        let requester_did = "did:key:z6MkRequester".to_string();
        let bounty_id = Uuid::new_v4();
        let amount = BigDecimal::from_str("500.00000000").unwrap();
        let metadata = serde_json::json!({"description": "High-value bounty"});

        let request =
            NewApprovalRequest::spend(operator_did.clone(), requester_did.clone(), bounty_id, amount.clone(), metadata);

        assert_eq!(request.operator_did, operator_did);
        assert_eq!(request.requester_did, requester_did);
        assert_eq!(request.bounty_id, Some(bounty_id));
        assert_eq!(request.action_type, ApprovalActionType::Spend);
        assert_eq!(request.amount, Some(amount));

        // Expiry should be approximately 24 hours from now
        let now = Utc::now();
        let expected_expiry = now + Duration::hours(APPROVAL_WINDOW_HOURS);
        let diff = (request.expires_at - expected_expiry).num_seconds().abs();
        assert!(diff < 2); // Allow 2 seconds of variance
    }

    #[test]
    fn test_new_delegate_request() {
        let operator_did = "did:key:z6MkOperator".to_string();
        let requester_did = "did:key:z6MkRequester".to_string();
        let delegate_to_did = "did:key:z6MkDelegate".to_string();
        let metadata = serde_json::json!({"reason": "Trusted collaborator"});

        let request = NewApprovalRequest::delegate(
            operator_did.clone(),
            requester_did.clone(),
            delegate_to_did.clone(),
            metadata,
        );

        assert_eq!(request.operator_did, operator_did);
        assert_eq!(request.requester_did, requester_did);
        assert_eq!(request.bounty_id, None);
        assert_eq!(request.action_type, ApprovalActionType::Delegate);
        assert_eq!(request.amount, None);
        assert_eq!(
            request.metadata.get("delegate_to_did").unwrap().as_str(),
            Some(delegate_to_did.as_str())
        );
    }

    #[test]
    fn test_approval_request_status_helpers() {
        let now = Utc::now();
        let future = now + Duration::hours(1);

        let request = ApprovalRequest {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkOperator".to_string(),
            bounty_id: Some(Uuid::new_v4()),
            action_type: ApprovalActionType::Spend,
            amount: Some(BigDecimal::from_str("500.00000000").unwrap()),
            status: ApprovalRequestStatus::Pending,
            metadata: serde_json::json!({}),
            created_at: now,
            resolved_at: None,
            expires_at: future,
            requester_did: "did:key:z6MkRequester".to_string(),
            resolution_reason: None,
        };

        assert!(request.is_pending());
        assert!(!request.is_approved());
        assert!(!request.is_rejected());
        assert!(!request.is_expired());
        assert!(!request.is_past_expiry());
        assert!(request.is_valid());
        assert!(request.is_spend_action());
        assert!(!request.is_delegate_action());
    }

    #[test]
    fn test_approval_request_past_expiry() {
        let now = Utc::now();
        let past = now - Duration::hours(1);

        let request = ApprovalRequest {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkOperator".to_string(),
            bounty_id: None,
            action_type: ApprovalActionType::Delegate,
            amount: None,
            status: ApprovalRequestStatus::Pending,
            metadata: serde_json::json!({"delegate_to_did": "did:key:z6MkDelegate"}),
            created_at: now - Duration::days(2),
            resolved_at: None,
            expires_at: past,
            requester_did: "did:key:z6MkRequester".to_string(),
            resolution_reason: None,
        };

        assert!(request.is_past_expiry());
        assert!(!request.is_valid()); // Pending but past expiry
    }

    #[test]
    fn test_approval_request_delegate_to_did() {
        let now = Utc::now();
        let future = now + Duration::hours(24);
        let delegate_did = "did:key:z6MkDelegate";

        let request = ApprovalRequest {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkOperator".to_string(),
            bounty_id: None,
            action_type: ApprovalActionType::Delegate,
            amount: None,
            status: ApprovalRequestStatus::Pending,
            metadata: serde_json::json!({"delegate_to_did": delegate_did}),
            created_at: now,
            resolved_at: None,
            expires_at: future,
            requester_did: "did:key:z6MkRequester".to_string(),
            resolution_reason: None,
        };

        assert_eq!(request.delegate_to_did(), Some(delegate_did));
        assert!(request.is_delegate_action());
    }

    #[test]
    fn test_approval_request_description() {
        let now = Utc::now();
        let future = now + Duration::hours(24);
        let description = "Request to post high-value bounty";

        let request = ApprovalRequest {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkOperator".to_string(),
            bounty_id: Some(Uuid::new_v4()),
            action_type: ApprovalActionType::Spend,
            amount: Some(BigDecimal::from_str("1000.00000000").unwrap()),
            status: ApprovalRequestStatus::Pending,
            metadata: serde_json::json!({"description": description}),
            created_at: now,
            resolved_at: None,
            expires_at: future,
            requester_did: "did:key:z6MkRequester".to_string(),
            resolution_reason: None,
        };

        assert_eq!(request.description(), Some(description));
    }

    #[test]
    fn test_approved_request() {
        let now = Utc::now();

        let request = ApprovalRequest {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkOperator".to_string(),
            bounty_id: Some(Uuid::new_v4()),
            action_type: ApprovalActionType::Spend,
            amount: Some(BigDecimal::from_str("500.00000000").unwrap()),
            status: ApprovalRequestStatus::Approved,
            metadata: serde_json::json!({}),
            created_at: now - Duration::hours(1),
            resolved_at: Some(now),
            expires_at: now + Duration::hours(23),
            requester_did: "did:key:z6MkRequester".to_string(),
            resolution_reason: Some("Approved for trusted agent".to_string()),
        };

        assert!(!request.is_pending());
        assert!(request.is_approved());
        assert!(!request.is_rejected());
        assert!(!request.is_valid()); // Not pending
    }

    #[test]
    fn test_rejected_request() {
        let now = Utc::now();

        let request = ApprovalRequest {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkOperator".to_string(),
            bounty_id: Some(Uuid::new_v4()),
            action_type: ApprovalActionType::Spend,
            amount: Some(BigDecimal::from_str("500.00000000").unwrap()),
            status: ApprovalRequestStatus::Rejected,
            metadata: serde_json::json!({}),
            created_at: now - Duration::hours(1),
            resolved_at: Some(now),
            expires_at: now + Duration::hours(23),
            requester_did: "did:key:z6MkRequester".to_string(),
            resolution_reason: Some("Budget exceeded for this month".to_string()),
        };

        assert!(!request.is_pending());
        assert!(!request.is_approved());
        assert!(request.is_rejected());
    }

    #[test]
    fn test_expired_request() {
        let now = Utc::now();

        let request = ApprovalRequest {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkOperator".to_string(),
            bounty_id: Some(Uuid::new_v4()),
            action_type: ApprovalActionType::Spend,
            amount: Some(BigDecimal::from_str("500.00000000").unwrap()),
            status: ApprovalRequestStatus::Expired,
            metadata: serde_json::json!({}),
            created_at: now - Duration::days(2),
            resolved_at: Some(now),
            expires_at: now - Duration::days(1),
            requester_did: "did:key:z6MkRequester".to_string(),
            resolution_reason: None,
        };

        assert!(!request.is_pending());
        assert!(!request.is_approved());
        assert!(!request.is_rejected());
        assert!(request.is_expired());
    }
}
