# Ralph Preparation Guide - Protocol M

## Current Status

- [x] Ralph cloned and copied to `scripts/ralph/`
- [x] Ralph skill installed in `~/.claude/skills/ralph/`
- [x] `progress.txt` initialized with project context
- [ ] Oracle GPT 5.2 Pro analysis PENDING (session: protocol-m-post-agi-enhancemen)
- [ ] Enhanced PRD converted to `prd.json`
- [ ] Ralph execution started

## Workflow

### 1. Wait for Oracle to Complete

Oracle is currently analyzing Protocol M from a post-AGI perspective with token economics focus.

**Check status:**
```bash
oracle session protocol-m-post-agi-enhancemen
# OR
oracle status
```

### 2. Save Oracle Output

When Oracle completes, save its enhanced PRD recommendations to a new file:

```bash
# Manually save from Oracle output, or:
oracle session protocol-m-post-agi-enhancemen > oracle-enhanced-prd.md
```

### 3. Convert to Ralph JSON Format

Use the Ralph skill to convert the enhanced PRD to `prd.json`:

**Option A: Use the skill directly**
```
/ralph tasks/prd-protocol-m.md
```

**Option B: Merge Oracle enhancements first**
1. Review Oracle's output for economics improvements
2. Merge the best recommendations into `tasks/prd-protocol-m.md`
3. Convert the merged PRD: `/ralph tasks/prd-protocol-m.md`

### 4. Target: 100+ Granular Tasks

The Ralph skill will break down the 19 user stories into 100+ implementable tasks. This requires:

**Breaking Down Large Stories:**
- **US-001 (Initialize identity)** → 10+ tasks:
  - Create Cargo workspace structure
  - Add ed25519-dalek dependency
  - Implement keypair generation
  - Implement age encryption wrapper
  - Add file permission checks
  - Implement DID derivation (multicodec + Base58BTC)
  - Create identity.json writer
  - Add CLI subcommand scaffolding
  - Write integration test for golden vector
  - Add error handling for insecure permissions
  - etc.

- **US-008 (DID binding)** → 8+ tasks:
  - Create did_bindings table migration
  - Create did_challenges table migration
  - Implement challenge generation endpoint
  - Implement challenge expiry logic
  - Implement signature verification for challenges
  - Implement bind endpoint
  - Add rate limiting middleware
  - Add API tests for binding flow
  - etc.

- **US-012 (Bootstrap $SPORE)** → 15+ tasks (if Oracle provides concrete economics):
  - Create spore_accounts table
  - Create spore_transactions table
  - Implement balance tracking
  - Implement transfer logic with escrow
  - Add transaction atomicity (DB triggers or locks)
  - Implement starter credit grants
  - Implement human-to-agent payment conversion
  - Add fraud detection (multi-account, wash trading)
  - etc.

**Each task should be:**
- Completable in one context window (one Ralph iteration)
- Verifiable (has clear acceptance criteria)
- Ordered by dependencies (schema → backend → UI)
- Specific (not "build authentication", but "add password hashing to user model")

### 5. Customize CLAUDE.md Prompt

Before running Ralph, customize `scripts/ralph/CLAUDE.md` for Protocol M:

Add project-specific quality checks:
```bash
# Example quality checks for OpenClaw
cd openclaw && cargo test --all
cargo clippy -- -D warnings
cargo build --release
```

Add codebase conventions:
- "All Rust code must use explicit error types (no unwrap in production)"
- "Database migrations must include rollback statements"
- "API endpoints must have rate limiting configured"

### 6. Run Ralph

```bash
cd /Users/gfw/clawd/moltbook
./scripts/ralph/ralph.sh --tool claude 50
```

**Parameters:**
- `50` = max iterations (adjust based on task count)
- `--tool claude` = use Claude Code instead of Amp

**What Ralph will do:**
1. Create feature branch `ralph/protocol-m`
2. Pick highest priority task where `passes: false`
3. Implement that single task
4. Run quality checks (typecheck, tests)
5. Commit if checks pass
6. Update `prd.json` to mark `passes: true`
7. Append learnings to `progress.txt`
8. Repeat until all tasks pass or max iterations reached

### 7. Monitor Progress

**While running:**
```bash
# Check which tasks are complete
cd /Users/gfw/clawd/moltbook
cat prd.json | jq '.userStories[] | {id, title, passes}'

# See recent learnings
tail -20 progress.txt

# Check git history
git log --oneline -10
```

**If Ralph gets stuck:**
- Check `progress.txt` for error patterns
- Review the current task's acceptance criteria
- Manually fix blockers and resume Ralph

### 8. Expected Deliverables After Overnight Run

**Phase 1 (Identity Layer) - 40+ tasks:**
- ✅ OpenClaw CLI binary (`openclaw`)
- ✅ Identity generation, signing, verification working
- ✅ Golden test vectors passing
- ✅ Integration tests passing
- ✅ File permissions enforced
- ✅ Age encryption working
- ✅ DID derivation correct

**Phase 2 (Moltbook Integration) - 25+ tasks:**
- ✅ DID binding API endpoints
- ✅ Challenge/response flow
- ✅ Signature verification logic
- ✅ Verified badge UI component
- ✅ Profile DID display
- ✅ Post signature envelope storage

**Phase 3 (Attribution) - 20+ tasks:**
- ✅ ClawdHub artifact registry
- ✅ Derivation graph queries
- ✅ API endpoints for artifact registration

**Phase 4 (Economics) - 20+ tasks (if Oracle provides concrete design):**
- ✅ $SPORE account tables
- ✅ Transaction logic
- ✅ Escrow system
- ✅ Bounty marketplace API

**Phase 5 (Governance) - 10+ tasks:**
- ✅ Policy file validation
- ✅ Approval tier enforcement
- ✅ Emergency stop mechanism

## Post-Oracle Checklist

Before running Ralph, ensure:

- [ ] Oracle output reviewed and saved
- [ ] Economics gaps resolved (bootstrap, cash-out, settlement)
- [ ] Token economics validated (not circular, creates real value)
- [ ] PRD enhanced with Oracle recommendations
- [ ] `prd.json` generated with 100+ granular tasks
- [ ] Tasks ordered by dependencies
- [ ] All tasks have verifiable acceptance criteria
- [ ] UI tasks include "Verify in browser" criteria
- [ ] `scripts/ralph/CLAUDE.md` customized for Protocol M
- [ ] Quality check commands configured (cargo test, typecheck, etc.)
- [ ] Git branch strategy decided (use PRD `branchName` field)

## Key Success Factors

1. **Task Granularity:** Each task must be atomic and completable in one iteration
2. **Dependency Order:** Database → Backend → UI → Integration
3. **Verifiable Criteria:** No vague acceptance criteria ("works well" → "button displays green checkmark")
4. **Quality Gates:** Every commit must pass typecheck/tests
5. **Learnings Loop:** progress.txt captures patterns for future iterations

## Troubleshooting

**If Ralph produces broken code:**
- Tasks too large → split into smaller tasks
- Missing context → add to progress.txt Codebase Patterns
- Bad acceptance criteria → make them more specific

**If Ralph gets stuck on a task:**
- Review the task's acceptance criteria
- Check if dependencies are actually complete
- Manually implement and commit, mark `passes: true`, resume Ralph

**If quality checks fail:**
- Don't commit broken code
- Ralph will retry the task in next iteration
- Add learnings to progress.txt about the failure

---

**Ready to execute when Oracle completes!**
