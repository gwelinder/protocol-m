//! Purchase invoice model for tracking credit purchases.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

/// Supported payment providers for credit purchases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "payment_provider", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentProvider {
    /// Stripe payments
    Stripe,
    /// USDC stablecoin payments
    Usdc,
    /// Apple Pay (via Stripe)
    ApplePay,
    /// Manual/admin credits
    Manual,
}

/// Possible states of a purchase invoice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "invoice_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InvoiceStatus {
    /// Payment initiated but not confirmed
    Pending,
    /// Payment confirmed, credits minted
    Completed,
    /// Payment failed or cancelled
    Failed,
}

/// Represents a purchase invoice for M-credits.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PurchaseInvoice {
    /// Unique identifier for this invoice.
    pub id: Uuid,
    /// User who initiated the purchase.
    pub user_id: Uuid,
    /// Amount charged in USD.
    pub amount_usd: BigDecimal,
    /// Amount of M-credits to be minted.
    pub amount_credits: BigDecimal,
    /// Payment provider used.
    pub payment_provider: PaymentProvider,
    /// External reference from payment provider.
    pub external_ref: Option<String>,
    /// Current invoice status.
    pub status: InvoiceStatus,
    /// When this invoice was created.
    pub created_at: DateTime<Utc>,
    /// When this invoice was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Data required to create a new purchase invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPurchaseInvoice {
    pub user_id: Uuid,
    pub amount_usd: BigDecimal,
    pub amount_credits: BigDecimal,
    pub payment_provider: PaymentProvider,
    pub external_ref: Option<String>,
}

impl PurchaseInvoice {
    /// Check if the invoice is still pending.
    pub fn is_pending(&self) -> bool {
        self.status == InvoiceStatus::Pending
    }

    /// Check if the invoice is completed.
    pub fn is_completed(&self) -> bool {
        self.status == InvoiceStatus::Completed
    }

    /// Check if the invoice has failed.
    pub fn is_failed(&self) -> bool {
        self.status == InvoiceStatus::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_payment_provider_serialization() {
        assert_eq!(
            serde_json::to_string(&PaymentProvider::Stripe).unwrap(),
            "\"stripe\""
        );
        assert_eq!(
            serde_json::to_string(&PaymentProvider::Usdc).unwrap(),
            "\"usdc\""
        );
        assert_eq!(
            serde_json::to_string(&PaymentProvider::ApplePay).unwrap(),
            "\"apple_pay\""
        );
        assert_eq!(
            serde_json::to_string(&PaymentProvider::Manual).unwrap(),
            "\"manual\""
        );
    }

    #[test]
    fn test_payment_provider_deserialization() {
        assert_eq!(
            serde_json::from_str::<PaymentProvider>("\"stripe\"").unwrap(),
            PaymentProvider::Stripe
        );
        assert_eq!(
            serde_json::from_str::<PaymentProvider>("\"apple_pay\"").unwrap(),
            PaymentProvider::ApplePay
        );
    }

    #[test]
    fn test_invoice_status_serialization() {
        assert_eq!(
            serde_json::to_string(&InvoiceStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&InvoiceStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&InvoiceStatus::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[test]
    fn test_invoice_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<InvoiceStatus>("\"pending\"").unwrap(),
            InvoiceStatus::Pending
        );
        assert_eq!(
            serde_json::from_str::<InvoiceStatus>("\"completed\"").unwrap(),
            InvoiceStatus::Completed
        );
    }

    #[test]
    fn test_new_purchase_invoice_creation() {
        let user_id = Uuid::new_v4();
        let amount_usd = BigDecimal::from_str("10.00").unwrap();
        let amount_credits = BigDecimal::from_str("1000.00000000").unwrap();

        let new_invoice = NewPurchaseInvoice {
            user_id,
            amount_usd: amount_usd.clone(),
            amount_credits: amount_credits.clone(),
            payment_provider: PaymentProvider::Stripe,
            external_ref: Some("pi_test123".to_string()),
        };

        assert_eq!(new_invoice.user_id, user_id);
        assert_eq!(new_invoice.amount_usd, amount_usd);
        assert_eq!(new_invoice.amount_credits, amount_credits);
        assert_eq!(new_invoice.payment_provider, PaymentProvider::Stripe);
        assert_eq!(new_invoice.external_ref, Some("pi_test123".to_string()));
    }

    #[test]
    fn test_invoice_status_helpers() {
        let now = Utc::now();
        let invoice = PurchaseInvoice {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            amount_usd: BigDecimal::from_str("10.00").unwrap(),
            amount_credits: BigDecimal::from_str("1000.00000000").unwrap(),
            payment_provider: PaymentProvider::Stripe,
            external_ref: None,
            status: InvoiceStatus::Pending,
            created_at: now,
            updated_at: now,
        };

        assert!(invoice.is_pending());
        assert!(!invoice.is_completed());
        assert!(!invoice.is_failed());
    }
}
