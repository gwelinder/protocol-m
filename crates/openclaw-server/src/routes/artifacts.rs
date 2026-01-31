//! Artifact registration endpoints.

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::artifact::{Artifact, NewArtifact};
use openclaw_crypto::SignatureEnvelopeV1;

/// Response for successful artifact registration.
#[derive(serde::Serialize)]
pub struct RegisterArtifactResponse {
    /// The UUID of the registered artifact.
    pub id: Uuid,
    /// The URL path to access this artifact.
    pub url: String,
}

/// Creates the artifacts router.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/", post(register_artifact))
        .with_state(pool)
}

/// POST /api/v1/artifacts
///
/// Registers a new signed artifact.
/// Accepts a SignatureEnvelopeV1 in the request body.
/// Returns the artifact ID and URL on success.
async fn register_artifact(
    State(pool): State<PgPool>,
    Json(envelope): Json<SignatureEnvelopeV1>,
) -> Result<Json<RegisterArtifactResponse>, AppError> {
    // Parse the timestamp from the envelope
    let timestamp = parse_timestamp(&envelope.timestamp)?;

    // Extract fields from the envelope
    let new_artifact = NewArtifact {
        hash: envelope.hash.value.clone(),
        did: envelope.signer.clone(),
        timestamp,
        metadata: envelope.metadata.clone().unwrap_or(serde_json::json!({})),
        signature: envelope.signature.clone(),
    };

    // Generate a new UUID for this artifact
    let id = Uuid::new_v4();

    // Insert into the database
    let artifact = insert_artifact(&pool, id, &new_artifact).await?;

    Ok(Json(RegisterArtifactResponse {
        id: artifact.id,
        url: artifact.url_path(),
    }))
}

/// Parses an ISO 8601 timestamp string into a DateTime<Utc>.
fn parse_timestamp(timestamp: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| AppError::BadRequest(format!("Invalid timestamp format: {}", e)))
}

/// Inserts a new artifact into the database.
async fn insert_artifact(
    pool: &PgPool,
    id: Uuid,
    artifact: &NewArtifact,
) -> Result<Artifact, AppError> {
    let row = sqlx::query_as::<_, Artifact>(
        r#"
        INSERT INTO artifacts (id, hash, did, timestamp, metadata, signature, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW())
        RETURNING id, hash, did, timestamp, metadata, signature, created_at
        "#,
    )
    .bind(id)
    .bind(&artifact.hash)
    .bind(&artifact.did)
    .bind(artifact.timestamp)
    .bind(&artifact.metadata)
    .bind(&artifact.signature)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_timestamp_valid() {
        let ts = "2024-01-15T10:30:00Z";
        let result = parse_timestamp(ts);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_timestamp_with_offset() {
        let ts = "2024-01-15T10:30:00+05:00";
        let result = parse_timestamp(ts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        let ts = "not a timestamp";
        let result = parse_timestamp(ts);
        assert!(result.is_err());
    }
}
