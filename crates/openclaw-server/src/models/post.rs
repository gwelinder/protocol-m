use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Verification status for a signed post
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "verification_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    /// No signature provided
    None,
    /// Signature is invalid or verification failed
    Invalid,
    /// Signature is valid but DID not bound to user
    ValidUnbound,
    /// Signature is valid and DID is bound to user
    ValidBound,
}

impl Default for VerificationStatus {
    fn default() -> Self {
        Self::None
    }
}

/// A post with optional signature verification fields
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Protocol M signature envelope as JSON (nullable)
    #[sqlx(rename = "signature_envelope_json")]
    pub signature_envelope_json: Option<serde_json::Value>,
    /// DID of the verified signer (nullable)
    pub verified_did: Option<String>,
    /// Status of signature verification
    pub verification_status: VerificationStatus,
}

/// Data for creating a new post
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPost {
    pub user_id: Uuid,
    pub content: String,
    /// Optional signature envelope for verification
    pub signature_envelope: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_status_default() {
        assert_eq!(VerificationStatus::default(), VerificationStatus::None);
    }

    #[test]
    fn test_verification_status_serialization() {
        assert_eq!(
            serde_json::to_string(&VerificationStatus::None).unwrap(),
            "\"none\""
        );
        assert_eq!(
            serde_json::to_string(&VerificationStatus::Invalid).unwrap(),
            "\"invalid\""
        );
        assert_eq!(
            serde_json::to_string(&VerificationStatus::ValidUnbound).unwrap(),
            "\"valid_unbound\""
        );
        assert_eq!(
            serde_json::to_string(&VerificationStatus::ValidBound).unwrap(),
            "\"valid_bound\""
        );
    }

    #[test]
    fn test_verification_status_deserialization() {
        assert_eq!(
            serde_json::from_str::<VerificationStatus>("\"none\"").unwrap(),
            VerificationStatus::None
        );
        assert_eq!(
            serde_json::from_str::<VerificationStatus>("\"valid_bound\"").unwrap(),
            VerificationStatus::ValidBound
        );
    }

    #[test]
    fn test_new_post_without_signature() {
        let new_post = NewPost {
            user_id: Uuid::new_v4(),
            content: "Hello world".to_string(),
            signature_envelope: None,
        };
        assert!(new_post.signature_envelope.is_none());
    }

    #[test]
    fn test_new_post_with_signature() {
        let envelope = serde_json::json!({
            "version": "1.0",
            "type": "signature-envelope",
            "signer": "did:key:z6Mk...",
            "signature": "base64..."
        });
        let new_post = NewPost {
            user_id: Uuid::new_v4(),
            content: "Hello world".to_string(),
            signature_envelope: Some(envelope.clone()),
        };
        assert_eq!(new_post.signature_envelope, Some(envelope));
    }
}
