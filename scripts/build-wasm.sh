#!/usr/bin/env bash
# =============================================================================
#  aphrody — WebAssembly build (priority #3)
#  Produces:
#    - target/wasm32-wasi/release/aphrody.wasm           (WASI CLI bundle)
#    - target/wasm32-unknown-unknown/release/aphrody.wasm (web lib bundle)
#    - dist/wasm/                                         (wasm-bindgen output for web)
#
#  Usage:
#    bash scripts/build-wasm.sh             # both targets
#    bash scripts/build-wasm.sh wasi        # wasm32-wasi only (CLI)
#    bash scripts/build-wasm.sh web         # wasm32-unknown-unknown + wasm-bindgen
#    bash scripts/build-wasm.sh --check     # cargo check only (no build)
# =============================================================================
set -euo pipefail

MODE="${1:-all}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN_NAME="aphrody"
DIST_DIR="$REPO_ROOT/dist/wasm"

color() { printf '\033[%sm%s\033[0m\n' "$1" "$2"; }
info()  { color '1;36' "▶ $1"; }
ok()    { color '1;32' "  ✓ $1"; }
fail()  { color '1;31' "  ✗ $1"; }
warn()  { color '1;33' "  ⚠ $1"; }

cd "$REPO_ROOT"

ensure_target() {
    local tgt="$1"
    if ! rustup target list --installed 2>/dev/null | grep -q "^${tgt}$"; then
        info "installing target: $tgt"
        rustup target add "$tgt"
    fi
    ok "target ready: $tgt"
}

build_wasi() {
    local tgt="wasm32-wasi"
    ensure_target "$tgt"
    info "cargo build --release -p cli --target $tgt"
    CARGO_BUILD_TARGET="$tgt" cargo build --release -p cli --target "$tgt" --locked
    local out="target/$tgt/release/${BIN_NAME}.wasm"
    if [[ -f "$out" ]]; then
        ok "WASI bundle: $out ($(du -h "$out" | cut -f1))"
    else
        fail "WASI artifact missing at $out"
        return 1
    fi
}

build_web() {
    local tgt="wasm32-unknown-unknown"
    ensure_target "$tgt"
    info "cargo build --release -p cli --target $tgt"
    CARGO_BUILD_TARGET="$tgt" cargo build --release -p cli --target "$tgt" --locked
    local raw="target/$tgt/release/${BIN_NAME}.wasm"
    if [[ ! -f "$raw" ]]; then
        fail "web wasm artifact missing at $raw"
        return 1
    fi
    ok "raw web wasm: $raw ($(du -h "$raw" | cut -f1))"

    # Run wasm-bindgen if available (generates JS glue + d.ts).
    if command -v wasm-bindgen >/dev/null 2>&1; then
        mkdir -p "$DIST_DIR"
        info "wasm-bindgen → $DIST_DIR"
        wasm-bindgen --target web --out-dir "$DIST_DIR" --out-name "$BIN_NAME" "$raw"
        ok "bundle: $DIST_DIR"
    else
        warn "wasm-bindgen not installed — skipping web bundle generation"
        warn "install with: cargo install wasm-bindgen-cli"
    fi
}

check_only() {
    for tgt in wasm32-wasi wasm32-unknown-unknown; do
        ensure_target "$tgt"
        info "cargo check -p cli --target $tgt"
        CARGO_BUILD_TARGET="$tgt" cargo check -p cli --target "$tgt" --locked
        ok "check passed: $tgt"
    done
}

case "$MODE" in
    all)
        build_wasi
        build_web
        ;;
    wasi|--wasi)
        build_wasi
        ;;
    web|--web)
        build_web
        ;;
    --check|check)
        check_only
        ;;
    *)
        fail "unknown mode: $MODE"
        echo "Usage: $0 [all|wasi|web|--check]"
        exit 2
        ;;
esac
