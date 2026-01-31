// OpenClaw Crypto - Cryptographic primitives for Protocol M

pub mod did;
pub mod hash;
pub mod jcs;
pub mod keys;
pub mod sign;
pub mod types;

pub use did::pubkey_to_did;
pub use hash::sha256_hex;
pub use jcs::jcs_canonical_bytes;
pub use keys::generate_keypair;
pub use sign::sign_artifact;
pub use types::{ArtifactInfo, HashRef, SignatureEnvelopeV1};
