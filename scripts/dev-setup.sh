#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# =============================================================================
#  aphrody — one-script dev-environment bootstrap (POSIX / Linux / macOS)
#
#  Mission: lower contribution barrier to
#    git clone <repo> && ./scripts/dev-setup.sh && cargo check
#
#  This script is the "native" complement to .devcontainer/devcontainer.json —
#  it installs the SAME toolchain (rust nightly + wasm + bun + cargo extras)
#  so a non-Codespace contributor can match Codespace parity in one shot.
#
#  For deeper Ubuntu provisioning (apt packages, libssl-dev, libgtk-3-dev),
#  use scripts/setup-linux.sh instead. This script is intentionally minimal
#  and assumes the OS already has gcc/clang, git, curl, pkg-config.
#
#  Properties:
#    - idempotent (safe to re-run after a partial setup)
#    - no sudo required (uses rustup-init + ~/.cargo, ~/.bun)
#    - no whole-workspace build (only `cargo check -p aphrody`)
#
#  Usage:
#    ./scripts/dev-setup.sh
# =============================================================================
set -euo pipefail

# ---------- pretty printers ---------------------------------------------------
color() { printf '\033[%sm%s\033[0m\n' "$1" "$2"; }
info()  { color '1;36' "▶ $*"; }
ok()    { color '1;32' "  ✓ $*"; }
warn()  { color '1;33' "  ⚠ $*"; }
fail()  { color '1;31' "  ✗ $*"; }

# ---------- locate repo root --------------------------------------------------
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# ---------- detect host platform ---------------------------------------------
OS="$(uname -s)"
ARCH="$(uname -m)"
info "aphrody dev-setup — host: $OS / $ARCH"
info "repo:  $REPO_ROOT"

case "$OS" in
    Linux|Darwin) ;;
    *) warn "unsupported OS '$OS' — script targets Linux/macOS. For Windows, use scripts/dev-setup.cmd." ;;
esac

# ---------- 1. bun (JS/TS runtime — node forbidden by policy) ----------------
info "step 1/6  bun"
if command -v bun >/dev/null 2>&1; then
    ok "bun present ($(bun --version))"
else
    warn "bun missing. Install with:"
    echo "    curl -fsSL https://bun.sh/install | bash"
    echo "  then re-run this script. Bun is mandatory (project policy: node forbidden)."
    exit 1
fi

# ---------- 2. rustup (toolchain manager) ------------------------------------
info "step 2/6  rustup"
if command -v rustup >/dev/null 2>&1; then
    ok "rustup present ($(rustup --version 2>/dev/null | head -1))"
else
    warn "rustup missing. Install with:"
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "  then 'source \"\$HOME/.cargo/env\"' and re-run this script."
    exit 1
fi

# ---------- 3. nightly toolchain + components --------------------------------
# rust-toolchain.toml pins the exact channel; rustup auto-syncs on next cargo
# invocation. We still explicitly install components for `rustup show` parity.
info "step 3/6  rust nightly + components"
rustup install nightly
rustup default nightly
rustup component add rustfmt clippy rust-src rust-analyzer
ok "nightly toolchain + components ready"

# ---------- 4. cross-compile targets (priorities 1, 2, 3) --------------------
info "step 4/6  cross-compile targets"
rustup target add \
    wasm32-unknown-unknown \
    wasm32-wasip1 \
    x86_64-unknown-linux-gnu
ok "targets: wasm32-unknown-unknown, wasm32-wasip1, x86_64-unknown-linux-gnu"

# ---------- 5. cargo extras (test runner + supply-chain + wasm) --------------
# Match .devcontainer/devcontainer.json postCreateCommand exactly.
info "step 5/6  cargo extras (this may take a few minutes on first run)"
CARGO_TOOLS=(
    cargo-nextest
    cargo-deny
    cargo-vet
    cargo-machete
    cargo-zigbuild
    cargo-auditable
    wasm-pack
)
for tool in "${CARGO_TOOLS[@]}"; do
    if cargo install --list 2>/dev/null | grep -q "^${tool} "; then
        ok "$tool already installed"
    else
        info "installing $tool"
        cargo install --locked "$tool" || warn "failed: $tool (continuing — re-run later)"
    fi
done
ok "cargo extras done"

# ---------- 6. bun workspace install -----------------------------------------
info "step 6/6  bun install (workspace deps)"
if [[ -f "$REPO_ROOT/bun.lock" ]]; then
    bun install --frozen-lockfile
    ok "bun workspaces synced"
else
    warn "no bun.lock — running unlocked bun install"
    bun install
fi

# ---------- verification ------------------------------------------------------
info "verifying: cargo check -p aphrody --locked --offline"
if cargo check -p aphrody --locked --offline 2>&1 | tail -3; then
    ok "verification passed"
else
    warn "verification check returned non-zero — see output above"
fi

# ---------- success -----------------------------------------------------------
info "Dev environment ready. Try: cargo build --release -p aphrody"
