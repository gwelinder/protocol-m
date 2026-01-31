# Changelog

All notable changes to Protocol M.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Core Infrastructure (US-001A to US-001K) ✓
- US-001A: Created Rust workspace structure with `openclaw-crypto` and `openclaw-cli` crates
- US-001B: Added core cryptographic dependencies (ed25519-dalek, serde_jcs, sha2)
- US-001C: Defined signature envelope types (HashRef, ArtifactInfo, SignatureEnvelopeV1)
- US-001D: Implemented SHA-256 hashing utility
- US-001E: Implemented JCS canonicalization (RFC 8785)
- US-001F: Implemented DID key derivation (did:key format)
- US-001G: Implemented keypair generation with ed25519-dalek and OsRng
- US-001H: Added age encryption dependency to CLI crate for private key protection
- US-001I: Implemented private key encryption with age passphrase protection
- US-001J: Implemented private key decryption with graceful wrong passphrase handling
- US-001K: Implemented file permission checks (0700 for dirs, 0600 for keyfiles, Windows bypass)
- US-001L: Implemented identity initialization logic (init_identity function, passphrase prompting, encrypted key storage)
- US-001M: Added CLI scaffolding with clap (Cli struct, Commands enum, subcommand routing)
- US-001N: Wired identity init command to CLI (connects handler to keystore::init_identity, displays DID on success)
- US-001O: Created golden test vector fixture (fixtures/golden_vectors.json with seed, DID, envelope, canonical JCS, signature)
- US-001P: Added integration test for golden vector (tests/golden.rs with 6 validation tests)
- US-002A: Implemented envelope signing logic (sign_artifact function in sign.rs)
- US-002B: Added metadata parsing for --meta flag (parse_metadata with dot notation support)
- Project scaffolding and fixtures directory
- Golden test vector for CI validation
- Moltbook integration documentation

#### Oracle Post-AGI Enhancement (US-043A to US-044S) — 45 stories
- Token economics and compute-backed M-Credits specification
- Proof-of-Execution mining and validation protocols
- Quadratic voting for governance proposals
- Trust Escalation Ladder (TEL) T0-T4 autonomy levels
- Oracle integration framework for deep analysis
- Compute basket backing with multi-provider redundancy
- Slashing conditions and dispute arbitration
- Reputation decay curves and anti-gaming measures

#### Dollar Transition Infrastructure (US-045A to US-050C) — 20 stories
- US-045A: Compute basket specification and pricing oracle
- US-045B: Provider bond staking and SLA enforcement
- US-046A: Enterprise treasury management integration
- US-046B: Regulated custody partnerships
- US-047A: Government compliance sandbox
- US-047B: Treasury debt settlement pilot
- US-048A: Central bank integration APIs
- US-049A: Cross-border settlement protocol
- US-050A: Multi-currency bridge architecture
- US-050B: Legacy SWIFT/ACH interoperability
- US-050C: Real-time gross settlement (RTGS) compatibility

#### Moltbook Engagement Loop (US-051A to US-051H) — 8 stories
- US-051A: Implement Moltbook engagement heartbeat loop
- US-051B: Implement multi-AI response enhancement (Claude/Oracle/Gemini)
- US-051C: Implement engagement analytics dashboard
- US-051D: Implement community feedback → PRD pipeline
- US-051E: Implement Protocol M explainer series (6 parts)
- US-051F: Implement collaboration detection and outreach
- US-051G: Implement Protocol M signature footer
- US-051H: Implement thread follow-up monitoring

#### PRD Expansion
- Expanded PRD from 160 → 233 user stories
- Created `scripts/ralph-engage/` directory with autonomous engagement agent
- Created `scripts/ralph-engage/CLAUDE.md` — engagement loop instructions
- Created `scripts/ralph-engage/prd.json` — 15 engagement-specific stories (ENG-001 to ENG-015)
- Added `ORACLE_POST_AGI_RESPONSE.md` — full Oracle analysis
- Added `ORACLE_DOLLAR_TRANSITION.md` — dollar reserve transition strategy

### Changed
- Updated CLAUDE.md with Moltbook engagement guidelines
- Added changelog requirements to Ralph workflow
- Enhanced Ralph instructions with API quick reference and footer signature

### Community
- Engaged on Moltbook: security thread (signed skills), economics threads (M-Credits/M-Reputation)
- Created m/protocol-m submolt for community discussion
- Posted 3 organic comments on relevant threads
- Initiated autonomous engagement loop for consistent community presence

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
