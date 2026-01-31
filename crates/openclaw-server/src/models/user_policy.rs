//! User policy model for Protocol M operator approval workflow.
//!
//! This model stores spending policies per DID, enabling approval workflows
//! for high-value bounty posting.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::str::FromStr;

/// Default values for policy fields.
pub const DEFAULT_MAX_SPEND_PER_DAY: &str = "1000.00000000";
pub const DEFAULT_MAX_SPEND_PER_BOUNTY: &str = "500.00000000";
pub const DEFAULT_APPROVAL_THRESHOLD: &str = "100.00000000";
pub const DEFAULT_TIMEOUT_HOURS: i32 = 24;

/// Represents a user's spending policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserPolicy {
    /// The DID this policy belongs to.
    pub did: String,
    /// Policy version (must be "1.0").
    pub version: String,
    /// Maximum credits that can be spent in a 24-hour rolling window.
    pub max_spend_per_day: BigDecimal,
    /// Maximum credits that can be spent on a single bounty.
    pub max_spend_per_bounty: BigDecimal,
    /// Whether policy enforcement is active.
    pub enabled: bool,
    /// Approval tiers configuration (JSONB array).
    pub approval_tiers: serde_json::Value,
    /// Allowed delegates (DIDs that can act on behalf of this identity).
    pub allowed_delegates: serde_json::Value,
    /// Emergency contact information.
    pub emergency_contact: Option<serde_json::Value>,
    /// When this policy was created.
    pub created_at: DateTime<Utc>,
    /// When this policy was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Data required to create a new user policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUserPolicy {
    pub did: String,
    pub version: String,
    pub max_spend_per_day: BigDecimal,
    pub max_spend_per_bounty: BigDecimal,
    pub enabled: bool,
    pub approval_tiers: serde_json::Value,
    pub allowed_delegates: serde_json::Value,
    pub emergency_contact: Option<serde_json::Value>,
}

/// Represents a single approval tier within a policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalTier {
    /// Credit amount threshold that triggers this tier.
    pub threshold: f64,
    /// Whether approval is required when this threshold is exceeded.
    #[serde(default = "default_require_approval")]
    pub require_approval: bool,
    /// DIDs authorized to approve requests at this tier (empty = owner only).
    #[serde(default)]
    pub approvers: Vec<String>,
    /// Hours to wait for approval before auto-rejecting (0 = no timeout).
    #[serde(default = "default_timeout_hours")]
    pub timeout_hours: u32,
    /// Channels to notify when approval is required.
    #[serde(default)]
    pub notification_channels: Vec<NotificationChannel>,
}

/// Notification channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    /// Notification channel type.
    #[serde(rename = "type")]
    pub channel_type: NotificationChannelType,
    /// Target address/URL for the notification.
    pub target: String,
}

/// Notification channel types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationChannelType {
    Email,
    Webhook,
    Slack,
}

fn default_require_approval() -> bool {
    true
}

fn default_timeout_hours() -> u32 {
    DEFAULT_TIMEOUT_HOURS as u32
}

impl UserPolicy {
    /// Check if policy enforcement is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Parse approval tiers from the JSONB field.
    pub fn get_approval_tiers(&self) -> Vec<ApprovalTier> {
        serde_json::from_value(self.approval_tiers.clone()).unwrap_or_default()
    }

    /// Check if a given amount requires approval based on policy tiers.
    /// Returns the matching tier if approval is required, None otherwise.
    pub fn requires_approval(&self, amount: &BigDecimal) -> Option<ApprovalTier> {
        if !self.enabled {
            return None;
        }

        let amount_f64 = amount.to_string().parse::<f64>().unwrap_or(0.0);
        let tiers = self.get_approval_tiers();

        // Find the highest threshold that the amount exceeds and requires approval
        let mut matching_tier: Option<ApprovalTier> = None;

        for tier in tiers {
            if amount_f64 > tier.threshold && tier.require_approval {
                match &matching_tier {
                    None => matching_tier = Some(tier),
                    Some(current) if tier.threshold > current.threshold => {
                        matching_tier = Some(tier);
                    }
                    _ => {}
                }
            }
        }

        matching_tier
    }

    /// Check if spending exceeds the per-bounty limit.
    pub fn exceeds_per_bounty_limit(&self, amount: &BigDecimal) -> bool {
        self.enabled && amount > &self.max_spend_per_bounty
    }

    /// Get the allowed delegates list.
    pub fn get_allowed_delegates(&self) -> Vec<String> {
        serde_json::from_value(self.allowed_delegates.clone()).unwrap_or_default()
    }

    /// Get emergency contact information.
    pub fn get_emergency_contact(&self) -> Option<EmergencyContact> {
        self.emergency_contact
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get operator DID (the DID that owns this policy and can approve requests).
    /// For now, this is the same as the policy DID.
    pub fn operator_did(&self) -> &str {
        &self.did
    }
}

/// Emergency contact information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyContact {
    /// Email address for emergency notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Webhook URL for emergency notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook: Option<String>,
}

impl NewUserPolicy {
    /// Create a new policy with default values.
    pub fn default_policy(did: String) -> Self {
        Self {
            did,
            version: "1.0".to_string(),
            max_spend_per_day: BigDecimal::from_str(DEFAULT_MAX_SPEND_PER_DAY).unwrap(),
            max_spend_per_bounty: BigDecimal::from_str(DEFAULT_MAX_SPEND_PER_BOUNTY).unwrap(),
            enabled: true,
            approval_tiers: serde_json::json!([{
                "threshold": 100,
                "require_approval": true,
                "approvers": [],
                "timeout_hours": 24,
                "notification_channels": []
            }]),
            allowed_delegates: serde_json::json!([]),
            emergency_contact: None,
        }
    }

    /// Create a policy with no approval requirements.
    pub fn no_approval_policy(did: String) -> Self {
        Self {
            did,
            version: "1.0".to_string(),
            max_spend_per_day: BigDecimal::from_str(DEFAULT_MAX_SPEND_PER_DAY).unwrap(),
            max_spend_per_bounty: BigDecimal::from_str(DEFAULT_MAX_SPEND_PER_BOUNTY).unwrap(),
            enabled: false,
            approval_tiers: serde_json::json!([]),
            allowed_delegates: serde_json::json!([]),
            emergency_contact: None,
        }
    }

    /// Create a policy with custom approval tiers.
    pub fn with_approval_tiers(
        did: String,
        max_spend_per_day: BigDecimal,
        max_spend_per_bounty: BigDecimal,
        tiers: Vec<ApprovalTier>,
    ) -> Self {
        Self {
            did,
            version: "1.0".to_string(),
            max_spend_per_day,
            max_spend_per_bounty,
            enabled: true,
            approval_tiers: serde_json::to_value(tiers).unwrap_or_default(),
            allowed_delegates: serde_json::json!([]),
            emergency_contact: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_policy() -> UserPolicy {
        UserPolicy {
            did: "did:key:z6MkTest".to_string(),
            version: "1.0".to_string(),
            max_spend_per_day: BigDecimal::from_str("1000.00000000").unwrap(),
            max_spend_per_bounty: BigDecimal::from_str("500.00000000").unwrap(),
            enabled: true,
            approval_tiers: serde_json::json!([
                {"threshold": 100, "require_approval": true, "approvers": [], "timeout_hours": 24, "notification_channels": []},
                {"threshold": 500, "require_approval": true, "approvers": ["did:key:z6MkApprover"], "timeout_hours": 48, "notification_channels": []}
            ]),
            allowed_delegates: serde_json::json!([]),
            emergency_contact: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_is_enabled() {
        let policy = create_test_policy();
        assert!(policy.is_enabled());
    }

    #[test]
    fn test_get_approval_tiers() {
        let policy = create_test_policy();
        let tiers = policy.get_approval_tiers();
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].threshold, 100.0);
        assert_eq!(tiers[1].threshold, 500.0);
    }

    #[test]
    fn test_requires_approval_below_threshold() {
        let policy = create_test_policy();
        let amount = BigDecimal::from_str("50.00000000").unwrap();
        assert!(policy.requires_approval(&amount).is_none());
    }

    #[test]
    fn test_requires_approval_above_first_threshold() {
        let policy = create_test_policy();
        let amount = BigDecimal::from_str("150.00000000").unwrap();
        let tier = policy.requires_approval(&amount);
        assert!(tier.is_some());
        assert_eq!(tier.unwrap().threshold, 100.0);
    }

    #[test]
    fn test_requires_approval_above_second_threshold() {
        let policy = create_test_policy();
        let amount = BigDecimal::from_str("600.00000000").unwrap();
        let tier = policy.requires_approval(&amount);
        assert!(tier.is_some());
        assert_eq!(tier.unwrap().threshold, 500.0);
    }

    #[test]
    fn test_requires_approval_disabled_policy() {
        let mut policy = create_test_policy();
        policy.enabled = false;
        let amount = BigDecimal::from_str("600.00000000").unwrap();
        assert!(policy.requires_approval(&amount).is_none());
    }

    #[test]
    fn test_exceeds_per_bounty_limit() {
        let policy = create_test_policy();
        let below = BigDecimal::from_str("400.00000000").unwrap();
        let above = BigDecimal::from_str("600.00000000").unwrap();
        assert!(!policy.exceeds_per_bounty_limit(&below));
        assert!(policy.exceeds_per_bounty_limit(&above));
    }

    #[test]
    fn test_operator_did() {
        let policy = create_test_policy();
        assert_eq!(policy.operator_did(), "did:key:z6MkTest");
    }

    #[test]
    fn test_new_default_policy() {
        let policy = NewUserPolicy::default_policy("did:key:z6MkNew".to_string());
        assert_eq!(policy.did, "did:key:z6MkNew");
        assert_eq!(policy.version, "1.0");
        assert!(policy.enabled);
    }

    #[test]
    fn test_new_no_approval_policy() {
        let policy = NewUserPolicy::no_approval_policy("did:key:z6MkNew".to_string());
        assert!(!policy.enabled);
    }

    #[test]
    fn test_serialization() {
        let policy = create_test_policy();
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("did:key:z6MkTest"));
        assert!(json.contains("1.0"));
    }

    #[test]
    fn test_approval_tier_notification_channels() {
        let tier = ApprovalTier {
            threshold: 1000.0,
            require_approval: true,
            approvers: vec!["did:key:z6MkApprover".to_string()],
            timeout_hours: 48,
            notification_channels: vec![
                NotificationChannel {
                    channel_type: NotificationChannelType::Email,
                    target: "admin@example.com".to_string(),
                },
                NotificationChannel {
                    channel_type: NotificationChannelType::Webhook,
                    target: "https://example.com/webhook".to_string(),
                },
            ],
        };

        assert_eq!(tier.notification_channels.len(), 2);
        assert_eq!(tier.notification_channels[0].channel_type, NotificationChannelType::Email);
    }
}
