//! API routes for OpenClaw server.

pub mod approvals;
pub mod artifacts;
pub mod bounties;
pub mod credits;
pub mod disputes;
pub mod identity;
pub mod posts;
pub mod profile;
pub mod reputation;

use axum::Router;
use sqlx::PgPool;

/// Creates the main API router with all routes mounted.
pub fn create_router(pool: PgPool) -> Router {
    Router::new().nest("/api/v1", api_v1_routes(pool))
}

/// Creates the v1 API routes.
fn api_v1_routes(pool: PgPool) -> Router {
    Router::new()
        .nest("/approvals", approvals::router(pool.clone()))
        .nest("/artifacts", artifacts::router(pool.clone()))
        .nest("/bounties", bounties::router(pool.clone()))
        .nest("/credits", credits::router(pool.clone()))
        .nest("/disputes", disputes::router(pool.clone()))
        .nest("/identity", identity::router(pool.clone()))
        .nest("/posts", posts::router(pool.clone()))
        .nest("/profile", profile::router(pool))
}
