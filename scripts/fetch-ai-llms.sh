#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Fetch llms.txt mirrors and refresh CLI --help snapshots for the unified agent stack.
set -euo pipefail

OUT="${APHRODY_ROOT:-/home/ubuntu/aphrody}/docs/agent-stack"
mkdir -p "$OUT"

fetch_llms() {
  local url="$1"
  local out="$2"
  if curl -fsSL --max-time 30 "$url" -o "$out.tmp" 2>/dev/null; then
    mv "$out.tmp" "$out"
    echo "  ok: $url -> $(basename "$out")"
  else
    rm -f "$out.tmp"
    echo "  skip: $url"
  fi
}

echo "==> llms.txt mirrors"
fetch_llms "https://docs.x.ai/llms.txt" "$OUT/x-ai-llms.txt"
fetch_llms "https://bun.com/docs/llms.txt" "$OUT/bun-llms.txt"
fetch_llms "https://docs.anthropic.com/llms.txt" "$OUT/anthropic-llms.txt" || true
fetch_llms "https://code.claude.com/docs/llms.txt" "$OUT/claude-code-llms.txt" || true

echo "==> CLI --help snapshots"
for cmd in agy grok claude bxc aphrody; do
  if command -v "$cmd" &>/dev/null; then
    "$cmd" --help >"$OUT/${cmd}-help.txt" 2>&1 || true
    echo "  $cmd -> ${cmd}-help.txt"
  fi
done

echo "Done: $OUT"