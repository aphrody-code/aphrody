#!/usr/bin/env bash
# =============================================================================
#  aphrody — Linux x86_64 native build (priority #1)
#  Builds the `aphrody` CLI binary natively on Ubuntu 26.04.
#
#  Usage:
#    bash scripts/build-linux.sh                # release dist build
#    bash scripts/build-linux.sh --debug        # debug build
#    bash scripts/build-linux.sh --check        # cargo check only
#    bash scripts/build-linux.sh --test         # cargo nextest run -p cli
# =============================================================================
set -euo pipefail

MODE="${1:-release}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET="x86_64-unknown-linux-gnu"
BIN_NAME="aphrody"

color() { printf '\033[%sm%s\033[0m\n' "$1" "$2"; }
info()  { color '1;36' "▶ $1"; }
ok()    { color '1;32' "  ✓ $1"; }
fail()  { color '1;31' "  ✗ $1"; }

cd "$REPO_ROOT"

# Override the default Windows-MSVC target from .cargo/config.toml.
export CARGO_BUILD_TARGET="$TARGET"
# Use sccache if available (set in .cargo/config.toml as rustc-wrapper).
if ! command -v sccache >/dev/null 2>&1; then
    info "sccache not in PATH; disabling rustc-wrapper for this build"
    export RUSTC_WRAPPER=""
fi

case "$MODE" in
    release|--release|dist)
        info "cargo build --release -p cli --target $TARGET"
        cargo build --release -p cli --target "$TARGET" --locked
        out="target/$TARGET/release/$BIN_NAME"
        if [[ -f "$out" ]]; then
            ok "built: $out ($(du -h "$out" | cut -f1))"
        else
            fail "build artifact not found at $out"
            exit 1
        fi
        ;;
    --debug|debug)
        info "cargo build -p cli --target $TARGET"
        cargo build -p cli --target "$TARGET" --locked
        ok "built (debug): target/$TARGET/debug/$BIN_NAME"
        ;;
    --check|check)
        info "cargo check -p cli --target $TARGET"
        cargo check -p cli --target "$TARGET" --locked
        ok "check passed"
        ;;
    --test|test)
        info "cargo nextest run -p cli --target $TARGET"
        cargo nextest run -p cli --target "$TARGET" --locked
        ok "tests passed"
        ;;
    --workspace)
        info "cargo build --workspace --target $TARGET (release)"
        cargo build --workspace --release --target "$TARGET" --locked
        ok "workspace build complete"
        ;;
    *)
        fail "unknown mode: $MODE"
        echo "Usage: $0 [release|--debug|--check|--test|--workspace]"
        exit 2
        ;;
esac
