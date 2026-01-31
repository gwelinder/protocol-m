// Keystore module - manages private key encryption and storage using age encryption

use age::secrecy::SecretString;
use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::path::Path;

/// Expected permission modes for identity files (Unix only)
#[cfg(unix)]
pub mod permissions {
    /// Directory should be owner-only: rwx------
    pub const DIRECTORY_MODE: u32 = 0o700;
    /// Private key file should be owner-only: rw-------
    pub const KEYFILE_MODE: u32 = 0o600;
}

/// Checks that file/directory permissions are secure.
///
/// On Unix systems:
/// - Directories should be 0700 (owner rwx only)
/// - Keyfiles should be 0600 (owner rw only)
///
/// On Windows, this check is skipped (returns Ok).
///
/// # Arguments
/// * `path` - The file or directory path to check
/// * `is_directory` - True if checking a directory, false for a keyfile
///
/// # Returns
/// Ok(()) if permissions are secure, Error if too permissive or path doesn't exist
#[cfg(unix)]
pub fn check_permissions(path: &Path, is_directory: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to read metadata for {}", path.display()))?;

    let mode = metadata.permissions().mode() & 0o777; // Extract permission bits only
    let expected = if is_directory {
        permissions::DIRECTORY_MODE
    } else {
        permissions::KEYFILE_MODE
    };

    // Check if permissions are more permissive than expected
    // We need to ensure no extra bits are set beyond what's allowed
    if mode & !expected != 0 {
        let path_type = if is_directory { "Directory" } else { "Keyfile" };
        anyhow::bail!(
            "{} '{}' has permissions {:04o}, expected {:04o}. \
            Permissions are too permissive. Fix with: chmod {:04o} {}",
            path_type,
            path.display(),
            mode,
            expected,
            expected,
            path.display()
        );
    }

    Ok(())
}

/// On Windows, permission checks are skipped.
/// Windows uses ACLs which require different handling.
#[cfg(not(unix))]
pub fn check_permissions(_path: &Path, _is_directory: bool) -> Result<()> {
    // Windows uses ACLs, skip permission check for now
    // Future: Could use windows-acl crate to verify ownership
    Ok(())
}

/// Encrypts private key bytes with a passphrase using age encryption.
///
/// # Arguments
/// * `key_bytes` - The raw private key bytes to encrypt
/// * `passphrase` - The passphrase to use for encryption
///
/// # Returns
/// Encrypted bytes suitable for file storage
pub fn encrypt_key(key_bytes: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let encryptor = age::Encryptor::with_user_passphrase(SecretString::from(passphrase.to_string()));

    let mut encrypted = vec![];
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .context("Failed to create age encryptor")?;

    writer
        .write_all(key_bytes)
        .context("Failed to write key bytes to encryptor")?;

    writer
        .finish()
        .context("Failed to finalize encryption")?;

    Ok(encrypted)
}

/// Decrypts private key bytes with a passphrase using age decryption.
///
/// # Arguments
/// * `encrypted` - The encrypted key bytes (age format)
/// * `passphrase` - The passphrase used during encryption
///
/// # Returns
/// Decrypted key bytes, or error if passphrase is incorrect or data is corrupted
pub fn decrypt_key(encrypted: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let decryptor = match age::Decryptor::new(encrypted)
        .context("Failed to parse encrypted data")?
    {
        age::Decryptor::Passphrase(d) => d,
        _ => anyhow::bail!("Encrypted data was not passphrase-protected"),
    };

    let mut decrypted = vec![];
    let mut reader = decryptor
        .decrypt(&SecretString::from(passphrase.to_string()), None)
        .map_err(|_| anyhow::anyhow!("Incorrect passphrase"))?;

    reader
        .read_to_end(&mut decrypted)
        .context("Failed to read decrypted bytes")?;

    Ok(decrypted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use tempfile::tempdir;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        // Test with a typical Ed25519 private key (32 bytes)
        let original_key: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
        ];
        let passphrase = "test-passphrase-123";

        // Encrypt the key
        let encrypted = encrypt_key(&original_key, passphrase)
            .expect("Encryption should succeed");

        // Verify encrypted bytes are different from original
        assert_ne!(&encrypted[..], &original_key[..]);

        // Decrypt and verify we get the original key back
        let decrypted = decrypt_key(&encrypted, passphrase)
            .expect("Decryption should succeed");

        assert_eq!(decrypted, original_key);
    }

    #[test]
    fn test_encrypt_produces_different_output_each_time() {
        let key_bytes: [u8; 32] = [0xab; 32];
        let passphrase = "my-secret-passphrase";

        let encrypted1 = encrypt_key(&key_bytes, passphrase).unwrap();
        let encrypted2 = encrypt_key(&key_bytes, passphrase).unwrap();

        // age uses random salt, so each encryption should produce different output
        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_decrypt_with_wrong_passphrase_fails() {
        let key_bytes: [u8; 32] = [0x42; 32];
        let correct_passphrase = "correct-passphrase";
        let wrong_passphrase = "wrong-passphrase";

        // Encrypt with the correct passphrase
        let encrypted = encrypt_key(&key_bytes, correct_passphrase)
            .expect("Encryption should succeed");

        // Attempt to decrypt with the wrong passphrase
        let result = decrypt_key(&encrypted, wrong_passphrase);

        // Decryption should fail
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Incorrect passphrase"));
    }

    #[cfg(unix)]
    #[test]
    fn test_check_permissions_directory_0700_passes() {
        let dir = tempdir().expect("Failed to create temp dir");
        let path = dir.path();

        // Set correct directory permissions
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .expect("Failed to set permissions");

        let result = check_permissions(path, true);
        assert!(result.is_ok(), "0700 directory should pass: {:?}", result);
    }

    #[cfg(unix)]
    #[test]
    fn test_check_permissions_directory_0755_fails() {
        let dir = tempdir().expect("Failed to create temp dir");
        let path = dir.path();

        // Set overly permissive directory permissions
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .expect("Failed to set permissions");

        let result = check_permissions(path, true);
        assert!(result.is_err(), "0755 directory should fail");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("too permissive"), "Error should mention 'too permissive': {}", err_msg);
    }

    #[cfg(unix)]
    #[test]
    fn test_check_permissions_file_0600_passes() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("test.key");

        // Create file and set correct permissions
        fs::write(&file_path, b"test key data").expect("Failed to create file");
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600))
            .expect("Failed to set permissions");

        let result = check_permissions(&file_path, false);
        assert!(result.is_ok(), "0600 keyfile should pass: {:?}", result);
    }

    #[cfg(unix)]
    #[test]
    fn test_check_permissions_file_0644_fails() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("test.key");

        // Create file and set overly permissive permissions
        fs::write(&file_path, b"test key data").expect("Failed to create file");
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644))
            .expect("Failed to set permissions");

        let result = check_permissions(&file_path, false);
        assert!(result.is_err(), "0644 keyfile should fail");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("too permissive"), "Error should mention 'too permissive': {}", err_msg);
    }

    #[cfg(unix)]
    #[test]
    fn test_check_permissions_nonexistent_path_fails() {
        let path = Path::new("/nonexistent/path/to/file");
        let result = check_permissions(path, false);
        assert!(result.is_err(), "Nonexistent path should fail");
    }
}
