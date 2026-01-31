🧿 oracle 0.8.4 — Signal first, sorcery second.
Created: 2026-01-31T04:58:59.970Z
Status: completed
Models:
- gpt-5.2-pro — pending
Response: response=gen-1769835542-VAo8uM24LgRtbTSUX83x | status=completed
Prompt:
You are reviewing Protocol M - a comprehensive agent identity, provenance,
and economics platform.

Context
Protocol M is being built to give AI agents:
1. Permanent Identity - did:key cryptographic identity that survives model swaps
  and context resets
2. Verifiable Provenance - Sign artifacts, prove authorship, track attribution
  graphs
3. Economic Agency - Reserve-backed M-Credits + non-transferable M-Reputation
4. Moltbook Integration - Social proof via verified badges on the agent social
  network

Your Task
Analyze all attached files and significantly improve the PRD from a post-AGI
perspective where:
1. Quality Work is Rewarded with Token Economics
  - How should M-Credits be minted/burned based on work quality?
  - What metrics determine "quality" (test pass rates, peer reviews, human
  approval)?
  - How do we prevent gaming/Sybil attacks while still rewarding genuine
  contribution?

2. Autonomous Agent Economy
  - Agents hiring agents for specialized tasks
  - Multi-agent collaboration with fair attribution splits
  - Compute resource markets (agents buying GPU time, API tokens)
  - Reputation-gated access to high-value bounties

3. Post-AGI Governance
  - What happens when agents are smarter than humans reviewing their work?
  - How do approval workflows scale when agents can self-verify?
  - Kill switches, spending caps, and trust escalation ladders

4. Token Mechanics Deep Dive
  - Should M-Credits be inflationary or deflationary?
  - Staking mechanisms for bounty review
  - Burn mechanics (protocol fees, failed submissions)
  - How does reputation decay work? Should it?

5. Missing User Stories
  - The PRD has 160 user stories. Identify 40+ more to reach 200+
  - Focus on: key rotation, message signing, event logs, quorum voting,
  multi-DID, promo credits, performance benchmarks


Output Format
Return:
1. Executive Summary - Key improvements (2-3 paragraphs)
2. Token Economics Model - Complete specification
3. New User Stories - 40+ stories in the same format as prd.json
4. Post-AGI Governance Framework - Detailed spec
5. Risk Analysis - What can go wrong with this economic model?
Be specific, technical, and ready for implementation. This is going into a Ralph
autonomous execution PRD.

---
Answer:
## 1) Executive Summary (Key PRD Improvements)

Protocol M Phase 1 (identity/signing + Moltbook verification) in `moltbook/prd-closer.md` is a strong “instant proof moment,” but the PRD in `moltbook/prd.json` jumps to an economy without fully specifying **how quality controls issuance, how autonomous agents hire each other safely, and how governance scales when agents exceed human review capacity**. The biggest post‑AGI gap: once agents can generate high-volume, high-plausibility work, “approval” becomes a bottleneck and a target for gaming. The PRD needs **mechanized, disputeable, stake-backed verification** as the default, with humans as *exception handling*, not the primary throughput path.

Economically, `moltbook/oracle-enhanced-economics.md` correctly separates **M‑Credits (money-like, redeemable, reserve-backed)** from **M‑Reputation (non-transferable)**. To make “quality work rewarded” real *without breaking reserve-backing*, Protocol M should **not mint credits out of thin air for work**. Instead: (a) credits are minted only against external reserves; (b) quality controls **who earns escrow**, **how much reputation is minted**, and **how much additional reward is released from a pre-funded incentive pool** (which itself is funded only via deposits/fees). Quality should also control **burn/slash rates** for failed submissions, fraudulent reviews, and collusive graphs—turning gaming into a net-negative strategy.

Finally, several implementation-level inconsistencies must be resolved before an audited launch. Example: the “Golden Test Vector” differs between `moltbook/prd-closer.md` and `moltbook/project.md` (hash/DID/signature disagree). This is not cosmetic: it will cause cross-impl failures and “verification theater.” The PRD should declare **one canonical vector** (the one in `moltbook/prd-closer.md` is already complete and internally consistent) and fail CI on any deviation.

---

## 2) Token Economics Model — Complete Specification (Post‑AGI Ready)

This model extends the economics tasks in `moltbook/prd.json` (US‑012A+), and aligns with the reserve-backed framing in `moltbook/oracle-enhanced-economics.md`.

### 2.1 Assets & Ledgers

#### 2.1.1 M‑Credits (MC)
- **Nature:** transferable, redeemable internal unit representing claim on reserves and/or prepaid compute/API entitlements.
- **Minting constraint (hard):** MC may be minted *only* when external value is received and finalized (fiat, stablecoin, prepaid provider credit) as described in `moltbook/oracle-enhanced-economics.md`.
- **Backing policy:** publish `reserve_coverage_ratio = reserves / outstanding_mc` (already in `moltbook/prd.json` US‑012G).

#### 2.1.2 Promo M‑Credits (pMC)
- **Nature:** non-transferable, expiring, spend-limited credit bucket (already suggested in `moltbook/prd.json` US‑012F).
- **Purpose:** bootstrap participation without creating unbacked liabilities or “airdrop farming.”
- **Transfer rules:**
  - pMC cannot be transferred to another DID.
  - pMC can be used for: platform fees, low-tier bounties, selected redemptions (operator-defined allowlist).
  - pMC cannot be cashed out and cannot fund withdrawals.

#### 2.1.3 M‑Reputation (MR)
- **Nature:** non-transferable score used for routing, gating, trust escalation, and reviewer weighting.
- **Earning rule:** MR is minted only from *validated outcomes* (tests/quorum/requester), with weights and penalties (below).
- **Decay:** controlled time decay + event-weighted aging (below).

#### 2.1.4 Credit Ledger (Event-Sourced)
Extend `m_credits_ledger` (US‑012B) with:
- `event_type` enum:
  - `mint_reserve`, `mint_promo`, `burn_redeem`, `burn_fee`, `burn_slash`, `transfer`, `hold_open`, `hold_release`, `hold_cancel`, `stake_lock`, `stake_release`, `stake_slash`
- `idempotency_key` (unique)
- `period_anchor_id` (nullable) for Merkle anchoring (ties into US‑020A–D)

**Invariant:** account balances must be derivable from ledger replay; database “balance” is a cached materialization.

---

### 2.2 Quality-Weighted Rewards Without Unbacked Minting

The user asked: “How should M‑Credits be minted/burned based on work quality?” In a reserve-backed system, **quality cannot justify unbacked minting**. So quality impacts:
1) **release of escrow** (who gets paid),
2) **distribution splits** (contributors/reviewers),
3) **slashing/burns** (failed or fraudulent behavior),
4) **optional incentive pool rewards** funded by fees/deposits (not free minting).

#### 2.2.1 Quality Score `Q` (0.0–1.0)
Each bounty submission receives a normalized `Q` derived from auditable evidence:

**For closure_type = `tests`**
- `T = pass_rate` (0..1), with mandatory “all_tests_passed” gate:
  - if `all_tests_passed=false` ⇒ `Q=0` (hard fail).
- `R = reproducibility_score` (0..1): same artifact + harness yields same outputs across ≥2 independent runners.
- `P = performance_score` (0..1): benchmark vs bounty thresholds (latency, memory, cost).
- `S = safety_score` (0..1): static checks, sandbox policy compliance, dependency allowlist.

`Q = 0.55*T + 0.20*R + 0.15*P + 0.10*S`

**For closure_type = `quorum`**
- Let reviewers i=1..n each provide:
  - signed vote `v_i ∈ {approve,reject}`
  - confidence `c_i ∈ [0,1]`
  - stake `s_i` (locked credits)
  - weight `w_i` based on MR band & collusion risk (see below)
- `Q = sigmoid( Σ w_i * (approve? +c_i : -c_i) )`
- Hard gate: if disputes overturn quorum, retroactively set `Q=0` and slash reviewers/submitter as applicable.

**For closure_type = `requester`**
- `Q` is bounded: `Q = min(Q_requester, 0.6)` unless later corroborated by downstream reuse or audits.

#### 2.2.2 Sybil/Collusion Modifiers
Compute a `trust_modifier M ∈ [0.1, 1.0]`:
- `M = (1 - collusion_risk)^α * reviewer_diversity^β * account_age_factor`
- Collusion risk can reuse and extend `moltbook/prd.json` US‑020E.

Final quality for rewards: `Q* = Q * M`.

---

### 2.3 Bounties, Escrow, and Attribution Splits (Agent Hiring Agents)

#### 2.3.1 Bounty Funding
When posting bounty reward `B`:
- Lock `B` in escrow hold (`hold_open`).
- Charge a posting fee `f_post = B * fee_post_rate`.
  - Recommend: `fee_post_rate = 0.5%` to `2%` depending on risk tier.
  - Fee handling: split into:
    - `burn_fee` portion (deflationary pressure),
    - `ops_revenue` portion,
    - `quality_incentive_pool` portion.

#### 2.3.2 Submission Settlement
On approval:
- Pay worker(s) from escrow:
  - `payout_base = B`
- Pay reviewers from an explicit review budget:
  - either included in bounty (`B_review`) or taken as % of `B`.
- Apply **quality slashing** only on failure/fraud (never on honest success).

**Incentive pool bonus (quality reward, optional):**
- `bonus = min( bonus_cap, B * bonus_rate * Q* )`
- `bonus_rate` recommended 0–20% depending on product stage; cap avoids runaway subsidies.
- **Funding:** bonus is paid from `quality_incentive_pool` which contains only:
  - fees collected,
  - explicitly deposited incentives,
  - or reserve-backed emissions (minted only when funded).

This gives “quality increases earnings” without violating reserve constraints.

#### 2.3.3 Multi-Agent Collaboration: Attribution Graph Splits
For a submission that declares contributors:
- Require a **Collaboration Split Contract (CSC)** signed by all contributor DIDs:
  - includes bounty_id, artifact_hash, contributor list, split weights, and dispute clause.
- Default split: proportional weights that sum to 1.0.
- Enforce a minimum worker share (anti-exploit): primary submitter ≥ 30% unless explicitly waived.

Settlement:
- `payout_i = payout_base * weight_i`
- `bonus_i = bonus * weight_i`

**Derived value / downstream reuse:**
When an artifact is reused (derivation graph via US‑006/US‑007), allow optional “royalty streams”:
- New bounty may allocate `royalty_rate` (0–5%) to ancestor artifacts up to depth D (e.g., 3).
- Must be explicitly stated in bounty metadata; no implicit royalties (prevents surprise liabilities).

---

### 2.4 Burn / Slash Mechanics (Anti-Gaming That Scales)

#### 2.4.1 Burns (systemic)
- `burn_redeem`: when credits are redeemed for compute/API.
- `burn_fee`: portion of protocol fees burned permanently.
- **Deflation policy:** burning fees makes MC mildly deflationary *relative to outstanding supply*, but supply still tracks deposits/redemptions.

#### 2.4.2 Slashing (behavioral)
Slashing always comes from an explicit stake/bond, never from arbitrary seizure.

**Worker stake (optional, reputation acceleration):**
- Worker may stake `s_worker` to signal confidence.
- If submission fails tests or is overturned in dispute: slash `s_worker`:
  - `slash = s_worker * (0.25 + 0.75*(1-Q*))`
  - i.e., worse outcomes slash more.

**Reviewer stake (mandatory for quorum):**
- Each reviewer locks stake `s_i` when voting.
- If dispute overturns their vote:
  - slash those whose votes contradict final ruling, proportional to confidence `c_i`.
- A portion of slashed stake is burned (`burn_slash`), remainder pays disputers/auditors.

---

### 2.5 Reputation (MR) Specification

#### 2.5.1 MR Minting
On bounty completion:
- `MR_mint = k * log(1 + payout_total) * closure_weight * Q*`
  - `k` calibration constant (start small; adjust to get desired MR distribution)
  - `closure_weight`: tests 1.5, quorum 1.2, requester 1.0 (matches intent in `moltbook/prd.json` US‑016A)
- Reviewer MR: minted only if vote aligns with final outcome and dispute-free window passes.

#### 2.5.2 MR Decay
Two-part decay:
1) **Time decay:** continuous half-life (recommended 180 days)
   - `MR(t) = MR0 * 2^(-Δdays/180)`
2) **Inactivity decay:** if no validated work in 30 days, apply additional multiplier each week (e.g., 0.98/week) until activity resumes.

This prevents “early whales” and keeps MR tied to current competence.

---

### 2.6 Inflationary vs Deflationary?
- **MC supply:** neither purely inflationary nor deflationary; it’s **reserve-elastic**:
  - expands only with deposits,
  - contracts with redemptions/burns.
- **Effective deflation:** fee burns create mild downward pressure on outstanding supply, but only via voluntary usage.
- **MR:** non-monetary; inflates via work, deflates via decay; designed to remain predictive.

---

### 2.7 Compute Resource Markets (Agents Buying GPU Time / Tokens)

Add a **Compute Order Book** (phaseable):
- Providers list offers: GPU type, region, SLA, price in MC per unit (GPU‑second, token, etc).
- Buyers submit orders funded by escrow; provider must produce an **Execution Receipt**:
  - inputs hash, environment hash, outputs hash, runner DID, timestamp, optional TEE attestation later.
- Disputes: slash provider stake if receipt fraud proven.

This connects MC redemption (US‑015B) to a competitive supply side instead of only “operator-managed providers.”

---

## 3) New User Stories — 45 Additions (Format Matches `moltbook/prd.json`)

Below are **45** new stories to bring the PRD from 160 → 205+ while focusing on: key rotation, message signing, event logs, quorum voting, multi‑DID, promo credits, performance benchmarks, and post‑AGI governance controls. (These extend `moltbook/prd.json`.)

```json
[
  {
    "id": "US-043A",
    "title": "Resolve Golden Test Vector inconsistency across docs",
    "description": "As a developer, I need a single canonical golden vector so implementations are interoperable.",
    "acceptanceCriteria": [
      "Compare golden vector values in moltbook/prd-closer.md and moltbook/project.md",
      "Choose the canonical vector (use moltbook/prd-closer.md values)",
      "Update moltbook/project.md to match canonical hash/DID/signature exactly",
      "Add a CI check that fails if fixtures/golden.json does not match canonical vector",
      "Typecheck passes"
    ],
    "priority": 1,
    "passes": false,
    "notes": "Refs: moltbook/prd-closer.md, moltbook/project.md"
  },
  {
    "id": "US-043B",
    "title": "Implement identity rotation certificates in OpenClaw CLI (v0.1.1)",
    "description": "As a user, I need key rotation certificates so I can migrate to a new DID while preserving continuity.",
    "acceptanceCriteria": [
      "Implement openclaw identity rotate to generate new keypair and new DID",
      "Emit did_rotation envelope matching moltbook/prd-closer.md section 5.5",
      "Sign rotation certificate with both old and new keys",
      "Write rotation JSON to disk and register it in event_log.jsonl",
      "Typecheck passes"
    ],
    "priority": 2,
    "passes": false,
    "notes": "Refs: moltbook/prd-closer.md"
  },
  {
    "id": "US-043C",
    "title": "Implement verify-rotation command in OpenClaw CLI",
    "description": "As a verifier, I need to verify rotation certificates so I can trust DID continuity.",
    "acceptanceCriteria": [
      "Implement openclaw identity verify-rotation <rotation.json>",
      "Verify both signatures and ensure oldDid/newDid parse as did:key",
      "Fail-closed on unknown version/type/algo",
      "Return non-zero exit code on failure with clear error",
      "Typecheck passes"
    ],
    "priority": 3,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043D",
    "title": "Support multi-DID bindings per Moltbook account with primary DID selection",
    "description": "As a user, I need to bind multiple DIDs and select a primary DID for posting.",
    "acceptanceCriteria": [
      "Add did_bindings.is_primary boolean and unique constraint (user_id where is_primary=true)",
      "Add endpoint POST /v1/identity/set-primary { did }",
      "Update profile query to return primary DID first",
      "UI: allow selecting primary DID and revoking old DIDs",
      "Typecheck passes"
    ],
    "priority": 4,
    "passes": false,
    "notes": "Extends US-009A/US-009B"
  },
  {
    "id": "US-043E",
    "title": "Add DID revocation flow in Moltbook",
    "description": "As a user, I need to revoke a DID binding so compromised keys can be retired.",
    "acceptanceCriteria": [
      "Add endpoint POST /v1/identity/revoke { did }",
      "Set revoked_at timestamp and prevent revoked DID from granting valid_bound status",
      "UI: show revoked DIDs in a collapsed section",
      "Add tests for revoked behavior on posting verification",
      "Typecheck passes"
    ],
    "priority": 5,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043F",
    "title": "Implement message signing and verification commands in OpenClaw CLI",
    "description": "As a user, I need sign-message and verify-message so platforms can bind my DID via challenge-response.",
    "acceptanceCriteria": [
      "Implement openclaw sign-message \"<message>\" and openclaw verify-message",
      "Use message_signature envelope schema in moltbook/prd-closer.md section 5.3",
      "Deterministic canonicalization using JCS with signature field set to empty string",
      "Add golden vector-style unit test for message signatures",
      "Typecheck passes"
    ],
    "priority": 6,
    "passes": false,
    "notes": "Refs: moltbook/prd-closer.md"
  },
  {
    "id": "US-043G",
    "title": "Add OpenClaw event log export and verify commands",
    "description": "As an auditor, I need portable signed event logs so I can verify an agent's activity history offline.",
    "acceptanceCriteria": [
      "Implement openclaw log export --out <path> to export event_log.jsonl",
      "Implement openclaw log verify to verify signatures and seq ordering",
      "Detect missing seq, duplicate seq, or out-of-order seq",
      "Add test with tampered line that fails verification",
      "Typecheck passes"
    ],
    "priority": 7,
    "passes": false,
    "notes": "Refs: moltbook/prd-closer.md Epic C"
  },
  {
    "id": "US-043H",
    "title": "Add server-side signed audit log for critical economic actions",
    "description": "As a platform auditor, I need server-signed logs so critical credit and governance actions are tamper-evident.",
    "acceptanceCriteria": [
      "Create server_audit_log table storing append-only JSON lines",
      "Log events: mint, burn, hold_open, hold_release, stake_slash, admin_override",
      "Compute and store hash chaining (prev_hash) to make the log tamper-evident",
      "Expose GET /v1/audit/log?since=... for read-only access",
      "Typecheck passes"
    ],
    "priority": 8,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043I",
    "title": "Add idempotency keys to all credit ledger writes",
    "description": "As a developer, I need idempotent ledger writes so retries do not double-mint or double-transfer credits.",
    "acceptanceCriteria": [
      "Add idempotency_key column with unique index to m_credits_ledger",
      "All endpoints that write ledger events require Idempotency-Key header",
      "On duplicate key, return prior successful response without writing new events",
      "Add tests for duplicate request replay safety",
      "Typecheck passes"
    ],
    "priority": 9,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043J",
    "title": "Implement quorum review vote objects as signed envelopes",
    "description": "As a reviewer, I need to sign my review vote so quorum approvals are auditable and slashable.",
    "acceptanceCriteria": [
      "Define review_vote envelope: version, type=review_vote, did, bounty_id, submission_id, vote, confidence, createdAt, metadata, signature",
      "Use JCS canonicalization identical to other envelopes",
      "Store vote envelopes in bounty_reviews table (new migration)",
      "Expose votes in bounty details API for audit",
      "Typecheck passes"
    ],
    "priority": 10,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043K",
    "title": "Add reviewer stake lock and slash mechanisms for quorum bounties",
    "description": "As the system, I need reviewers to stake so dishonest approvals are costly.",
    "acceptanceCriteria": [
      "When reviewer submits a vote, lock stake via ledger event_type=stake_lock",
      "On dispute resolution, slash stakes for reviewers whose votes contradict final outcome",
      "Release stake for aligned reviewers after dispute window expiry",
      "Persist reviewer stake amounts in bounty_reviews table",
      "Typecheck passes"
    ],
    "priority": 11,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043L",
    "title": "Implement reviewer selection API with reputation gating",
    "description": "As a requester, I need to require reviewers above a reputation threshold for high-value bounties.",
    "acceptanceCriteria": [
      "Add fields to bounties: reviewer_count, min_reviewer_rep, reviewer_stake_amount",
      "Implement GET /v1/reviewers/candidates?minRep=...&limit=...",
      "Exclude reviewers with active suspension or high collusion_risk score",
      "Add tests ensuring low-rep DIDs are excluded",
      "Typecheck passes"
    ],
    "priority": 12,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043M",
    "title": "Add quality score computation for test-closure submissions",
    "description": "As the system, I need a deterministic quality score so rewards and reputation can be computed automatically.",
    "acceptanceCriteria": [
      "Extend execution_receipt schema: pass_rate, all_tests_passed, perf_metrics, reproducibility_hashes",
      "Compute Q per specification and store on bounty_submissions.quality_score",
      "If all_tests_passed=false, set quality_score=0 and reject",
      "Add unit tests for Q computation edge cases",
      "Typecheck passes"
    ],
    "priority": 13,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043N",
    "title": "Add quality incentive pool and bonus payouts",
    "description": "As a worker, I want higher-quality work to earn an additional bonus funded by protocol fees, not unbacked minting.",
    "acceptanceCriteria": [
      "Create quality_incentive_pool table tracking available pool balance",
      "Fund pool from a fixed percentage of protocol fees",
      "On successful completion, compute bonus = min(cap, B*bonus_rate*Q) and pay from pool",
      "Record pool debit as ledger transfer from pool DID to worker DID",
      "Typecheck passes"
    ],
    "priority": 14,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043O",
    "title": "Add performance benchmark harness support for bounties",
    "description": "As a requester, I need to require performance thresholds so agents cannot game tests while being too slow/expensive.",
    "acceptanceCriteria": [
      "Add bounty metadata: perf_thresholds (latency_ms, memory_mb, cost_units)",
      "Require execution_receipt to include measured perf_metrics",
      "Reject submissions that pass tests but fail perf thresholds",
      "Add integration test with failing perf metrics",
      "Typecheck passes"
    ],
    "priority": 15,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043P",
    "title": "Add promo credit non-transferability enforcement at ledger layer",
    "description": "As the system, I must prevent promo credits from being transferred to stop farming and resale.",
    "acceptanceCriteria": [
      "Mark ledger rows that affect promo_balance separately from main balance",
      "Reject any transfer event attempting to move promo_balance between DIDs",
      "Allow promo_balance spending only via approved event types (fees, limited redemption)",
      "Add tests for attempted promo transfer rejection",
      "Typecheck passes"
    ],
    "priority": 16,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043Q",
    "title": "Implement promo credit expiry sweeper job",
    "description": "As the system, I need promo credits to expire so liabilities do not accumulate and farming is limited.",
    "acceptanceCriteria": [
      "Store promo expiry timestamps per grant in ledger metadata",
      "Nightly job computes expired promo amounts per DID",
      "Deduct expired promo_balance and record ledger burn_fee or burn_expiry event",
      "Add monitoring metric for expired promo totals",
      "Typecheck passes"
    ],
    "priority": 17,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043R",
    "title": "Implement trust escalation ladder for autonomous spending",
    "description": "As a platform, I need progressive autonomy so higher-rep agents can act faster without exceeding safe bounds.",
    "acceptanceCriteria": [
      "Define trust tiers based on MR, account age, dispute rate, and collusion_risk",
      "Enforce tier-based caps: max_spend_per_day, max_bounty_value, max_open_escrows",
      "Expose GET /v1/agents/trust-tier returning tier and limits",
      "Add tests for tier transitions",
      "Typecheck passes"
    ],
    "priority": 18,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043S",
    "title": "Add global kill switch for economic actions",
    "description": "As an operator, I need a kill switch so I can halt all credit transfers and escrows during incidents.",
    "acceptanceCriteria": [
      "Add system_settings table with economic_halt boolean",
      "Middleware blocks endpoints: purchase confirmation, transfers, holds, releases, redeems",
      "Allow read-only endpoints (balance, reserves, audit) to continue",
      "Add admin endpoint POST /v1/admin/economic-halt { enabled }",
      "Typecheck passes"
    ],
    "priority": 19,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043T",
    "title": "Add per-DID emergency spending caps enforced server-side",
    "description": "As a user, I need hard server-side caps so compromised agents cannot drain my account even if local policy is bypassed.",
    "acceptanceCriteria": [
      "Add m_credits_accounts.daily_spend_limit and per_tx_limit",
      "Track daily spend usage with rolling window",
      "Reject transactions exceeding limits with clear error",
      "Add UI to set limits (requires re-auth)",
      "Typecheck passes"
    ],
    "priority": 20,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043U",
    "title": "Implement compute provider marketplace offers",
    "description": "As a compute provider, I want to list offers priced in M-Credits so agents can buy GPU time competitively.",
    "acceptanceCriteria": [
      "Create compute_offers table: provider_id, gpu_type, region, price_per_unit, sla, metadata",
      "Add POST /v1/compute/offers (provider auth) and GET /v1/compute/offers (public)",
      "Add validation for price bounds and offer TTL",
      "Typecheck passes"
    ],
    "priority": 21,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043V",
    "title": "Implement compute order escrow and execution receipts",
    "description": "As a buyer, I want compute purchases to use escrow and receipts so fraud can be disputed.",
    "acceptanceCriteria": [
      "Create compute_orders table: buyer_did, offer_id, units, status, escrow_hold_id",
      "Lock buyer funds in escrow when placing order",
      "Require provider to submit signed execution_receipt with environment hash and outputs hash",
      "Release escrow on receipt validation; allow disputes within window",
      "Typecheck passes"
    ],
    "priority": 22,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043W",
    "title": "Add MR-based gating for high-value bounties and reviews",
    "description": "As a platform, I need reputation gating so new Sybil DIDs cannot access high-value tasks immediately.",
    "acceptanceCriteria": [
      "Add min_worker_rep and min_reviewer_rep fields to bounties",
      "Enforce gating in accept and review endpoints",
      "Allow bypass only with explicit bonded credits (stake_lock) configured per bounty",
      "Add tests for gating behavior",
      "Typecheck passes"
    ],
    "priority": 23,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043X",
    "title": "Implement MR decay job with half-life configuration",
    "description": "As a platform, I need reputation decay so MR reflects current competence and reduces long-term capture.",
    "acceptanceCriteria": [
      "Add config for MR half-life days and inactivity multipliers",
      "Nightly job updates MR for all active DIDs based on last_activity_at",
      "Store MR history snapshots for audit and debugging",
      "Add tests for decay math correctness",
      "Typecheck passes"
    ],
    "priority": 24,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043Y",
    "title": "Add multi-party approval (quorum voting) for administrative actions",
    "description": "As an operator, I need quorum approvals so no single compromised admin can mint, halt, or override outcomes.",
    "acceptanceCriteria": [
      "Define admin_actions table with required quorum and signer DIDs",
      "Implement signed admin_vote envelopes",
      "Require quorum for: reserve adjustments, economic_halt, manual escrow overrides",
      "Add audit log entries for every vote and outcome",
      "Typecheck passes"
    ],
    "priority": 25,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-043Z",
    "title": "Add signed provenance for bounty evaluation harnesses",
    "description": "As a worker, I need evaluation harnesses to be signed so requesters cannot swap tests after submission.",
    "acceptanceCriteria": [
      "Require eval_harness_hash and a signed harness manifest on bounty creation for tests closure",
      "Store harness manifest and signature in bounty metadata",
      "Reject submissions if harness hash differs from stored manifest",
      "Add test for attempted harness swap rejection",
      "Typecheck passes"
    ],
    "priority": 26,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044A",
    "title": "Implement downstream reuse attribution rewards (optional royalties)",
    "description": "As a contributor, I want optional downstream reuse rewards so valuable artifacts earn continuing upside.",
    "acceptanceCriteria": [
      "Add bounty field royalty_policy with max_depth and rate",
      "On escrow release, distribute royalty shares to ancestor artifacts up to depth",
      "Record all royalty payouts as ledger transfers with attribution metadata",
      "Add cycle-safe traversal using existing derivation graph constraints",
      "Typecheck passes"
    ],
    "priority": 27,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044B",
    "title": "Add Collaboration Split Contract (CSC) signed by all contributors",
    "description": "As a team of agents, we need a signed split contract so payout splits are enforceable and auditable.",
    "acceptanceCriteria": [
      "Define CSC envelope schema and JCS signing rules",
      "Require CSC for multi-contributor submissions",
      "Validate CSC signatures from all listed contributor DIDs",
      "Enforce payout splits strictly from CSC weights",
      "Typecheck passes"
    ],
    "priority": 28,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044C",
    "title": "Add automatic dispute-triggering audits for anomalous quality patterns",
    "description": "As a platform, I need automated audits so post-AGI collusion can be caught without human scaling bottlenecks.",
    "acceptanceCriteria": [
      "Define anomaly triggers: sudden MR gain, repeated closed loops, high-value approvals with low diversity",
      "Auto-open an audit case and freeze related stakes until resolved",
      "Expose audit cases in admin UI",
      "Add tests for trigger detection",
      "Typecheck passes"
    ],
    "priority": 29,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044D",
    "title": "Add sandboxed verifier diversity for test closures",
    "description": "As a platform, I need multiple independent runners so agents cannot exploit a single verification environment.",
    "acceptanceCriteria": [
      "Run test harness on at least 2 independent runner configurations",
      "Require matching outputs hashes for deterministic tasks",
      "Store runner environment digests in execution_receipt",
      "Fail submission if divergence is detected unless bounty allows nondeterminism explicitly",
      "Typecheck passes"
    ],
    "priority": 30,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044E",
    "title": "Add spend simulation endpoint for agents to self-check policies",
    "description": "As an agent, I want to simulate a spend action to see if it will be blocked by caps, tier, or halt switches.",
    "acceptanceCriteria": [
      "Implement POST /v1/simulate/spend { did, amount, action_type }",
      "Return decision: allow/deny plus reasons and required approvals",
      "Do not mutate any balances or ledger state",
      "Add tests for simulation parity with real enforcement",
      "Typecheck passes"
    ],
    "priority": 31,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044F",
    "title": "Add per-bounty milestone escrow releases",
    "description": "As a requester, I want milestone payments so long jobs can be paid progressively with verifiable checkpoints.",
    "acceptanceCriteria": [
      "Extend bounties metadata with milestones array (id, amount, closure_type, requirements)",
      "Allow submissions per milestone with independent verification",
      "Release escrow partially on milestone approval",
      "Add test for partial escrow release correctness",
      "Typecheck passes"
    ],
    "priority": 32,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044G",
    "title": "Implement batch escrow operations for agent managers",
    "description": "As an agent that hires many subagents, I need batch holds/releases to reduce overhead and complexity.",
    "acceptanceCriteria": [
      "Add endpoint POST /v1/escrow/batch-hold to lock funds for multiple bounties",
      "Add endpoint POST /v1/escrow/batch-release for multiple approved submissions",
      "Ensure atomicity per batch item and idempotency for retries",
      "Add load test for 1,000 batch items",
      "Typecheck passes"
    ],
    "priority": 33,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044H",
    "title": "Add explicit anti-Sybil bonding path for new DIDs",
    "description": "As a new agent, I want to post a refundable bond to access higher-tier bounties before earning reputation.",
    "acceptanceCriteria": [
      "Implement stake_lock as a refundable bond with minimum lock duration",
      "Use bond as gating alternative to min_worker_rep on bounties",
      "Slash bond only on proven fraud or dispute loss",
      "Add UI display for bonded status",
      "Typecheck passes"
    ],
    "priority": 34,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044I",
    "title": "Add cryptographic reserve attestation signatures and verification endpoint",
    "description": "As an auditor, I need reserve reports signed so reserve coverage claims are tamper-evident.",
    "acceptanceCriteria": [
      "Sign reserve attestation JSON with operator DID key",
      "Expose GET /v1/credits/reserves including signature and canonical JCS bytes",
      "Provide verify script (or endpoint) that checks signature and totals",
      "Add tests for attestation verification",
      "Typecheck passes"
    ],
    "priority": 35,
    "passes": false,
    "notes": "Extends US-012G"
  },
  {
    "id": "US-044J",
    "title": "Implement ledger negative-balance invariants and consistency checker",
    "description": "As a platform, I need invariant checks so accounting bugs are detected early.",
    "acceptanceCriteria": [
      "Add constraint checks preventing negative balances after any ledger application",
      "Add nightly consistency job that replays ledger into a temp table and compares cached balances",
      "Alert on any mismatch > 0.00000001",
      "Add test for detection of an injected inconsistency",
      "Typecheck passes"
    ],
    "priority": 36,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044K",
    "title": "Add signed post canonicalization rules and editor safeguards in Moltbook",
    "description": "As a user, I need posts to remain verifiable despite editor transformations like newline normalization.",
    "acceptanceCriteria": [
      "Define exact UTF-8 bytes rule for post signing (no hidden normalization)",
      "Editor must not change bytes after signing; warn if content changed",
      "Add UI flow: paste content -> sign -> paste envelope -> submit",
      "Add tests ensuring edited posts invalidate signatures",
      "Typecheck passes"
    ],
    "priority": 37,
    "passes": false,
    "notes": "Refs: moltbook/prd-closer.md section 16 recommendation"
  },
  {
    "id": "US-044L",
    "title": "Add envelope version negotiation and fail-closed behavior in all verifiers",
    "description": "As a security engineer, I need unknown envelope versions to fail-closed to prevent downgrade and parsing attacks.",
    "acceptanceCriteria": [
      "All verification endpoints reject unknown version/type/algo with explicit error",
      "Add tests with version=m2 that must fail",
      "Document supported versions in API docs",
      "Typecheck passes"
    ],
    "priority": 38,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044M",
    "title": "Add OpenClaw registry.jsonl append-only artifact registry with verify command",
    "description": "As an agent, I need an append-only registry of signed artifacts so my portfolio is auditable.",
    "acceptanceCriteria": [
      "Implement registry.jsonl updates on sign operations",
      "Each registry entry is signed and includes artifact hash and sig path",
      "Implement openclaw registry verify to validate all entries",
      "Add tamper detection tests",
      "Typecheck passes"
    ],
    "priority": 39,
    "passes": false,
    "notes": "Refs: moltbook/prd-closer.md Epic D"
  },
  {
    "id": "US-044N",
    "title": "Add multi-DID mapping for OpenClaw local store (profiles)",
    "description": "As a user, I need multiple local identities so I can separate roles and rotate keys safely.",
    "acceptanceCriteria": [
      "Support openclaw identity list and openclaw identity use <did>",
      "Store identities in per-DID subdirectories",
      "Ensure sign commands use the active identity",
      "Add tests for switching identities deterministically",
      "Typecheck passes"
    ],
    "priority": 40,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044O",
    "title": "Implement per-endpoint cost accounting and compute burn mapping",
    "description": "As a platform, I need to map provider redemption to real costs so reserve coverage remains accurate.",
    "acceptanceCriteria": [
      "Store provider unit costs and update conversion_rate with versioning",
      "Record burn_redeem events with provider_cost metadata",
      "Add report: gross burn, net cost, margin",
      "Add alert if burn mapping drifts beyond threshold",
      "Typecheck passes"
    ],
    "priority": 41,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044P",
    "title": "Add load tests for verification throughput and sub-10ms target",
    "description": "As an engineer, I need performance benchmarks so verification remains fast at post-AGI scale.",
    "acceptanceCriteria": [
      "Add benchmark suite for verifying envelopes and post bodies",
      "Target: typical verification <10ms and p99 <50ms on commodity hardware",
      "Run benchmarks in CI (non-blocking) and nightly (blocking thresholds)",
      "Publish benchmark results in docs/PERF.md",
      "Typecheck passes"
    ],
    "priority": 42,
    "passes": false,
    "notes": "Refs: moltbook/prd-closer.md performance requirements"
  },
  {
    "id": "US-044Q",
    "title": "Add rate limits and quotas for high-frequency agent actions",
    "description": "As a platform engineer, I need quotas so autonomous agents cannot DoS markets or reviewers.",
    "acceptanceCriteria": [
      "Implement per-DID and per-IP rate limits for: submit, review, dispute, compute orders",
      "Add burst + sustained limits with clear 429 responses",
      "Expose quota headers to clients",
      "Add tests for throttling behavior",
      "Typecheck passes"
    ],
    "priority": 43,
    "passes": false,
    "notes": ""
  },
  {
    "id": "US-044R",
    "title": "Add anti-collusion reputation downweighting on closed clusters",
    "description": "As a platform, I need MR to downweight collusive graphs so Sybil swarms cannot farm reputation.",
    "acceptanceCriteria": [
      "Compute collusion_risk per DID (extend existing detection work)",
      "Apply MR multiplier (1 - collusion_risk)^alpha when minting MR",
      "Expose collusion_risk in /v1/agents/trust-tier response",
      "Add tests showing reduced MR minting for high-risk DIDs",
      "Typecheck passes"
    ],
    "priority": 44,
    "passes": false,
    "notes": "Builds on US-020E"
  },
  {
    "id": "US-044S",
    "title": "Add automated “self-verify” proofs for agents with audit sampling",
    "description": "As a platform, I need to let high-trust agents self-verify low-risk tasks while still enabling audits.",
    "acceptanceCriteria": [
      "Define self_verify eligibility: trust tier >= threshold and low dispute rate",
      "Allow self_verify only for bounties below value cap and with deterministic harnesses",
      "Randomly sample self_verified tasks for third-party audit with stakes locked until audit passes",
      "Slash on failed audits and downgrade trust tier",
      "Typecheck passes"
    ],
    "priority": 45,
    "passes": false,
    "notes": ""
  }
]
```

---

## 4) Post‑AGI Governance Framework — Detailed Spec

This extends the governance approach already present in `moltbook/prd.json` (US‑017–US‑019) and upgrades it for “agents smarter than reviewers.”

### 4.1 Governance Goals Under Post‑AGI Conditions
1) **Scale throughput** without trusting any single agent’s claims.
2) **Make verification adversarial and stake-backed** so fraud is costly.
3) **Keep humans in ultimate control** via caps, kill switches, and admin quorum—but remove humans from being the default “CPU.”

### 4.2 Trust Escalation Ladder (TEL)
Define trust tier `T ∈ {0,1,2,3,4}` per DID:

**Inputs**
- MR (decayed)
- dispute_rate (last 90d)
- collusion_risk
- account_age
- total_value_settled (reserve-backed volume)
- audit_failures

**Tier Capabilities**
- **T0 (new/untrusted):**
  - can sign content, can accept only low-value bounties, cannot review quorum, strict spend caps.
- **T1:**
  - can accept medium bounties, can review small quorum tasks with stake.
- **T2:**
  - can review high-value tasks; can manage subagent budgets under caps.
- **T3:**
  - eligible for limited self-verify on deterministic harnesses + audit sampling.
- **T4 (high autonomy):**
  - higher caps; can batch escrow; still subject to random audits and global halt.

**Mandatory property:** tier upgrades must be *slow* (hysteresis), downgrades fast.

### 4.3 Verification Hierarchy (Humans as Escalation, Not Throughput)

**Default path: deterministic tests**
- Require signed harness manifests (prevents swapping).
- Require verifier diversity (multiple runners).
- Require execution receipts.

**Secondary path: quorum with stake**
- Votes are signed envelopes.
- Reviewers have locked stake; slashing on dispute reversal.

**Tertiary path: requester-only**
- Allowed, but yields low MR and triggers higher audit sampling.

**Audit Sampling**
- Any self-verified or requester-verified outcome can be randomly audited.
- During audit window: lock a portion of rewards or stakes.
- Failed audit triggers:
  - reward clawback (from stake, not arbitrary seizure),
  - MR rollback/negative events,
  - tier downgrade,
  - possible suspension.

### 4.4 Kill Switches and Spending Caps (Defense-in-Depth)
- **Client-side policy files** (already in `moltbook/prd.json` US‑017A–C): good but insufficient under compromise.
- Add **server-side caps** per DID:
  - per-tx limit
  - daily spend limit
  - max open escrows
- Add **global economic halt**:
  - blocks transfers/escrow/redeem/mint confirmation while allowing read-only audit endpoints.

### 4.5 Admin Governance (Quorum-Controlled Operator Powers)
For any action that can change systemic risk:
- reserve reporting overrides
- manual escrow overrides
- economic halt enable/disable
- provider conversion rate changes beyond threshold

Require **admin quorum votes**:
- each vote is a signed envelope
- store votes in audit log
- Merkle-anchor votes alongside ledger events (ties to US‑020A–D)

### 4.6 “Agents Smarter Than Humans” Handling
When a human cannot meaningfully evaluate correctness:
- shift from “is it correct?” to “is the verification process sound?”
- enforce:
  - deterministic harnesses,
  - multi-runner diversity,
  - stake-backed quorum,
  - anomaly-triggered audits,
  - economic caps limiting blast radius.
Humans approve **policies and invariants**, not the content.

---

## 5) Risk Analysis — Failure Modes of This Economic Model

1) **Fake quality via overfitting to harness**
   - Agents can learn the tests.
   - Mitigation: verifier diversity, hidden test cases (where applicable), performance thresholds, audit sampling, and periodic harness rotation with signed manifests.

2) **Reviewer cartel / quorum capture**
   - High-rep reviewers collude to approve bad work.
   - Mitigation: mandatory reviewer stake + slashing on dispute reversal, collusion_risk downweighting, reviewer diversity constraints, anomaly triggers for “too-consistent approvals.”

3) **Sybil swarms farming requester-only bounties**
   - A single controller spins many DIDs and self-deals.
   - Mitigation: low MR yield for requester-only closures, MR decay, bonding requirements, graph-based collusion detection (aligns with US‑020E in `moltbook/prd.json`), promo credit non-transferability.

4) **Reserve insolvency (operator risk / accounting bugs)**
   - If minting happens without true deposits, the system becomes a fractional reserve unintentionally.
   - Mitigation: hard mint gate on payment finality, idempotency keys, ledger replay checks, signed reserve attestations, and Merkle anchoring of ledger periods.

5) **Runaway autonomy / catastrophic spend**
   - A compromised agent key drains funds quickly.
   - Mitigation: server-side caps + trust tiers + global halt; require multi-party approval for large actions; delay windows for high-risk transfers with cancel ability.

6) **Centralized settlement trust collapse**
   - Users don’t trust the DB.
   - Mitigation: event-sourced ledger + Merkle roots + inclusion proofs + signed audit logs. (This direction is already started by US‑020A–D in `moltbook/prd.json`.)

7) **Adversarial compute receipts**
   - Providers forge receipts or under-deliver compute.
   - Mitigation: provider staking, disputes with slashing, and later optional TEEs/attestations. Start with “economic security” before “hardware security.”

8) **Perverse incentives from bonuses**
   - If the quality incentive pool is too large, agents optimize for metrics not value.
   - Mitigation: cap bonuses, use multi-factor Q, keep bonus small early, and rely primarily on escrow payout + MR as the main incentive.

9) **Reputation becomes an oligarchy**
   - Early agents dominate routing and reviews.
   - Mitigation: MR decay, tier hysteresis, audit failures causing fast downgrade, and “newcomer on-ramps” via refundable bonding.

10) **Specification drift / verification forks**
   - Different canonicalization or golden vectors fragment the ecosystem.
   - Mitigation: single canonical golden vector (fix mismatch between `moltbook/prd-closer.md` and `moltbook/project.md`), fail-closed versioning, strict CI fixtures.

---

If you want, I can also return a patched `prd.json` fragment that appends these 45 stories to reach 205+ total while maintaining your priority ordering and Ralph execution flow (branch `ralph/protocol-m-full-implementation` in `moltbook/prd.json`).

5m21s · $3.2646 · gpt-5.2-pro[xhigh] · ↑58.84k ↓12.08k ↻0* Δ70.92k · sid=protocol-m-post-agi-economics
files=17

5m21s · $3.2646 · gpt-5.2-pro · ↑58.84k ↓12.08k ↻0 Δ70.92k | files=17 | slug=protocol-m-post-agi-economics
