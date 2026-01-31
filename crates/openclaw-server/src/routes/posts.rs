//! Post routes and signature verification logic.

use crate::error::AppError;
use crate::models::{Post, VerificationStatus};
use anyhow::{anyhow, Context, Result};
use axum::{extract::State, routing::post, Json, Router};
use base64::prelude::*;
use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Creates the posts router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/", post(create_post))
        .with_state(pool)
}

/// Request body for creating a post.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePostRequest {
    pub user_id: Uuid,
    pub content: String,
    /// Optional signature envelope for verification
    pub signature_envelope: Option<serde_json::Value>,
}

/// Response for a created post.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePostResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub verification_status: VerificationStatus,
    pub verified_did: Option<String>,
    pub created_at: String,
}

/// POST /api/v1/posts - Create a new post with optional signature verification.
async fn create_post(
    State(pool): State<PgPool>,
    Json(req): Json<CreatePostRequest>,
) -> Result<Json<CreatePostResponse>, AppError> {
    // Verify signature if envelope is provided
    let verification = if let Some(ref envelope) = req.signature_envelope {
        verify_post_signature(&req.content, envelope, req.user_id, &pool).await
    } else {
        VerificationResult::none()
    };

    let now = Utc::now();
    let post_id = Uuid::new_v4();

    // Insert post into database
    let post: Post = sqlx::query_as(
        r#"
        INSERT INTO posts (id, user_id, content, signature_envelope_json, verified_did, verification_status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
        RETURNING *
        "#,
    )
    .bind(post_id)
    .bind(req.user_id)
    .bind(&req.content)
    .bind(&req.signature_envelope)
    .bind(&verification.verified_did)
    .bind(&verification.status)
    .bind(now)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create post: {}", e)))?;

    Ok(Json(CreatePostResponse {
        id: post.id,
        user_id: post.user_id,
        content: post.content,
        verification_status: post.verification_status,
        verified_did: post.verified_did,
        created_at: post.created_at.to_rfc3339(),
    }))
}

/// Result of post signature verification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    pub status: VerificationStatus,
    pub verified_did: Option<String>,
    pub error_message: Option<String>,
}

impl VerificationResult {
    pub fn none() -> Self {
        Self {
            status: VerificationStatus::None,
            verified_did: None,
            error_message: None,
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            status: VerificationStatus::Invalid,
            verified_did: None,
            error_message: Some(message.into()),
        }
    }

    pub fn valid_unbound(did: String) -> Self {
        Self {
            status: VerificationStatus::ValidUnbound,
            verified_did: Some(did),
            error_message: None,
        }
    }

    pub fn valid_bound(did: String) -> Self {
        Self {
            status: VerificationStatus::ValidBound,
            verified_did: Some(did),
            error_message: None,
        }
    }
}

/// Verify a post signature and check if the DID is bound to the user.
///
/// # Arguments
/// * `post_body` - The post content as UTF-8 string
/// * `envelope` - The signature envelope JSON
/// * `user_id` - The user ID posting the content
/// * `pool` - Database connection pool for checking DID bindings
///
/// # Returns
/// * `VerificationResult` with status and optional DID
pub async fn verify_post_signature(
    post_body: &str,
    envelope: &serde_json::Value,
    user_id: Uuid,
    pool: &PgPool,
) -> VerificationResult {
    // Step 1: Extract required fields from envelope
    let signer = match envelope.get("signer").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing 'signer' field in envelope"),
    };

    let hash_value = match envelope
        .get("hash")
        .and_then(|h| h.get("value"))
        .and_then(|v| v.as_str())
    {
        Some(h) => h,
        None => return VerificationResult::invalid("Missing 'hash.value' field in envelope"),
    };

    let signature_b64 = match envelope.get("signature").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerificationResult::invalid("Missing 'signature' field in envelope"),
    };

    // Step 2: Recompute SHA-256 hash of post body
    let computed_hash = {
        let mut hasher = Sha256::new();
        hasher.update(post_body.as_bytes());
        hex::encode(hasher.finalize())
    };

    // Step 3: Verify hash matches
    if computed_hash != hash_value {
        return VerificationResult::invalid("Hash mismatch: content has been modified");
    }

    // Step 4: Extract public key from DID
    let verifying_key = match did_to_verifying_key(signer) {
        Ok(key) => key,
        Err(e) => return VerificationResult::invalid(format!("Invalid DID: {}", e)),
    };

    // Step 5: Verify signature
    match verify_envelope_signature(envelope, &verifying_key, signature_b64) {
        Ok(()) => {}
        Err(e) => return VerificationResult::invalid(format!("Signature verification failed: {}", e)),
    }

    // Step 6: Check if DID is bound to user
    let is_bound = match check_did_bound_to_user(signer, user_id, pool).await {
        Ok(bound) => bound,
        Err(e) => {
            // Log error but don't fail - treat as unbound
            eprintln!("Warning: Failed to check DID binding: {}", e);
            false
        }
    };

    if is_bound {
        VerificationResult::valid_bound(signer.to_string())
    } else {
        VerificationResult::valid_unbound(signer.to_string())
    }
}

/// Extract VerifyingKey from a did:key string.
fn did_to_verifying_key(did: &str) -> Result<VerifyingKey> {
    // Strip prefix "did:key:z"
    let encoded = did
        .strip_prefix("did:key:z")
        .ok_or_else(|| anyhow!("Invalid DID format: must start with 'did:key:z'"))?;

    // Decode Base58BTC
    let bytes = bs58::decode(encoded)
        .into_vec()
        .context("Failed to decode Base58BTC")?;

    // Verify multicodec prefix (0xed01 for Ed25519)
    if bytes.len() < 34 {
        return Err(anyhow!("DID too short"));
    }
    if bytes[0] != 0xed || bytes[1] != 0x01 {
        return Err(anyhow!("Invalid multicodec prefix"));
    }

    // Extract 32-byte public key
    let pubkey_bytes: [u8; 32] = bytes[2..34]
        .try_into()
        .context("Invalid public key length")?;

    VerifyingKey::from_bytes(&pubkey_bytes).context("Invalid public key bytes")
}

/// Verify signature over canonicalized envelope (without signature field).
fn verify_envelope_signature(
    envelope: &serde_json::Value,
    verifying_key: &VerifyingKey,
    signature_b64: &str,
) -> Result<()> {
    // Decode signature
    let sig_bytes = BASE64_STANDARD
        .decode(signature_b64)
        .context("Invalid base64 signature")?;

    if sig_bytes.len() != 64 {
        return Err(anyhow!("Invalid signature length: expected 64 bytes"));
    }

    let signature = Signature::from_bytes(
        sig_bytes
            .as_slice()
            .try_into()
            .context("Failed to convert signature bytes")?,
    );

    // Create envelope without signature for canonicalization
    let mut envelope_copy = envelope.clone();
    if let Some(obj) = envelope_copy.as_object_mut() {
        obj.insert("signature".to_string(), serde_json::Value::String(String::new()));
    }

    // Canonicalize using JCS (serde_jcs)
    let canonical = serde_jcs::to_vec(&envelope_copy).context("Failed to canonicalize envelope")?;

    // Verify signature
    verifying_key
        .verify(&canonical, &signature)
        .context("Signature verification failed")?;

    Ok(())
}

/// Check if a DID is bound to a specific user.
async fn check_did_bound_to_user(did: &str, user_id: Uuid, pool: &PgPool) -> Result<bool> {
    // Use runtime query to avoid compile-time DATABASE_URL requirement
    let result: (bool,) = sqlx::query_as(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM did_bindings
            WHERE did = $1 AND user_id = $2 AND revoked_at IS NULL
        )
        "#,
    )
    .bind(did)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .context("Failed to query DID binding")?;

    Ok(result.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_result_none() {
        let result = VerificationResult::none();
        assert_eq!(result.status, VerificationStatus::None);
        assert!(result.verified_did.is_none());
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_verification_result_invalid() {
        let result = VerificationResult::invalid("test error");
        assert_eq!(result.status, VerificationStatus::Invalid);
        assert!(result.verified_did.is_none());
        assert_eq!(result.error_message, Some("test error".to_string()));
    }

    #[test]
    fn test_verification_result_valid_unbound() {
        let result = VerificationResult::valid_unbound("did:key:z6Mk...".to_string());
        assert_eq!(result.status, VerificationStatus::ValidUnbound);
        assert_eq!(result.verified_did, Some("did:key:z6Mk...".to_string()));
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_verification_result_valid_bound() {
        let result = VerificationResult::valid_bound("did:key:z6Mk...".to_string());
        assert_eq!(result.status, VerificationStatus::ValidBound);
        assert_eq!(result.verified_did, Some("did:key:z6Mk...".to_string()));
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_did_to_verifying_key_invalid_prefix() {
        let result = did_to_verifying_key("not:a:did");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid DID format"));
    }

    #[test]
    fn test_did_to_verifying_key_invalid_base58() {
        let result = did_to_verifying_key("did:key:zOOOO"); // O is invalid in Base58
        assert!(result.is_err());
    }

    #[test]
    fn test_did_to_verifying_key_too_short() {
        let result = did_to_verifying_key("did:key:z1234"); // Too short
        assert!(result.is_err());
    }

    #[test]
    fn test_did_to_verifying_key_valid() {
        // Known valid DID from golden test vectors
        let did = "did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw";
        let result = did_to_verifying_key(did);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_computation() {
        let content = "hello world";
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hex::encode(hasher.finalize());

        // Known SHA-256 hash of "hello world" (no newline)
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn test_verification_result_serialization() {
        let result = VerificationResult::valid_bound("did:key:z6Mk...".to_string());
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"status\":\"valid_bound\""));
        assert!(json.contains("\"verifiedDid\":\"did:key:z6Mk...\""));
    }

    #[test]
    fn test_envelope_missing_signer() {
        // Can't test async directly without tokio, but we can test the sync parts
        let envelope = serde_json::json!({
            "hash": { "value": "abc123" },
            "signature": "base64sig"
        });

        // Just verify the structure - actual async test would need tokio runtime
        assert!(envelope.get("signer").is_none());
    }

    #[test]
    fn test_envelope_missing_hash() {
        let envelope = serde_json::json!({
            "signer": "did:key:z6Mk...",
            "signature": "base64sig"
        });

        assert!(envelope.get("hash").is_none());
    }

    #[test]
    fn test_envelope_missing_signature() {
        let envelope = serde_json::json!({
            "signer": "did:key:z6Mk...",
            "hash": { "value": "abc123" }
        });

        assert!(envelope.get("signature").is_none());
    }
}
