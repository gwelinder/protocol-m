//! OpenClaw Server - API for Protocol M
//!
//! This crate provides the REST API server for Protocol M's attribution
//! and artifact registration system.

pub mod db;
pub mod models;
pub mod error;

pub use error::AppError;
