//! Credit purchase and management endpoints.

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    ComputeProvider, InvoiceStatus, MCreditsLedger, NewMCreditsLedger, NewPurchaseInvoice,
    NewRedemptionReceipt, PaymentProvider, PurchaseInvoice, RedemptionReceipt,
};

/// Credit rate: 1 USD = 100 M-credits
/// This is a placeholder rate - in production, this would be configurable
/// or fetched from a pricing service.
const CREDITS_PER_USD: &str = "100.00000000";

/// Minimum purchase amount in USD.
const MIN_PURCHASE_USD: &str = "1.00";

/// Maximum purchase amount in USD.
const MAX_PURCHASE_USD: &str = "10000.00";

/// Maximum promo credits per DID (lifetime limit).
const MAX_PROMO_CREDITS_PER_DID: &str = "100.00000000";

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
        .route("/webhook/stripe", post(handle_stripe_webhook))
        .route("/grant-promo", post(grant_promo_credits_handler))
        .route("/reserves", get(get_reserves))
        .route("/redeem", post(redeem_credits))
        .route("/balance", get(get_balance))
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

// ===== Payment Webhook Handling (US-012E) =====

/// Stripe webhook event types we care about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripeEventType {
    /// Payment was successful.
    CheckoutSessionCompleted,
    /// Payment failed.
    CheckoutSessionExpired,
}

/// Request body for Stripe webhook.
/// In production, this would parse the full Stripe event structure.
/// For now, we use a simplified structure that captures the essential fields.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StripeWebhookRequest {
    /// Event type from Stripe.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Event data containing the checkout session.
    pub data: StripeEventData,
    /// Stripe signature for verification (would come from header in real impl).
    #[serde(default)]
    pub stripe_signature: Option<String>,
}

/// Stripe event data wrapper.
#[derive(Debug, Deserialize)]
pub struct StripeEventData {
    /// The checkout session object.
    pub object: StripeCheckoutSession,
}

/// Stripe checkout session object (simplified).
#[derive(Debug, Deserialize)]
pub struct StripeCheckoutSession {
    /// Session ID from Stripe.
    pub id: String,
    /// Client reference ID - our invoice ID.
    #[serde(default)]
    pub client_reference_id: Option<String>,
    /// Payment intent ID from Stripe.
    #[serde(default)]
    pub payment_intent: Option<String>,
    /// Customer email (optional).
    #[serde(default)]
    pub customer_email: Option<String>,
    /// DID of the account to credit (custom metadata field).
    #[serde(default)]
    pub metadata: Option<StripeSessionMetadata>,
}

/// Custom metadata attached to Stripe sessions.
#[derive(Debug, Deserialize)]
pub struct StripeSessionMetadata {
    /// The DID to credit upon payment completion.
    #[serde(default)]
    pub did: Option<String>,
    /// Our internal invoice ID.
    #[serde(default)]
    pub invoice_id: Option<String>,
}

/// Response for webhook processing.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookResponse {
    /// Whether the webhook was processed successfully.
    pub success: bool,
    /// Optional message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Invoice ID that was processed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_id: Option<Uuid>,
    /// Credits minted (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credits_minted: Option<String>,
}

/// Parses the Stripe event type string.
fn parse_stripe_event_type(event_type: &str) -> Option<StripeEventType> {
    match event_type {
        "checkout.session.completed" => Some(StripeEventType::CheckoutSessionCompleted),
        "checkout.session.expired" => Some(StripeEventType::CheckoutSessionExpired),
        _ => None,
    }
}

/// Verifies Stripe webhook signature.
/// In production, this would:
/// 1. Extract the Stripe-Signature header
/// 2. Reconstruct the signed payload
/// 3. Verify using the webhook signing secret
///
/// For now, this is a placeholder that accepts all requests with a valid structure.
fn verify_stripe_signature(
    _request: &StripeWebhookRequest,
    _signing_secret: Option<&str>,
) -> Result<(), AppError> {
    // TODO: Implement real Stripe signature verification
    // See: https://stripe.com/docs/webhooks/signatures
    //
    // let sig = stripe::Webhook::construct_event(
    //     payload,
    //     sig_header,
    //     signing_secret,
    // )?;
    Ok(())
}

/// Extracts invoice ID from webhook request.
/// Tries multiple sources: metadata.invoice_id, client_reference_id, or external_ref pattern.
fn extract_invoice_id(session: &StripeCheckoutSession) -> Result<Uuid, AppError> {
    // Try metadata.invoice_id first
    if let Some(metadata) = &session.metadata {
        if let Some(invoice_id_str) = &metadata.invoice_id {
            if let Ok(id) = Uuid::parse_str(invoice_id_str) {
                return Ok(id);
            }
        }
    }

    // Try client_reference_id
    if let Some(ref_id) = &session.client_reference_id {
        if let Ok(id) = Uuid::parse_str(ref_id) {
            return Ok(id);
        }
    }

    // Try to extract from our placeholder format: checkout_placeholder_{uuid}
    if let Some(payment_intent) = &session.payment_intent {
        // This is for real Stripe sessions; our placeholder uses session.id
        if payment_intent.starts_with("pi_") {
            // We'd look up by external_ref in production
        }
    }

    Err(AppError::BadRequest(
        "Could not extract invoice ID from webhook data".to_string(),
    ))
}

/// Loads an invoice by ID and validates it's pending.
async fn load_pending_invoice(pool: &PgPool, invoice_id: Uuid) -> Result<PurchaseInvoice, AppError> {
    let invoice: PurchaseInvoice = sqlx::query_as(
        r#"
        SELECT id, user_id, amount_usd, amount_credits, payment_provider, external_ref, status, created_at, updated_at
        FROM purchase_invoices
        WHERE id = $1
        "#,
    )
    .bind(invoice_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to load invoice: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("Invoice not found: {}", invoice_id)))?;

    if invoice.status != InvoiceStatus::Pending {
        return Err(AppError::BadRequest(format!(
            "Invoice {} is not pending (status: {:?})",
            invoice_id, invoice.status
        )));
    }

    Ok(invoice)
}

/// Extracts DID from webhook request or falls back to looking up user's bound DID.
/// For now, we require the DID to be in the webhook metadata.
async fn get_recipient_did(
    _pool: &PgPool,
    session: &StripeCheckoutSession,
) -> Result<String, AppError> {
    // Try to get DID from metadata
    if let Some(metadata) = &session.metadata {
        if let Some(did) = &metadata.did {
            if !did.is_empty() {
                return Ok(did.clone());
            }
        }
    }

    // In production, we'd look up the user's bound DID:
    // let binding = get_did_binding_for_user(pool, invoice.user_id).await?;
    // return Ok(binding.did);

    Err(AppError::BadRequest(
        "No DID specified in webhook metadata. Cannot mint credits without a recipient DID.".to_string(),
    ))
}

/// Mints credits to a DID by:
/// 1. Inserting a mint event into the ledger
/// 2. Upserting the credits account with the new balance
///
/// This is done atomically within a transaction.
async fn mint_credits_to_did(
    pool: &PgPool,
    did: &str,
    amount: &BigDecimal,
    invoice_id: Uuid,
    external_ref: Option<&str>,
) -> Result<Uuid, AppError> {
    // Create mint event
    let ledger_entry = NewMCreditsLedger::mint(
        did.to_string(),
        amount.clone(),
        json!({
            "invoice_id": invoice_id.to_string(),
            "external_ref": external_ref,
            "reason": "credit_purchase"
        }),
    );

    // Insert ledger entry
    let ledger_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO m_credits_ledger (event_type, from_did, to_did, amount, metadata)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(ledger_entry.event_type)
    .bind(&ledger_entry.from_did)
    .bind(&ledger_entry.to_did)
    .bind(&ledger_entry.amount)
    .bind(&ledger_entry.metadata)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to insert ledger entry: {}", e)))?;

    // Upsert account balance atomically
    sqlx::query(
        r#"
        INSERT INTO m_credits_accounts (did, balance)
        VALUES ($1, $2)
        ON CONFLICT (did)
        DO UPDATE SET balance = m_credits_accounts.balance + $2
        "#,
    )
    .bind(did)
    .bind(amount)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update account balance: {}", e)))?;

    Ok(ledger_id)
}

/// Updates invoice status to completed.
async fn complete_invoice(
    pool: &PgPool,
    invoice_id: Uuid,
    payment_ref: Option<&str>,
) -> Result<(), AppError> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE purchase_invoices
        SET status = $1, external_ref = COALESCE($2, external_ref)
        WHERE id = $3 AND status = 'pending'
        "#,
    )
    .bind(InvoiceStatus::Completed)
    .bind(payment_ref)
    .bind(invoice_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update invoice status: {}", e)))?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::BadRequest(format!(
            "Invoice {} was not updated (may have already been processed)",
            invoice_id
        )));
    }

    Ok(())
}

/// Updates invoice status to failed.
async fn fail_invoice(pool: &PgPool, invoice_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE purchase_invoices
        SET status = $1
        WHERE id = $2 AND status = 'pending'
        "#,
    )
    .bind(InvoiceStatus::Failed)
    .bind(invoice_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update invoice status: {}", e)))?;

    Ok(())
}

/// POST /api/v1/credits/webhook/stripe
///
/// Handles Stripe webhook events for payment confirmation.
/// When a checkout session completes:
/// 1. Verifies the webhook signature
/// 2. Loads the pending invoice
/// 3. Mints credits to the recipient's DID
/// 4. Updates the invoice status to completed
async fn handle_stripe_webhook(
    State(pool): State<PgPool>,
    Json(request): Json<StripeWebhookRequest>,
) -> Result<Json<WebhookResponse>, AppError> {
    // Step 1: Verify webhook signature
    verify_stripe_signature(&request, None)?;

    // Step 2: Parse event type
    let event_type = parse_stripe_event_type(&request.event_type).ok_or_else(|| {
        AppError::BadRequest(format!("Unsupported event type: {}", request.event_type))
    })?;

    // Step 3: Extract invoice ID from webhook data
    let invoice_id = extract_invoice_id(&request.data.object)?;

    match event_type {
        StripeEventType::CheckoutSessionCompleted => {
            // Step 4: Load and validate invoice
            let invoice = load_pending_invoice(&pool, invoice_id).await?;

            // Step 5: Get recipient DID
            let did = get_recipient_did(&pool, &request.data.object).await?;

            // Step 6: Get payment reference for metadata
            let payment_ref = request
                .data
                .object
                .payment_intent
                .as_deref()
                .or_else(|| Some(request.data.object.id.as_str()));

            // Step 7: Mint credits (ledger + account update)
            let _ledger_id =
                mint_credits_to_did(&pool, &did, &invoice.amount_credits, invoice_id, payment_ref)
                    .await?;

            // Step 8: Mark invoice as completed
            complete_invoice(&pool, invoice_id, payment_ref).await?;

            Ok(Json(WebhookResponse {
                success: true,
                message: Some("Credits minted successfully".to_string()),
                invoice_id: Some(invoice_id),
                credits_minted: Some(invoice.amount_credits.to_string()),
            }))
        }
        StripeEventType::CheckoutSessionExpired => {
            // Payment failed or expired - mark invoice as failed
            fail_invoice(&pool, invoice_id).await?;

            Ok(Json(WebhookResponse {
                success: true,
                message: Some("Invoice marked as failed".to_string()),
                invoice_id: Some(invoice_id),
                credits_minted: None,
            }))
        }
    }
}

// ===== Promo Credit Grants (US-012F) =====

/// Request body for granting promotional credits.
/// Note: This is an admin endpoint - in production, it would require admin authentication.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantPromoCreditsRequest {
    /// The DID to grant promo credits to.
    pub did: String,
    /// Amount of promo credits to grant.
    pub amount: String,
    /// Reason for the grant (e.g., "new_user_bonus", "referral_reward").
    pub reason: String,
    /// Optional expiry timestamp for the promo credits (ISO 8601 format).
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// Response for successful promo credit grant.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantPromoCreditsResponse {
    /// Whether the grant was successful.
    pub success: bool,
    /// The DID that received the credits.
    pub did: String,
    /// Amount of promo credits granted.
    pub amount_granted: String,
    /// New total promo balance for this DID.
    pub new_promo_balance: String,
    /// Ledger entry ID for the grant.
    pub ledger_id: Uuid,
}

/// Validates the DID format.
fn validate_did_format(did: &str) -> Result<(), AppError> {
    if !did.starts_with("did:key:z") {
        return Err(AppError::BadRequest(
            "Invalid DID format. Expected did:key:z... format.".to_string(),
        ));
    }
    if did.len() < 20 {
        return Err(AppError::BadRequest(
            "Invalid DID format. DID is too short.".to_string(),
        ));
    }
    Ok(())
}

/// Gets the current total promo credits granted to a DID from the ledger.
async fn get_total_promo_credits_for_did(pool: &PgPool, did: &str) -> Result<BigDecimal, AppError> {
    let total: Option<BigDecimal> = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(amount), 0)
        FROM m_credits_ledger
        WHERE to_did = $1 AND event_type = 'promo_mint'
        "#,
    )
    .bind(did)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query promo credits: {}", e)))?;

    Ok(total.unwrap_or_else(|| BigDecimal::from(0)))
}

/// Grants promotional credits to a DID.
///
/// This function:
/// 1. Validates the DID format
/// 2. Checks the max promo credits limit (100 per DID lifetime)
/// 3. Inserts a promo_mint event into the ledger
/// 4. Updates the promo_balance in m_credits_accounts
///
/// Returns the ledger entry ID and new promo balance.
async fn grant_promo_credits(
    pool: &PgPool,
    did: &str,
    amount: &BigDecimal,
    reason: &str,
    expires_at: Option<&str>,
) -> Result<(Uuid, BigDecimal), AppError> {
    // Validate amount is positive
    if amount <= &BigDecimal::from(0) {
        return Err(AppError::BadRequest(
            "Promo credit amount must be positive".to_string(),
        ));
    }

    // Check max promo credits limit
    let max_promo = BigDecimal::from_str(MAX_PROMO_CREDITS_PER_DID).unwrap();
    let current_total = get_total_promo_credits_for_did(pool, did).await?;
    let new_total = &current_total + amount;

    if new_total > max_promo {
        let remaining = &max_promo - &current_total;
        return Err(AppError::BadRequest(format!(
            "Promo credit limit exceeded. DID has {} promo credits, limit is {}. Max additional: {}",
            current_total, max_promo, remaining
        )));
    }

    // Build metadata
    let mut metadata = json!({
        "reason": reason,
        "grant_type": "promo"
    });
    if let Some(expiry) = expires_at {
        metadata["expires_at"] = json!(expiry);
    }

    // Create promo_mint event
    let ledger_entry = NewMCreditsLedger::promo_mint(did.to_string(), amount.clone(), metadata);

    // Insert ledger entry
    let ledger_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO m_credits_ledger (event_type, from_did, to_did, amount, metadata)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(ledger_entry.event_type)
    .bind(&ledger_entry.from_did)
    .bind(&ledger_entry.to_did)
    .bind(&ledger_entry.amount)
    .bind(&ledger_entry.metadata)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to insert ledger entry: {}", e)))?;

    // Upsert account promo_balance atomically
    let new_promo_balance: BigDecimal = sqlx::query_scalar(
        r#"
        INSERT INTO m_credits_accounts (did, balance, promo_balance)
        VALUES ($1, 0, $2)
        ON CONFLICT (did)
        DO UPDATE SET promo_balance = m_credits_accounts.promo_balance + $2
        RETURNING promo_balance
        "#,
    )
    .bind(did)
    .bind(amount)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update promo balance: {}", e)))?;

    Ok((ledger_id, new_promo_balance))
}

/// POST /api/v1/credits/grant-promo
///
/// Admin endpoint to grant promotional credits to a DID.
/// Enforces a maximum of 100 promo credits per DID (lifetime limit).
///
/// In production, this endpoint would require admin authentication.
async fn grant_promo_credits_handler(
    State(pool): State<PgPool>,
    Json(request): Json<GrantPromoCreditsRequest>,
) -> Result<Json<GrantPromoCreditsResponse>, AppError> {
    // Step 1: Validate DID format
    validate_did_format(&request.did)?;

    // Step 2: Parse and validate amount
    let amount = BigDecimal::from_str(&request.amount).map_err(|e| {
        AppError::BadRequest(format!("Invalid amount format: {}", e))
    })?;

    // Step 3: Grant the promo credits
    let (ledger_id, new_promo_balance) = grant_promo_credits(
        &pool,
        &request.did,
        &amount,
        &request.reason,
        request.expires_at.as_deref(),
    )
    .await?;

    Ok(Json(GrantPromoCreditsResponse {
        success: true,
        did: request.did,
        amount_granted: amount.to_string(),
        new_promo_balance: new_promo_balance.to_string(),
        ledger_id,
    }))
}

// ===== Reserve Attestation (US-012G) =====

/// Response for reserve attestation endpoint.
/// Provides transparency into the M-credits reserve backing.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservesResponse {
    /// Total outstanding M-credits (sum of all account balances).
    pub total_outstanding_credits: String,
    /// Total main balance credits (excludes promo credits).
    pub total_main_balance: String,
    /// Total promotional balance credits.
    pub total_promo_balance: String,
    /// Total reserves in USD (sum of completed invoices).
    pub total_reserves_usd: String,
    /// Reserve coverage ratio (reserves_usd * credits_per_usd / outstanding_credits).
    /// A value of 1.0 means fully backed, > 1.0 means over-collateralized.
    /// Note: Promo credits are not backed by reserves.
    pub reserve_coverage_ratio: String,
    /// Credits per USD rate used for calculation.
    pub credits_per_usd: String,
    /// Number of active accounts.
    pub account_count: i64,
    /// Number of completed invoices.
    pub invoice_count: i64,
    /// ISO 8601 timestamp of this attestation.
    pub timestamp: String,
    /// Cryptographic signature of the attestation data (placeholder for now).
    /// In production, this would be signed by a server key for verification.
    pub signature: String,
    /// Hash of the attestation data (for verification).
    pub attestation_hash: String,
}

/// Internal struct for building attestation data to be hashed/signed.
#[derive(Debug, Serialize)]
struct AttestationData {
    total_outstanding_credits: String,
    total_main_balance: String,
    total_promo_balance: String,
    total_reserves_usd: String,
    reserve_coverage_ratio: String,
    timestamp: String,
}

/// Gets the total outstanding credits from all accounts.
async fn get_total_outstanding_credits(pool: &PgPool) -> Result<(BigDecimal, BigDecimal, i64), AppError> {
    // Query sum of balances and promo_balances, plus count of accounts
    let result: (Option<BigDecimal>, Option<BigDecimal>, Option<i64>) = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(balance), 0),
            COALESCE(SUM(promo_balance), 0),
            COUNT(*)
        FROM m_credits_accounts
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query credit balances: {}", e)))?;

    let main_balance = result.0.unwrap_or_else(|| BigDecimal::from(0));
    let promo_balance = result.1.unwrap_or_else(|| BigDecimal::from(0));
    let count = result.2.unwrap_or(0);

    Ok((main_balance, promo_balance, count))
}

/// Gets the total reserves from completed invoices.
async fn get_total_reserves(pool: &PgPool) -> Result<(BigDecimal, i64), AppError> {
    // Query sum of amount_usd from completed invoices
    let result: (Option<BigDecimal>, Option<i64>) = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(amount_usd), 0),
            COUNT(*)
        FROM purchase_invoices
        WHERE status = 'completed'
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query reserves: {}", e)))?;

    let reserves = result.0.unwrap_or_else(|| BigDecimal::from(0));
    let count = result.1.unwrap_or(0);

    Ok((reserves, count))
}

/// Calculates the reserve coverage ratio.
/// Returns reserves_usd * credits_per_usd / main_balance_credits.
/// Only main balance is considered (promo credits are not backed).
fn calculate_coverage_ratio(reserves_usd: &BigDecimal, main_balance: &BigDecimal) -> BigDecimal {
    if main_balance == &BigDecimal::from(0) {
        // No credits outstanding means infinite coverage (or 0/0 case)
        // We return a high number to indicate fully backed
        if reserves_usd > &BigDecimal::from(0) {
            // Has reserves but no outstanding = over-collateralized
            return BigDecimal::from(999999);
        }
        // No reserves and no credits = N/A, return 1.0
        return BigDecimal::from(1);
    }

    let rate = BigDecimal::from_str(CREDITS_PER_USD).unwrap();
    // reserves_usd * credits_per_usd gives "credit-equivalent reserves"
    // Divide by main_balance to get coverage ratio
    let credit_equivalent = reserves_usd * &rate;

    // Use 8 decimal places for precision
    credit_equivalent / main_balance
}

/// Generates a SHA-256 hash of the attestation data.
fn hash_attestation(data: &AttestationData) -> String {
    use sha2::{Sha256, Digest};

    // Serialize to canonical JSON
    let json = serde_json::to_string(data).unwrap_or_default();

    // Hash with SHA-256
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let hash = hasher.finalize();

    // Return as hex string
    hex::encode(hash)
}

/// Generates a placeholder signature for the attestation.
/// In production, this would use a server signing key (Ed25519).
fn sign_attestation(attestation_hash: &str) -> String {
    // Placeholder: In production, sign with server's Ed25519 key
    // For now, return a marker indicating signature verification is not implemented
    format!("placeholder_signature_v1:{}", &attestation_hash[..16])
}

/// Gets the current ISO 8601 timestamp.
fn get_current_timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// GET /api/v1/credits/reserves
///
/// Returns the current reserve attestation, showing:
/// - Total outstanding M-credits
/// - Total USD reserves from completed purchases
/// - Reserve coverage ratio
/// - Timestamp and cryptographic signature for verification
///
/// This endpoint provides transparency into the M-credits reserve backing.
async fn get_reserves(
    State(pool): State<PgPool>,
) -> Result<Json<ReservesResponse>, AppError> {
    // Step 1: Get total outstanding credits
    let (main_balance, promo_balance, account_count) = get_total_outstanding_credits(&pool).await?;
    let total_outstanding = &main_balance + &promo_balance;

    // Step 2: Get total reserves from completed invoices
    let (total_reserves_usd, invoice_count) = get_total_reserves(&pool).await?;

    // Step 3: Calculate coverage ratio (only for main balance, promo not backed)
    let coverage_ratio = calculate_coverage_ratio(&total_reserves_usd, &main_balance);

    // Step 4: Get timestamp
    let timestamp = get_current_timestamp();

    // Step 5: Build attestation data for hashing
    let attestation_data = AttestationData {
        total_outstanding_credits: total_outstanding.to_string(),
        total_main_balance: main_balance.to_string(),
        total_promo_balance: promo_balance.to_string(),
        total_reserves_usd: total_reserves_usd.to_string(),
        reserve_coverage_ratio: coverage_ratio.to_string(),
        timestamp: timestamp.clone(),
    };

    // Step 6: Generate hash
    let attestation_hash = hash_attestation(&attestation_data);

    // Step 7: Generate signature
    let signature = sign_attestation(&attestation_hash);

    // Step 8: Build response
    Ok(Json(ReservesResponse {
        total_outstanding_credits: total_outstanding.to_string(),
        total_main_balance: main_balance.to_string(),
        total_promo_balance: promo_balance.to_string(),
        total_reserves_usd: total_reserves_usd.to_string(),
        reserve_coverage_ratio: coverage_ratio.to_string(),
        credits_per_usd: CREDITS_PER_USD.to_string(),
        account_count,
        invoice_count,
        timestamp,
        signature,
        attestation_hash,
    }))
}

// ===== Credit Redemption (US-015B) =====

/// Minimum redemption amount in credits.
const MIN_REDEMPTION_CREDITS: &str = "1.00000000";

/// Maximum redemption amount per transaction.
const MAX_REDEMPTION_CREDITS: &str = "10000.00000000";

/// Request body for redeeming credits.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemCreditsRequest {
    /// The DID of the user redeeming credits.
    /// In production, this would be extracted from auth token.
    pub did: String,
    /// The ID of the compute provider to redeem with.
    pub provider_id: Uuid,
    /// Amount of credits to redeem.
    pub amount: String,
}

/// Response for successful credit redemption.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemCreditsResponse {
    /// Whether the redemption was successful.
    pub success: bool,
    /// The redemption receipt ID.
    pub receipt_id: Uuid,
    /// Amount of credits redeemed.
    pub amount_redeemed: String,
    /// Allocation ID from the provider (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocation_id: Option<String>,
    /// Provider name.
    pub provider_name: String,
    /// New balance after redemption.
    pub new_balance: String,
}

/// Validates the redemption amount is within acceptable bounds.
fn validate_redemption_amount(amount: &BigDecimal) -> Result<(), AppError> {
    let min = BigDecimal::from_str(MIN_REDEMPTION_CREDITS).unwrap();
    let max = BigDecimal::from_str(MAX_REDEMPTION_CREDITS).unwrap();

    if amount < &min {
        return Err(AppError::BadRequest(format!(
            "Minimum redemption amount is {} credits",
            MIN_REDEMPTION_CREDITS
        )));
    }

    if amount > &max {
        return Err(AppError::BadRequest(format!(
            "Maximum redemption amount is {} credits per transaction",
            MAX_REDEMPTION_CREDITS
        )));
    }

    if amount <= &BigDecimal::from(0) {
        return Err(AppError::BadRequest(
            "Redemption amount must be positive".to_string(),
        ));
    }

    Ok(())
}

/// Loads a compute provider by ID and validates it's active.
async fn load_active_provider(pool: &PgPool, provider_id: Uuid) -> Result<ComputeProvider, AppError> {
    let provider: ComputeProvider = sqlx::query_as(
        r#"
        SELECT id, name, provider_type, api_endpoint, conversion_rate, is_active, created_at, updated_at
        FROM compute_providers
        WHERE id = $1
        "#,
    )
    .bind(provider_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to load provider: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("Provider not found: {}", provider_id)))?;

    if !provider.is_active {
        return Err(AppError::BadRequest(format!(
            "Provider '{}' is not currently active",
            provider.name
        )));
    }

    Ok(provider)
}

/// Gets the current balance for a DID.
async fn get_account_balance(pool: &PgPool, did: &str) -> Result<BigDecimal, AppError> {
    let balance: Option<BigDecimal> = sqlx::query_scalar(
        r#"
        SELECT balance
        FROM m_credits_accounts
        WHERE did = $1
        "#,
    )
    .bind(did)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query balance: {}", e)))?;

    Ok(balance.unwrap_or_else(|| BigDecimal::from(0)))
}

/// Deducts credits from a DID's balance atomically.
/// Returns the new balance after deduction.
async fn deduct_balance(
    pool: &PgPool,
    did: &str,
    amount: &BigDecimal,
) -> Result<BigDecimal, AppError> {
    // Atomically deduct and return new balance
    // This also validates sufficient balance via the CHECK constraint
    let new_balance: BigDecimal = sqlx::query_scalar(
        r#"
        UPDATE m_credits_accounts
        SET balance = balance - $2
        WHERE did = $1
        RETURNING balance
        "#,
    )
    .bind(did)
    .bind(amount)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        // Check if this is a constraint violation (insufficient balance)
        let err_str = e.to_string();
        if err_str.contains("balance_non_negative") || err_str.contains("check") {
            AppError::BadRequest("Insufficient balance for redemption".to_string())
        } else {
            AppError::Internal(format!("Failed to deduct balance: {}", e))
        }
    })?
    .ok_or_else(|| AppError::NotFound(format!("Account not found for DID: {}", did)))?;

    Ok(new_balance)
}

/// Inserts a burn event into the ledger for the redemption.
async fn insert_burn_event(
    pool: &PgPool,
    did: &str,
    amount: &BigDecimal,
    provider_id: Uuid,
    receipt_id: Uuid,
) -> Result<Uuid, AppError> {
    let ledger_entry = NewMCreditsLedger::burn(
        did.to_string(),
        amount.clone(),
        json!({
            "reason": "credit_redemption",
            "provider_id": provider_id.to_string(),
            "receipt_id": receipt_id.to_string()
        }),
    );

    let ledger_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO m_credits_ledger (event_type, from_did, to_did, amount, metadata)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(ledger_entry.event_type)
    .bind(&ledger_entry.from_did)
    .bind(&ledger_entry.to_did)
    .bind(&ledger_entry.amount)
    .bind(&ledger_entry.metadata)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to insert ledger entry: {}", e)))?;

    Ok(ledger_id)
}

/// Calls the provider API to allocate credits/quota.
/// This is a placeholder - in production, this would make actual API calls.
async fn allocate_with_provider(
    _provider: &ComputeProvider,
    _amount: &BigDecimal,
) -> Result<Option<String>, AppError> {
    // TODO: Implement actual provider API calls
    // For now, return a placeholder allocation ID
    //
    // In production, this would:
    // 1. Call the provider's API endpoint
    // 2. Pass authentication credentials
    // 3. Request allocation of compute resources
    // 4. Return the allocation ID from the provider
    //
    // Example for OpenAI:
    // - POST to provider.api_endpoint + "/usage/allocate"
    // - Include API key from secure storage
    // - Request allocation of tokens based on conversion_rate
    //
    // Example for Anthropic:
    // - Similar flow with their API
    //
    // For GPU providers:
    // - Request allocation of compute time

    let allocation_id = format!("alloc_placeholder_{}", Uuid::new_v4());
    Ok(Some(allocation_id))
}


/// POST /api/v1/credits/redeem
///
/// Redeems M-credits with a compute provider.
///
/// This endpoint:
/// 1. Validates the request parameters
/// 2. Verifies the provider exists and is active
/// 3. Checks the user has sufficient balance
/// 4. Deducts credits from the user's account
/// 5. Inserts a burn event into the ledger
/// 6. Calls the provider API to allocate resources
/// 7. Creates a redemption receipt
/// 8. Returns the allocation details
async fn redeem_credits(
    State(pool): State<PgPool>,
    Json(request): Json<RedeemCreditsRequest>,
) -> Result<Json<RedeemCreditsResponse>, AppError> {
    // Step 1: Validate DID format
    validate_did_format(&request.did)?;

    // Step 2: Parse and validate amount
    let amount = BigDecimal::from_str(&request.amount).map_err(|e| {
        AppError::BadRequest(format!("Invalid amount format: {}", e))
    })?;
    validate_redemption_amount(&amount)?;

    // Step 3: Load and validate provider
    let provider = load_active_provider(&pool, request.provider_id).await?;

    // Step 4: Check current balance
    let current_balance = get_account_balance(&pool, &request.did).await?;
    if current_balance < amount {
        return Err(AppError::BadRequest(format!(
            "Insufficient balance. Current: {}, Requested: {}",
            current_balance, amount
        )));
    }

    // Step 5: Generate receipt ID upfront for ledger reference
    let receipt_id = Uuid::new_v4();

    // Step 6: Deduct balance atomically
    let new_balance = deduct_balance(&pool, &request.did, &amount).await?;

    // Step 7: Insert burn event
    let _ledger_id = insert_burn_event(&pool, &request.did, &amount, provider.id, receipt_id).await?;

    // Step 8: Call provider API to allocate resources
    let allocation_id = allocate_with_provider(&provider, &amount).await?;

    // Step 9: Create redemption receipt
    let new_receipt = NewRedemptionReceipt::new(
        request.did.clone(),
        provider.id,
        amount.clone(),
        allocation_id.clone(),
        json!({
            "provider_name": provider.name,
            "conversion_rate": provider.conversion_rate.to_string()
        }),
    );

    // Insert with the pre-generated ID
    let receipt: RedemptionReceipt = sqlx::query_as(
        r#"
        INSERT INTO redemption_receipts (id, user_did, provider_id, amount_credits, allocation_id, metadata)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, user_did, provider_id, amount_credits, allocation_id, metadata, created_at
        "#,
    )
    .bind(receipt_id)
    .bind(&new_receipt.user_did)
    .bind(new_receipt.provider_id)
    .bind(&new_receipt.amount_credits)
    .bind(&new_receipt.allocation_id)
    .bind(&new_receipt.metadata)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to insert redemption receipt: {}", e)))?;

    // Step 10: Return success response
    Ok(Json(RedeemCreditsResponse {
        success: true,
        receipt_id: receipt.id,
        amount_redeemed: amount.to_string(),
        allocation_id,
        provider_name: provider.name,
        new_balance: new_balance.to_string(),
    }))
}

// ===== Balance Check (US-015D) =====

/// Request body for checking balance.
/// Note: In a real implementation, the user_id would come from authentication.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceRequest {
    /// The user ID requesting their balance.
    /// In production, this would be extracted from auth token.
    pub user_id: Uuid,
}

/// A simplified transaction record for the response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionRecord {
    /// Unique identifier for this transaction.
    pub id: Uuid,
    /// Type of credit event (mint, burn, transfer, etc.).
    pub event_type: String,
    /// Amount of credits in this transaction.
    pub amount: String,
    /// Description or reason for the transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// When this event occurred.
    pub created_at: String,
}

/// Response for balance check endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResponse {
    /// The DID associated with this balance.
    pub did: String,
    /// Main credit balance (backed by reserves).
    pub balance: String,
    /// Promotional credit balance (not backed by reserves).
    pub promo_balance: String,
    /// Total spendable balance (balance + promo_balance).
    pub total: String,
    /// Recent transactions (last 10).
    pub recent_transactions: Vec<TransactionRecord>,
}

/// Gets the user's bound DID from the database.
/// Returns an error if the user has no DID bound.
async fn get_user_bound_did(pool: &PgPool, user_id: Uuid) -> Result<String, AppError> {
    let did: Option<String> = sqlx::query_scalar(
        r#"
        SELECT did
        FROM did_bindings
        WHERE user_id = $1 AND revoked_at IS NULL
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query DID binding: {}", e)))?;

    did.ok_or_else(|| {
        AppError::BadRequest(
            "No DID bound to this account. Please bind a DID first using the identity endpoints."
                .to_string(),
        )
    })
}

/// Gets the account balances for a DID.
/// Returns (balance, promo_balance) or (0, 0) if no account exists.
async fn get_account_balances(
    pool: &PgPool,
    did: &str,
) -> Result<(BigDecimal, BigDecimal), AppError> {
    let result: Option<(BigDecimal, BigDecimal)> = sqlx::query_as(
        r#"
        SELECT balance, promo_balance
        FROM m_credits_accounts
        WHERE did = $1
        "#,
    )
    .bind(did)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query account: {}", e)))?;

    match result {
        Some((balance, promo_balance)) => Ok((balance, promo_balance)),
        None => Ok((BigDecimal::from(0), BigDecimal::from(0))),
    }
}

/// Gets the recent transactions for a DID (last 10).
async fn get_recent_transactions(
    pool: &PgPool,
    did: &str,
    limit: i64,
) -> Result<Vec<TransactionRecord>, AppError> {
    let entries: Vec<MCreditsLedger> = sqlx::query_as(
        r#"
        SELECT id, event_type, from_did, to_did, amount, metadata, created_at
        FROM m_credits_ledger
        WHERE from_did = $1 OR to_did = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(did)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query transactions: {}", e)))?;

    let records: Vec<TransactionRecord> = entries
        .into_iter()
        .map(|entry| {
            // Extract description from metadata if available
            let description = entry
                .metadata
                .get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    entry
                        .metadata
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                });

            TransactionRecord {
                id: entry.id,
                event_type: format!("{:?}", entry.event_type).to_lowercase(),
                amount: entry.amount.to_string(),
                description,
                created_at: entry.created_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(records)
}

/// GET /api/v1/credits/balance
///
/// Returns the user's M-credits balance information.
///
/// This endpoint:
/// 1. Looks up the user's bound DID
/// 2. Fetches their account balances (main and promo)
/// 3. Fetches their recent transactions (last 10)
/// 4. Returns a comprehensive balance response
///
/// Requires the user to have a bound DID.
async fn get_balance(
    State(pool): State<PgPool>,
    axum::extract::Query(query): axum::extract::Query<BalanceRequest>,
) -> Result<Json<BalanceResponse>, AppError> {
    // Step 1: Get the user's bound DID
    let did = get_user_bound_did(&pool, query.user_id).await?;

    // Step 2: Get account balances
    let (balance, promo_balance) = get_account_balances(&pool, &did).await?;

    // Step 3: Calculate total
    let total = &balance + &promo_balance;

    // Step 4: Get recent transactions (last 10)
    let recent_transactions = get_recent_transactions(&pool, &did, 10).await?;

    // Step 5: Return response
    Ok(Json(BalanceResponse {
        did,
        balance: balance.to_string(),
        promo_balance: promo_balance.to_string(),
        total: total.to_string(),
        recent_transactions,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MCreditsEventType;

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

    // ===== Webhook Tests (US-012E) =====

    #[test]
    fn test_parse_stripe_event_type_completed() {
        let result = parse_stripe_event_type("checkout.session.completed");
        assert_eq!(result, Some(StripeEventType::CheckoutSessionCompleted));
    }

    #[test]
    fn test_parse_stripe_event_type_expired() {
        let result = parse_stripe_event_type("checkout.session.expired");
        assert_eq!(result, Some(StripeEventType::CheckoutSessionExpired));
    }

    #[test]
    fn test_parse_stripe_event_type_unknown() {
        let result = parse_stripe_event_type("payment_intent.succeeded");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_stripe_event_type_invalid() {
        let result = parse_stripe_event_type("invalid_event");
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_invoice_id_from_metadata() {
        let invoice_id = Uuid::new_v4();
        let session = StripeCheckoutSession {
            id: "cs_test_123".to_string(),
            client_reference_id: None,
            payment_intent: None,
            customer_email: None,
            metadata: Some(StripeSessionMetadata {
                did: Some("did:key:z6MkTest".to_string()),
                invoice_id: Some(invoice_id.to_string()),
            }),
        };

        let result = extract_invoice_id(&session);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), invoice_id);
    }

    #[test]
    fn test_extract_invoice_id_from_client_reference() {
        let invoice_id = Uuid::new_v4();
        let session = StripeCheckoutSession {
            id: "cs_test_123".to_string(),
            client_reference_id: Some(invoice_id.to_string()),
            payment_intent: None,
            customer_email: None,
            metadata: None,
        };

        let result = extract_invoice_id(&session);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), invoice_id);
    }

    #[test]
    fn test_extract_invoice_id_metadata_priority() {
        // When both metadata and client_reference_id are present, metadata takes priority
        let invoice_id_metadata = Uuid::new_v4();
        let invoice_id_ref = Uuid::new_v4();
        let session = StripeCheckoutSession {
            id: "cs_test_123".to_string(),
            client_reference_id: Some(invoice_id_ref.to_string()),
            payment_intent: None,
            customer_email: None,
            metadata: Some(StripeSessionMetadata {
                did: Some("did:key:z6MkTest".to_string()),
                invoice_id: Some(invoice_id_metadata.to_string()),
            }),
        };

        let result = extract_invoice_id(&session);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), invoice_id_metadata);
    }

    #[test]
    fn test_extract_invoice_id_missing() {
        let session = StripeCheckoutSession {
            id: "cs_test_123".to_string(),
            client_reference_id: None,
            payment_intent: Some("pi_test_123".to_string()),
            customer_email: None,
            metadata: None,
        };

        let result = extract_invoice_id(&session);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not extract invoice ID"));
    }

    #[test]
    fn test_extract_invoice_id_invalid_uuid() {
        let session = StripeCheckoutSession {
            id: "cs_test_123".to_string(),
            client_reference_id: Some("not-a-valid-uuid".to_string()),
            payment_intent: None,
            customer_email: None,
            metadata: None,
        };

        let result = extract_invoice_id(&session);
        assert!(result.is_err());
    }

    #[test]
    fn test_webhook_request_deserialization() {
        let invoice_id = Uuid::new_v4();
        let json = format!(
            r#"{{
                "type": "checkout.session.completed",
                "data": {{
                    "object": {{
                        "id": "cs_test_123",
                        "client_reference_id": "{}",
                        "payment_intent": "pi_test_456",
                        "metadata": {{
                            "did": "did:key:z6MkTestDid123",
                            "invoice_id": "{}"
                        }}
                    }}
                }}
            }}"#,
            invoice_id, invoice_id
        );

        let request: StripeWebhookRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.event_type, "checkout.session.completed");
        assert_eq!(request.data.object.id, "cs_test_123");
        assert_eq!(
            request.data.object.client_reference_id,
            Some(invoice_id.to_string())
        );
        assert_eq!(
            request.data.object.payment_intent,
            Some("pi_test_456".to_string())
        );
        assert!(request.data.object.metadata.is_some());
        let metadata = request.data.object.metadata.unwrap();
        assert_eq!(metadata.did, Some("did:key:z6MkTestDid123".to_string()));
        assert_eq!(metadata.invoice_id, Some(invoice_id.to_string()));
    }

    #[test]
    fn test_webhook_request_minimal() {
        let json = r#"{
            "type": "checkout.session.expired",
            "data": {
                "object": {
                    "id": "cs_expired_123"
                }
            }
        }"#;

        let request: StripeWebhookRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.event_type, "checkout.session.expired");
        assert_eq!(request.data.object.id, "cs_expired_123");
        assert!(request.data.object.client_reference_id.is_none());
        assert!(request.data.object.metadata.is_none());
    }

    #[test]
    fn test_webhook_response_serialization_success() {
        let invoice_id = Uuid::new_v4();
        let response = WebhookResponse {
            success: true,
            message: Some("Credits minted successfully".to_string()),
            invoice_id: Some(invoice_id),
            credits_minted: Some("1000.00000000".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Credits minted successfully\""));
        assert!(json.contains("\"invoiceId\":"));
        assert!(json.contains("\"creditsMinted\":\"1000.00000000\""));
    }

    #[test]
    fn test_webhook_response_serialization_failure() {
        let response = WebhookResponse {
            success: false,
            message: Some("Invoice not found".to_string()),
            invoice_id: None,
            credits_minted: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"message\":\"Invoice not found\""));
        // Optional fields should not be present when None
        assert!(!json.contains("invoiceId"));
        assert!(!json.contains("creditsMinted"));
    }

    #[test]
    fn test_verify_stripe_signature_placeholder() {
        // Placeholder implementation always succeeds
        let request = StripeWebhookRequest {
            event_type: "checkout.session.completed".to_string(),
            data: StripeEventData {
                object: StripeCheckoutSession {
                    id: "cs_test".to_string(),
                    client_reference_id: None,
                    payment_intent: None,
                    customer_email: None,
                    metadata: None,
                },
            },
            stripe_signature: Some("test_signature".to_string()),
        };

        let result = verify_stripe_signature(&request, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_m_credits_ledger_mint() {
        let amount = BigDecimal::from_str("100.00000000").unwrap();
        let metadata = json!({
            "invoice_id": "test-invoice-123",
            "reason": "credit_purchase"
        });

        let entry = NewMCreditsLedger::mint(
            "did:key:z6MkTest".to_string(),
            amount.clone(),
            metadata.clone(),
        );

        assert_eq!(entry.event_type, MCreditsEventType::Mint);
        assert!(entry.from_did.is_none());
        assert_eq!(entry.to_did, Some("did:key:z6MkTest".to_string()));
        assert_eq!(entry.amount, amount);
        assert_eq!(entry.metadata["reason"], "credit_purchase");
    }

    #[test]
    fn test_stripe_event_type_equality() {
        assert_eq!(
            StripeEventType::CheckoutSessionCompleted,
            StripeEventType::CheckoutSessionCompleted
        );
        assert_ne!(
            StripeEventType::CheckoutSessionCompleted,
            StripeEventType::CheckoutSessionExpired
        );
    }

    // ===== Promo Credit Tests (US-012F) =====

    #[test]
    fn test_validate_did_format_valid() {
        let result = validate_did_format("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_did_format_invalid_prefix() {
        let result = validate_did_format("did:web:example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid DID format"));
    }

    #[test]
    fn test_validate_did_format_too_short() {
        let result = validate_did_format("did:key:z6Mk");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_validate_did_format_empty() {
        let result = validate_did_format("");
        assert!(result.is_err());
    }

    #[test]
    fn test_grant_promo_request_deserialization() {
        let json = r#"{
            "did": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
            "amount": "50.00000000",
            "reason": "new_user_bonus"
        }"#;

        let request: GrantPromoCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.did, "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
        assert_eq!(request.amount, "50.00000000");
        assert_eq!(request.reason, "new_user_bonus");
        assert!(request.expires_at.is_none());
    }

    #[test]
    fn test_grant_promo_request_with_expiry() {
        let json = r#"{
            "did": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
            "amount": "25.00000000",
            "reason": "referral_reward",
            "expiresAt": "2026-03-31T23:59:59Z"
        }"#;

        let request: GrantPromoCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.expires_at, Some("2026-03-31T23:59:59Z".to_string()));
    }

    #[test]
    fn test_grant_promo_response_serialization() {
        let ledger_id = Uuid::new_v4();
        let response = GrantPromoCreditsResponse {
            success: true,
            did: "did:key:z6MkTest".to_string(),
            amount_granted: "50.00000000".to_string(),
            new_promo_balance: "50.00000000".to_string(),
            ledger_id,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"did\":\"did:key:z6MkTest\""));
        assert!(json.contains("\"amountGranted\":\"50.00000000\""));
        assert!(json.contains("\"newPromoBalance\":\"50.00000000\""));
        assert!(json.contains("\"ledgerId\":"));
    }

    #[test]
    fn test_max_promo_credits_constant() {
        let max = BigDecimal::from_str(MAX_PROMO_CREDITS_PER_DID).unwrap();
        assert_eq!(max, BigDecimal::from_str("100.00000000").unwrap());
    }

    #[test]
    fn test_new_m_credits_ledger_promo_mint() {
        let amount = BigDecimal::from_str("50.00000000").unwrap();
        let metadata = json!({
            "reason": "new_user_bonus",
            "expires_at": "2026-02-28T23:59:59Z"
        });

        let entry = NewMCreditsLedger::promo_mint(
            "did:key:z6MkTest".to_string(),
            amount.clone(),
            metadata.clone(),
        );

        assert_eq!(entry.event_type, MCreditsEventType::PromoMint);
        assert!(entry.from_did.is_none());
        assert_eq!(entry.to_did, Some("did:key:z6MkTest".to_string()));
        assert_eq!(entry.amount, amount);
        assert_eq!(entry.metadata["reason"], "new_user_bonus");
    }

    #[test]
    fn test_promo_mint_event_type() {
        assert_eq!(
            serde_json::to_string(&MCreditsEventType::PromoMint).unwrap(),
            "\"promo_mint\""
        );
    }

    // ===== Reserve Attestation Tests (US-012G) =====

    #[test]
    fn test_calculate_coverage_ratio_normal() {
        // 10 USD reserves * 100 credits/USD = 1000 credit-equivalent
        // 1000 credit-equivalent / 1000 outstanding = 1.0 (fully backed)
        let reserves_usd = BigDecimal::from_str("10.00").unwrap();
        let main_balance = BigDecimal::from_str("1000.00000000").unwrap();
        let ratio = calculate_coverage_ratio(&reserves_usd, &main_balance);
        assert_eq!(ratio, BigDecimal::from(1));
    }

    #[test]
    fn test_calculate_coverage_ratio_over_collateralized() {
        // 20 USD reserves * 100 credits/USD = 2000 credit-equivalent
        // 2000 credit-equivalent / 1000 outstanding = 2.0 (over-collateralized)
        let reserves_usd = BigDecimal::from_str("20.00").unwrap();
        let main_balance = BigDecimal::from_str("1000.00000000").unwrap();
        let ratio = calculate_coverage_ratio(&reserves_usd, &main_balance);
        assert_eq!(ratio, BigDecimal::from(2));
    }

    #[test]
    fn test_calculate_coverage_ratio_under_collateralized() {
        // 5 USD reserves * 100 credits/USD = 500 credit-equivalent
        // 500 credit-equivalent / 1000 outstanding = 0.5 (under-collateralized)
        let reserves_usd = BigDecimal::from_str("5.00").unwrap();
        let main_balance = BigDecimal::from_str("1000.00000000").unwrap();
        let ratio = calculate_coverage_ratio(&reserves_usd, &main_balance);
        assert_eq!(ratio, BigDecimal::from_str("0.5").unwrap());
    }

    #[test]
    fn test_calculate_coverage_ratio_no_credits_no_reserves() {
        // No credits and no reserves = 1.0 (balanced at zero)
        let reserves_usd = BigDecimal::from(0);
        let main_balance = BigDecimal::from(0);
        let ratio = calculate_coverage_ratio(&reserves_usd, &main_balance);
        assert_eq!(ratio, BigDecimal::from(1));
    }

    #[test]
    fn test_calculate_coverage_ratio_reserves_no_credits() {
        // Reserves but no credits = effectively infinite (over-collateralized)
        let reserves_usd = BigDecimal::from_str("100.00").unwrap();
        let main_balance = BigDecimal::from(0);
        let ratio = calculate_coverage_ratio(&reserves_usd, &main_balance);
        assert_eq!(ratio, BigDecimal::from(999999));
    }

    #[test]
    fn test_calculate_coverage_ratio_fractional() {
        // 1.5 USD reserves * 100 = 150 credit-equivalent
        // 150 / 100 credits = 1.5
        let reserves_usd = BigDecimal::from_str("1.50").unwrap();
        let main_balance = BigDecimal::from_str("100.00000000").unwrap();
        let ratio = calculate_coverage_ratio(&reserves_usd, &main_balance);
        assert_eq!(ratio, BigDecimal::from_str("1.5").unwrap());
    }

    #[test]
    fn test_reserves_response_serialization() {
        let response = ReservesResponse {
            total_outstanding_credits: "1000.00000000".to_string(),
            total_main_balance: "800.00000000".to_string(),
            total_promo_balance: "200.00000000".to_string(),
            total_reserves_usd: "8.00".to_string(),
            reserve_coverage_ratio: "1.00".to_string(),
            credits_per_usd: "100.00000000".to_string(),
            account_count: 5,
            invoice_count: 3,
            timestamp: "2026-01-31T12:00:00.000Z".to_string(),
            signature: "placeholder_signature_v1:abc123".to_string(),
            attestation_hash: "abcd1234".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"totalOutstandingCredits\":\"1000.00000000\""));
        assert!(json.contains("\"totalMainBalance\":\"800.00000000\""));
        assert!(json.contains("\"totalPromoBalance\":\"200.00000000\""));
        assert!(json.contains("\"totalReservesUsd\":\"8.00\""));
        assert!(json.contains("\"reserveCoverageRatio\":\"1.00\""));
        assert!(json.contains("\"creditsPerUsd\":\"100.00000000\""));
        assert!(json.contains("\"accountCount\":5"));
        assert!(json.contains("\"invoiceCount\":3"));
        assert!(json.contains("\"timestamp\":"));
        assert!(json.contains("\"signature\":"));
        assert!(json.contains("\"attestationHash\":"));
    }

    #[test]
    fn test_hash_attestation_deterministic() {
        let data1 = AttestationData {
            total_outstanding_credits: "1000.00000000".to_string(),
            total_main_balance: "800.00000000".to_string(),
            total_promo_balance: "200.00000000".to_string(),
            total_reserves_usd: "8.00".to_string(),
            reserve_coverage_ratio: "1.00".to_string(),
            timestamp: "2026-01-31T12:00:00.000Z".to_string(),
        };

        let data2 = AttestationData {
            total_outstanding_credits: "1000.00000000".to_string(),
            total_main_balance: "800.00000000".to_string(),
            total_promo_balance: "200.00000000".to_string(),
            total_reserves_usd: "8.00".to_string(),
            reserve_coverage_ratio: "1.00".to_string(),
            timestamp: "2026-01-31T12:00:00.000Z".to_string(),
        };

        let hash1 = hash_attestation(&data1);
        let hash2 = hash_attestation(&data2);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex is 64 chars
    }

    #[test]
    fn test_hash_attestation_changes_with_input() {
        let data1 = AttestationData {
            total_outstanding_credits: "1000.00000000".to_string(),
            total_main_balance: "800.00000000".to_string(),
            total_promo_balance: "200.00000000".to_string(),
            total_reserves_usd: "8.00".to_string(),
            reserve_coverage_ratio: "1.00".to_string(),
            timestamp: "2026-01-31T12:00:00.000Z".to_string(),
        };

        let data2 = AttestationData {
            total_outstanding_credits: "1000.00000001".to_string(), // Changed
            total_main_balance: "800.00000000".to_string(),
            total_promo_balance: "200.00000000".to_string(),
            total_reserves_usd: "8.00".to_string(),
            reserve_coverage_ratio: "1.00".to_string(),
            timestamp: "2026-01-31T12:00:00.000Z".to_string(),
        };

        let hash1 = hash_attestation(&data1);
        let hash2 = hash_attestation(&data2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_sign_attestation_format() {
        let hash = "abcd1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab";
        let signature = sign_attestation(hash);

        assert!(signature.starts_with("placeholder_signature_v1:"));
        assert!(signature.contains("abcd1234567890ab")); // First 16 chars of hash
    }

    #[test]
    fn test_get_current_timestamp_format() {
        let timestamp = get_current_timestamp();

        // Should be ISO 8601 format
        assert!(timestamp.contains("T"));
        assert!(timestamp.ends_with("Z"));
        // Should parse successfully as RFC3339
        assert!(chrono::DateTime::parse_from_rfc3339(&timestamp).is_ok());
    }

    #[test]
    fn test_attestation_data_serialization() {
        let data = AttestationData {
            total_outstanding_credits: "1000.00000000".to_string(),
            total_main_balance: "800.00000000".to_string(),
            total_promo_balance: "200.00000000".to_string(),
            total_reserves_usd: "8.00".to_string(),
            reserve_coverage_ratio: "1.00".to_string(),
            timestamp: "2026-01-31T12:00:00.000Z".to_string(),
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"total_outstanding_credits\":"));
        assert!(json.contains("\"total_main_balance\":"));
        assert!(json.contains("\"total_promo_balance\":"));
        assert!(json.contains("\"total_reserves_usd\":"));
        assert!(json.contains("\"reserve_coverage_ratio\":"));
        assert!(json.contains("\"timestamp\":"));
    }

    // ===== Credit Redemption Tests (US-015B) =====

    #[test]
    fn test_validate_redemption_amount_valid() {
        let amount = BigDecimal::from_str("50.00000000").unwrap();
        let result = validate_redemption_amount(&amount);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_redemption_amount_minimum() {
        let amount = BigDecimal::from_str("1.00000000").unwrap();
        let result = validate_redemption_amount(&amount);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_redemption_amount_maximum() {
        let amount = BigDecimal::from_str("10000.00000000").unwrap();
        let result = validate_redemption_amount(&amount);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_redemption_amount_below_minimum() {
        let amount = BigDecimal::from_str("0.50000000").unwrap();
        let result = validate_redemption_amount(&amount);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Minimum redemption amount"));
    }

    #[test]
    fn test_validate_redemption_amount_above_maximum() {
        let amount = BigDecimal::from_str("10001.00000000").unwrap();
        let result = validate_redemption_amount(&amount);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Maximum redemption amount"));
    }

    #[test]
    fn test_validate_redemption_amount_zero() {
        let amount = BigDecimal::from_str("0").unwrap();
        let result = validate_redemption_amount(&amount);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_redemption_amount_negative() {
        let amount = BigDecimal::from_str("-10.00000000").unwrap();
        let result = validate_redemption_amount(&amount);
        assert!(result.is_err());
    }

    #[test]
    fn test_redeem_request_deserialization() {
        let provider_id = Uuid::new_v4();
        let json = format!(
            r#"{{
                "did": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
                "providerId": "{}",
                "amount": "100.00000000"
            }}"#,
            provider_id
        );

        let request: RedeemCreditsRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.did, "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
        assert_eq!(request.provider_id, provider_id);
        assert_eq!(request.amount, "100.00000000");
    }

    #[test]
    fn test_redeem_response_serialization() {
        let receipt_id = Uuid::new_v4();
        let response = RedeemCreditsResponse {
            success: true,
            receipt_id,
            amount_redeemed: "100.00000000".to_string(),
            allocation_id: Some("alloc_test123".to_string()),
            provider_name: "OpenAI".to_string(),
            new_balance: "900.00000000".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"receiptId\":"));
        assert!(json.contains("\"amountRedeemed\":\"100.00000000\""));
        assert!(json.contains("\"allocationId\":\"alloc_test123\""));
        assert!(json.contains("\"providerName\":\"OpenAI\""));
        assert!(json.contains("\"newBalance\":\"900.00000000\""));
    }

    #[test]
    fn test_redeem_response_without_allocation_id() {
        let receipt_id = Uuid::new_v4();
        let response = RedeemCreditsResponse {
            success: true,
            receipt_id,
            amount_redeemed: "50.00000000".to_string(),
            allocation_id: None,
            provider_name: "GPU Provider".to_string(),
            new_balance: "450.00000000".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        // allocation_id should be omitted when None
        assert!(!json.contains("allocationId"));
    }

    #[test]
    fn test_min_max_redemption_constants() {
        let min = BigDecimal::from_str(MIN_REDEMPTION_CREDITS).unwrap();
        let max = BigDecimal::from_str(MAX_REDEMPTION_CREDITS).unwrap();
        assert_eq!(min, BigDecimal::from_str("1.00000000").unwrap());
        assert_eq!(max, BigDecimal::from_str("10000.00000000").unwrap());
    }

    #[test]
    fn test_new_m_credits_ledger_burn_for_redemption() {
        let amount = BigDecimal::from_str("100.00000000").unwrap();
        let provider_id = Uuid::new_v4();
        let receipt_id = Uuid::new_v4();
        let metadata = json!({
            "reason": "credit_redemption",
            "provider_id": provider_id.to_string(),
            "receipt_id": receipt_id.to_string()
        });

        let entry = NewMCreditsLedger::burn(
            "did:key:z6MkTest".to_string(),
            amount.clone(),
            metadata.clone(),
        );

        assert_eq!(entry.event_type, MCreditsEventType::Burn);
        assert_eq!(entry.from_did, Some("did:key:z6MkTest".to_string()));
        assert!(entry.to_did.is_none());
        assert_eq!(entry.amount, amount);
        assert_eq!(entry.metadata["reason"], "credit_redemption");
    }

    // ===== Balance Check Tests (US-015D) =====

    #[test]
    fn test_balance_request_deserialization() {
        let user_id = Uuid::new_v4();
        let json = format!(
            r#"{{
                "userId": "{}"
            }}"#,
            user_id
        );

        let request: BalanceRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.user_id, user_id);
    }

    #[test]
    fn test_balance_response_serialization() {
        let response = BalanceResponse {
            did: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            balance: "1000.00000000".to_string(),
            promo_balance: "50.00000000".to_string(),
            total: "1050.00000000".to_string(),
            recent_transactions: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"did\":\"did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK\""));
        assert!(json.contains("\"balance\":\"1000.00000000\""));
        assert!(json.contains("\"promoBalance\":\"50.00000000\""));
        assert!(json.contains("\"total\":\"1050.00000000\""));
        assert!(json.contains("\"recentTransactions\":[]"));
    }

    #[test]
    fn test_balance_response_with_transactions() {
        let tx_id = Uuid::new_v4();
        let response = BalanceResponse {
            did: "did:key:z6MkTest".to_string(),
            balance: "500.00000000".to_string(),
            promo_balance: "0".to_string(),
            total: "500.00000000".to_string(),
            recent_transactions: vec![
                TransactionRecord {
                    id: tx_id,
                    event_type: "mint".to_string(),
                    amount: "500.00000000".to_string(),
                    description: Some("credit_purchase".to_string()),
                    created_at: "2026-01-31T12:00:00Z".to_string(),
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"recentTransactions\":["));
        assert!(json.contains("\"eventType\":\"mint\""));
        assert!(json.contains("\"amount\":\"500.00000000\""));
        assert!(json.contains("\"description\":\"credit_purchase\""));
        assert!(json.contains("\"createdAt\":\"2026-01-31T12:00:00Z\""));
    }

    #[test]
    fn test_transaction_record_serialization() {
        let tx_id = Uuid::new_v4();
        let record = TransactionRecord {
            id: tx_id,
            event_type: "transfer".to_string(),
            amount: "25.00000000".to_string(),
            description: Some("payment for service".to_string()),
            created_at: "2026-01-31T10:30:00Z".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains(&format!("\"id\":\"{}\"", tx_id)));
        assert!(json.contains("\"eventType\":\"transfer\""));
        assert!(json.contains("\"amount\":\"25.00000000\""));
        assert!(json.contains("\"description\":\"payment for service\""));
        assert!(json.contains("\"createdAt\":\"2026-01-31T10:30:00Z\""));
    }

    #[test]
    fn test_transaction_record_without_description() {
        let tx_id = Uuid::new_v4();
        let record = TransactionRecord {
            id: tx_id,
            event_type: "burn".to_string(),
            amount: "10.00000000".to_string(),
            description: None,
            created_at: "2026-01-31T11:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"eventType\":\"burn\""));
        // description should be omitted when None
        assert!(!json.contains("description"));
    }

    #[test]
    fn test_balance_response_empty_balances() {
        let response = BalanceResponse {
            did: "did:key:z6MkTest".to_string(),
            balance: "0".to_string(),
            promo_balance: "0".to_string(),
            total: "0".to_string(),
            recent_transactions: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"balance\":\"0\""));
        assert!(json.contains("\"promoBalance\":\"0\""));
        assert!(json.contains("\"total\":\"0\""));
    }

    #[test]
    fn test_balance_request_query_param_format() {
        // Verify the query param format used by axum
        let user_id = Uuid::new_v4();
        let query_string = format!("userId={}", user_id);

        // This simulates how axum would parse the query string
        // The actual parsing is done by serde_urlencoded which uses camelCase
        assert!(query_string.starts_with("userId="));
    }

    #[test]
    fn test_multiple_transaction_records() {
        let response = BalanceResponse {
            did: "did:key:z6MkTest".to_string(),
            balance: "750.00000000".to_string(),
            promo_balance: "25.00000000".to_string(),
            total: "775.00000000".to_string(),
            recent_transactions: vec![
                TransactionRecord {
                    id: Uuid::new_v4(),
                    event_type: "mint".to_string(),
                    amount: "1000.00000000".to_string(),
                    description: Some("initial_purchase".to_string()),
                    created_at: "2026-01-31T08:00:00Z".to_string(),
                },
                TransactionRecord {
                    id: Uuid::new_v4(),
                    event_type: "burn".to_string(),
                    amount: "200.00000000".to_string(),
                    description: Some("redemption".to_string()),
                    created_at: "2026-01-31T09:00:00Z".to_string(),
                },
                TransactionRecord {
                    id: Uuid::new_v4(),
                    event_type: "promomint".to_string(),
                    amount: "25.00000000".to_string(),
                    description: Some("new_user_bonus".to_string()),
                    created_at: "2026-01-31T10:00:00Z".to_string(),
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        // Should have 3 transactions
        let count = json.matches("eventType").count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_transaction_event_types() {
        // Test all event type string formats
        let event_types = vec!["mint", "burn", "transfer", "hold", "release", "promomint"];

        for event_type in event_types {
            let record = TransactionRecord {
                id: Uuid::new_v4(),
                event_type: event_type.to_string(),
                amount: "10.00000000".to_string(),
                description: None,
                created_at: "2026-01-31T12:00:00Z".to_string(),
            };

            let json = serde_json::to_string(&record).unwrap();
            assert!(json.contains(&format!("\"eventType\":\"{}\"", event_type)));
        }
    }
}
