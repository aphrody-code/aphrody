#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Periodic full X account sync → ~/.aphrody/yoyo.sqlite (no password; cookie session).
set -euo pipefail

LOG_DIR="${HOME}/.aphrody/logs"
LOG="${LOG_DIR}/yoyo-sync.log"
BXC="${BXC_ROOT:-/home/ubuntu/bxc}"

mkdir -p "$LOG_DIR"
source "${HOME}/.bashrc" 2>/dev/null || true

# ~/.bashrc early-returns for non-interactive shells (cron / headless / systemd),
# so bun's PATH addition never loads there. Resolve bun deterministically: prefer
# PATH, then the standard install locations. Fail loudly if it's genuinely absent.
if ! command -v bun >/dev/null 2>&1; then
  for cand in "${HOME}/.bun/bin" "${BUN_INSTALL:-}/bin" /usr/local/bin; do
    [ -x "${cand}/bun" ] && export PATH="${cand}:${PATH}" && break
  done
fi
BUN_BIN="$(command -v bun || true)"

{
  echo "=== $(date -u +%Y-%m-%dT%H:%M:%SZ) yoyo sync ==="
  if [ -z "$BUN_BIN" ]; then
    echo "ERROR: bun not found in PATH or ~/.bun/bin — install bun or set BUN_INSTALL." >&2
    exit 127
  fi
  cd "$BXC"
  "$BUN_BIN" run scripts/x-yoyo-map.ts --max-pages 5 --count 40 2>&1
} >>"$LOG" 2>&1

echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) ok" >>"$LOG"