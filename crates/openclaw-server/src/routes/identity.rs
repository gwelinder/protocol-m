//! Identity management endpoints for DID binding.

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use chrono::{Duration, Utc};
use ed25519_dalek::{Signature, Verifier};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::did_binding::DidBinding;
use crate::models::did_challenge::{DidChallenge, MAX_BIND_ATTEMPTS};
use openclaw_crypto::did_to_verifying_key;

/// Challenge length in bytes (32 bytes = 256 bits).
const CHALLENGE_BYTES: usize = 32;

/// Challenge expiry time in minutes.
const CHALLENGE_EXPIRY_MINUTES: i64 = 10;

/// Maximum challenges per user per hour.
const MAX_CHALLENGES_PER_HOUR: i64 = 5;

/// Rate limit window in minutes.
const RATE_LIMIT_WINDOW_MINUTES: i64 = 60;

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

/// Request body for binding a DID to a user account.
/// Note: In a real implementation, the user_id would come from authentication.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindDidRequest {
    /// The user ID requesting the binding.
    /// In production, this would be extracted from auth token.
    pub user_id: Uuid,
    /// The DID to bind (did:key:z...).
    pub did: String,
    /// The challenge that was signed.
    pub challenge: String,
    /// Base64-encoded Ed25519 signature over the challenge bytes.
    pub challenge_signature: String,
}

/// Response for successful DID binding.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindDidResponse {
    /// The bound DID.
    pub did: String,
    /// Success message.
    pub message: String,
}

/// Creates the identity router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/challenge", post(create_challenge))
        .route("/bind", post(bind_did))
        .with_state(pool)
}

/// Generates a random 32-byte challenge as a hex string.
fn generate_challenge() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; CHALLENGE_BYTES] = rng.gen();
    hex::encode(bytes)
}

/// Result of a rate limit check.
struct RateLimitResult {
    /// Whether the rate limit has been exceeded.
    exceeded: bool,
    /// Seconds until the rate limit window resets (if exceeded).
    retry_after: u64,
}

/// Checks if a user has exceeded the challenge rate limit.
/// Returns the number of seconds to wait if rate limit is exceeded.
async fn check_challenge_rate_limit(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<RateLimitResult, AppError> {
    let window_start = Utc::now() - Duration::minutes(RATE_LIMIT_WINDOW_MINUTES);

    // Count challenges created in the rate limit window
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) as count
        FROM did_challenges
        WHERE user_id = $1 AND created_at >= $2
        "#,
    )
    .bind(user_id)
    .bind(window_start)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to check rate limit: {}", e)))?;

    let count = row.0;

    if count >= MAX_CHALLENGES_PER_HOUR {
        // Find the oldest challenge in the window to calculate retry_after
        let oldest_challenge: Option<(chrono::DateTime<Utc>,)> = sqlx::query_as(
            r#"
            SELECT created_at
            FROM did_challenges
            WHERE user_id = $1 AND created_at >= $2
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(window_start)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to query oldest challenge: {}", e)))?;

        // Calculate seconds until the oldest challenge expires from the rate limit window
        let retry_after = if let Some((oldest_created_at,)) = oldest_challenge {
            let window_end = oldest_created_at + Duration::minutes(RATE_LIMIT_WINDOW_MINUTES);
            let now = Utc::now();
            if window_end > now {
                (window_end - now).num_seconds().max(1) as u64
            } else {
                1 // Already past window, retry after 1 second
            }
        } else {
            // Fallback: retry after full window
            (RATE_LIMIT_WINDOW_MINUTES * 60) as u64
        };

        Ok(RateLimitResult {
            exceeded: true,
            retry_after,
        })
    } else {
        Ok(RateLimitResult {
            exceeded: false,
            retry_after: 0,
        })
    }
}

/// Increments the failed_attempts counter for a challenge.
async fn increment_failed_attempts(
    pool: &PgPool,
    challenge_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE did_challenges
        SET failed_attempts = failed_attempts + 1
        WHERE id = $1
        "#,
    )
    .bind(challenge_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to increment failed attempts: {}", e)))?;

    Ok(())
}

/// POST /api/v1/identity/challenge
///
/// Creates a new challenge for DID binding.
/// The client must sign this challenge with their private key
/// and submit the signature to the /bind endpoint.
///
/// Rate limited to 5 challenges per user per hour.
async fn create_challenge(
    State(pool): State<PgPool>,
    Json(request): Json<CreateChallengeRequest>,
) -> Result<Json<ChallengeResponse>, AppError> {
    // Check rate limit
    let rate_limit = check_challenge_rate_limit(&pool, request.user_id).await?;
    if rate_limit.exceeded {
        return Err(AppError::TooManyRequests {
            message: format!(
                "Rate limit exceeded: maximum {} challenges per hour",
                MAX_CHALLENGES_PER_HOUR
            ),
            retry_after: rate_limit.retry_after,
        });
    }

    let challenge = generate_challenge();
    let expires_at = Utc::now() + Duration::minutes(CHALLENGE_EXPIRY_MINUTES);

    // Insert challenge into database
    let _inserted: DidChallenge = sqlx::query_as(
        r#"
        INSERT INTO did_challenges (id, user_id, challenge, expires_at, created_at, failed_attempts)
        VALUES ($1, $2, $3, $4, $5, 0)
        RETURNING id, user_id, challenge, expires_at, used_at, created_at, failed_attempts
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

/// POST /api/v1/identity/bind
///
/// Binds a DID to a user account after verifying ownership.
/// The client must have previously requested a challenge and signed it
/// with their private key corresponding to the DID.
async fn bind_did(
    State(pool): State<PgPool>,
    Json(request): Json<BindDidRequest>,
) -> Result<Json<BindDidResponse>, AppError> {
    // Step 1: Validate DID format and extract verifying key
    let verifying_key = did_to_verifying_key(&request.did)
        .map_err(|e| AppError::BadRequest(format!("Invalid DID format: {}", e)))?;

    // Step 2: Load challenge from database
    let challenge_record: Option<DidChallenge> = sqlx::query_as(
        r#"
        SELECT id, user_id, challenge, expires_at, used_at, created_at, failed_attempts
        FROM did_challenges
        WHERE challenge = $1 AND user_id = $2
        "#,
    )
    .bind(&request.challenge)
    .bind(request.user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query challenge: {}", e)))?;

    let challenge_record = challenge_record
        .ok_or_else(|| AppError::BadRequest("Challenge not found".to_string()))?;

    // Step 3: Verify challenge is not locked due to too many failed attempts
    if challenge_record.is_locked() {
        return Err(AppError::TooManyRequests {
            message: format!(
                "Challenge locked after {} failed attempts. Please request a new challenge.",
                MAX_BIND_ATTEMPTS
            ),
            retry_after: 0, // Indicates a new challenge is needed, not a wait
        });
    }

    // Step 4: Verify challenge is not expired
    if challenge_record.is_expired() {
        return Err(AppError::BadRequest("Challenge has expired".to_string()));
    }

    // Step 5: Verify challenge is not already used
    if challenge_record.is_used() {
        return Err(AppError::BadRequest("Challenge has already been used".to_string()));
    }

    // Step 6: Decode the challenge hex to bytes for signature verification
    let challenge_bytes = hex::decode(&request.challenge)
        .map_err(|e| AppError::BadRequest(format!("Invalid challenge format: {}", e)))?;

    // Step 7: Decode the base64 signature
    use base64::Engine;
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(&request.challenge_signature)
        .map_err(|e| AppError::BadRequest(format!("Invalid signature encoding: {}", e)))?;

    // Step 8: Parse signature bytes into Signature struct
    let signature_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| AppError::BadRequest("Invalid signature length: expected 64 bytes".to_string()))?;
    let signature = Signature::from_bytes(&signature_array);

    // Step 9: Verify the signature over the challenge bytes
    if let Err(_) = verifying_key.verify(&challenge_bytes, &signature) {
        // Increment failed attempts counter
        increment_failed_attempts(&pool, challenge_record.id).await?;

        let new_attempts = challenge_record.failed_attempts + 1;
        if new_attempts >= MAX_BIND_ATTEMPTS {
            return Err(AppError::TooManyRequests {
                message: format!(
                    "Challenge locked after {} failed attempts. Please request a new challenge.",
                    MAX_BIND_ATTEMPTS
                ),
                retry_after: 0,
            });
        } else {
            let remaining = MAX_BIND_ATTEMPTS - new_attempts;
            return Err(AppError::BadRequest(format!(
                "Signature verification failed. {} attempt(s) remaining.",
                remaining
            )));
        }
    }

    // Step 10: Check if DID is already bound to this user
    let existing_binding: Option<DidBinding> = sqlx::query_as(
        r#"
        SELECT id, user_id, did, created_at, revoked_at
        FROM did_bindings
        WHERE did = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(&request.did)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to check existing binding: {}", e)))?;

    if let Some(existing) = existing_binding {
        if existing.user_id == request.user_id {
            return Err(AppError::BadRequest("DID is already bound to your account".to_string()));
        } else {
            return Err(AppError::BadRequest("DID is already bound to another account".to_string()));
        }
    }

    // Step 11: Insert DID binding in a transaction
    let mut tx = pool.begin().await
        .map_err(|e| AppError::Internal(format!("Failed to begin transaction: {}", e)))?;

    // Insert the DID binding
    let _binding: DidBinding = sqlx::query_as(
        r#"
        INSERT INTO did_bindings (id, user_id, did, created_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id, user_id, did, created_at, revoked_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(request.user_id)
    .bind(&request.did)
    .bind(Utc::now())
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create DID binding: {}", e)))?;

    // Mark the challenge as used
    sqlx::query(
        r#"
        UPDATE did_challenges
        SET used_at = $1
        WHERE id = $2
        "#,
    )
    .bind(Utc::now())
    .bind(challenge_record.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to mark challenge as used: {}", e)))?;

    // Commit transaction
    tx.commit().await
        .map_err(|e| AppError::Internal(format!("Failed to commit transaction: {}", e)))?;

    Ok(Json(BindDidResponse {
        did: request.did,
        message: "DID successfully bound to your account".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use ed25519_dalek::{SigningKey, Signer};
    use openclaw_crypto::pubkey_to_did;

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

    #[test]
    fn test_bind_request_deserialization() {
        let json = r#"{
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "did": "did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw",
            "challenge": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "challengeSignature": "dGVzdCBzaWduYXR1cmU="
        }"#;

        let request: BindDidRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.user_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
        assert!(request.did.starts_with("did:key:z"));
        assert_eq!(request.challenge.len(), 64);
    }

    #[test]
    fn test_bind_response_serialization() {
        let response = BindDidResponse {
            did: "did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw".to_string(),
            message: "DID successfully bound".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"did\":"));
        assert!(json.contains("\"message\":"));
    }

    #[test]
    fn test_signature_verification_logic() {
        // Create a test keypair
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let did = pubkey_to_did(&verifying_key);

        // Generate a challenge
        let challenge = generate_challenge();
        let challenge_bytes = hex::decode(&challenge).unwrap();

        // Sign the challenge
        let signature = signing_key.sign(&challenge_bytes);
        let signature_base64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

        // Verify using the same logic as the handler
        let recovered_key = did_to_verifying_key(&did).unwrap();
        let signature_bytes = base64::engine::general_purpose::STANDARD
            .decode(&signature_base64)
            .unwrap();
        let sig_array: [u8; 64] = signature_bytes.try_into().unwrap();
        let sig = Signature::from_bytes(&sig_array);

        // This should succeed
        assert!(recovered_key.verify(&challenge_bytes, &sig).is_ok());
    }

    #[test]
    fn test_signature_verification_fails_with_wrong_key() {
        // Create two different keypairs
        let seed1: [u8; 32] = [0x42; 32];
        let seed2: [u8; 32] = [0x43; 32];
        let signing_key1 = SigningKey::from_bytes(&seed1);
        let signing_key2 = SigningKey::from_bytes(&seed2);
        let verifying_key2 = signing_key2.verifying_key();
        let did2 = pubkey_to_did(&verifying_key2);

        // Generate and sign challenge with key1
        let challenge = generate_challenge();
        let challenge_bytes = hex::decode(&challenge).unwrap();
        let signature = signing_key1.sign(&challenge_bytes);
        let signature_base64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

        // Try to verify with key2's DID (should fail)
        let recovered_key = did_to_verifying_key(&did2).unwrap();
        let signature_bytes = base64::engine::general_purpose::STANDARD
            .decode(&signature_base64)
            .unwrap();
        let sig_array: [u8; 64] = signature_bytes.try_into().unwrap();
        let sig = Signature::from_bytes(&sig_array);

        // This should fail
        assert!(recovered_key.verify(&challenge_bytes, &sig).is_err());
    }

    #[test]
    fn test_signature_verification_fails_with_wrong_challenge() {
        // Create a keypair
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let did = pubkey_to_did(&verifying_key);

        // Sign one challenge, but verify against a different one
        let challenge1 = generate_challenge();
        let challenge1_bytes = hex::decode(&challenge1).unwrap();
        let signature = signing_key.sign(&challenge1_bytes);
        let signature_base64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

        // Different challenge for verification
        let challenge2 = generate_challenge();
        let challenge2_bytes = hex::decode(&challenge2).unwrap();

        let recovered_key = did_to_verifying_key(&did).unwrap();
        let signature_bytes = base64::engine::general_purpose::STANDARD
            .decode(&signature_base64)
            .unwrap();
        let sig_array: [u8; 64] = signature_bytes.try_into().unwrap();
        let sig = Signature::from_bytes(&sig_array);

        // This should fail
        assert!(recovered_key.verify(&challenge2_bytes, &sig).is_err());
    }

    #[test]
    fn test_invalid_signature_length() {
        // Invalid base64 that decodes to wrong length
        let short_sig = base64::engine::general_purpose::STANDARD.encode(vec![0u8; 32]);
        let decoded = base64::engine::general_purpose::STANDARD.decode(&short_sig).unwrap();
        let result: Result<[u8; 64], _> = decoded.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_did_format() {
        let invalid_did = "did:web:example.com";
        let result = did_to_verifying_key(invalid_did);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must start with"));
    }

    #[test]
    fn test_invalid_challenge_hex() {
        let invalid_hex = "zzzz";
        let result = hex::decode(invalid_hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limit_constants() {
        // Verify rate limit configuration
        assert_eq!(MAX_CHALLENGES_PER_HOUR, 5);
        assert_eq!(RATE_LIMIT_WINDOW_MINUTES, 60);
    }

    #[test]
    fn test_rate_limit_result_struct() {
        let result = RateLimitResult {
            exceeded: true,
            retry_after: 3600,
        };
        assert!(result.exceeded);
        assert_eq!(result.retry_after, 3600);

        let result = RateLimitResult {
            exceeded: false,
            retry_after: 0,
        };
        assert!(!result.exceeded);
        assert_eq!(result.retry_after, 0);
    }

    #[test]
    fn test_max_bind_attempts_constant() {
        // Verify bind attempt limit is set to 3
        assert_eq!(MAX_BIND_ATTEMPTS, 3);
    }

    #[test]
    fn test_bind_attempt_remaining_calculation() {
        // Test that remaining attempts are correctly calculated
        for failed in 0..MAX_BIND_ATTEMPTS {
            let remaining = MAX_BIND_ATTEMPTS - (failed + 1);
            if failed + 1 >= MAX_BIND_ATTEMPTS {
                assert_eq!(remaining, 0);
            } else {
                assert!(remaining > 0);
            }
        }
    }

    #[test]
    fn test_locked_challenge_returns_429_message() {
        // Verify the error message format for locked challenges
        let message = format!(
            "Challenge locked after {} failed attempts. Please request a new challenge.",
            MAX_BIND_ATTEMPTS
        );
        assert!(message.contains("3 failed attempts"));
        assert!(message.contains("new challenge"));
    }

    #[test]
    fn test_failed_attempt_error_message() {
        // Test error message format for failed attempts with remaining tries
        let failed_attempts = 1;
        let remaining = MAX_BIND_ATTEMPTS - (failed_attempts + 1);
        let message = format!(
            "Signature verification failed. {} attempt(s) remaining.",
            remaining
        );
        assert!(message.contains("1 attempt(s) remaining"));

        let failed_attempts = 2;
        let remaining = MAX_BIND_ATTEMPTS - (failed_attempts + 1);
        let message = format!(
            "Signature verification failed. {} attempt(s) remaining.",
            remaining
        );
        assert!(message.contains("0 attempt(s) remaining"));
    }
}
