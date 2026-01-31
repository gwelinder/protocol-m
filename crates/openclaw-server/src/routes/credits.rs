//! Credit purchase and management endpoints.

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{InvoiceStatus, NewPurchaseInvoice, PaymentProvider, PurchaseInvoice};

/// Credit rate: 1 USD = 100 M-credits
/// This is a placeholder rate - in production, this would be configurable
/// or fetched from a pricing service.
const CREDITS_PER_USD: &str = "100.00000000";

/// Minimum purchase amount in USD.
const MIN_PURCHASE_USD: &str = "1.00";

/// Maximum purchase amount in USD.
const MAX_PURCHASE_USD: &str = "10000.00";

/// Request body for purchasing credits.
/// Note: In a real implementation, the user_id would come from authentication.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseCreditsRequest {
    /// The user ID making the purchase.
    /// In production, this would be extracted from auth token.
    pub user_id: Uuid,
    /// Amount to charge in USD.
    pub amount_usd: String,
    /// Optional payment provider preference.
    /// Defaults to Stripe if not specified.
    #[serde(default)]
    pub payment_provider: Option<String>,
}

/// Response for successful purchase initiation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseCreditsResponse {
    /// The invoice ID for tracking this purchase.
    pub invoice_id: Uuid,
    /// Amount in USD.
    pub amount_usd: String,
    /// Credits that will be minted upon payment completion.
    pub amount_credits: String,
    /// Current invoice status (pending).
    pub status: InvoiceStatus,
    /// URL for completing payment (Stripe Checkout session URL).
    /// Note: This is a placeholder - real Stripe integration would
    /// create an actual checkout session.
    pub payment_url: String,
}

/// Creates the credits router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/purchase", post(purchase_credits))
        .with_state(pool)
}

/// Calculates the number of credits for a given USD amount.
fn calculate_credits(amount_usd: &BigDecimal) -> BigDecimal {
    let rate = BigDecimal::from_str(CREDITS_PER_USD).unwrap();
    amount_usd * rate
}

/// Parses and validates the payment provider.
fn parse_payment_provider(provider: Option<&str>) -> Result<PaymentProvider, AppError> {
    match provider {
        None => Ok(PaymentProvider::Stripe),
        Some("stripe") => Ok(PaymentProvider::Stripe),
        Some("usdc") => Ok(PaymentProvider::Usdc),
        Some("apple_pay") => Ok(PaymentProvider::ApplePay),
        Some(other) => Err(AppError::BadRequest(format!(
            "Unsupported payment provider: {}. Supported: stripe, usdc, apple_pay",
            other
        ))),
    }
}

/// Validates the purchase amount is within acceptable bounds.
fn validate_amount(amount_usd: &BigDecimal) -> Result<(), AppError> {
    let min = BigDecimal::from_str(MIN_PURCHASE_USD).unwrap();
    let max = BigDecimal::from_str(MAX_PURCHASE_USD).unwrap();

    if amount_usd < &min {
        return Err(AppError::BadRequest(format!(
            "Minimum purchase amount is ${} USD",
            MIN_PURCHASE_USD
        )));
    }

    if amount_usd > &max {
        return Err(AppError::BadRequest(format!(
            "Maximum purchase amount is ${} USD",
            MAX_PURCHASE_USD
        )));
    }

    // Check amount is positive
    if amount_usd <= &BigDecimal::from(0) {
        return Err(AppError::BadRequest(
            "Amount must be positive".to_string()
        ));
    }

    Ok(())
}

/// Generates a placeholder Stripe checkout URL.
/// In production, this would:
/// 1. Create a Stripe Checkout Session with the Stripe API
/// 2. Return the session URL for the customer to complete payment
/// 3. Store the session ID as external_ref in the invoice
fn generate_checkout_url(invoice_id: &Uuid, _amount_usd: &BigDecimal) -> String {
    // Placeholder URL - in production this would be a real Stripe Checkout URL
    format!(
        "https://checkout.stripe.com/placeholder?invoice={}",
        invoice_id
    )
}

/// POST /api/v1/credits/purchase
///
/// Initiates a credit purchase.
/// Creates a pending invoice and returns a payment URL for the user
/// to complete the purchase.
///
/// The credits are NOT minted until payment is confirmed via webhook (US-012E).
async fn purchase_credits(
    State(pool): State<PgPool>,
    Json(request): Json<PurchaseCreditsRequest>,
) -> Result<Json<PurchaseCreditsResponse>, AppError> {
    // Step 1: Parse and validate amount
    let amount_usd = BigDecimal::from_str(&request.amount_usd).map_err(|e| {
        AppError::BadRequest(format!("Invalid amount format: {}", e))
    })?;

    validate_amount(&amount_usd)?;

    // Step 2: Parse payment provider
    let payment_provider = parse_payment_provider(request.payment_provider.as_deref())?;

    // Step 3: Calculate credits to be minted
    let amount_credits = calculate_credits(&amount_usd);

    // Step 4: Generate invoice ID
    let invoice_id = Uuid::new_v4();

    // Step 5: Create placeholder checkout URL
    // In production, this would create a real Stripe Checkout session
    let payment_url = generate_checkout_url(&invoice_id, &amount_usd);

    // For now, we use the invoice ID as the external ref.
    // In production, this would be the Stripe session ID.
    let external_ref = format!("checkout_placeholder_{}", invoice_id);

    // Step 6: Create new invoice
    let new_invoice = NewPurchaseInvoice {
        user_id: request.user_id,
        amount_usd: amount_usd.clone(),
        amount_credits: amount_credits.clone(),
        payment_provider,
        external_ref: Some(external_ref),
    };

    // Step 7: Insert invoice into database with status=pending
    let invoice: PurchaseInvoice = sqlx::query_as(
        r#"
        INSERT INTO purchase_invoices (id, user_id, amount_usd, amount_credits, payment_provider, external_ref, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
        RETURNING id, user_id, amount_usd, amount_credits, payment_provider, external_ref, status, created_at, updated_at
        "#,
    )
    .bind(invoice_id)
    .bind(new_invoice.user_id)
    .bind(&new_invoice.amount_usd)
    .bind(&new_invoice.amount_credits)
    .bind(new_invoice.payment_provider)
    .bind(&new_invoice.external_ref)
    .bind(InvoiceStatus::Pending)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create invoice: {}", e)))?;

    // Step 8: Return response with payment URL
    Ok(Json(PurchaseCreditsResponse {
        invoice_id: invoice.id,
        amount_usd: invoice.amount_usd.to_string(),
        amount_credits: invoice.amount_credits.to_string(),
        status: invoice.status,
        payment_url,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_credits() {
        let usd = BigDecimal::from_str("10.00").unwrap();
        let credits = calculate_credits(&usd);
        assert_eq!(credits, BigDecimal::from_str("1000.00000000").unwrap());
    }

    #[test]
    fn test_calculate_credits_fractional() {
        let usd = BigDecimal::from_str("1.50").unwrap();
        let credits = calculate_credits(&usd);
        assert_eq!(credits, BigDecimal::from_str("150.00000000").unwrap());
    }

    #[test]
    fn test_calculate_credits_large_amount() {
        let usd = BigDecimal::from_str("1000.00").unwrap();
        let credits = calculate_credits(&usd);
        assert_eq!(credits, BigDecimal::from_str("100000.00000000").unwrap());
    }

    #[test]
    fn test_parse_payment_provider_default() {
        let result = parse_payment_provider(None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PaymentProvider::Stripe);
    }

    #[test]
    fn test_parse_payment_provider_stripe() {
        let result = parse_payment_provider(Some("stripe"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PaymentProvider::Stripe);
    }

    #[test]
    fn test_parse_payment_provider_usdc() {
        let result = parse_payment_provider(Some("usdc"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PaymentProvider::Usdc);
    }

    #[test]
    fn test_parse_payment_provider_apple_pay() {
        let result = parse_payment_provider(Some("apple_pay"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PaymentProvider::ApplePay);
    }

    #[test]
    fn test_parse_payment_provider_invalid() {
        let result = parse_payment_provider(Some("bitcoin"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported payment provider"));
    }

    #[test]
    fn test_validate_amount_valid() {
        let amount = BigDecimal::from_str("10.00").unwrap();
        let result = validate_amount(&amount);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_amount_minimum() {
        let amount = BigDecimal::from_str("1.00").unwrap();
        let result = validate_amount(&amount);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_amount_maximum() {
        let amount = BigDecimal::from_str("10000.00").unwrap();
        let result = validate_amount(&amount);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_amount_below_minimum() {
        let amount = BigDecimal::from_str("0.50").unwrap();
        let result = validate_amount(&amount);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Minimum purchase amount"));
    }

    #[test]
    fn test_validate_amount_above_maximum() {
        let amount = BigDecimal::from_str("10001.00").unwrap();
        let result = validate_amount(&amount);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Maximum purchase amount"));
    }

    #[test]
    fn test_validate_amount_zero() {
        let amount = BigDecimal::from_str("0").unwrap();
        let result = validate_amount(&amount);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Minimum purchase amount"));
    }

    #[test]
    fn test_validate_amount_negative() {
        let amount = BigDecimal::from_str("-10.00").unwrap();
        let result = validate_amount(&amount);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_checkout_url() {
        let invoice_id = Uuid::new_v4();
        let amount = BigDecimal::from_str("10.00").unwrap();
        let url = generate_checkout_url(&invoice_id, &amount);
        assert!(url.starts_with("https://checkout.stripe.com/placeholder"));
        assert!(url.contains(&invoice_id.to_string()));
    }

    #[test]
    fn test_purchase_request_deserialization() {
        let json = r#"{
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "amountUsd": "50.00"
        }"#;

        let request: PurchaseCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.user_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(request.amount_usd, "50.00");
        assert!(request.payment_provider.is_none());
    }

    #[test]
    fn test_purchase_request_with_provider() {
        let json = r#"{
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "amountUsd": "50.00",
            "paymentProvider": "usdc"
        }"#;

        let request: PurchaseCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.payment_provider, Some("usdc".to_string()));
    }

    #[test]
    fn test_purchase_response_serialization() {
        let response = PurchaseCreditsResponse {
            invoice_id: Uuid::new_v4(),
            amount_usd: "50.00".to_string(),
            amount_credits: "5000.00000000".to_string(),
            status: InvoiceStatus::Pending,
            payment_url: "https://checkout.stripe.com/test".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"invoiceId\":"));
        assert!(json.contains("\"amountUsd\":"));
        assert!(json.contains("\"amountCredits\":"));
        assert!(json.contains("\"status\":\"pending\""));
        assert!(json.contains("\"paymentUrl\":"));
    }

    #[test]
    fn test_new_purchase_invoice_creation() {
        let user_id = Uuid::new_v4();
        let amount_usd = BigDecimal::from_str("25.00").unwrap();
        let amount_credits = calculate_credits(&amount_usd);

        let new_invoice = NewPurchaseInvoice {
            user_id,
            amount_usd: amount_usd.clone(),
            amount_credits: amount_credits.clone(),
            payment_provider: PaymentProvider::Stripe,
            external_ref: Some("cs_test_123".to_string()),
        };

        assert_eq!(new_invoice.user_id, user_id);
        assert_eq!(new_invoice.amount_usd, amount_usd);
        assert_eq!(new_invoice.amount_credits, BigDecimal::from_str("2500.00000000").unwrap());
        assert_eq!(new_invoice.payment_provider, PaymentProvider::Stripe);
        assert_eq!(new_invoice.external_ref, Some("cs_test_123".to_string()));
    }

    #[test]
    fn test_credits_rate_constant() {
        let rate = BigDecimal::from_str(CREDITS_PER_USD).unwrap();
        assert_eq!(rate, BigDecimal::from_str("100.00000000").unwrap());
    }

    #[test]
    fn test_min_max_constants() {
        let min = BigDecimal::from_str(MIN_PURCHASE_USD).unwrap();
        let max = BigDecimal::from_str(MAX_PURCHASE_USD).unwrap();
        assert_eq!(min, BigDecimal::from(1));
        assert_eq!(max, BigDecimal::from(10000));
    }
}
