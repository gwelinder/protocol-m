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

#### DID Binding (US-008A to US-008C) — Identity Linking
- US-008A: Created did_bindings table migration for linking DIDs to user accounts
- US-008A: Added indexes on user_id and did columns for fast lookups
- US-008A: Added unique constraint on active (non-revoked) DID bindings
- US-008A: Defined DidBinding and NewDidBinding model types with is_active() helper
- US-008B: Created did_challenges table migration for secure DID binding flow
- US-008B: Added indexes on challenge, expires_at, and user_id for efficient lookups
- US-008B: Defined DidChallenge and NewDidChallenge model types with is_valid/is_used/is_expired helpers
- US-008C: Implemented POST /api/v1/identity/challenge endpoint for DID binding flow
- US-008C: Added identity routes module with router and create_challenge handler
- US-008C: Generates random 32-byte challenge (hex-encoded), 10-minute expiry
- US-008D: Implemented POST /api/v1/identity/bind endpoint for linking DIDs to accounts
- US-008D: Added signature verification over challenge bytes
- US-008D: Added transaction handling for atomic binding creation and challenge consumption
- US-008D: Added 8 unit tests for signature verification and validation
- US-008E: Added rate limiting for challenge endpoint (5 challenges per user per hour)
- US-008E: Added TooManyRequests error variant with Retry-After header support
- US-008E: Added 2 unit tests for rate limit configuration and result struct
- US-008F: Added rate limiting for bind endpoint (3 attempts per challenge)
- US-008F: Added failed_attempts column to did_challenges table
- US-008F: Added is_locked() helper to DidChallenge model
- US-008F: Added increment_failed_attempts function for atomic counter updates
- US-008F: Returns 429 with descriptive message when challenge locked
- US-008F: Added 4 unit tests for bind attempt limiting
- US-008G: Added integration tests for DID binding flow (5 tests covering full flow, invalid signature, expired challenge, used challenge, reuse prevention)
- US-008G: Tests marked #[ignore] requiring PostgreSQL database
- US-008G: Run with: `cargo test --test did_binding_integration -- --ignored`

#### User Profile DID Integration (US-009A to US-009C) — Profile Query & UI
- US-009A: Created GET /api/v1/profile/{user_id}/dids endpoint for retrieving bound DIDs
- US-009A: Returns array of BoundDid objects with did and createdAt fields
- US-009A: Filters revoked bindings (WHERE revoked_at IS NULL)
- US-009A: Orders by created_at DESC (newest first)

#### Frontend UI Components (US-009B to US-009C) — React/Next.js
- US-009B: Created Next.js frontend scaffold (web/ directory with TypeScript)
- US-009B: Implemented IdentityBadge component with:
  - Middle truncation (did:key:z6Mk...Wp format)
  - Tooltip showing full DID on hover
  - Click-to-copy functionality with clipboard API
  - Binding timestamp display
- US-009B: Added utility functions (truncateDid, formatTimestamp, copyToClipboard)
- US-009B: TypeScript typecheck passes
- US-009C: Created ProfileIdentities component with:
  - IdentityBadge integration for all bound DIDs
  - Expand/collapse for 5+ DIDs (configurable maxVisible)
  - Empty state with "No identity bound" message
  - "Bind DID" button linking to instructions
- US-009C: Created profile page at /profile/[userId]
- US-009C: Created bind-identity instructions page at /bind-identity

#### Post Signature Verification (US-010A to US-010C) — Database Schema, Logic & API
- US-010A: Created migration for posts table signature fields
- US-010A: Added verification_status enum (none, invalid, valid_unbound, valid_bound)
- US-010A: Added signature_envelope_json (JSONB nullable) column
- US-010A: Added verified_did (text nullable) column
- US-010A: Added indexes on verification_status and verified_did
- US-010A: Created Post, NewPost, and VerificationStatus model types with 5 unit tests
- US-010B: Implemented verify_post_signature function with:
  - SHA-256 hash recomputation and verification
  - JCS canonicalization for signature verification
  - DID extraction from signature envelope
  - Database check for DID binding to user
  - Returns VerificationResult with status and DID
- US-010B: Added 13 unit tests for verification logic
- US-010C: Created POST /api/v1/posts endpoint:
  - Accepts optional signatureEnvelope field in request body
  - Calls verify_post_signature when envelope provided
  - Stores envelope JSON, verified_did, and verification_status
  - Returns post with verification status in response

#### Verified Post UI (US-011A to US-011C) — Frontend Display
- US-011A: Created VerifiedBadge component with:
  - Green checkmark + "Verified" text for valid_bound status
  - "Signed" text (no checkmark) for valid_unbound status
  - Nothing rendered for none or invalid status
  - Hover tooltip showing truncated signer DID
- US-011B: Created PostCard component with:
  - VerifiedBadge integration positioned next to author name
  - Author avatar, name, username, and timestamp display
  - Post content with pre-wrap formatting
  - Upvote and comment count buttons in footer
  - Props for verification_status and verified_did
- US-011C: Created SignatureModal component with:
  - Full envelope JSON display with syntax highlighting (keys purple, strings green, numbers blue)
  - Summary fields showing DID, hash, timestamp, signature with truncation
  - Copy JSON button with visual feedback on success
  - Opens when clicking VerifiedBadge (requires signatureEnvelope prop)
  - Escape key and backdrop click to close modal
  - Updated PostCard to pass signatureEnvelope to VerifiedBadge

#### M-Credits Economy (US-012A to US-012C) — Token Accounts, Ledger & Invoices
- US-012A: Created m_credits_accounts table migration:
  - NUMERIC(20,8) for precise decimal handling (up to 999B credits with 8 decimal places)
  - Unique constraint on DID (one account per identity)
  - Non-negative balance constraints (balance >= 0, promo_balance >= 0)
  - Automatic updated_at trigger for audit trail
  - Index on did for fast account lookups
- US-012A: Implemented MCreditsAccount model with:
  - total_balance() helper combining balance + promo_balance
  - has_sufficient_balance() check for payment authorization
  - NewMCreditsAccount with zero-default balances
  - 4 unit tests for balance calculations
- US-012B: Created m_credits_ledger table migration for event sourcing:
  - m_credits_event_type enum (mint, burn, transfer, hold, release)
  - NUMERIC(20,8) amount field with positive constraint
  - from_did and to_did fields for transaction parties
  - metadata JSONB for flexible transaction context
  - Indexes on event_type, from_did, to_did, created_at for efficient queries
  - Composite index for DID history queries
- US-012B: Implemented MCreditsLedger model with:
  - MCreditsEventType enum with serde and sqlx serialization
  - NewMCreditsLedger with factory methods (mint, burn, transfer, hold, release)
  - 7 unit tests for event type serialization and factory methods
- US-012C: Created purchase_invoices table migration:
  - payment_provider enum (stripe, usdc, apple_pay, manual)
  - invoice_status enum (pending, completed, failed)
  - NUMERIC(10,2) for USD amounts, NUMERIC(20,8) for credits
  - Indexes on user_id, status, created_at, external_ref
  - Automatic updated_at trigger for tracking state changes
- US-012C: Implemented PurchaseInvoice model with:
  - PaymentProvider and InvoiceStatus enums with serde/sqlx mapping
  - Status helper methods (is_pending, is_completed, is_failed)
  - NewPurchaseInvoice for creating new invoice records
  - 6 unit tests for serialization and status helpers
- US-012D: Implemented POST /api/v1/credits/purchase endpoint:
  - PurchaseCreditsRequest/Response types with camelCase JSON serialization
  - Credit calculation: 1 USD = 100 M-credits (configurable rate constant)
  - Amount validation: $1-$10,000 USD bounds with positivity check
  - Payment provider parsing: Stripe (default), USDC, Apple Pay
  - Invoice creation with status=pending in purchase_invoices table
  - Placeholder Stripe Checkout URL generation (production would use real Stripe API)
  - 22 unit tests for helper functions, validation, and serialization
- US-012E: Implemented POST /api/v1/credits/webhook/stripe endpoint:
  - Stripe webhook handler for checkout.session.completed and checkout.session.expired events
  - StripeWebhookRequest/Response types for parsing webhook payloads
  - Invoice ID extraction from metadata.invoice_id or client_reference_id
  - Pending invoice loading and validation before processing
  - mint_credits_to_did function: inserts ledger entry and upserts account balance atomically
  - complete_invoice and fail_invoice functions for status updates
  - Placeholder signature verification (production uses stripe crate for real verification)
  - External payment reference tracking in ledger metadata
  - 17 unit tests for event parsing, invoice ID extraction, and response serialization
- US-012F: Implemented promo credit grants:
  - Added promo_mint event type to MCreditsEventType enum with serde/sqlx serialization
  - Migration 20260131000010 adds promo_mint value to PostgreSQL enum
  - grant_promo_credits function with 100 credits lifetime limit per DID
  - Validates total promo credits across all ledger entries before granting
  - POST /api/v1/credits/grant-promo admin endpoint with optional expiry support
  - Updates promo_balance in m_credits_accounts (separate from transferable balance)
  - Expiry timestamp stored in ledger entry metadata for future expiration handling
  - 13 unit tests for DID validation, request/response serialization, and promo mint
- US-012G: Implemented reserve attestation endpoint:
  - GET /api/v1/credits/reserves for transparent reserve backing visibility
  - Returns total outstanding credits (main + promo balances)
  - Returns total USD reserves from completed purchase invoices
  - Calculates reserve_coverage_ratio (only main balance is backed, promo not counted)
  - Includes account_count and invoice_count for context
  - Includes ISO 8601 timestamp with millisecond precision
  - Generates SHA-256 attestation hash for verification
  - Placeholder cryptographic signature (production would use server Ed25519 key)
  - Handles edge cases: zero credits (ratio=1), reserves without credits (ratio=999999)
  - 14 unit tests for coverage ratio calculation, serialization, and timestamp format

#### Bounty Marketplace (US-013A to US-016F-R) — In Progress
- US-013A: Created bounties table for task marketplace:
  - Migration 20260131000011 with bounty_closure_type enum (tests, quorum, requester)
  - bounty_status enum (open, in_progress, completed, cancelled)
  - bounties table with NUMERIC(20,8) reward_credits, JSONB metadata, optional deadline
  - Indexes on poster_did, status, deadline, created_at, plus composite marketplace index
  - Bounty and NewBounty Rust model structs with factory methods
  - Helper methods: is_open, is_active, is_expired, uses_tests, uses_quorum, uses_requester
  - Metadata extraction helpers: eval_harness_hash, reviewer_count, min_reviewer_rep
  - 10 unit tests for serialization, status helpers, and metadata extraction
- US-013B: Created escrow_holds table for bounty payment locking:
  - Migration 20260131000012 with escrow_status enum (held, released, cancelled)
  - escrow_holds table with bounty_id FK, holder_did, NUMERIC(20,8) amount, timestamps
  - Indexes on bounty_id, holder_did, status, plus composite index for active escrows
  - EscrowHold, NewEscrowHold, EscrowStatus Rust model structs
  - Helper methods: is_held, is_released, is_cancelled, is_finalized
  - 5 unit tests for serialization and status helpers
- US-013C: Implemented POST /api/v1/bounties endpoint for bounty posting:
  - CreateBountyRequest/Response types with camelCase JSON serialization
  - Title and description validation (length limits, non-empty)
  - Reward amount validation (1-1,000,000 M-credits)
  - Closure-type-specific metadata validation:
    - Tests: requires evalHarnessHash in metadata
    - Quorum: requires reviewerCount (≥1) and minReviewerRep (≥0)
    - Requester: no special metadata required
  - DID binding verification (user must have active bound DID)
  - Balance sufficiency check (main + promo balance)
  - Escrow hold creation: ledger entry + escrow_holds record + balance deduction
  - Bounty record creation with status=open
  - Returns bounty_id, escrow_id, ledger_id on success
  - 35 unit tests for validation, serialization, and balance checks
- US-014A: Created bounty_submissions table for work submission tracking:
  - Migration 20260131000013 with submission_status enum (pending, approved, rejected)
  - bounty_submissions table with bounty_id FK, submitter_did, artifact_hash, signature_envelope JSONB
  - Optional execution_receipt JSONB for test-based bounty verification
  - Indexes on bounty_id, submitter_did, status, plus composite index for pending submissions
  - BountySubmission, NewBountySubmission, SubmissionStatus Rust model structs
  - Factory methods: with_execution_receipt, without_execution_receipt
  - Helper methods: is_pending, is_approved, is_rejected, has_execution_receipt
  - Execution receipt helpers: execution_harness_hash, all_tests_passed, test_results
  - 8 unit tests for serialization, status helpers, and execution receipt extraction
- US-014B: Implemented bounty submission endpoint POST /api/v1/bounties/{id}/submit:
  - Added route POST /api/v1/bounties/{id}/submit to bounties router
  - Requires authentication and DID binding (get_user_bound_did validation)
  - Accepts SignatureEnvelopeV1 (signature_envelope) with optional execution_receipt
  - Verifies envelope signature cryptographically (ed25519, JCS canonicalization)
  - Validates envelope version (1.0), type (signature-envelope/contribution-manifest), algo (ed25519), hash algo (sha-256)
  - Verifies envelope signer matches submitter's bound DID
  - For test-based bounties: validates execution_receipt contains harness_hash and all_tests_passed
  - Supports both snake_case and camelCase fields in execution_receipt
  - Validates bounty is open and not expired before accepting submissions
  - Inserts submission with status=pending
  - Returns submission_id, bounty_id, submitter_did, status
  - 20 unit tests for request/response serialization, execution receipt validation, envelope parsing, and signature verification
  - 244 total tests pass
- US-014C: Implemented test-based auto-approval for bounty submissions:
  - Added TestVerificationResult enum (Approved, HarnessHashMismatch, TestsFailed)
  - Added verify_test_submission function to check harness hash match and all_tests_passed
  - Added get_receipt_harness_hash and get_receipt_all_tests_passed helpers (support both snake_case and camelCase)
  - Added update_submission_status function for approval/rejection
  - Modified submit_bounty to auto-approve/reject test-based submissions immediately
  - Extended SubmitBountyResponse with optional auto_approved and message fields
  - Auto-approves when: harness_hash matches bounty's eval_harness_hash AND all_tests_passed is true
  - Auto-rejects when: harness_hash mismatch OR all_tests_passed is false/missing
  - Returns informative messages explaining approval/rejection reasons
  - 14 unit tests for verification logic, helper functions, and result equality
  - 258 total tests pass
- US-014D: Implemented escrow release on bounty approval:
  - Added release_escrow function performing atomic operations:
    - Marks escrow_hold status as 'released' with released_at timestamp
    - Inserts release ledger entry (NewMCreditsLedger::release)
    - Updates recipient's m_credits_accounts balance via upsert (ON CONFLICT DO UPDATE)
    - Updates bounty status to 'completed'
  - Added mint_reputation_for_submission function with closure-type weighted reputation:
    - Tests closure: 1.5x weight (automated verification, highest trust)
    - Quorum closure: 1.2x weight (peer-reviewed)
    - Requester closure: 1.0x weight (single approver)
    - Base formula: reward_credits * 0.1 * closure_type_weight
    - Records reputation as 0-amount mint with metadata (full m_reputation table deferred to US-016A)
  - Wired escrow release and reputation minting into submit_bounty auto-approval flow
  - Updated success message to indicate escrow released
  - 11 new unit tests for escrow release metadata, reputation weights, and response serialization
  - 267 total tests pass
- US-014E: Implemented attribution recording for bounty completions:
  - Added migration 20260131000014 adding artifact_id column to bounty_submissions table
  - Added artifact_id field to BountySubmission model with has_artifact() and artifact_id() helpers
  - Added register_submission_artifact function that:
    - Checks if artifact already exists by hash before registering
    - Registers new artifacts with signature envelope if not found
    - Enriches metadata with bounty context (bounty_id, bounty_title)
    - Checks for parent_artifact_id in bounty metadata (supports snake_case and camelCase)
    - Creates derivation links with cycle detection
    - Updates submission with artifact_id reference
  - Helper functions: get_parent_artifact_from_metadata, resolve_parent_artifact, detect_cycle_for_derivation, create_derivation_link
  - Integrated artifact registration into submit_bounty approval flow
  - 10 new unit tests for attribution functionality

#### Credit Redemption System (US-015A) — Compute Provider Tracking
- US-015A: Created compute_providers table for tracking credit redemption providers:
  - Migration 20260131000015 with provider_type enum (openai, anthropic, gpu_provider)
  - compute_providers table with name, api_endpoint, conversion_rate NUMERIC(20,8), is_active boolean
  - Indexes on provider_type and is_active for efficient queries
  - Unique constraint on provider name to prevent duplicates
  - Automatic updated_at trigger for tracking configuration changes
  - Default providers seeded: OpenAI and Anthropic with placeholder conversion rates
- US-015A: Implemented ComputeProvider model with:
  - ProviderType enum with serde/sqlx serialization
  - ComputeProvider struct with is_available(), has_endpoint(), credits_for_units() helpers
  - NewComputeProvider with factory methods: openai(), anthropic(), gpu()
  - 8 unit tests for serialization and helper methods
  - 287 total tests pass

#### Credit Redemption Endpoint (US-015B) — Spending M-Credits
- US-015B: Created POST /api/v1/credits/redeem endpoint for credit redemption:
  - Created redemption_receipts table migration (20260131000016) with user_did, provider_id, amount_credits, allocation_id, metadata (JSONB)
  - Indexes on user_did, provider_id, and created_at for efficient queries
  - RedemptionReceipt model with NewRedemptionReceipt factory
  - RedeemCreditsRequest/Response types with camelCase JSON serialization
  - validate_redemption_amount() with min (1.0) and max (10,000.0) credit bounds
  - load_active_provider() to verify provider exists and is active
  - get_account_balance() to check current DID balance
  - deduct_balance() for atomic balance deduction with CHECK constraint validation
  - insert_burn_event() to record credit burn in ledger
  - allocate_with_provider() placeholder for provider API integration
  - 302 total tests pass

#### Balance Check Endpoint (US-015D) — Account Balance API
- US-015D: Created GET /api/v1/credits/balance endpoint for checking account balances:
  - BalanceRequest takes user_id (extracted from auth in production)
  - BalanceResponse returns did, balance, promo_balance, total
  - TransactionRecord type for recent transactions with event_type, amount, description
  - get_user_bound_did() requires DID binding before balance queries
  - get_account_balances() returns main and promo balances (or 0, 0 if no account)
  - get_recent_transactions() fetches last 10 transactions for DID
  - Uses query params (GET) via axum::extract::Query
  - 311 total tests pass (10 new balance tests)

#### Reputation System (US-016A) — Quality-Weighted Reputation
- US-016A: Implemented reputation calculation system:
  - Migration 20260131000017 creates m_reputation table with did, total_rep, decay_factor, last_updated
  - Migration 20260131000018 creates reputation_events ledger table for event sourcing
  - reputation_event_type enum (bounty_completion, review_contribution, manual_adjustment, decay)
  - MReputation model with effective_reputation() applying time decay
  - ReputationEvent model with closure_type_weight and reviewer_weight fields
  - Closure type weights: tests=1.5x, quorum=1.2x, requester=1.0x
  - mint_reputation() function: ensures reputation record exists, applies decay, records event, updates total
  - apply_time_decay() function: 0.99 multiplier per month since last_updated
  - get_reputation() and get_effective_reputation() for querying current/decayed reputation
  - Updated bounties mint_reputation_for_submission to use new reputation system
  - 20+ new unit tests for models and routes
  - 340 server tests pass, 414 total tests pass

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
