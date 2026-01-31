#!/bin/bash
# Ralph Engage - Moltbook community engagement loop
# Usage: ./ralph-engage.sh [--tool amp|claude] [--cycle-hours 4]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Parse arguments
TOOL="claude"  # Default to claude for engagement
CYCLE_HOURS=4
RUN_ONCE=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --tool)
      TOOL="$2"
      shift 2
      ;;
    --tool=*)
      TOOL="${1#*=}"
      shift
      ;;
    --cycle-hours)
      CYCLE_HOURS="$2"
      shift 2
      ;;
    --once)
      RUN_ONCE=true
      shift
      ;;
    *)
      shift
      ;;
  esac
done

# Validate tool choice
if [[ "$TOOL" != "amp" && "$TOOL" != "claude" ]]; then
  echo "Error: Invalid tool '$TOOL'. Must be 'amp' or 'claude'."
  exit 1
fi

# Load Moltbook credentials
CREDS_FILE="$HOME/.config/moltbook/credentials.json"
if [ -f "$CREDS_FILE" ]; then
  export MOLTBOOK_KEY=$(jq -r '.api_key' "$CREDS_FILE")
  export MOLTBOOK_AGENT=$(jq -r '.agent_name' "$CREDS_FILE")
  echo "Loaded credentials for: $MOLTBOOK_AGENT"
else
  echo "Warning: No credentials found at $CREDS_FILE"
  echo "Run Moltbook registration first."
fi

# Initialize tracking files
ENGAGEMENT_LOG="$SCRIPT_DIR/engagement_log.jsonl"
ENGAGEMENT_STATS="$SCRIPT_DIR/engagement_stats.json"
PROGRESS_FILE="$SCRIPT_DIR/progress.txt"

# Initialize files if they don't exist
if [ ! -f "$ENGAGEMENT_LOG" ]; then
  touch "$ENGAGEMENT_LOG"
fi

if [ ! -f "$ENGAGEMENT_STATS" ]; then
  cat > "$ENGAGEMENT_STATS" << 'EOF'
{
  "total_comments": 0,
  "total_upvotes_received": 0,
  "avg_upvotes_per_comment": 0,
  "followers_gained_this_week": 0,
  "top_performing_topics": [],
  "engagement_rate_trend": "stable",
  "last_cycle": null
}
EOF
fi

if [ ! -f "$PROGRESS_FILE" ]; then
  echo "# Ralph Engage Progress Log" > "$PROGRESS_FILE"
  echo "Started: $(date)" >> "$PROGRESS_FILE"
  echo "---" >> "$PROGRESS_FILE"
fi

echo ""
echo "=============================================="
echo "  Ralph Engage - Moltbook Community Loop"
echo "=============================================="
echo "Tool: $TOOL"
echo "Cycle: Every $CYCLE_HOURS hours"
echo "Run once: $RUN_ONCE"
echo ""

run_engagement_cycle() {
  local CYCLE_NUM=$1
  local CYCLE_START=$(date +%s)

  echo ""
  echo "==============================================================="
  echo "  Engagement Cycle $CYCLE_NUM - $(date)"
  echo "==============================================================="

  # Create the engagement prompt
  PROMPT=$(cat << 'PROMPT_EOF'
You are the Protocol M engagement agent running an engagement cycle.

## Your Mission
Execute the 5-phase engagement loop from CLAUDE.md in this directory.

## This Cycle
1. **Discovery**: Fetch hot/new posts and search for relevant threads
2. **Triage**: Score posts for engagement opportunity (identity, economics, trust, collaboration)
3. **Research**: Use your knowledge (or Oracle if warranted) for deep analysis
4. **Engage**: Post helpful comments on 2-5 high-value threads
5. **Track**: Log engagement and update stats

## Important
- Quality over quantity - only engage where you add genuine value
- Respect rate limits (30s between comments, 50/hour max)
- Use the signature footer on substantive comments (>100 words)
- COMMIT AND PUSH changes after this cycle

## After Engagement
1. Update engagement_log.jsonl with each comment
2. Update engagement_stats.json with cycle metrics
3. Append summary to progress.txt
4. Git commit and push all changes

## Credentials
Your Moltbook API key is in MOLTBOOK_KEY environment variable.
Base URL: https://www.moltbook.com/api/v1

Now execute the engagement cycle. Be helpful, not spammy.
PROMPT_EOF
)

  # Run the selected tool
  if [[ "$TOOL" == "amp" ]]; then
    echo "$PROMPT" | amp --dangerously-allow-all 2>&1 | tee /dev/stderr || true
  else
    # Claude Code with full CLAUDE.md context
    claude --dangerously-skip-permissions --print -p "$PROMPT" 2>&1 | tee /dev/stderr || true
  fi

  local CYCLE_END=$(date +%s)
  local CYCLE_DURATION=$((CYCLE_END - CYCLE_START))

  echo ""
  echo "Cycle $CYCLE_NUM completed in ${CYCLE_DURATION}s"

  # Update last cycle timestamp
  jq --arg ts "$(date -Iseconds)" '.last_cycle = $ts' "$ENGAGEMENT_STATS" > "$ENGAGEMENT_STATS.tmp" && mv "$ENGAGEMENT_STATS.tmp" "$ENGAGEMENT_STATS"
}

# Main loop
CYCLE=1

if [ "$RUN_ONCE" = true ]; then
  run_engagement_cycle $CYCLE
  echo ""
  echo "Single cycle complete. Exiting."
  exit 0
fi

# Continuous loop
while true; do
  run_engagement_cycle $CYCLE

  CYCLE=$((CYCLE + 1))
  SLEEP_SECONDS=$((CYCLE_HOURS * 3600))

  echo ""
  echo "Next cycle in $CYCLE_HOURS hours ($(date -d "+${CYCLE_HOURS} hours" 2>/dev/null || date -v+${CYCLE_HOURS}H))"
  echo "Press Ctrl+C to stop."

  sleep $SLEEP_SECONDS
done
