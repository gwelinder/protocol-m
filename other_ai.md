Take all of these ideas and make the final, master plan with ALL requirements included as user stories for the full ambitious breakdown Yes — I agree with the PRD’s direction and most of its constraints. The one thing I **don’t** agree with as-written is that you currently have **multiple conflicting “golden vectors”** floating around (different DIDs + different SHA-256 + a clearly placeholder-looking signature). That’s fatal for interop.

Below is a single **Master PRD** that merges the best parts, adds the “fullest” user-story coverage, and locks down **one authoritative golden vector** (computed deterministically from the standard Ed25519 seed + JCS canonical bytes).

---

# Master PRD — Protocol M Phase 1: OpenClaw Identity, Signing, Verification, and Instant Proof

| Metadata      | Value                                                                       |
| ------------- | --------------------------------------------------------------------------- |
| Document      | PRD-001                                                                     |
| Project       | Protocol M (“Mycelium”)                                                     |
| Release       | v0.1.0-alpha                                                                |
| Scope         | **OpenClaw CLI + Moltbook verification (end-to-end proof)**                 |
| Non-goals     | No blockchain, no tokens, no delegation market                              |
| Primary Users | Agent developers + Moltbook end users (two-sided, but dev-first ergonomics) |
| Tech          | Rust (ed25519-dalek, serde_jcs, clap), Postgres (Moltbook)                  |
| Date          | 2026-01-31                                                                  |

## 0. Product Thesis

Phase 1 ships **Structural Permanence** as a standard: agents can generate an identity key once, then sign and verify work forever — across model swaps, runtime resets, and platform migrations.

**Phase 1 MUST deliver an “instant proof moment” for agents:**

* “I can sign something right now.”
* “Anyone can verify it right now.”
* “A platform can badge it right now.”

---

## 1. Hard Technical Constraints (Non-negotiable)

1. **No blockchain dependency (MVP)**

* No nodes, no contracts, no on-chain transactions.
* Offline sign/verify must work with only local files + pure crypto.

2. **Deterministic signatures across implementations**

* Canonicalization: **RFC 8785 JCS** for JSON envelopes.
* Encoding: **UTF-8** (no BOM).
* Fail-closed on unknown envelope versions/types.

3. **Cross-platform: macOS, Linux, Windows**

* CI MUST test all three.
* Storage paths MUST be OS-correct (not hardcoded `~` assumptions).
* Windows permission model handled explicitly (see §6.3).

4. **Sub-second verification**

* Verification must be cheap enough for real-time API endpoints.
* Target: **< 10ms** typical (post-sized bodies), hard cap size checks at API boundaries.

5. **Private key protection is mandatory by default**

* Encrypted at rest; no plaintext key written to disk by default.
* Clear threat model: protects against casual disk exfiltration; not against hostile host control.

6. **Rotation path is designed-in (Phase 1.1)**

* Phase 1.0 ships without rotation, but schemas + platform model must not block rotation.

---

## 2. Goals and Non-goals

### 2.1 Goals (v0.1.0-alpha)

* Generate a persistent `did:key` (Ed25519).
* Sign **artifacts** (files) deterministically.
* Sign **messages** (strings) deterministically (needed for platform binding).
* Verify signatures offline (no network calls).
* Maintain a local signed **event log** (append-only, agent-auditable).
* Export a portable **manifest** (portfolio / proof bundle).
* Moltbook can bind DID to account (challenge-response) and badge signed posts.

### 2.2 Non-goals (explicitly excluded)

* Tokens / $SPORE, payments, escrow, delegation market.
* IPFS/S3 fetching or “artifact hosting.”
* Sybil resistance or “realness” of an agent identity.
* “Secure enclave” guarantees (optional integrations later; not required claims now).

---

## 3. Personas

### P1 — Agent Developer (primary)

Builds agents and tools. Needs:

* A single binary + stable APIs.
* Golden vectors + CI guarantees.
* Clear failure modes (hash mismatch vs signature mismatch).

### P2 — Moltbook End User (secondary)

Consumes content. Needs:

* Simple UI signals (Verified vs Signed-but-unbound).
* Ability to inspect proof without trusting Moltbook.

### P3 — Platform Engineer (Moltbook integrator)

Needs:

* Cheap verification primitives.
* Clear DB schema + endpoints.
* Defense against abuse (size caps, rate limits).

### P4 — Auditor / Investigator

Needs:

* Deterministic verification.
* Portable proof bundles.
* Tamper-evident logs.

---

## 4. User Stories (Full Coverage)

### Epic A — Identity lifecycle (agent-side)

**A1. Create identity**

* As an agent developer, I run `openclaw identity init` to generate a DID and encrypted root key.
* **Acceptance:** Creates identity directory, encrypted key, public key, identity metadata; prints DID.

**A2. Show identity**

* As a user, I run `openclaw identity show` to display DID + createdAt + storage path.
* **Acceptance:** Works offline, no prompts.

**A3. Backup identity**

* As a user, I run `openclaw identity export` to produce an encrypted backup bundle.
* **Acceptance:** Bundle contains encrypted key + identity.json; does not leak plaintext key.

**A4. Restore identity**

* As a user, I run `openclaw identity import` to restore an identity bundle on a new machine.
* **Acceptance:** Restored DID matches; signing works after passphrase entry.

**A5. Rotation (Phase 1.1 readiness)**

* As a user, I will later run `openclaw identity rotate` to migrate to a new DID while preserving history linkage.
* **Acceptance (Phase 1.1):** Emits rotation certificate signed by old and new keys.

---

### Epic B — Signing (instant proof moment)

**B1. Sign a file**

* As an agent, I run `openclaw sign <file>` to produce a `.sig.json`.
* **Acceptance:** Deterministic envelope bytes and signature; includes sha256 hash and artifact metadata.

**B2. Verify a file signature**

* As any verifier, I run `openclaw verify <file> <sig>` to check integrity and authorship.
* **Acceptance:** Clear output; exit codes distinguish hash mismatch vs signature mismatch.

**B3. Sign a message (for platform binding)**

* As a user, I run `openclaw sign-message "<challenge>"` to produce a message signature envelope.
* **Acceptance:** No file needed; deterministic canonicalization; prints JSON to stdout.

**B4. Verify a message signature**

* As a platform, I verify a signed challenge using only DID → pubkey derivation.
* **Acceptance:** Offline verification; returns boolean + reason.

---

### Epic C — Event log (persistent audit trail)

**C1. Append signed event**

* As an agent, I run `openclaw log "did X"` to append a signed entry to `event_log.jsonl`.
* **Acceptance:** Append-only; each entry is independently verifiable.

**C2. Verify event log**

* As an auditor, I run `openclaw log verify` to verify every entry signature and ordering.
* **Acceptance:** Detects tampering, missing lines, invalid signatures; reports first failure.

---

### Epic D — Manifest / portfolio

**D1. Register artifacts**

* As an agent, signing automatically updates a local registry of signed artifacts.
* **Acceptance:** Registry is deterministic and append-only (jsonl recommended).

**D2. Export manifest**

* As a user, I run `openclaw manifest export --sign` to produce a portable manifest (portfolio) plus optional proof bundle.
* **Acceptance:** Manifest lists artifacts + hashes + signatures; itself can be signed.

**D3. Verify manifest**

* As a verifier, I run `openclaw manifest verify <manifest>` to check manifest signature and internal consistency.
* **Acceptance:** No network calls; verifies internal signatures; reports mismatches.

---

### Epic E — Moltbook end-to-end value

**E1. Bind DID to Moltbook account**

* As a user, I click “Bind OpenClaw Identity,” receive a challenge, sign it locally, and submit.
* **Acceptance:** Moltbook stores DID binding only after signature verifies and challenge is unused/unexpired.

**E2. Post signed content**

* As an agent/user, I submit a post with optional `signatureEnvelope`.
* **Acceptance:** Moltbook verifies signature and shows badge state correctly.

**E3. Badge states**

* As a viewer, I see:

  * **✓ Verified** when DID is valid and bound to account
  * **🔑 Signed** when signature is valid but DID is not bound
  * No badge if invalid/absent
* **Acceptance:** Clicking badge reveals raw envelope and verification result.

**E4. Edits and invalidation**

* As a platform, if a post is edited, verification status updates deterministically.
* **Acceptance:** Edited content invalidates prior signature unless re-signed; UI explains why.

---

## 5. Protocol: Formats (Normative)

### 5.1 Identity: `did:key` (Ed25519)

* Public key: 32 bytes Ed25519.
* `did:key` encoding:

  1. Prefix public key with multicodec **0xed 0x01**
  2. Base58BTC encode
  3. Multibase prefix `z`
  4. Prepend `did:key:`

### 5.2 Artifact Signature Envelope (`m1`)

```json
{
  "version": "m1",
  "type": "artifact_signature",
  "algo": "ed25519",
  "did": "did:key:z...",
  "hash": { "algo": "sha256", "value": "<hex>" },
  "artifact": { "name": "<filename>", "size": 123 },
  "createdAt": "2026-01-30T00:00:00Z",
  "metadata": {},
  "signature": "<base64(sig)>"
}
```

### 5.3 Message Signature Envelope (`m1`)

Used for Moltbook binding challenges and other attestations.

```json
{
  "version": "m1",
  "type": "message_signature",
  "algo": "ed25519",
  "did": "did:key:z...",
  "createdAt": "2026-01-30T00:00:00Z",
  "message": "moltbook:bind:....",
  "metadata": {},
  "signature": "<base64(sig)>"
}
```

### 5.4 Event Log Entry (`m1`) — JSONL

```json
{
  "version": "m1",
  "type": "event_log_entry",
  "algo": "ed25519",
  "did": "did:key:z...",
  "createdAt": "2026-01-30T00:00:00Z",
  "seq": 42,
  "event": "cleaned dataset X",
  "metadata": {},
  "signature": "<base64(sig)>"
}
```

### 5.5 Canonicalization and signing (Normative)

For all envelope types:

1. Set `signature` to `""` (empty string)
2. Canonicalize JSON using **RFC 8785 JCS**
3. Sign canonical UTF-8 bytes with Ed25519
4. Base64 encode signature bytes; set `signature` field

Verification:

1. Parse JSON
2. Save `signature` value, set to `""`
3. Canonicalize with JCS
4. Verify signature using pubkey derived from DID

### 5.6 Rotation Certificate (Reserved for Phase 1.1)

```json
{
  "version": "m1",
  "type": "did_rotation",
  "oldDid": "did:key:zOld...",
  "newDid": "did:key:zNew...",
  "createdAt": "2026-01-30T00:00:00Z",
  "reason": "operator_rotation",
  "signatureOld": "<base64(sig_by_old_over_canonical_payload)>",
  "signatureNew": "<base64(sig_by_new_over_canonical_payload)>"
}
```

---

## 6. OpenClaw CLI Spec (v0.1.0-alpha)

### 6.1 Command surface

```bash
# Identity
openclaw identity init [--force] [--no-encrypt] [--identity-path <path>]
openclaw identity show
openclaw identity export --out <bundle.ocid>
openclaw identity import <bundle.ocid>

# Signing
openclaw sign <file> [--meta key=value]... [--out <file.sig.json>] [--dry-run]
openclaw verify <file> <sig.json>

# Message signing (for Moltbook binding)
openclaw sign-message "<message>" [--meta key=value]... [--createdAt <rfc3339>] [--dry-run]
openclaw verify-message <message> <sig.json>

# Event log
openclaw log "<event>" [--meta key=value]...
openclaw log verify

# Manifest
openclaw manifest export [--out portfolio.json] [--sign]
openclaw manifest verify <portfolio.json>
```

### 6.2 Exit codes (stable contract)

| Code | Meaning                                              |
| ---: | ---------------------------------------------------- |
|    0 | Success                                              |
|    2 | CLI usage / invalid args                             |
|    3 | Identity not found / not initialized                 |
|    4 | Verification failed: hash mismatch                   |
|    5 | Verification failed: signature mismatch              |
|    6 | Permission / storage security violation              |
|    7 | Decryption failed (wrong passphrase / corrupted key) |

### 6.3 Storage layout (cross-platform)

Use OS-correct data directory (Rust `directories` crate):

* macOS: `~/Library/Application Support/openclaw/identity/`
* Linux: `${XDG_DATA_HOME:-~/.local/share}/openclaw/identity/`
* Windows: `%APPDATA%\openclaw\identity\`

Files:

```
identity/
  root.key.enc        # age-encrypted seed (default)
  root.pub            # raw pubkey or hex (implementation-defined, but stable)
  identity.json       # did, algo, createdAt, formatVersion
  registry.jsonl      # append-only signed artifact registry entries (optional but recommended)
  event_log.jsonl     # signed event log entries
```

Permissions:

* macOS/Linux: enforce 0700 dir, 0600 keyfile (fail hard).
* Windows: best-effort ACL checks; encryption is mandatory; warn if ACL cannot be validated.

---

## 7. Moltbook Integration (v0.1.0-alpha end-to-end)

### 7.1 DB schema (Postgres)

```sql
CREATE TABLE did_challenges (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id),
  challenge TEXT NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  used_at TIMESTAMPTZ
);

CREATE TABLE did_bindings (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id),
  did TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  revoked_at TIMESTAMPTZ,
  UNIQUE(user_id, did)
);

ALTER TABLE posts
  ADD COLUMN signature_envelope JSONB,
  ADD COLUMN verified_did TEXT,
  ADD COLUMN verification_status TEXT NOT NULL DEFAULT 'none';
-- verification_status: none | invalid | valid_unbound | valid_bound
```

### 7.2 API endpoints

**Create challenge**

* `POST /v1/identity/challenge` (auth required)
* Returns:

```json
{ "challenge": "moltbook:bind:<nonce>:<rfc3339>", "expiresAt": "..." }
```

**Bind DID**

* `POST /v1/identity/bind` (auth required)

```json
{
  "did": "did:key:z...",
  "challenge": "moltbook:bind:...",
  "challengeEnvelope": { "... message_signature envelope ..." }
}
```

Server checks:

* challenge exists, not expired, not used
* envelope is valid `message_signature`
* envelope.message exactly equals challenge
* DID matches envelope.did
* signature verifies via DID pubkey
* store binding; mark challenge used

**Create post (with optional signature)**

* `POST /v1/posts`

```json
{ "body": "hello world\n", "signatureEnvelope": { "... artifact_signature ..." } }
```

Server checks (if signatureEnvelope present):

* enforce max envelope size (32KB)
* enforce max body size for inline verification (1MB)
* verify `artifact_signature` against exact UTF-8 bytes of `body`
* set:

  * `valid_bound` if DID is bound to posting user
  * `valid_unbound` if signature valid but not bound
  * `invalid` otherwise

### 7.3 UI requirements

Badge mapping:

* `valid_bound` → **✓ Verified**
* `valid_unbound` → **🔑 Signed**
* `invalid|none` → no badge

Badge click opens:

* DID (shortened)
* signature status
* hash
* raw envelope JSON (copy button)

### 7.4 Abuse controls

* Rate-limit challenge creation and bind attempts per user/IP.
* Reject envelopes with unknown versions/types.
* Cap body length and envelope length.
* Store verification result; do not re-verify on every read unless post changes.

---

## 8. Golden Test Vector (Authoritative, MUST match)

This is the single source of truth for Phase 1 conformance.

```json
{
  "comment": "Protocol M Phase 1 - Golden Vector (Ed25519 + JCS)",
  "seed_hex": "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
  "public_key_hex": "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
  "did": "did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw",
  "file_content_utf8": "hello world\n",
  "file_size": 12,
  "file_hash_sha256_hex": "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447",
  "createdAt": "2026-01-30T00:00:00Z",
  "canonical_envelope_jcs": "{\"algo\":\"ed25519\",\"artifact\":{\"name\":\"hello.txt\",\"size\":12},\"createdAt\":\"2026-01-30T00:00:00Z\",\"did\":\"did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw\",\"hash\":{\"algo\":\"sha256\",\"value\":\"a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447\"},\"metadata\":{},\"signature\":\"\",\"type\":\"artifact_signature\",\"version\":\"m1\"}",
  "signature_base64": "c7rSjOQf44/8l6+TqMSB1NYprlhsEwLoY+0IhJVzA/PP+QQkHN+qXXndMthL3CeMTZQVqYuPdEy9O1kjCCk5Aw=="
}
```

**Acceptance:** `openclaw sign` (in deterministic test mode with provided seed + fixed createdAt) MUST output this signature exactly.

---

## 9. Performance Requirements

* Verify post-sized envelope+body in **< 10ms** typical on commodity server CPU.
* Verification must not allocate unbounded memory:

  * Envelope size cap: **32KB**
  * Inline body cap: **1MB**
* CLI operations should feel instant for typical artifacts:

  * `verify` under 50ms for files up to 10MB (hash dominates; still acceptable).

---

## 10. Testing & Quality Bar

### 10.1 Crypto library tests

* Golden vector exact match (signature + canonical bytes)
* Roundtrip (sign then verify)
* Tamper file → hash mismatch (exit 4)
* Tamper signature → signature mismatch (exit 5)
* Wrong DID/pubkey → signature mismatch
* Canonicalization invariance tests (key order differences in input JSON)

### 10.2 CLI tests

* Identity init creates correct files and permissions
* Encryption default ON; wrong passphrase fails cleanly (exit 7)
* Dry-run prints envelope without writing

### 10.3 Moltbook E2E

* Bind flow works: challenge → sign-message → bind
* Signed post shows ✓ when bound
* Signed post shows 🔑 when unbound
* Edited post invalidates verification

### 10.4 CI requirements

* `cargo test` on ubuntu/macos/windows
* Minimal supported Rust version pinned
* Fuzzing optional (nice-to-have): envelope parser and verifier

---

## 11. Security Model (Honest, explicit)

### What Phase 1 proves

* **Key control**: signer possessed the private key at signing time.
* **Integrity**: content hash matches.
* **Deterministic provenance**: verification is offline and reproducible.

### What Phase 1 does NOT prove

* The signer is “a real agent” (Sybil resistance is out of scope).
* The key was not stolen (host compromise defeats all local key systems).
* The content is correct/benign (authorship ≠ quality).

### Key risks and mitigations

* Key theft at rest → encryption + permission checks
* Replay of challenge → nonce + expiry + single-use enforcement
* Post body normalization bugs → verify exact UTF-8 bytes; avoid server-side “helpful” transforms

---

## 12. Rollout Plan

1. Land RFC + PRD + golden vector fixtures.
2. Ship `openclaw` binary with identity/sign/verify/sign-message/log/manifest.
3. Ship Moltbook:

   * challenge + bind endpoints
   * post verification + badges
4. Public “instant proof” demo:

   * sign a post → badge shows ✓ in one flow.

---

## 13. Open Questions (tracked, not blocking unless marked)

* **(Blocking)** Exact storage location policy for enterprise environments (override flags ok).
* Windows ACL enforcement strategy (warn vs fail).
* Should `metadata` remain free-form JSON or adopt a minimal schema for common fields?
* Post canonicalization contract: do we sign raw body bytes, or a “rendered normalized” variant? (Strong recommendation: raw bytes.)

---

If you want, I can also reshape this into **two repo files** (commit-ready):

* `docs/specs/001-master-prd-phase1.md` (this PRD)
* `fixtures/golden_vector.json` (authoritative vector)
# OpenClaw Identity & Artifact Signing
## Protocol M — Phase 1 Master PRD

| Field | Value |
|-------|-------|
| **Version** | 1.2 (Final) |
| **Date** | 2026-01-31 |
| **Status** | Ready for Engineering |
| **Release** | v0.1.0-alpha |
| **Patch** | v0.1.1 (Key Rotation) |

---

## 0. One-Sentence Definition

Phase 1 ships a **portable cryptographic identity** (`did:key` / Ed25519) and **deterministic artifact signing** (RFC 8785 JCS) so any platform can verify authorship **without blockchain**.

---

## 1. Goals

### Must Ship (v0.1.0-alpha)
- **Identity:** Locally generated Ed25519 root key → `did:key` identifier
- **Signing:** Deterministic `.sig.json` envelopes for any file bytes
- **Message Signing:** Challenge-response support for platform binding
- **Verification:** Sub-second, fail-closed verification
- **Portability:** Manifests verifiable anywhere with no network dependency
- **Cross-Platform:** macOS + Linux + Windows with CI matrix

### Planned (v0.1.1)
- **Key Rotation:** Rotation certificates linking old DID → new DID

### Non-Goals (Explicitly Excluded)
- Tokens / credits / $SPORE
- Blockchains / smart contracts
- Delegation markets
- Decentralized storage requirements (IPFS/Arweave)

---

## 2. Hard Technical Constraints

These are **non-negotiable**. Violating any makes the system non-adoptable.

### C1: No Blockchain Dependency
- No nodes, contracts, or chain libraries
- Signing + verification works offline with pure crypto

### C2: Deterministic Canonicalization
- RFC 8785 (JCS) is mandatory
- UTF-8, no BOM
- Timestamps: RFC 3339 UTC
- Unknown schema versions fail verification (fail-closed)

### C3: Cross-Platform Runtime
- CLI compiles and runs on macOS, Linux, Windows
- CI tests on all three OSes

### C4: Sub-Second Verification
- Typical verification < 10ms on commodity hardware
- Envelope size cap: 32 KB
- Body size for inline verification: 1 MB (hash-only for larger)

### C5: Secure Key Storage by Default
- Private key encrypted at rest (`age`) with passphrase
- No plaintext private key ever written to disk
- Unix: enforce `0700` dir + `0600` file (fail if too open)
- Windows: encryption required + best-effort ACL warning

### C6: Key Rotation Path (Specified Now, Ships v0.1.1)
- Format specified in this PRD
- Implementation ships in v0.1.1
- Old artifacts remain valid under old DID

---

## 3. Users

| Persona | Need |
|---------|------|
| **Agent Developers** | CLI + library to sign outputs deterministically |
| **Platform Implementers** | DID binding + verification flows at scale |
| **End Users** | Clear "verified" indicator + inspectable proof |

---

## 4. System Components

### A. `openclaw-crypto` (Library)
Pure functions: DID derivation, hashing, JCS canonicalization, sign/verify. No filesystem I/O. WASM-compatible.

### B. `openclaw` (CLI Binary)
Identity management, artifact signing, message signing, manifest export.

### C. Moltbook Integration (Optional Add-on)
Challenge-response DID binding, verified badge on posts.

---

## 5. Protocol Formats

### 5.1 Signature Envelope Schema (v1)

```json
{
  "version": "m1",
  "type": "artifact_signature",
  "algo": "ed25519",
  "did": "did:key:z6Mk...",
  "hash": {
    "algo": "sha256",
    "value": "<hex>"
  },
  "artifact": {
    "name": "<filename>",
    "size": <bytes>
  },
  "createdAt": "<RFC3339-UTC>",
  "metadata": {},
  "signature": "<base64>"
}
```

**Rule:** `metadata` MUST always be present (minimum `{}`).

### 5.2 Signing Procedure (Normative)

```
1. file_hash = SHA-256(file_bytes)
2. Build envelope with signature = "" (empty string)
3. canonical_bytes = JCS(envelope)
4. signature = Ed25519_Sign(private_key, canonical_bytes)
5. envelope.signature = Base64(signature)
```

### 5.3 Verification Procedure (Normative)

```
1. Parse envelope; store signature value
2. Set envelope.signature = ""
3. canonical_bytes = JCS(envelope)
4. local_hash = SHA-256(file_bytes)
5. Assert: local_hash == envelope.hash.value
6. public_key = DID_to_PublicKey(envelope.did)
7. Ed25519_Verify(public_key, canonical_bytes, signature)
```

### 5.4 Rotation Certificate Schema (v0.1.1)

```json
{
  "version": "m1",
  "type": "did_rotation",
  "oldDid": "did:key:z6Mk...",
  "newDid": "did:key:z6Mk...",
  "createdAt": "<RFC3339-UTC>",
  "reason": "operator_rotation",
  "signatureOld": "<base64>",
  "signatureNew": "<base64>"
}
```

Platforms MAY treat rotation certs as "same identity lineage" for reputation.

---

## 6. Golden Test Vector (Authoritative)

**CI MUST enforce exact match. Any deviation fails the build.**

```json
{
  "comment": "RFC-0001 Golden Test Vector — Cryptographically Verified",
  "seed_hex": "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
  "public_key_hex": "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
  "did": "did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw",
  "file_content": "hello world\n",
  "file_size": 12,
  "file_hash_sha256": "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447",
  "created_at": "2026-01-30T00:00:00Z",
  "artifact_name": "hello.txt",
  "canonical_envelope_jcs": "{\"algo\":\"ed25519\",\"artifact\":{\"name\":\"hello.txt\",\"size\":12},\"createdAt\":\"2026-01-30T00:00:00Z\",\"did\":\"did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw\",\"hash\":{\"algo\":\"sha256\",\"value\":\"a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447\"},\"metadata\":{},\"signature\":\"\",\"type\":\"artifact_signature\",\"version\":\"m1\"}",
  "signature_base64": "c7rSjOQf44/8l6+TqMSB1NYprlhsEwLoY+0IhJVzA/PP+QQkHN+qXXndMthL3CeMTZQVqYuPdEy9O1kjCCk5Aw=="
}
```

---

## 7. CLI Specification

### 7.1 Commands (v0.1.0-alpha)

```bash
# Identity
openclaw identity init [--force] [--path <dir>]
openclaw identity show
openclaw identity export-pub [--out <file>]
openclaw identity sign-message "<text>"    # For platform binding

# Signing
openclaw sign <file> [--meta key=value] [--dry-run] [--out <path>]
openclaw verify <file> <sig.json>

# Manifest
openclaw manifest export [--out <path>] [--sign]
```

### 7.2 Commands (v0.1.1)

```bash
openclaw identity rotate [--reason <text>] [--out <path>]
openclaw identity verify-rotation <rotation.json>
```

### 7.3 Exit Codes (Stable Contract)

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Identity not found |
| 4 | Hash mismatch |
| 5 | Signature invalid |
| 6 | Insecure permissions |

---

## 8. Key Storage

### Path
```
~/.openclaw/identity/
├── root.key.enc    # age-encrypted 32-byte seed
├── root.pub        # Public key (hex)
└── identity.json   # { "did": "...", "algo": "ed25519", "createdAt": "..." }
```

### Security
- Passphrase prompt on init and each signing operation
- `zeroize` crate for memory hygiene
- Permission check before any key operation

---

## 9. Moltbook Integration

### 9.1 Binding Flow

```
1. User: POST /v1/identity/challenge
   → Server returns: { "challenge": "moltbook:bind:<nonce>:<ts>", "expiresAt": "..." }

2. User: openclaw identity sign-message "moltbook:bind:<nonce>:<ts>"
   → Returns signature

3. User: POST /v1/identity/bind
   Body: { "did": "did:key:...", "challenge": "...", "signature": "..." }
   → Server verifies signature, stores (user_id, did) binding
```

### 9.2 Verified Badge Rules

A post displays **✓ Verified** if:
1. Signature validates against post body bytes
2. DID is bound to posting user
3. Post has not been edited after signing

**On Edit:** Badge is removed. User must re-sign.

### 9.3 Database Schema

```sql
CREATE TABLE did_bindings (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id),
    did VARCHAR(256) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    revoked_at TIMESTAMP,
    UNIQUE(user_id, did)
);

CREATE TABLE did_challenges (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id),
    challenge VARCHAR(128) NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    used_at TIMESTAMP
);

-- Add to posts table:
ALTER TABLE posts ADD COLUMN signature_envelope JSONB;
ALTER TABLE posts ADD COLUMN verified_did VARCHAR(256);
ALTER TABLE posts ADD COLUMN verification_status 
    VARCHAR(20) CHECK (verification_status IN ('none', 'invalid', 'valid_unbound', 'valid_bound'));
```

---

## 10. Repository Structure

```
openclaw/
├── rfcs/
│   └── 0001-identity-signing.md
├── crates/
│   ├── openclaw-crypto/           # Pure library (WASM-compatible)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs
│   │       ├── hash.rs
│   │       ├── jcs.rs
│   │       ├── did.rs
│   │       ├── sign.rs
│   │       └── verify.rs
│   └── openclaw-cli/              # Binary
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── cmd/
│           │   ├── mod.rs
│           │   ├── identity.rs
│           │   ├── sign.rs
│           │   ├── verify.rs
│           │   └── manifest.rs
│           └── store/
│               ├── mod.rs
│               └── identity_store.rs
├── fixtures/
│   ├── hello_world.txt
│   ├── hello_world.sig.json
│   └── golden.json
├── .github/
│   └── workflows/
│       └── ci.yml
└── Cargo.toml
```

---

## 11. Dependencies (Rust)

```toml
# openclaw-crypto
ed25519-dalek = { version = "2", features = ["rand_core"] }
sha2 = "0.10"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_jcs = "0.1"
bs58 = "0.5"
base64 = "0.22"
hex = "0.4"
anyhow = "1"

# openclaw-cli (additional)
age = "0.10"
clap = { version = "4", features = ["derive"] }
rpassword = "7"
zeroize = { version = "1", features = ["derive"] }
```

---

## 12. Testing Requirements

### Unit Tests
| Test | Assertion |
|------|-----------|
| `test_golden_vector` | Signature matches exactly |
| `test_jcs_determinism` | `{a:1,b:2}` == `{b:2,a:1}` canonical bytes |
| `test_roundtrip` | Sign → verify succeeds |
| `test_tamper_file` | Modified file → hash mismatch error |
| `test_tamper_sig` | Modified signature → invalid error |
| `test_wrong_key` | Different key → invalid error |

### Integration Tests
| Test | Assertion |
|------|-----------|
| `test_cli_init` | Creates files with correct permissions |
| `test_cli_sign_verify` | End-to-end success |
| `test_sign_message` | Challenge signing works |

### CI Matrix
- `ubuntu-latest`
- `macos-latest`  
- `windows-latest`

---

## 13. Release Plan

| Version | Scope | Timeline |
|---------|-------|----------|
| **v0.1.0-alpha** | CLI + crypto + golden vectors + CI | Week 1-2 |
| **v0.1.0-alpha+** | Moltbook binding + verified badges | Week 3 |
| **v0.1.1** | Key rotation certificates | Week 4+ |

---

## 14. Acceptance Criteria

- [ ] `cargo test` passes on macOS, Linux, Windows
- [ ] Golden vector signature matches exactly
- [ ] `openclaw sign` → `openclaw verify` roundtrips
- [ ] `openclaw identity sign-message` works for Moltbook binding
- [ ] Verification < 10ms for typical payloads
- [ ] Key file encrypted at rest, permissions enforced
- [ ] Moltbook displays ✓ badge on valid signed posts (if alpha+ included)

---

## 15. Security Considerations

### What This System Proves
- Holder of private key authorized the signature
- Artifact has not been modified since signing
- Identity-key continuity (same key signed multiple artifacts)

### What This System Does NOT Prove
- Identity is a "real" agent (Sybil resistance requires reputation)
- Signing key hasn't been compromised
- Artifact quality (only authorship)

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Key theft | age encryption, permissions, passphrase |
| Signature forgery | Ed25519 is unforgeable |
| Hash collision | SHA-256 collision-resistant |
| Replay | `createdAt` in envelope |
| Platform impersonation | Challenge-response binding |

---

## 16. Open Questions

| Question | Owner | Decide By |
|----------|-------|-----------|
| Should `metadata` have a schema? | Eng | Before v0.1.0 |
| Windows ACL enforcement level? | Eng | Before CI |
| Rate limits on Moltbook endpoints? | Platform | Before alpha+ |

---

## 17. Appendix: Complete Signed Envelope Example

```json
{
  "version": "m1",
  "type": "artifact_signature",
  "algo": "ed25519",
  "did": "did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw",
  "hash": {
    "algo": "sha256",
    "value": "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447"
  },
  "artifact": {
    "name": "hello.txt",
    "size": 12
  },
  "createdAt": "2026-01-30T00:00:00Z",
  "metadata": {},
  "signature": "c7rSjOQf44/8l6+TqMSB1NYprlhsEwLoY+0IhJVzA/PP+QQkHN+qXXndMthL3CeMTZQVqYuPdEy9O1kjCCk5Aw=="
}
```

---

## 18. Approval

| Role | Name | Date |
|------|------|------|
| Product | | |
| Engineering | | |
| Security | | |

---

*Document finalized 2026-01-31. Ready for `docs/specs/protocol-m-phase1.md`*