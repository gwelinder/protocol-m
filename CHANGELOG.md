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
- US-002C: Implemented sign command handler (--meta and --dry-run flags, identity loading, signing)
- US-002D: Implemented signature output in normal mode (<file>.sig.json with 0644 perms)
- US-002E: Implemented signature output in dry-run mode (JSON to stdout)
- US-002F: Added signing roundtrip tests (4 tests: basic, metadata, tamper detection, different keys)
- US-003A: Implemented envelope verification logic (verify_artifact function with 9 unit tests)
- US-003B: Implemented DID to public key extraction (did_to_verifying_key function with 6 unit tests)
- US-003C: Implemented verify command handler (--sig flag, envelope parsing, verification)
- US-003D: Added colored verification output (green ✓ / red ✗, truncated DID display)
- US-003E: Added local identity indicator (cyan "Local Identity" / yellow "External Identity")
- US-003F: Added tamper detection test (verify_artifact with hash mismatch error check)
- US-004A: Defined manifest data structure (ContributionManifest, ArtifactReference types)
- US-004B: Implemented manifest export logic (export_manifest function with 5 unit tests)
- US-004C: Implemented manifest export command (--output flag, --path args, directory scanning)

#### Attribution Database (US-005A to US-005C) — Server Layer
- US-005A: Created openclaw-server crate with axum 0.8, sqlx 0.8, PostgreSQL
- US-005A: Created artifacts table migration with UUID PK, hash, DID, timestamp, metadata (JSONB), signature
- US-005A: Added database pool creation and migration runner
- US-005A: Defined Artifact and NewArtifact model types with sqlx FromRow
- US-005B: Implemented POST /api/v1/artifacts endpoint for artifact registration
- US-005B: Added routes module with router setup and artifacts handler
- US-005B: Parse SignatureEnvelopeV1, extract fields, insert into database, return ID and URL
- US-005C: Added artifact registration validation (signature verification, DID parsing, metadata validation)
- US-005C: Added duplicate hash detection to prevent registering same artifact twice
- US-005C: Added metadata size limit (10KB max) and structure validation (must be JSON object)

#### Derivation Tracking (US-006A to US-006C) — Attribution Relationships
- US-006A: Created artifact_derivations table migration with foreign keys to artifacts
- US-006A: Added unique constraint on (artifact_id, derived_from_id) pair
- US-006A: Added indexes for efficient parent/child artifact lookups
- US-006A: Defined ArtifactDerivation and NewArtifactDerivation model types
- US-006B: Implemented derivation declaration during artifact registration
- US-006B: Parse metadata.derivedFrom field (string or array of IDs/hashes)
- US-006B: Resolve artifact references by UUID or content hash
- US-006B: Insert derivation records with ON CONFLICT handling
- US-006C: Implemented cycle detection using depth-first search
- US-006C: Limited search depth to 100 to prevent DoS attacks
- US-006C: Return 400 error with descriptive message when cycle detected

#### Attribution Query (US-007A to US-007B) — Graph Traversal
- US-007A: Implemented GET /api/v1/artifacts/{id}/attribution endpoint
- US-007A: Added depth parameter (default 1, max 10) for recursive traversal
- US-007A: BFS traversal with visited set for cycle prevention in output
- US-007A: Returns parent artifacts with DID, timestamp, description, metadata
- US-007A: Limited to 100 results per depth level for performance
- US-007B: Added DepthLevel struct to group results by depth
- US-007B: Response includes both flat 'parents' array and grouped 'levels' array
- US-007B: Results ordered by timestamp DESC within each level

#### DID Binding (US-008A) — Identity Linking
- US-008A: Created did_bindings table migration for linking DIDs to user accounts
- US-008A: Added indexes on user_id and did columns for fast lookups
- US-008A: Added unique constraint on active (non-revoked) DID bindings
- US-008A: Defined DidBinding and NewDidBinding model types with is_active() helper

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
