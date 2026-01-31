// OpenClaw Crypto - Cryptographic primitives for Protocol M

pub mod did;
pub mod hash;
pub mod jcs;
pub mod types;

pub use did::pubkey_to_did;
pub use hash::sha256_hex;
pub use jcs::jcs_canonical_bytes;
pub use types::{ArtifactInfo, HashRef, SignatureEnvelopeV1};
