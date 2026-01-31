// Envelope signing logic for Protocol M
//
// This module provides the core signing functionality that creates
// SignatureEnvelopeV1 structures from artifacts.

use anyhow::Result;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};

use crate::hash::sha256_hex;
use crate::jcs::jcs_canonical_bytes;
use crate::types::{ArtifactInfo, HashRef, SignatureEnvelopeV1};

/// Signs an artifact and returns a complete SignatureEnvelopeV1.
///
/// This function:
/// 1. Computes the SHA-256 hash of the file bytes
/// 2. Constructs an envelope with an empty signature field
/// 3. Canonicalizes the envelope using JCS (RFC 8785)
/// 4. Signs the canonical bytes with Ed25519
/// 5. Inserts the base64-encoded signature into the envelope
///
/// # Arguments
/// * `signing_key` - The Ed25519 signing key
/// * `did` - The signer's DID (did:key format)
/// * `filename` - Name of the artifact
/// * `bytes` - Raw bytes of the artifact
/// * `timestamp` - ISO 8601 timestamp string
/// * `metadata` - Optional metadata to include in the envelope
///
/// # Returns
/// A complete SignatureEnvelopeV1 with a valid signature
pub fn sign_artifact(
    signing_key: &SigningKey,
    did: String,
    filename: String,
    bytes: &[u8],
    timestamp: String,
    metadata: Option<serde_json::Value>,
) -> Result<SignatureEnvelopeV1> {
    // Step 1: Compute SHA-256 hash of file bytes
    let hash_value = sha256_hex(bytes);

    // Step 2: Construct envelope with empty signature
    let mut envelope = SignatureEnvelopeV1::new(
        did,
        timestamp,
        HashRef {
            algo: "sha-256".to_string(),
            value: hash_value,
        },
        ArtifactInfo {
            name: filename,
            size: bytes.len() as u64,
        },
        metadata,
    );

    // Step 3: Canonicalize envelope with JCS
    let canonical_bytes = jcs_canonical_bytes(&envelope)?;

    // Step 4: Sign canonical bytes with Ed25519
    let signature = signing_key.sign(&canonical_bytes);

    // Step 5: Insert base64-encoded signature into envelope
    envelope.signature = BASE64_STANDARD.encode(signature.to_bytes());

    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pubkey_to_did;
    use ed25519_dalek::Verifier;

    #[test]
    fn test_sign_artifact_produces_valid_signature() {
        // Create a deterministic keypair
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let did = pubkey_to_did(&verifying_key);

        // Sign an artifact
        let content = b"test file content";
        let envelope = sign_artifact(
            &signing_key,
            did.clone(),
            "test.txt".to_string(),
            content,
            "2026-01-30T12:00:00Z".to_string(),
            None,
        )
        .expect("signing should succeed");

        // Verify envelope fields
        assert_eq!(envelope.version, "1.0");
        assert_eq!(envelope.envelope_type, "signature-envelope");
        assert_eq!(envelope.algo, "ed25519");
        assert_eq!(envelope.signer, did);
        assert_eq!(envelope.hash.algo, "sha-256");
        assert_eq!(envelope.artifact.name, "test.txt");
        assert_eq!(envelope.artifact.size, content.len() as u64);
        assert!(!envelope.signature.is_empty());

        // Verify the signature is valid
        let mut verify_envelope = envelope.clone();
        verify_envelope.signature = String::new();
        let canonical_bytes = jcs_canonical_bytes(&verify_envelope).expect("canonicalize");
        let sig_bytes = BASE64_STANDARD
            .decode(&envelope.signature)
            .expect("decode base64");
        let signature =
            ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().expect("64 bytes"));
        verifying_key
            .verify(&canonical_bytes, &signature)
            .expect("signature should verify");
    }

    #[test]
    fn test_sign_artifact_with_metadata() {
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let did = pubkey_to_did(&verifying_key);

        let metadata = serde_json::json!({
            "author": "Alice",
            "version": "1.0.0"
        });

        let envelope = sign_artifact(
            &signing_key,
            did,
            "app.wasm".to_string(),
            b"binary content",
            "2026-01-30T12:00:00Z".to_string(),
            Some(metadata.clone()),
        )
        .expect("signing should succeed");

        assert_eq!(envelope.metadata, Some(metadata));
    }

    #[test]
    fn test_sign_artifact_computes_correct_hash() {
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let did = pubkey_to_did(&verifying_key);

        // Use content with known hash
        let content = b"hello";
        let expected_hash = sha256_hex(content);

        let envelope = sign_artifact(
            &signing_key,
            did,
            "hello.txt".to_string(),
            content,
            "2026-01-30T12:00:00Z".to_string(),
            None,
        )
        .expect("signing should succeed");

        assert_eq!(envelope.hash.value, expected_hash);
    }

    #[test]
    fn test_different_content_produces_different_signature() {
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let did = pubkey_to_did(&verifying_key);

        let envelope1 = sign_artifact(
            &signing_key,
            did.clone(),
            "file.txt".to_string(),
            b"content A",
            "2026-01-30T12:00:00Z".to_string(),
            None,
        )
        .expect("signing should succeed");

        let envelope2 = sign_artifact(
            &signing_key,
            did,
            "file.txt".to_string(),
            b"content B",
            "2026-01-30T12:00:00Z".to_string(),
            None,
        )
        .expect("signing should succeed");

        assert_ne!(envelope1.signature, envelope2.signature);
        assert_ne!(envelope1.hash.value, envelope2.hash.value);
    }
}
