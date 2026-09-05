<!-- SPDX-License-Identifier: Apache-2.0 -->
# xAI / Grok + bxc test checklist

Run after `source ~/.bashrc` (loads `~/.bash_secrets`).

## xAI REST API

- [ ] `test -n "${XAI_API_KEY:-}"` — key present (do not echo)
- [ ] `bash ~/aphrody/scripts/test-xai-grok-bxc.sh` — le runner protège le
  Bearer hors de la liste des processus et teste `/models`
- [ ] Chat smoke seulement avec opt-in :
  `RUN_PAID_XAI_SMOKE=1 XAI_SMOKE_MODEL=grok-3-mini bash ~/aphrody/scripts/test-xai-grok-bxc.sh`

## Grok CLI

- [ ] `command -v grok >/dev/null && grok inspect` — seulement si le CLI optionnel est installé
- [ ] `command -v grok >/dev/null && grok mcp list` — seulement si installé

## bxc (browser / X tools)

- [ ] `bxc --version`
- [ ] `systemctl is-active bxc.service` or `curl -sS http://127.0.0.1:9222/json/version`
- [ ] `test -x ~/.local/bin/bxc-mcp`
- [ ] `bxc x whoami` — only if X cookies configured

## aphrody

- [ ] `aphrody mcp list` — aphrody + bxc OK
- [ ] `bash ~/aphrody/scripts/test-xai-grok-bxc.sh` — `/models` non génératif, aucun chat par défaut

Automated runner: `bash ~/aphrody/scripts/test-xai-grok-bxc.sh` (repo script).
Le chat nécessite l'opt-in potentiellement facturable
`RUN_PAID_XAI_SMOKE=1`.
