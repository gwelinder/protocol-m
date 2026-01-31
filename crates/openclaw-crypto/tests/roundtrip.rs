// Roundtrip test for signing and verification
//
// This test validates that:
// 1. A new identity can generate a valid signature
// 2. The signature can be immediately verified
// 3. Tampered content fails verification

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier};
use openclaw_crypto::{
    generate_keypair, jcs_canonical_bytes, pubkey_to_did, sha256_hex, sign_artifact,
};

#[test]
fn test_signing_roundtrip() {
    // Step 1: Generate a new identity
    let (signing_key, verifying_key) = generate_keypair();
    let did = pubkey_to_did(&verifying_key);

    // Step 2: Sign a test file
    let file_content = b"This is a test file for roundtrip signing.";
    let filename = "test_file.txt".to_string();
    let timestamp = "2026-01-31T12:00:00Z".to_string();

    let envelope = sign_artifact(
        &signing_key,
        did.clone(),
        filename.clone(),
        file_content,
        timestamp.clone(),
        None,
    )
    .expect("Signing should succeed");

    // Step 3: Verify the signature immediately
    // 3a. Verify envelope metadata
    assert_eq!(envelope.version, "1.0");
    assert_eq!(envelope.envelope_type, "signature-envelope");
    assert_eq!(envelope.algo, "ed25519");
    assert_eq!(envelope.signer, did);
    assert_eq!(envelope.timestamp, timestamp);
    assert_eq!(envelope.artifact.name, filename);
    assert_eq!(envelope.artifact.size, file_content.len() as u64);
    assert_eq!(envelope.hash.algo, "sha-256");
    assert_eq!(envelope.hash.value, sha256_hex(file_content));

    // 3b. Verify the cryptographic signature
    let mut verify_envelope = envelope.clone();
    verify_envelope.signature = String::new();
    let canonical_bytes = jcs_canonical_bytes(&verify_envelope).expect("canonicalize");

    let signature_bytes = BASE64_STANDARD
        .decode(&envelope.signature)
        .expect("decode base64");
    let signature = Signature::from_bytes(&signature_bytes.try_into().expect("64 bytes"));

    verifying_key
        .verify(&canonical_bytes, &signature)
        .expect("Signature verification should succeed");
}

#[test]
fn test_signing_roundtrip_with_metadata() {
    let (signing_key, verifying_key) = generate_keypair();
    let did = pubkey_to_did(&verifying_key);

    let file_content = b"Binary content";
    let metadata = serde_json::json!({
        "author": {
            "name": "Test Agent",
            "email": "agent@test.com"
        },
        "version": "1.0.0",
        "license": "MIT"
    });

    let envelope = sign_artifact(
        &signing_key,
        did,
        "package.wasm".to_string(),
        file_content,
        "2026-01-31T12:00:00Z".to_string(),
        Some(metadata.clone()),
    )
    .expect("Signing should succeed");

    // Verify metadata is preserved
    assert_eq!(envelope.metadata, Some(metadata));

    // Verify signature is still valid
    let mut verify_envelope = envelope.clone();
    verify_envelope.signature = String::new();
    let canonical_bytes = jcs_canonical_bytes(&verify_envelope).expect("canonicalize");

    let signature_bytes = BASE64_STANDARD
        .decode(&envelope.signature)
        .expect("decode base64");
    let signature = Signature::from_bytes(&signature_bytes.try_into().expect("64 bytes"));

    verifying_key
        .verify(&canonical_bytes, &signature)
        .expect("Signature verification with metadata should succeed");
}

#[test]
fn test_tampered_content_fails_verification() {
    let (signing_key, verifying_key) = generate_keypair();
    let did = pubkey_to_did(&verifying_key);

    // Sign original content
    let original_content = b"Original file content";
    let envelope = sign_artifact(
        &signing_key,
        did,
        "file.txt".to_string(),
        original_content,
        "2026-01-31T12:00:00Z".to_string(),
        None,
    )
    .expect("Signing should succeed");

    // Tampered content should have different hash
    let tampered_content = b"Tampered file content";
    let tampered_hash = sha256_hex(tampered_content);

    // The envelope's hash should not match the tampered content
    assert_ne!(envelope.hash.value, tampered_hash);

    // Create a fake envelope with tampered hash to test signature failure
    let mut tampered_envelope = envelope.clone();
    tampered_envelope.hash.value = tampered_hash;
    tampered_envelope.signature = String::new();

    let canonical_bytes = jcs_canonical_bytes(&tampered_envelope).expect("canonicalize");

    let signature_bytes = BASE64_STANDARD
        .decode(&envelope.signature)
        .expect("decode base64");
    let signature = Signature::from_bytes(&signature_bytes.try_into().expect("64 bytes"));

    // Verification should fail because the canonical bytes don't match
    let result = verifying_key.verify(&canonical_bytes, &signature);
    assert!(
        result.is_err(),
        "Verification should fail for tampered content"
    );
}

#[test]
fn test_different_keys_produce_different_signatures() {
    let (signing_key1, _) = generate_keypair();
    let (signing_key2, verifying_key2) = generate_keypair();
    let did1 = pubkey_to_did(&signing_key1.verifying_key());
    let did2 = pubkey_to_did(&verifying_key2);

    let file_content = b"Same content";
    let timestamp = "2026-01-31T12:00:00Z".to_string();

    let envelope1 = sign_artifact(
        &signing_key1,
        did1,
        "file.txt".to_string(),
        file_content,
        timestamp.clone(),
        None,
    )
    .expect("Signing should succeed");

    let envelope2 = sign_artifact(
        &signing_key2,
        did2,
        "file.txt".to_string(),
        file_content,
        timestamp,
        None,
    )
    .expect("Signing should succeed");

    // Signatures should be different due to different keys
    assert_ne!(envelope1.signature, envelope2.signature);
    // DIDs should also be different
    assert_ne!(envelope1.signer, envelope2.signer);
}
