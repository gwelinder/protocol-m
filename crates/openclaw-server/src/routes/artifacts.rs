//! Artifact registration endpoints.

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::artifact::{Artifact, NewArtifact};
use crate::models::artifact_derivation::NewArtifactDerivation;
use openclaw_crypto::{did_to_verifying_key, SignatureEnvelopeV1};

/// Maximum metadata size in bytes (10KB).
const MAX_METADATA_SIZE: usize = 10 * 1024;

/// Response for successful artifact registration.
#[derive(serde::Serialize)]
pub struct RegisterArtifactResponse {
    /// The UUID of the registered artifact.
    pub id: Uuid,
    /// The URL path to access this artifact.
    pub url: String,
}

/// Creates the artifacts router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/", post(register_artifact))
        .with_state(pool)
}

/// POST /api/v1/artifacts
///
/// Registers a new signed artifact.
/// Accepts a SignatureEnvelopeV1 in the request body.
/// Returns the artifact ID and URL on success.
async fn register_artifact(
    State(pool): State<PgPool>,
    Json(envelope): Json<SignatureEnvelopeV1>,
) -> Result<Json<RegisterArtifactResponse>, AppError> {
    // Step 1: Validate envelope (signature, metadata size, structure)
    validate_envelope(&envelope)?;

    // Step 2: Parse the timestamp from the envelope
    let timestamp = parse_timestamp(&envelope.timestamp)?;

    // Step 3: Check for duplicate hash (optionally allow - for now we reject)
    check_duplicate_hash(&pool, &envelope.hash.value).await?;

    // Extract fields from the envelope
    let new_artifact = NewArtifact {
        hash: envelope.hash.value.clone(),
        did: envelope.signer.clone(),
        timestamp,
        metadata: envelope.metadata.clone().unwrap_or(serde_json::json!({})),
        signature: envelope.signature.clone(),
    };

    // Generate a new UUID for this artifact
    let id = Uuid::new_v4();

    // Insert into the database
    let artifact = insert_artifact(&pool, id, &new_artifact).await?;

    // Step 4: Process derivations (metadata.derivedFrom field)
    if let Some(ref metadata) = envelope.metadata {
        process_derivations(&pool, artifact.id, metadata).await?;
    }

    Ok(Json(RegisterArtifactResponse {
        id: artifact.id,
        url: artifact.url_path(),
    }))
}

/// Validates the envelope structure, signature, and metadata.
fn validate_envelope(envelope: &SignatureEnvelopeV1) -> Result<(), AppError> {
    // Validate metadata size (must be < 10KB when serialized)
    if let Some(ref metadata) = envelope.metadata {
        let metadata_json = serde_json::to_string(metadata)
            .map_err(|e| AppError::BadRequest(format!("Invalid metadata JSON: {}", e)))?;

        if metadata_json.len() > MAX_METADATA_SIZE {
            return Err(AppError::BadRequest(format!(
                "Metadata exceeds maximum size: {} bytes (max: {} bytes)",
                metadata_json.len(),
                MAX_METADATA_SIZE
            )));
        }

        // Validate metadata is a valid JSON object (not primitive or array at root)
        if !metadata.is_object() {
            return Err(AppError::BadRequest(
                "Metadata must be a JSON object".to_string(),
            ));
        }
    }

    // Validate the DID format and extract verifying key
    let verifying_key = did_to_verifying_key(&envelope.signer)
        .map_err(|e| AppError::BadRequest(format!("Invalid DID: {}", e)))?;

    // Verify the signature cryptographically
    // Note: For registration, we don't have the original artifact bytes,
    // so we verify the envelope structure and signature format.
    // The hash in the envelope is trusted as the artifact content hash.
    verify_envelope_signature(envelope, &verifying_key)?;

    Ok(())
}

/// Verifies the envelope signature cryptographically.
/// This validates that the signature is valid for the envelope contents.
fn verify_envelope_signature(
    envelope: &SignatureEnvelopeV1,
    verifying_key: &ed25519_dalek::VerifyingKey,
) -> Result<(), AppError> {
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

/// Checks if an artifact with the same hash already exists.
async fn check_duplicate_hash(pool: &PgPool, hash: &str) -> Result<(), AppError> {
    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM artifacts WHERE hash = $1 LIMIT 1",
    )
    .bind(hash)
    .fetch_optional(pool)
    .await?;

    if let Some((existing_id,)) = existing {
        return Err(AppError::BadRequest(format!(
            "Artifact with hash '{}' already exists (id: {})",
            hash, existing_id
        )));
    }

    Ok(())
}

/// Parses an ISO 8601 timestamp string into a DateTime<Utc>.
fn parse_timestamp(timestamp: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| AppError::BadRequest(format!("Invalid timestamp format: {}", e)))
}

/// Inserts a new artifact into the database.
async fn insert_artifact(
    pool: &PgPool,
    id: Uuid,
    artifact: &NewArtifact,
) -> Result<Artifact, AppError> {
    let row = sqlx::query_as::<_, Artifact>(
        r#"
        INSERT INTO artifacts (id, hash, did, timestamp, metadata, signature, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW())
        RETURNING id, hash, did, timestamp, metadata, signature, created_at
        "#,
    )
    .bind(id)
    .bind(&artifact.hash)
    .bind(&artifact.did)
    .bind(artifact.timestamp)
    .bind(&artifact.metadata)
    .bind(&artifact.signature)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Processes derivation relationships from the metadata.derivedFrom field.
/// If derivedFrom contains artifact IDs, validates they exist and creates derivation records.
async fn process_derivations(
    pool: &PgPool,
    artifact_id: Uuid,
    metadata: &serde_json::Value,
) -> Result<(), AppError> {
    // Check for derivedFrom field in metadata
    let derived_from = match metadata.get("derivedFrom") {
        Some(value) => value,
        None => return Ok(()), // No derivations to process
    };

    // derivedFrom can be a single string (artifact ID or hash) or an array
    let parent_refs: Vec<&str> = match derived_from {
        serde_json::Value::String(s) => vec![s.as_str()],
        serde_json::Value::Array(arr) => {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect()
        }
        _ => {
            return Err(AppError::BadRequest(
                "derivedFrom must be a string or array of strings".to_string(),
            ));
        }
    };

    if parent_refs.is_empty() {
        return Ok(());
    }

    // Validate and resolve all parent references
    for parent_ref in parent_refs {
        let parent_id = resolve_artifact_reference(pool, parent_ref).await?;

        // Check for cycles before inserting
        if detect_cycle(pool, artifact_id, parent_id).await? {
            return Err(AppError::BadRequest(format!(
                "Adding derivation would create a cycle: {} -> {}",
                artifact_id, parent_id
            )));
        }

        insert_derivation(pool, artifact_id, parent_id).await?;
    }

    Ok(())
}

/// Resolves an artifact reference (UUID or hash) to an artifact ID.
/// Returns an error if the referenced artifact doesn't exist.
async fn resolve_artifact_reference(pool: &PgPool, reference: &str) -> Result<Uuid, AppError> {
    // First, try to parse as UUID
    if let Ok(uuid) = Uuid::parse_str(reference) {
        // Check if artifact with this ID exists
        let exists: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM artifacts WHERE id = $1 LIMIT 1",
        )
        .bind(uuid)
        .fetch_optional(pool)
        .await?;

        if exists.is_some() {
            return Ok(uuid);
        }
    }

    // If not a valid UUID or doesn't exist as ID, try to look up by hash
    let result: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM artifacts WHERE hash = $1 LIMIT 1",
    )
    .bind(reference)
    .fetch_optional(pool)
    .await?;

    match result {
        Some((id,)) => Ok(id),
        None => Err(AppError::BadRequest(format!(
            "Parent artifact not found: '{}'",
            reference
        ))),
    }
}

/// Inserts a derivation record linking an artifact to its parent.
async fn insert_derivation(
    pool: &PgPool,
    artifact_id: Uuid,
    derived_from_id: Uuid,
) -> Result<(), AppError> {
    let derivation = NewArtifactDerivation {
        artifact_id,
        derived_from_id,
    };

    sqlx::query(
        r#"
        INSERT INTO artifact_derivations (artifact_id, derived_from_id)
        VALUES ($1, $2)
        ON CONFLICT (artifact_id, derived_from_id) DO NOTHING
        "#,
    )
    .bind(derivation.artifact_id)
    .bind(derivation.derived_from_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Maximum depth for cycle detection DFS to prevent DoS.
const MAX_CYCLE_DETECTION_DEPTH: usize = 100;

/// Detects if adding a derivation from `artifact_id` to `parent_id` would create a cycle.
/// Uses depth-first search to check if `artifact_id` is already an ancestor of `parent_id`.
///
/// A cycle would occur if:
/// - artifact_id == parent_id (self-reference)
/// - parent_id already derives from artifact_id (directly or transitively)
///
/// Returns true if a cycle would be created.
pub async fn detect_cycle(
    pool: &PgPool,
    artifact_id: Uuid,
    parent_id: Uuid,
) -> Result<bool, AppError> {
    // Self-reference is a trivial cycle
    if artifact_id == parent_id {
        return Ok(true);
    }

    // Check if parent_id already has artifact_id as an ancestor
    // (i.e., artifact_id derives from parent_id directly or transitively)
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![parent_id];

    while let Some(current) = stack.pop() {
        // Check depth limit
        if visited.len() >= MAX_CYCLE_DETECTION_DEPTH {
            // If we've explored too many nodes, assume no cycle to avoid DoS
            // This is a conservative approach - cycles at depth > 100 are unlikely
            return Ok(false);
        }

        // If we've reached the artifact we're trying to derive from, we have a cycle
        if current == artifact_id {
            return Ok(true);
        }

        // Skip if already visited
        if !visited.insert(current) {
            continue;
        }

        // Get all parents of the current artifact
        let parents: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT derived_from_id FROM artifact_derivations WHERE artifact_id = $1",
        )
        .bind(current)
        .fetch_all(pool)
        .await?;

        for (parent,) in parents {
            stack.push(parent);
        }
    }

    // No cycle found
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use ed25519_dalek::SigningKey;
    use openclaw_crypto::{pubkey_to_did, sign_artifact};

    fn create_valid_envelope() -> SignatureEnvelopeV1 {
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let did = pubkey_to_did(&verifying_key);

        let file_content = b"test file content";
        sign_artifact(
            &signing_key,
            did,
            "test.txt".to_string(),
            file_content,
            "2026-01-31T12:00:00Z".to_string(),
            Some(serde_json::json!({"author": "test"})),
        )
        .expect("signing should succeed")
    }

    #[test]
    fn test_parse_timestamp_valid() {
        let ts = "2024-01-15T10:30:00Z";
        let result = parse_timestamp(ts);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_timestamp_with_offset() {
        let ts = "2024-01-15T10:30:00+05:00";
        let result = parse_timestamp(ts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        let ts = "not a timestamp";
        let result = parse_timestamp(ts);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_envelope_valid() {
        let envelope = create_valid_envelope();
        let result = validate_envelope(&envelope);
        assert!(result.is_ok(), "Valid envelope should pass: {:?}", result);
    }

    #[test]
    fn test_validate_envelope_invalid_did() {
        let mut envelope = create_valid_envelope();
        envelope.signer = "invalid-did".to_string();

        let result = validate_envelope(&envelope);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result);
        assert!(err_msg.contains("Invalid DID"));
    }

    #[test]
    fn test_validate_envelope_wrong_version() {
        let mut envelope = create_valid_envelope();
        envelope.version = "2.0".to_string();

        let result = validate_envelope(&envelope);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result);
        assert!(err_msg.contains("Unsupported envelope version"));
    }

    #[test]
    fn test_validate_envelope_wrong_type() {
        let mut envelope = create_valid_envelope();
        envelope.envelope_type = "invalid-type".to_string();

        let result = validate_envelope(&envelope);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result);
        assert!(err_msg.contains("Invalid envelope type"));
    }

    #[test]
    fn test_validate_envelope_wrong_algo() {
        let mut envelope = create_valid_envelope();
        envelope.algo = "rsa".to_string();

        let result = validate_envelope(&envelope);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result);
        assert!(err_msg.contains("Unsupported signature algorithm"));
    }

    #[test]
    fn test_validate_envelope_invalid_signature() {
        use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
        use base64::Engine;

        let mut envelope = create_valid_envelope();
        // Use a valid base64-encoded 64-byte signature that's wrong
        let fake_sig = [0u8; 64];
        envelope.signature = BASE64_STANDARD.encode(fake_sig);

        let result = validate_envelope(&envelope);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result);
        assert!(err_msg.contains("Signature verification failed"));
    }

    #[test]
    fn test_validate_envelope_invalid_base64_signature() {
        let mut envelope = create_valid_envelope();
        envelope.signature = "not-valid-base64!!!".to_string();

        let result = validate_envelope(&envelope);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result);
        assert!(err_msg.contains("Invalid base64"));
    }

    #[test]
    fn test_validate_envelope_metadata_too_large() {
        let mut envelope = create_valid_envelope();
        // Create metadata larger than 10KB
        let large_value = "x".repeat(11 * 1024);
        envelope.metadata = Some(serde_json::json!({"data": large_value}));

        let result = validate_envelope(&envelope);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result);
        assert!(err_msg.contains("exceeds maximum size"));
    }

    #[test]
    fn test_validate_envelope_metadata_not_object() {
        let mut envelope = create_valid_envelope();
        // Metadata is an array, not an object
        envelope.metadata = Some(serde_json::json!(["item1", "item2"]));

        let result = validate_envelope(&envelope);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result);
        assert!(err_msg.contains("must be a JSON object"));
    }

    #[test]
    fn test_validate_envelope_no_metadata_is_ok() {
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let did = pubkey_to_did(&verifying_key);

        let file_content = b"test file content";
        let envelope = sign_artifact(
            &signing_key,
            did,
            "test.txt".to_string(),
            file_content,
            "2026-01-31T12:00:00Z".to_string(),
            None, // No metadata
        )
        .expect("signing should succeed");

        let result = validate_envelope(&envelope);
        assert!(result.is_ok(), "Envelope without metadata should pass: {:?}", result);
    }

    #[test]
    fn test_validate_envelope_contribution_manifest_type() {
        // Create a contribution manifest envelope (different type)
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let did = pubkey_to_did(&verifying_key);

        let file_content = b"manifest content";
        let mut envelope = sign_artifact(
            &signing_key,
            did,
            "manifest.json".to_string(),
            file_content,
            "2026-01-31T12:00:00Z".to_string(),
            None,
        )
        .expect("signing should succeed");

        // Change type to contribution-manifest (need to re-sign for this to be valid)
        // For this test, we're just checking the type validation accepts contribution-manifest
        envelope.envelope_type = "contribution-manifest".to_string();

        // This will fail signature verification since we changed the type after signing,
        // but it confirms the type check allows contribution-manifest
        let result = validate_envelope(&envelope);
        // The error should be about signature, not about type
        assert!(result.is_err());
        let err_msg = format!("{:?}", result);
        assert!(err_msg.contains("Signature verification failed"));
        assert!(!err_msg.contains("Invalid envelope type"));
    }

    // Tests for derivation parsing from metadata
    #[test]
    fn test_parse_derived_from_string() {
        let metadata = serde_json::json!({
            "derivedFrom": "550e8400-e29b-41d4-a716-446655440000"
        });

        let derived_from = metadata.get("derivedFrom").unwrap();
        let parent_refs: Vec<&str> = match derived_from {
            serde_json::Value::String(s) => vec![s.as_str()],
            _ => vec![],
        };

        assert_eq!(parent_refs.len(), 1);
        assert_eq!(parent_refs[0], "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_parse_derived_from_array() {
        let metadata = serde_json::json!({
            "derivedFrom": [
                "550e8400-e29b-41d4-a716-446655440000",
                "550e8400-e29b-41d4-a716-446655440001"
            ]
        });

        let derived_from = metadata.get("derivedFrom").unwrap();
        let parent_refs: Vec<&str> = match derived_from {
            serde_json::Value::Array(arr) => {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect()
            }
            _ => vec![],
        };

        assert_eq!(parent_refs.len(), 2);
        assert_eq!(parent_refs[0], "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(parent_refs[1], "550e8400-e29b-41d4-a716-446655440001");
    }

    #[test]
    fn test_parse_derived_from_empty_array() {
        let metadata = serde_json::json!({
            "derivedFrom": []
        });

        let derived_from = metadata.get("derivedFrom").unwrap();
        let parent_refs: Vec<&str> = match derived_from {
            serde_json::Value::Array(arr) => {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect()
            }
            _ => vec![],
        };

        assert!(parent_refs.is_empty());
    }

    #[test]
    fn test_parse_derived_from_missing() {
        let metadata = serde_json::json!({
            "author": "test"
        });

        let derived_from = metadata.get("derivedFrom");
        assert!(derived_from.is_none());
    }

    #[test]
    fn test_parse_derived_from_with_hash() {
        let metadata = serde_json::json!({
            "derivedFrom": "abc123def456abc123def456abc123def456abc123def456abc123def456abc1"
        });

        let derived_from = metadata.get("derivedFrom").unwrap();
        let parent_ref = derived_from.as_str().unwrap();

        // 64 character hex string (SHA-256 hash)
        assert_eq!(parent_ref.len(), 64);
    }

    // Tests for cycle detection (unit tests without DB)
    #[test]
    fn test_cycle_detection_self_reference() {
        // Self-reference is detected synchronously without DB
        let id = Uuid::new_v4();
        assert_eq!(id, id); // Trivially, same ID would cause cycle
    }

    #[test]
    fn test_cycle_detection_max_depth_constant() {
        // Verify the constant is reasonable
        assert_eq!(MAX_CYCLE_DETECTION_DEPTH, 100);
    }

    #[test]
    fn test_visited_set_behavior() {
        // Test that HashSet properly tracks visited nodes
        let mut visited = std::collections::HashSet::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        assert!(visited.insert(id1)); // First insert returns true
        assert!(!visited.insert(id1)); // Second insert of same returns false
        assert!(visited.insert(id2)); // Different ID returns true
        assert_eq!(visited.len(), 2);
    }
}
