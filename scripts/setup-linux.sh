#!/usr/bin/env bash
# =============================================================================
#  aphrody — Ubuntu 26.04 setup (priority #1 target)
#  Installs all dependencies needed to build, test, and audit the `aphrody`
#  CLI binary natively on Linux x86_64.
#
#  Usage:
#    bash scripts/setup-linux.sh           # full install
#    bash scripts/setup-linux.sh --check   # verify without installing
#    bash scripts/setup-linux.sh --tools   # cargo tools only (skip apt)
# =============================================================================
set -euo pipefail

MODE="${1:-install}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

color() { printf '\033[%sm%s\033[0m\n' "$1" "$2"; }
ok()    { color '1;32' "  ✓ $1"; }
fail()  { color '1;31' "  ✗ $1"; }
info()  { color '1;36' "▶ $1"; }
warn()  { color '1;33' "  ⚠ $1"; }

# -----------------------------------------------------------------------------
# 1. System dependencies (apt) — Ubuntu 26.04 baseline
# -----------------------------------------------------------------------------
APT_PACKAGES=(
    build-essential
    pkg-config
    libssl-dev
    libudev-dev
    libgtk-3-dev        # only needed for crates/gui (wry+tao); skip if cli-only
    libwebkit2gtk-4.1-dev
    cmake
    ninja-build
    nasm
    lld
    clang
    curl
    git
    ca-certificates
    unzip
    xz-utils
    zstd
)

install_apt() {
    info "Installing apt packages (Ubuntu 26.04 baseline)"
    sudo apt-get update -qq
    sudo apt-get install -y --no-install-recommends "${APT_PACKAGES[@]}"
    ok "apt packages installed"
}

# -----------------------------------------------------------------------------
# 2. Bun (JS/TS runtime — node forbidden)
# -----------------------------------------------------------------------------
install_bun() {
    if command -v bun >/dev/null 2>&1; then
        ok "bun already present ($(bun --version))"
        return
    fi
    info "Installing Bun"
    curl -fsSL https://bun.sh/install | bash
    # shellcheck disable=SC1091
    export PATH="$HOME/.bun/bin:$PATH"
    ok "bun installed ($(bun --version))"
}

# -----------------------------------------------------------------------------
# 3. Rust toolchain (nightly, pinned via rust-toolchain.toml)
# -----------------------------------------------------------------------------
install_rust() {
    if ! command -v rustup >/dev/null 2>&1; then
        info "Installing rustup"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
            -y --no-modify-path --default-toolchain none
        # shellcheck disable=SC1091
        source "$HOME/.cargo/env"
    fi

    info "Installing nightly toolchain + components"
    rustup toolchain install nightly --component clippy --component rustfmt --component rust-src --component miri
    rustup default nightly
    ok "rust toolchain ready ($(rustc --version))"

    info "Adding cross-platform targets"
    rustup target add \
        x86_64-unknown-linux-gnu \
        x86_64-pc-windows-msvc \
        wasm32-wasi \
        wasm32-unknown-unknown
    ok "targets installed"
}

# -----------------------------------------------------------------------------
# 4. Cargo tooling (supply-chain + dev-loop)
# -----------------------------------------------------------------------------
CARGO_TOOLS=(
    sccache
    cargo-nextest
    cargo-deny
    cargo-vet
    cargo-machete
    cargo-auditable
    cargo-zigbuild
    wasm-bindgen-cli
    wasm-pack
)

install_cargo_tools() {
    info "Installing cargo tools"
    for tool in "${CARGO_TOOLS[@]}"; do
        if cargo install --list 2>/dev/null | grep -q "^${tool} "; then
            ok "$tool already installed"
        else
            cargo install --locked "$tool" || warn "failed: $tool (continuing)"
        fi
    done
    ok "cargo tools done"
}

# -----------------------------------------------------------------------------
# 5. Verification
# -----------------------------------------------------------------------------
verify() {
    info "Verification"
    local fail_count=0

    # System
    for cmd in cc pkg-config cmake ninja nasm lld clang git curl; do
        if command -v "$cmd" >/dev/null 2>&1; then ok "$cmd"
        else fail "$cmd missing"; fail_count=$((fail_count + 1)); fi
    done

    # Rust
    if command -v rustc >/dev/null 2>&1; then
        ok "rustc $(rustc --version)"
    else
        fail "rustc missing"; fail_count=$((fail_count + 1))
    fi

    # Bun
    if command -v bun >/dev/null 2>&1; then
        ok "bun $(bun --version)"
    else
        fail "bun missing"; fail_count=$((fail_count + 1))
    fi

    # Targets
    for tgt in x86_64-unknown-linux-gnu x86_64-pc-windows-msvc wasm32-wasi wasm32-unknown-unknown; do
        if rustup target list --installed 2>/dev/null | grep -q "^${tgt}$"; then
            ok "target: $tgt"
        else
            fail "target missing: $tgt"; fail_count=$((fail_count + 1))
        fi
    done

    # Cargo tools
    for tool in sccache cargo-nextest cargo-deny cargo-vet cargo-machete; do
        if command -v "$tool" >/dev/null 2>&1; then ok "$tool"
        else fail "$tool missing"; fail_count=$((fail_count + 1)); fi
    done

    if [[ $fail_count -gt 0 ]]; then
        fail "$fail_count component(s) missing"
        return 1
    fi
    ok "all components present"
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------
info "aphrody — Linux setup ($MODE)"
info "repo: $REPO_ROOT"

case "$MODE" in
    install)
        install_apt
        install_bun
        install_rust
        install_cargo_tools
        verify
        ;;
    --tools)
        install_cargo_tools
        verify
        ;;
    --check)
        verify
        ;;
    *)
        fail "unknown mode: $MODE"
        echo "Usage: $0 [install|--tools|--check]"
        exit 2
        ;;
esac

info "Done. Build with: cargo build --release -p aphrody --target x86_64-unknown-linux-gnu"
