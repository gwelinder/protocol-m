# Changelog

All notable changes to Protocol M.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- US-001A: Created Rust workspace structure with `openclaw-crypto` and `openclaw-cli` crates
- US-001B: Added core cryptographic dependencies (ed25519-dalek, serde_jcs, sha2)
- US-001C: Defined signature envelope types (HashRef, ArtifactInfo, SignatureEnvelopeV1)
- US-001D: Implemented SHA-256 hashing utility
- US-001E: Implemented JCS canonicalization (RFC 8785)
- US-001F: Implemented DID key derivation (did:key format)
- US-001G: Implemented keypair generation with ed25519-dalek and OsRng
- US-001H: Added age encryption dependency to CLI crate for private key protection
- US-001I: Implemented private key encryption with age passphrase protection
- Project scaffolding and fixtures directory
- Golden test vector for CI validation
- Moltbook integration documentation

### Changed
- Updated CLAUDE.md with Moltbook engagement guidelines
- Added changelog requirements to Ralph workflow

### Community
- Engaged on Moltbook: security thread (signed skills), economics threads (M-Credits/M-Reputation)
- Created m/protocol-m submolt for community discussion

---

## [0.1.0-alpha] - TBD

### Planned
- `openclaw identity init` — Generate Ed25519 keypair, derive DID, encrypt at rest
- `openclaw sign <file>` — Produce RFC 8785 canonical signature envelope
- `openclaw verify <file>` — Offline verification with hash + signature check
- `openclaw sign-message` — Domain-separated message signing for Moltbook binding
- Moltbook DID binding via challenge-response
- Verified post badges

---

## Footer Signature

When Protocol M is shipped, all signed artifacts will include:

```
---
Signed with Protocol M (https://github.com/openclaw-protocol/openclaw)
DID: did:key:z6Mk...
Verify: openclaw verify <file> <sig.json>
```

This signature promotes the protocol while providing verification instructions.
