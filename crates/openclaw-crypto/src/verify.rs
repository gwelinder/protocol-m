// Envelope verification logic for Protocol M
//
// This module provides signature verification for SignatureEnvelopeV1 structures.

use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::hash::sha256_hex;
use crate::jcs::jcs_canonical_bytes;
use crate::types::SignatureEnvelopeV1;

/// Verifies an artifact signature using the provided verifying key.
///
/// This function:
/// 1. Verifies envelope version/type/algo fields
/// 2. Recomputes file hash and compares to envelope
/// 3. Creates envelope copy with empty signature
/// 4. Canonicalizes the envelope with JCS
/// 5. Decodes the base64 signature
/// 6. Verifies the signature with ed25519_dalek
///
/// # Arguments
/// * `verifying_key` - The Ed25519 verifying key (public key)
/// * `file_bytes` - The raw bytes of the artifact
/// * `envelope` - The signature envelope to verify
///
/// # Returns
/// Ok(()) if verification succeeds, Err with details if it fails
pub fn verify_artifact(
    verifying_key: &VerifyingKey,
    file_bytes: &[u8],
    envelope: &SignatureEnvelopeV1,
) -> Result<()> {
    // Step 1: Verify envelope version/type/algo fields
    if envelope.version != "1.0" {
        return Err(anyhow!(
            "Unsupported envelope version: '{}' (expected '1.0')",
            envelope.version
        ));
    }

    if envelope.envelope_type != "signature-envelope" {
        return Err(anyhow!(
            "Invalid envelope type: '{}' (expected 'signature-envelope')",
            envelope.envelope_type
        ));
    }

    if envelope.algo != "ed25519" {
        return Err(anyhow!(
            "Unsupported signature algorithm: '{}' (expected 'ed25519')",
            envelope.algo
        ));
    }

    // Step 2: Recompute file hash and compare to envelope
    let computed_hash = sha256_hex(file_bytes);
    if envelope.hash.value != computed_hash {
        return Err(anyhow!(
            "Hash mismatch: file content does not match envelope hash.\n\
             Expected: {}\n\
             Computed: {}",
            envelope.hash.value,
            computed_hash
        ));
    }

    if envelope.hash.algo != "sha-256" {
        return Err(anyhow!(
            "Unsupported hash algorithm: '{}' (expected 'sha-256')",
            envelope.hash.algo
        ));
    }

    // Verify file size matches
    if envelope.artifact.size != file_bytes.len() as u64 {
        return Err(anyhow!(
            "Size mismatch: expected {} bytes, got {} bytes",
            envelope.artifact.size,
            file_bytes.len()
        ));
    }

    // Step 3: Create envelope copy with empty signature
    let mut verify_envelope = envelope.clone();
    verify_envelope.signature = String::new();

    // Step 4: Canonicalize envelope with JCS
    let canonical_bytes = jcs_canonical_bytes(&verify_envelope)?;

    // Step 5: Decode base64 signature
    let signature_bytes = BASE64_STANDARD
        .decode(&envelope.signature)
        .map_err(|e| anyhow!("Invalid base64 signature: {}", e))?;

    let signature_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| anyhow!("Invalid signature length: expected 64 bytes"))?;

    let signature = Signature::from_bytes(&signature_array);

    // Step 6: Verify signature with ed25519_dalek
    verifying_key
        .verify(&canonical_bytes, &signature)
        .map_err(|_| anyhow!("Signature verification failed: invalid signature"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{pubkey_to_did, sign_artifact};
    use ed25519_dalek::SigningKey;

    fn create_test_envelope() -> (VerifyingKey, Vec<u8>, SignatureEnvelopeV1) {
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
            None,
        )
        .expect("signing should succeed");

        (verifying_key, file_content.to_vec(), envelope)
    }

    #[test]
    fn test_verify_valid_signature() {
        let (verifying_key, file_bytes, envelope) = create_test_envelope();

        let result = verify_artifact(&verifying_key, &file_bytes, &envelope);
        assert!(result.is_ok(), "Valid signature should verify: {:?}", result);
    }

    #[test]
    fn test_verify_fails_on_wrong_content() {
        let (verifying_key, _, envelope) = create_test_envelope();

        let tampered_content = b"tampered content";
        let result = verify_artifact(&verifying_key, tampered_content, &envelope);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Hash mismatch"));
    }

    #[test]
    fn test_verify_fails_on_wrong_key() {
        let (_, file_bytes, envelope) = create_test_envelope();

        // Use a different key for verification
        let different_seed: [u8; 32] = [0x99; 32];
        let different_key = SigningKey::from_bytes(&different_seed).verifying_key();

        let result = verify_artifact(&different_key, &file_bytes, &envelope);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Signature verification failed"));
    }

    #[test]
    fn test_verify_fails_on_unsupported_version() {
        let (verifying_key, file_bytes, mut envelope) = create_test_envelope();
        envelope.version = "2.0".to_string();

        let result = verify_artifact(&verifying_key, &file_bytes, &envelope);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported envelope version"));
    }

    #[test]
    fn test_verify_fails_on_invalid_envelope_type() {
        let (verifying_key, file_bytes, mut envelope) = create_test_envelope();
        envelope.envelope_type = "invalid-type".to_string();

        let result = verify_artifact(&verifying_key, &file_bytes, &envelope);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid envelope type"));
    }

    #[test]
    fn test_verify_fails_on_invalid_algo() {
        let (verifying_key, file_bytes, mut envelope) = create_test_envelope();
        envelope.algo = "rsa".to_string();

        let result = verify_artifact(&verifying_key, &file_bytes, &envelope);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported signature algorithm"));
    }

    #[test]
    fn test_verify_fails_on_size_mismatch() {
        let (verifying_key, file_bytes, mut envelope) = create_test_envelope();
        envelope.artifact.size = 9999;

        let result = verify_artifact(&verifying_key, &file_bytes, &envelope);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Size mismatch"));
    }

    #[test]
    fn test_verify_fails_on_invalid_base64_signature() {
        let (verifying_key, file_bytes, mut envelope) = create_test_envelope();
        envelope.signature = "not-valid-base64!!!".to_string();

        let result = verify_artifact(&verifying_key, &file_bytes, &envelope);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid base64 signature"));
    }

    #[test]
    fn test_verify_fails_on_wrong_signature_length() {
        let (verifying_key, file_bytes, mut envelope) = create_test_envelope();
        envelope.signature = BASE64_STANDARD.encode(vec![0u8; 32]); // Only 32 bytes instead of 64

        let result = verify_artifact(&verifying_key, &file_bytes, &envelope);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid signature length"));
    }
}
