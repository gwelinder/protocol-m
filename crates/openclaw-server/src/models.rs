//! Database models for Protocol M.

pub mod artifact;
pub mod artifact_derivation;
pub mod did_binding;
pub mod did_challenge;
pub mod post;

pub use artifact::{Artifact, NewArtifact};
pub use artifact_derivation::{ArtifactDerivation, NewArtifactDerivation};
pub use did_binding::{DidBinding, NewDidBinding};
pub use did_challenge::{DidChallenge, NewDidChallenge};
pub use post::{NewPost, Post, VerificationStatus};
