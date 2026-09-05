#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Smoke-test xAI API, Grok CLI, and bxc on the VPS. Never prints API keys.
set -euo pipefail

PASS=0
FAIL=0
note() { printf '%s\n' "$*"; }
ok() { note "  OK  $1"; PASS=$((PASS + 1)); }
bad() { note "  FAIL $1"; FAIL=$((FAIL + 1)); }

secret_file_is_private() {
	local path=$1 owner mode
	[ -f "$path" ] && [ ! -L "$path" ] || return 1
	read -r owner mode < <(stat -c '%u %a' -- "$path" 2>/dev/null) || return 1
	[ "$owner" = "$(id -u)" ] && [ "$mode" = "600" ]
}

# Do not source .bashrc: its non-interactive guard may legitimately return a
# non-zero status, which is fatal under errexit. Load only the paths and secret
# file required by this non-interactive runner.
APHRODY_SMOKE_HOME="${APHRODY_SMOKE_HOME:-$HOME}"
export PATH="$APHRODY_SMOKE_HOME/.local/bin:$APHRODY_SMOKE_HOME/.cargo/bin:$APHRODY_SMOKE_HOME/.bun/bin:$APHRODY_SMOKE_HOME/.grok/bin:$PATH"
BASH_SECRETS="$APHRODY_SMOKE_HOME/.bash_secrets"
if [ -e "$BASH_SECRETS" ] && ! secret_file_is_private "$BASH_SECRETS"; then
	bad "refusing ~/.bash_secrets: require a regular, non-symlink file owned by the current user with mode 600"
elif [ -f "$BASH_SECRETS" ]; then
	# Secret files sometimes reference optional variables. Keep nounset strict
	# for the runner itself without imposing it or errexit on this
	# operator-owned file.
	set +e
	set +u
	# shellcheck source=/dev/null
	. "$BASH_SECRETS"
	set -u
	set -e
fi

note "=== xAI / Grok + bxc smoke ($(date -Iseconds)) ==="

BXC_GROK="${BXC_BIN:-$APHRODY_SMOKE_HOME/bxc/bin/bxc}"
GROK_AUTH="$APHRODY_SMOKE_HOME/.grok/auth.json"
GROK_AUTH_USABLE=0
if [ -e "$GROK_AUTH" ]; then
	if secret_file_is_private "$GROK_AUTH"; then
		GROK_AUTH_USABLE=1
	else
		bad "refusing ~/.grok/auth.json: require a regular, non-symlink file owned by the current user with mode 600"
	fi
fi

if [ -n "${XAI_API_KEY:-}" ]; then
  ok "XAI_API_KEY set"
elif [ "$GROK_AUTH_USABLE" = "1" ]; then
  ok "Grok OIDC session (~/.grok/auth.json)"
else
  bad "No XAI_API_KEY and no ~/.grok/auth.json"
fi

if command -v "$BXC_GROK" >/dev/null 2>&1 || [ -x "$BXC_GROK" ]; then
  if env -u XAI_API_KEY "$BXC_GROK" grok whoami >/dev/null 2>&1; then
    ok "bxc grok whoami (keyless)"
  else
    bad "bxc grok whoami"
  fi
else
  bad "bxc grok binary missing ($BXC_GROK)"
fi

if [ -n "${XAI_API_KEY:-}" ] || [ "$GROK_AUTH_USABLE" = "1" ]; then
	XAI_AUTH_TOKEN=""
	if [ -n "${XAI_API_KEY:-}" ]; then
		XAI_AUTH_TOKEN="$XAI_API_KEY"
	else
		XAI_AUTH_TOKEN=$(python3 -c "import json,sys; d=json.load(open(sys.argv[1])); print(next(iter(d.values()))['key'])" "$GROK_AUTH" 2>/dev/null || true)
	fi

	# Passing a Bearer header through `curl -H` exposes it in the process list.
	# A mode-600 config keeps argv secret-free and is removed on every exit path.
	CURL_AUTH_CONFIG=""
	cleanup_auth_config() {
		if [ -n "$CURL_AUTH_CONFIG" ] && [ -f "$CURL_AUTH_CONFIG" ]; then
			rm -f -- "$CURL_AUTH_CONFIG"
		fi
	}
	trap cleanup_auth_config EXIT
	trap 'exit 129' HUP
	trap 'exit 130' INT
	trap 'exit 143' TERM
	if [ -n "$XAI_AUTH_TOKEN" ]; then
		old_umask=$(umask)
		umask 077
		CURL_AUTH_CONFIG=$(mktemp "${TMPDIR:-/tmp}/aphrody-xai-curl.XXXXXX")
		printf 'header = "Authorization: Bearer %s"\n' "$XAI_AUTH_TOKEN" >"$CURL_AUTH_CONFIG"
		umask "$old_umask"
	fi
	unset XAI_API_KEY XAI_AUTH_TOKEN

	# GET /models is metadata-only. It does not generate tokens or model output.
	if [ -n "$CURL_AUTH_CONFIG" ] && models_json=$(curl -sfS --max-time 30 \
		--config "$CURL_AUTH_CONFIG" \
		"https://api.x.ai/v1/models" 2>/dev/null); then
		count=$(printf '%s' "$models_json" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('data',d.get('models',[]))))" 2>/dev/null || echo "?")
		ok "GET /v1/models metadata-only ($count models)"
	else
		bad "GET /v1/models"
	fi

	if [ "${RUN_PAID_XAI_SMOKE:-0}" = "1" ]; then
		XAI_SMOKE_MODEL=${XAI_SMOKE_MODEL:-grok-3-mini}
		case "$XAI_SMOKE_MODEL" in
			*[!A-Za-z0-9._-]*) bad "invalid XAI_SMOKE_MODEL" ;;
			*)
				reply_length=$(curl -sfS --max-time 60 \
					--config "$CURL_AUTH_CONFIG" \
					-H "Content-Type: application/json" \
					-d "{\"model\":\"${XAI_SMOKE_MODEL}\",\"messages\":[{\"role\":\"user\",\"content\":\"Reply with exactly: OK\"}],\"max_tokens\":16}" \
					"https://api.x.ai/v1/chat/completions" 2>/dev/null \
					| python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d['choices'][0]['message']['content']))" 2>/dev/null || true)
				if [ -n "$reply_length" ] && [ "$reply_length" -le 128 ] 2>/dev/null; then
					ok "paid chat smoke $XAI_SMOKE_MODEL (response redacted, $reply_length chars)"
				else
					bad "paid chat smoke $XAI_SMOKE_MODEL"
				fi
				;;
		esac
	else
		note "  SKIP paid chat smoke (set RUN_PAID_XAI_SMOKE=1 to opt in)"
	fi
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

if [ -x "${APHRODY_SMOKE_HOME}/.local/bin/bxc-mcp" ]; then
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
  mcp_cfg="${APHRODY_MCP_CONFIG:-$APHRODY_SMOKE_HOME/.config/aphrody/mcp.json}"
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
