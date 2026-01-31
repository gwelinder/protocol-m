// OpenClaw Crypto - Cryptographic primitives for Protocol M

pub mod hash;
pub mod types;

pub use hash::sha256_hex;
pub use types::{ArtifactInfo, HashRef, SignatureEnvelopeV1};
