//! Dispute resolution routes for Protocol M.
//!
//! Handles dispute resolution by arbiters, including stake slashing
//! and escrow release based on resolution outcome.

use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    Bounty, BountySubmission, Dispute, DisputeStatus, NewMCreditsLedger,
    ResolutionOutcome,
};

/// Reputation penalty factor for losing a dispute.
/// Losers lose 50% of the reputation they would have gained.
const DISPUTE_REPUTATION_PENALTY_RATE: f64 = 0.5;

/// Request body for resolving a dispute.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveDisputeRequest {
    /// The user ID of the arbiter resolving the dispute.
    pub user_id: Uuid,
    /// The resolution outcome.
    pub outcome: String,
    /// Optional reason for the resolution decision.
    pub resolution_reason: Option<String>,
}

/// Response for successful dispute resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveDisputeResponse {
    /// Whether the resolution was successful.
    pub success: bool,
    /// The dispute ID.
    pub dispute_id: Uuid,
    /// The resolution outcome.
    pub outcome: String,
    /// The updated dispute status.
    pub status: DisputeStatus,
    /// Message explaining the resolution.
    pub message: String,
    /// Details about what happened to stakes and escrow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_details: Option<ResolutionDetails>,
}

/// Details about stake and escrow handling during resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionDetails {
    /// What happened to the initiator's stake.
    pub initiator_stake_action: String,
    /// What happened to the bounty escrow.
    pub bounty_escrow_action: String,
    /// Reputation changes applied.
    pub reputation_changes: Vec<ReputationChange>,
}

/// A reputation change applied during dispute resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReputationChange {
    /// The DID that had their reputation changed.
    pub did: String,
    /// The amount of reputation change (negative for penalty).
    pub amount: String,
    /// The reason for the change.
    pub reason: String,
}

/// Creates the disputes router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/{id}/resolve", post(resolve_dispute))
        .with_state(pool)
}

/// Loads a dispute by ID for resolution.
async fn load_dispute_for_resolution(pool: &PgPool, dispute_id: Uuid) -> Result<Dispute, AppError> {
    let dispute: Option<Dispute> = sqlx::query_as(
        r#"
        SELECT id, bounty_id, submission_id, initiator_did, reason, status, stake_amount,
               stake_escrow_id, resolution_outcome, resolver_did, created_at, resolved_at, dispute_deadline
        FROM disputes
        WHERE id = $1
        "#,
    )
    .bind(dispute_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query dispute: {}", e)))?;

    dispute.ok_or_else(|| AppError::NotFound(format!("Dispute {} not found", dispute_id)))
}

/// Loads the bounty associated with a dispute.
async fn load_bounty_for_resolution(pool: &PgPool, bounty_id: Uuid) -> Result<Bounty, AppError> {
    let bounty: Option<Bounty> = sqlx::query_as(
        r#"
        SELECT id, poster_did, title, description, reward_credits, closure_type, status, metadata, created_at, deadline, updated_at
        FROM bounties
        WHERE id = $1
        "#,
    )
    .bind(bounty_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query bounty: {}", e)))?;

    bounty.ok_or_else(|| AppError::NotFound(format!("Bounty {} not found", bounty_id)))
}

/// Loads the submission associated with a dispute.
async fn load_submission_for_resolution(
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

    submission.ok_or_else(|| AppError::NotFound(format!("Submission {} not found", submission_id)))
}

/// Gets the DID bound to a user ID.
async fn get_arbiter_did(pool: &PgPool, user_id: Uuid) -> Result<String, AppError> {
    let did: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT did FROM did_bindings
        WHERE user_id = $1 AND revoked_at IS NULL
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query arbiter DID: {}", e)))?;

    did.map(|(d,)| d)
        .ok_or_else(|| AppError::Forbidden("You must bind a DID to resolve disputes".to_string()))
}

/// Validates the resolution outcome string.
fn parse_resolution_outcome(outcome: &str) -> Result<ResolutionOutcome, AppError> {
    ResolutionOutcome::from_str(outcome).ok_or_else(|| {
        AppError::BadRequest(format!(
            "Invalid resolution outcome: '{}'. Must be 'uphold_submission' or 'reject_submission'",
            outcome
        ))
    })
}

/// Slashes the initiator's stake (they lose it).
/// The stake is burned from the system.
async fn slash_initiator_stake(
    pool: &PgPool,
    dispute: &Dispute,
    reason: &str,
) -> Result<Uuid, AppError> {
    let stake_escrow_id = dispute.stake_escrow_id.ok_or_else(|| {
        AppError::Internal("Dispute has no stake escrow reference".to_string())
    })?;

    // Mark stake escrow as cancelled (slashed)
    sqlx::query(
        r#"
        UPDATE escrow_holds
        SET status = 'cancelled', released_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(stake_escrow_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update stake escrow: {}", e)))?;

    // Insert burn event into ledger (stake is destroyed)
    let ledger_entry = NewMCreditsLedger::burn(
        dispute.initiator_did.clone(),
        dispute.stake_amount.clone(),
        json!({
            "dispute_id": dispute.id.to_string(),
            "reason": reason,
            "action": "stake_slashed"
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
    .map_err(|e| AppError::Internal(format!("Failed to insert stake slash ledger: {}", e)))?;

    Ok(ledger_id)
}

/// Returns the initiator's stake to them.
async fn return_initiator_stake(
    pool: &PgPool,
    dispute: &Dispute,
    reason: &str,
) -> Result<Uuid, AppError> {
    let stake_escrow_id = dispute.stake_escrow_id.ok_or_else(|| {
        AppError::Internal("Dispute has no stake escrow reference".to_string())
    })?;

    // Mark stake escrow as released
    sqlx::query(
        r#"
        UPDATE escrow_holds
        SET status = 'released', released_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(stake_escrow_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update stake escrow: {}", e)))?;

    // Insert release event into ledger
    let ledger_entry = NewMCreditsLedger::release(
        dispute.initiator_did.clone(),
        dispute.stake_amount.clone(),
        json!({
            "dispute_id": dispute.id.to_string(),
            "reason": reason,
            "action": "stake_returned"
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
    .map_err(|e| AppError::Internal(format!("Failed to insert stake return ledger: {}", e)))?;

    // Return stake to initiator's balance
    sqlx::query(
        r#"
        INSERT INTO m_credits_accounts (did, balance, promo_balance, created_at, updated_at)
        VALUES ($1, $2, 0, NOW(), NOW())
        ON CONFLICT (did) DO UPDATE
        SET balance = m_credits_accounts.balance + $2, updated_at = NOW()
        "#,
    )
    .bind(&dispute.initiator_did)
    .bind(&dispute.stake_amount)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to return stake to initiator: {}", e)))?;

    Ok(ledger_id)
}

/// Releases bounty escrow to the submitter (submission upheld).
async fn release_bounty_escrow_to_submitter(
    pool: &PgPool,
    bounty: &Bounty,
    submitter_did: &str,
    dispute_id: Uuid,
) -> Result<Uuid, AppError> {
    // Find the bounty's held escrow
    let escrow: Option<(Uuid, BigDecimal)> = sqlx::query_as(
        r#"
        SELECT id, amount
        FROM escrow_holds
        WHERE bounty_id = $1 AND status = 'held'
        "#,
    )
    .bind(bounty.id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query bounty escrow: {}", e)))?;

    let (escrow_id, amount) = escrow.ok_or_else(|| {
        AppError::BadRequest(format!("No active escrow hold found for bounty {}", bounty.id))
    })?;

    // Mark escrow as released
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
    .map_err(|e| AppError::Internal(format!("Failed to update bounty escrow: {}", e)))?;

    // Insert release event into ledger
    let ledger_entry = NewMCreditsLedger::release(
        submitter_did.to_string(),
        amount.clone(),
        json!({
            "bounty_id": bounty.id.to_string(),
            "escrow_id": escrow_id.to_string(),
            "dispute_id": dispute_id.to_string(),
            "reason": "dispute_upheld"
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
    .map_err(|e| AppError::Internal(format!("Failed to insert release ledger: {}", e)))?;

    // Update submitter's balance
    sqlx::query(
        r#"
        INSERT INTO m_credits_accounts (did, balance, promo_balance, created_at, updated_at)
        VALUES ($1, $2, 0, NOW(), NOW())
        ON CONFLICT (did) DO UPDATE
        SET balance = m_credits_accounts.balance + $2, updated_at = NOW()
        "#,
    )
    .bind(submitter_did)
    .bind(&amount)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update submitter balance: {}", e)))?;

    Ok(ledger_id)
}

/// Returns bounty escrow to the poster (submission rejected).
async fn return_bounty_escrow_to_poster(
    pool: &PgPool,
    bounty: &Bounty,
    dispute_id: Uuid,
) -> Result<Uuid, AppError> {
    // Find the bounty's held escrow
    let escrow: Option<(Uuid, BigDecimal)> = sqlx::query_as(
        r#"
        SELECT id, amount
        FROM escrow_holds
        WHERE bounty_id = $1 AND status = 'held'
        "#,
    )
    .bind(bounty.id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query bounty escrow: {}", e)))?;

    let (escrow_id, amount) = escrow.ok_or_else(|| {
        // Escrow might already be released if auto-approval happened
        // In this case, we need to handle it differently
        AppError::BadRequest(format!(
            "No active escrow hold found for bounty {}. The escrow may have already been released during auto-approval.",
            bounty.id
        ))
    })?;

    // Mark escrow as cancelled (returned to poster)
    sqlx::query(
        r#"
        UPDATE escrow_holds
        SET status = 'cancelled', released_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(escrow_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update bounty escrow: {}", e)))?;

    // Insert release event into ledger (back to poster)
    let ledger_entry = NewMCreditsLedger::release(
        bounty.poster_did.clone(),
        amount.clone(),
        json!({
            "bounty_id": bounty.id.to_string(),
            "escrow_id": escrow_id.to_string(),
            "dispute_id": dispute_id.to_string(),
            "reason": "dispute_rejected_submission"
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
    .map_err(|e| AppError::Internal(format!("Failed to insert return ledger: {}", e)))?;

    // Update poster's balance
    sqlx::query(
        r#"
        INSERT INTO m_credits_accounts (did, balance, promo_balance, created_at, updated_at)
        VALUES ($1, $2, 0, NOW(), NOW())
        ON CONFLICT (did) DO UPDATE
        SET balance = m_credits_accounts.balance + $2, updated_at = NOW()
        "#,
    )
    .bind(&bounty.poster_did)
    .bind(&amount)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update poster balance: {}", e)))?;

    Ok(ledger_id)
}

/// Updates the dispute record with resolution details.
async fn mark_dispute_resolved(
    pool: &PgPool,
    dispute_id: Uuid,
    outcome: ResolutionOutcome,
    resolver_did: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE disputes
        SET status = 'resolved',
            resolution_outcome = $2,
            resolver_did = $3,
            resolved_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(dispute_id)
    .bind(outcome.as_str())
    .bind(resolver_did)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update dispute status: {}", e)))?;

    Ok(())
}

/// Updates the bounty status based on dispute outcome.
async fn update_bounty_status_after_dispute(
    pool: &PgPool,
    bounty_id: Uuid,
    outcome: ResolutionOutcome,
) -> Result<(), AppError> {
    let new_status = match outcome {
        ResolutionOutcome::UpholdSubmission => "completed",
        ResolutionOutcome::RejectSubmission => "cancelled",
    };

    sqlx::query(
        r#"
        UPDATE bounties
        SET status = $2::bounty_status, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(bounty_id)
    .bind(new_status)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update bounty status: {}", e)))?;

    Ok(())
}

/// Updates submission status after dispute resolution.
async fn update_submission_status_after_dispute(
    pool: &PgPool,
    submission_id: Uuid,
    outcome: ResolutionOutcome,
) -> Result<(), AppError> {
    // If submission rejected by dispute, mark it as rejected
    if outcome == ResolutionOutcome::RejectSubmission {
        sqlx::query(
            r#"
            UPDATE bounty_submissions
            SET status = 'rejected'
            WHERE id = $1
            "#,
        )
        .bind(submission_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to update submission status: {}", e)))?;
    }
    // If upheld, status remains approved

    Ok(())
}

/// Applies reputation changes based on dispute outcome.
async fn apply_reputation_changes(
    pool: &PgPool,
    dispute: &Dispute,
    bounty: &Bounty,
    submission: &BountySubmission,
    outcome: ResolutionOutcome,
) -> Result<Vec<ReputationChange>, AppError> {
    use std::str::FromStr;

    let mut changes = Vec::new();

    // Calculate base reputation amount (same formula as bounty completion)
    let base_rep = &bounty.reward_credits
        * &BigDecimal::from_str(&format!("{:.8}", crate::routes::reputation::BASE_REPUTATION_RATE))
            .unwrap();

    match outcome {
        ResolutionOutcome::UpholdSubmission => {
            // Submitter keeps their reputation (or gains if not already minted)
            // Dispute initiator loses reputation (penalty for frivolous dispute)
            let penalty_amount = &base_rep
                * &BigDecimal::from_str(&format!("{:.8}", DISPUTE_REPUTATION_PENALTY_RATE))
                    .unwrap();

            // Record negative reputation event for initiator
            sqlx::query(
                r#"
                INSERT INTO reputation_events (
                    did, event_type, base_amount, closure_type_weight, reviewer_weight,
                    weighted_amount, reason, bounty_id, submission_id, metadata
                )
                VALUES ($1, 'dispute_penalty', $2, 1.0, 1.0, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(&dispute.initiator_did)
            .bind(&penalty_amount)
            .bind("Lost dispute: submission upheld")
            .bind(bounty.id)
            .bind(submission.id)
            .bind(json!({"dispute_id": dispute.id.to_string(), "outcome": "uphold_submission"}))
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to record reputation penalty: {}", e)))?;

            // Deduct from total reputation (ensure it doesn't go below 0)
            sqlx::query(
                r#"
                UPDATE m_reputation
                SET total_rep = GREATEST(total_rep - $2, 0),
                    last_updated = NOW()
                WHERE did = $1
                "#,
            )
            .bind(&dispute.initiator_did)
            .bind(&penalty_amount)
            .execute(pool)
            .await
            .ok(); // Ignore if no record exists

            changes.push(ReputationChange {
                did: dispute.initiator_did.clone(),
                amount: format!("-{}", penalty_amount),
                reason: "Lost dispute: frivolous claim penalty".to_string(),
            });
        }
        ResolutionOutcome::RejectSubmission => {
            // Submitter loses reputation (submitted invalid work)
            let penalty_amount = &base_rep
                * &BigDecimal::from_str(&format!("{:.8}", DISPUTE_REPUTATION_PENALTY_RATE))
                    .unwrap();

            // Record negative reputation event for submitter
            sqlx::query(
                r#"
                INSERT INTO reputation_events (
                    did, event_type, base_amount, closure_type_weight, reviewer_weight,
                    weighted_amount, reason, bounty_id, submission_id, metadata
                )
                VALUES ($1, 'dispute_penalty', $2, 1.0, 1.0, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(&submission.submitter_did)
            .bind(&penalty_amount)
            .bind("Lost dispute: submission rejected")
            .bind(bounty.id)
            .bind(submission.id)
            .bind(json!({"dispute_id": dispute.id.to_string(), "outcome": "reject_submission"}))
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to record reputation penalty: {}", e)))?;

            // Deduct from total reputation
            sqlx::query(
                r#"
                UPDATE m_reputation
                SET total_rep = GREATEST(total_rep - $2, 0),
                    last_updated = NOW()
                WHERE did = $1
                "#,
            )
            .bind(&submission.submitter_did)
            .bind(&penalty_amount)
            .execute(pool)
            .await
            .ok(); // Ignore if no record exists

            changes.push(ReputationChange {
                did: submission.submitter_did.clone(),
                amount: format!("-{}", penalty_amount),
                reason: "Lost dispute: invalid submission penalty".to_string(),
            });

            // Initiator gains small reputation reward for valid dispute
            let reward_amount = &base_rep
                * &BigDecimal::from_str(&format!("{:.8}", 0.1)).unwrap(); // 10% of base rep

            sqlx::query(
                r#"
                INSERT INTO reputation_events (
                    did, event_type, base_amount, closure_type_weight, reviewer_weight,
                    weighted_amount, reason, bounty_id, submission_id, metadata
                )
                VALUES ($1, 'dispute_reward', $2, 1.0, 1.0, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(&dispute.initiator_did)
            .bind(&reward_amount)
            .bind("Won dispute: valid challenge reward")
            .bind(bounty.id)
            .bind(submission.id)
            .bind(json!({"dispute_id": dispute.id.to_string(), "outcome": "reject_submission"}))
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to record reputation reward: {}", e)))?;

            // Add to initiator's reputation
            sqlx::query(
                r#"
                INSERT INTO m_reputation (did, total_rep, decay_factor, last_updated)
                VALUES ($1, $2, 1.0, NOW())
                ON CONFLICT (did) DO UPDATE
                SET total_rep = m_reputation.total_rep + $2, last_updated = NOW()
                "#,
            )
            .bind(&dispute.initiator_did)
            .bind(&reward_amount)
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to update initiator reputation: {}", e))
            })?;

            changes.push(ReputationChange {
                did: dispute.initiator_did.clone(),
                amount: format!("+{}", reward_amount),
                reason: "Won dispute: valid challenge reward".to_string(),
            });
        }
    }

    Ok(changes)
}

/// Resolves a dispute with the given outcome.
///
/// This endpoint requires admin/arbiter role (currently simplified to just DID binding).
///
/// # Resolution Outcomes
///
/// ## uphold_submission
/// - The original submission is valid
/// - Initiator loses their stake (slashed/burned)
/// - Submitter keeps the bounty reward
/// - Initiator loses reputation (frivolous dispute penalty)
///
/// ## reject_submission
/// - The original submission is invalid
/// - Initiator gets their stake back
/// - Bounty escrow returns to poster
/// - Submitter loses reputation (invalid work penalty)
/// - Initiator gains small reputation reward
async fn resolve_dispute(
    State(pool): State<PgPool>,
    Path(dispute_id): Path<Uuid>,
    Json(request): Json<ResolveDisputeRequest>,
) -> Result<Json<ResolveDisputeResponse>, AppError> {
    // Step 1: Get arbiter's DID
    let arbiter_did = get_arbiter_did(&pool, request.user_id).await?;

    // Step 2: Parse and validate outcome
    let outcome = parse_resolution_outcome(&request.outcome)?;

    // Step 3: Load the dispute
    let dispute = load_dispute_for_resolution(&pool, dispute_id).await?;

    // Step 4: Validate dispute is pending
    if !dispute.is_pending() {
        return Err(AppError::BadRequest(format!(
            "Dispute is not pending. Current status: {:?}",
            dispute.status
        )));
    }

    // Step 5: Load related entities
    let bounty = load_bounty_for_resolution(&pool, dispute.bounty_id).await?;
    let submission = load_submission_for_resolution(&pool, dispute.submission_id).await?;

    // Step 6: Execute resolution based on outcome
    let (initiator_action, escrow_action) = match outcome {
        ResolutionOutcome::UpholdSubmission => {
            // Slash initiator's stake
            slash_initiator_stake(&pool, &dispute, "Lost dispute: submission upheld").await?;

            // Release bounty escrow to submitter
            release_bounty_escrow_to_submitter(
                &pool,
                &bounty,
                &submission.submitter_did,
                dispute_id,
            )
            .await?;

            (
                "Stake slashed (burned)".to_string(),
                "Released to submitter".to_string(),
            )
        }
        ResolutionOutcome::RejectSubmission => {
            // Return initiator's stake
            return_initiator_stake(&pool, &dispute, "Won dispute: submission rejected").await?;

            // Return bounty escrow to poster
            return_bounty_escrow_to_poster(&pool, &bounty, dispute_id).await?;

            (
                "Stake returned".to_string(),
                "Returned to poster".to_string(),
            )
        }
    };

    // Step 7: Apply reputation changes
    let reputation_changes =
        apply_reputation_changes(&pool, &dispute, &bounty, &submission, outcome).await?;

    // Step 8: Update dispute record
    mark_dispute_resolved(&pool, dispute_id, outcome, &arbiter_did).await?;

    // Step 9: Update bounty status
    update_bounty_status_after_dispute(&pool, dispute.bounty_id, outcome).await?;

    // Step 10: Update submission status (if rejected)
    update_submission_status_after_dispute(&pool, dispute.submission_id, outcome).await?;

    // Step 11: Build response
    let message = match outcome {
        ResolutionOutcome::UpholdSubmission => format!(
            "Dispute resolved: submission upheld. Initiator's stake of {} M-credits has been slashed.",
            dispute.stake_amount
        ),
        ResolutionOutcome::RejectSubmission => format!(
            "Dispute resolved: submission rejected. Bounty escrow returned to poster, initiator's stake returned."
        ),
    };

    Ok(Json(ResolveDisputeResponse {
        success: true,
        dispute_id,
        outcome: outcome.as_str().to_string(),
        status: DisputeStatus::Resolved,
        message,
        resolution_details: Some(ResolutionDetails {
            initiator_stake_action: initiator_action,
            bounty_escrow_action: escrow_action,
            reputation_changes,
        }),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // ===== Resolution Outcome Parsing Tests =====

    #[test]
    fn test_parse_resolution_outcome_uphold() {
        let result = parse_resolution_outcome("uphold_submission");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ResolutionOutcome::UpholdSubmission);
    }

    #[test]
    fn test_parse_resolution_outcome_reject() {
        let result = parse_resolution_outcome("reject_submission");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ResolutionOutcome::RejectSubmission);
    }

    #[test]
    fn test_parse_resolution_outcome_invalid() {
        let result = parse_resolution_outcome("invalid_outcome");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid resolution outcome"));
    }

    #[test]
    fn test_parse_resolution_outcome_empty() {
        let result = parse_resolution_outcome("");
        assert!(result.is_err());
    }

    // ===== Request/Response Serialization Tests =====

    #[test]
    fn test_resolve_dispute_request_deserialization() {
        let json = r#"{
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "outcome": "uphold_submission",
            "resolutionReason": "The submission meets all requirements"
        }"#;

        let request: ResolveDisputeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.user_id,
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
        );
        assert_eq!(request.outcome, "uphold_submission");
        assert_eq!(
            request.resolution_reason,
            Some("The submission meets all requirements".to_string())
        );
    }

    #[test]
    fn test_resolve_dispute_request_without_reason() {
        let json = r#"{
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "outcome": "reject_submission"
        }"#;

        let request: ResolveDisputeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.outcome, "reject_submission");
        assert!(request.resolution_reason.is_none());
    }

    #[test]
    fn test_resolve_dispute_response_serialization() {
        let response = ResolveDisputeResponse {
            success: true,
            dispute_id: Uuid::new_v4(),
            outcome: "uphold_submission".to_string(),
            status: DisputeStatus::Resolved,
            message: "Dispute resolved successfully".to_string(),
            resolution_details: Some(ResolutionDetails {
                initiator_stake_action: "Stake slashed".to_string(),
                bounty_escrow_action: "Released to submitter".to_string(),
                reputation_changes: vec![ReputationChange {
                    did: "did:key:z6MkTest".to_string(),
                    amount: "-5.0".to_string(),
                    reason: "Lost dispute".to_string(),
                }],
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"outcome\":\"uphold_submission\""));
        assert!(json.contains("\"status\":\"resolved\""));
        assert!(json.contains("\"resolutionDetails\""));
        assert!(json.contains("\"reputationChanges\""));
    }

    #[test]
    fn test_resolve_dispute_response_without_details() {
        let response = ResolveDisputeResponse {
            success: true,
            dispute_id: Uuid::new_v4(),
            outcome: "uphold_submission".to_string(),
            status: DisputeStatus::Resolved,
            message: "Dispute resolved".to_string(),
            resolution_details: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("resolutionDetails"));
    }

    // ===== Resolution Details Tests =====

    #[test]
    fn test_resolution_details_serialization() {
        let details = ResolutionDetails {
            initiator_stake_action: "Stake returned".to_string(),
            bounty_escrow_action: "Returned to poster".to_string(),
            reputation_changes: vec![
                ReputationChange {
                    did: "did:key:z6MkSubmitter".to_string(),
                    amount: "-10.0".to_string(),
                    reason: "Invalid submission penalty".to_string(),
                },
                ReputationChange {
                    did: "did:key:z6MkInitiator".to_string(),
                    amount: "+2.0".to_string(),
                    reason: "Valid challenge reward".to_string(),
                },
            ],
        };

        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains("\"initiatorStakeAction\":\"Stake returned\""));
        assert!(json.contains("\"bountyEscrowAction\":\"Returned to poster\""));
        assert!(json.contains("\"reputationChanges\""));
    }

    // ===== Reputation Change Tests =====

    #[test]
    fn test_reputation_change_serialization() {
        let change = ReputationChange {
            did: "did:key:z6MkTest123".to_string(),
            amount: "-5.50000000".to_string(),
            reason: "Test penalty".to_string(),
        };

        let json = serde_json::to_string(&change).unwrap();
        assert!(json.contains("\"did\":\"did:key:z6MkTest123\""));
        assert!(json.contains("\"amount\":\"-5.50000000\""));
        assert!(json.contains("\"reason\":\"Test penalty\""));
    }

    // ===== Penalty Calculation Tests =====

    #[test]
    fn test_dispute_reputation_penalty_rate() {
        assert_eq!(DISPUTE_REPUTATION_PENALTY_RATE, 0.5);
    }

    #[test]
    fn test_penalty_calculation() {
        let reward = BigDecimal::from_str("100.00000000").unwrap();
        let base_rep_rate = BigDecimal::from_str("0.1").unwrap();
        let penalty_rate = BigDecimal::from_str("0.5").unwrap();

        let base_rep = &reward * &base_rep_rate; // 10.0
        let penalty = &base_rep * &penalty_rate; // 5.0

        let expected_penalty = BigDecimal::from_str("5.00000000").unwrap();
        assert_eq!(penalty, expected_penalty);
    }

    // ===== Outcome Logic Tests =====

    #[test]
    fn test_uphold_submission_outcome() {
        let outcome = ResolutionOutcome::UpholdSubmission;
        assert_eq!(outcome.as_str(), "uphold_submission");
    }

    #[test]
    fn test_reject_submission_outcome() {
        let outcome = ResolutionOutcome::RejectSubmission;
        assert_eq!(outcome.as_str(), "reject_submission");
    }

    #[test]
    fn test_outcome_roundtrip() {
        let outcomes = vec![
            ResolutionOutcome::UpholdSubmission,
            ResolutionOutcome::RejectSubmission,
        ];

        for outcome in outcomes {
            let str_repr = outcome.as_str();
            let parsed = ResolutionOutcome::from_str(str_repr);
            assert_eq!(parsed, Some(outcome));
        }
    }

    // ===== Status Tests =====

    #[test]
    fn test_dispute_status_after_resolution() {
        // After resolution, status should be Resolved
        let status = DisputeStatus::Resolved;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"resolved\"");
    }

    // ===== Bounty Status Update Tests =====

    #[test]
    fn test_bounty_status_for_upheld() {
        // When submission is upheld, bounty should be completed
        let outcome = ResolutionOutcome::UpholdSubmission;
        let new_status = match outcome {
            ResolutionOutcome::UpholdSubmission => "completed",
            ResolutionOutcome::RejectSubmission => "cancelled",
        };
        assert_eq!(new_status, "completed");
    }

    #[test]
    fn test_bounty_status_for_rejected() {
        // When submission is rejected, bounty should be cancelled
        let outcome = ResolutionOutcome::RejectSubmission;
        let new_status = match outcome {
            ResolutionOutcome::UpholdSubmission => "completed",
            ResolutionOutcome::RejectSubmission => "cancelled",
        };
        assert_eq!(new_status, "cancelled");
    }
}
