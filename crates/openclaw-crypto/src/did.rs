// DID key derivation for Protocol M
// Implements did:key method with Ed25519 keys (multicodec 0xed01)

use anyhow::{anyhow, Result};
use ed25519_dalek::VerifyingKey;

/// Multicodec prefix for Ed25519 public keys
/// 0xed = Ed25519, 0x01 = varint encoding of the codec
const ED25519_MULTICODEC_PREFIX: [u8; 2] = [0xed, 0x01];

/// Converts an Ed25519 public key to a did:key identifier.
///
/// The DID format is:
/// - did:key:z<base58btc-encoded-multicodec-pubkey>
///
/// Where the encoded value is:
/// - 2 bytes: multicodec prefix (0xed, 0x01) for Ed25519
/// - 32 bytes: raw public key bytes
///
/// # Example
/// ```
/// use ed25519_dalek::VerifyingKey;
/// use openclaw_crypto::pubkey_to_did;
///
/// // Given a verifying key, derive its DID
/// // let did = pubkey_to_did(&verifying_key);
/// // assert!(did.starts_with("did:key:z"));
/// ```
pub fn pubkey_to_did(public_key: &VerifyingKey) -> String {
    // Get raw public key bytes (32 bytes for Ed25519)
    let pubkey_bytes = public_key.as_bytes();

    // Create multicodec-prefixed bytes: 0xed01 + pubkey
    let mut multicodec_bytes = Vec::with_capacity(2 + pubkey_bytes.len());
    multicodec_bytes.extend_from_slice(&ED25519_MULTICODEC_PREFIX);
    multicodec_bytes.extend_from_slice(pubkey_bytes);

    // Encode with Base58BTC
    let base58_encoded = bs58::encode(&multicodec_bytes).into_string();

    // Prepend did:key:z (z = Base58BTC multibase prefix)
    format!("did:key:z{}", base58_encoded)
}

/// Extracts an Ed25519 VerifyingKey from a did:key identifier.
///
/// This is the reverse of `pubkey_to_did`. It:
/// 1. Strips the "did:key:z" prefix
/// 2. Decodes the Base58BTC encoded value
/// 3. Verifies the multicodec prefix is 0xed01 (Ed25519)
/// 4. Extracts and validates the 32-byte public key
///
/// # Arguments
/// * `did` - A did:key string in the format "did:key:z<base58btc-encoded-value>"
///
/// # Returns
/// * `Ok(VerifyingKey)` - The extracted Ed25519 verifying key
/// * `Err` - If the DID format is invalid or the key cannot be constructed
///
/// # Example
/// ```
/// use openclaw_crypto::{pubkey_to_did, did_to_verifying_key};
/// use ed25519_dalek::SigningKey;
///
/// let seed: [u8; 32] = [0x42; 32];
/// let signing_key = SigningKey::from_bytes(&seed);
/// let verifying_key = signing_key.verifying_key();
///
/// let did = pubkey_to_did(&verifying_key);
/// let recovered_key = did_to_verifying_key(&did).unwrap();
///
/// assert_eq!(verifying_key, recovered_key);
/// ```
pub fn did_to_verifying_key(did: &str) -> Result<VerifyingKey> {
    // Step 1: Check and strip the did:key:z prefix
    let encoded = did
        .strip_prefix("did:key:z")
        .ok_or_else(|| anyhow!("Invalid DID format: must start with 'did:key:z'"))?;

    // Step 2: Decode Base58BTC
    let decoded = bs58::decode(encoded)
        .into_vec()
        .map_err(|e| anyhow!("Invalid Base58BTC encoding: {}", e))?;

    // Minimum length: 2 bytes prefix + 32 bytes key = 34 bytes
    if decoded.len() < 34 {
        return Err(anyhow!(
            "Decoded DID too short: expected at least 34 bytes, got {}",
            decoded.len()
        ));
    }

    // Step 3: Verify multicodec prefix is 0xed01
    if decoded[0] != 0xed || decoded[1] != 0x01 {
        return Err(anyhow!(
            "Invalid multicodec prefix: expected 0xed01 (Ed25519), got 0x{:02x}{:02x}",
            decoded[0],
            decoded[1]
        ));
    }

    // Step 4: Extract 32-byte public key
    let key_bytes: [u8; 32] = decoded[2..34]
        .try_into()
        .map_err(|_| anyhow!("Invalid public key length"))?;

    // Step 5: Construct VerifyingKey
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| anyhow!("Invalid Ed25519 public key: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn test_did_format() {
        // Create a deterministic keypair from a known seed for testing
        let seed: [u8; 32] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();

        let did = pubkey_to_did(&verifying_key);

        // Verify DID format
        assert!(did.starts_with("did:key:z"), "DID should start with did:key:z");

        // Verify we can decode it back
        let encoded_part = &did[9..]; // Skip "did:key:z"
        let decoded = bs58::decode(encoded_part).into_vec().unwrap();

        // Verify multicodec prefix
        assert_eq!(decoded[0], 0xed, "First byte should be 0xed");
        assert_eq!(decoded[1], 0x01, "Second byte should be 0x01");

        // Verify public key bytes
        assert_eq!(
            &decoded[2..],
            verifying_key.as_bytes(),
            "Decoded bytes should match public key"
        );
    }

    #[test]
    fn test_did_length() {
        // Create a keypair
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();

        let did = pubkey_to_did(&verifying_key);

        // DID format: "did:key:z" (9 chars) + base58(34 bytes)
        // Base58 encoding of 34 bytes is typically 46-48 characters
        assert!(did.len() > 50, "DID should be at least 50 characters");
        assert!(did.len() < 60, "DID should be less than 60 characters");
    }

    #[test]
    fn test_different_keys_produce_different_dids() {
        let seed1: [u8; 32] = [0x01; 32];
        let seed2: [u8; 32] = [0x02; 32];

        let key1 = SigningKey::from_bytes(&seed1).verifying_key();
        let key2 = SigningKey::from_bytes(&seed2).verifying_key();

        let did1 = pubkey_to_did(&key1);
        let did2 = pubkey_to_did(&key2);

        assert_ne!(did1, did2, "Different keys should produce different DIDs");
    }

    #[test]
    fn test_same_key_produces_same_did() {
        let seed: [u8; 32] = [0x42; 32];
        let key = SigningKey::from_bytes(&seed).verifying_key();

        let did1 = pubkey_to_did(&key);
        let did2 = pubkey_to_did(&key);

        assert_eq!(did1, did2, "Same key should always produce same DID");
    }

    #[test]
    fn test_did_to_verifying_key_roundtrip() {
        let seed: [u8; 32] = [0x42; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let original_key = signing_key.verifying_key();

        let did = pubkey_to_did(&original_key);
        let recovered_key = did_to_verifying_key(&did).expect("should parse DID");

        assert_eq!(original_key, recovered_key);
    }

    #[test]
    fn test_did_to_verifying_key_known_did() {
        // Use the golden vector DID
        let known_did = "did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw";

        let result = did_to_verifying_key(known_did);
        assert!(result.is_ok(), "Should parse known DID: {:?}", result);

        // Verify we can convert back to the same DID
        let key = result.unwrap();
        let roundtrip_did = pubkey_to_did(&key);
        assert_eq!(roundtrip_did, known_did);
    }

    #[test]
    fn test_did_to_verifying_key_invalid_prefix() {
        let invalid_did = "did:web:example.com";
        let result = did_to_verifying_key(invalid_did);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must start with 'did:key:z'"));
    }

    #[test]
    fn test_did_to_verifying_key_invalid_base58() {
        let invalid_did = "did:key:z0OIl"; // Contains invalid Base58 characters
        let result = did_to_verifying_key(invalid_did);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid Base58BTC"));
    }

    #[test]
    fn test_did_to_verifying_key_too_short() {
        // Valid Base58, but too short to contain a key
        let short_did = "did:key:z6Mk"; // Very short
        let result = did_to_verifying_key(short_did);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_did_to_verifying_key_wrong_multicodec() {
        // Create a DID with wrong multicodec prefix (not Ed25519)
        // Use 0x1200 instead of 0xed01
        let mut wrong_prefix_bytes = vec![0x12, 0x00];
        wrong_prefix_bytes.extend_from_slice(&[0u8; 32]); // Add 32 bytes of key

        let encoded = bs58::encode(&wrong_prefix_bytes).into_string();
        let wrong_did = format!("did:key:z{}", encoded);

        let result = did_to_verifying_key(&wrong_did);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid multicodec prefix"));
    }
}
