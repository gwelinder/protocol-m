//! Type definitions for OpenClaw signature envelopes.
//!
//! These types implement the Protocol M signature envelope specification,
//! using RFC 8785 JCS canonicalization for deterministic signing.

use serde::{Deserialize, Serialize};

/// Reference to a hash of some content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HashRef {
    /// Hash algorithm used (e.g., "sha-256")
    pub algo: String,
    /// Hex-encoded hash value
    pub value: String,
}

/// Information about the artifact being signed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactInfo {
    /// Name/filename of the artifact
    pub name: String,
    /// Size of the artifact in bytes
    pub size: u64,
}

/// Version 1 signature envelope conforming to Protocol M specification.
///
/// The envelope is designed for deterministic signing using JCS canonicalization.
/// The `signature` field is set to empty string during canonicalization,
/// then populated with the base64-encoded Ed25519 signature.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignatureEnvelopeV1 {
    /// Envelope version, always "1.0"
    pub version: String,

    /// Envelope type, always "signature-envelope"
    #[serde(rename = "type")]
    pub envelope_type: String,

    /// Signature algorithm, e.g., "ed25519"
    pub algo: String,

    /// DID of the signer (did:key format)
    pub signer: String,

    /// ISO 8601 timestamp of when the signature was created
    pub timestamp: String,

    /// Hash reference of the signed content
    pub hash: HashRef,

    /// Information about the artifact
    pub artifact: ArtifactInfo,

    /// Optional metadata associated with the signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Base64-encoded signature (empty string during canonicalization)
    pub signature: String,
}

impl SignatureEnvelopeV1 {
    /// Create a new envelope with default version, type, and algorithm.
    pub fn new(
        signer: String,
        timestamp: String,
        hash: HashRef,
        artifact: ArtifactInfo,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        Self {
            version: "1.0".to_string(),
            envelope_type: "signature-envelope".to_string(),
            algo: "ed25519".to_string(),
            signer,
            timestamp,
            hash,
            artifact,
            metadata,
            signature: String::new(),
        }
    }
}

/// Reference to a signed artifact within a manifest.
///
/// Each artifact reference captures the essential identifying information
/// of a signed artifact, allowing it to be included in a contribution manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactReference {
    /// SHA-256 hash of the artifact content (hex-encoded)
    pub hash: String,
    /// Base64-encoded signature from the original envelope
    pub signature: String,
    /// ISO 8601 timestamp of when the artifact was signed
    pub timestamp: String,
    /// Optional metadata from the original envelope
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// A contribution manifest aggregating multiple signed artifacts.
///
/// The manifest allows an agent to collect and sign a summary of their
/// contributions, making them portable across systems. The manifest
/// itself is wrapped in a SignatureEnvelopeV1 when exported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContributionManifest {
    /// DID of the agent who created the manifest
    pub did: String,
    /// ISO 8601 timestamp of manifest creation
    pub timestamp: String,
    /// List of artifact references included in this manifest
    pub artifacts: Vec<ArtifactReference>,
}

impl ContributionManifest {
    /// Create a new contribution manifest.
    pub fn new(did: String, timestamp: String, artifacts: Vec<ArtifactReference>) -> Self {
        Self {
            did,
            timestamp,
            artifacts,
        }
    }
}

impl ArtifactReference {
    /// Create a new artifact reference from signature envelope details.
    pub fn new(
        hash: String,
        signature: String,
        timestamp: String,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        Self {
            hash,
            signature,
            timestamp,
            metadata,
        }
    }

    /// Create an artifact reference from a SignatureEnvelopeV1.
    pub fn from_envelope(envelope: &SignatureEnvelopeV1) -> Self {
        Self {
            hash: envelope.hash.value.clone(),
            signature: envelope.signature.clone(),
            timestamp: envelope.timestamp.clone(),
            metadata: envelope.metadata.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_envelope_serialization() {
        let envelope = SignatureEnvelopeV1::new(
            "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            "2024-01-15T10:30:00Z".to_string(),
            HashRef {
                algo: "sha-256".to_string(),
                value: "abc123".to_string(),
            },
            ArtifactInfo {
                name: "test.txt".to_string(),
                size: 1024,
            },
            None,
        );

        let json = serde_json::to_string(&envelope).unwrap();
        assert!(json.contains("\"type\":\"signature-envelope\""));
        assert!(json.contains("\"version\":\"1.0\""));
        assert!(json.contains("\"algo\":\"ed25519\""));
    }

    #[test]
    fn test_signature_envelope_deserialization() {
        let json = r#"{
            "version": "1.0",
            "type": "signature-envelope",
            "algo": "ed25519",
            "signer": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
            "timestamp": "2024-01-15T10:30:00Z",
            "hash": {
                "algo": "sha-256",
                "value": "abc123"
            },
            "artifact": {
                "name": "test.txt",
                "size": 1024
            },
            "signature": ""
        }"#;

        let envelope: SignatureEnvelopeV1 = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.version, "1.0");
        assert_eq!(envelope.envelope_type, "signature-envelope");
        assert_eq!(envelope.algo, "ed25519");
    }

    #[test]
    fn test_artifact_reference_serialization() {
        let artifact_ref = ArtifactReference::new(
            "abc123".to_string(),
            "signature_base64".to_string(),
            "2024-01-15T10:30:00Z".to_string(),
            Some(serde_json::json!({"author": "alice"})),
        );

        let json = serde_json::to_string(&artifact_ref).unwrap();
        assert!(json.contains("\"hash\":\"abc123\""));
        assert!(json.contains("\"signature\":\"signature_base64\""));
        assert!(json.contains("\"timestamp\":\"2024-01-15T10:30:00Z\""));
        assert!(json.contains("\"author\":\"alice\""));
    }

    #[test]
    fn test_artifact_reference_without_metadata() {
        let artifact_ref = ArtifactReference::new(
            "hash123".to_string(),
            "sig123".to_string(),
            "2024-01-15T10:30:00Z".to_string(),
            None,
        );

        let json = serde_json::to_string(&artifact_ref).unwrap();
        assert!(!json.contains("metadata"));
    }

    #[test]
    fn test_artifact_reference_from_envelope() {
        let envelope = SignatureEnvelopeV1 {
            version: "1.0".to_string(),
            envelope_type: "signature-envelope".to_string(),
            algo: "ed25519".to_string(),
            signer: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            hash: HashRef {
                algo: "sha-256".to_string(),
                value: "abcdef123456".to_string(),
            },
            artifact: ArtifactInfo {
                name: "test.txt".to_string(),
                size: 1024,
            },
            metadata: Some(serde_json::json!({"key": "value"})),
            signature: "base64sig".to_string(),
        };

        let artifact_ref = ArtifactReference::from_envelope(&envelope);
        assert_eq!(artifact_ref.hash, "abcdef123456");
        assert_eq!(artifact_ref.signature, "base64sig");
        assert_eq!(artifact_ref.timestamp, "2024-01-15T10:30:00Z");
        assert_eq!(
            artifact_ref.metadata,
            Some(serde_json::json!({"key": "value"}))
        );
    }

    #[test]
    fn test_contribution_manifest_serialization() {
        let manifest = ContributionManifest::new(
            "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            "2024-01-15T12:00:00Z".to_string(),
            vec![
                ArtifactReference::new(
                    "hash1".to_string(),
                    "sig1".to_string(),
                    "2024-01-15T10:00:00Z".to_string(),
                    None,
                ),
                ArtifactReference::new(
                    "hash2".to_string(),
                    "sig2".to_string(),
                    "2024-01-15T11:00:00Z".to_string(),
                    Some(serde_json::json!({"type": "code"})),
                ),
            ],
        );

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        assert!(json.contains("\"did\":"));
        assert!(json.contains("\"timestamp\":"));
        assert!(json.contains("\"artifacts\":"));
        assert!(json.contains("\"hash1\""));
        assert!(json.contains("\"hash2\""));
    }

    #[test]
    fn test_contribution_manifest_deserialization() {
        let json = r#"{
            "did": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
            "timestamp": "2024-01-15T12:00:00Z",
            "artifacts": [
                {
                    "hash": "abc123",
                    "signature": "sig1",
                    "timestamp": "2024-01-15T10:00:00Z"
                }
            ]
        }"#;

        let manifest: ContributionManifest = serde_json::from_str(json).unwrap();
        assert_eq!(
            manifest.did,
            "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
        );
        assert_eq!(manifest.timestamp, "2024-01-15T12:00:00Z");
        assert_eq!(manifest.artifacts.len(), 1);
        assert_eq!(manifest.artifacts[0].hash, "abc123");
    }
}
