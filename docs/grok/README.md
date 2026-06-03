<!-- SPDX-License-Identifier: Apache-2.0 -->
# xAI Grok on the VPS

**Not** Twitter/X — that platform lives under [`../x/README.md`](../x/README.md) (`@aphrody-code/x`, `aphrody-x`, `bxc x`).

This tree documents **xAI** (Grok API + Grok Build CLI) as used with aphrody and `awesome-grok-build`.

## Documentation map

| Doc | Contents |
| --- | --- |
| [api-endpoints.md](api-endpoints.md) | OpenAI-compatible `https://api.x.ai/v1` routes |
| [models.md](models.md) | Model ids from RAGFlow vendor config + Grok CLI |
| [bxc-and-grok.md](bxc-and-grok.md) | What bxc does *not* cover; MCP layout on VPS |
| [env-and-auth.md](env-and-auth.md) | `XAI_API_KEY`, `~/.grok/auth.json`, audit script |
| [test-checklist.md](test-checklist.md) | Smoke tests (API + CLI + MCP) |

## Auth priority (aphrody policy)

1. **Grok CLI session** — `~/.grok/auth.json` (`grok login`) — preferred for Grok Build and **`bxc grok`** (no developer key)
2. **`XAI_API_KEY`** — OpenAI-compatible REST (`api.x.ai`) — metered fallback; set in `~/.bash_secrets`
3. **Antigravity / agy OAuth** — Gemini/Code Assist, not xAI — see `docs/agy-cli/`

## Quick verify

```bash
source ~/.bashrc
# Masked audit — never prints full keys
bash ~/awesome-grok-build/scripts/aphrody-env-audit.sh
grok inspect
bash ~/aphrody/docs/grok/test-checklist.md  # follow checklist
```