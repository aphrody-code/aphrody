#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# scan-repo.sh — one-shot health & inventory scan of the aphrody monorepo.
#
# Surfaces, in one pass, the things a reviewer (or a release gate) wants to know:
#   1. Workspace inventory     — Cargo members, crate dirs on disk, TS/py surfaces.
#   2. Doc-link integrity       — delegates to scan-doc-links.py (broken rel links).
#   3. Stale-reference scan     — docs/code that still name removed crates/dirs.
#   4. Stub / TODO markers       — `todo!()`, `unimplemented!()`, "TODO: implement".
#   5. Hardcoded machine paths   — committed absolute paths (/home/<user>, C:\..., /Users).
#
# Read-only: never modifies the tree. Intended to be run from anywhere.
#
# Usage:
#   scripts/scan-repo.sh              # full scan, human-readable
#   scripts/scan-repo.sh --quiet      # only section headers + counts
#   FAIL_ON_LINKS=1 scripts/scan-repo.sh   # exit non-zero if broken doc links
#
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

QUIET=0
[ "${1:-}" = "--quiet" ] && QUIET=1

hr() { printf '\n=== %s ===\n' "$1"; }
have() { command -v "$1" >/dev/null 2>&1; }

# ---------------------------------------------------------------------------
hr "1. Workspace inventory"
if have python3; then
  python3 - <<'PY'
import re
t = open("Cargo.toml", encoding="utf-8", errors="replace").read()
def block(name):
    m = re.search(r'\b' + name + r'\s*=\s*\[(.*?)\]', t, re.S)
    return re.findall(r'"([^"]+)"', m.group(1)) if m else []
print(f"  Cargo workspace members : {len(block('members'))}")
print(f"  Cargo workspace exclude : {len(block('exclude'))}")
PY
fi
echo "  crate dirs on disk      : $(find crates -maxdepth 1 -mindepth 1 -type d 2>/dev/null | wc -l)"
echo "  TS packages (packages/) : $(find packages -maxdepth 1 -mindepth 1 -type d 2>/dev/null | wc -l)"
echo "  TS apps (apps/)         : $(find apps -maxdepth 1 -mindepth 1 -type d 2>/dev/null | wc -l)"
echo "  doc files (docs/**.md)  : $(find docs -name '*.md' 2>/dev/null | wc -l)"
echo "  workspace version       : $(grep -m1 '^version' Cargo.toml | sed 's/.*=\s*//')"

# ---------------------------------------------------------------------------
hr "2. Doc-link integrity"
if have python3 && [ -f "$SCRIPT_DIR/scan-doc-links.py" ]; then
  if python3 "$SCRIPT_DIR/scan-doc-links.py"; then
    LINKS_OK=1
  else
    LINKS_OK=0
  fi
else
  echo "  (skipped — needs python3 + scripts/scan-doc-links.py)"
  LINKS_OK=1
fi

# ---------------------------------------------------------------------------
hr "3. Stale references to removed crates/dirs"
# Crates/dirs removed during the cleanup; any *current-tense* mention in docs is suspect.
STALE='crates/gui|crates/google_os|crates/bun_ffi|crates/aphrody-wasm|crates/mrx-(core|detect|audit|watch|cli)|vendor/coreutils|vendor/util-linux|packaging/install\.(sh|ps1)|docs/posts/|docs/ievr/|docs/launch/|docs/extensions/|docs/adr/'
if have rg; then
  HITS=$(rg -n --no-heading -e "$STALE" --glob '*.md' --glob '!node_modules' . 2>/dev/null | wc -l)
  echo "  doc mentions of removed paths: $HITS"
  [ "$QUIET" = 0 ] && rg -n --no-heading -e "$STALE" --glob '*.md' --glob '!node_modules' . 2>/dev/null | head -30
else
  HITS=$(grep -rEn "$STALE" --include='*.md' . 2>/dev/null | grep -v node_modules | wc -l)
  echo "  doc mentions of removed paths: $HITS"
  [ "$QUIET" = 0 ] && grep -rEn "$STALE" --include='*.md' . 2>/dev/null | grep -v node_modules | head -30
fi

# ---------------------------------------------------------------------------
hr "4. Stub / TODO markers in Rust"
if have rg; then
  STUBS=$(rg -n --no-heading -e 'todo!\(\)|unimplemented!\(\)|TODO: ?implement|FIXME' --glob 'crates/**/*.rs' . 2>/dev/null | wc -l)
  echo "  todo!()/unimplemented!()/FIXME in crates/: $STUBS"
  [ "$QUIET" = 0 ] && rg -n --no-heading -e 'todo!\(\)|unimplemented!\(\)|TODO: ?implement|FIXME' --glob 'crates/**/*.rs' . 2>/dev/null | head -20
else
  echo "  (rg not found — skipped)"
fi

# ---------------------------------------------------------------------------
hr "5. Hardcoded machine paths in committed code/scripts"
# Absolute home/user paths that break portability if committed.
PATHPAT='/home/[a-z][a-z0-9_-]*/|C:\\\\Users\\\\|/Users/[a-z]'
if have rg; then
  PHITS=$(rg -n --no-heading -e "$PATHPAT" \
    --glob 'scripts/**' --glob 'crates/**/*.rs' --glob '.cargo/**' \
    --glob '!*.md' . 2>/dev/null | grep -vE 'runner=|remap-path' | wc -l)
  echo "  hardcoded user/home paths (scripts, crates, .cargo): $PHITS"
  [ "$QUIET" = 0 ] && rg -n --no-heading -e "$PATHPAT" \
    --glob 'scripts/**' --glob 'crates/**/*.rs' --glob '.cargo/**' \
    --glob '!*.md' . 2>/dev/null | grep -vE 'runner=|remap-path' | head -20
else
  echo "  (rg not found — skipped)"
fi

# ---------------------------------------------------------------------------
hr "Summary"
echo "  doc links: $([ "${LINKS_OK:-1}" = 1 ] && echo OK || echo 'BROKEN — see section 2')"
if [ "${FAIL_ON_LINKS:-0}" = 1 ] && [ "${LINKS_OK:-1}" != 1 ]; then
  exit 1
fi
exit 0
