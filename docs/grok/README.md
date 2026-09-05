<!-- SPDX-License-Identifier: Apache-2.0 -->
# xAI Grok on the VPS

**Not** Twitter/X — that platform lives under [`../x/README.md`](../x/README.md) (`@aphrody-code/x`, `aphrody-x`, `bxc x`).

This tree documents **xAI** (Grok API + Grok Build CLI) as used with Aphrody.

## Documentation map

| Doc | Contents |
| --- | --- |
| [../../DEPLOY.md](../../DEPLOY.md) | VPS deploy (Rust CLI, MCP, A2A) |
| [api-endpoints.md](api-endpoints.md) | OpenAI-compatible `https://api.x.ai/v1` routes |
| [models.md](models.md) | Model ids from RAGFlow vendor config + Grok CLI |
| [bxc-and-grok.md](bxc-and-grok.md) | What bxc does *not* cover; MCP layout on VPS |
| [env-and-auth.md](env-and-auth.md) | `XAI_API_KEY`, `~/.grok/auth.json`, diagnostic local et smoke optionnel |
| [test-checklist.md](test-checklist.md) | Smoke tests (API + CLI + MCP) |

## Auth priority (aphrody policy)

1. **Grok CLI session** — `~/.grok/auth.json` (`grok login`) — preferred for Grok Build and **`bxc grok`** (no developer key)
2. **`XAI_API_KEY`** — OpenAI-compatible REST (`api.x.ai`) — metered fallback; set in `~/.bash_secrets`
3. **Antigravity / agy OAuth** — Gemini/Code Assist, not xAI — see `docs/agy-cli/`

## Quick verify

```bash
source ~/.bashrc
# Smoke local — GET `/models` non génératif ; chat désactivé par défaut
bash ~/aphrody/scripts/test-xai-grok-bxc.sh
# Opt-in explicite, potentiellement facturable :
RUN_PAID_XAI_SMOKE=1 bash ~/aphrody/scripts/test-xai-grok-bxc.sh
# Checklist complémentaire à lire : docs/grok/test-checklist.md
```
