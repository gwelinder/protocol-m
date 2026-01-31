//! Redemption receipt model for tracking credit redemptions with compute providers.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Represents a redemption receipt recording an M-credit redemption.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RedemptionReceipt {
    /// Unique identifier for this receipt.
    pub id: Uuid,
    /// DID of the user who redeemed credits.
    pub user_did: String,
    /// Reference to the compute provider.
    pub provider_id: Uuid,
    /// Amount of M-credits redeemed.
    pub amount_credits: BigDecimal,
    /// Allocation ID from the provider (if available).
    pub allocation_id: Option<String>,
    /// Additional redemption metadata.
    pub metadata: serde_json::Value,
    /// When this redemption occurred.
    pub created_at: DateTime<Utc>,
}

/// Data required to create a new redemption receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRedemptionReceipt {
    pub user_did: String,
    pub provider_id: Uuid,
    pub amount_credits: BigDecimal,
    pub allocation_id: Option<String>,
    pub metadata: serde_json::Value,
}

impl NewRedemptionReceipt {
    /// Create a new redemption receipt.
    pub fn new(
        user_did: String,
        provider_id: Uuid,
        amount_credits: BigDecimal,
        allocation_id: Option<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            user_did,
            provider_id,
            amount_credits,
            allocation_id,
            metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn test_new_redemption_receipt() {
        let provider_id = Uuid::new_v4();
        let amount = BigDecimal::from_str("50.00000000").unwrap();
        let metadata = json!({
            "usage_quota": "1000 tokens",
            "expires_at": "2026-02-28T23:59:59Z"
        });

        let receipt = NewRedemptionReceipt::new(
            "did:key:z6MkTest123".to_string(),
            provider_id,
            amount.clone(),
            Some("alloc_abc123".to_string()),
            metadata.clone(),
        );

        assert_eq!(receipt.user_did, "did:key:z6MkTest123");
        assert_eq!(receipt.provider_id, provider_id);
        assert_eq!(receipt.amount_credits, amount);
        assert_eq!(receipt.allocation_id, Some("alloc_abc123".to_string()));
        assert_eq!(receipt.metadata["usage_quota"], "1000 tokens");
    }

    #[test]
    fn test_new_redemption_receipt_without_allocation_id() {
        let provider_id = Uuid::new_v4();
        let amount = BigDecimal::from_str("25.00000000").unwrap();

        let receipt = NewRedemptionReceipt::new(
            "did:key:z6MkTest456".to_string(),
            provider_id,
            amount.clone(),
            None,
            json!({}),
        );

        assert!(receipt.allocation_id.is_none());
    }

    #[test]
    fn test_redemption_receipt_serialization() {
        let now = Utc::now();
        let receipt = RedemptionReceipt {
            id: Uuid::new_v4(),
            user_did: "did:key:z6MkTest".to_string(),
            provider_id: Uuid::new_v4(),
            amount_credits: BigDecimal::from_str("100.00000000").unwrap(),
            allocation_id: Some("alloc_test".to_string()),
            metadata: json!({"provider_response": "success"}),
            created_at: now,
        };

        let json = serde_json::to_string(&receipt).unwrap();
        assert!(json.contains("\"user_did\":\"did:key:z6MkTest\""));
        assert!(json.contains("\"allocation_id\":\"alloc_test\""));
    }
}
