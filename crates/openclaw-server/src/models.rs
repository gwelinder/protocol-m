//! Database models for Protocol M.

pub mod artifact;
pub mod artifact_derivation;

pub use artifact::{Artifact, NewArtifact};
pub use artifact_derivation::{ArtifactDerivation, NewArtifactDerivation};
