// OpenClaw CLI - Command-line interface for Protocol M

mod keystore;
pub mod metadata;
pub mod policy;

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
    /// Manage policy settings
    Policy {
        #[command(subcommand)]
        action: PolicyAction,
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
    /// Export contribution manifest from signature files
    Export {
        /// Paths to signature files (.sig.json) to include in manifest
        /// If not specified, scans ~/.openclaw/signatures/
        #[arg(value_name = "SIG_FILE")]
        paths: Vec<String>,

        /// Output file path (defaults to manifest.json)
        #[arg(short, long, default_value = "manifest.json")]
        output: String,
    },
    /// Import contribution manifest
    Import {
        /// Path to manifest file
        path: String,
    },
}

#[derive(Subcommand)]
enum PolicyAction {
    /// Set a policy from a JSON file
    Set {
        /// Path to the policy JSON file
        #[arg(short, long)]
        file: String,
    },
    /// Show current policy
    Show,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Identity { action } => handle_identity(action),
        Commands::Sign { path, meta, dry_run } => handle_sign(&path, meta, dry_run),
        Commands::Verify { path, sig } => handle_verify(&path, sig.as_deref()),
        Commands::Manifest { action } => handle_manifest(action),
        Commands::Policy { action } => handle_policy(action),
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
    use colored::Colorize;
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
    match openclaw_crypto::verify_artifact(&verifying_key, &file_bytes, &envelope) {
        Ok(()) => {
            // Success - print green checkmark
            println!("{} {}", "✓".green().bold(), "Signature verified".green());
            println!();

            // Check if signer matches local identity
            let identity_indicator = match keystore::load_identity_info() {
                Ok(local_identity) if local_identity.did == envelope.signer => {
                    "(Local Identity)".cyan().to_string()
                }
                Ok(_) => "(External Identity)".yellow().to_string(),
                Err(_) => "(No local identity)".dimmed().to_string(),
            };

            // Truncate DID for readability (show first 20 chars + ... + last 8)
            let truncated_did = truncate_did(&envelope.signer);
            println!("  Signer:    {} {}", truncated_did, identity_indicator);
            println!("  Timestamp: {}", envelope.timestamp);
            println!("  File:      {}", envelope.artifact.name);
            Ok(())
        }
        Err(e) => {
            // Failure - print red X
            eprintln!("{} {}", "✗".red().bold(), "Signature verification failed".red());
            eprintln!();
            eprintln!("  Error: {}", e);
            Err(e)
        }
    }
}

/// Truncates a DID for readability: "did:key:z6Mk...last8chars"
fn truncate_did(did: &str) -> String {
    if did.len() <= 30 {
        return did.to_string();
    }
    let prefix = &did[..20];
    let suffix = &did[did.len() - 8..];
    format!("{}...{}", prefix, suffix)
}

fn handle_manifest(action: ManifestAction) -> anyhow::Result<()> {
    match action {
        ManifestAction::Export { paths, output } => handle_manifest_export(paths, &output),
        ManifestAction::Import { path } => {
            println!("Manifest import placeholder: {}", path);
            Ok(())
        }
    }
}

fn handle_manifest_export(paths: Vec<String>, output: &str) -> anyhow::Result<()> {
    use colored::Colorize;
    use std::path::Path;

    // Load identity from ~/.openclaw/identity/
    let identity = keystore::load_identity_info()?;

    // Prompt for passphrase and decrypt private key
    let passphrase = keystore::prompt_passphrase_single()?;
    let signing_key = keystore::load_signing_key(&passphrase)?;

    // Collect signature files to include
    let sig_files = if paths.is_empty() {
        // Scan ~/.openclaw/signatures/ for .sig.json files
        let sig_dir = get_signatures_dir()?;
        if !sig_dir.exists() {
            return Err(anyhow::anyhow!(
                "No signature files provided and {} does not exist.\n\
                 Usage: openclaw manifest export <SIG_FILE>... [-o <OUTPUT>]",
                sig_dir.display()
            ));
        }
        scan_signature_files(&sig_dir)?
    } else {
        // Use provided paths
        paths.iter().map(|p| Path::new(p).to_path_buf()).collect()
    };

    if sig_files.is_empty() {
        return Err(anyhow::anyhow!(
            "No signature files found. Provide .sig.json files as arguments."
        ));
    }

    // Load and parse signature envelopes
    let mut artifact_refs = Vec::new();
    for sig_file in &sig_files {
        let content = std::fs::read_to_string(sig_file)
            .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", sig_file.display(), e))?;

        let envelope: openclaw_crypto::SignatureEnvelopeV1 = serde_json::from_str(&content)
            .map_err(|e| {
                anyhow::anyhow!("Failed to parse '{}': {}", sig_file.display(), e)
            })?;

        // Convert envelope to artifact reference
        artifact_refs.push(openclaw_crypto::ArtifactReference::from_envelope(&envelope));
    }

    // Get current timestamp
    let timestamp = keystore::chrono_iso8601_now();

    // Export manifest
    let manifest_envelope = openclaw_crypto::export_manifest(
        &signing_key,
        identity.did.clone(),
        artifact_refs,
        timestamp,
    )?;

    // Write manifest to output file
    let json = serde_json::to_string_pretty(&manifest_envelope)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(output, &json)?;
        std::fs::set_permissions(output, std::fs::Permissions::from_mode(0o644))?;
    }

    #[cfg(not(unix))]
    {
        std::fs::write(output, &json)?;
    }

    // Print success message
    println!(
        "{} Manifest exported successfully!",
        "✓".green().bold()
    );
    println!();
    println!("  Output:    {}", output);
    println!("  Signer:    {}", truncate_did(&identity.did));
    println!("  Artifacts: {}", sig_files.len());
    println!();
    println!("  Files included:");
    for sig_file in &sig_files {
        println!("    - {}", sig_file.display());
    }

    Ok(())
}

/// Get the default signatures directory path (~/.openclaw/signatures/)
fn get_signatures_dir() -> anyhow::Result<std::path::PathBuf> {
    #[cfg(unix)]
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;

    #[cfg(windows)]
    let home = std::env::var("USERPROFILE")
        .map_err(|_| anyhow::anyhow!("USERPROFILE environment variable not set"))?;

    Ok(std::path::PathBuf::from(home)
        .join(".openclaw")
        .join("signatures"))
}

/// Scan a directory for .sig.json files
fn scan_signature_files(dir: &std::path::Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut sig_files = Vec::new();

    for entry in std::fs::read_dir(dir)
        .map_err(|e| anyhow::anyhow!("Failed to read directory '{}': {}", dir.display(), e))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".sig.json") {
                    sig_files.push(path);
                }
            }
        }
    }

    // Sort by filename for consistent ordering
    sig_files.sort();

    Ok(sig_files)
}

fn handle_policy(action: PolicyAction) -> anyhow::Result<()> {
    match action {
        PolicyAction::Set { file } => handle_policy_set(&file),
        PolicyAction::Show => handle_policy_show(),
    }
}

fn handle_policy_set(file_path: &str) -> anyhow::Result<()> {
    use colored::Colorize;

    // Read the policy file
    let policy_content = std::fs::read_to_string(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to read policy file '{}': {}", file_path, e))?;

    // Validate the policy
    let policy = policy::validate_policy(&policy_content)?;

    // Get the policy directory path (~/.openclaw/)
    let policy_dir = get_openclaw_dir()?;

    // Create directory if it doesn't exist
    if !policy_dir.exists() {
        std::fs::create_dir_all(&policy_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create directory '{}': {}", policy_dir.display(), e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&policy_dir, std::fs::Permissions::from_mode(0o700))?;
        }
    }

    // Write the validated policy to ~/.openclaw/policy.json
    let output_path = policy_dir.join("policy.json");

    // Serialize the validated policy (ensures consistent formatting)
    let json = serde_json::to_string_pretty(&policy)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&output_path, &json)?;
        std::fs::set_permissions(&output_path, std::fs::Permissions::from_mode(0o600))?;
    }

    #[cfg(not(unix))]
    {
        std::fs::write(&output_path, &json)?;
    }

    // Print success message with policy summary
    println!("{} Policy set successfully!", "✓".green().bold());
    println!();
    println!("  Location: {}", output_path.display());
    println!();
    println!("  {}", "Policy Summary:".bold());
    for line in policy.summary().lines() {
        println!("    {}", line);
    }

    Ok(())
}

fn handle_policy_show() -> anyhow::Result<()> {
    use colored::Colorize;

    // Get the policy file path
    let policy_path = get_openclaw_dir()?.join("policy.json");

    if !policy_path.exists() {
        println!("{}", "No policy configured.".yellow());
        println!();
        println!("Set a policy with: openclaw policy set --file <policy.json>");
        return Ok(());
    }

    // Read and validate the policy
    let content = std::fs::read_to_string(&policy_path)
        .map_err(|e| anyhow::anyhow!("Failed to read policy file: {}", e))?;

    let policy = policy::validate_policy(&content)?;

    println!("{} Current Policy", "✓".green().bold());
    println!();
    println!("  Location: {}", policy_path.display());
    println!();
    println!("  {}", "Policy Summary:".bold());
    for line in policy.summary().lines() {
        println!("    {}", line);
    }

    if !policy.allowed_delegates.is_empty() {
        println!();
        println!("  {}", "Allowed Delegates:".bold());
        for delegate in &policy.allowed_delegates {
            println!("    - {}", truncate_did(delegate));
        }
    }

    if !policy.approval_tiers.is_empty() {
        println!();
        println!("  {}", "Approval Tiers:".bold());
        for (i, tier) in policy.approval_tiers.iter().enumerate() {
            println!("    {}. Threshold: {} credits", i + 1, tier.threshold);
            println!("       Requires approval: {}", tier.require_approval);
            println!("       Timeout: {} hours", tier.timeout_hours);
            if !tier.approvers.is_empty() {
                println!("       Approvers: {} DID(s)", tier.approvers.len());
            }
        }
    }

    if let Some(ref contact) = policy.emergency_contact {
        println!();
        println!("  {}", "Emergency Contact:".bold());
        if let Some(ref email) = contact.email {
            println!("    Email: {}", email);
        }
        if let Some(ref webhook) = contact.webhook {
            println!("    Webhook: {}", webhook);
        }
    }

    Ok(())
}

/// Get the OpenClaw directory path (~/.openclaw/)
fn get_openclaw_dir() -> anyhow::Result<std::path::PathBuf> {
    #[cfg(unix)]
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;

    #[cfg(windows)]
    let home = std::env::var("USERPROFILE")
        .map_err(|_| anyhow::anyhow!("USERPROFILE environment variable not set"))?;

    Ok(std::path::PathBuf::from(home).join(".openclaw"))
}
