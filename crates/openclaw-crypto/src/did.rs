// DID key derivation for Protocol M
// Implements did:key method with Ed25519 keys (multicodec 0xed01)

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
}
