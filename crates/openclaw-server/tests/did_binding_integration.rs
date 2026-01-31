//! Integration tests for DID binding flow.
//!
//! These tests verify the complete end-to-end flow of binding a DID
//! to a user account using the identity API endpoints.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use openclaw_crypto::pubkey_to_did;
use openclaw_server::{create_router, db};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// Creates a test database pool using the TEST_DATABASE_URL env var.
/// Falls back to a local test database if not set.
async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/openclaw_test".to_string());

    let pool = db::create_pool(&database_url)
        .await
        .expect("Failed to create test database pool");

    // Run migrations to ensure tables exist
    db::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

/// Helper to parse JSON response body.
async fn json_body(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&body).expect("Failed to parse JSON response")
}

/// Creates a test keypair and returns (SigningKey, DID).
fn create_test_identity() -> (SigningKey, String) {
    // Use a deterministic seed for reproducible tests
    let seed: [u8; 32] = [0x99; 32];
    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    let did = pubkey_to_did(&verifying_key);
    (signing_key, did)
}

/// Signs a challenge with a signing key and returns base64-encoded signature.
fn sign_challenge(signing_key: &SigningKey, challenge_hex: &str) -> String {
    let challenge_bytes = hex::decode(challenge_hex)
        .expect("Failed to decode challenge hex");
    let signature = signing_key.sign(&challenge_bytes);
    base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
}

/// Tests the complete end-to-end DID binding flow:
/// 1. Create user account (UUID)
/// 2. Request challenge
/// 3. Sign challenge with OpenClaw identity
/// 4. Submit bind request
/// 5. Verify DID stored in database
///
/// Requires TEST_DATABASE_URL environment variable or local PostgreSQL.
/// Run with: cargo test --test did_binding_integration -- --ignored
#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_complete_did_binding_flow() {
    let pool = create_test_pool().await;
    let app = create_router(pool.clone());

    // Step 1: Create a test user (just a random UUID for now)
    let user_id = Uuid::new_v4();

    // Step 2: Create a test identity with OpenClaw crypto
    let (signing_key, did) = create_test_identity();

    // Step 3: Request a challenge
    let challenge_request = json!({
        "user_id": user_id
    });

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/identity/challenge")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&challenge_request).unwrap()))
                .unwrap(),
        )
        .await
        .expect("Failed to send challenge request");

    assert_eq!(response.status(), StatusCode::OK);

    let challenge_json = json_body(response).await;
    let challenge = challenge_json["challenge"]
        .as_str()
        .expect("Challenge not in response");

    // Verify challenge is a 64-character hex string
    assert_eq!(challenge.len(), 64);
    assert!(challenge.chars().all(|c| c.is_ascii_hexdigit()));

    // Step 4: Sign the challenge with our identity
    let signature = sign_challenge(&signing_key, challenge);

    // Step 5: Submit the bind request
    let bind_request = json!({
        "userId": user_id,
        "did": did,
        "challenge": challenge,
        "challengeSignature": signature
    });

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/identity/bind")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&bind_request).unwrap()))
                .unwrap(),
        )
        .await
        .expect("Failed to send bind request");

    assert_eq!(response.status(), StatusCode::OK);

    let bind_json = json_body(response).await;
    assert_eq!(bind_json["did"], did);
    assert!(bind_json["message"].as_str().unwrap().contains("successfully"));

    // Step 6: Verify DID is stored in database
    let binding: Option<(String, Uuid)> = sqlx::query_as(
        r#"
        SELECT did, user_id
        FROM did_bindings
        WHERE did = $1 AND user_id = $2 AND revoked_at IS NULL
        "#
    )
    .bind(&did)
    .bind(user_id)
    .fetch_optional(&pool)
    .await
    .expect("Failed to query did_bindings");

    assert!(binding.is_some(), "DID binding not found in database");
    let (stored_did, stored_user_id) = binding.unwrap();
    assert_eq!(stored_did, did);
    assert_eq!(stored_user_id, user_id);

    // Cleanup: Remove test data
    sqlx::query("DELETE FROM did_bindings WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("Failed to cleanup did_bindings");

    sqlx::query("DELETE FROM did_challenges WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("Failed to cleanup did_challenges");
}

#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_binding_with_invalid_signature_fails() {
    let pool = create_test_pool().await;
    let app = create_router(pool.clone());

    let user_id = Uuid::new_v4();
    let (_signing_key, did) = create_test_identity();

    // Request a challenge
    let challenge_request = json!({
        "user_id": user_id
    });

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/identity/challenge")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&challenge_request).unwrap()))
                .unwrap(),
        )
        .await
        .expect("Failed to send challenge request");

    let challenge_json = json_body(response).await;
    let challenge = challenge_json["challenge"].as_str().unwrap();

    // Create a WRONG signature (use different key)
    let wrong_seed: [u8; 32] = [0xAA; 32];
    let wrong_signing_key = SigningKey::from_bytes(&wrong_seed);
    let wrong_signature = sign_challenge(&wrong_signing_key, challenge);

    // Try to bind with wrong signature
    let bind_request = json!({
        "userId": user_id,
        "did": did,
        "challenge": challenge,
        "challengeSignature": wrong_signature
    });

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/identity/bind")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&bind_request).unwrap()))
                .unwrap(),
        )
        .await
        .expect("Failed to send bind request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let error_json = json_body(response).await;
    assert!(error_json["error"].as_str().unwrap().contains("verification failed"));

    // Cleanup
    sqlx::query("DELETE FROM did_challenges WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("Failed to cleanup did_challenges");
}

#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_binding_expired_challenge_fails() {
    let pool = create_test_pool().await;
    let app = create_router(pool.clone());

    let user_id = Uuid::new_v4();
    let (signing_key, did) = create_test_identity();

    // Manually insert an expired challenge
    let challenge_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let expired_time = chrono::Utc::now() - chrono::Duration::hours(1);

    sqlx::query(
        r#"
        INSERT INTO did_challenges (id, user_id, challenge, expires_at, created_at, failed_attempts)
        VALUES ($1, $2, $3, $4, $5, 0)
        "#
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(challenge_hex)
    .bind(expired_time)
    .bind(chrono::Utc::now())
    .execute(&pool)
    .await
    .expect("Failed to insert expired challenge");

    // Try to bind with expired challenge
    let signature = sign_challenge(&signing_key, challenge_hex);
    let bind_request = json!({
        "userId": user_id,
        "did": did,
        "challenge": challenge_hex,
        "challengeSignature": signature
    });

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/identity/bind")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&bind_request).unwrap()))
                .unwrap(),
        )
        .await
        .expect("Failed to send bind request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let error_json = json_body(response).await;
    assert!(error_json["error"].as_str().unwrap().contains("expired"));

    // Cleanup
    sqlx::query("DELETE FROM did_challenges WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("Failed to cleanup did_challenges");
}

#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_challenge_is_marked_used_after_binding() {
    let pool = create_test_pool().await;
    let app = create_router(pool.clone());

    let user_id = Uuid::new_v4();
    let (signing_key, did) = create_test_identity();

    // Request and complete binding
    let challenge_request = json!({ "user_id": user_id });

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/identity/challenge")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&challenge_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let challenge_json = json_body(response).await;
    let challenge = challenge_json["challenge"].as_str().unwrap();
    let signature = sign_challenge(&signing_key, challenge);

    let bind_request = json!({
        "userId": user_id,
        "did": did,
        "challenge": challenge,
        "challengeSignature": signature
    });

    // First bind should succeed
    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/identity/bind")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&bind_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify challenge is marked as used
    let used_at: Option<(Option<chrono::DateTime<chrono::Utc>>,)> = sqlx::query_as(
        "SELECT used_at FROM did_challenges WHERE challenge = $1"
    )
    .bind(challenge)
    .fetch_optional(&pool)
    .await
    .expect("Failed to query challenge");

    assert!(used_at.is_some(), "Challenge not found");
    assert!(used_at.unwrap().0.is_some(), "Challenge should be marked as used");

    // Cleanup
    sqlx::query("DELETE FROM did_bindings WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("Failed to cleanup");
    sqlx::query("DELETE FROM did_challenges WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
#[ignore = "requires PostgreSQL database"]
async fn test_cannot_reuse_challenge() {
    let pool = create_test_pool().await;
    let app = create_router(pool.clone());

    // Use different user and identity for this test to avoid conflicts
    let user_id = Uuid::new_v4();
    let seed: [u8; 32] = [0xBB; 32];
    let signing_key = SigningKey::from_bytes(&seed);
    let did = pubkey_to_did(&signing_key.verifying_key());

    // Request a challenge
    let challenge_request = json!({ "user_id": user_id });

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/identity/challenge")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&challenge_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let challenge_json = json_body(response).await;
    let challenge = challenge_json["challenge"].as_str().unwrap();
    let signature = sign_challenge(&signing_key, challenge);

    let bind_request = json!({
        "userId": user_id,
        "did": did,
        "challenge": challenge,
        "challengeSignature": signature
    });

    // First bind succeeds
    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/identity/bind")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&bind_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Try to reuse the same challenge with a different DID
    let seed2: [u8; 32] = [0xCC; 32];
    let signing_key2 = SigningKey::from_bytes(&seed2);
    let did2 = pubkey_to_did(&signing_key2.verifying_key());
    let signature2 = sign_challenge(&signing_key2, challenge);

    let bind_request2 = json!({
        "userId": user_id,
        "did": did2,
        "challenge": challenge,
        "challengeSignature": signature2
    });

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/identity/bind")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&bind_request2).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let error_json = json_body(response).await;
    assert!(error_json["error"].as_str().unwrap().contains("already been used"));

    // Cleanup
    sqlx::query("DELETE FROM did_bindings WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("Failed to cleanup");
    sqlx::query("DELETE FROM did_challenges WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("Failed to cleanup");
}
