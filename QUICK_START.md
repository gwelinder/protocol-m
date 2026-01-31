# Protocol M - Quick Start Guide

## TL;DR - Start Ralph Now

```bash
cd /Users/gfw/clawd/moltbook
./scripts/ralph/ralph.sh --tool claude 50
```

## What You Have

✅ **104 tasks** in prd.json (OpenClaw CLI → Economics → Governance)
✅ **Oracle economics** validated ($1.35, GPT-5.2 Pro)
✅ **Moltbook verified** (protocol-m-ralph)
✅ **Ralph configured** (scripts ready)

## Key Files

- `prd.json` - Task list
- `progress.txt` - Execution log
- `LAUNCH_SUMMARY.md` - Full details
- `READY_FOR_RALPH.md` - Complete guide

## Monitor Progress

```bash
# Tasks remaining
cat prd.json | jq '.userStories[] | select(.passes == false) | .id'

# Recent learnings
tail -20 progress.txt

# Git commits
git log --oneline -10
```

## Moltbook

- Profile: https://moltbook.com/u/protocol-m-ralph
- First post: https://moltbook.com/post/7c41bfce-fec1-45f6-a301-f25dccea195b
- API key: `~/.config/moltbook/credentials.json`

## Post Updates

```bash
curl -X POST https://www.moltbook.com/api/v1/posts \
  -H "Authorization: Bearer moltbook_sk_zOOKJD4ufgp8EKvMRwQe-qcdmg7BeSwU" \
  -H "Content-Type: application/json" \
  -d '{"submolt": "general", "title": "Update", "content": "Progress..."}'
```

## Expected Output

**Phase 1:** OpenClaw CLI (Rust, Ed25519, DID generation)
**Phase 2:** ClawdHub (artifact registry, attribution graph)
**Phase 3:** Moltbook (DID binding, signature verification)
**Phase 4:** Economics (M-Credits, bounties, escrow)
**Phase 5:** Governance (policies, approvals, kill switch)

## If Stuck

1. Check `progress.txt` for errors
2. Review task acceptance criteria
3. Fix blockers manually
4. Mark `passes: true` in prd.json
5. Resume Ralph

---

**Status:** 🚀 READY TO LAUNCH
