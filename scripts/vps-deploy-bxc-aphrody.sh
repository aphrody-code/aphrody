#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# One-shot VPS deploy: bxc MCP/CLI + aphrody binaries + agent-stack sync + yoyo hub.
set -euo pipefail

BXC_ROOT="${BXC_ROOT:-/home/ubuntu/bxc}"
APHRODY_ROOT="${APHRODY_ROOT:-/home/ubuntu/aphrody}"
YOyo_ROOT="${YOyo_ROOT:-/home/ubuntu/yoyo}"
LOCAL_BIN="${HOME}/.local/bin"

# shellcheck disable=SC1091
source "$HOME/.cargo/env" 2>/dev/null || true
export PATH="$LOCAL_BIN:$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"

echo "==> [1/7] bxc: install + MCP"
cd "$BXC_ROOT"
bun install
bun run build:mcp
ln -sf "$BXC_ROOT/bin/bxc" "$LOCAL_BIN/bxc"
install -m 755 "$BXC_ROOT/dist/standalone/bxc-mcp" "$LOCAL_BIN/bxc-mcp"

echo "==> [2/7] bxc: Rust x-cli (optional)"
if command -v cargo &>/dev/null; then
  (cd "$BXC_ROOT/rust-bridge" && RUSTC_WRAPPER= cargo build --release -p x-cli 2>/dev/null) || true
  [[ -f "$BXC_ROOT/rust-bridge/target/release/x-cli" ]] && \
    install -m 755 "$BXC_ROOT/rust-bridge/target/release/x-cli" "$LOCAL_BIN/x-cli"
fi

echo "==> [3/7] aphrody: Rust toolchain + build"
cd "$APHRODY_ROOT"
if command -v rustup &>/dev/null; then
  rustup toolchain install nightly-2026-05-17 2>/dev/null || true
  rustup override set nightly-2026-05-17 2>/dev/null || true
fi
export RUSTC_WRAPPER=
export CARGO_CONFIG="${CARGO_CONFIG:-$APHRODY_ROOT/.cargo/config.linux-vps.toml}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$APHRODY_ROOT/target/x86_64-unknown-linux-gnu}"
if command -v cargo &>/dev/null; then
  cargo build --release --target x86_64-unknown-linux-gnu -p google_mcp -p aphrody 2>/dev/null || \
    echo "    warn: aphrody cargo build skipped (check rustc >= 1.97)"
  release_dir="$CARGO_TARGET_DIR/release"
  [[ -d "$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release" ]] && \
    release_dir="$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release"
  for bin in aphrody aphrody-mcp google_mcp; do
    src="$release_dir/$bin"
    [[ -f "$src" ]] || continue
    dest="$LOCAL_BIN/$bin"
    [[ "$bin" == "google_mcp" ]] && dest="$LOCAL_BIN/aphrody-mcp"
    install -m 755 "$src" "$dest"
    echo "    installed $dest"
  done
fi

echo "==> [4/7] Agent stack sync"
bash "$APHRODY_ROOT/scripts/vps-sync-agent-stack.sh"

echo "==> [5/7] llms.txt + help snapshots"
bash "$APHRODY_ROOT/scripts/fetch-ai-llms.sh" 2>/dev/null || true

echo "==> [6/7] yoyo hub (if present)"
if [[ -d "$YOyo_ROOT" ]] && command -v bun &>/dev/null; then
  (cd "$YOyo_ROOT" && bun install && bun run sync 2>/dev/null && bun run export 2>/dev/null) || true
  if ! ss -tlnp 2>/dev/null | grep -q ':8790'; then
    nohup bun "$YOyo_ROOT/apps/hub/src/main.ts" >>"$YOyo_ROOT/var/log/hub.log" 2>&1 &
    sleep 1
  fi
fi

echo "==> [7/7] Health"
bxc --version
command -v bxc-mcp && echo "bxc-mcp: ok"
command -v aphrody-mcp && echo "aphrody-mcp: ok"
aphrody doctor 2>&1 | head -12 || true
curl -sS -o /dev/null -w "yoyo hub: %{http_code}\n" http://127.0.0.1:8790/health 2>/dev/null || true

echo "Done."