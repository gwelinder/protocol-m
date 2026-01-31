//! OpenClaw Server - API for Protocol M
//!
//! This crate provides the REST API server for Protocol M's attribution
//! and artifact registration system.

pub mod db;
pub mod error;
pub mod models;
pub mod routes;

pub use error::AppError;
pub use routes::create_router;
