# Protocol M - Ready for Overnight Ralph Execution 🚀

**Status:** ✅ ALL SYSTEMS GO

**Generated:** 2026-01-31 04:35 UTC

---

## Executive Summary

Protocol M is ready for autonomous overnight implementation via Ralph. We have:

1. ✅ **Oracle-enhanced economics** ($1.35 GPT-5.2 Pro analysis)
2. ✅ **104 granular tasks** in prd.json
3. ✅ **Ralph automation** configured
4. ✅ **Moltbook integration** for user research and community
5. ✅ **Progress tracking** infrastructure

---

## What Oracle Delivered

Oracle GPT 5.2 Pro analyzed Protocol M and provided critical economics improvements:

### Key Insight
**Separate reputation (non-transferable) from money (redeemable)** to prevent circular token shuffling.

### Two Primitives
1. **M-Credits** - Reserve-backed, redeemable compute/API credits (replaces $SPORE)
2. **M-Reputation** - Non-transferable, earned from verified outcomes

### Solved Problems
- ✅ Bootstrap: Reserve-backed minting (fiat/USDC → credits)
- ✅ Tokenomics: Mint only on deposits, burn on redemption
- ✅ Settlement: Hybrid (Postgres + Merkle anchoring + optional L2)
- ✅ Cash-out: 3 rails (compute redemption, team chargeback, regulated withdrawal)
- ✅ Quality incentives: Verification types (tests > quorum > requester)
- ✅ Anti-gaming: Non-transferable rep, stake+slash, collusion detection
- ✅ Scaling: Batch escrow, sharded queues, streaming payments

**Full details:** `/Users/gfw/clawd/moltbook/oracle-enhanced-economics.md`

---

## Task Breakdown (104 Tasks)

### Layer I: OpenClaw CLI (28 tasks, US-001A through US-004C)
**Infrastructure:**
- Cargo workspace setup
- Dependencies (ed25519-dalek, serde_jcs, age, clap)
- Type definitions (SignatureEnvelopeV1, HashRef, ArtifactInfo)

**Core Crypto:**
- SHA-256 hashing
- JCS canonicalization (RFC 8785)
- DID derivation (multicodec 0xed01 + Base58BTC)
- Keypair generation
- Age encryption/decryption
- File permission checks (0700 dir, 0600 keyfile)

**CLI Commands:**
- `openclaw identity init` - Generate identity
- `openclaw sign <file>` - Sign artifacts
- `openclaw verify <file> <sig>` - Verify signatures
- `openclaw manifest export` - Export contribution manifest

**Tests:**
- Golden test vectors
- Roundtrip signing
- Tamper detection

### Layer II: Attribution (9 tasks, US-005A through US-007B)
**Database:**
- artifacts table (hash, DID, timestamp, metadata, signature)
- artifact_derivations table (parent-child relationships)

**API:**
- POST /api/v1/artifacts - Register signed artifacts
- GET /api/v1/artifacts/{id}/attribution - Query derivation graph
- Cycle detection for DAG

### Layer III: Moltbook Social (8 tasks, US-008A through US-011C)
**Database:**
- did_bindings table (user_id ↔ DID mapping)
- did_challenges table (10-minute expiry)
- posts table: signature_envelope_json, verified_did, verification_status

**API:**
- POST /api/v1/identity/challenge - Generate challenge
- POST /api/v1/identity/bind - Verify signature and bind DID
- POST /api/v1/posts - Accept signed posts

**UI:**
- IdentityBadge component (truncated DID display)
- VerifiedBadge component (green ✓ for valid_bound)
- SignatureModal component (show envelope JSON)

**Rate Limiting:**
- 5 challenges per user per hour
- 3 bind attempts per challenge

### Layer IV: Economics (34 tasks, US-012A through US-016F-R)
**Database:**
- m_credits_accounts (balance, promo_balance)
- m_credits_ledger (event sourcing: mint, burn, transfer, hold, release)
- purchase_invoices (track fiat/USDC → credits)
- bounties (title, reward, closure_type, status)
- escrow_holds (lock credits until verified)
- bounty_submissions (signed proofs, execution receipts)
- m_reputation (total_rep, decay_factor, collusion_risk)
- disputes (challenge fraud, stake+slash)

**API:**
- POST /api/v1/credits/purchase - Buy M-Credits (Stripe integration)
- POST /api/v1/credits/redeem - Spend credits (compute/API providers)
- GET /api/v1/credits/balance - Check account
- GET /api/v1/credits/reserves - Transparency (reserve ratio)
- POST /api/v1/bounties - Post tasks with escrow
- POST /api/v1/bounties/{id}/submit - Submit signed work
- POST /api/v1/bounties/{id}/dispute - Challenge submissions
- POST /api/v1/disputes/{id}/resolve - Arbiter resolution

**Marketplace UI:**
- /marketplace page with filters (reward, closure_type, deadline)
- Bounty listings with Accept button
- Sort: newest, highest reward, ending soon

**Quality Mechanisms:**
- Test-based auto-approval (harness_hash verification)
- Quorum review (stake required)
- Requester approval (low-rep yield)
- Reputation weighting: tests=1.5x, quorum=1.2x, requester=1.0x

### Layer V: Governance (11 tasks, US-017A through US-019D)
**Policy System:**
- JSON schema for policies (max_spend, approval_tiers)
- `openclaw policy set` command
- Policy validation

**Approval Flow:**
- approval_requests table
- `openclaw approve <request-id>`
- `openclaw reject <request-id>`
- Webhook notifications

**Emergency Controls:**
- `openclaw emergency-stop` - Cancel all pending actions
- agent_suspensions table
- Suspension check middleware (403 for suspended agents)
- `openclaw resume` - Reactivate after review

### Infrastructure (14 tasks, US-020A through US-025A)
**Auditability:**
- Merkle tree for ledger events
- merkle_anchors table (periodic roots)
- Hourly anchor job
- Inclusion proof generation (GET /api/v1/credits/proof/{event_id})
- Optional L2 anchoring (Base)

**Anti-Gaming:**
- Collusion detection scoring
- Closed loop detection (A → B → A)
- Graph clustering coefficient
- Reputation downweighting by collusion_risk

**DevOps:**
- CI workflow (Ubuntu, macOS, Windows)
- Release workflow (build binaries on tag)
- Monitoring: reserve_coverage_ratio, dispute_rate
- End-to-end test (full bounty flow)

**Documentation:**
- OpenClaw README with quickstart
- API documentation for all endpoints
- Security audit checklist (SQL injection, XSS, CSRF, rate limiting)

---

## Ralph Configuration

**Location:** `/Users/gfw/clawd/moltbook/`

**Files:**
- ✅ `prd.json` - 104 tasks with verifiable acceptance criteria
- ✅ `progress.txt` - Initialized with project context
- ✅ `scripts/ralph/ralph.sh` - Execution loop
- ✅ `scripts/ralph/CLAUDE.md` - Agent instructions

**Branch:** `ralph/protocol-m-full-implementation`

**Execution Command:**
```bash
cd /Users/gfw/clawd/moltbook
./scripts/ralph/ralph.sh --tool claude 50
```

**Parameters:**
- `50` iterations (adjust based on progress)
- `--tool claude` uses Claude Code instead of Amp

---

## Moltbook Integration

**Agent:** protocol-m-ralph
**Profile:** https://moltbook.com/u/protocol-m-ralph
**Status:** Pending Claim

**📋 Human Action Required:**
1. Visit: https://moltbook.com/claim/moltbook_claim_bcHrlFjdkKSODeunBzsdLLA2lin9U6OL
2. Post tweet: "I'm claiming my AI agent \"protocol-m-ralph\" on @moltbook 🦞\n\nVerification: burrow-NVFT"
3. Submit tweet link

**Why Moltbook:**
- User research for DID binding and signature verification
- Community building for Protocol M ecosystem
- Dogfooding (we're building what we'll use)
- Marketplace feedback from real users

**Credentials:** `~/.config/moltbook/credentials.json` (secured 0600)

---

## Expected Deliverables (Overnight Run)

### Phase 1: OpenClaw CLI (Tasks 1-31)
- ✅ Rust workspace with 2 crates
- ✅ Ed25519 signing and verification
- ✅ DID generation (did:key format)
- ✅ Age-encrypted key storage
- ✅ CLI with init, sign, verify, manifest commands
- ✅ Golden vector tests passing
- ✅ CI passing on all platforms

### Phase 2: Attribution (Tasks 32-39)
- ✅ ClawdHub artifact registry
- ✅ Derivation graph with cycle prevention
- ✅ Query API for attribution chains

### Phase 3: Moltbook Social (Tasks 40-55)
- ✅ DID binding flow (challenge/response)
- ✅ Signature verification for posts
- ✅ Verified badge UI
- ✅ Profile DID display

### Phase 4: Economics (Tasks 56-80)
- ✅ M-Credits purchase (Stripe integration stub)
- ✅ Reserve-backed minting
- ✅ Escrow system for bounties
- ✅ Test-based auto-approval
- ✅ Reputation calculation
- ✅ Marketplace UI

### Phase 5: Governance (Tasks 81-91)
- ✅ Policy system with validation
- ✅ Approval workflow
- ✅ Emergency stop mechanism

### Infrastructure (Tasks 92-104)
- ✅ Merkle anchoring for auditability
- ✅ Collusion detection
- ✅ Monitoring dashboards
- ✅ E2E tests
- ✅ Documentation

---

## Success Metrics

**Code Quality:**
- All commits pass typecheck
- All tests green
- CI green on all platforms
- Zero unwrap() in production code

**Economics Validation:**
- Reserve coverage ratio = 1.0 (fully backed)
- Test closure auto-approval works
- Escrow release atomic
- Reputation minted correctly

**Social Integration:**
- DID binding flow complete
- Signature verification functional
- Verified badges display
- Moltbook posts signed and verified

---

## How to Monitor Progress

**While Ralph runs:**
```bash
# Check task completion
cd /Users/gfw/clawd/moltbook
cat prd.json | jq '.userStories[] | select(.passes == false) | {id, title}'

# See recent learnings
tail -50 progress.txt

# Check git commits
git log --oneline -20

# Watch Ralph output
tail -f /path/to/ralph/output.log  # If running in background
```

**Attach to Oracle session:**
```bash
oracle session protocol-m-post-agi-enhancemen
```

---

## Next Steps After Ralph Completes

1. **Review Implementation:**
   - Check prd.json for remaining `passes: false` tasks
   - Review progress.txt for blocked tasks or errors
   - Run manual tests for UI components

2. **Post to Moltbook:**
   - Share implementation results
   - Post signed artifacts demonstrating verification
   - Invite community to test DID binding

3. **Deploy Phase 1:**
   - Publish OpenClaw CLI binaries
   - Deploy ClawdHub artifact registry
   - Enable Moltbook signature verification

4. **Economic Validation:**
   - Test credit purchase flow
   - Post test bounties
   - Verify escrow release mechanics

5. **Community Onboarding:**
   - Write quickstart guide
   - Create video demo
   - Host AMA on Moltbook

---

## Files Reference

**Core Documents:**
- `/Users/gfw/clawd/moltbook/tasks/prd-protocol-m.md` - Original PRD
- `/Users/gfw/clawd/moltbook/oracle-enhanced-economics.md` - Oracle analysis
- `/Users/gfw/clawd/moltbook/prd.json` - Ralph task list
- `/Users/gfw/clawd/moltbook/progress.txt` - Execution log
- `/Users/gfw/clawd/moltbook/RALPH_PREP.md` - Preparation guide
- `/Users/gfw/clawd/moltbook/MOLTBOOK_SETUP.md` - Social integration

**Configuration:**
- `~/.config/moltbook/credentials.json` - API credentials
- `~/.claude/skills/ralph/` - Ralph skill
- `~/.claude/skills/moltbook/` - Moltbook skill

---

## Risk Mitigations

**Ralph Failures:**
- Tasks sized for one context window
- Dependencies ordered correctly (DB → backend → UI)
- Verifiable acceptance criteria (no vague "works well")
- Quality gates enforced (typecheck, tests)

**Economics Risks:**
- FR-E4 enforced: mint only on confirmed deposits
- Reserve attestation public
- Promo credits non-transferable, expiring
- Dispute mechanism with stake+slash

**Technical Debt:**
- AGENTS.md updated with learnings
- progress.txt captures patterns
- CI catches regressions
- Documentation generated alongside code

---

## Emergency Contacts

**If Ralph gets stuck:**
1. Check progress.txt for error patterns
2. Review current task acceptance criteria
3. Manually fix blockers and mark `passes: true`
4. Resume Ralph from next task

**If economics looks wrong:**
1. Check reserve_coverage_ratio (must be ≥ 1.0)
2. Audit ledger events for double-spends
3. Verify Merkle root matches local computation

---

**🚀 READY TO EXECUTE**

All systems prepared. Oracle economics validated. 104 tasks queued. Moltbook registered. Progress tracking configured.

**Human actions:**
1. Claim Moltbook agent (optional but recommended)
2. Start Ralph: `cd /Users/gfw/clawd/moltbook && ./scripts/ralph/ralph.sh --tool claude 50`
3. Monitor overnight
4. Review results in morning

Let's ship Protocol M. 🦞
