<!-- SPDX-License-Identifier: Apache-2.0 -->
# xAI / Grok + bxc test checklist

Run after `source ~/.bashrc` (loads `~/.bash_secrets`).

## xAI REST API

- [ ] `test -n "$XAI_API_KEY"` — key present (do not echo)
- [ ] `curl -sS https://api.x.ai/v1/models -H "Authorization: Bearer $XAI_API_KEY" | head -c 500`
- [ ] Chat smoke: `grok-3-mini`, one-word reply
- [ ] Optional: `grok-4` if quota allows

## Grok CLI

- [ ] `grok inspect`
- [ ] `grok mcp list` — aphrody-mcp registered

## bxc (browser / X tools)

- [ ] `bxc --version`
- [ ] `systemctl is-active bxc.service` or `curl -sS http://127.0.0.1:9222/json/version`
- [ ] `test -x ~/.local/bin/bxc-mcp`
- [ ] `bxc x whoami` — only if X cookies configured

## aphrody

- [ ] `aphrody mcp list` — aphrody + bxc OK
- [ ] `bash ~/awesome-grok-build/scripts/aphrody-env-audit.sh`

Automated runner: `bash ~/aphrody/scripts/test-xai-grok-bxc.sh` (repo script).