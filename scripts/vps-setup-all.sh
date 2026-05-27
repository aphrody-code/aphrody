#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# VPS dependency provisioning and environment setup script (Ubuntu 26.04).
# Installs system libraries, Rust nightly, Bun, and UV in one go.
set -euo pipefail

echo "==> 1. Updating APT and installing system libraries"
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  protobuf-compiler \
  libportaudio2 \
  lld-21 \
  git \
  curl

echo "==> 2. Setting up lld linker symlinks"
sudo ln -sf /usr/bin/lld-21 /usr/bin/lld
sudo ln -sf /usr/bin/ld.lld-21 /usr/bin/ld.lld

echo "==> 3. Installing Rustup (nightly toolchain)"
if ! command -v rustup &>/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
else
  echo "Rustup already installed."
fi
# Load cargo environment
# shellcheck disable=SC1090
source "$HOME/.cargo/env"
rustup target add x86_64-unknown-linux-gnu wasm32-unknown-unknown wasm32-wasip1

echo "==> 4. Installing Bun (JavaScript runtime)"
if ! command -v bun &>/dev/null; then
  curl -fsSL https://bun.sh/install | bash
else
  echo "Bun already installed."
fi

echo "==> 5. Installing uv (Python package manager)"
if ! command -v uv &>/dev/null; then
  curl -LsSf https://astral.sh/uv/install.sh | sh
else
  echo "uv already installed."
fi

echo "==> VPS environment bootstrap complete!"
