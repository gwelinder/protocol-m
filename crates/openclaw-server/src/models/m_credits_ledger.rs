//! M-credits ledger model for event sourcing of credit transactions.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

/// Types of M-credit ledger events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "m_credits_event_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MCreditsEventType {
    /// New credits created (from purchase or reward).
    Mint,
    /// Credits destroyed (refund or expiry).
    Burn,
    /// Credits moved between DIDs.
    Transfer,
    /// Credits reserved (pending transaction).
    Hold,
    /// Credits released from hold.
    Release,
}

/// Represents an immutable ledger entry for M-credit transactions.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MCreditsLedger {
    /// Unique identifier for this ledger entry.
    pub id: Uuid,
    /// Type of credit event.
    pub event_type: MCreditsEventType,
    /// Source DID (null for mint events).
    pub from_did: Option<String>,
    /// Destination DID (null for burn events).
    pub to_did: Option<String>,
    /// Amount of credits in this transaction.
    pub amount: BigDecimal,
    /// Additional transaction metadata (JSONB).
    pub metadata: serde_json::Value,
    /// When this event occurred (immutable).
    pub created_at: DateTime<Utc>,
}

/// Data required to create a new ledger entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMCreditsLedger {
    pub event_type: MCreditsEventType,
    pub from_did: Option<String>,
    pub to_did: Option<String>,
    pub amount: BigDecimal,
    pub metadata: serde_json::Value,
}

impl NewMCreditsLedger {
    /// Create a new mint event (credits entering the system).
    pub fn mint(to_did: String, amount: BigDecimal, metadata: serde_json::Value) -> Self {
        Self {
            event_type: MCreditsEventType::Mint,
            from_did: None,
            to_did: Some(to_did),
            amount,
            metadata,
        }
    }

    /// Create a new burn event (credits leaving the system).
    pub fn burn(from_did: String, amount: BigDecimal, metadata: serde_json::Value) -> Self {
        Self {
            event_type: MCreditsEventType::Burn,
            from_did: Some(from_did),
            to_did: None,
            amount,
            metadata,
        }
    }

    /// Create a new transfer event (credits moving between DIDs).
    pub fn transfer(
        from_did: String,
        to_did: String,
        amount: BigDecimal,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            event_type: MCreditsEventType::Transfer,
            from_did: Some(from_did),
            to_did: Some(to_did),
            amount,
            metadata,
        }
    }

    /// Create a new hold event (credits reserved for pending transaction).
    pub fn hold(from_did: String, amount: BigDecimal, metadata: serde_json::Value) -> Self {
        Self {
            event_type: MCreditsEventType::Hold,
            from_did: Some(from_did),
            to_did: None,
            amount,
            metadata,
        }
    }

    /// Create a new release event (credits released from hold).
    pub fn release(to_did: String, amount: BigDecimal, metadata: serde_json::Value) -> Self {
        Self {
            event_type: MCreditsEventType::Release,
            from_did: None,
            to_did: Some(to_did),
            amount,
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
    fn test_event_type_serialization() {
        assert_eq!(
            serde_json::to_string(&MCreditsEventType::Mint).unwrap(),
            "\"mint\""
        );
        assert_eq!(
            serde_json::to_string(&MCreditsEventType::Burn).unwrap(),
            "\"burn\""
        );
        assert_eq!(
            serde_json::to_string(&MCreditsEventType::Transfer).unwrap(),
            "\"transfer\""
        );
        assert_eq!(
            serde_json::to_string(&MCreditsEventType::Hold).unwrap(),
            "\"hold\""
        );
        assert_eq!(
            serde_json::to_string(&MCreditsEventType::Release).unwrap(),
            "\"release\""
        );
    }

    #[test]
    fn test_event_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<MCreditsEventType>("\"mint\"").unwrap(),
            MCreditsEventType::Mint
        );
        assert_eq!(
            serde_json::from_str::<MCreditsEventType>("\"transfer\"").unwrap(),
            MCreditsEventType::Transfer
        );
    }

    #[test]
    fn test_new_mint_event() {
        let amount = BigDecimal::from_str("100.50000000").unwrap();
        let metadata = json!({"invoice_id": "inv_123", "reason": "purchase"});
        let event = NewMCreditsLedger::mint(
            "did:key:z6MkRecipient".to_string(),
            amount.clone(),
            metadata.clone(),
        );

        assert_eq!(event.event_type, MCreditsEventType::Mint);
        assert!(event.from_did.is_none());
        assert_eq!(event.to_did, Some("did:key:z6MkRecipient".to_string()));
        assert_eq!(event.amount, amount);
        assert_eq!(event.metadata, metadata);
    }

    #[test]
    fn test_new_burn_event() {
        let amount = BigDecimal::from_str("50.00000000").unwrap();
        let metadata = json!({"reason": "refund"});
        let event =
            NewMCreditsLedger::burn("did:key:z6MkSender".to_string(), amount.clone(), metadata);

        assert_eq!(event.event_type, MCreditsEventType::Burn);
        assert_eq!(event.from_did, Some("did:key:z6MkSender".to_string()));
        assert!(event.to_did.is_none());
        assert_eq!(event.amount, amount);
    }

    #[test]
    fn test_new_transfer_event() {
        let amount = BigDecimal::from_str("25.00000000").unwrap();
        let metadata = json!({"note": "payment for service"});
        let event = NewMCreditsLedger::transfer(
            "did:key:z6MkSender".to_string(),
            "did:key:z6MkRecipient".to_string(),
            amount.clone(),
            metadata,
        );

        assert_eq!(event.event_type, MCreditsEventType::Transfer);
        assert_eq!(event.from_did, Some("did:key:z6MkSender".to_string()));
        assert_eq!(event.to_did, Some("did:key:z6MkRecipient".to_string()));
        assert_eq!(event.amount, amount);
    }

    #[test]
    fn test_new_hold_event() {
        let amount = BigDecimal::from_str("10.00000000").unwrap();
        let metadata = json!({"pending_tx": "tx_456"});
        let event = NewMCreditsLedger::hold(
            "did:key:z6MkHolder".to_string(),
            amount.clone(),
            metadata,
        );

        assert_eq!(event.event_type, MCreditsEventType::Hold);
        assert_eq!(event.from_did, Some("did:key:z6MkHolder".to_string()));
        assert!(event.to_did.is_none());
    }

    #[test]
    fn test_new_release_event() {
        let amount = BigDecimal::from_str("10.00000000").unwrap();
        let metadata = json!({"released_from": "tx_456"});
        let event = NewMCreditsLedger::release(
            "did:key:z6MkHolder".to_string(),
            amount.clone(),
            metadata,
        );

        assert_eq!(event.event_type, MCreditsEventType::Release);
        assert!(event.from_did.is_none());
        assert_eq!(event.to_did, Some("did:key:z6MkHolder".to_string()));
    }
}
