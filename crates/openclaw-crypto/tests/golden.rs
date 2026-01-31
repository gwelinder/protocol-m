// Golden vector integration test for Protocol M
//
// This test validates the implementation against the canonical test vector
// defined in fixtures/golden_vectors.json. If any value differs, the test
// MUST fail - this ensures cross-implementation compatibility.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use openclaw_crypto::{
    jcs_canonical_bytes, pubkey_to_did, sha256_hex, ArtifactInfo, HashRef, SignatureEnvelopeV1,
};
use serde::Deserialize;
use std::fs;

/// Structure for parsing the golden vector file
#[derive(Deserialize)]
struct GoldenVector {
    seed_hex: String,
    public_key_hex: String,
    did: String,
    file_bytes_utf8: String,
    file_size: u64,
    sha256_hex: String,
    timestamp: String,
    artifact_name: String,
    canonical_jcs: String,
    signature_base64: String,
}

fn load_golden_vector() -> GoldenVector {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/golden_vectors.json");
    let content = fs::read_to_string(path).expect("Failed to read golden_vectors.json");
    serde_json::from_str(&content).expect("Failed to parse golden_vectors.json")
}

#[test]
fn test_keypair_from_seed() {
    let golden = load_golden_vector();

    // Decode seed hex
    let seed_bytes: [u8; 32] = hex::decode(&golden.seed_hex)
        .expect("valid seed hex")
        .try_into()
        .expect("32 bytes");

    // Generate keypair from seed
    let signing_key = SigningKey::from_bytes(&seed_bytes);
    let verifying_key = signing_key.verifying_key();

    // Verify public key matches expected
    let actual_pubkey_hex = hex::encode(verifying_key.as_bytes());
    assert_eq!(
        actual_pubkey_hex, golden.public_key_hex,
        "Public key derivation mismatch"
    );
}

#[test]
fn test_did_derivation() {
    let golden = load_golden_vector();

    // Decode seed and generate keypair
    let seed_bytes: [u8; 32] = hex::decode(&golden.seed_hex)
        .expect("valid seed hex")
        .try_into()
        .expect("32 bytes");
    let signing_key = SigningKey::from_bytes(&seed_bytes);
    let verifying_key = signing_key.verifying_key();

    // Derive DID
    let actual_did = pubkey_to_did(&verifying_key);

    assert_eq!(actual_did, golden.did, "DID derivation mismatch");
}

#[test]
fn test_file_hash() {
    let golden = load_golden_vector();

    // Hash the file content
    let actual_hash = sha256_hex(golden.file_bytes_utf8.as_bytes());

    assert_eq!(actual_hash, golden.sha256_hex, "File hash mismatch");
}

#[test]
fn test_envelope_canonicalization() {
    let golden = load_golden_vector();

    // Decode seed and derive DID
    let seed_bytes: [u8; 32] = hex::decode(&golden.seed_hex)
        .expect("valid seed hex")
        .try_into()
        .expect("32 bytes");
    let signing_key = SigningKey::from_bytes(&seed_bytes);
    let verifying_key = signing_key.verifying_key();
    let did = pubkey_to_did(&verifying_key);

    // Create envelope (with empty signature for canonicalization)
    let envelope = SignatureEnvelopeV1::new(
        did,
        golden.timestamp.clone(),
        HashRef {
            algo: "sha-256".to_string(),
            value: golden.sha256_hex.clone(),
        },
        ArtifactInfo {
            name: golden.artifact_name.clone(),
            size: golden.file_size,
        },
        None,
    );

    // Canonicalize
    let canonical_bytes = jcs_canonical_bytes(&envelope).expect("canonicalization");
    let actual_canonical = String::from_utf8(canonical_bytes).expect("utf8");

    assert_eq!(
        actual_canonical, golden.canonical_jcs,
        "JCS canonicalization mismatch"
    );
}

#[test]
fn test_signature_generation() {
    let golden = load_golden_vector();

    // Decode seed and derive keypair
    let seed_bytes: [u8; 32] = hex::decode(&golden.seed_hex)
        .expect("valid seed hex")
        .try_into()
        .expect("32 bytes");
    let signing_key = SigningKey::from_bytes(&seed_bytes);
    let verifying_key = signing_key.verifying_key();
    let did = pubkey_to_did(&verifying_key);

    // Create envelope
    let envelope = SignatureEnvelopeV1::new(
        did,
        golden.timestamp.clone(),
        HashRef {
            algo: "sha-256".to_string(),
            value: golden.sha256_hex.clone(),
        },
        ArtifactInfo {
            name: golden.artifact_name.clone(),
            size: golden.file_size,
        },
        None,
    );

    // Canonicalize and sign
    let canonical_bytes = jcs_canonical_bytes(&envelope).expect("canonicalization");
    let signature: Signature = signing_key.sign(&canonical_bytes);
    let actual_signature = BASE64_STANDARD.encode(signature.to_bytes());

    assert_eq!(
        actual_signature, golden.signature_base64,
        "Signature mismatch - this is critical for cross-implementation compatibility"
    );
}

#[test]
fn test_signature_verification() {
    let golden = load_golden_vector();

    // Decode seed and derive keypair
    let seed_bytes: [u8; 32] = hex::decode(&golden.seed_hex)
        .expect("valid seed hex")
        .try_into()
        .expect("32 bytes");
    let signing_key = SigningKey::from_bytes(&seed_bytes);
    let verifying_key = signing_key.verifying_key();
    let did = pubkey_to_did(&verifying_key);

    // Create envelope
    let envelope = SignatureEnvelopeV1::new(
        did,
        golden.timestamp.clone(),
        HashRef {
            algo: "sha-256".to_string(),
            value: golden.sha256_hex.clone(),
        },
        ArtifactInfo {
            name: golden.artifact_name.clone(),
            size: golden.file_size,
        },
        None,
    );

    // Canonicalize
    let canonical_bytes = jcs_canonical_bytes(&envelope).expect("canonicalization");

    // Decode the expected signature from the golden vector
    let signature_bytes = BASE64_STANDARD
        .decode(&golden.signature_base64)
        .expect("valid base64");
    let signature =
        Signature::from_bytes(&signature_bytes.try_into().expect("64 bytes"));

    // Verify the signature
    verifying_key
        .verify(&canonical_bytes, &signature)
        .expect("Signature verification should succeed with golden vector");
}
