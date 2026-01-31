// SHA-256 hashing utilities for Protocol M

use sha2::{Digest, Sha256};

/// Computes the SHA-256 hash of the input bytes and returns it as a lowercase hex string.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_known_hash() {
        // SHA-256 of empty string is well-known
        let empty_hash = sha256_hex(b"");
        assert_eq!(
            empty_hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // SHA-256 of "hello" is also well-known
        let hello_hash = sha256_hex(b"hello");
        assert_eq!(
            hello_hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_sha256_returns_lowercase_hex() {
        let hash = sha256_hex(b"test");
        // Verify all characters are lowercase hex
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        // SHA-256 produces 64 hex characters (256 bits / 4 bits per hex char)
        assert_eq!(hash.len(), 64);
    }
}
