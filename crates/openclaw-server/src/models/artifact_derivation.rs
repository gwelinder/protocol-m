//! Artifact derivation model for tracking attribution relationships.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Represents a derivation relationship between artifacts.
/// An artifact can be derived from one or more parent artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ArtifactDerivation {
    /// Unique identifier for this derivation record.
    pub id: Uuid,
    /// The artifact that is derived from a parent.
    pub artifact_id: Uuid,
    /// The parent artifact this was derived from.
    pub derived_from_id: Uuid,
    /// When this derivation was recorded.
    pub created_at: DateTime<Utc>,
}

/// Data required to create a new derivation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewArtifactDerivation {
    pub artifact_id: Uuid,
    pub derived_from_id: Uuid,
}

impl ArtifactDerivation {
    /// Create a new derivation record input.
    pub fn new(artifact_id: Uuid, derived_from_id: Uuid) -> NewArtifactDerivation {
        NewArtifactDerivation {
            artifact_id,
            derived_from_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_artifact_derivation() {
        let artifact_id = Uuid::new_v4();
        let derived_from_id = Uuid::new_v4();

        let derivation = ArtifactDerivation::new(artifact_id, derived_from_id);

        assert_eq!(derivation.artifact_id, artifact_id);
        assert_eq!(derivation.derived_from_id, derived_from_id);
    }

    #[test]
    fn test_artifact_derivation_serialization() {
        let derivation = ArtifactDerivation {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            artifact_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
            derived_from_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap(),
            created_at: DateTime::parse_from_rfc3339("2026-01-31T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };

        let json = serde_json::to_string(&derivation).unwrap();
        assert!(json.contains("550e8400-e29b-41d4-a716-446655440000"));
        assert!(json.contains("artifact_id"));
        assert!(json.contains("derived_from_id"));
    }
}
