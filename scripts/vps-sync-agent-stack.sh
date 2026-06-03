#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Sync bxc + aphrody MCP, binaries, and agent configs on this VPS.
set -euo pipefail

BXC_ROOT="${BXC_ROOT:-/home/ubuntu/bxc}"
APHRODY_ROOT="${APHRODY_ROOT:-/home/ubuntu/aphrody}"
LOCAL_BIN="${HOME}/.local/bin"
MCP_CONFIG="${HOME}/.config/aphrody/mcp.json"

echo "==> [1/6] Build bxc MCP + link CLI"
cd "$BXC_ROOT"
bun install --frozen-lockfile 2>/dev/null || bun install
bun run build:mcp 2>/dev/null || bun build src/mcp/server.ts --outfile dist/standalone/bxc-mcp --target=bun --compile 2>/dev/null || true
if [[ -f dist/standalone/bxc-mcp ]]; then
  install -m 755 dist/standalone/bxc-mcp "$LOCAL_BIN/bxc-mcp"
fi
if [[ -f bin/bxc ]] && [[ "$(readlink -f bin/bxc 2>/dev/null || echo bin/bxc)" != "$(readlink -f "$LOCAL_BIN/bxc" 2>/dev/null || echo MISSING)" ]]; then
  ln -sf "$BXC_ROOT/bin/bxc" "$LOCAL_BIN/bxc" 2>/dev/null || install -m 755 bin/bxc "$LOCAL_BIN/bxc"
fi

echo "==> [2/6] Build Rust x-cli (optional)"
if command -v cargo &>/dev/null; then
  (cd "$BXC_ROOT/rust-bridge/crates/x-client" && cargo build --release 2>/dev/null) || true
  [[ -f "$BXC_ROOT/rust-bridge/target/release/x-cli" ]] && \
    install -m 755 "$BXC_ROOT/rust-bridge/target/release/x-cli" "$LOCAL_BIN/x-cli"
fi

echo "==> [3/6] aphrody MCP binary"
if [[ -x "$APHRODY_ROOT/target/release/aphrody-mcp" ]]; then
  install -m 755 "$APHRODY_ROOT/target/release/aphrody-mcp" "$LOCAL_BIN/aphrody-mcp"
elif command -v aphrody-mcp &>/dev/null; then
  echo "    aphrody-mcp already in PATH"
else
  echo "    skip: build aphrody-mcp with: cd $APHRODY_ROOT && cargo build -p aphrody-mcp --release"
fi

echo "==> [4/6] Ensure MCP config"
mkdir -p "$(dirname "$MCP_CONFIG")"
if [[ ! -f "$MCP_CONFIG" ]]; then
  cat >"$MCP_CONFIG" <<'EOF'
{
  "mcpServers": {
    "aphrody": {
      "command": "/home/ubuntu/.local/bin/aphrody-mcp",
      "args": [],
      "env": {
        "BXC_DB_PATH": "/home/ubuntu/bxc/data/bxc.sqlite",
        "BXC_MEMORY_DB": "/home/ubuntu/bxc/bxc-memory.sqlite",
        "REDIS_URL": "redis://127.0.0.1:6379"
      }
    },
    "bxc": {
      "command": "/home/ubuntu/.local/bin/bxc-mcp",
      "args": [],
      "env": {
        "BXC_DB_PATH": "/home/ubuntu/bxc/data/bxc.sqlite",
        "BXC_MEMORY_DB": "/home/ubuntu/bxc/bxc-memory.sqlite",
        "REDIS_URL": "redis://127.0.0.1:6379"
      }
    }
  }
}
EOF
fi

echo "==> [5/6] Grok MCP compat (config.toml)"
GROK_CFG="${HOME}/.grok/config.toml"
if [[ -f "$GROK_CFG" ]] && ! grep -q 'mcp_servers.bxc' "$GROK_CFG" 2>/dev/null; then
  cat >>"$GROK_CFG" <<'EOF'

[mcp_servers.bxc]
command = "/home/ubuntu/.local/bin/bxc-mcp"
startup_timeout_sec = 15
tool_timeout_sec = 120
EOF
  echo "    appended [mcp_servers.bxc] to $GROK_CFG"
fi

echo "==> [6/6] Refresh agent-stack docs"
if [[ -x "$APHRODY_ROOT/scripts/vps-sync-agent-stack.sh" ]]; then
  for cmd in agy grok claude bxc aphrody; do
    if command -v "$cmd" &>/dev/null; then
      "$cmd" --help >"$APHRODY_ROOT/docs/agent-stack/${cmd}-help.txt" 2>&1 || true
    fi
  done
fi

echo "==> Health checks"
command -v bxc && bxc --version 2>/dev/null || true
command -v bxc-mcp && echo "bxc-mcp: ok" || echo "bxc-mcp: MISSING"
command -v aphrody-mcp && echo "aphrody-mcp: ok" || echo "aphrody-mcp: MISSING"
test -f "$MCP_CONFIG" && echo "mcp.json: ok"

if systemctl is-active bxc.service &>/dev/null; then
  echo "==> Restarting bxc.service"
  sudo systemctl restart bxc.service || true
fi

echo "Done. Agents share: $MCP_CONFIG"