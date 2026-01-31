//! Approval request endpoints for Protocol M operator approval workflow.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    ApprovalActionType, ApprovalRequest, Bounty, BountyStatus, EscrowStatus, NewEscrowHold,
    NewMCreditsLedger,
};

/// Response for getting approval request details.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequestResponse {
    /// The unique approval request ID.
    pub id: Uuid,
    /// DID of the operator who must approve.
    pub operator_did: String,
    /// DID of the agent/user who requested approval.
    pub requester_did: String,
    /// Type of action requiring approval.
    pub action_type: String,
    /// Amount of credits involved (for spend actions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    /// Current status of the request.
    pub status: String,
    /// When the request expires.
    pub expires_at: String,
    /// Whether the request has expired.
    pub is_expired: bool,
    /// Bounty details (if action_type is spend).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounty: Option<BountyDetails>,
    /// Additional metadata.
    pub metadata: serde_json::Value,
}

/// Bounty details included in approval request response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BountyDetails {
    /// The bounty ID.
    pub id: Uuid,
    /// Title of the bounty.
    pub title: String,
    /// Description of the bounty.
    pub description: String,
    /// Amount of credits offered as reward.
    pub reward_credits: String,
    /// How bounty completion is verified.
    pub closure_type: String,
    /// Current bounty status.
    pub status: String,
}

/// Request body for approving an approval request.
/// Note: In production, operator_did would come from authentication.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveRequest {
    /// DID of the operator approving the request.
    /// In production, this would be extracted from auth token.
    pub operator_did: String,
    /// Optional reason for approval.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Response for successful approval.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveResponse {
    /// Whether the approval was successful.
    pub success: bool,
    /// The approval request ID.
    pub approval_request_id: Uuid,
    /// Message explaining the result.
    pub message: String,
    /// The bounty ID (if action_type is spend).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounty_id: Option<Uuid>,
    /// The escrow hold ID (if escrow was created).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escrow_id: Option<Uuid>,
    /// The ledger entry ID (if escrow was created).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger_id: Option<Uuid>,
}

/// Creates the approvals router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/{id}", get(get_approval_request))
        .route("/{id}/approve", post(approve_request))
        .with_state(pool)
}

/// Gets an approval request by ID.
async fn get_approval_request(
    State(pool): State<PgPool>,
    Path(request_id): Path<Uuid>,
) -> Result<Json<ApprovalRequestResponse>, AppError> {
    // Load approval request
    let request = load_approval_request(&pool, request_id).await?;

    // Load bounty details if this is a spend action
    let bounty = if let Some(bounty_id) = request.bounty_id {
        let b = load_bounty(&pool, bounty_id).await?;
        Some(BountyDetails {
            id: b.id,
            title: b.title,
            description: b.description,
            reward_credits: b.reward_credits.to_string(),
            closure_type: format!("{:?}", b.closure_type).to_lowercase(),
            status: format!("{:?}", b.status).to_lowercase(),
        })
    } else {
        None
    };

    let is_expired = request.is_past_expiry();
    Ok(Json(ApprovalRequestResponse {
        id: request.id,
        operator_did: request.operator_did,
        requester_did: request.requester_did,
        action_type: request.action_type.as_str().to_string(),
        amount: request.amount.as_ref().map(|a| a.to_string()),
        status: request.status.as_str().to_string(),
        expires_at: request.expires_at.to_rfc3339(),
        is_expired,
        bounty,
        metadata: request.metadata,
    }))
}

/// Approves an approval request and proceeds with the bounty creation.
async fn approve_request(
    State(pool): State<PgPool>,
    Path(request_id): Path<Uuid>,
    Json(request): Json<ApproveRequest>,
) -> Result<Json<ApproveResponse>, AppError> {
    // Load and validate approval request
    let approval_request = load_approval_request(&pool, request_id).await?;

    // Verify operator DID matches
    if approval_request.operator_did != request.operator_did {
        return Err(AppError::Forbidden(
            "Only the designated operator can approve this request".to_string(),
        ));
    }

    // Check if request is still valid
    if !approval_request.is_pending() {
        return Err(AppError::BadRequest(format!(
            "Approval request is not pending (status: {})",
            approval_request.status.as_str()
        )));
    }

    if approval_request.is_past_expiry() {
        // Mark as expired
        mark_approval_expired(&pool, request_id).await?;
        return Err(AppError::BadRequest(
            "Approval request has expired".to_string(),
        ));
    }

    // Handle based on action type
    match approval_request.action_type {
        ApprovalActionType::Spend => {
            // Load the bounty
            let bounty_id = approval_request.bounty_id.ok_or_else(|| {
                AppError::Internal("Spend approval missing bounty_id".to_string())
            })?;
            let bounty = load_bounty(&pool, bounty_id).await?;

            // Verify bounty is in pending_approval status
            if bounty.status != BountyStatus::PendingApproval {
                return Err(AppError::BadRequest(format!(
                    "Bounty is not pending approval (status: {:?})",
                    bounty.status
                )));
            }

            // Get the reward amount
            let amount = approval_request.amount.ok_or_else(|| {
                AppError::Internal("Spend approval missing amount".to_string())
            })?;

            // Create escrow hold
            let (escrow_id, ledger_id) =
                create_escrow_hold(&pool, bounty_id, &bounty.poster_did, &amount).await?;

            // Update bounty status to open
            update_bounty_status(&pool, bounty_id, BountyStatus::Open).await?;

            // Mark approval request as approved
            mark_approval_approved(&pool, request_id, request.reason.as_deref()).await?;

            Ok(Json(ApproveResponse {
                success: true,
                approval_request_id: request_id,
                message: "Bounty approved and escrow created successfully".to_string(),
                bounty_id: Some(bounty_id),
                escrow_id: Some(escrow_id),
                ledger_id: Some(ledger_id),
            }))
        }
        ApprovalActionType::Delegate => {
            // For delegate actions, just mark as approved
            mark_approval_approved(&pool, request_id, request.reason.as_deref()).await?;

            Ok(Json(ApproveResponse {
                success: true,
                approval_request_id: request_id,
                message: "Delegation approved successfully".to_string(),
                bounty_id: None,
                escrow_id: None,
                ledger_id: None,
            }))
        }
    }
}

/// Loads an approval request by ID.
async fn load_approval_request(pool: &PgPool, id: Uuid) -> Result<ApprovalRequest, AppError> {
    sqlx::query_as::<_, ApprovalRequest>(
        r#"
        SELECT id, operator_did, bounty_id, action_type, amount, status, metadata,
               created_at, resolved_at, expires_at, requester_did, resolution_reason
        FROM approval_requests
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to load approval request: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Approval request not found".to_string()))
}

/// Loads a bounty by ID.
async fn load_bounty(pool: &PgPool, id: Uuid) -> Result<Bounty, AppError> {
    sqlx::query_as::<_, Bounty>(
        r#"
        SELECT id, poster_did, title, description, reward_credits, closure_type,
               status, metadata, created_at, deadline
        FROM bounties
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to load bounty: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Bounty not found".to_string()))
}

/// Marks an approval request as approved.
async fn mark_approval_approved(
    pool: &PgPool,
    id: Uuid,
    reason: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE approval_requests
        SET status = 'approved', resolved_at = NOW(), resolution_reason = $2
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(reason)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update approval request: {}", e)))?;

    Ok(())
}

/// Marks an approval request as expired.
async fn mark_approval_expired(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE approval_requests
        SET status = 'expired', resolved_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update approval request: {}", e)))?;

    Ok(())
}

/// Updates a bounty's status.
async fn update_bounty_status(
    pool: &PgPool,
    bounty_id: Uuid,
    status: BountyStatus,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE bounties
        SET status = $2
        WHERE id = $1
        "#,
    )
    .bind(bounty_id)
    .bind(status)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update bounty status: {}", e)))?;

    Ok(())
}

/// Creates an escrow hold by:
/// 1. Inserting a hold event into the ledger
/// 2. Creating an escrow_holds record
/// 3. Deducting from the holder's balance
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
            "reason": "Bounty escrow (approved)"
        }),
    );

    let ledger_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO m_credits_ledger (id, event_type, from_did, to_did, amount, metadata, created_at)
        VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, NOW())
        RETURNING id
        "#,
    )
    .bind(&ledger_entry.event_type)
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

    // Step 3: Deduct from holder's balance
    sqlx::query(
        r#"
        UPDATE m_credits_accounts
        SET balance = balance - $2, updated_at = NOW()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ApprovalRequestStatus;

    // ===== Response Serialization Tests =====

    #[test]
    fn test_approval_request_response_serialization() {
        let response = ApprovalRequestResponse {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkOperator".to_string(),
            requester_did: "did:key:z6MkRequester".to_string(),
            action_type: "spend".to_string(),
            amount: Some("500.00000000".to_string()),
            status: "pending".to_string(),
            expires_at: "2026-02-01T00:00:00+00:00".to_string(),
            is_expired: false,
            bounty: Some(BountyDetails {
                id: Uuid::new_v4(),
                title: "Test bounty".to_string(),
                description: "A test bounty".to_string(),
                reward_credits: "500.00000000".to_string(),
                closure_type: "tests".to_string(),
                status: "pending_approval".to_string(),
            }),
            metadata: json!({"description": "High-value bounty"}),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"operatorDid\":"));
        assert!(json.contains("\"requesterDid\":"));
        assert!(json.contains("\"actionType\":\"spend\""));
        assert!(json.contains("\"amount\":\"500.00000000\""));
        assert!(json.contains("\"isExpired\":false"));
        assert!(json.contains("\"bounty\":"));
    }

    #[test]
    fn test_approval_request_response_without_bounty() {
        let response = ApprovalRequestResponse {
            id: Uuid::new_v4(),
            operator_did: "did:key:z6MkOperator".to_string(),
            requester_did: "did:key:z6MkRequester".to_string(),
            action_type: "delegate".to_string(),
            amount: None,
            status: "pending".to_string(),
            expires_at: "2026-02-01T00:00:00+00:00".to_string(),
            is_expired: false,
            bounty: None,
            metadata: json!({"delegate_to_did": "did:key:z6MkDelegate"}),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"actionType\":\"delegate\""));
        assert!(!json.contains("\"amount\":"));
        assert!(!json.contains("\"bounty\":"));
    }

    #[test]
    fn test_bounty_details_serialization() {
        let details = BountyDetails {
            id: Uuid::new_v4(),
            title: "Fix critical bug".to_string(),
            description: "A critical bug needs fixing".to_string(),
            reward_credits: "1000.00000000".to_string(),
            closure_type: "tests".to_string(),
            status: "pending_approval".to_string(),
        };

        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains("\"title\":\"Fix critical bug\""));
        assert!(json.contains("\"rewardCredits\":\"1000.00000000\""));
        assert!(json.contains("\"closureType\":\"tests\""));
        assert!(json.contains("\"status\":\"pending_approval\""));
    }

    #[test]
    fn test_approve_request_deserialization() {
        let json = r#"{
            "operatorDid": "did:key:z6MkOperator",
            "reason": "Approved for trusted agent"
        }"#;

        let request: ApproveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.operator_did, "did:key:z6MkOperator");
        assert_eq!(request.reason, Some("Approved for trusted agent".to_string()));
    }

    #[test]
    fn test_approve_request_without_reason() {
        let json = r#"{
            "operatorDid": "did:key:z6MkOperator"
        }"#;

        let request: ApproveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.operator_did, "did:key:z6MkOperator");
        assert_eq!(request.reason, None);
    }

    #[test]
    fn test_approve_response_serialization() {
        let response = ApproveResponse {
            success: true,
            approval_request_id: Uuid::new_v4(),
            message: "Bounty approved and escrow created successfully".to_string(),
            bounty_id: Some(Uuid::new_v4()),
            escrow_id: Some(Uuid::new_v4()),
            ledger_id: Some(Uuid::new_v4()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"approvalRequestId\":"));
        assert!(json.contains("\"bountyId\":"));
        assert!(json.contains("\"escrowId\":"));
        assert!(json.contains("\"ledgerId\":"));
    }

    #[test]
    fn test_approve_response_delegate() {
        let response = ApproveResponse {
            success: true,
            approval_request_id: Uuid::new_v4(),
            message: "Delegation approved successfully".to_string(),
            bounty_id: None,
            escrow_id: None,
            ledger_id: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Delegation approved successfully\""));
        // Optional fields should not be present
        assert!(!json.contains("\"bountyId\":"));
        assert!(!json.contains("\"escrowId\":"));
        assert!(!json.contains("\"ledgerId\":"));
    }

    // ===== Validation Tests =====

    #[test]
    fn test_approval_action_type_display() {
        assert_eq!(ApprovalActionType::Spend.as_str(), "spend");
        assert_eq!(ApprovalActionType::Delegate.as_str(), "delegate");
    }

    #[test]
    fn test_approval_request_status_display() {
        assert_eq!(ApprovalRequestStatus::Pending.as_str(), "pending");
        assert_eq!(ApprovalRequestStatus::Approved.as_str(), "approved");
        assert_eq!(ApprovalRequestStatus::Rejected.as_str(), "rejected");
        assert_eq!(ApprovalRequestStatus::Expired.as_str(), "expired");
    }
}
