#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Smoke-test xAI API, Grok CLI, and bxc on the VPS. Never prints full API keys.
set -euo pipefail

PASS=0
FAIL=0
note() { printf '%s\n' "$*"; }
ok() { note "  OK  $1"; PASS=$((PASS + 1)); }
bad() { note "  FAIL $1"; FAIL=$((FAIL + 1)); }

[ -f "$HOME/.bashrc" ] && . "$HOME/.bashrc"
[ -f "$HOME/.bash_secrets" ] && . "$HOME/.bash_secrets"

note "=== xAI / Grok + bxc smoke ($(date -Iseconds)) ==="

BXC_GROK="${BXC_BIN:-$HOME/bxc/bin/bxc}"
GROK_AUTH="$HOME/.grok/auth.json"

if [ -n "${XAI_API_KEY:-}" ]; then
  ok "XAI_API_KEY set (${#XAI_API_KEY} chars)"
elif [ -f "$GROK_AUTH" ]; then
  ok "Grok OIDC session (~/.grok/auth.json)"
else
  bad "No XAI_API_KEY and no ~/.grok/auth.json"
fi

if command -v "$BXC_GROK" >/dev/null 2>&1 || [ -x "$BXC_GROK" ]; then
  if unset XAI_API_KEY; "$BXC_GROK" grok whoami >/dev/null 2>&1; then
    ok "bxc grok whoami (keyless)"
  else
    bad "bxc grok whoami"
  fi
else
  bad "bxc grok binary missing ($BXC_GROK)"
fi

if [ -n "${XAI_API_KEY:-}" ] || [ -f "$GROK_AUTH" ]; then
  AUTH_HDR=""
  if [ -n "${XAI_API_KEY:-}" ]; then
    AUTH_HDR="Authorization: Bearer ${XAI_API_KEY}"
  else
    AUTH_HDR=$(python3 -c "import json,os; d=json.load(open(os.path.expanduser('~/.grok/auth.json'))); print('Authorization: Bearer '+next(iter(d.values()))['key'])" 2>/dev/null || true)
  fi
  if [ -n "$AUTH_HDR" ] && models_json=$(curl -sfS --max-time 30 \
    -H "$AUTH_HDR" \
    "https://api.x.ai/v1/models" 2>/dev/null); then
    count=$(printf '%s' "$models_json" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('data',d.get('models',[]))))" 2>/dev/null || echo "?")
    ok "GET /v1/models ($count models)"
  else
    bad "GET /v1/models"
  fi

  reply=$(curl -sfS --max-time 60 \
    -H "$AUTH_HDR" \
    -H "Content-Type: application/json" \
    -d '{"model":"grok-3-mini","messages":[{"role":"user","content":"Reply with exactly: OK"}],"max_tokens":16}' \
    "https://api.x.ai/v1/chat/completions" 2>/dev/null \
    | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['choices'][0]['message']['content'])" 2>/dev/null || true)
  if [ -n "$reply" ]; then
    ok "POST /v1/chat/completions grok-3-mini → ${reply//[$'\n\r']/}"
  else
    bad "POST /v1/chat/completions grok-3-mini"
  fi

  for model in grok-3 grok-4; do
    if curl -sfS --max-time 45 \
      -H "$AUTH_HDR" \
      -H "Content-Type: application/json" \
      -d "{\"model\":\"${model}\",\"messages\":[{\"role\":\"user\",\"content\":\"Say hi\"}],\"max_tokens\":8}" \
      "https://api.x.ai/v1/chat/completions" >/dev/null 2>&1; then
      ok "POST /v1/chat/completions ${model}"
    else
      bad "POST /v1/chat/completions ${model}"
    fi
  done
fi

if command -v grok >/dev/null 2>&1; then
  if grok inspect >/dev/null 2>&1; then
    ok "grok inspect"
  else
    bad "grok inspect"
  fi
else
  bad "grok not in PATH"
fi

if [ -x "${HOME}/.local/bin/bxc-mcp" ]; then
  ok "bxc-mcp binary"
else
  bad "bxc-mcp missing"
fi

if command -v bxc >/dev/null 2>&1; then
  ver=$(bxc --version 2>/dev/null | head -1 || true)
  ok "bxc CLI ${ver:-present}"
else
  bad "bxc not in PATH"
fi

if systemctl is-active --quiet bxc.service 2>/dev/null; then
  ok "bxc.service active (CDP :9222)"
elif curl -sfS --max-time 3 "http://127.0.0.1:9222/json/version" 2>/dev/null | grep -q '"Browser"'; then
  ok "bxc CDP :9222"
else
  bad "bxc.service / CDP :9222 (start: sudo systemctl start bxc.service)"
fi

if command -v aphrody >/dev/null 2>&1; then
  mcp_cfg="${APHRODY_MCP_CONFIG:-$HOME/.config/aphrody/mcp.json}"
  if [ -f "$mcp_cfg" ] && grep -q '"bxc"' "$mcp_cfg" 2>/dev/null; then
    ok "aphrody mcp.json includes bxc"
  else
    bad "aphrody mcp.json missing bxc ($mcp_cfg)"
  fi
else
  bad "aphrody not in PATH"
fi

note ""
note "Result: ${PASS} passed, ${FAIL} failed"
[ "$FAIL" -eq 0 ]