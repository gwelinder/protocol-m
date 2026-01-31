// OpenClaw CLI - Command-line interface for Protocol M

mod keystore;
pub mod metadata;

use clap::{Parser, Subcommand};

/// OpenClaw - Protocol M Identity & Signing Tool
#[derive(Parser)]
#[command(name = "openclaw")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage identity (init, show)
    Identity {
        #[command(subcommand)]
        action: IdentityAction,
    },
    /// Sign an artifact
    Sign {
        /// Path to the file to sign
        path: String,

        /// Add metadata key=value pairs (can be specified multiple times)
        #[arg(short, long = "meta", value_name = "KEY=VALUE")]
        meta: Vec<String>,

        /// Preview envelope without writing to disk
        #[arg(long)]
        dry_run: bool,
    },
    /// Verify a signature
    Verify {
        /// Path to the file to verify
        path: String,

        /// Path to the signature file (defaults to <file>.sig.json)
        #[arg(short, long)]
        sig: Option<String>,
    },
    /// Manage contribution manifests
    Manifest {
        #[command(subcommand)]
        action: ManifestAction,
    },
}

#[derive(Subcommand)]
enum IdentityAction {
    /// Initialize a new identity
    Init {
        /// Force overwrite existing identity
        #[arg(short, long)]
        force: bool,
    },
    /// Show current identity
    Show,
}

#[derive(Subcommand)]
enum ManifestAction {
    /// Export contribution manifest
    Export,
    /// Import contribution manifest
    Import {
        /// Path to manifest file
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Identity { action } => handle_identity(action),
        Commands::Sign { path, meta, dry_run } => handle_sign(&path, meta, dry_run),
        Commands::Verify { path, sig } => handle_verify(&path, sig.as_deref()),
        Commands::Manifest { action } => handle_manifest(action),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn handle_identity(action: IdentityAction) -> anyhow::Result<()> {
    match action {
        IdentityAction::Init { force } => {
            let did = keystore::init_identity(force)?;
            println!("Identity initialized successfully!");
            println!();
            println!("Your DID: {}", did);
            println!();
            println!("Your identity files are stored in ~/.openclaw/identity/");
            println!("Keep your passphrase safe - it cannot be recovered!");
            Ok(())
        }
        IdentityAction::Show => {
            println!("Identity show placeholder");
            Ok(())
        }
    }
}

fn handle_sign(path: &str, meta: Vec<String>, dry_run: bool) -> anyhow::Result<()> {
    use std::path::Path;

    // Load identity from ~/.openclaw/identity/
    let identity = keystore::load_identity_info()?;

    // Prompt for passphrase and decrypt private key
    let passphrase = keystore::prompt_passphrase_single()?;
    let signing_key = keystore::load_signing_key(&passphrase)?;

    // Read file bytes
    let file_path = Path::new(path);
    let file_bytes = std::fs::read(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", path, e))?;

    let filename = file_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());

    // Get current timestamp (ISO 8601)
    let timestamp = keystore::chrono_iso8601_now();

    // Parse metadata if provided
    let metadata = if meta.is_empty() {
        None
    } else {
        Some(metadata::parse_metadata(meta)?)
    };

    // Call sign_artifact from openclaw-crypto
    let envelope = openclaw_crypto::sign_artifact(
        &signing_key,
        identity.did.clone(),
        filename,
        &file_bytes,
        timestamp,
        metadata,
    )?;

    if dry_run {
        // Output envelope to stdout (handled in US-002E)
        let json = serde_json::to_string_pretty(&envelope)?;
        println!("{}", json);
    } else {
        // Write envelope to file (handled in US-002D)
        let output_path = format!("{}.sig.json", path);
        let json = serde_json::to_string_pretty(&envelope)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(&output_path, &json)?;
            std::fs::set_permissions(&output_path, std::fs::Permissions::from_mode(0o644))?;
        }

        #[cfg(not(unix))]
        {
            std::fs::write(&output_path, &json)?;
        }

        println!("Signature written to: {}", output_path);
        println!("Signer: {}", identity.did);
    }

    Ok(())
}

fn handle_verify(path: &str, sig_path: Option<&str>) -> anyhow::Result<()> {
    use std::path::Path;

    // Determine signature file path
    let sig_file = match sig_path {
        Some(p) => p.to_string(),
        None => format!("{}.sig.json", path),
    };

    // Load envelope from .sig.json file
    let sig_content = std::fs::read_to_string(&sig_file)
        .map_err(|e| anyhow::anyhow!("Failed to read signature file '{}': {}", sig_file, e))?;

    // Parse envelope as SignatureEnvelopeV1
    let envelope: openclaw_crypto::SignatureEnvelopeV1 = serde_json::from_str(&sig_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse signature envelope: {}", e))?;

    // Extract DID and convert to VerifyingKey
    let verifying_key = openclaw_crypto::did_to_verifying_key(&envelope.signer)
        .map_err(|e| anyhow::anyhow!("Failed to extract public key from DID: {}", e))?;

    // Read file bytes
    let file_path = Path::new(path);
    let file_bytes = std::fs::read(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", path, e))?;

    // Call verify_artifact from openclaw-crypto
    openclaw_crypto::verify_artifact(&verifying_key, &file_bytes, &envelope)?;

    // Success - verification passed
    println!("Verification successful!");
    println!("Signer: {}", envelope.signer);
    println!("Timestamp: {}", envelope.timestamp);

    Ok(())
}

fn handle_manifest(action: ManifestAction) -> anyhow::Result<()> {
    match action {
        ManifestAction::Export => {
            println!("Manifest export placeholder");
            Ok(())
        }
        ManifestAction::Import { path } => {
            println!("Manifest import placeholder: {}", path);
            Ok(())
        }
    }
}
