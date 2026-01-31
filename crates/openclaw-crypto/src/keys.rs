// OpenClaw Crypto - Key generation for Protocol M

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand_core::OsRng;

/// Generates a new Ed25519 keypair using secure random bytes from the OS.
///
/// Returns a tuple of (SigningKey, VerifyingKey) that can be used for
/// signing and verifying Protocol M signature envelopes.
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    #[test]
    fn test_generate_keypair_signs_and_verifies() {
        let (signing_key, verifying_key) = generate_keypair();

        // Sign a test message
        let message = b"Protocol M test message";
        let signature = signing_key.sign(message);

        // Verify the signature using the verifying key
        use ed25519_dalek::Verifier;
        assert!(
            verifying_key.verify(message, &signature).is_ok(),
            "Signature verification should succeed"
        );
    }

    #[test]
    fn test_generate_keypair_produces_different_keys() {
        let (_, verifying_key1) = generate_keypair();
        let (_, verifying_key2) = generate_keypair();

        // Each call should produce a different keypair
        assert_ne!(
            verifying_key1.as_bytes(),
            verifying_key2.as_bytes(),
            "Sequential keypair generation should produce different keys"
        );
    }

    #[test]
    fn test_verifying_key_wrong_message_fails() {
        let (signing_key, verifying_key) = generate_keypair();

        let message = b"Original message";
        let signature = signing_key.sign(message);

        // Verification with wrong message should fail
        let wrong_message = b"Tampered message";
        use ed25519_dalek::Verifier;
        assert!(
            verifying_key.verify(wrong_message, &signature).is_err(),
            "Verification with wrong message should fail"
        );
    }
}
