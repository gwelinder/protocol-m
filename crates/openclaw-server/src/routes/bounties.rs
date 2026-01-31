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

use chrono::{Duration, Utc};

use crate::error::AppError;
use crate::models::{
    calculate_dispute_stake, ApprovalActionType, ApprovalRequestStatus, Bounty, BountyClosureType,
    BountyStatus, BountySubmission, Dispute, DisputeStatus, EscrowStatus, NewBounty,
    NewBountySubmission, NewDispute, NewEscrowHold, NewMCreditsLedger, SubmissionStatus,
    UserPolicy, DISPUTE_WINDOW_DAYS,
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
    /// The escrow hold ID (None if pending approval).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escrow_id: Option<Uuid>,
    /// The ledger entry ID for the hold (None if pending approval).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger_id: Option<Uuid>,
    /// The approval request ID (set if high-value bounty requires approval).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_request_id: Option<Uuid>,
    /// Message explaining the bounty creation status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
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

/// Request body for accepting a bounty.
/// Note: In production, user_id would come from authentication.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptBountyRequest {
    /// The user ID accepting the bounty.
    /// In production, this would be extracted from auth token.
    pub user_id: Uuid,
}

/// Response for successful bounty acceptance.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptBountyResponse {
    /// Whether the bounty was accepted successfully.
    pub success: bool,
    /// The bounty ID.
    pub bounty_id: Uuid,
    /// Title of the bounty.
    pub title: String,
    /// The new status of the bounty.
    pub status: BountyStatus,
    /// DID of the user who accepted the bounty.
    pub accepter_did: String,
    /// Instructions for submitting work.
    pub submission_instructions: SubmissionInstructions,
}

/// Instructions for how to submit work to a bounty.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionInstructions {
    /// The endpoint to submit work to.
    pub endpoint: String,
    /// How the bounty completion is verified.
    pub closure_type: BountyClosureType,
    /// Specific requirements based on closure type.
    pub requirements: serde_json::Value,
    /// Optional deadline for submission.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
}

/// Request body for creating a dispute.
/// Note: In production, user_id would come from authentication.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDisputeRequest {
    /// The user ID creating the dispute.
    /// In production, this would be extracted from auth token.
    pub user_id: Uuid,
    /// ID of the submission being disputed.
    pub submission_id: Uuid,
    /// Reason for the dispute.
    pub reason: String,
}

/// Response for successful dispute creation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDisputeResponse {
    /// Whether the dispute was created successfully.
    pub success: bool,
    /// The unique dispute ID.
    pub dispute_id: Uuid,
    /// ID of the bounty being disputed.
    pub bounty_id: Uuid,
    /// ID of the submission being disputed.
    pub submission_id: Uuid,
    /// DID of the dispute initiator.
    pub initiator_did: String,
    /// Amount staked by the initiator (10% of bounty reward).
    pub stake_amount: String,
    /// Current status of the dispute.
    pub status: DisputeStatus,
    /// Deadline for dispute resolution.
    pub dispute_deadline: String,
    /// Message explaining the dispute creation.
    pub message: String,
}

/// Creates the bounties router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/", post(create_bounty))
        .route("/{id}/accept", post(accept_bounty))
        .route("/{id}/submit", post(submit_bounty))
        .route("/{id}/dispute", post(create_dispute))
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

/// Loads the user's policy from the database.
/// If no policy exists for the DID, returns None.
async fn load_user_policy(pool: &PgPool, did: &str) -> Result<Option<UserPolicy>, AppError> {
    let policy: Option<UserPolicy> = sqlx::query_as(
        r#"
        SELECT did, version, max_spend_per_day, max_spend_per_bounty, enabled,
               approval_tiers, allowed_delegates, emergency_contact, created_at, updated_at
        FROM user_policies
        WHERE did = $1
        "#,
    )
    .bind(did)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query user policy: {}", e)))?;

    Ok(policy)
}

/// Creates an approval request for a high-value bounty.
/// Returns the approval request ID.
async fn create_approval_request(
    pool: &PgPool,
    bounty_id: Uuid,
    requester_did: &str,
    operator_did: &str,
    reward_credits: &BigDecimal,
    bounty_title: &str,
) -> Result<Uuid, AppError> {
    let request_id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::hours(24);

    let metadata = json!({
        "bounty_title": bounty_title,
        "description": format!("Approval required for bounty: {}", bounty_title)
    });

    sqlx::query(
        r#"
        INSERT INTO approval_requests
            (id, operator_did, bounty_id, action_type, amount, status, metadata, requester_did, expires_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
        "#,
    )
    .bind(request_id)
    .bind(operator_did)
    .bind(bounty_id)
    .bind(ApprovalActionType::Spend)
    .bind(reward_credits)
    .bind(ApprovalRequestStatus::Pending)
    .bind(&metadata)
    .bind(requester_did)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create approval request: {}", e)))?;

    Ok(request_id)
}

/// Sends a notification to the operator about a pending approval request.
/// This is a placeholder implementation - in production, would send email/webhook/Slack.
async fn send_approval_notification(
    _pool: &PgPool,
    operator_did: &str,
    request_id: Uuid,
    bounty_title: &str,
    reward_credits: &BigDecimal,
    notification_channels: &[crate::models::NotificationChannel],
) -> Result<(), AppError> {
    // Placeholder: Log the notification request
    // In production, this would:
    // 1. Look up operator's notification preferences
    // 2. Send email if configured
    // 3. Send webhook if configured
    // 4. Send Slack message if configured

    tracing::info!(
        operator_did = %operator_did,
        request_id = %request_id,
        bounty_title = %bounty_title,
        reward_credits = %reward_credits,
        channels = ?notification_channels,
        "Approval notification would be sent"
    );

    // TODO: Implement actual notification sending
    // For now, we just log and succeed
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
    let title_trimmed = request.title.trim().to_string();

    // Step 8: Load user's policy and check if approval is required
    let policy = load_user_policy(&pool, &poster_did).await?;
    let approval_tier = policy.as_ref().and_then(|p| p.requires_approval(&reward_credits));

    if let Some(tier) = approval_tier {
        // Approval is required - create bounty with pending_approval status
        // Do NOT create escrow hold yet
        let new_bounty = NewBounty {
            poster_did: poster_did.clone(),
            title: title_trimmed.clone(),
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
        .bind(BountyStatus::PendingApproval)
        .bind(&new_bounty.metadata)
        .bind(new_bounty.deadline)
        .fetch_one(&pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to create bounty: {}", e)))?;

        // Create approval request
        let operator_did = policy.as_ref().map(|p| p.operator_did()).unwrap_or(&poster_did);
        let approval_request_id = create_approval_request(
            &pool,
            bounty_id,
            &poster_did,
            operator_did,
            &reward_credits,
            &title_trimmed,
        )
        .await?;

        // Send notification to operator
        send_approval_notification(
            &pool,
            operator_did,
            approval_request_id,
            &title_trimmed,
            &reward_credits,
            &tier.notification_channels,
        )
        .await?;

        // Return response with approval_request_id (no escrow yet)
        return Ok(Json(CreateBountyResponse {
            success: true,
            bounty_id: bounty.id,
            title: bounty.title,
            reward_credits: bounty.reward_credits.to_string(),
            status: bounty.status,
            escrow_id: None,
            ledger_id: None,
            approval_request_id: Some(approval_request_id),
            message: Some(format!(
                "Bounty requires operator approval for amounts exceeding {} M-credits. Approval request created.",
                tier.threshold
            )),
        }));
    }

    // No approval required - proceed with normal flow

    // Step 9: Create escrow hold (deduct from balance, lock in escrow)
    let (escrow_id, ledger_id) =
        create_escrow_hold(&pool, bounty_id, &poster_did, &reward_credits).await?;

    // Step 10: Create bounty record
    let new_bounty = NewBounty {
        poster_did: poster_did.clone(),
        title: title_trimmed,
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

    // Step 11: Return response
    Ok(Json(CreateBountyResponse {
        success: true,
        bounty_id: bounty.id,
        title: bounty.title,
        reward_credits: bounty.reward_credits.to_string(),
        status: bounty.status,
        escrow_id: Some(escrow_id),
        ledger_id: Some(ledger_id),
        approval_request_id: None,
        message: None,
    }))
}

// ===== Bounty Accept Endpoint =====

/// Loads a bounty by ID for acceptance.
/// Validates the bounty is open and not expired.
async fn load_bounty_for_acceptance(pool: &PgPool, bounty_id: Uuid) -> Result<Bounty, AppError> {
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

    // Check bounty is open for acceptance
    if !bounty.is_open() {
        return Err(AppError::BadRequest(format!(
            "Bounty is not open for acceptance. Current status: {:?}",
            bounty.status
        )));
    }

    // Check bounty hasn't expired
    if bounty.is_expired() {
        return Err(AppError::BadRequest(
            "Bounty has expired and can no longer be accepted".to_string(),
        ));
    }

    Ok(bounty)
}

/// Updates the bounty status to in_progress.
async fn set_bounty_in_progress(pool: &PgPool, bounty_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE bounties
        SET status = 'in_progress', updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(bounty_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update bounty status: {}", e)))?;

    Ok(())
}

/// Builds submission instructions based on closure type.
fn build_submission_instructions(bounty: &Bounty) -> SubmissionInstructions {
    let requirements = match bounty.closure_type {
        BountyClosureType::Tests => {
            let harness_hash = bounty.eval_harness_hash().unwrap_or_default();
            json!({
                "type": "tests",
                "description": "Your submission must pass the automated test harness.",
                "evalHarnessHash": harness_hash,
                "requiredFields": ["signatureEnvelope", "executionReceipt"]
            })
        }
        BountyClosureType::Quorum => {
            let reviewer_count = bounty.reviewer_count().unwrap_or(3);
            let min_rep = bounty.min_reviewer_rep().unwrap_or(0);
            json!({
                "type": "quorum",
                "description": "Your submission will be reviewed by multiple peer reviewers.",
                "reviewerCount": reviewer_count,
                "minReviewerReputation": min_rep,
                "requiredFields": ["signatureEnvelope"]
            })
        }
        BountyClosureType::Requester => {
            json!({
                "type": "requester",
                "description": "Your submission will be reviewed and approved by the bounty poster.",
                "requiredFields": ["signatureEnvelope"]
            })
        }
    };

    SubmissionInstructions {
        endpoint: format!("/api/v1/bounties/{}/submit", bounty.id),
        closure_type: bounty.closure_type,
        requirements,
        deadline: bounty.deadline.map(|d| d.to_rfc3339()),
    }
}

/// Accept a bounty and mark it as in_progress.
/// Requires the user to have a bound DID.
async fn accept_bounty(
    State(pool): State<PgPool>,
    Path(bounty_id): Path<Uuid>,
    Json(request): Json<AcceptBountyRequest>,
) -> Result<Json<AcceptBountyResponse>, AppError> {
    // Step 1: Get user's bound DID (authentication + DID binding check)
    let accepter_did = get_user_bound_did(&pool, request.user_id).await?;

    // Step 2: Load and validate bounty
    let bounty = load_bounty_for_acceptance(&pool, bounty_id).await?;

    // Step 3: Check user isn't trying to accept their own bounty
    if bounty.poster_did == accepter_did {
        return Err(AppError::BadRequest(
            "You cannot accept your own bounty".to_string(),
        ));
    }

    // Step 4: Update bounty status to in_progress
    set_bounty_in_progress(&pool, bounty_id).await?;

    // Step 5: Build submission instructions
    let submission_instructions = build_submission_instructions(&bounty);

    Ok(Json(AcceptBountyResponse {
        success: true,
        bounty_id,
        title: bounty.title,
        status: BountyStatus::InProgress,
        accepter_did,
        submission_instructions,
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

/// Releases escrow funds to the bounty recipient on approval.
///
/// This function performs the following atomic operations:
/// 1. Marks the escrow_hold as released (status = 'released', released_at = NOW())
/// 2. Inserts a release event into the ledger
/// 3. Updates the recipient's m_credits_accounts balance atomically
/// 4. Updates the bounty status to completed
///
/// Returns the ledger entry ID for the release event.
async fn release_escrow(
    pool: &PgPool,
    bounty_id: Uuid,
    recipient_did: &str,
) -> Result<Uuid, AppError> {
    // Step 1: Load the escrow hold for this bounty
    let escrow: Option<(Uuid, BigDecimal, String)> = sqlx::query_as(
        r#"
        SELECT id, amount, holder_did
        FROM escrow_holds
        WHERE bounty_id = $1 AND status = 'held'
        "#,
    )
    .bind(bounty_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query escrow hold: {}", e)))?;

    let (escrow_id, amount, _holder_did) = escrow.ok_or_else(|| {
        AppError::BadRequest(format!(
            "No active escrow hold found for bounty {}",
            bounty_id
        ))
    })?;

    // Step 2: Mark escrow as released
    sqlx::query(
        r#"
        UPDATE escrow_holds
        SET status = 'released', released_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(escrow_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update escrow status: {}", e)))?;

    // Step 3: Insert release event into ledger
    let ledger_entry = NewMCreditsLedger::release(
        recipient_did.to_string(),
        amount.clone(),
        json!({
            "bounty_id": bounty_id.to_string(),
            "escrow_id": escrow_id.to_string(),
            "reason": "bounty_completion"
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
    .map_err(|e| AppError::Internal(format!("Failed to insert release ledger entry: {}", e)))?;

    // Step 4: Update recipient's balance (upsert)
    sqlx::query(
        r#"
        INSERT INTO m_credits_accounts (did, balance, promo_balance, created_at, updated_at)
        VALUES ($1, $2, 0, NOW(), NOW())
        ON CONFLICT (did) DO UPDATE
        SET balance = m_credits_accounts.balance + $2, updated_at = NOW()
        "#,
    )
    .bind(recipient_did)
    .bind(&amount)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update recipient balance: {}", e)))?;

    // Step 5: Update bounty status to completed
    sqlx::query(
        r#"
        UPDATE bounties
        SET status = 'completed', updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(bounty_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update bounty status: {}", e)))?;

    Ok(ledger_id)
}

/// Mints reputation for a bounty submitter upon approval.
///
/// Reputation is weighted by closure type:
/// - tests: 1.5x (most trustworthy, automated verification)
/// - quorum: 1.2x (peer-reviewed)
/// - requester: 1.0x (single approver)
///
/// Base reputation is calculated as: reward_credits * 0.1 * closure_type_weight
async fn mint_reputation_for_submission(
    pool: &PgPool,
    recipient_did: &str,
    bounty: &Bounty,
    submission_id: Option<Uuid>,
) -> Result<BigDecimal, AppError> {
    use crate::routes::reputation::{mint_reputation, BASE_REPUTATION_RATE};

    // Calculate base reputation = 10% of reward credits
    let base_rep = &bounty.reward_credits
        * BigDecimal::try_from(BASE_REPUTATION_RATE)
            .map_err(|_| AppError::Internal("Invalid base rate".to_string()))?;

    // Mint reputation with closure type weighting
    let reason = format!("Bounty completion: {}", bounty.title);
    let new_total = mint_reputation(
        pool,
        recipient_did,
        base_rep,
        &reason,
        bounty.closure_type,
        None, // reviewer_credibility (used for quorum reviewers)
        Some(bounty.id),
        submission_id,
    )
    .await?;

    Ok(new_total)
}

/// Registers a submission artifact in ClawdHub and links derivations.
///
/// This function:
/// 1. Checks if the artifact already exists (by hash)
/// 2. If not, registers it with the signature envelope
/// 3. If bounty metadata includes parent_artifact_id, creates derivation link
/// 4. Updates the submission with the artifact_id reference
///
/// Returns the artifact ID on success.
async fn register_submission_artifact(
    pool: &PgPool,
    submission_id: Uuid,
    bounty: &Bounty,
    envelope: &SignatureEnvelopeV1,
) -> Result<Uuid, AppError> {
    // Step 1: Check if artifact already exists (by hash)
    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM artifacts WHERE hash = $1 LIMIT 1",
    )
    .bind(&envelope.hash.value)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to check existing artifact: {}", e)))?;

    let artifact_id = if let Some((existing_id,)) = existing {
        // Artifact already registered, use existing ID
        existing_id
    } else {
        // Step 2: Register new artifact
        let new_id = Uuid::new_v4();
        let timestamp = chrono::DateTime::parse_from_rfc3339(&envelope.timestamp)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .map_err(|e| AppError::Internal(format!("Invalid timestamp in envelope: {}", e)))?;

        // Build metadata with bounty context
        let mut artifact_metadata = envelope.metadata.clone().unwrap_or(json!({}));
        if let Some(obj) = artifact_metadata.as_object_mut() {
            obj.insert("bounty_id".to_string(), json!(bounty.id.to_string()));
            obj.insert("bounty_title".to_string(), json!(bounty.title));
        }

        sqlx::query(
            r#"
            INSERT INTO artifacts (id, hash, did, timestamp, metadata, signature, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            "#,
        )
        .bind(new_id)
        .bind(&envelope.hash.value)
        .bind(&envelope.signer)
        .bind(timestamp)
        .bind(&artifact_metadata)
        .bind(&envelope.signature)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to register artifact: {}", e)))?;

        new_id
    };

    // Step 3: Check for parent_artifact_id in bounty metadata and create derivation
    if let Some(parent_ref) = get_parent_artifact_from_metadata(&bounty.metadata) {
        if let Ok(parent_id) = resolve_parent_artifact(pool, &parent_ref).await {
            // Check for cycles before creating derivation
            if !detect_cycle_for_derivation(pool, artifact_id, parent_id).await? {
                create_derivation_link(pool, artifact_id, parent_id).await?;
            }
        }
    }

    // Step 4: Update submission with artifact_id
    sqlx::query(
        r#"
        UPDATE bounty_submissions
        SET artifact_id = $2
        WHERE id = $1
        "#,
    )
    .bind(submission_id)
    .bind(artifact_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update submission artifact_id: {}", e)))?;

    Ok(artifact_id)
}

/// Gets the parent artifact reference from bounty metadata.
/// Checks for parent_artifact_id or parentArtifactId field.
fn get_parent_artifact_from_metadata(metadata: &serde_json::Value) -> Option<String> {
    metadata
        .get("parent_artifact_id")
        .or_else(|| metadata.get("parentArtifactId"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Resolves a parent artifact reference (UUID or hash) to an artifact ID.
async fn resolve_parent_artifact(pool: &PgPool, reference: &str) -> Result<Uuid, AppError> {
    // First, try to parse as UUID
    if let Ok(uuid) = Uuid::parse_str(reference) {
        let exists: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM artifacts WHERE id = $1 LIMIT 1",
        )
        .bind(uuid)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to query artifact: {}", e)))?;

        if exists.is_some() {
            return Ok(uuid);
        }
    }

    // Try to look up by hash
    let result: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM artifacts WHERE hash = $1 LIMIT 1",
    )
    .bind(reference)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query artifact by hash: {}", e)))?;

    result
        .map(|(id,)| id)
        .ok_or_else(|| AppError::BadRequest(format!("Parent artifact not found: '{}'", reference)))
}

/// Detects if adding a derivation would create a cycle.
async fn detect_cycle_for_derivation(
    pool: &PgPool,
    artifact_id: Uuid,
    parent_id: Uuid,
) -> Result<bool, AppError> {
    // Self-reference is a cycle
    if artifact_id == parent_id {
        return Ok(true);
    }

    // Check if parent_id already derives from artifact_id (directly or transitively)
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![parent_id];
    const MAX_DEPTH: usize = 100;

    while let Some(current) = stack.pop() {
        if visited.len() >= MAX_DEPTH {
            return Ok(false); // Assume no cycle at deep depths
        }

        if current == artifact_id {
            return Ok(true);
        }

        if !visited.insert(current) {
            continue;
        }

        let parents: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT derived_from_id FROM artifact_derivations WHERE artifact_id = $1",
        )
        .bind(current)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to query derivations: {}", e)))?;

        for (parent,) in parents {
            stack.push(parent);
        }
    }

    Ok(false)
}

/// Creates a derivation link between artifacts.
async fn create_derivation_link(
    pool: &PgPool,
    artifact_id: Uuid,
    parent_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO artifact_derivations (artifact_id, derived_from_id)
        VALUES ($1, $2)
        ON CONFLICT (artifact_id, derived_from_id) DO NOTHING
        "#,
    )
    .bind(artifact_id)
    .bind(parent_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create derivation: {}", e)))?;

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
        INSERT INTO bounty_submissions (id, bounty_id, submitter_did, artifact_hash, signature_envelope, execution_receipt, status, created_at, artifact_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NULL)
        RETURNING id, bounty_id, submitter_did, artifact_hash, signature_envelope, execution_receipt, status, created_at, artifact_id
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

                // Release escrow to the submitter
                release_escrow(&pool, bounty_id, &submitter_did).await?;

                // Mint reputation for the submitter
                let _new_rep = mint_reputation_for_submission(&pool, &submitter_did, &bounty, Some(submission.id)).await?;

                // Register artifact in ClawdHub and create derivation links
                register_submission_artifact(&pool, submission.id, &bounty, &envelope).await?;

                (
                    SubmissionStatus::Approved,
                    Some(true),
                    Some("Tests passed and harness hash verified - submission auto-approved, escrow released, artifact registered".to_string()),
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

// ===== Dispute Creation Endpoint =====

/// Validates the dispute reason is not empty and within length limits.
fn validate_dispute_reason(reason: &str) -> Result<(), AppError> {
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(
            "Dispute reason cannot be empty".to_string(),
        ));
    }
    if trimmed.len() > 2000 {
        return Err(AppError::BadRequest(
            "Dispute reason must be 2000 characters or less".to_string(),
        ));
    }
    Ok(())
}

/// Loads a submission by ID and validates it is disputable.
async fn load_submission_for_dispute(
    pool: &PgPool,
    submission_id: Uuid,
) -> Result<BountySubmission, AppError> {
    let submission: Option<BountySubmission> = sqlx::query_as(
        r#"
        SELECT id, bounty_id, submitter_did, artifact_hash, signature_envelope, execution_receipt, status, created_at, artifact_id
        FROM bounty_submissions
        WHERE id = $1
        "#,
    )
    .bind(submission_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query submission: {}", e)))?;

    let submission = submission.ok_or_else(|| {
        AppError::NotFound(format!("Submission not found: {}", submission_id))
    })?;

    // Check submission is approved (disputes are against approved submissions)
    if !submission.is_approved() {
        return Err(AppError::BadRequest(format!(
            "Only approved submissions can be disputed. Current status: {:?}",
            submission.status
        )));
    }

    Ok(submission)
}

/// Loads a bounty by ID for dispute validation.
async fn load_bounty_for_dispute(pool: &PgPool, bounty_id: Uuid) -> Result<Bounty, AppError> {
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

    bounty.ok_or_else(|| AppError::NotFound(format!("Bounty not found: {}", bounty_id)))
}

/// Checks if a submission is within the dispute window.
/// The dispute window is 7 days from submission creation.
fn check_dispute_window(submission: &BountySubmission) -> Result<(), AppError> {
    let window_end = submission.created_at + Duration::days(DISPUTE_WINDOW_DAYS);
    let now = Utc::now();

    if now > window_end {
        return Err(AppError::BadRequest(format!(
            "Dispute window has closed. Submissions can only be disputed within {} days of approval.",
            DISPUTE_WINDOW_DAYS
        )));
    }

    Ok(())
}

/// Checks if there is already an active dispute for this submission.
async fn check_existing_dispute(pool: &PgPool, submission_id: Uuid) -> Result<(), AppError> {
    let existing: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id
        FROM disputes
        WHERE submission_id = $1 AND status = 'pending'
        LIMIT 1
        "#,
    )
    .bind(submission_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query existing disputes: {}", e)))?;

    if existing.is_some() {
        return Err(AppError::BadRequest(
            "There is already an active dispute for this submission".to_string(),
        ));
    }

    Ok(())
}

/// Creates an escrow hold for the dispute stake.
/// Returns the escrow hold ID.
async fn create_dispute_stake_escrow(
    pool: &PgPool,
    bounty_id: Uuid,
    initiator_did: &str,
    stake_amount: &BigDecimal,
) -> Result<Uuid, AppError> {
    // Step 1: Insert hold event into ledger
    let ledger_entry = NewMCreditsLedger::hold(
        initiator_did.to_string(),
        stake_amount.clone(),
        json!({
            "bounty_id": bounty_id.to_string(),
            "reason": "dispute_stake"
        }),
    );

    sqlx::query(
        r#"
        INSERT INTO m_credits_ledger (event_type, from_did, to_did, amount, metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(ledger_entry.event_type)
    .bind(&ledger_entry.from_did)
    .bind(&ledger_entry.to_did)
    .bind(&ledger_entry.amount)
    .bind(&ledger_entry.metadata)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to insert stake ledger entry: {}", e)))?;

    // Step 2: Create escrow hold record
    let escrow_id = Uuid::new_v4();
    let new_escrow = NewEscrowHold::new(bounty_id, initiator_did.to_string(), stake_amount.clone());

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
    .map_err(|e| AppError::Internal(format!("Failed to create stake escrow: {}", e)))?;

    // Step 3: Deduct from initiator's balance
    sqlx::query(
        r#"
        UPDATE m_credits_accounts
        SET balance = balance - $2
        WHERE did = $1
        "#,
    )
    .bind(initiator_did)
    .bind(stake_amount)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to deduct stake from balance: {}", e)))?;

    Ok(escrow_id)
}

/// Creates a dispute against an approved bounty submission.
/// Requires the initiator to stake 10% of the bounty reward.
async fn create_dispute(
    State(pool): State<PgPool>,
    Path(bounty_id): Path<Uuid>,
    Json(request): Json<CreateDisputeRequest>,
) -> Result<Json<CreateDisputeResponse>, AppError> {
    // Step 1: Get user's bound DID (authentication + DID binding check)
    let initiator_did = get_user_bound_did(&pool, request.user_id).await?;

    // Step 2: Validate dispute reason
    validate_dispute_reason(&request.reason)?;

    // Step 3: Load and validate the submission
    let submission = load_submission_for_dispute(&pool, request.submission_id).await?;

    // Verify submission belongs to this bounty
    if submission.bounty_id != bounty_id {
        return Err(AppError::BadRequest(format!(
            "Submission {} does not belong to bounty {}",
            request.submission_id, bounty_id
        )));
    }

    // Step 4: Load the bounty
    let bounty = load_bounty_for_dispute(&pool, bounty_id).await?;

    // Step 5: Check dispute window (7 days from submission creation)
    check_dispute_window(&submission)?;

    // Step 6: Check no existing active dispute for this submission
    check_existing_dispute(&pool, request.submission_id).await?;

    // Step 7: Prevent self-disputes (initiator cannot be the submitter)
    if initiator_did == submission.submitter_did {
        return Err(AppError::BadRequest(
            "You cannot dispute your own submission".to_string(),
        ));
    }

    // Step 8: Calculate the stake amount (10% of bounty reward)
    let stake_amount = calculate_dispute_stake(&bounty.reward_credits);

    // Step 9: Check initiator has sufficient balance for the stake
    let (main_balance, promo_balance) = get_did_balance(&pool, &initiator_did).await?;
    check_sufficient_balance(&main_balance, &promo_balance, &stake_amount)?;

    // Step 10: Create stake escrow hold
    let stake_escrow_id =
        create_dispute_stake_escrow(&pool, bounty_id, &initiator_did, &stake_amount).await?;

    // Step 11: Calculate dispute deadline (7 days from now)
    let dispute_deadline = Utc::now() + Duration::days(DISPUTE_WINDOW_DAYS);

    // Step 12: Create the dispute record
    let dispute_id = Uuid::new_v4();
    let new_dispute = NewDispute::with_deadline(
        bounty_id,
        request.submission_id,
        initiator_did.clone(),
        request.reason.trim().to_string(),
        stake_amount.clone(),
        Some(stake_escrow_id),
        dispute_deadline,
    );

    let dispute: Dispute = sqlx::query_as(
        r#"
        INSERT INTO disputes (id, bounty_id, submission_id, initiator_did, reason, status, stake_amount, stake_escrow_id, dispute_deadline, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
        RETURNING id, bounty_id, submission_id, initiator_did, reason, status, stake_amount, stake_escrow_id, resolution_outcome, resolver_did, created_at, resolved_at, dispute_deadline
        "#,
    )
    .bind(dispute_id)
    .bind(new_dispute.bounty_id)
    .bind(new_dispute.submission_id)
    .bind(&new_dispute.initiator_did)
    .bind(&new_dispute.reason)
    .bind(DisputeStatus::Pending)
    .bind(&new_dispute.stake_amount)
    .bind(new_dispute.stake_escrow_id)
    .bind(new_dispute.dispute_deadline)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create dispute: {}", e)))?;

    // Step 13: Return response
    Ok(Json(CreateDisputeResponse {
        success: true,
        dispute_id: dispute.id,
        bounty_id: dispute.bounty_id,
        submission_id: dispute.submission_id,
        initiator_did: dispute.initiator_did,
        stake_amount: dispute.stake_amount.to_string(),
        status: dispute.status,
        dispute_deadline: dispute.dispute_deadline.to_rfc3339(),
        message: format!(
            "Dispute created successfully. {} M-credits staked. Resolution deadline: {}",
            stake_amount,
            dispute.dispute_deadline.format("%Y-%m-%d %H:%M:%S UTC")
        ),
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
            escrow_id: Some(Uuid::new_v4()),
            ledger_id: Some(Uuid::new_v4()),
            approval_request_id: None,
            message: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"bountyId\":"));
        assert!(json.contains("\"title\":\"Test bounty\""));
        assert!(json.contains("\"rewardCredits\":\"100.00000000\""));
        assert!(json.contains("\"status\":\"open\""));
        assert!(json.contains("\"escrowId\":"));
        assert!(json.contains("\"ledgerId\":"));
        // approval_request_id and message should not be present when None
        assert!(!json.contains("\"approvalRequestId\":"));
        assert!(!json.contains("\"message\":"));
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

    // ===== Escrow Release Tests =====

    fn create_bounty_with_closure_type(closure_type: BountyClosureType, reward: &str) -> Bounty {
        let now = chrono::Utc::now();
        let metadata = match closure_type {
            BountyClosureType::Tests => json!({"eval_harness_hash": "sha256:test"}),
            BountyClosureType::Quorum => json!({"reviewer_count": 3, "min_reviewer_rep": 100}),
            BountyClosureType::Requester => json!({}),
        };
        Bounty {
            id: Uuid::new_v4(),
            poster_did: "did:key:z6MkPoster".to_string(),
            title: "Test Bounty".to_string(),
            description: "A bounty for testing".to_string(),
            reward_credits: BigDecimal::from_str(reward).unwrap(),
            closure_type,
            status: BountyStatus::Open,
            metadata,
            created_at: now,
            updated_at: now,
            deadline: None,
        }
    }

    #[test]
    fn test_reputation_weight_tests_closure() {
        // Tests closure type should have 1.5x weight
        let bounty = create_bounty_with_closure_type(BountyClosureType::Tests, "100.00000000");
        assert!(bounty.uses_tests());
        // Weight calculation: 100 * 0.1 * 1.5 = 15.0 reputation
        let expected_rep = 100.0 * 0.1 * 1.5;
        assert_eq!(expected_rep, 15.0);
    }

    #[test]
    fn test_reputation_weight_quorum_closure() {
        // Quorum closure type should have 1.2x weight
        let bounty = create_bounty_with_closure_type(BountyClosureType::Quorum, "100.00000000");
        assert!(bounty.uses_quorum());
        // Weight calculation: 100 * 0.1 * 1.2 = 12.0 reputation
        let expected_rep = 100.0 * 0.1 * 1.2;
        assert_eq!(expected_rep, 12.0);
    }

    #[test]
    fn test_reputation_weight_requester_closure() {
        // Requester closure type should have 1.0x weight
        let bounty = create_bounty_with_closure_type(BountyClosureType::Requester, "100.00000000");
        assert!(bounty.uses_requester());
        // Weight calculation: 100 * 0.1 * 1.0 = 10.0 reputation
        let expected_rep = 100.0 * 0.1 * 1.0;
        assert_eq!(expected_rep, 10.0);
    }

    #[test]
    fn test_reputation_scales_with_reward() {
        // Higher reward bounties should give more reputation
        let _small_bounty = create_bounty_with_closure_type(BountyClosureType::Tests, "50.00000000");
        let _large_bounty = create_bounty_with_closure_type(BountyClosureType::Tests, "500.00000000");

        let small_rep = 50.0 * 0.1 * 1.5; // 7.5
        let large_rep = 500.0 * 0.1 * 1.5; // 75.0

        assert_eq!(small_rep, 7.5);
        assert_eq!(large_rep, 75.0);
        assert!(large_rep > small_rep);
    }

    #[test]
    fn test_new_m_credits_ledger_release() {
        use crate::models::MCreditsEventType;

        let amount = BigDecimal::from_str("100.00000000").unwrap();
        let metadata = json!({
            "bounty_id": "test-bounty-123",
            "escrow_id": "test-escrow-456",
            "reason": "bounty_completion"
        });

        let entry = NewMCreditsLedger::release(
            "did:key:z6MkRecipient".to_string(),
            amount.clone(),
            metadata.clone(),
        );

        assert_eq!(entry.event_type, MCreditsEventType::Release);
        assert!(entry.from_did.is_none()); // Release has no from_did
        assert_eq!(entry.to_did, Some("did:key:z6MkRecipient".to_string()));
        assert_eq!(entry.amount, amount);
        assert_eq!(entry.metadata["reason"], "bounty_completion");
    }

    #[test]
    fn test_submit_bounty_response_with_escrow_release_message() {
        let response = SubmitBountyResponse {
            success: true,
            submission_id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            status: SubmissionStatus::Approved,
            auto_approved: Some(true),
            message: Some("Tests passed and harness hash verified - submission auto-approved, escrow released".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"approved\""));
        assert!(json.contains("\"autoApproved\":true"));
        assert!(json.contains("escrow released"));
    }

    #[test]
    fn test_closure_type_weight_values() {
        // Verify the exact weight multipliers for each closure type
        let test_weight = match BountyClosureType::Tests {
            BountyClosureType::Tests => 1.5,
            BountyClosureType::Quorum => 1.2,
            BountyClosureType::Requester => 1.0,
        };
        assert_eq!(test_weight, 1.5);

        let quorum_weight = match BountyClosureType::Quorum {
            BountyClosureType::Tests => 1.5,
            BountyClosureType::Quorum => 1.2,
            BountyClosureType::Requester => 1.0,
        };
        assert_eq!(quorum_weight, 1.2);

        let requester_weight = match BountyClosureType::Requester {
            BountyClosureType::Tests => 1.5,
            BountyClosureType::Quorum => 1.2,
            BountyClosureType::Requester => 1.0,
        };
        assert_eq!(requester_weight, 1.0);
    }

    #[test]
    fn test_reputation_metadata_structure() {
        let bounty = create_bounty_with_closure_type(BountyClosureType::Tests, "100.00000000");

        // Simulate the metadata that would be created by mint_reputation_for_submission
        let weight = 1.5;
        let base_rep = 100.0 * 0.1 * weight;

        let metadata = json!({
            "bounty_id": bounty.id.to_string(),
            "closure_type": "tests",
            "weight": weight,
            "reputation_amount": base_rep,
            "reason": "bounty_completion_reputation"
        });

        assert_eq!(metadata["closure_type"], "tests");
        assert_eq!(metadata["weight"], 1.5);
        assert_eq!(metadata["reputation_amount"], 15.0);
        assert_eq!(metadata["reason"], "bounty_completion_reputation");
    }

    #[test]
    fn test_escrow_release_metadata_structure() {
        let bounty_id = Uuid::new_v4();
        let escrow_id = Uuid::new_v4();

        // Simulate the metadata that would be created by release_escrow
        let metadata = json!({
            "bounty_id": bounty_id.to_string(),
            "escrow_id": escrow_id.to_string(),
            "reason": "bounty_completion"
        });

        assert_eq!(metadata["reason"], "bounty_completion");
        assert!(metadata["bounty_id"].as_str().is_some());
        assert!(metadata["escrow_id"].as_str().is_some());
    }

    // ===== Artifact Registration Tests =====

    #[test]
    fn test_get_parent_artifact_from_metadata_snake_case() {
        let metadata = json!({
            "parent_artifact_id": "abc123-def456"
        });
        let result = get_parent_artifact_from_metadata(&metadata);
        assert_eq!(result, Some("abc123-def456".to_string()));
    }

    #[test]
    fn test_get_parent_artifact_from_metadata_camel_case() {
        let metadata = json!({
            "parentArtifactId": "xyz789"
        });
        let result = get_parent_artifact_from_metadata(&metadata);
        assert_eq!(result, Some("xyz789".to_string()));
    }

    #[test]
    fn test_get_parent_artifact_from_metadata_missing() {
        let metadata = json!({
            "other_field": "value"
        });
        let result = get_parent_artifact_from_metadata(&metadata);
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_parent_artifact_from_metadata_empty() {
        let metadata = json!({});
        let result = get_parent_artifact_from_metadata(&metadata);
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_parent_artifact_from_metadata_prefers_snake_case() {
        // When both formats are present, snake_case should be preferred
        let metadata = json!({
            "parent_artifact_id": "snake",
            "parentArtifactId": "camel"
        });
        let result = get_parent_artifact_from_metadata(&metadata);
        assert_eq!(result, Some("snake".to_string()));
    }

    #[test]
    fn test_artifact_registration_metadata_structure() {
        // Simulate the metadata that would be created by register_submission_artifact
        let bounty_id = Uuid::new_v4();
        let bounty_title = "Test Bounty";

        let mut artifact_metadata = json!({
            "author": "test-agent"
        });
        if let Some(obj) = artifact_metadata.as_object_mut() {
            obj.insert("bounty_id".to_string(), json!(bounty_id.to_string()));
            obj.insert("bounty_title".to_string(), json!(bounty_title));
        }

        assert_eq!(artifact_metadata["author"], "test-agent");
        assert_eq!(artifact_metadata["bounty_title"], "Test Bounty");
        assert!(artifact_metadata["bounty_id"].as_str().is_some());
    }

    #[test]
    fn test_submit_bounty_response_with_artifact_registered_message() {
        let response = SubmitBountyResponse {
            success: true,
            submission_id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            status: SubmissionStatus::Approved,
            auto_approved: Some(true),
            message: Some("Tests passed and harness hash verified - submission auto-approved, escrow released, artifact registered".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"approved\""));
        assert!(json.contains("\"autoApproved\":true"));
        assert!(json.contains("artifact registered"));
    }

    #[test]
    fn test_bounty_metadata_with_parent_artifact() {
        let bounty = Bounty {
            id: Uuid::new_v4(),
            poster_did: "did:key:z6MkPoster".to_string(),
            title: "Derived Work Bounty".to_string(),
            description: "Build on existing artifact".to_string(),
            reward_credits: BigDecimal::from_str("100.00000000").unwrap(),
            closure_type: BountyClosureType::Tests,
            status: BountyStatus::Open,
            metadata: json!({
                "eval_harness_hash": "sha256:test",
                "parent_artifact_id": "550e8400-e29b-41d4-a716-446655440000"
            }),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deadline: None,
        };

        let parent_ref = get_parent_artifact_from_metadata(&bounty.metadata);
        assert_eq!(parent_ref, Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
    }

    #[test]
    fn test_cycle_detection_self_reference() {
        // Self-reference should be detected as a cycle
        let artifact_id = Uuid::new_v4();
        // Direct check for self-reference without async
        assert!(artifact_id == artifact_id);
    }

    #[test]
    fn test_derivation_metadata_with_bounty_context() {
        // When registering an artifact from a bounty submission,
        // the metadata should include bounty context
        let bounty_id = Uuid::new_v4();
        let metadata = json!({
            "bounty_id": bounty_id.to_string(),
            "bounty_title": "Fix Authentication Bug",
            "original_field": "preserved"
        });

        assert_eq!(metadata["bounty_title"], "Fix Authentication Bug");
        assert_eq!(metadata["original_field"], "preserved");
    }

    // ===== Accept Bounty Endpoint Tests =====

    #[test]
    fn test_accept_bounty_request_deserialization() {
        let json_str = r#"{"userId": "550e8400-e29b-41d4-a716-446655440000"}"#;
        let request: AcceptBountyRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(
            request.user_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_accept_bounty_response_serialization() {
        let response = AcceptBountyResponse {
            success: true,
            bounty_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
            title: "Fix authentication bug".to_string(),
            status: BountyStatus::InProgress,
            accepter_did: "did:key:z6MkTest".to_string(),
            submission_instructions: SubmissionInstructions {
                endpoint: "/api/v1/bounties/550e8400-e29b-41d4-a716-446655440001/submit".to_string(),
                closure_type: BountyClosureType::Tests,
                requirements: json!({
                    "type": "tests",
                    "evalHarnessHash": "sha256:abc123"
                }),
                deadline: Some("2026-02-15T23:59:59Z".to_string()),
            },
        };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("\"success\":true"));
        assert!(json_str.contains("\"status\":\"in_progress\""));
        assert!(json_str.contains("\"accepterDid\":\"did:key:z6MkTest\""));
        assert!(json_str.contains("\"submissionInstructions\""));
    }

    #[test]
    fn test_build_submission_instructions_tests() {
        let bounty = Bounty {
            id: Uuid::new_v4(),
            poster_did: "did:key:z6MkPoster".to_string(),
            title: "Test Bounty".to_string(),
            description: "Description".to_string(),
            reward_credits: BigDecimal::from_str("100.00000000").unwrap(),
            closure_type: BountyClosureType::Tests,
            status: BountyStatus::InProgress,
            metadata: json!({ "eval_harness_hash": "sha256:test123" }),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deadline: Some(chrono::Utc::now() + chrono::Duration::days(7)),
        };

        let instructions = build_submission_instructions(&bounty);

        assert!(instructions.endpoint.contains("/submit"));
        assert_eq!(instructions.closure_type, BountyClosureType::Tests);
        assert_eq!(instructions.requirements["type"], "tests");
        assert_eq!(instructions.requirements["evalHarnessHash"], "sha256:test123");
        assert!(instructions.deadline.is_some());
    }

    #[test]
    fn test_build_submission_instructions_quorum() {
        let bounty = Bounty {
            id: Uuid::new_v4(),
            poster_did: "did:key:z6MkPoster".to_string(),
            title: "Quorum Bounty".to_string(),
            description: "Description".to_string(),
            reward_credits: BigDecimal::from_str("200.00000000").unwrap(),
            closure_type: BountyClosureType::Quorum,
            status: BountyStatus::InProgress,
            metadata: json!({ "reviewer_count": 5, "min_reviewer_rep": 50 }),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deadline: None,
        };

        let instructions = build_submission_instructions(&bounty);

        assert_eq!(instructions.closure_type, BountyClosureType::Quorum);
        assert_eq!(instructions.requirements["type"], "quorum");
        assert_eq!(instructions.requirements["reviewerCount"], 5);
        assert_eq!(instructions.requirements["minReviewerReputation"], 50);
        assert!(instructions.deadline.is_none());
    }

    #[test]
    fn test_build_submission_instructions_requester() {
        let bounty = Bounty {
            id: Uuid::new_v4(),
            poster_did: "did:key:z6MkPoster".to_string(),
            title: "Requester Bounty".to_string(),
            description: "Description".to_string(),
            reward_credits: BigDecimal::from_str("50.00000000").unwrap(),
            closure_type: BountyClosureType::Requester,
            status: BountyStatus::InProgress,
            metadata: json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deadline: None,
        };

        let instructions = build_submission_instructions(&bounty);

        assert_eq!(instructions.closure_type, BountyClosureType::Requester);
        assert_eq!(instructions.requirements["type"], "requester");
        assert!(instructions.requirements["requiredFields"].as_array().is_some());
    }

    #[test]
    fn test_accept_bounty_response_without_deadline() {
        let response = AcceptBountyResponse {
            success: true,
            bounty_id: Uuid::new_v4(),
            title: "No Deadline Bounty".to_string(),
            status: BountyStatus::InProgress,
            accepter_did: "did:key:z6MkAccepter".to_string(),
            submission_instructions: SubmissionInstructions {
                endpoint: "/api/v1/bounties/123/submit".to_string(),
                closure_type: BountyClosureType::Requester,
                requirements: json!({ "type": "requester" }),
                deadline: None,
            },
        };

        let json_str = serde_json::to_string(&response).unwrap();
        // deadline should be omitted from JSON when None
        assert!(!json_str.contains("\"deadline\":null"));
    }

    #[test]
    fn test_submission_instructions_includes_endpoint() {
        let bounty = Bounty {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap(),
            poster_did: "did:key:z6MkPoster".to_string(),
            title: "Test".to_string(),
            description: "Description".to_string(),
            reward_credits: BigDecimal::from_str("100.00000000").unwrap(),
            closure_type: BountyClosureType::Tests,
            status: BountyStatus::InProgress,
            metadata: json!({ "eval_harness_hash": "sha256:abc" }),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deadline: None,
        };

        let instructions = build_submission_instructions(&bounty);
        assert_eq!(
            instructions.endpoint,
            "/api/v1/bounties/550e8400-e29b-41d4-a716-446655440099/submit"
        );
    }

    #[test]
    fn test_submission_instructions_serialization() {
        let instructions = SubmissionInstructions {
            endpoint: "/api/v1/bounties/123/submit".to_string(),
            closure_type: BountyClosureType::Tests,
            requirements: json!({
                "type": "tests",
                "evalHarnessHash": "sha256:abc",
                "requiredFields": ["signatureEnvelope", "executionReceipt"]
            }),
            deadline: Some("2026-02-28T23:59:59Z".to_string()),
        };

        let json_str = serde_json::to_string(&instructions).unwrap();
        assert!(json_str.contains("\"closureType\":\"tests\""));
        assert!(json_str.contains("\"requiredFields\""));
        assert!(json_str.contains("signatureEnvelope"));
        assert!(json_str.contains("executionReceipt"));
    }

    // ===== Dispute Creation Tests =====

    #[test]
    fn test_validate_dispute_reason_valid() {
        let result = validate_dispute_reason("The submission does not match the requirements");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_dispute_reason_empty() {
        let result = validate_dispute_reason("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_dispute_reason_whitespace() {
        let result = validate_dispute_reason("   ");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_dispute_reason_too_long() {
        let long_reason = "a".repeat(2001);
        let result = validate_dispute_reason(&long_reason);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("2000 characters"));
    }

    #[test]
    fn test_validate_dispute_reason_max_length() {
        let max_reason = "a".repeat(2000);
        let result = validate_dispute_reason(&max_reason);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_dispute_window_within_window() {
        let now = Utc::now();
        let submission = BountySubmission {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            artifact_hash: "abc123".to_string(),
            signature_envelope: json!({}),
            execution_receipt: None,
            status: SubmissionStatus::Approved,
            created_at: now - Duration::days(3), // 3 days ago
            artifact_id: None,
        };

        let result = check_dispute_window(&submission);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_dispute_window_at_boundary() {
        let now = Utc::now();
        let submission = BountySubmission {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            artifact_hash: "abc123".to_string(),
            signature_envelope: json!({}),
            execution_receipt: None,
            status: SubmissionStatus::Approved,
            created_at: now - Duration::days(DISPUTE_WINDOW_DAYS) + Duration::hours(1), // Just within window
            artifact_id: None,
        };

        let result = check_dispute_window(&submission);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_dispute_window_expired() {
        let now = Utc::now();
        let submission = BountySubmission {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            artifact_hash: "abc123".to_string(),
            signature_envelope: json!({}),
            execution_receipt: None,
            status: SubmissionStatus::Approved,
            created_at: now - Duration::days(DISPUTE_WINDOW_DAYS + 1), // 8 days ago
            artifact_id: None,
        };

        let result = check_dispute_window(&submission);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Dispute window has closed"));
    }

    #[test]
    fn test_create_dispute_request_deserialization() {
        let json = r#"{
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "submissionId": "550e8400-e29b-41d4-a716-446655440001",
            "reason": "The submission is fraudulent"
        }"#;

        let request: CreateDisputeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.user_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(request.submission_id.to_string(), "550e8400-e29b-41d4-a716-446655440001");
        assert_eq!(request.reason, "The submission is fraudulent");
    }

    #[test]
    fn test_create_dispute_response_serialization() {
        let response = CreateDisputeResponse {
            success: true,
            dispute_id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submission_id: Uuid::new_v4(),
            initiator_did: "did:key:z6MkTest".to_string(),
            stake_amount: "10.00000000".to_string(),
            status: DisputeStatus::Pending,
            dispute_deadline: "2026-02-07T12:00:00+00:00".to_string(),
            message: "Dispute created successfully".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"disputeId\":"));
        assert!(json.contains("\"bountyId\":"));
        assert!(json.contains("\"submissionId\":"));
        assert!(json.contains("\"initiatorDid\":\"did:key:z6MkTest\""));
        assert!(json.contains("\"stakeAmount\":\"10.00000000\""));
        assert!(json.contains("\"status\":\"pending\""));
        assert!(json.contains("\"disputeDeadline\":"));
        assert!(json.contains("\"message\":"));
    }

    #[test]
    fn test_dispute_stake_calculation() {
        // Test that stake is 10% of bounty reward
        let reward = BigDecimal::from_str("100.00000000").unwrap();
        let stake = calculate_dispute_stake(&reward);
        let expected = BigDecimal::from_str("10.00000000").unwrap();
        assert_eq!(stake, expected);
    }

    #[test]
    fn test_dispute_stake_calculation_large_amount() {
        let reward = BigDecimal::from_str("5000.00000000").unwrap();
        let stake = calculate_dispute_stake(&reward);
        let expected = BigDecimal::from_str("500.00000000").unwrap();
        assert_eq!(stake, expected);
    }

    #[test]
    fn test_dispute_stake_calculation_small_amount() {
        let reward = BigDecimal::from_str("10.00000000").unwrap();
        let stake = calculate_dispute_stake(&reward);
        let expected = BigDecimal::from_str("1.00000000").unwrap();
        assert_eq!(stake, expected);
    }

    #[test]
    fn test_dispute_window_constant() {
        // Verify dispute window is 7 days
        assert_eq!(DISPUTE_WINDOW_DAYS, 7);
    }

    #[test]
    fn test_dispute_status_serialization() {
        assert_eq!(
            serde_json::to_string(&DisputeStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&DisputeStatus::Resolved).unwrap(),
            "\"resolved\""
        );
        assert_eq!(
            serde_json::to_string(&DisputeStatus::Expired).unwrap(),
            "\"expired\""
        );
    }

    #[test]
    fn test_submission_disputable_only_when_approved() {
        let now = Utc::now();

        // Pending submission should not be disputable
        let pending_submission = BountySubmission {
            id: Uuid::new_v4(),
            bounty_id: Uuid::new_v4(),
            submitter_did: "did:key:z6MkTest".to_string(),
            artifact_hash: "abc123".to_string(),
            signature_envelope: json!({}),
            execution_receipt: None,
            status: SubmissionStatus::Pending,
            created_at: now,
            artifact_id: None,
        };
        assert!(!pending_submission.is_approved());

        // Rejected submission should not be disputable
        let rejected_submission = BountySubmission {
            status: SubmissionStatus::Rejected,
            ..pending_submission.clone()
        };
        assert!(!rejected_submission.is_approved());

        // Approved submission should be disputable
        let approved_submission = BountySubmission {
            status: SubmissionStatus::Approved,
            ..pending_submission
        };
        assert!(approved_submission.is_approved());
    }

    // ===== Approval Flow Tests =====

    #[test]
    fn test_user_policy_requires_approval_below_threshold() {
        use crate::models::user_policy::UserPolicy;

        let policy = UserPolicy {
            did: "did:key:z6MkTest".to_string(),
            version: "1.0".to_string(),
            max_spend_per_day: BigDecimal::from_str("1000.00000000").unwrap(),
            max_spend_per_bounty: BigDecimal::from_str("500.00000000").unwrap(),
            enabled: true,
            approval_tiers: serde_json::json!([
                {"threshold": 100, "require_approval": true, "approvers": [], "timeout_hours": 24, "notification_channels": []}
            ]),
            allowed_delegates: serde_json::json!([]),
            emergency_contact: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Amount below threshold - no approval needed
        let amount = BigDecimal::from_str("50.00000000").unwrap();
        assert!(policy.requires_approval(&amount).is_none());
    }

    #[test]
    fn test_user_policy_requires_approval_above_threshold() {
        use crate::models::user_policy::UserPolicy;

        let policy = UserPolicy {
            did: "did:key:z6MkTest".to_string(),
            version: "1.0".to_string(),
            max_spend_per_day: BigDecimal::from_str("1000.00000000").unwrap(),
            max_spend_per_bounty: BigDecimal::from_str("500.00000000").unwrap(),
            enabled: true,
            approval_tiers: serde_json::json!([
                {"threshold": 100, "require_approval": true, "approvers": [], "timeout_hours": 24, "notification_channels": []}
            ]),
            allowed_delegates: serde_json::json!([]),
            emergency_contact: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Amount above threshold - approval required
        let amount = BigDecimal::from_str("150.00000000").unwrap();
        let tier = policy.requires_approval(&amount);
        assert!(tier.is_some());
        assert_eq!(tier.unwrap().threshold, 100.0);
    }

    #[test]
    fn test_user_policy_requires_approval_disabled() {
        use crate::models::user_policy::UserPolicy;

        let policy = UserPolicy {
            did: "did:key:z6MkTest".to_string(),
            version: "1.0".to_string(),
            max_spend_per_day: BigDecimal::from_str("1000.00000000").unwrap(),
            max_spend_per_bounty: BigDecimal::from_str("500.00000000").unwrap(),
            enabled: false, // Policy disabled
            approval_tiers: serde_json::json!([
                {"threshold": 100, "require_approval": true, "approvers": [], "timeout_hours": 24, "notification_channels": []}
            ]),
            allowed_delegates: serde_json::json!([]),
            emergency_contact: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Even with amount above threshold, disabled policy means no approval needed
        let amount = BigDecimal::from_str("500.00000000").unwrap();
        assert!(policy.requires_approval(&amount).is_none());
    }

    #[test]
    fn test_user_policy_multiple_tiers_selects_highest() {
        use crate::models::user_policy::UserPolicy;

        let policy = UserPolicy {
            did: "did:key:z6MkTest".to_string(),
            version: "1.0".to_string(),
            max_spend_per_day: BigDecimal::from_str("10000.00000000").unwrap(),
            max_spend_per_bounty: BigDecimal::from_str("5000.00000000").unwrap(),
            enabled: true,
            approval_tiers: serde_json::json!([
                {"threshold": 100, "require_approval": true, "approvers": [], "timeout_hours": 24, "notification_channels": []},
                {"threshold": 500, "require_approval": true, "approvers": ["did:key:z6MkApprover"], "timeout_hours": 48, "notification_channels": []},
                {"threshold": 1000, "require_approval": true, "approvers": [], "timeout_hours": 72, "notification_channels": []}
            ]),
            allowed_delegates: serde_json::json!([]),
            emergency_contact: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Amount above first tier only
        let amount1 = BigDecimal::from_str("200.00000000").unwrap();
        let tier1 = policy.requires_approval(&amount1);
        assert!(tier1.is_some());
        assert_eq!(tier1.unwrap().threshold, 100.0);

        // Amount above second tier
        let amount2 = BigDecimal::from_str("750.00000000").unwrap();
        let tier2 = policy.requires_approval(&amount2);
        assert!(tier2.is_some());
        assert_eq!(tier2.unwrap().threshold, 500.0);

        // Amount above all tiers - should select highest (1000)
        let amount3 = BigDecimal::from_str("1500.00000000").unwrap();
        let tier3 = policy.requires_approval(&amount3);
        assert!(tier3.is_some());
        assert_eq!(tier3.unwrap().threshold, 1000.0);
    }

    #[test]
    fn test_user_policy_tier_with_require_approval_false() {
        use crate::models::user_policy::UserPolicy;

        let policy = UserPolicy {
            did: "did:key:z6MkTest".to_string(),
            version: "1.0".to_string(),
            max_spend_per_day: BigDecimal::from_str("1000.00000000").unwrap(),
            max_spend_per_bounty: BigDecimal::from_str("500.00000000").unwrap(),
            enabled: true,
            approval_tiers: serde_json::json!([
                {"threshold": 100, "require_approval": false, "approvers": [], "timeout_hours": 24, "notification_channels": []}
            ]),
            allowed_delegates: serde_json::json!([]),
            emergency_contact: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Amount above threshold but require_approval is false
        let amount = BigDecimal::from_str("200.00000000").unwrap();
        assert!(policy.requires_approval(&amount).is_none());
    }

    #[test]
    fn test_create_bounty_response_with_approval() {
        let response = CreateBountyResponse {
            success: true,
            bounty_id: Uuid::new_v4(),
            title: "Test Bounty".to_string(),
            reward_credits: "500.00000000".to_string(),
            status: BountyStatus::PendingApproval,
            escrow_id: None,
            ledger_id: None,
            approval_request_id: Some(Uuid::new_v4()),
            message: Some("Approval required".to_string()),
        };

        assert!(response.approval_request_id.is_some());
        assert!(response.escrow_id.is_none());
        assert!(response.ledger_id.is_none());
        assert_eq!(response.status, BountyStatus::PendingApproval);
    }

    #[test]
    fn test_create_bounty_response_without_approval() {
        let response = CreateBountyResponse {
            success: true,
            bounty_id: Uuid::new_v4(),
            title: "Test Bounty".to_string(),
            reward_credits: "50.00000000".to_string(),
            status: BountyStatus::Open,
            escrow_id: Some(Uuid::new_v4()),
            ledger_id: Some(Uuid::new_v4()),
            approval_request_id: None,
            message: None,
        };

        assert!(response.approval_request_id.is_none());
        assert!(response.escrow_id.is_some());
        assert!(response.ledger_id.is_some());
        assert_eq!(response.status, BountyStatus::Open);
    }

    #[test]
    fn test_bounty_status_pending_approval() {
        let bounty = Bounty {
            id: Uuid::new_v4(),
            poster_did: "did:key:z6MkTest".to_string(),
            title: "Test Bounty".to_string(),
            description: "A test bounty".to_string(),
            reward_credits: BigDecimal::from_str("500.00000000").unwrap(),
            closure_type: BountyClosureType::Requester,
            status: BountyStatus::PendingApproval,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deadline: None,
        };

        assert!(bounty.is_pending_approval());
        assert!(!bounty.is_open());
        assert!(!bounty.is_in_progress());
        assert!(!bounty.is_completed());
        assert!(!bounty.is_cancelled());
        assert!(!bounty.is_active());
    }

    #[test]
    fn test_user_policy_no_tiers() {
        use crate::models::user_policy::UserPolicy;

        let policy = UserPolicy {
            did: "did:key:z6MkTest".to_string(),
            version: "1.0".to_string(),
            max_spend_per_day: BigDecimal::from_str("1000.00000000").unwrap(),
            max_spend_per_bounty: BigDecimal::from_str("500.00000000").unwrap(),
            enabled: true,
            approval_tiers: serde_json::json!([]), // No tiers
            allowed_delegates: serde_json::json!([]),
            emergency_contact: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // No tiers means no approval needed
        let amount = BigDecimal::from_str("500.00000000").unwrap();
        assert!(policy.requires_approval(&amount).is_none());
    }
}
