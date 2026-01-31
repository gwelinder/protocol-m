//! Agent suspension model for Protocol M kill switch functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

/// Represents an agent suspension record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentSuspension {
    /// Unique identifier for this suspension.
    pub id: Uuid,
    /// DID of the operator/agent that was suspended.
    pub operator_did: String,
    /// Reason for the suspension.
    pub reason: String,
    /// When the agent was suspended.
    pub suspended_at: DateTime<Utc>,
    /// When the agent was resumed (null if still suspended).
    pub resumed_at: Option<DateTime<Utc>>,
    /// Additional metadata (bounties cancelled, escrow refunded, etc.).
    pub metadata: Option<JsonValue>,
    /// DID of the user/admin who resumed the agent (null if still suspended).
    pub resumed_by_did: Option<String>,
}

/// Data required to create a new agent suspension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAgentSuspension {
    pub operator_did: String,
    pub reason: String,
    pub metadata: Option<JsonValue>,
}

impl AgentSuspension {
    /// Check if the suspension is currently active.
    pub fn is_active(&self) -> bool {
        self.resumed_at.is_none()
    }

    /// Check if the suspension has been lifted.
    pub fn is_resumed(&self) -> bool {
        self.resumed_at.is_some()
    }

    /// Get the duration of the suspension (or time since suspended if still active).
    pub fn duration_seconds(&self) -> i64 {
        let end_time = self.resumed_at.unwrap_or_else(Utc::now);
        (end_time - self.suspended_at).num_seconds()
    }

    /// Get the number of bounties cancelled from metadata.
    pub fn bounties_cancelled(&self) -> Option<i64> {
        self.metadata
            .as_ref()
            .and_then(|m| m.get("bounties_cancelled"))
            .and_then(|v| v.as_i64())
    }

    /// Get the number of approval requests cancelled from metadata.
    pub fn approval_requests_cancelled(&self) -> Option<i64> {
        self.metadata
            .as_ref()
            .and_then(|m| m.get("approval_requests_cancelled"))
            .and_then(|v| v.as_i64())
    }

    /// Get the total escrow refunded from metadata.
    pub fn escrow_refunded(&self) -> Option<&str> {
        self.metadata
            .as_ref()
            .and_then(|m| m.get("escrow_refunded"))
            .and_then(|v| v.as_str())
    }
}

impl NewAgentSuspension {
    /// Create a new agent suspension with the given details.
    pub fn new(operator_did: String, reason: String) -> Self {
        Self {
            operator_did,
            reason,
            metadata: None,
        }
    }

    /// Create a new agent suspension with metadata.
    pub fn with_metadata(operator_did: String, reason: String, metadata: JsonValue) -> Self {
        Self {
            operator_did,
            reason,
            metadata: Some(metadata),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serde_json::json;

    #[test]
    fn test_agent_suspension_serialization() {
        let now = Utc::now();
        let suspension = AgentSuspension {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkTest".to_string(),
            reason: "Runaway spending detected".to_string(),
            suspended_at: now,
            resumed_at: None,
            metadata: Some(json!({
                "bounties_cancelled": 5,
                "escrow_refunded": "500.00000000"
            })),
            resumed_by_did: None,
        };

        let json = serde_json::to_string(&suspension).unwrap();
        assert!(json.contains("did:key:z6MkTest"));
        assert!(json.contains("Runaway spending detected"));
    }

    #[test]
    fn test_agent_suspension_deserialization() {
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "operator_did": "did:key:z6MkTest",
            "reason": "Emergency stop",
            "suspended_at": "2026-01-31T12:00:00Z",
            "resumed_at": null,
            "metadata": null,
            "resumed_by_did": null
        }"#;

        let suspension: AgentSuspension = serde_json::from_str(json).unwrap();
        assert_eq!(suspension.operator_did, "did:key:z6MkTest");
        assert_eq!(suspension.reason, "Emergency stop");
        assert!(suspension.is_active());
    }

    #[test]
    fn test_is_active() {
        let now = Utc::now();

        // Active suspension
        let active = AgentSuspension {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkTest".to_string(),
            reason: "Test".to_string(),
            suspended_at: now,
            resumed_at: None,
            metadata: None,
            resumed_by_did: None,
        };
        assert!(active.is_active());
        assert!(!active.is_resumed());

        // Resumed suspension
        let resumed = AgentSuspension {
            resumed_at: Some(now + Duration::hours(1)),
            resumed_by_did: Some("did:key:z6MkAdmin".to_string()),
            ..active
        };
        assert!(!resumed.is_active());
        assert!(resumed.is_resumed());
    }

    #[test]
    fn test_duration_seconds() {
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);

        // Active suspension (1 hour ago)
        let active = AgentSuspension {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkTest".to_string(),
            reason: "Test".to_string(),
            suspended_at: one_hour_ago,
            resumed_at: None,
            metadata: None,
            resumed_by_did: None,
        };
        // Duration should be approximately 3600 seconds (1 hour)
        let duration = active.duration_seconds();
        assert!(duration >= 3598 && duration <= 3602);

        // Resumed suspension (lasted exactly 30 minutes)
        let resumed = AgentSuspension {
            resumed_at: Some(one_hour_ago + Duration::minutes(30)),
            ..active
        };
        assert_eq!(resumed.duration_seconds(), 1800);
    }

    #[test]
    fn test_metadata_accessors() {
        let suspension = AgentSuspension {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkTest".to_string(),
            reason: "Test".to_string(),
            suspended_at: Utc::now(),
            resumed_at: None,
            metadata: Some(json!({
                "bounties_cancelled": 5,
                "approval_requests_cancelled": 3,
                "escrow_refunded": "500.00000000"
            })),
            resumed_by_did: None,
        };

        assert_eq!(suspension.bounties_cancelled(), Some(5));
        assert_eq!(suspension.approval_requests_cancelled(), Some(3));
        assert_eq!(suspension.escrow_refunded(), Some("500.00000000"));
    }

    #[test]
    fn test_metadata_accessors_missing_fields() {
        let suspension = AgentSuspension {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkTest".to_string(),
            reason: "Test".to_string(),
            suspended_at: Utc::now(),
            resumed_at: None,
            metadata: Some(json!({})),
            resumed_by_did: None,
        };

        assert_eq!(suspension.bounties_cancelled(), None);
        assert_eq!(suspension.approval_requests_cancelled(), None);
        assert_eq!(suspension.escrow_refunded(), None);
    }

    #[test]
    fn test_metadata_accessors_no_metadata() {
        let suspension = AgentSuspension {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkTest".to_string(),
            reason: "Test".to_string(),
            suspended_at: Utc::now(),
            resumed_at: None,
            metadata: None,
            resumed_by_did: None,
        };

        assert_eq!(suspension.bounties_cancelled(), None);
        assert_eq!(suspension.approval_requests_cancelled(), None);
        assert_eq!(suspension.escrow_refunded(), None);
    }

    #[test]
    fn test_new_agent_suspension() {
        let new_suspension = NewAgentSuspension::new(
            "did:key:z6MkTest".to_string(),
            "Runaway spending".to_string(),
        );

        assert_eq!(new_suspension.operator_did, "did:key:z6MkTest");
        assert_eq!(new_suspension.reason, "Runaway spending");
        assert!(new_suspension.metadata.is_none());
    }

    #[test]
    fn test_new_agent_suspension_with_metadata() {
        let metadata = json!({
            "bounties_cancelled": 3,
            "escrow_refunded": "250.00000000"
        });

        let new_suspension = NewAgentSuspension::with_metadata(
            "did:key:z6MkTest".to_string(),
            "Emergency stop".to_string(),
            metadata.clone(),
        );

        assert_eq!(new_suspension.operator_did, "did:key:z6MkTest");
        assert_eq!(new_suspension.reason, "Emergency stop");
        assert_eq!(new_suspension.metadata, Some(metadata));
    }
}
