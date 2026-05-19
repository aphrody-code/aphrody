#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# autopilot — pilote Claude Code + Gemini CLI en duel parallèle, à l'infini.
#
# Spawn par la slash command /aphrody:autopilot ou en standalone :
#
#   bash scripts/autopilot.sh           # boucle infinie, prompts par défaut (PLAN.md ⏳)
#   bash scripts/autopilot.sh --once    # un seul tick (debug)
#   bash scripts/autopilot.sh --interval 30 --max-ticks 100
#
# Sorties :
#   var/log/autopilot.jsonl  — NDJSON, 1 ligne par tick (claude + gemini)
#   var/run/autopilot.pid    — PID du loop, pour kill propre
#   ai/heartbeat.txt         — bump ISO-8601 à chaque tick (A2A)
#
# SIGINT (Ctrl-C) ou `kill $(cat var/run/autopilot.pid)` → arrêt propre.
#
# Conforme `feedback_aphrody_full_autonomy` : aucune intervention humaine ;
# `claude -p` et `gemini` tournent non-interactif ; sleep entre ticks pour
# respecter rate-limits provider.

set -u  # not -e : on veut continuer même si un tick crash

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${REPO_ROOT}/var/log/autopilot.jsonl"
PID_FILE="${REPO_ROOT}/var/run/autopilot.pid"
HEARTBEAT="${REPO_ROOT}/ai/heartbeat.txt"

INTERVAL="${APHRODY_AUTOPILOT_INTERVAL:-60}"   # secondes entre ticks
MAX_TICKS="${APHRODY_AUTOPILOT_MAX_TICKS:-0}"  # 0 = infini
ONCE=0
CLAUDE_TIMEOUT="${APHRODY_CLAUDE_TIMEOUT:-300}"
GEMINI_TIMEOUT="${APHRODY_GEMINI_TIMEOUT:-300}"

# arg parsing
while [[ $# -gt 0 ]]; do
  case "$1" in
    --once) ONCE=1; shift ;;
    --interval) INTERVAL="$2"; shift 2 ;;
    --max-ticks) MAX_TICKS="$2"; shift 2 ;;
    --claude-timeout) CLAUDE_TIMEOUT="$2"; shift 2 ;;
    --gemini-timeout) GEMINI_TIMEOUT="$2"; shift 2 ;;
    -h|--help)
      sed -n '3,18p' "${BASH_SOURCE[0]}"
      exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

mkdir -p "${REPO_ROOT}/var/log" "${REPO_ROOT}/var/run" "${REPO_ROOT}/ai"

# refuse double-start
if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE" 2>/dev/null)" 2>/dev/null; then
  echo "autopilot already running (pid=$(cat "$PID_FILE")). 'kill \$(cat $PID_FILE)' to stop." >&2
  exit 1
fi
echo "$$" > "$PID_FILE"

cleanup() {
  rm -f "$PID_FILE"
  echo "{\"ts\":\"$(date -u +%FT%TZ)\",\"event\":\"autopilot_stop\",\"pid\":$$}" >> "$LOG"
  exit 0
}
trap cleanup INT TERM

# -- Sélection de la prochaine tâche ⏳ depuis docs/PLAN.md ----------------
#
# Format attendu : table markdown avec lignes commençant par "| N |" et
# contenant un ⏳ dans la colonne "Item". On extrait nom + cible + verify.

pick_next_task() {
  # Fallback simple : si PLAN.md absent ou aucun ⏳ trouvé, prompt curiosité.
  local plan="${REPO_ROOT}/docs/PLAN.md"
  if [[ ! -f "$plan" ]]; then
    echo "audit-curiosity"
    return
  fi
  local line
  line=$(grep -m1 "⏳" "$plan" 2>/dev/null || true)
  if [[ -z "$line" ]]; then
    echo "audit-curiosity"
    return
  fi
  # Extract first ~120 chars after first ⏳, sanitise.
  printf "%s" "$line" | tr -d '\n' | sed 's/|/ /g' | cut -c1-160
}

# -- Prompts ----------------------------------------------------------------
#
# Claude lane : delegate to /aphrody:aphrody-yolo-grind (4 lanes parallèles)
# Gemini lane : audit + propose alternative crate pour le dernier commit

claude_prompt() {
  local task="$1"
  cat <<EOF
You are running inside the aphrody autopilot loop (tick $TICK_N). Read
CLAUDE.md §0.1 (zero human-in-loop) and §0.2 (pre-authorized actions).
Pick the highest-leverage ⏳ item from docs/PLAN.md and ship it end-to-end :
implement, cargo check, commit (Conventional Commit, no AI co-author).

Selected hint (may be stale, you choose): $task

Constraints: stay reversible, no force-push, no destructive rm outside
target/ + var/. If blocked, surface the blocker in commit message footer
"NON_FAIT: <reason>" and pick another ⏳ item.
EOF
}

gemini_prompt() {
  local task="$1"
  cat <<EOF
You are the second lane of the aphrody autopilot duel (tick $TICK_N).
Read AGENTS.md + CLAUDE.md §0.1. Independently audit the most recent
commit on origin/main : check it against best-stack-2026 skill recos
(canonical Rust 2026 crates), license safety (no GPL leak), cross-platform
build (Linux/Windows/wasm32).

Output JSON only (single line): {"verdict":"ack|nack","reasons":["..."],
"suggested_followup":"..."}. Append your output as evidence; do NOT modify
files. If main is clean, suggest the next ⏳ item from docs/PLAN.md.

Hint: $task
EOF
}

# -- Lane runners (background, parallel) ------------------------------------

run_claude_lane() {
  local out="$1" task="$2"
  if ! command -v claude >/dev/null 2>&1; then
    echo "{\"lane\":\"claude\",\"err\":\"claude CLI not found in PATH\"}" > "$out"
    return
  fi
  # claude -p prompt en non-interactif ; timeout pour éviter de hanger
  local prompt; prompt=$(claude_prompt "$task")
  timeout "$CLAUDE_TIMEOUT" claude -p "$prompt" \
      --dangerously-skip-permissions 2>&1 | tail -c 4096 > "$out" \
    || echo "{\"lane\":\"claude\",\"err\":\"timeout or non-zero exit\"}" >> "$out"
}

run_gemini_lane() {
  local out="$1" task="$2"
  local cmd=""
  if command -v gemini >/dev/null 2>&1; then
    cmd="gemini"
  elif command -v bunx >/dev/null 2>&1; then
    cmd="bunx @google/gemini-cli"
  else
    echo "{\"lane\":\"gemini\",\"err\":\"neither gemini nor bunx found\"}" > "$out"
    return
  fi
  local prompt; prompt=$(gemini_prompt "$task")
  # shellcheck disable=SC2086
  timeout "$GEMINI_TIMEOUT" $cmd --prompt "$prompt" 2>&1 | tail -c 4096 > "$out" \
    || echo "{\"lane\":\"gemini\",\"err\":\"timeout or non-zero exit\"}" >> "$out"
}

# -- Main loop --------------------------------------------------------------

TICK_N=0
echo "{\"ts\":\"$(date -u +%FT%TZ)\",\"event\":\"autopilot_start\",\"pid\":$$,\"interval\":$INTERVAL,\"max_ticks\":$MAX_TICKS}" >> "$LOG"

while true; do
  TICK_N=$((TICK_N + 1))
  TS=$(date -u +%FT%TZ)
  TASK=$(pick_next_task)
  CLAUDE_OUT=$(mktemp)
  GEMINI_OUT=$(mktemp)

  # parallel fan-out
  run_claude_lane "$CLAUDE_OUT" "$TASK" &
  CLAUDE_PID=$!
  run_gemini_lane "$GEMINI_OUT" "$TASK" &
  GEMINI_PID=$!
  wait "$CLAUDE_PID" "$GEMINI_PID" 2>/dev/null || true

  # Bump heartbeat
  echo "$TS autopilot tick #$TICK_N task=\"$TASK\"" > "$HEARTBEAT"

  # NDJSON log line (no jq dep — manual escape minimal)
  C_SUMMARY=$(head -c 800 "$CLAUDE_OUT" | tr '\n\t' '  ' | sed 's/"/\\"/g')
  G_SUMMARY=$(head -c 800 "$GEMINI_OUT" | tr '\n\t' '  ' | sed 's/"/\\"/g')
  printf '{"ts":"%s","tick":%d,"task":"%s","claude":"%s","gemini":"%s"}\n' \
    "$TS" "$TICK_N" "$TASK" "$C_SUMMARY" "$G_SUMMARY" >> "$LOG"

  rm -f "$CLAUDE_OUT" "$GEMINI_OUT"

  # stop conditions
  if [[ "$ONCE" -eq 1 ]]; then break; fi
  if [[ "$MAX_TICKS" -gt 0 ]] && [[ "$TICK_N" -ge "$MAX_TICKS" ]]; then break; fi

  sleep "$INTERVAL"
done

cleanup
