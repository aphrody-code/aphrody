#!/usr/bin/env bash
# regen-baseline — regenerate the 7 tracked baseline snapshots
# Refuses if the n2b binary version doesn't match Cargo.toml (stale binary).

set -eu
cd "$(git rev-parse --show-toplevel)" || { echo "not in a git repo"; exit 1; }

BIN="./target/release/n2b"
DIR="tests/snapshots/baseline"
DIM="\033[2m"
GREEN="\033[32m"
YELLOW="\033[33m"
RED="\033[31m"
RESET="\033[0m"

echo "=== regen-baseline ==="

# 1. Build the binary if missing (release).
if [ ! -x "$BIN" ]; then
  echo -e "${YELLOW}binary missing — running cargo build --release -p n2b${RESET}"
  cargo build --release -p n2b
fi

# 2. Anti-stale check: n2b --version must match Cargo.toml.
BIN_VER=$("$BIN" --version | awk '{print $2}')
CARGO_VER=$(grep -m1 '^version' crates/n2b-cli/Cargo.toml | sed -E 's/version *= *"([^"]+)"/\1/')
if [ "$BIN_VER" != "$CARGO_VER" ]; then
  echo -e "${RED}stale binary: n2b --version=$BIN_VER vs Cargo.toml=$CARGO_VER${RESET}"
  echo "rebuild first: cargo build --release -p n2b"
  exit 1
fi
echo -e "binary: n2b $BIN_VER (matches Cargo.toml ${GREEN}✓${RESET})"

mkdir -p "$DIR"

# 3. Fixture baselines (5 formats).
i=1
for fmt in text json jsonl md sarif; do
  ext="$fmt"
  [ "$fmt" = "text" ] && ext="txt"
  out="$DIR/fixture.$ext"
  echo -e "${DIM}[$i/7] $out${RESET}"
  "$BIN" test/fixture --report="$fmt" > "$out"
  i=$((i + 1))
done

# 4. Rules baselines (json + text).
echo -e "${DIM}[$i/7] $DIR/rules.json${RESET}"
"$BIN" rules --report=json > "$DIR/rules.json"
i=$((i + 1))

echo -e "${DIM}[$i/7] $DIR/rules.txt${RESET}"
"$BIN" rules > "$DIR/rules.txt"

echo ""
echo "git diff --stat:"
git diff --stat "$DIR" || true

echo ""
echo -e "${YELLOW}→ Relire le diff avant de commit. Si N fichiers diff alors qu'on en attendait M, STOP.${RESET}"
