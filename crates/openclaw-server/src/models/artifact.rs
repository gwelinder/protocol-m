//! Artifact model for storing signed artifacts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Represents a signed artifact stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Artifact {
    /// Unique identifier for this artifact record.
    pub id: Uuid,
    /// SHA-256 hash of the artifact content (hex-encoded).
    pub hash: String,
    /// DID of the signer (did:key:z6Mk...).
    pub did: String,
    /// Timestamp from the signature envelope.
    pub timestamp: DateTime<Utc>,
    /// Additional metadata from the signature envelope.
    pub metadata: serde_json::Value,
    /// Base64-encoded signature.
    pub signature: String,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
}

/// Data required to create a new artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewArtifact {
    pub hash: String,
    pub did: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
    pub signature: String,
}

impl Artifact {
    /// Returns the URL path for this artifact.
    pub fn url_path(&self) -> String {
        format!("/api/v1/artifacts/{}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_url_path() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let artifact = Artifact {
            id,
            hash: "abc123".to_string(),
            did: "did:key:z6Mk...".to_string(),
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            signature: "sig...".to_string(),
            created_at: Utc::now(),
        };
        assert_eq!(
            artifact.url_path(),
            "/api/v1/artifacts/550e8400-e29b-41d4-a716-446655440000"
        );
    }
}
