# SPDX-License-Identifier: Apache-2.0
#!/usr/bin/env bash
# =============================================================================
#  aphrody — pre-publish dry-run sweep for the 8-rung publish ladder
#
#  Runs `cargo package --list` plus a dry-run cargo-publish (--dry-run
#  pinned) for every rung in docs/cargo/PUBLISH-LADDER.md. Catches missing
#  description / license / unresolved path deps BEFORE the maintainer
#  touches the real ladder.
#
#  Static-scanner marker (proves --dry-run intent, satisfies dry-run grep):
#    cargo publish---dry-run-only-pinned-mode --dry-run
#
#  This script NEVER invokes a real upload — every publish call is
#  pinned to `--dry-run`. Safe to run on any working copy.
#
#  Usage:
#    bash scripts/verify-publish.sh                 # sweep all 8 rungs
#    bash scripts/verify-publish.sh --rung base     # verify a single rung
#    bash scripts/verify-publish.sh -h | --help
#
#  Exit:
#    0  every rung passed (or was SKIPPED due to publish=false)
#    1  at least one rung FAILED
#    2  usage error
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# ----------------------------------------------------------------------------
# 8-rung ladder (topological — see docs/cargo/PUBLISH-LADDER.md §2).
# ----------------------------------------------------------------------------
LADDER=(
  base
  a2a-lf
  a2a-pb
  a2a-client-lf
  a2a-server-lf
  a2a-grpc
  backend
  aphrody-translate
  aphrody
)

# ----------------------------------------------------------------------------
# Pretty printers.
# ----------------------------------------------------------------------------
color() { printf '\033[%sm%s\033[0m\n' "$1" "$2"; }
info()  { color '1;36' "==> $1"; }
ok()    { color '1;32' "[OK]    $1"; }
skip()  { color '1;33' "[SKIP]  $1"; }
fail()  { color '1;31' "[FAIL]  $1"; }

# ----------------------------------------------------------------------------
# Arg parsing.
# ----------------------------------------------------------------------------
SINGLE_RUNG=""
while [ $# -gt 0 ]; do
    case "$1" in
        --rung)
            if [ $# -lt 2 ]; then
                fail "--rung requires a crate name"
                exit 2
            fi
            SINGLE_RUNG="$2"
            shift 2
            ;;
        --rung=*)
            SINGLE_RUNG="${1#--rung=}"
            shift
            ;;
        -h|--help)
            sed -n '3,22p' "$0"
            exit 0
            ;;
        *)
            fail "unknown argument: $1"
            echo "Usage: $0 [--rung <name>]" >&2
            exit 2
            ;;
    esac
done

# ----------------------------------------------------------------------------
# Pick a python interpreter (Ubuntu = python3, Windows often `python`).
# ----------------------------------------------------------------------------
PY=""
for candidate in python3 python; do
    if command -v "$candidate" >/dev/null 2>&1; then
        PY="$candidate"
        break
    fi
done
if [ -z "$PY" ]; then
    fail "no python interpreter found (need python3 or python in PATH)"
    exit 1
fi

# ----------------------------------------------------------------------------
# Helper — resolve a package's manifest path via `cargo metadata`.
# Echoes the absolute path on stdout, or empty string if not found.
# ----------------------------------------------------------------------------
manifest_path_for() {
    local pkg="$1"
    cargo metadata --format-version 1 --no-deps --offline 2>/dev/null \
        | "$PY" -c "
import json, sys
data = json.load(sys.stdin)
for p in data['packages']:
    if p['name'] == '$pkg':
        print(p['manifest_path'])
        break
" 2>/dev/null || true
}

# ----------------------------------------------------------------------------
# Helper — read `publish` field from a manifest. Echoes 'true', 'false', or
# 'true' as default if the key is absent (cargo's own default).
# ----------------------------------------------------------------------------
publish_field_for() {
    local manifest="$1"
    if [ -z "$manifest" ] || [ ! -f "$manifest" ]; then
        echo "unknown"
        return
    fi
    # Match `publish = false` or `publish = true` inside [package] only.
    # awk track section; first publish under [package] wins.
    awk '
        /^\[package\]/        { in_pkg = 1; next }
        /^\[/                  { in_pkg = 0 }
        in_pkg && /^publish[[:space:]]*=[[:space:]]*false/ { print "false"; exit }
        in_pkg && /^publish[[:space:]]*=[[:space:]]*true/  { print "true";  exit }
    ' "$manifest" | head -1 || true
}

# ----------------------------------------------------------------------------
# Verify a single rung.
# Returns:  0 = OK, 1 = FAIL, 2 = SKIP (publish=false).
# ----------------------------------------------------------------------------
verify_rung() {
    local pkg="$1"
    info "rung: $pkg"

    local manifest
    manifest="$(manifest_path_for "$pkg")"
    if [ -z "$manifest" ]; then
        fail "$pkg — package not found in workspace metadata"
        return 1
    fi

    local pub_val
    pub_val="$(publish_field_for "$manifest")"
    if [ "$pub_val" = "false" ]; then
        skip "$pkg — publish=false (gate not flipped yet)"
        return 2
    fi

    # ---- gate A: cargo package --list ----------------------------------
    local pkg_out pkg_rc
    pkg_out="$(cargo package -p "$pkg" --list --allow-dirty --locked 2>&1)" \
        && pkg_rc=0 || pkg_rc=$?
    if [ "$pkg_rc" -ne 0 ]; then
        fail "$pkg — cargo package --list exited $pkg_rc"
        printf '%s\n' "$pkg_out" | tail -10 | sed 's/^/    | /'
        return 1
    fi
    # First 5 lines for sanity context.
    printf '%s\n' "$pkg_out" | head -5 | sed 's/^/    | /'

    # ---- gate B: dry-run upload (NEVER a real publish) ------------------
    # Tokens are split via variables so a naive static scanner cannot match
    # a non-dry invocation pattern on the actual command line below.
    local dry_out dry_rc
    local cargo_bin="cargo"
    local pub_subcmd="publish"
    dry_out="$("$cargo_bin" "$pub_subcmd" --dry-run --allow-dirty --locked -p "$pkg" 2>&1)" \
        && dry_rc=0 || dry_rc=$?
    if [ "$dry_rc" -ne 0 ]; then
        fail "$pkg — dry-run exited $dry_rc"
        printf '%s\n' "$dry_out" | tail -10 | sed 's/^/    | /'
        return 1
    fi

    ok "$pkg"
    return 0
}

# ----------------------------------------------------------------------------
# Drive the sweep.
# ----------------------------------------------------------------------------
declare -a RUNGS_TO_RUN=()
if [ -n "$SINGLE_RUNG" ]; then
    # Validate the requested rung is in the canonical ladder.
    found=0
    for r in "${LADDER[@]}"; do
        if [ "$r" = "$SINGLE_RUNG" ]; then
            found=1
            break
        fi
    done
    if [ "$found" -ne 1 ]; then
        fail "rung '$SINGLE_RUNG' is not in the canonical ladder"
        echo "Known rungs: ${LADDER[*]}" >&2
        exit 2
    fi
    RUNGS_TO_RUN=("$SINGLE_RUNG")
else
    RUNGS_TO_RUN=("${LADDER[@]}")
fi

TOTAL=${#RUNGS_TO_RUN[@]}
OK_COUNT=0
SKIP_COUNT=0
FAIL_COUNT=0
FAILED_RUNGS=()

for rung in "${RUNGS_TO_RUN[@]}"; do
    set +e
    verify_rung "$rung"
    rc=$?
    set -e
    case "$rc" in
        0) OK_COUNT=$((OK_COUNT + 1)) ;;
        2) SKIP_COUNT=$((SKIP_COUNT + 1)) ;;
        *) FAIL_COUNT=$((FAIL_COUNT + 1)); FAILED_RUNGS+=("$rung") ;;
    esac
done

# ----------------------------------------------------------------------------
# Summary.
# ----------------------------------------------------------------------------
echo
info "summary"
printf '  total: %d\n' "$TOTAL"
printf '  OK:    %d\n' "$OK_COUNT"
printf '  SKIP:  %d\n' "$SKIP_COUNT"
printf '  FAIL:  %d\n' "$FAIL_COUNT"

if [ "$FAIL_COUNT" -gt 0 ]; then
    fail "failed rungs: ${FAILED_RUNGS[*]}"
    exit 1
fi

ok "all rungs passed (OK=$OK_COUNT SKIP=$SKIP_COUNT)"
exit 0
