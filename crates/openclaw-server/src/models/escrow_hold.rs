//! Escrow hold model for Protocol M bounty marketplace.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

/// Possible states of an escrow hold in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "escrow_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum EscrowStatus {
    /// Funds are currently locked in escrow.
    Held,
    /// Funds have been released to the bounty recipient.
    Released,
    /// Funds have been returned to the holder (bounty cancelled).
    Cancelled,
}

/// Represents an escrow hold for a bounty in Protocol M.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EscrowHold {
    /// Unique identifier for this escrow hold.
    pub id: Uuid,
    /// Reference to the bounty this escrow is for.
    pub bounty_id: Uuid,
    /// DID of the agent/user who funded the escrow.
    pub holder_did: String,
    /// Amount of M-credits held in escrow.
    pub amount: BigDecimal,
    /// Current status of the escrow.
    pub status: EscrowStatus,
    /// When this escrow hold was created.
    pub created_at: DateTime<Utc>,
    /// When the escrow was released or cancelled (null if still held).
    pub released_at: Option<DateTime<Utc>>,
}

/// Data required to create a new escrow hold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEscrowHold {
    pub bounty_id: Uuid,
    pub holder_did: String,
    pub amount: BigDecimal,
}

impl EscrowHold {
    /// Check if the escrow is currently held.
    pub fn is_held(&self) -> bool {
        self.status == EscrowStatus::Held
    }

    /// Check if the escrow has been released.
    pub fn is_released(&self) -> bool {
        self.status == EscrowStatus::Released
    }

    /// Check if the escrow has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.status == EscrowStatus::Cancelled
    }

    /// Check if the escrow is finalized (released or cancelled).
    pub fn is_finalized(&self) -> bool {
        self.is_released() || self.is_cancelled()
    }
}

impl NewEscrowHold {
    /// Create a new escrow hold for a bounty.
    pub fn new(bounty_id: Uuid, holder_did: String, amount: BigDecimal) -> Self {
        Self {
            bounty_id,
            holder_did,
            amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_escrow_status_serialization() {
        assert_eq!(
            serde_json::to_string(&EscrowStatus::Held).unwrap(),
            "\"held\""
        );
        assert_eq!(
            serde_json::to_string(&EscrowStatus::Released).unwrap(),
            "\"released\""
        );
        assert_eq!(
            serde_json::to_string(&EscrowStatus::Cancelled).unwrap(),
            "\"cancelled\""
        );
    }

    #[test]
    fn test_escrow_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<EscrowStatus>("\"held\"").unwrap(),
            EscrowStatus::Held
        );
        assert_eq!(
            serde_json::from_str::<EscrowStatus>("\"released\"").unwrap(),
            EscrowStatus::Released
        );
        assert_eq!(
            serde_json::from_str::<EscrowStatus>("\"cancelled\"").unwrap(),
            EscrowStatus::Cancelled
        );
    }

    #[test]
    fn test_new_escrow_hold() {
        let bounty_id = Uuid::new_v4();
        let holder_did = "did:key:z6MkTest".to_string();
        let amount = BigDecimal::from_str("100.00000000").unwrap();

        let new_escrow = NewEscrowHold::new(bounty_id, holder_did.clone(), amount.clone());

        assert_eq!(new_escrow.bounty_id, bounty_id);
        assert_eq!(new_escrow.holder_did, holder_did);
        assert_eq!(new_escrow.amount, amount);
    }

    #[test]
    fn test_escrow_hold_status_helpers() {
        let now = Utc::now();
        let bounty_id = Uuid::new_v4();

        let escrow_held = EscrowHold {
            id: Uuid::new_v4(),
            bounty_id,
            holder_did: "did:key:z6MkTest".to_string(),
            amount: BigDecimal::from_str("100.00000000").unwrap(),
            status: EscrowStatus::Held,
            created_at: now,
            released_at: None,
        };

        assert!(escrow_held.is_held());
        assert!(!escrow_held.is_released());
        assert!(!escrow_held.is_cancelled());
        assert!(!escrow_held.is_finalized());

        let escrow_released = EscrowHold {
            status: EscrowStatus::Released,
            released_at: Some(now),
            ..escrow_held.clone()
        };

        assert!(!escrow_released.is_held());
        assert!(escrow_released.is_released());
        assert!(!escrow_released.is_cancelled());
        assert!(escrow_released.is_finalized());

        let escrow_cancelled = EscrowHold {
            status: EscrowStatus::Cancelled,
            released_at: Some(now),
            ..escrow_held
        };

        assert!(!escrow_cancelled.is_held());
        assert!(!escrow_cancelled.is_released());
        assert!(escrow_cancelled.is_cancelled());
        assert!(escrow_cancelled.is_finalized());
    }

    #[test]
    fn test_escrow_hold_serialization() {
        let now = Utc::now();
        let bounty_id = Uuid::new_v4();
        let escrow_id = Uuid::new_v4();

        let escrow = EscrowHold {
            id: escrow_id,
            bounty_id,
            holder_did: "did:key:z6MkTest".to_string(),
            amount: BigDecimal::from_str("100.00000000").unwrap(),
            status: EscrowStatus::Held,
            created_at: now,
            released_at: None,
        };

        let json = serde_json::to_value(&escrow).unwrap();

        assert_eq!(json["id"], escrow_id.to_string());
        assert_eq!(json["bounty_id"], bounty_id.to_string());
        assert_eq!(json["holder_did"], "did:key:z6MkTest");
        assert_eq!(json["status"], "held");
        assert!(json["released_at"].is_null());
    }
}
