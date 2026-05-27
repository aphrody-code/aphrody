#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# VPS dependency provisioning and environment setup script (Ubuntu 26.04).
# Installs system libraries, Rust nightly, Bun, and UV in one go.
set -euo pipefail

echo "==> 0. Applying OS & Kernel Performance Tuning"
# Apply sysctl settings for high network/connection concurrency
sudo mkdir -p /etc/sysctl.d
cat <<EOF | sudo tee /etc/sysctl.d/99-aphrody.conf >/dev/null
# Socket backlog limits
net.core.somaxconn = 32768
net.ipv4.tcp_max_syn_backlog = 16384

# Connection recycling and reuse
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 15

# Device queue backlog
net.core.netdev_max_backlog = 16384

# TCP window scale & buffer sizes
net.ipv4.tcp_window_scaling = 1
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216

# Virtual Memory
vm.swappiness = 10
vm.dirty_background_ratio = 5
vm.dirty_ratio = 10
EOF

sudo sysctl --system || true

# Configure limits for file descriptors
sudo mkdir -p /etc/security/limits.d
cat <<EOF | sudo tee /etc/security/limits.d/99-aphrody.conf >/dev/null
aphrody    soft    nofile    65536
aphrody    hard    nofile    1048576
EOF

# Enable Transparent Hugepages (THP)
if [[ -f /sys/kernel/mm/transparent_hugepage/enabled ]]; then
  echo "always" | sudo tee /sys/kernel/mm/transparent_hugepage/enabled >/dev/null || true
fi
if [[ -f /sys/kernel/mm/transparent_hugepage/defrag ]]; then
  echo "always" | sudo tee /sys/kernel/mm/transparent_hugepage/defrag >/dev/null || true
fi

# Try to make THP persistent in /etc/default/grub if grub config and update-grub exist
if [[ -f /etc/default/grub ]] && command -v update-grub &>/dev/null; then
  if ! grep -q "transparent_hugepage=always" /etc/default/grub; then
    sudo sed -i 's/\(GRUB_CMDLINE_LINUX_DEFAULT="[^"]*\)"/\1 transparent_hugepage=always"/' /etc/default/grub || true
    sudo update-grub || true
  fi
fi

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
  curl \
  libjemalloc-dev \
  libjemalloc2

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
