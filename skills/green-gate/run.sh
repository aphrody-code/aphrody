#!/usr/bin/env bash
# green-gate — n2b verification pipeline
# Stops at first failure. Exit 0 = all green, 1 = at least one stage red.

set -u
cd "$(git rev-parse --show-toplevel)" || { echo "not in a git repo"; exit 1; }

GREEN="\033[32m"
RED="\033[31m"
DIM="\033[2m"
RESET="\033[0m"

run() {
  local label="$1"
  shift
  echo -e "${DIM}[$label] $*${RESET}"
  if "$@"; then
    echo -e "  ${GREEN}✓ OK${RESET}"
    return 0
  else
    echo -e "  ${RED}✗ FAILED${RESET}"
    echo -e "${RED}green-gate: $label failed — aborting${RESET}"
    exit 1
  fi
}

echo "=== green-gate (n2b) ==="

run "1/6 fmt"       cargo fmt --all -- --check
run "2/6 build"     cargo build --workspace
run "3/6 test"      cargo test --workspace
run "4/6 clippy"    cargo clippy --workspace --all-targets -- -D warnings
run "5/6 codegen"   bun run codegen:schema:check
run "6/6 baselines" bash tests/compare-baseline.sh

echo ""
echo -e "${GREEN}green-gate: ALL GREEN — safe to push/deploy${RESET}"
