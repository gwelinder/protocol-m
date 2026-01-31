//! Profile endpoints for user data retrieval.

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

/// Response structure for a bound DID.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundDid {
    /// The DID string (did:key:z6Mk...).
    pub did: String,
    /// When this DID was bound to the account.
    pub created_at: DateTime<Utc>,
}

/// Response structure for user profile with DIDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResponse {
    /// User ID.
    pub user_id: Uuid,
    /// Array of bound DIDs, ordered by created_at (newest first).
    pub dids: Vec<BoundDid>,
}

/// Creates the profile router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/{user_id}/dids", get(get_user_dids))
        .with_state(pool)
}

/// GET /api/v1/profile/{user_id}/dids
///
/// Returns all active (non-revoked) DIDs bound to a user account.
/// Results are ordered by created_at (newest first).
async fn get_user_dids(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ProfileResponse>, AppError> {
    // Query active DID bindings for this user
    let bindings: Vec<(String, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT did, created_at
        FROM did_bindings
        WHERE user_id = $1 AND revoked_at IS NULL
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query DID bindings: {}", e)))?;

    let dids: Vec<BoundDid> = bindings
        .into_iter()
        .map(|(did, created_at)| BoundDid { did, created_at })
        .collect();

    Ok(Json(ProfileResponse { user_id, dids }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bound_did_serialization() {
        let bound_did = BoundDid {
            did: "did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw".to_string(),
            created_at: DateTime::parse_from_rfc3339("2026-01-31T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };

        let json = serde_json::to_string(&bound_did).unwrap();
        assert!(json.contains("\"did\":"));
        assert!(json.contains("\"createdAt\":")); // camelCase
    }

    #[test]
    fn test_profile_response_serialization() {
        let response = ProfileResponse {
            user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            dids: vec![
                BoundDid {
                    did: "did:key:z6Mk1...".to_string(),
                    created_at: Utc::now(),
                },
                BoundDid {
                    did: "did:key:z6Mk2...".to_string(),
                    created_at: Utc::now(),
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"userId\":"));
        assert!(json.contains("\"dids\":"));
    }

    #[test]
    fn test_profile_response_with_empty_dids() {
        let response = ProfileResponse {
            user_id: Uuid::new_v4(),
            dids: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"dids\":[]"));
    }

    #[test]
    fn test_bound_did_deserialization() {
        let json = r#"{
            "did": "did:key:z6MkTest",
            "createdAt": "2026-01-31T12:00:00Z"
        }"#;

        let bound_did: BoundDid = serde_json::from_str(json).unwrap();
        assert_eq!(bound_did.did, "did:key:z6MkTest");
    }
}
