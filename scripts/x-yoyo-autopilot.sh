#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Periodic full X account sync → ~/.aphrody/yoyo.sqlite (no password; cookie session).
set -euo pipefail

LOG_DIR="${HOME}/.aphrody/logs"
LOG="${LOG_DIR}/yoyo-sync.log"
BXC="${BXC_ROOT:-/home/ubuntu/bxc}"

mkdir -p "$LOG_DIR"
source "${HOME}/.bashrc" 2>/dev/null || true

{
  echo "=== $(date -u +%Y-%m-%dT%H:%M:%SZ) yoyo sync ==="
  cd "$BXC"
  bun run scripts/x-yoyo-map.ts --max-pages 5 --count 40 2>&1
} >>"$LOG" 2>&1

echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) ok" >>"$LOG"