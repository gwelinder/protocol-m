//! Bounty marketplace endpoints.

use axum::{
    extract::{Path, State},
    routing::post,
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
    Bounty, BountyClosureType, BountyStatus, BountySubmission, EscrowStatus, NewBounty,
    NewBountySubmission, NewEscrowHold, NewMCreditsLedger, SubmissionStatus,
};
use openclaw_crypto::{did_to_verifying_key, SignatureEnvelopeV1};

/// Minimum bounty reward in credits.
const MIN_BOUNTY_REWARD: &str = "1.00000000";

/// Maximum bounty reward in credits.
const MAX_BOUNTY_REWARD: &str = "1000000.00000000";

/// Request body for creating a bounty.
/// Note: In production, user_id would come from authentication.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBountyRequest {
    /// The user ID creating the bounty.
    /// In production, this would be extracted from auth token.
    pub user_id: Uuid,
    /// Title of the bounty.
    pub title: String,
    /// Detailed description of the task.
    pub description: String,
    /// Amount of M-credits offered as reward.
    pub reward_credits: String,
    /// How bounty completion is verified.
    pub closure_type: BountyClosureType,
    /// Optional deadline for bounty completion (ISO 8601 format).
    #[serde(default)]
    pub deadline: Option<String>,
    /// Additional closure-type specific configuration.
    /// For tests: { "evalHarnessHash": "sha256:..." }
    /// For quorum: { "reviewerCount": 3, "minReviewerRep": 100 }
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Response for successful bounty creation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBountyResponse {
    /// Whether the bounty was created successfully.
    pub success: bool,
    /// The unique bounty ID.
    pub bounty_id: Uuid,
    /// Title of the bounty.
    pub title: String,
    /// Amount of credits in escrow.
    pub reward_credits: String,
    /// Current bounty status.
    pub status: BountyStatus,
    /// The escrow hold ID.
    pub escrow_id: Uuid,
    /// The ledger entry ID for the hold.
    pub ledger_id: Uuid,
}

/// Request body for submitting work to a bounty.
/// Note: In production, user_id would come from authentication.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitBountyRequest {
    /// The user ID submitting the work.
    /// In production, this would be extracted from auth token.
    pub user_id: Uuid,
    /// The signed artifact envelope (SignatureEnvelopeV1).
    pub signature_envelope: serde_json::Value,
    /// Optional execution receipt for test-based bounties.
    /// Required for closure_type=tests.
    #[serde(default)]
    pub execution_receipt: Option<serde_json::Value>,
}

/// Response for successful bounty submission.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitBountyResponse {
    /// Whether the submission was created successfully.
    pub success: bool,
    /// The unique submission ID.
    pub submission_id: Uuid,
    /// ID of the bounty this submission is for.
    pub bounty_id: Uuid,
    /// DID of the submitter.
    pub submitter_did: String,
    /// Current status of the submission.
    pub status: SubmissionStatus,
    /// Whether auto-approval was performed (for test-based bounties).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_approved: Option<bool>,
    /// Message explaining the submission result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Creates the bounties router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/", post(create_bounty))
        .route("/{id}/submit", post(submit_bounty))
        .with_state(pool)
}

/// Validates DID format.
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

/// Validates the bounty reward amount is within acceptable bounds.
fn validate_reward_amount(amount: &BigDecimal) -> Result<(), AppError> {
    let min = BigDecimal::from_str(MIN_BOUNTY_REWARD).unwrap();
    let max = BigDecimal::from_str(MAX_BOUNTY_REWARD).unwrap();

    if amount < &min {
        return Err(AppError::BadRequest(format!(
            "Minimum bounty reward is {} M-credits",
            MIN_BOUNTY_REWARD
        )));
    }

    if amount > &max {
        return Err(AppError::BadRequest(format!(
            "Maximum bounty reward is {} M-credits",
            MAX_BOUNTY_REWARD
        )));
    }

    if amount <= &BigDecimal::from(0) {
        return Err(AppError::BadRequest(
            "Reward amount must be positive".to_string(),
        ));
    }

    Ok(())
}

/// Validates the title is not empty and within length limits.
fn validate_title(title: &str) -> Result<(), AppError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(
            "Bounty title cannot be empty".to_string(),
        ));
    }
    if trimmed.len() > 200 {
        return Err(AppError::BadRequest(
            "Bounty title must be 200 characters or less".to_string(),
        ));
    }
    Ok(())
}

/// Validates the description is not empty.
fn validate_description(description: &str) -> Result<(), AppError> {
    let trimmed = description.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(
            "Bounty description cannot be empty".to_string(),
        ));
    }
    if trimmed.len() > 10000 {
        return Err(AppError::BadRequest(
            "Bounty description must be 10000 characters or less".to_string(),
        ));
    }
    Ok(())
}

/// Validates closure-type-specific metadata requirements.
fn validate_closure_type_metadata(
    closure_type: BountyClosureType,
    metadata: &Option<serde_json::Value>,
) -> Result<serde_json::Value, AppError> {
    match closure_type {
        BountyClosureType::Tests => {
            // Require eval_harness_hash in metadata
            let meta = metadata.as_ref().ok_or_else(|| {
                AppError::BadRequest(
                    "Tests closure type requires 'evalHarnessHash' in metadata".to_string(),
                )
            })?;

            let hash = meta
                .get("evalHarnessHash")
                .or_else(|| meta.get("eval_harness_hash"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AppError::BadRequest(
                        "Tests closure type requires 'evalHarnessHash' in metadata".to_string(),
                    )
                })?;

            if hash.trim().is_empty() {
                return Err(AppError::BadRequest(
                    "evalHarnessHash cannot be empty".to_string(),
                ));
            }

            // Return normalized metadata
            Ok(json!({
                "eval_harness_hash": hash
            }))
        }
        BountyClosureType::Quorum => {
            // Require reviewer_count and min_reviewer_rep in metadata
            let meta = metadata.as_ref().ok_or_else(|| {
                AppError::BadRequest(
                    "Quorum closure type requires 'reviewerCount' and 'minReviewerRep' in metadata"
                        .to_string(),
                )
            })?;

            let reviewer_count = meta
                .get("reviewerCount")
                .or_else(|| meta.get("reviewer_count"))
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    AppError::BadRequest(
                        "Quorum closure type requires 'reviewerCount' (integer) in metadata"
                            .to_string(),
                    )
                })?;

            let min_reviewer_rep = meta
                .get("minReviewerRep")
                .or_else(|| meta.get("min_reviewer_rep"))
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    AppError::BadRequest(
                        "Quorum closure type requires 'minReviewerRep' (integer) in metadata"
                            .to_string(),
                    )
                })?;

            if reviewer_count < 1 {
                return Err(AppError::BadRequest(
                    "reviewerCount must be at least 1".to_string(),
                ));
            }

            if min_reviewer_rep < 0 {
                return Err(AppError::BadRequest(
                    "minReviewerRep cannot be negative".to_string(),
                ));
            }

            // Return normalized metadata
            Ok(json!({
                "reviewer_count": reviewer_count,
                "min_reviewer_rep": min_reviewer_rep
            }))
        }
        BountyClosureType::Requester => {
            // No special metadata required for requester-based closure
            Ok(json!({}))
        }
    }
}

/// Parses and validates the deadline timestamp.
fn parse_deadline(deadline: Option<&str>) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
    match deadline {
        None => Ok(None),
        Some(d) => {
            let parsed = chrono::DateTime::parse_from_rfc3339(d)
                .map_err(|e| AppError::BadRequest(format!("Invalid deadline format: {}", e)))?;

            let deadline_utc = parsed.with_timezone(&chrono::Utc);

            // Check deadline is in the future
            if deadline_utc <= chrono::Utc::now() {
                return Err(AppError::BadRequest(
                    "Deadline must be in the future".to_string(),
                ));
            }

            Ok(Some(deadline_utc))
        }
    }
}

/// Gets the active DID binding for a user.
/// Returns the DID if the user has an active binding, error otherwise.
async fn get_user_bound_did(pool: &PgPool, user_id: Uuid) -> Result<String, AppError> {
    let binding: Option<(String,)> = sqlx::query_as(
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

    binding
        .map(|(did,)| did)
        .ok_or_else(|| AppError::BadRequest(
            "User must have a bound DID to post bounties. Please bind your DID first.".to_string(),
        ))
}

/// Gets the current balance for a DID.
/// Returns (main_balance, promo_balance).
async fn get_did_balance(pool: &PgPool, did: &str) -> Result<(BigDecimal, BigDecimal), AppError> {
    let account: Option<(BigDecimal, BigDecimal)> = sqlx::query_as(
        r#"
        SELECT balance, promo_balance
        FROM m_credits_accounts
        WHERE did = $1
        "#,
    )
    .bind(did)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query balance: {}", e)))?;

    Ok(account.unwrap_or_else(|| (BigDecimal::from(0), BigDecimal::from(0))))
}

/// Checks if a DID has sufficient balance for a bounty.
/// Returns an error if insufficient balance.
fn check_sufficient_balance(
    main_balance: &BigDecimal,
    promo_balance: &BigDecimal,
    required: &BigDecimal,
) -> Result<(), AppError> {
    let total = main_balance + promo_balance;
    if &total < required {
        return Err(AppError::BadRequest(format!(
            "Insufficient balance. Required: {}, Available: {} (main: {}, promo: {})",
            required, total, main_balance, promo_balance
        )));
    }
    Ok(())
}

/// Creates an escrow hold by:
/// 1. Inserting a hold event into the ledger
/// 2. Creating an escrow_holds record
/// 3. Deducting from the poster's balance
///
/// Returns (escrow_id, ledger_id).
async fn create_escrow_hold(
    pool: &PgPool,
    bounty_id: Uuid,
    holder_did: &str,
    amount: &BigDecimal,
) -> Result<(Uuid, Uuid), AppError> {
    // Step 1: Insert hold event into ledger
    let ledger_entry = NewMCreditsLedger::hold(
        holder_did.to_string(),
        amount.clone(),
        json!({
            "bounty_id": bounty_id.to_string(),
            "reason": "bounty_escrow"
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

    // Step 2: Create escrow hold record
    let escrow_id = Uuid::new_v4();
    let new_escrow = NewEscrowHold::new(bounty_id, holder_did.to_string(), amount.clone());

    sqlx::query(
        r#"
        INSERT INTO escrow_holds (id, bounty_id, holder_did, amount, status, created_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        "#,
    )
    .bind(escrow_id)
    .bind(new_escrow.bounty_id)
    .bind(&new_escrow.holder_did)
    .bind(&new_escrow.amount)
    .bind(EscrowStatus::Held)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create escrow hold: {}", e)))?;

    // Step 3: Deduct from poster's balance
    // First try to deduct from main balance, then from promo balance if needed
    // For simplicity, we deduct from main balance first
    sqlx::query(
        r#"
        UPDATE m_credits_accounts
        SET balance = balance - $2
        WHERE did = $1
        "#,
    )
    .bind(holder_did)
    .bind(amount)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to deduct balance: {}", e)))?;

    Ok((escrow_id, ledger_id))
}

/// Creates a new bounty record.
async fn create_bounty(
    State(pool): State<PgPool>,
    Json(request): Json<CreateBountyRequest>,
) -> Result<Json<CreateBountyResponse>, AppError> {
    // Step 1: Validate title and description
    validate_title(&request.title)?;
    validate_description(&request.description)?;

    // Step 2: Parse and validate reward amount
    let reward_credits = BigDecimal::from_str(&request.reward_credits).map_err(|e| {
        AppError::BadRequest(format!("Invalid reward amount format: {}", e))
    })?;
    validate_reward_amount(&reward_credits)?;

    // Step 3: Validate closure-type specific metadata
    let normalized_metadata =
        validate_closure_type_metadata(request.closure_type, &request.metadata)?;

    // Step 4: Parse deadline
    let deadline = parse_deadline(request.deadline.as_deref())?;

    // Step 5: Get user's bound DID (authentication + DID binding check)
    let poster_did = get_user_bound_did(&pool, request.user_id).await?;
    validate_did_format(&poster_did)?;

    // Step 6: Check poster has sufficient balance
    let (main_balance, promo_balance) = get_did_balance(&pool, &poster_did).await?;
    check_sufficient_balance(&main_balance, &promo_balance, &reward_credits)?;

    // Step 7: Generate bounty ID
    let bounty_id = Uuid::new_v4();

    // Step 8: Create escrow hold (deduct from balance, lock in escrow)
    let (escrow_id, ledger_id) =
        create_escrow_hold(&pool, bounty_id, &poster_did, &reward_credits).await?;

    // Step 9: Create bounty record
    let new_bounty = NewBounty {
        poster_did: poster_did.clone(),
        title: request.title.trim().to_string(),
        description: request.description.trim().to_string(),
        reward_credits: reward_credits.clone(),
        closure_type: request.closure_type,
        metadata: normalized_metadata,
        deadline,
    };

    let bounty: Bounty = sqlx::query_as(
        r#"
        INSERT INTO bounties (id, poster_did, title, description, reward_credits, closure_type, status, metadata, created_at, updated_at, deadline)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW(), $9)
        RETURNING id, poster_did, title, description, reward_credits, closure_type, status, metadata, created_at, updated_at, deadline
        "#,
    )
    .bind(bounty_id)
    .bind(&new_bounty.poster_did)
    .bind(&new_bounty.title)
    .bind(&new_bounty.description)
    .bind(&new_bounty.reward_credits)
    .bind(new_bounty.closure_type)
    .bind(BountyStatus::Open)
    .bind(&new_bounty.metadata)
    .bind(new_bounty.deadline)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create bounty: {}", e)))?;

    // Step 10: Return response
    Ok(Json(CreateBountyResponse {
        success: true,
        bounty_id: bounty.id,
        title: bounty.title,
        reward_credits: bounty.reward_credits.to_string(),
        status: bounty.status,
        escrow_id,
        ledger_id,
    }))
}

// ===== Bounty Submission Endpoint =====

/// Loads a bounty by ID and validates it is open for submissions.
async fn load_bounty_for_submission(pool: &PgPool, bounty_id: Uuid) -> Result<Bounty, AppError> {
    let bounty: Option<Bounty> = sqlx::query_as(
        r#"
        SELECT id, poster_did, title, description, reward_credits, closure_type, status, metadata, created_at, updated_at, deadline
        FROM bounties
        WHERE id = $1
        "#,
    )
    .bind(bounty_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query bounty: {}", e)))?;

    let bounty = bounty.ok_or_else(|| AppError::NotFound(format!("Bounty not found: {}", bounty_id)))?;

    // Check bounty is open for submissions
    if !bounty.is_open() {
        return Err(AppError::BadRequest(format!(
            "Bounty is not open for submissions. Current status: {:?}",
            bounty.status
        )));
    }

    // Check bounty hasn't expired
    if bounty.is_expired() {
        return Err(AppError::BadRequest(
            "Bounty has expired and is no longer accepting submissions".to_string(),
        ));
    }

    Ok(bounty)
}

/// Parses and validates the signature envelope from the request.
fn parse_signature_envelope(envelope_json: &serde_json::Value) -> Result<SignatureEnvelopeV1, AppError> {
    serde_json::from_value(envelope_json.clone())
        .map_err(|e| AppError::BadRequest(format!("Invalid signature envelope format: {}", e)))
}

/// Verifies the signature envelope cryptographically.
fn verify_envelope_signature(envelope: &SignatureEnvelopeV1) -> Result<(), AppError> {
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;
    use ed25519_dalek::{Signature, Verifier};
    use openclaw_crypto::jcs_canonical_bytes;

    // Validate envelope version/type/algo
    if envelope.version != "1.0" {
        return Err(AppError::BadRequest(format!(
            "Unsupported envelope version: '{}' (expected '1.0')",
            envelope.version
        )));
    }

    // Allow both "signature-envelope" and "contribution-manifest" types
    if envelope.envelope_type != "signature-envelope" && envelope.envelope_type != "contribution-manifest" {
        return Err(AppError::BadRequest(format!(
            "Invalid envelope type: '{}' (expected 'signature-envelope' or 'contribution-manifest')",
            envelope.envelope_type
        )));
    }

    if envelope.algo != "ed25519" {
        return Err(AppError::BadRequest(format!(
            "Unsupported signature algorithm: '{}' (expected 'ed25519')",
            envelope.algo
        )));
    }

    if envelope.hash.algo != "sha-256" {
        return Err(AppError::BadRequest(format!(
            "Unsupported hash algorithm: '{}' (expected 'sha-256')",
            envelope.hash.algo
        )));
    }

    // Extract verifying key from DID
    let verifying_key = did_to_verifying_key(&envelope.signer)
        .map_err(|e| AppError::BadRequest(format!("Invalid DID in envelope: {}", e)))?;

    // Create envelope copy with empty signature for canonicalization
    let mut verify_envelope = envelope.clone();
    verify_envelope.signature = String::new();

    // Canonicalize envelope with JCS
    let canonical_bytes = jcs_canonical_bytes(&verify_envelope)
        .map_err(|e| AppError::BadRequest(format!("Failed to canonicalize envelope: {}", e)))?;

    // Decode base64 signature
    let signature_bytes = BASE64_STANDARD
        .decode(&envelope.signature)
        .map_err(|e| AppError::BadRequest(format!("Invalid base64 signature: {}", e)))?;

    let signature_array: [u8; 64] = signature_bytes.try_into().map_err(|_| {
        AppError::BadRequest("Invalid signature length: expected 64 bytes".to_string())
    })?;

    let signature = Signature::from_bytes(&signature_array);

    // Verify signature with ed25519
    verifying_key
        .verify(&canonical_bytes, &signature)
        .map_err(|_| AppError::BadRequest("Signature verification failed".to_string()))?;

    Ok(())
}

/// Validates the execution receipt for test-based bounties.
fn validate_execution_receipt_for_tests(
    execution_receipt: &Option<serde_json::Value>,
) -> Result<(), AppError> {
    let receipt = execution_receipt.as_ref().ok_or_else(|| {
        AppError::BadRequest(
            "Test-based bounties require an execution_receipt with test results".to_string(),
        )
    })?;

    // Check that harness_hash is present
    if receipt.get("harness_hash").and_then(|v| v.as_str()).is_none()
        && receipt.get("harnessHash").and_then(|v| v.as_str()).is_none()
    {
        return Err(AppError::BadRequest(
            "Execution receipt must include 'harness_hash' field".to_string(),
        ));
    }

    // Check that all_tests_passed is present (boolean)
    let has_tests_passed = receipt.get("all_tests_passed").and_then(|v| v.as_bool()).is_some()
        || receipt.get("allTestsPassed").and_then(|v| v.as_bool()).is_some();

    if !has_tests_passed {
        return Err(AppError::BadRequest(
            "Execution receipt must include 'all_tests_passed' (boolean) field".to_string(),
        ));
    }

    Ok(())
}

/// Result of test-based auto-approval verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestVerificationResult {
    /// All tests passed and harness hash matches - approve submission.
    Approved,
    /// Harness hash does not match - reject submission.
    HarnessHashMismatch { expected: String, actual: String },
    /// Tests did not all pass - reject submission.
    TestsFailed,
}

/// Gets the harness hash from an execution receipt (supports both camelCase and snake_case).
fn get_receipt_harness_hash(receipt: &serde_json::Value) -> Option<&str> {
    receipt
        .get("harness_hash")
        .or_else(|| receipt.get("harnessHash"))
        .and_then(|v| v.as_str())
}

/// Gets the all_tests_passed value from an execution receipt (supports both camelCase and snake_case).
fn get_receipt_all_tests_passed(receipt: &serde_json::Value) -> Option<bool> {
    receipt
        .get("all_tests_passed")
        .or_else(|| receipt.get("allTestsPassed"))
        .and_then(|v| v.as_bool())
}

/// Verifies a test-based submission by checking:
/// 1. Execution receipt harness_hash matches bounty's eval_harness_hash
/// 2. all_tests_passed is true
///
/// Returns the verification result.
fn verify_test_submission(
    bounty: &Bounty,
    execution_receipt: &serde_json::Value,
) -> TestVerificationResult {
    // Get the expected harness hash from bounty metadata
    let expected_hash = match bounty.eval_harness_hash() {
        Some(h) => h,
        None => {
            // This shouldn't happen for test-based bounties, but handle gracefully
            return TestVerificationResult::HarnessHashMismatch {
                expected: "".to_string(),
                actual: "unknown".to_string(),
            };
        }
    };

    // Get the actual harness hash from execution receipt
    let actual_hash = match get_receipt_harness_hash(execution_receipt) {
        Some(h) => h,
        None => {
            return TestVerificationResult::HarnessHashMismatch {
                expected: expected_hash.to_string(),
                actual: "".to_string(),
            };
        }
    };

    // Check harness hash matches
    if expected_hash != actual_hash {
        return TestVerificationResult::HarnessHashMismatch {
            expected: expected_hash.to_string(),
            actual: actual_hash.to_string(),
        };
    }

    // Check all tests passed
    match get_receipt_all_tests_passed(execution_receipt) {
        Some(true) => TestVerificationResult::Approved,
        Some(false) | None => TestVerificationResult::TestsFailed,
    }
}

/// Updates a submission status to approved or rejected.
async fn update_submission_status(
    pool: &PgPool,
    submission_id: Uuid,
    status: SubmissionStatus,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE bounty_submissions
        SET status = $2
        WHERE id = $1
        "#,
    )
    .bind(submission_id)
    .bind(status)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update submission status: {}", e)))?;

    Ok(())
}

/// Submits work to an open bounty.
async fn submit_bounty(
    State(pool): State<PgPool>,
    Path(bounty_id): Path<Uuid>,
    Json(request): Json<SubmitBountyRequest>,
) -> Result<Json<SubmitBountyResponse>, AppError> {
    // Step 1: Load and validate the bounty
    let bounty = load_bounty_for_submission(&pool, bounty_id).await?;

    // Step 2: Get user's bound DID (authentication + DID binding check)
    let submitter_did = get_user_bound_did(&pool, request.user_id).await
        .map_err(|_| AppError::BadRequest(
            "User must have a bound DID to submit work. Please bind your DID first.".to_string(),
        ))?;

    // Step 3: Parse and validate the signature envelope
    let envelope = parse_signature_envelope(&request.signature_envelope)?;

    // Step 4: Verify envelope signature cryptographically
    verify_envelope_signature(&envelope)?;

    // Step 5: Verify the envelope signer matches the submitter's DID
    if envelope.signer != submitter_did {
        return Err(AppError::BadRequest(format!(
            "Envelope signer '{}' does not match submitter DID '{}'",
            envelope.signer, submitter_did
        )));
    }

    // Step 6: For test-based bounties, validate execution_receipt
    if bounty.uses_tests() {
        validate_execution_receipt_for_tests(&request.execution_receipt)?;
    }

    // Step 7: Extract artifact hash from envelope
    let artifact_hash = envelope.hash.value.clone();

    // Step 8: Create the submission record
    let submission_id = Uuid::new_v4();
    let new_submission = if bounty.uses_tests() {
        NewBountySubmission::with_execution_receipt(
            bounty_id,
            submitter_did.clone(),
            artifact_hash.clone(),
            request.signature_envelope.clone(),
            request.execution_receipt.clone().unwrap(),
        )
    } else {
        NewBountySubmission::without_execution_receipt(
            bounty_id,
            submitter_did.clone(),
            artifact_hash.clone(),
            request.signature_envelope.clone(),
        )
    };

    let submission: BountySubmission = sqlx::query_as(
        r#"
        INSERT INTO bounty_submissions (id, bounty_id, submitter_did, artifact_hash, signature_envelope, execution_receipt, status, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
        RETURNING id, bounty_id, submitter_did, artifact_hash, signature_envelope, execution_receipt, status, created_at
        "#,
    )
    .bind(submission_id)
    .bind(new_submission.bounty_id)
    .bind(&new_submission.submitter_did)
    .bind(&new_submission.artifact_hash)
    .bind(&new_submission.signature_envelope)
    .bind(&new_submission.execution_receipt)
    .bind(SubmissionStatus::Pending)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create submission: {}", e)))?;

    // Step 9: For test-based bounties, perform auto-approval verification
    let (final_status, auto_approved, message) = if bounty.uses_tests() {
        let execution_receipt = request.execution_receipt.as_ref().unwrap(); // Safe: validated earlier
        let result = verify_test_submission(&bounty, execution_receipt);

        match result {
            TestVerificationResult::Approved => {
                // Auto-approve the submission
                update_submission_status(&pool, submission.id, SubmissionStatus::Approved).await?;
                (
                    SubmissionStatus::Approved,
                    Some(true),
                    Some("Tests passed and harness hash verified - submission auto-approved".to_string()),
                )
            }
            TestVerificationResult::HarnessHashMismatch { expected, actual } => {
                // Auto-reject the submission
                update_submission_status(&pool, submission.id, SubmissionStatus::Rejected).await?;
                (
                    SubmissionStatus::Rejected,
                    Some(true),
                    Some(format!(
                        "Harness hash mismatch - expected '{}', got '{}'",
                        expected, actual
                    )),
                )
            }
            TestVerificationResult::TestsFailed => {
                // Auto-reject the submission
                update_submission_status(&pool, submission.id, SubmissionStatus::Rejected).await?;
                (
                    SubmissionStatus::Rejected,
                    Some(true),
                    Some("Tests did not pass - submission auto-rejected".to_string()),
                )
            }
        }
    } else {
        // Non-test bounties remain pending for manual review
        (SubmissionStatus::Pending, None, None)
    };

    // Step 10: Return response
    Ok(Json(SubmitBountyResponse {
        success: true,
        submission_id: submission.id,
        bounty_id: submission.bounty_id,
        submitter_did: submission.submitter_did,
        status: final_status,
        auto_approved,
        message,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Validation Tests =====

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
    fn test_validate_reward_amount_valid() {
        let amount = BigDecimal::from_str("100.00000000").unwrap();
        let result = validate_reward_amount(&amount);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_reward_amount_minimum() {
        let amount = BigDecimal::from_str("1.00000000").unwrap();
        let result = validate_reward_amount(&amount);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_reward_amount_below_minimum() {
        let amount = BigDecimal::from_str("0.50000000").unwrap();
        let result = validate_reward_amount(&amount);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Minimum bounty reward"));
    }

    #[test]
    fn test_validate_reward_amount_above_maximum() {
        let amount = BigDecimal::from_str("1000001.00000000").unwrap();
        let result = validate_reward_amount(&amount);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Maximum bounty reward"));
    }

    #[test]
    fn test_validate_reward_amount_zero() {
        let amount = BigDecimal::from(0);
        let result = validate_reward_amount(&amount);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_reward_amount_negative() {
        let amount = BigDecimal::from_str("-10.00000000").unwrap();
        let result = validate_reward_amount(&amount);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_title_valid() {
        let result = validate_title("Fix authentication bug");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_title_empty() {
        let result = validate_title("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_title_whitespace() {
        let result = validate_title("   ");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_title_too_long() {
        let long_title = "a".repeat(201);
        let result = validate_title(&long_title);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("200 characters"));
    }

    #[test]
    fn test_validate_description_valid() {
        let result = validate_description("Please fix this important bug.");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_description_empty() {
        let result = validate_description("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    // ===== Closure Type Metadata Validation Tests =====

    #[test]
    fn test_validate_tests_closure_with_hash() {
        let metadata = Some(json!({
            "evalHarnessHash": "sha256:abc123def456"
        }));
        let result = validate_closure_type_metadata(BountyClosureType::Tests, &metadata);
        assert!(result.is_ok());
        let normalized = result.unwrap();
        assert_eq!(normalized["eval_harness_hash"], "sha256:abc123def456");
    }

    #[test]
    fn test_validate_tests_closure_snake_case() {
        let metadata = Some(json!({
            "eval_harness_hash": "sha256:abc123def456"
        }));
        let result = validate_closure_type_metadata(BountyClosureType::Tests, &metadata);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_tests_closure_missing_hash() {
        let metadata = Some(json!({}));
        let result = validate_closure_type_metadata(BountyClosureType::Tests, &metadata);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("evalHarnessHash"));
    }

    #[test]
    fn test_validate_tests_closure_no_metadata() {
        let result = validate_closure_type_metadata(BountyClosureType::Tests, &None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("evalHarnessHash"));
    }

    #[test]
    fn test_validate_tests_closure_empty_hash() {
        let metadata = Some(json!({
            "evalHarnessHash": ""
        }));
        let result = validate_closure_type_metadata(BountyClosureType::Tests, &metadata);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_quorum_closure_valid() {
        let metadata = Some(json!({
            "reviewerCount": 3,
            "minReviewerRep": 100
        }));
        let result = validate_closure_type_metadata(BountyClosureType::Quorum, &metadata);
        assert!(result.is_ok());
        let normalized = result.unwrap();
        assert_eq!(normalized["reviewer_count"], 3);
        assert_eq!(normalized["min_reviewer_rep"], 100);
    }

    #[test]
    fn test_validate_quorum_closure_snake_case() {
        let metadata = Some(json!({
            "reviewer_count": 5,
            "min_reviewer_rep": 50
        }));
        let result = validate_closure_type_metadata(BountyClosureType::Quorum, &metadata);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_quorum_closure_missing_reviewer_count() {
        let metadata = Some(json!({
            "minReviewerRep": 100
        }));
        let result = validate_closure_type_metadata(BountyClosureType::Quorum, &metadata);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reviewerCount"));
    }

    #[test]
    fn test_validate_quorum_closure_missing_min_rep() {
        let metadata = Some(json!({
            "reviewerCount": 3
        }));
        let result = validate_closure_type_metadata(BountyClosureType::Quorum, &metadata);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("minReviewerRep"));
    }

    #[test]
    fn test_validate_quorum_closure_invalid_reviewer_count() {
        let metadata = Some(json!({
            "reviewerCount": 0,
            "minReviewerRep": 100
        }));
        let result = validate_closure_type_metadata(BountyClosureType::Quorum, &metadata);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least 1"));
    }

    #[test]
    fn test_validate_quorum_closure_negative_min_rep() {
        let metadata = Some(json!({
            "reviewerCount": 3,
            "minReviewerRep": -10
        }));
        let result = validate_closure_type_metadata(BountyClosureType::Quorum, &metadata);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be negative"));
    }

    #[test]
    fn test_validate_requester_closure_no_metadata() {
        let result = validate_closure_type_metadata(BountyClosureType::Requester, &None);
        assert!(result.is_ok());
        assert!(result.unwrap().as_object().unwrap().is_empty());
    }

    #[test]
    fn test_validate_requester_closure_with_metadata() {
        let metadata = Some(json!({
            "extra": "data"
        }));
        // Requester closure ignores extra metadata
        let result = validate_closure_type_metadata(BountyClosureType::Requester, &metadata);
        assert!(result.is_ok());
    }

    // ===== Deadline Parsing Tests =====

    #[test]
    fn test_parse_deadline_none() {
        let result = parse_deadline(None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_deadline_valid_future() {
        // Use a date far in the future
        let result = parse_deadline(Some("2030-12-31T23:59:59Z"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_parse_deadline_invalid_format() {
        let result = parse_deadline(Some("not-a-date"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid deadline format"));
    }

    #[test]
    fn test_parse_deadline_past_date() {
        let result = parse_deadline(Some("2020-01-01T00:00:00Z"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be in the future"));
    }

    // ===== Balance Check Tests =====

    #[test]
    fn test_check_sufficient_balance_enough() {
        let main = BigDecimal::from_str("100.00000000").unwrap();
        let promo = BigDecimal::from_str("50.00000000").unwrap();
        let required = BigDecimal::from_str("120.00000000").unwrap();
        let result = check_sufficient_balance(&main, &promo, &required);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_sufficient_balance_exact() {
        let main = BigDecimal::from_str("100.00000000").unwrap();
        let promo = BigDecimal::from_str("0.00000000").unwrap();
        let required = BigDecimal::from_str("100.00000000").unwrap();
        let result = check_sufficient_balance(&main, &promo, &required);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_sufficient_balance_insufficient() {
        let main = BigDecimal::from_str("50.00000000").unwrap();
        let promo = BigDecimal::from_str("30.00000000").unwrap();
        let required = BigDecimal::from_str("100.00000000").unwrap();
        let result = check_sufficient_balance(&main, &promo, &required);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Insufficient balance"));
    }

    #[test]
    fn test_check_sufficient_balance_zero_balance() {
        let main = BigDecimal::from(0);
        let promo = BigDecimal::from(0);
        let required = BigDecimal::from_str("10.00000000").unwrap();
        let result = check_sufficient_balance(&main, &promo, &required);
        assert!(result.is_err());
    }

    // ===== Request Deserialization Tests =====

    #[test]
    fn test_create_bounty_request_deserialization() {
        let json = r#"{
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "title": "Fix login bug",
            "description": "The login page crashes on mobile",
            "rewardCredits": "100.00000000",
            "closureType": "tests",
            "metadata": {
                "evalHarnessHash": "sha256:abc123"
            }
        }"#;

        let request: CreateBountyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.title, "Fix login bug");
        assert_eq!(request.reward_credits, "100.00000000");
        assert_eq!(request.closure_type, BountyClosureType::Tests);
        assert!(request.deadline.is_none());
    }

    #[test]
    fn test_create_bounty_request_with_deadline() {
        let json = r#"{
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "title": "Review code",
            "description": "Review my pull request",
            "rewardCredits": "50.00000000",
            "closureType": "quorum",
            "deadline": "2030-06-30T12:00:00Z",
            "metadata": {
                "reviewerCount": 3,
                "minReviewerRep": 50
            }
        }"#;

        let request: CreateBountyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.closure_type, BountyClosureType::Quorum);
        assert_eq!(request.deadline, Some("2030-06-30T12:00:00Z".to_string()));
    }

    #[test]
    fn test_create_bounty_request_requester_type() {
        let json = r#"{
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "title": "Design logo",
            "description": "Create a new logo for my project",
            "rewardCredits": "200.00000000",
            "closureType": "requester"
        }"#;

        let request: CreateBountyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.closure_type, BountyClosureType::Requester);
        assert!(request.metadata.is_none());
    }

    // ===== Response Serialization Tests =====

    #[test]
    fn test_create_bounty_response_serialization() {
        let response = CreateBountyResponse {
            success: true,
            bounty_id: Uuid::new_v4(),
            title: "Test bounty".to_string(),
            reward_credits: "100.00000000".to_string(),
            status: BountyStatus::Open,
            escrow_id: Uuid::new_v4(),
            ledger_id: Uuid::new_v4(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"bountyId\":"));
        assert!(json.contains("\"title\":\"Test bounty\""));
        assert!(json.contains("\"rewardCredits\":\"100.00000000\""));
        assert!(json.contains("\"status\":\"open\""));
        assert!(json.contains("\"escrowId\":"));
        assert!(json.contains("\"ledgerId\":"));
    }

    // ===== NewMCreditsLedger Hold Test =====

    #[test]
    fn test_new_m_credits_ledger_hold() {
        use crate::models::MCreditsEventType;

        let amount = BigDecimal::from_str("100.00000000").unwrap();
        let metadata = json!({
            "bounty_id": "test-bounty-123",
            "reason": "bounty_escrow"
        });

        let entry = NewMCreditsLedger::hold(
            "did:key:z6MkTest".to_string(),
            amount.clone(),
            metadata.clone(),
        );

        assert_eq!(entry.event_type, MCreditsEventType::Hold);
        assert_eq!(entry.from_did, Some("did:key:z6MkTest".to_string()));
        assert!(entry.to_did.is_none());
        assert_eq!(entry.amount, amount);
        assert_eq!(entry.metadata["reason"], "bounty_escrow");
    }

    // ===== Submission Request/Response Tests =====

    #[test]
    fn test_submit_bounty_request_deserialization() {
        let json = r#"{
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "signatureEnvelope": {
                "version": "1.0",
                "type": "signature-envelope",
                "algo": "ed25519",
                "signer": "did:key:z6MkTest",
                "hash": {"algo": "sha-256", "value": "abc123"},
                "artifact": {"name": "test.txt", "size": 100},
                "signature": "base64sig",
                "timestamp": "2026-01-01T00:00:00Z"
            }
        }"#;

        let request: SubmitBountyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.user_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
        assert!(request.execution_receipt.is_none());
    }

    #[test]
    fn test_submit_bounty_request_with_execution_receipt() {
        let json = r#"{
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "signatureEnvelope": {
                "version": "1.0",
                "type": "signature-envelope",
                "algo": "ed25519",
                "signer": "did:key:z6MkTest",
                "hash": {"algo": "sha-256", "value": "abc123"},
                "artifact": {"name": "test.txt", "size": 100},
                "signature": "base64sig",
                "timestamp": "2026-01-01T00:00:00Z"
            },
            "executionReceipt": {
                "harness_hash": "sha256:testharness",
                "all_tests_passed": true,
                "test_results": {"passed": 10, "failed": 0}
            }
        }"#;

        let request: SubmitBountyRequest = serde_json::from_str(json).unwrap();
        assert!(request.execution_receipt.is_some());
        let receipt = request.execution_receipt.unwrap();
        assert_eq!(receipt["harness_hash"], "sha256:testharness");
        assert_eq!(receipt["all_tests_passed"], true);
    }

    #[test]
    fn test_submit_bounty_response_serialization() {
        let response = SubmitBountyResponse {
            success: true,
            submission_id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            status: SubmissionStatus::Pending,
            auto_approved: None,
            message: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"submissionId\":"));
        assert!(json.contains("\"bountyId\":"));
        assert!(json.contains("\"submitterDid\":\"did:key:z6MkTest\""));
        assert!(json.contains("\"status\":\"pending\""));
        // Optional fields should be omitted when None
        assert!(!json.contains("autoApproved"));
        assert!(!json.contains("message"));
    }

    #[test]
    fn test_submit_bounty_response_with_auto_approval() {
        let response = SubmitBountyResponse {
            success: true,
            submission_id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            status: SubmissionStatus::Approved,
            auto_approved: Some(true),
            message: Some("Tests passed - auto-approved".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"approved\""));
        assert!(json.contains("\"autoApproved\":true"));
        assert!(json.contains("\"message\":\"Tests passed - auto-approved\""));
    }

    // ===== Execution Receipt Validation Tests =====

    #[test]
    fn test_validate_execution_receipt_valid() {
        let receipt = Some(json!({
            "harness_hash": "sha256:testharness",
            "all_tests_passed": true,
            "test_results": {"passed": 10, "failed": 0}
        }));

        let result = validate_execution_receipt_for_tests(&receipt);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_execution_receipt_camel_case() {
        let receipt = Some(json!({
            "harnessHash": "sha256:testharness",
            "allTestsPassed": false,
            "testResults": {"passed": 8, "failed": 2}
        }));

        let result = validate_execution_receipt_for_tests(&receipt);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_execution_receipt_missing() {
        let result = validate_execution_receipt_for_tests(&None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("require an execution_receipt"));
    }

    #[test]
    fn test_validate_execution_receipt_missing_harness_hash() {
        let receipt = Some(json!({
            "all_tests_passed": true,
            "test_results": {"passed": 10, "failed": 0}
        }));

        let result = validate_execution_receipt_for_tests(&receipt);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("harness_hash"));
    }

    #[test]
    fn test_validate_execution_receipt_missing_tests_passed() {
        let receipt = Some(json!({
            "harness_hash": "sha256:testharness",
            "test_results": {"passed": 10, "failed": 0}
        }));

        let result = validate_execution_receipt_for_tests(&receipt);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("all_tests_passed"));
    }

    // ===== Signature Envelope Parsing Tests =====

    #[test]
    fn test_parse_signature_envelope_valid() {
        let envelope_json = json!({
            "version": "1.0",
            "type": "signature-envelope",
            "algo": "ed25519",
            "signer": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
            "hash": {"algo": "sha-256", "value": "abc123def456"},
            "artifact": {"name": "test.txt", "size": 100},
            "signature": "YmFzZTY0c2ln",
            "timestamp": "2026-01-01T00:00:00Z"
        });

        let result = parse_signature_envelope(&envelope_json);
        assert!(result.is_ok());
        let envelope = result.unwrap();
        assert_eq!(envelope.version, "1.0");
        assert_eq!(envelope.envelope_type, "signature-envelope");
        assert_eq!(envelope.hash.value, "abc123def456");
    }

    #[test]
    fn test_parse_signature_envelope_invalid_structure() {
        let envelope_json = json!({
            "version": "1.0"
            // Missing required fields
        });

        let result = parse_signature_envelope(&envelope_json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid signature envelope format"));
    }

    // ===== Envelope Signature Verification Tests =====

    #[test]
    fn test_verify_envelope_invalid_version() {
        let envelope = SignatureEnvelopeV1 {
            version: "2.0".to_string(),
            envelope_type: "signature-envelope".to_string(),
            algo: "ed25519".to_string(),
            signer: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            hash: openclaw_crypto::HashRef {
                algo: "sha-256".to_string(),
                value: "abc123".to_string(),
            },
            artifact: openclaw_crypto::ArtifactInfo {
                name: "test.txt".to_string(),
                size: 100,
            },
            signature: "YmFzZTY0c2ln".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            metadata: None,
        };

        let result = verify_envelope_signature(&envelope);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported envelope version"));
    }

    #[test]
    fn test_verify_envelope_invalid_type() {
        let envelope = SignatureEnvelopeV1 {
            version: "1.0".to_string(),
            envelope_type: "invalid-type".to_string(),
            algo: "ed25519".to_string(),
            signer: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            hash: openclaw_crypto::HashRef {
                algo: "sha-256".to_string(),
                value: "abc123".to_string(),
            },
            artifact: openclaw_crypto::ArtifactInfo {
                name: "test.txt".to_string(),
                size: 100,
            },
            signature: "YmFzZTY0c2ln".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            metadata: None,
        };

        let result = verify_envelope_signature(&envelope);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid envelope type"));
    }

    #[test]
    fn test_verify_envelope_invalid_algo() {
        let envelope = SignatureEnvelopeV1 {
            version: "1.0".to_string(),
            envelope_type: "signature-envelope".to_string(),
            algo: "rsa".to_string(),
            signer: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            hash: openclaw_crypto::HashRef {
                algo: "sha-256".to_string(),
                value: "abc123".to_string(),
            },
            artifact: openclaw_crypto::ArtifactInfo {
                name: "test.txt".to_string(),
                size: 100,
            },
            signature: "YmFzZTY0c2ln".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            metadata: None,
        };

        let result = verify_envelope_signature(&envelope);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported signature algorithm"));
    }

    #[test]
    fn test_verify_envelope_invalid_hash_algo() {
        let envelope = SignatureEnvelopeV1 {
            version: "1.0".to_string(),
            envelope_type: "signature-envelope".to_string(),
            algo: "ed25519".to_string(),
            signer: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            hash: openclaw_crypto::HashRef {
                algo: "sha-512".to_string(),
                value: "abc123".to_string(),
            },
            artifact: openclaw_crypto::ArtifactInfo {
                name: "test.txt".to_string(),
                size: 100,
            },
            signature: "YmFzZTY0c2ln".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            metadata: None,
        };

        let result = verify_envelope_signature(&envelope);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported hash algorithm"));
    }

    #[test]
    fn test_verify_envelope_invalid_did() {
        let envelope = SignatureEnvelopeV1 {
            version: "1.0".to_string(),
            envelope_type: "signature-envelope".to_string(),
            algo: "ed25519".to_string(),
            signer: "invalid-did-format".to_string(),
            hash: openclaw_crypto::HashRef {
                algo: "sha-256".to_string(),
                value: "abc123".to_string(),
            },
            artifact: openclaw_crypto::ArtifactInfo {
                name: "test.txt".to_string(),
                size: 100,
            },
            signature: "YmFzZTY0c2ln".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            metadata: None,
        };

        let result = verify_envelope_signature(&envelope);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid DID"));
    }

    #[test]
    fn test_verify_envelope_invalid_base64_signature() {
        let envelope = SignatureEnvelopeV1 {
            version: "1.0".to_string(),
            envelope_type: "signature-envelope".to_string(),
            algo: "ed25519".to_string(),
            signer: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            hash: openclaw_crypto::HashRef {
                algo: "sha-256".to_string(),
                value: "abc123".to_string(),
            },
            artifact: openclaw_crypto::ArtifactInfo {
                name: "test.txt".to_string(),
                size: 100,
            },
            signature: "not-valid-base64!!!".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            metadata: None,
        };

        let result = verify_envelope_signature(&envelope);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid base64 signature"));
    }

    #[test]
    fn test_verify_envelope_wrong_signature_length() {
        let envelope = SignatureEnvelopeV1 {
            version: "1.0".to_string(),
            envelope_type: "signature-envelope".to_string(),
            algo: "ed25519".to_string(),
            signer: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            hash: openclaw_crypto::HashRef {
                algo: "sha-256".to_string(),
                value: "abc123".to_string(),
            },
            artifact: openclaw_crypto::ArtifactInfo {
                name: "test.txt".to_string(),
                size: 100,
            },
            signature: "YWJj".to_string(), // Only 3 bytes when decoded
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            metadata: None,
        };

        let result = verify_envelope_signature(&envelope);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid signature length"));
    }

    // ===== Test-Based Auto-Approval Tests =====

    fn create_test_bounty(eval_harness_hash: &str) -> Bounty {
        let now = chrono::Utc::now();
        Bounty {
            id: Uuid::new_v4(),
            poster_did: "did:key:z6MkPoster".to_string(),
            title: "Test Bounty".to_string(),
            description: "A test-based bounty".to_string(),
            reward_credits: BigDecimal::from_str("100.00000000").unwrap(),
            closure_type: BountyClosureType::Tests,
            status: BountyStatus::Open,
            metadata: json!({"eval_harness_hash": eval_harness_hash}),
            created_at: now,
            updated_at: now,
            deadline: None,
        }
    }

    #[test]
    fn test_verify_test_submission_approved() {
        let bounty = create_test_bounty("sha256:abc123");
        let execution_receipt = json!({
            "harness_hash": "sha256:abc123",
            "all_tests_passed": true,
            "test_results": {"passed": 10, "failed": 0}
        });

        let result = verify_test_submission(&bounty, &execution_receipt);
        assert_eq!(result, TestVerificationResult::Approved);
    }

    #[test]
    fn test_verify_test_submission_approved_camel_case() {
        let bounty = create_test_bounty("sha256:abc123");
        let execution_receipt = json!({
            "harnessHash": "sha256:abc123",
            "allTestsPassed": true,
            "testResults": {"passed": 10, "failed": 0}
        });

        let result = verify_test_submission(&bounty, &execution_receipt);
        assert_eq!(result, TestVerificationResult::Approved);
    }

    #[test]
    fn test_verify_test_submission_harness_hash_mismatch() {
        let bounty = create_test_bounty("sha256:expected");
        let execution_receipt = json!({
            "harness_hash": "sha256:different",
            "all_tests_passed": true,
            "test_results": {"passed": 10, "failed": 0}
        });

        let result = verify_test_submission(&bounty, &execution_receipt);
        assert_eq!(
            result,
            TestVerificationResult::HarnessHashMismatch {
                expected: "sha256:expected".to_string(),
                actual: "sha256:different".to_string(),
            }
        );
    }

    #[test]
    fn test_verify_test_submission_tests_failed() {
        let bounty = create_test_bounty("sha256:abc123");
        let execution_receipt = json!({
            "harness_hash": "sha256:abc123",
            "all_tests_passed": false,
            "test_results": {"passed": 8, "failed": 2}
        });

        let result = verify_test_submission(&bounty, &execution_receipt);
        assert_eq!(result, TestVerificationResult::TestsFailed);
    }

    #[test]
    fn test_verify_test_submission_missing_all_tests_passed() {
        let bounty = create_test_bounty("sha256:abc123");
        let execution_receipt = json!({
            "harness_hash": "sha256:abc123",
            "test_results": {"passed": 10, "failed": 0}
        });

        let result = verify_test_submission(&bounty, &execution_receipt);
        assert_eq!(result, TestVerificationResult::TestsFailed);
    }

    #[test]
    fn test_verify_test_submission_missing_harness_hash() {
        let bounty = create_test_bounty("sha256:expected");
        let execution_receipt = json!({
            "all_tests_passed": true,
            "test_results": {"passed": 10, "failed": 0}
        });

        let result = verify_test_submission(&bounty, &execution_receipt);
        match result {
            TestVerificationResult::HarnessHashMismatch { expected, actual } => {
                assert_eq!(expected, "sha256:expected");
                assert_eq!(actual, "");
            }
            _ => panic!("Expected HarnessHashMismatch"),
        }
    }

    #[test]
    fn test_get_receipt_harness_hash_snake_case() {
        let receipt = json!({
            "harness_hash": "sha256:test123"
        });
        assert_eq!(get_receipt_harness_hash(&receipt), Some("sha256:test123"));
    }

    #[test]
    fn test_get_receipt_harness_hash_camel_case() {
        let receipt = json!({
            "harnessHash": "sha256:test456"
        });
        assert_eq!(get_receipt_harness_hash(&receipt), Some("sha256:test456"));
    }

    #[test]
    fn test_get_receipt_harness_hash_missing() {
        let receipt = json!({
            "other_field": "value"
        });
        assert_eq!(get_receipt_harness_hash(&receipt), None);
    }

    #[test]
    fn test_get_receipt_all_tests_passed_snake_case() {
        let receipt = json!({
            "all_tests_passed": true
        });
        assert_eq!(get_receipt_all_tests_passed(&receipt), Some(true));
    }

    #[test]
    fn test_get_receipt_all_tests_passed_camel_case() {
        let receipt = json!({
            "allTestsPassed": false
        });
        assert_eq!(get_receipt_all_tests_passed(&receipt), Some(false));
    }

    #[test]
    fn test_get_receipt_all_tests_passed_missing() {
        let receipt = json!({
            "other_field": "value"
        });
        assert_eq!(get_receipt_all_tests_passed(&receipt), None);
    }

    #[test]
    fn test_verification_result_equality() {
        assert_eq!(TestVerificationResult::Approved, TestVerificationResult::Approved);
        assert_eq!(TestVerificationResult::TestsFailed, TestVerificationResult::TestsFailed);
        assert_eq!(
            TestVerificationResult::HarnessHashMismatch {
                expected: "a".to_string(),
                actual: "b".to_string()
            },
            TestVerificationResult::HarnessHashMismatch {
                expected: "a".to_string(),
                actual: "b".to_string()
            }
        );
        assert_ne!(TestVerificationResult::Approved, TestVerificationResult::TestsFailed);
    }
}
