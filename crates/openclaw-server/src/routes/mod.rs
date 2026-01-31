//! API routes for OpenClaw server.

pub mod artifacts;
pub mod identity;

use axum::Router;
use sqlx::PgPool;

/// Creates the main API router with all routes mounted.
pub fn create_router(pool: PgPool) -> Router {
    Router::new().nest("/api/v1", api_v1_routes(pool))
}

/// Creates the v1 API routes.
fn api_v1_routes(pool: PgPool) -> Router {
    Router::new()
        .nest("/artifacts", artifacts::router(pool.clone()))
        .nest("/identity", identity::router(pool))
}
