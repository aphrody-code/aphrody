# SPDX-License-Identifier: Apache-2.0
#!/usr/bin/env bash
# =============================================================================
#  aphrody — operational release helper
#
#  Bumps workspace version, runs pre-release gates, commits & tags locally.
#  Never auto-pushes (requires explicit --push). Never runs `cargo-publish`
#  (publish is a separate, manual ladder — see docs/cargo/PUBLISH-LADDER.md).
#
#  Usage:
#    ./scripts/release.sh 1.0.0-canary.1            # explicit version
#    ./scripts/release.sh --bump patch              # 1.0.0 -> 1.0.1
#    ./scripts/release.sh --bump minor              # 1.0.0 -> 1.1.0
#    ./scripts/release.sh --bump major              # 1.0.0 -> 2.0.0
#    ./scripts/release.sh 1.0.0-canary.1 --push     # also push to origin
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

CARGO_TOML="$REPO_ROOT/Cargo.toml"
MANIFEST="$REPO_ROOT/.release-please-manifest.json"
CARGO_BAK="$CARGO_TOML.bak"
MANIFEST_BAK="$MANIFEST.bak"

color() { printf '\033[%sm%s\033[0m\n' "$1" "$2"; }
info()  { color '1;36' "==> $1"; }
ok()    { color '1;32' "  OK $1"; }
fail()  { color '1;31' "  !! $1"; }

restore_backups() {
    if [ -f "$CARGO_BAK" ]; then
        mv "$CARGO_BAK" "$CARGO_TOML"
        fail "restored $CARGO_TOML from backup"
    fi
    if [ -f "$MANIFEST_BAK" ]; then
        mv "$MANIFEST_BAK" "$MANIFEST"
        fail "restored $MANIFEST from backup"
    fi
}
trap 'rc=$?; if [ $rc -ne 0 ]; then restore_backups; fi; exit $rc' EXIT

# ----------------------------------------------------------------------------
# Arg parsing
# ----------------------------------------------------------------------------
VERSION=""
BUMP=""
PUSH=0
for arg in "$@"; do
    case "$arg" in
        --push)         PUSH=1 ;;
        --bump=patch|--bump=minor|--bump=major) BUMP="${arg#--bump=}" ;;
        --bump)         shift; ;;
        patch|minor|major)
            if [ -z "$BUMP" ] && [ -z "$VERSION" ]; then BUMP="$arg"; fi ;;
        -h|--help)
            sed -n '3,17p' "$0"; exit 0 ;;
        *)
            if [ -z "$VERSION" ] && [ -z "$BUMP" ]; then VERSION="$arg"; fi ;;
    esac
done

# Re-walk argv to catch `--bump patch` (space-separated) form.
prev=""
for arg in "$@"; do
    if [ "$prev" = "--bump" ]; then
        case "$arg" in
            patch|minor|major) BUMP="$arg" ;;
        esac
    fi
    prev="$arg"
done

if [ -z "$VERSION" ] && [ -z "$BUMP" ]; then
    fail "usage: $0 <VERSION> | --bump patch|minor|major [--push]"
    exit 2
fi

# ----------------------------------------------------------------------------
# Resolve current version + compute new version if --bump
# ----------------------------------------------------------------------------
CURRENT="$(grep -E '^version[[:space:]]*=' "$CARGO_TOML" | head -1 | sed -E 's/.*=[[:space:]]*"([^"]+)".*/\1/')"
if [ -z "$CURRENT" ]; then
    fail "could not read current version from $CARGO_TOML"; exit 1
fi
info "current workspace version: $CURRENT"

if [ -n "$BUMP" ] && [ -z "$VERSION" ]; then
    # Strip pre-release tag for semver math, keep only X.Y.Z.
    base="${CURRENT%%-*}"
    IFS='.' read -r MAJ MIN PAT <<< "$base"
    case "$BUMP" in
        patch) PAT=$((PAT + 1)) ;;
        minor) MIN=$((MIN + 1)); PAT=0 ;;
        major) MAJ=$((MAJ + 1)); MIN=0; PAT=0 ;;
    esac
    VERSION="${MAJ}.${MIN}.${PAT}"
fi

# Basic semver-ish sanity (X.Y.Z[-prerelease]).
if ! printf '%s' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.+-]+)?$'; then
    fail "version '$VERSION' is not a valid X.Y.Z[-pre] string"; exit 1
fi
info "target release version: $VERSION"

# ----------------------------------------------------------------------------
# Preflight: clean tree + on main
# ----------------------------------------------------------------------------
info "preflight: working tree must be clean"
if ! git diff --quiet || ! git diff --cached --quiet; then
    fail "working tree is dirty; commit or stash first"; exit 1
fi
ok "tree clean"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$BRANCH" != "main" ]; then
    fail "must be on main (currently on '$BRANCH')"; exit 1
fi
ok "on main"

# Refuse to re-tag an existing version.
if git rev-parse -q --verify "refs/tags/v$VERSION" >/dev/null; then
    fail "tag v$VERSION already exists"; exit 1
fi
ok "tag v$VERSION is free"

# ----------------------------------------------------------------------------
# Pre-release gates (4)
# ----------------------------------------------------------------------------
GATES_RUN=0

info "gate 1/4: cargo check --workspace --all-targets --locked --offline"
cargo check --workspace --all-targets --locked --offline
GATES_RUN=$((GATES_RUN + 1)); ok "check passed"

info "gate 2/4: cargo clippy --workspace --all-targets --locked --offline -- -D warnings"
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
GATES_RUN=$((GATES_RUN + 1)); ok "clippy clean"

info "gate 3/4: cargo deny check"
cargo deny check
GATES_RUN=$((GATES_RUN + 1)); ok "supply-chain clean"

info "gate 4/4: cargo nextest run --workspace --locked --offline"
cargo nextest run --workspace --locked --offline
GATES_RUN=$((GATES_RUN + 1)); ok "tests green"

# ----------------------------------------------------------------------------
# Apply version bump (with backups)
# ----------------------------------------------------------------------------
info "bumping version in Cargo.toml + .release-please-manifest.json"
cp "$CARGO_TOML" "$CARGO_BAK"
cp "$MANIFEST"   "$MANIFEST_BAK"

# Only the FIRST `version = "..."` after [workspace.package] is rewritten.
# Strategy: find the [workspace.package] section and rewrite its `version` line.
awk -v new="$VERSION" '
    BEGIN { in_pkg = 0; done = 0 }
    /^\[workspace\.package\]/ { in_pkg = 1; print; next }
    /^\[/ && !/^\[workspace\.package\]/ { in_pkg = 0 }
    {
        if (in_pkg && !done && $0 ~ /^version[[:space:]]*=/) {
            sub(/"[^"]+"/, "\"" new "\"")
            done = 1
        }
        print
    }
' "$CARGO_BAK" > "$CARGO_TOML"

# .release-please-manifest.json — single key ".".
awk -v new="$VERSION" '
    {
        if ($0 ~ /"\.":[[:space:]]*"/) {
            sub(/"[^"]+"[[:space:]]*$/, "\"" new "\"")
        }
        print
    }
' "$MANIFEST_BAK" > "$MANIFEST"

# Verify the rewrites took effect.
NEW_CARGO="$(grep -E '^version[[:space:]]*=' "$CARGO_TOML" | head -1 | sed -E 's/.*=[[:space:]]*"([^"]+)".*/\1/')"
NEW_MAN="$(grep -E '"\.":' "$MANIFEST" | sed -E 's/.*"([^"]+)"[[:space:]]*$/\1/')"
if [ "$NEW_CARGO" != "$VERSION" ] || [ "$NEW_MAN" != "$VERSION" ]; then
    fail "rewrite verification failed (Cargo=$NEW_CARGO, Manifest=$NEW_MAN)"; exit 1
fi
ok "Cargo.toml -> $NEW_CARGO"
ok "manifest   -> $NEW_MAN"

# Cleanup backups now that rewrites are confirmed.
rm -f "$CARGO_BAK" "$MANIFEST_BAK"

# ----------------------------------------------------------------------------
# Commit + tag (NO publish, NO push by default)
# ----------------------------------------------------------------------------
info "committing version bump"
git add "$CARGO_TOML" "$MANIFEST"
git commit -m "chore(release): bump version to $VERSION"
COMMITS_GENERATED=1
ok "commit created"

info "tagging v$VERSION"
git tag -a "v$VERSION" -m "Release v$VERSION"
TAGS_GENERATED=1
ok "annotated tag v$VERSION created"

# ----------------------------------------------------------------------------
# Push (only if --push)
# ----------------------------------------------------------------------------
if [ "$PUSH" = "1" ]; then
    info "pushing main + tag v$VERSION to origin"
    git push origin main
    git push origin "v$VERSION"
    ok "pushed"
else
    cat <<EOF

Release v$VERSION ready locally.
To publish:
  git push origin main
  git push origin v$VERSION
The push will trigger:
  - .github/workflows/release.yml (binary builds)
  - .github/workflows/release-please.yml (release PR)

NOTE: \`cargo-publish\` is NOT run by this script — see docs/cargo/PUBLISH-LADDER.md.
EOF
fi

# ----------------------------------------------------------------------------
# Done — disarm restore trap (we want the bump to survive)
# ----------------------------------------------------------------------------
trap - EXIT
info "release v$VERSION prepared"
ok   "gates run:        4"
ok   "commits created:  ${COMMITS_GENERATED:-0}"
ok   "tags created:     ${TAGS_GENERATED:-0}"
ok   "pushed:           $( [ "$PUSH" = "1" ] && echo yes || echo no )"
