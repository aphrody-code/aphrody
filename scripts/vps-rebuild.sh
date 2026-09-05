#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Rebuild and redeploy aphrody stack natively on the VPS.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Load cargo and local binary paths
# shellcheck disable=SC1091
source "$HOME/.cargo/env" || true
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"

# Verify required commands are available
for cmd in rustc bun uv; do
  if ! command -v "$cmd" &>/dev/null; then
    echo "ERROR: Required command '$cmd' not found. Please run scripts/vps-setup-all.sh first." >&2
    exit 1
  fi
done

echo "==> 1. Fetching latest changes from git"
git pull origin main --rebase || echo "Warning: git pull failed, using current local state."

echo "==> 2. Rebuilding Rust CLI binary natively with target-cpu=native and advanced features"
export RUSTC_WRAPPER=""
export RUSTFLAGS="-C target-cpu=native"
cargo build --profile release-fast -p aphrody --target x86_64-unknown-linux-gnu --features "yara forensics index firefly"
cargo build --profile release-fast -p google_mcp --target x86_64-unknown-linux-gnu

echo "==> 3. Installing Rust binaries to local path"
mkdir -p "$HOME/.local/bin"
# install(1) gère le remplacement même si le binaire tourne (unlink+create).
install -m 0755 target/x86_64-unknown-linux-gnu/release-fast/aphrody "$HOME/.local/bin/aphrody"
install -m 0755 target/x86_64-unknown-linux-gnu/release-fast/aphrody-mcp "$HOME/.local/bin/aphrody-mcp"

echo "==> 4. Setting up Python virtual environment and dependencies"
cd "$REPO_ROOT/py"
uv sync --all-extras
# Build python package wheel (force native Linux target)
export CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu
uv build --package aphrody --wheel
WHEEL_PATH="$REPO_ROOT/py/$(find dist/ -name "aphrody-*.whl" | head -n 1)"

echo "==> 5. Setting up TS/JS workspaces with Bun"
cd "$REPO_ROOT"
bun install

MODE="${1:-rust}"
echo "==> 5.5 Building the full-Rust aphrody.com origin"
cargo build --profile release-fast -p aphrody-site --target x86_64-unknown-linux-gnu
install -m 0755 target/x86_64-unknown-linux-gnu/release-fast/aphrody-site "$HOME/.local/bin/aphrody-site"

echo "==> 6. Deploying systemd services with dynamic memory configurations"
sudo "$REPO_ROOT/py/aphrody/deploy/deploy-vps.sh" --mode rust --wheel "$WHEEL_PATH" --host 0.0.0.0 --port 8082

echo "==> Redeployment completed successfully!"
