//! M-credits account model for tracking token balances.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Represents an M-credits account for a DID holder.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MCreditsAccount {
    /// Unique identifier for this account record.
    pub id: Uuid,
    /// DID of the account holder (did:key:z6Mk...).
    pub did: String,
    /// Current balance of M-credits.
    pub balance: BigDecimal,
    /// Promotional/bonus balance of M-credits.
    pub promo_balance: BigDecimal,
    /// When this account was created.
    pub created_at: DateTime<Utc>,
    /// When this account was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Data required to create a new M-credits account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMCreditsAccount {
    pub did: String,
    pub balance: BigDecimal,
    pub promo_balance: BigDecimal,
}

impl MCreditsAccount {
    /// Returns the total available balance (balance + promo_balance).
    pub fn total_balance(&self) -> BigDecimal {
        &self.balance + &self.promo_balance
    }

    /// Returns true if the account has sufficient balance for the given amount.
    pub fn has_sufficient_balance(&self, amount: &BigDecimal) -> bool {
        &self.total_balance() >= amount
    }
}

impl Default for NewMCreditsAccount {
    fn default() -> Self {
        Self {
            did: String::new(),
            balance: BigDecimal::from(0),
            promo_balance: BigDecimal::from(0),
        }
    }
}

impl NewMCreditsAccount {
    /// Create a new account with just the DID (zero balances).
    pub fn new(did: String) -> Self {
        Self {
            did,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_total_balance() {
        let account = MCreditsAccount {
            id: Uuid::new_v4(),
            did: "did:key:z6MkTest123".to_string(),
            balance: BigDecimal::from_str("100.50000000").unwrap(),
            promo_balance: BigDecimal::from_str("25.25000000").unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(
            account.total_balance(),
            BigDecimal::from_str("125.75000000").unwrap()
        );
    }

    #[test]
    fn test_has_sufficient_balance() {
        let account = MCreditsAccount {
            id: Uuid::new_v4(),
            did: "did:key:z6MkTest123".to_string(),
            balance: BigDecimal::from_str("100.00000000").unwrap(),
            promo_balance: BigDecimal::from_str("0.00000000").unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(account.has_sufficient_balance(&BigDecimal::from_str("50.00000000").unwrap()));
        assert!(account.has_sufficient_balance(&BigDecimal::from_str("100.00000000").unwrap()));
        assert!(!account.has_sufficient_balance(&BigDecimal::from_str("100.00000001").unwrap()));
    }

    #[test]
    fn test_new_account_defaults() {
        let new_account = NewMCreditsAccount::new("did:key:z6MkTest123".to_string());
        assert_eq!(new_account.did, "did:key:z6MkTest123");
        assert_eq!(new_account.balance, BigDecimal::from(0));
        assert_eq!(new_account.promo_balance, BigDecimal::from(0));
    }

    #[test]
    fn test_has_sufficient_balance_with_promo() {
        let account = MCreditsAccount {
            id: Uuid::new_v4(),
            did: "did:key:z6MkTest123".to_string(),
            balance: BigDecimal::from_str("50.00000000").unwrap(),
            promo_balance: BigDecimal::from_str("50.00000000").unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Can cover 100 credits from 50 balance + 50 promo
        assert!(account.has_sufficient_balance(&BigDecimal::from_str("100.00000000").unwrap()));
        assert!(!account.has_sufficient_balance(&BigDecimal::from_str("100.00000001").unwrap()));
    }
}
