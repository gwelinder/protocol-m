//! Identity management endpoints for DID binding.

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use chrono::{Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::did_challenge::DidChallenge;

/// Challenge length in bytes (32 bytes = 256 bits).
const CHALLENGE_BYTES: usize = 32;

/// Challenge expiry time in minutes.
const CHALLENGE_EXPIRY_MINUTES: i64 = 10;

/// Request body for creating a challenge.
/// Note: In a real implementation, the user_id would come from authentication.
#[derive(Debug, Deserialize)]
pub struct CreateChallengeRequest {
    /// The user ID requesting the challenge.
    /// In production, this would be extracted from auth token.
    pub user_id: Uuid,
}

/// Response for successful challenge creation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeResponse {
    /// The random challenge (hex-encoded).
    pub challenge: String,
    /// When this challenge expires (ISO 8601).
    pub expires_at: String,
}

/// Creates the identity router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/challenge", post(create_challenge))
        .with_state(pool)
}

/// Generates a random 32-byte challenge as a hex string.
fn generate_challenge() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; CHALLENGE_BYTES] = rng.gen();
    hex::encode(bytes)
}

/// POST /api/v1/identity/challenge
///
/// Creates a new challenge for DID binding.
/// The client must sign this challenge with their private key
/// and submit the signature to the /bind endpoint.
async fn create_challenge(
    State(pool): State<PgPool>,
    Json(request): Json<CreateChallengeRequest>,
) -> Result<Json<ChallengeResponse>, AppError> {
    let challenge = generate_challenge();
    let expires_at = Utc::now() + Duration::minutes(CHALLENGE_EXPIRY_MINUTES);

    // Insert challenge into database
    let _inserted: DidChallenge = sqlx::query_as(
        r#"
        INSERT INTO did_challenges (id, user_id, challenge, expires_at, created_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, challenge, expires_at, used_at, created_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(request.user_id)
    .bind(&challenge)
    .bind(expires_at)
    .bind(Utc::now())
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create challenge: {}", e)))?;

    Ok(Json(ChallengeResponse {
        challenge,
        expires_at: expires_at.to_rfc3339(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_challenge_length() {
        let challenge = generate_challenge();
        // 32 bytes = 64 hex characters
        assert_eq!(challenge.len(), 64);
    }

    #[test]
    fn test_generate_challenge_is_hex() {
        let challenge = generate_challenge();
        assert!(challenge.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_challenge_is_random() {
        let c1 = generate_challenge();
        let c2 = generate_challenge();
        // Two random challenges should be different (with overwhelming probability)
        assert_ne!(c1, c2);
    }

    #[test]
    fn test_challenge_response_serialization() {
        let response = ChallengeResponse {
            challenge: "0123456789abcdef".repeat(4),
            expires_at: "2026-01-31T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"challenge\":"));
        assert!(json.contains("\"expiresAt\":")); // camelCase in JSON
    }
}
