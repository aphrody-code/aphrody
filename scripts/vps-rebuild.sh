#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Rebuild and redeploy aphrody stack natively on the VPS.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "==> 1. Fetching latest changes from git"
git pull origin main

# Load Cargo environment
# shellcheck disable=SC1091
source "$HOME/.cargo/env" || true
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

echo "==> 2. Rebuilding Rust CLI binary natively"
export RUSTC_WRAPPER=""
export RUSTFLAGS="-C target-cpu=native"
cargo build --release -p aphrody --target x86_64-unknown-linux-gnu

echo "==> 3. Installing Rust binaries to local path"
mkdir -p "$HOME/.local/bin"
cp target/x86_64-unknown-linux-gnu/release/aphrody "$HOME/.local/bin/"

echo "==> 4. Setting up Python virtual environment and dependencies"
cd "$REPO_ROOT/py"
uv sync --all-extras
# Build python package wheel
uv build --package aphrody --wheel
WHEEL_PATH=$(find dist/ -name "aphrody-*.whl" | head -n 1)

echo "==> 5. Setting up TS/JS workspaces with Bun"
cd "$REPO_ROOT"
# Ensure bun in PATH
export PATH="$HOME/.bun/bin:$PATH"
bun install

echo "==> 6. Deploying systemd services with dynamic memory configurations"
sudo "$REPO_ROOT/py/aphrody/deploy/deploy-vps.sh" --mode react --wheel "$WHEEL_PATH" --site "$REPO_ROOT/apps/desktop/src" --host 0.0.0.0 --port 8080

echo "==> Redeployment completed successfully!"
