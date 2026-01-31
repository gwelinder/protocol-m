// Policy validation for Protocol M
// Validates agent spending limits and approval workflows

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Default values for policy fields
const DEFAULT_MAX_SPEND_PER_DAY: f64 = 1000.0;
const DEFAULT_MAX_SPEND_PER_BOUNTY: f64 = 500.0;
const DEFAULT_ENABLED: bool = true;
const DEFAULT_APPROVAL_TIER_THRESHOLD: f64 = 100.0;
const DEFAULT_TIMEOUT_HOURS: u32 = 24;

/// Protocol M Policy configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Policy {
    /// Policy schema version (must be "1.0")
    pub version: String,

    /// Maximum credits that can be spent in a 24-hour rolling window
    #[serde(default = "default_max_spend_per_day")]
    pub max_spend_per_day: f64,

    /// Maximum credits that can be spent on a single bounty
    #[serde(default = "default_max_spend_per_bounty")]
    pub max_spend_per_bounty: f64,

    /// DIDs that are allowed to act on behalf of this identity
    #[serde(default)]
    pub allowed_delegates: Vec<String>,

    /// Thresholds that require human/operator approval before proceeding
    #[serde(default = "default_approval_tiers")]
    pub approval_tiers: Vec<ApprovalTier>,

    /// Contact information for emergency stop notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emergency_contact: Option<EmergencyContact>,

    /// Whether policy enforcement is active
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Approval tier configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApprovalTier {
    /// Credit amount threshold that triggers this tier
    pub threshold: f64,

    /// Whether approval is required when this threshold is exceeded
    #[serde(default = "default_require_approval")]
    pub require_approval: bool,

    /// DIDs authorized to approve requests at this tier (empty = owner only)
    #[serde(default)]
    pub approvers: Vec<String>,

    /// Hours to wait for approval before auto-rejecting (0 = no timeout)
    #[serde(default = "default_timeout_hours")]
    pub timeout_hours: u32,

    /// Channels to notify when approval is required
    #[serde(default)]
    pub notification_channels: Vec<NotificationChannel>,
}

/// Notification channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationChannel {
    /// Notification channel type
    #[serde(rename = "type")]
    pub channel_type: NotificationChannelType,

    /// Target address/URL for the notification
    pub target: String,
}

/// Notification channel types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationChannelType {
    Email,
    Webhook,
    Slack,
}

/// Emergency contact configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmergencyContact {
    /// Email address for emergency notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Webhook URL for emergency notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook: Option<String>,
}

// Default value functions for serde
fn default_max_spend_per_day() -> f64 {
    DEFAULT_MAX_SPEND_PER_DAY
}

fn default_max_spend_per_bounty() -> f64 {
    DEFAULT_MAX_SPEND_PER_BOUNTY
}

fn default_enabled() -> bool {
    DEFAULT_ENABLED
}

fn default_require_approval() -> bool {
    true
}

fn default_timeout_hours() -> u32 {
    DEFAULT_TIMEOUT_HOURS
}

fn default_approval_tiers() -> Vec<ApprovalTier> {
    vec![ApprovalTier {
        threshold: DEFAULT_APPROVAL_TIER_THRESHOLD,
        require_approval: true,
        approvers: vec![],
        timeout_hours: DEFAULT_TIMEOUT_HOURS,
        notification_channels: vec![],
    }]
}

/// Validates a policy JSON string and returns a Policy struct if valid.
///
/// # Arguments
/// * `policy_json` - JSON string representing the policy
///
/// # Returns
/// * `Ok(Policy)` - The validated policy struct
/// * `Err` - If the policy is invalid (bad JSON, invalid DIDs, negative thresholds, etc.)
///
/// # Validation Rules
/// 1. JSON must be valid and parse into Policy struct
/// 2. Version must be "1.0"
/// 3. All numeric values (thresholds, limits) must be non-negative
/// 4. All DIDs in allowed_delegates must be valid did:key format
/// 5. All DIDs in approval tier approvers must be valid did:key format
/// 6. Approval tier thresholds must be positive (> 0)
pub fn validate_policy(policy_json: &str) -> Result<Policy> {
    // Step 1: Parse JSON
    let policy: Policy = serde_json::from_str(policy_json)
        .map_err(|e| anyhow!("Invalid JSON: {}", e))?;

    // Step 2: Validate version
    if policy.version != "1.0" {
        return Err(anyhow!(
            "Invalid policy version: expected '1.0', got '{}'",
            policy.version
        ));
    }

    // Step 3: Validate numeric values are non-negative
    if policy.max_spend_per_day < 0.0 {
        return Err(anyhow!(
            "max_spend_per_day must be non-negative, got {}",
            policy.max_spend_per_day
        ));
    }

    if policy.max_spend_per_bounty < 0.0 {
        return Err(anyhow!(
            "max_spend_per_bounty must be non-negative, got {}",
            policy.max_spend_per_bounty
        ));
    }

    // Step 4: Validate allowed_delegates DIDs
    for (i, did) in policy.allowed_delegates.iter().enumerate() {
        validate_did(did).map_err(|e| {
            anyhow!("Invalid DID in allowed_delegates[{}]: {}", i, e)
        })?;
    }

    // Step 5: Validate approval_tiers
    for (i, tier) in policy.approval_tiers.iter().enumerate() {
        // Threshold must be positive
        if tier.threshold <= 0.0 {
            return Err(anyhow!(
                "approval_tiers[{}].threshold must be positive, got {}",
                i,
                tier.threshold
            ));
        }

        // Validate approver DIDs
        for (j, did) in tier.approvers.iter().enumerate() {
            validate_did(did).map_err(|e| {
                anyhow!("Invalid DID in approval_tiers[{}].approvers[{}]: {}", i, j, e)
            })?;
        }
    }

    Ok(policy)
}

/// Validates that a string is a valid did:key DID.
///
/// Uses openclaw_crypto::did_to_verifying_key to validate the DID format
/// and verify that it encodes a valid Ed25519 public key.
fn validate_did(did: &str) -> Result<()> {
    openclaw_crypto::did_to_verifying_key(did)
        .map_err(|e| anyhow!("{}", e))?;
    Ok(())
}

impl Policy {
    /// Creates a new minimal policy with just the version field.
    /// All other fields use their defaults.
    pub fn minimal() -> Self {
        Policy {
            version: "1.0".to_string(),
            max_spend_per_day: default_max_spend_per_day(),
            max_spend_per_bounty: default_max_spend_per_bounty(),
            allowed_delegates: vec![],
            approval_tiers: default_approval_tiers(),
            emergency_contact: None,
            enabled: default_enabled(),
        }
    }

    /// Returns a summary of the policy for display.
    pub fn summary(&self) -> String {
        let mut lines = vec![];
        lines.push(format!("Version: {}", self.version));
        lines.push(format!("Enabled: {}", self.enabled));
        lines.push(format!("Max spend/day: {} credits", self.max_spend_per_day));
        lines.push(format!("Max spend/bounty: {} credits", self.max_spend_per_bounty));
        lines.push(format!("Delegates: {}", self.allowed_delegates.len()));
        lines.push(format!("Approval tiers: {}", self.approval_tiers.len()));
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Valid test DID (from golden vector)
    const VALID_DID: &str = "did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw";

    #[test]
    fn test_validate_minimal_policy() {
        let json = r#"{"version": "1.0"}"#;
        let policy = validate_policy(json).expect("should validate minimal policy");

        assert_eq!(policy.version, "1.0");
        assert_eq!(policy.max_spend_per_day, DEFAULT_MAX_SPEND_PER_DAY);
        assert_eq!(policy.max_spend_per_bounty, DEFAULT_MAX_SPEND_PER_BOUNTY);
        assert!(policy.allowed_delegates.is_empty());
        assert_eq!(policy.approval_tiers.len(), 1);
        assert!(policy.enabled);
    }

    #[test]
    fn test_validate_full_policy() {
        let json = format!(
            r#"{{
                "version": "1.0",
                "max_spend_per_day": 500,
                "max_spend_per_bounty": 100,
                "allowed_delegates": ["{}"],
                "approval_tiers": [
                    {{
                        "threshold": 50,
                        "require_approval": true,
                        "approvers": [],
                        "timeout_hours": 12,
                        "notification_channels": [
                            {{"type": "email", "target": "test@example.com"}}
                        ]
                    }}
                ],
                "emergency_contact": {{
                    "email": "emergency@example.com"
                }},
                "enabled": true
            }}"#,
            VALID_DID
        );

        let policy = validate_policy(&json).expect("should validate full policy");

        assert_eq!(policy.version, "1.0");
        assert_eq!(policy.max_spend_per_day, 500.0);
        assert_eq!(policy.max_spend_per_bounty, 100.0);
        assert_eq!(policy.allowed_delegates.len(), 1);
        assert_eq!(policy.allowed_delegates[0], VALID_DID);
        assert_eq!(policy.approval_tiers.len(), 1);
        assert_eq!(policy.approval_tiers[0].threshold, 50.0);
        assert_eq!(policy.approval_tiers[0].timeout_hours, 12);
        assert_eq!(policy.approval_tiers[0].notification_channels.len(), 1);
        assert!(policy.emergency_contact.is_some());
        assert!(policy.enabled);
    }

    #[test]
    fn test_validate_invalid_json() {
        let json = "not valid json";
        let result = validate_policy(json);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid JSON"));
    }

    #[test]
    fn test_validate_invalid_version() {
        let json = r#"{"version": "2.0"}"#;
        let result = validate_policy(json);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid policy version"));
    }

    #[test]
    fn test_validate_missing_version() {
        let json = r#"{"max_spend_per_day": 100}"#;
        let result = validate_policy(json);

        assert!(result.is_err());
        // serde_json error for missing required field
        assert!(result.unwrap_err().to_string().contains("version"));
    }

    #[test]
    fn test_validate_negative_max_spend_per_day() {
        let json = r#"{"version": "1.0", "max_spend_per_day": -100}"#;
        let result = validate_policy(json);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_spend_per_day must be non-negative"));
    }

    #[test]
    fn test_validate_negative_max_spend_per_bounty() {
        let json = r#"{"version": "1.0", "max_spend_per_bounty": -50}"#;
        let result = validate_policy(json);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_spend_per_bounty must be non-negative"));
    }

    #[test]
    fn test_validate_invalid_delegate_did() {
        let json = r#"{
            "version": "1.0",
            "allowed_delegates": ["not-a-valid-did"]
        }"#;
        let result = validate_policy(json);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid DID in allowed_delegates"));
    }

    #[test]
    fn test_validate_invalid_approver_did() {
        let json = r#"{
            "version": "1.0",
            "approval_tiers": [
                {
                    "threshold": 100,
                    "approvers": ["invalid-did"]
                }
            ]
        }"#;
        let result = validate_policy(json);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid DID in approval_tiers"));
    }

    #[test]
    fn test_validate_zero_threshold() {
        let json = r#"{
            "version": "1.0",
            "approval_tiers": [
                {"threshold": 0}
            ]
        }"#;
        let result = validate_policy(json);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("threshold must be positive"));
    }

    #[test]
    fn test_validate_negative_threshold() {
        let json = r#"{
            "version": "1.0",
            "approval_tiers": [
                {"threshold": -10}
            ]
        }"#;
        let result = validate_policy(json);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("threshold must be positive"));
    }

    #[test]
    fn test_validate_policy_with_valid_dids() {
        let json = format!(
            r#"{{
                "version": "1.0",
                "allowed_delegates": ["{}"],
                "approval_tiers": [
                    {{
                        "threshold": 100,
                        "approvers": ["{}"]
                    }}
                ]
            }}"#,
            VALID_DID, VALID_DID
        );
        let policy = validate_policy(&json).expect("should validate with valid DIDs");

        assert_eq!(policy.allowed_delegates.len(), 1);
        assert_eq!(policy.approval_tiers[0].approvers.len(), 1);
    }

    #[test]
    fn test_validate_empty_approval_tiers() {
        let json = r#"{
            "version": "1.0",
            "approval_tiers": []
        }"#;
        let policy = validate_policy(json).expect("should validate with empty approval tiers");

        assert!(policy.approval_tiers.is_empty());
    }

    #[test]
    fn test_validate_disabled_policy() {
        let json = r#"{"version": "1.0", "enabled": false}"#;
        let policy = validate_policy(json).expect("should validate disabled policy");

        assert!(!policy.enabled);
    }

    #[test]
    fn test_policy_minimal() {
        let policy = Policy::minimal();

        assert_eq!(policy.version, "1.0");
        assert_eq!(policy.max_spend_per_day, DEFAULT_MAX_SPEND_PER_DAY);
        assert!(policy.enabled);
    }

    #[test]
    fn test_policy_summary() {
        let policy = Policy::minimal();
        let summary = policy.summary();

        assert!(summary.contains("Version: 1.0"));
        assert!(summary.contains("Enabled: true"));
        assert!(summary.contains("Max spend/day:"));
    }

    #[test]
    fn test_notification_channel_types() {
        let json = r#"{
            "version": "1.0",
            "approval_tiers": [
                {
                    "threshold": 100,
                    "notification_channels": [
                        {"type": "email", "target": "a@b.com"},
                        {"type": "webhook", "target": "https://example.com/hook"},
                        {"type": "slack", "target": "https://hooks.slack.com/xxx"}
                    ]
                }
            ]
        }"#;
        let policy = validate_policy(json).expect("should validate notification channels");

        let channels = &policy.approval_tiers[0].notification_channels;
        assert_eq!(channels.len(), 3);
        assert_eq!(channels[0].channel_type, NotificationChannelType::Email);
        assert_eq!(channels[1].channel_type, NotificationChannelType::Webhook);
        assert_eq!(channels[2].channel_type, NotificationChannelType::Slack);
    }

    #[test]
    fn test_emergency_contact() {
        let json = r#"{
            "version": "1.0",
            "emergency_contact": {
                "email": "emergency@example.com",
                "webhook": "https://example.com/emergency"
            }
        }"#;
        let policy = validate_policy(json).expect("should validate emergency contact");

        let contact = policy.emergency_contact.unwrap();
        assert_eq!(contact.email, Some("emergency@example.com".to_string()));
        assert_eq!(contact.webhook, Some("https://example.com/emergency".to_string()));
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let original = Policy::minimal();
        let json = serde_json::to_string(&original).expect("should serialize");
        let deserialized: Policy = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(original, deserialized);
    }
}
