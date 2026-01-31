// Manifest export logic for Protocol M
//
// This module provides functionality to aggregate signed artifacts into
// a portable contribution manifest.

use anyhow::Result;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};

use crate::hash::sha256_hex;
use crate::jcs::jcs_canonical_bytes;
use crate::types::{ArtifactInfo, ArtifactReference, ContributionManifest, HashRef, SignatureEnvelopeV1};

/// Exports a contribution manifest as a signed envelope.
///
/// This function:
/// 1. Constructs a ContributionManifest with the provided artifact references
/// 2. Serializes the manifest to JSON
/// 3. Computes the SHA-256 hash of the serialized manifest
/// 4. Creates a SignatureEnvelopeV1 wrapping the manifest
/// 5. Signs the envelope and returns it
///
/// # Arguments
/// * `signing_key` - The Ed25519 signing key for signing the manifest
/// * `did` - The signer's DID (did:key format)
/// * `artifact_refs` - List of artifact references to include in the manifest
/// * `timestamp` - ISO 8601 timestamp for the manifest
///
/// # Returns
/// A SignatureEnvelopeV1 containing the signed manifest
pub fn export_manifest(
    signing_key: &SigningKey,
    did: String,
    artifact_refs: Vec<ArtifactReference>,
    timestamp: String,
) -> Result<SignatureEnvelopeV1> {
    // Step 1: Construct the contribution manifest
    let manifest = ContributionManifest::new(did.clone(), timestamp.clone(), artifact_refs);

    // Step 2: Serialize manifest to JSON bytes
    let manifest_json = serde_json::to_string(&manifest)?;
    let manifest_bytes = manifest_json.as_bytes();

    // Step 3: Compute SHA-256 hash of manifest
    let hash_value = sha256_hex(manifest_bytes);

    // Step 4: Create envelope with empty signature
    let mut envelope = SignatureEnvelopeV1 {
        version: "1.0".to_string(),
        envelope_type: "contribution-manifest".to_string(),
        algo: "ed25519".to_string(),
        signer: did,
        timestamp,
        hash: HashRef {
            algo: "sha-256".to_string(),
            value: hash_value,
        },
        artifact: ArtifactInfo {
            name: "manifest.json".to_string(),
            size: manifest_bytes.len() as u64,
        },
        metadata: Some(serde_json::to_value(&manifest)?),
        signature: String::new(),
    };

    // Step 5: Canonicalize and sign
    let canonical_bytes = jcs_canonical_bytes(&envelope)?;
    let signature = signing_key.sign(&canonical_bytes);
    envelope.signature = BASE64_STANDARD.encode(signature.to_bytes());

    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pubkey_to_did;
    use ed25519_dalek::Verifier;

    fn create_test_keypair() -> (SigningKey, String) {
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let did = pubkey_to_did(&verifying_key);
        (signing_key, did)
    }

    #[test]
    fn test_export_manifest_creates_valid_envelope() {
        let (signing_key, did) = create_test_keypair();

        let artifact_refs = vec![
            ArtifactReference::new(
                "abc123".to_string(),
                "sig1".to_string(),
                "2026-01-30T10:00:00Z".to_string(),
                None,
            ),
        ];

        let envelope = export_manifest(
            &signing_key,
            did.clone(),
            artifact_refs,
            "2026-01-30T12:00:00Z".to_string(),
        )
        .expect("export should succeed");

        assert_eq!(envelope.version, "1.0");
        assert_eq!(envelope.envelope_type, "contribution-manifest");
        assert_eq!(envelope.algo, "ed25519");
        assert_eq!(envelope.signer, did);
        assert_eq!(envelope.hash.algo, "sha-256");
        assert_eq!(envelope.artifact.name, "manifest.json");
        assert!(!envelope.signature.is_empty());
    }

    #[test]
    fn test_export_manifest_includes_artifacts_in_metadata() {
        let (signing_key, did) = create_test_keypair();

        let artifact_refs = vec![
            ArtifactReference::new(
                "hash1".to_string(),
                "sig1".to_string(),
                "2026-01-30T10:00:00Z".to_string(),
                Some(serde_json::json!({"author": "alice"})),
            ),
            ArtifactReference::new(
                "hash2".to_string(),
                "sig2".to_string(),
                "2026-01-30T11:00:00Z".to_string(),
                None,
            ),
        ];

        let envelope = export_manifest(
            &signing_key,
            did.clone(),
            artifact_refs,
            "2026-01-30T12:00:00Z".to_string(),
        )
        .expect("export should succeed");

        // Verify metadata contains the manifest
        let metadata = envelope.metadata.as_ref().expect("metadata should exist");
        let manifest: ContributionManifest =
            serde_json::from_value(metadata.clone()).expect("should parse as manifest");

        assert_eq!(manifest.did, did);
        assert_eq!(manifest.artifacts.len(), 2);
        assert_eq!(manifest.artifacts[0].hash, "hash1");
        assert_eq!(manifest.artifacts[1].hash, "hash2");
    }

    #[test]
    fn test_export_manifest_signature_is_valid() {
        let (signing_key, did) = create_test_keypair();
        let verifying_key = signing_key.verifying_key();

        let artifact_refs = vec![ArtifactReference::new(
            "abc123".to_string(),
            "sig1".to_string(),
            "2026-01-30T10:00:00Z".to_string(),
            None,
        )];

        let envelope = export_manifest(
            &signing_key,
            did,
            artifact_refs,
            "2026-01-30T12:00:00Z".to_string(),
        )
        .expect("export should succeed");

        // Verify signature by re-canonicalizing
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
    fn test_export_manifest_with_empty_artifacts() {
        let (signing_key, did) = create_test_keypair();

        let envelope = export_manifest(
            &signing_key,
            did.clone(),
            vec![],
            "2026-01-30T12:00:00Z".to_string(),
        )
        .expect("export should succeed");

        let metadata = envelope.metadata.as_ref().expect("metadata should exist");
        let manifest: ContributionManifest =
            serde_json::from_value(metadata.clone()).expect("should parse");

        assert_eq!(manifest.artifacts.len(), 0);
        assert_eq!(manifest.did, did);
    }

    #[test]
    fn test_export_manifest_hash_matches_content() {
        let (signing_key, did) = create_test_keypair();

        let artifact_refs = vec![ArtifactReference::new(
            "hash1".to_string(),
            "sig1".to_string(),
            "2026-01-30T10:00:00Z".to_string(),
            None,
        )];

        let envelope = export_manifest(
            &signing_key,
            did.clone(),
            artifact_refs.clone(),
            "2026-01-30T12:00:00Z".to_string(),
        )
        .expect("export should succeed");

        // Reconstruct manifest and verify hash
        let manifest = ContributionManifest::new(
            did,
            "2026-01-30T12:00:00Z".to_string(),
            artifact_refs,
        );
        let manifest_json = serde_json::to_string(&manifest).expect("serialize");
        let expected_hash = sha256_hex(manifest_json.as_bytes());

        assert_eq!(envelope.hash.value, expected_hash);
        assert_eq!(envelope.artifact.size, manifest_json.len() as u64);
    }
}
