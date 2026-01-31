//! Agent management endpoints for Protocol M, including kill switch functionality.

use axum::{
    extract::State,
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
    AgentSuspension, BountyStatus, MCreditsEventType, NewAgentSuspension,
};

/// Request body for emergency stop.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyStopRequest {
    /// DID of the operator initiating emergency stop.
    pub operator_did: String,
    /// Optional reason for the emergency stop.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Response for emergency stop.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyStopResponse {
    /// Whether the emergency stop was successful.
    pub success: bool,
    /// The suspension record ID.
    pub suspension_id: Uuid,
    /// Message explaining the result.
    pub message: String,
    /// Number of bounties cancelled.
    pub bounties_cancelled: i64,
    /// Number of approval requests cancelled.
    pub approval_requests_cancelled: i64,
    /// Total escrow refunded (as string for precision).
    pub escrow_refunded: String,
}

/// Creates the agents router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/emergency-stop", post(emergency_stop))
        .with_state(pool)
}

/// Handles emergency stop - suspends the agent and cancels all pending operations.
async fn emergency_stop(
    State(pool): State<PgPool>,
    Json(request): Json<EmergencyStopRequest>,
) -> Result<Json<EmergencyStopResponse>, AppError> {
    // Validate DID format
    validate_did(&request.operator_did)?;

    // Check if agent is already suspended
    if is_agent_suspended(&pool, &request.operator_did).await? {
        return Err(AppError::BadRequest(
            "Agent is already suspended".to_string(),
        ));
    }

    // Start a transaction for atomicity
    let mut tx = pool.begin().await.map_err(|e| {
        AppError::Internal(format!("Failed to start transaction: {}", e))
    })?;

    // Step 1: Cancel all pending approval requests for this DID
    let approval_requests_cancelled = cancel_pending_approval_requests(&mut tx, &request.operator_did).await?;

    // Step 2: Cancel all open bounties and refund escrow
    let (bounties_cancelled, escrow_refunded) = cancel_open_bounties_and_refund(&mut tx, &request.operator_did).await?;

    // Step 3: Create suspension record
    let reason = request.reason.unwrap_or_else(|| "Emergency stop initiated by operator".to_string());
    let metadata = json!({
        "bounties_cancelled": bounties_cancelled,
        "approval_requests_cancelled": approval_requests_cancelled,
        "escrow_refunded": escrow_refunded.to_string()
    });

    let new_suspension = NewAgentSuspension::with_metadata(
        request.operator_did.clone(),
        reason,
        metadata,
    );

    let suspension: AgentSuspension = sqlx::query_as(
        r#"
        INSERT INTO agent_suspensions (id, operator_did, reason, suspended_at, metadata)
        VALUES (gen_random_uuid(), $1, $2, NOW(), $3)
        RETURNING id, operator_did, reason, suspended_at, resumed_at, metadata, resumed_by_did
        "#,
    )
    .bind(&new_suspension.operator_did)
    .bind(&new_suspension.reason)
    .bind(&new_suspension.metadata)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create suspension: {}", e)))?;

    // Commit transaction
    tx.commit().await.map_err(|e| {
        AppError::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    // Log the emergency stop
    tracing::warn!(
        operator_did = %request.operator_did,
        suspension_id = %suspension.id,
        bounties_cancelled = bounties_cancelled,
        approval_requests_cancelled = approval_requests_cancelled,
        escrow_refunded = %escrow_refunded,
        "Agent emergency stop executed"
    );

    Ok(Json(EmergencyStopResponse {
        success: true,
        suspension_id: suspension.id,
        message: "Agent suspended successfully. All pending operations cancelled.".to_string(),
        bounties_cancelled,
        approval_requests_cancelled,
        escrow_refunded: escrow_refunded.to_string(),
    }))
}

/// Validates a DID format.
fn validate_did(did: &str) -> Result<(), AppError> {
    if !did.starts_with("did:key:z") {
        return Err(AppError::BadRequest(format!(
            "Invalid DID format: {}. Expected did:key:z...",
            did
        )));
    }
    // Verify it's a valid DID by trying to parse it
    openclaw_crypto::did_to_verifying_key(did)
        .map_err(|e| AppError::BadRequest(format!("Invalid DID: {}", e)))?;
    Ok(())
}

/// Checks if an agent is currently suspended.
pub async fn is_agent_suspended(pool: &PgPool, operator_did: &str) -> Result<bool, AppError> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM agent_suspensions
        WHERE operator_did = $1 AND resumed_at IS NULL
        "#,
    )
    .bind(operator_did)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to check suspension: {}", e)))?;

    Ok(count > 0)
}

/// Cancels all pending approval requests for a DID.
/// Returns the number of requests cancelled.
async fn cancel_pending_approval_requests(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    operator_did: &str,
) -> Result<i64, AppError> {
    // Mark all pending approval requests as expired
    let result = sqlx::query(
        r#"
        UPDATE approval_requests
        SET status = 'expired', resolved_at = NOW(), resolution_reason = 'Emergency stop'
        WHERE requester_did = $1 AND status = 'pending'
        "#,
    )
    .bind(operator_did)
    .execute(&mut **tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to cancel approval requests: {}", e)))?;

    Ok(result.rows_affected() as i64)
}

/// Cancels all open bounties posted by a DID and refunds escrow.
/// Returns (bounties_cancelled, total_escrow_refunded).
async fn cancel_open_bounties_and_refund(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    poster_did: &str,
) -> Result<(i64, BigDecimal), AppError> {
    // Find all open or pending_approval bounties posted by this DID
    let bounties: Vec<(Uuid, BigDecimal)> = sqlx::query_as(
        r#"
        SELECT b.id, b.reward_credits
        FROM bounties b
        WHERE b.poster_did = $1 AND b.status IN ('open', 'pending_approval', 'in_progress')
        "#,
    )
    .bind(poster_did)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch bounties: {}", e)))?;

    let bounties_count = bounties.len() as i64;
    let mut total_refunded = BigDecimal::from_str("0.00000000").unwrap();

    for (bounty_id, _reward) in &bounties {
        // Find and release held escrow for this bounty
        let escrow: Option<(Uuid, BigDecimal)> = sqlx::query_as(
            r#"
            SELECT id, amount FROM escrow_holds
            WHERE bounty_id = $1 AND status = 'held'
            "#,
        )
        .bind(bounty_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch escrow: {}", e)))?;

        if let Some((escrow_id, amount)) = escrow {
            // Release escrow: mark as cancelled
            sqlx::query(
                r#"
                UPDATE escrow_holds
                SET status = 'cancelled', released_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(escrow_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to release escrow: {}", e)))?;

            // Refund to poster's balance
            sqlx::query(
                r#"
                UPDATE m_credits_accounts
                SET balance = balance + $2, updated_at = NOW()
                WHERE did = $1
                "#,
            )
            .bind(poster_did)
            .bind(&amount)
            .execute(&mut **tx)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to refund balance: {}", e)))?;

            // Record refund in ledger
            sqlx::query(
                r#"
                INSERT INTO m_credits_ledger (id, event_type, from_did, to_did, amount, metadata, created_at)
                VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, NOW())
                "#,
            )
            .bind(MCreditsEventType::Release)
            .bind(Option::<String>::None) // from_did is null for release
            .bind(poster_did)
            .bind(&amount)
            .bind(json!({
                "reason": "Emergency stop - escrow refund",
                "bounty_id": bounty_id.to_string(),
                "escrow_id": escrow_id.to_string()
            }))
            .execute(&mut **tx)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to record ledger entry: {}", e)))?;

            total_refunded = total_refunded + amount;
        }

        // Cancel the bounty
        sqlx::query(
            r#"
            UPDATE bounties
            SET status = $2
            WHERE id = $1
            "#,
        )
        .bind(bounty_id)
        .bind(BountyStatus::Cancelled)
        .execute(&mut **tx)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to cancel bounty: {}", e)))?;
    }

    Ok((bounties_count, total_refunded))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Request/Response Serialization Tests =====

    #[test]
    fn test_emergency_stop_request_deserialization() {
        let json = r#"{
            "operatorDid": "did:key:z6MkTest",
            "reason": "Runaway spending detected"
        }"#;

        let request: EmergencyStopRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.operator_did, "did:key:z6MkTest");
        assert_eq!(request.reason, Some("Runaway spending detected".to_string()));
    }

    #[test]
    fn test_emergency_stop_request_without_reason() {
        let json = r#"{
            "operatorDid": "did:key:z6MkTest"
        }"#;

        let request: EmergencyStopRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.operator_did, "did:key:z6MkTest");
        assert_eq!(request.reason, None);
    }

    #[test]
    fn test_emergency_stop_response_serialization() {
        let response = EmergencyStopResponse {
            success: true,
            suspension_id: Uuid::new_v4(),
            message: "Agent suspended successfully".to_string(),
            bounties_cancelled: 5,
            approval_requests_cancelled: 3,
            escrow_refunded: "1500.00000000".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"suspensionId\":"));
        assert!(json.contains("\"bountiesCancelled\":5"));
        assert!(json.contains("\"approvalRequestsCancelled\":3"));
        assert!(json.contains("\"escrowRefunded\":\"1500.00000000\""));
    }

    #[test]
    fn test_emergency_stop_response_zero_values() {
        let response = EmergencyStopResponse {
            success: true,
            suspension_id: Uuid::new_v4(),
            message: "Agent suspended successfully".to_string(),
            bounties_cancelled: 0,
            approval_requests_cancelled: 0,
            escrow_refunded: "0.00000000".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"bountiesCancelled\":0"));
        assert!(json.contains("\"approvalRequestsCancelled\":0"));
        assert!(json.contains("\"escrowRefunded\":\"0.00000000\""));
    }

    // ===== Validation Tests =====

    #[test]
    fn test_validate_did_valid() {
        // This test uses a real DID derived from a known seed
        // The DID format is did:key:z + Base58BTC(0xed01 + 32-byte pubkey)
        let valid_did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        assert!(validate_did(valid_did).is_ok());
    }

    #[test]
    fn test_validate_did_invalid_prefix() {
        let invalid_did = "did:web:example.com";
        let result = validate_did(invalid_did);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid DID format"));
    }

    #[test]
    fn test_validate_did_invalid_format() {
        let invalid_did = "did:key:zInvalidDID";
        let result = validate_did(invalid_did);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid DID"));
    }

    #[test]
    fn test_validate_did_empty() {
        let empty_did = "";
        let result = validate_did(empty_did);
        assert!(result.is_err());
    }

    // ===== Integration-style Tests =====

    #[test]
    fn test_new_agent_suspension_with_metadata() {
        let reason = "Emergency stop".to_string();
        let metadata = json!({
            "bounties_cancelled": 5,
            "approval_requests_cancelled": 3,
            "escrow_refunded": "1500.00000000"
        });

        let suspension = NewAgentSuspension::with_metadata(
            "did:key:z6MkTest".to_string(),
            reason.clone(),
            metadata.clone(),
        );

        assert_eq!(suspension.operator_did, "did:key:z6MkTest");
        assert_eq!(suspension.reason, "Emergency stop");
        assert_eq!(suspension.metadata, Some(metadata));
    }
}
