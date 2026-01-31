// Keystore module - manages private key encryption and storage using age encryption

use age::secrecy::SecretString;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Identity metadata stored in identity.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityInfo {
    /// DID of this identity (did:key:z...)
    pub did: String,
    /// Timestamp when the identity was created (ISO 8601)
    pub created_at: String,
    /// Type of key algorithm used
    pub key_type: String,
}

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

/// Returns the path to the OpenClaw identity directory (~/.openclaw/identity/)
pub fn identity_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("Could not determine home directory")?;
    Ok(PathBuf::from(home).join(".openclaw").join("identity"))
}

/// Checks if an identity already exists
pub fn identity_exists() -> Result<bool> {
    let identity_path = identity_dir()?;
    Ok(identity_path.join("identity.json").exists())
}

/// Prompts for passphrase with confirmation
fn prompt_passphrase() -> Result<String> {
    let passphrase = rpassword::prompt_password("Enter passphrase for private key: ")
        .context("Failed to read passphrase")?;

    if passphrase.is_empty() {
        anyhow::bail!("Passphrase cannot be empty");
    }

    let confirm = rpassword::prompt_password("Confirm passphrase: ")
        .context("Failed to read passphrase confirmation")?;

    if passphrase != confirm {
        anyhow::bail!("Passphrases do not match");
    }

    Ok(passphrase)
}

/// Creates a directory with secure permissions (0700 on Unix)
#[cfg(unix)]
fn create_secure_dir(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory {}", path.display()))?;

    fs::set_permissions(path, fs::Permissions::from_mode(permissions::DIRECTORY_MODE))
        .with_context(|| format!("Failed to set permissions on {}", path.display()))?;

    Ok(())
}

/// Creates a directory (Windows version - no special permissions)
#[cfg(not(unix))]
fn create_secure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory {}", path.display()))?;
    Ok(())
}

/// Writes a file with specific permissions
#[cfg(unix)]
fn write_file_with_perms(path: &Path, contents: &[u8], mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, contents)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("Failed to set permissions on {}", path.display()))?;

    Ok(())
}

/// Writes a file (Windows version - no special permissions)
#[cfg(not(unix))]
fn write_file_with_perms(path: &Path, contents: &[u8], _mode: u32) -> Result<()> {
    fs::write(path, contents)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Initializes a new OpenClaw identity.
///
/// This function:
/// 1. Checks if identity already exists (errors unless force=true)
/// 2. Creates ~/.openclaw/identity/ with 0700 permissions
/// 3. Generates an Ed25519 keypair
/// 4. Derives DID from the public key
/// 5. Prompts for passphrase (with confirmation)
/// 6. Encrypts the private key with age
/// 7. Writes root.key.enc (0600), root.pub (0644), identity.json
///
/// # Arguments
/// * `force` - If true, overwrites existing identity
///
/// # Returns
/// The DID of the newly created identity
pub fn init_identity(force: bool) -> Result<String> {
    // Check if identity already exists
    if identity_exists()? && !force {
        anyhow::bail!(
            "Identity already exists at {:?}. Use --force to overwrite.",
            identity_dir()?
        );
    }

    // Create identity directory with secure permissions
    let identity_path = identity_dir()?;
    create_secure_dir(&identity_path)?;

    // Generate keypair
    let (signing_key, verifying_key) = openclaw_crypto::generate_keypair();

    // Derive DID from public key
    let did = openclaw_crypto::pubkey_to_did(&verifying_key);

    // Prompt for passphrase
    let passphrase = prompt_passphrase()?;

    // Encrypt private key
    let private_key_bytes = signing_key.to_bytes();
    let encrypted_key = encrypt_key(&private_key_bytes, &passphrase)?;

    // Write encrypted private key (0600)
    let key_enc_path = identity_path.join("root.key.enc");
    write_file_with_perms(&key_enc_path, &encrypted_key, permissions::KEYFILE_MODE)?;

    // Write public key (0644 - readable by others)
    let pub_key_path = identity_path.join("root.pub");
    let public_key_bytes = verifying_key.to_bytes();
    write_file_with_perms(&pub_key_path, &public_key_bytes, 0o644)?;

    // Create identity.json
    let now = chrono_iso8601_now();
    let identity_info = IdentityInfo {
        did: did.clone(),
        created_at: now,
        key_type: "Ed25519".to_string(),
    };

    let identity_json = serde_json::to_string_pretty(&identity_info)
        .context("Failed to serialize identity info")?;

    // Write identity.json (0644 - readable metadata)
    let identity_json_path = identity_path.join("identity.json");
    write_file_with_perms(&identity_json_path, identity_json.as_bytes(), 0o644)?;

    Ok(did)
}

/// Returns current time as ISO 8601 string
fn chrono_iso8601_now() -> String {
    // Use simple time formatting without external chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Convert to UTC components (simplified - doesn't account for leap seconds)
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Calculate year, month, day from days since epoch (1970-01-01)
    let (year, month, day) = days_to_ymd(days as i64);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to (year, month, day)
fn days_to_ymd(mut days: i64) -> (i32, u32, u32) {
    // Days from year 1 to Unix epoch (1970-01-01)
    days += 719468;

    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era [0, 399]
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month starting from March [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };

    (y as i32, m, d)
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
