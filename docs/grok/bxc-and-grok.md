<!-- SPDX-License-Identifier: Apache-2.0 -->
# bxc vs xAI Grok

## bxc xAI integration (`bxc grok` + `@aphrody-code/xai`)

| Component | Twitter/X | xAI Grok |
| --- | --- | --- |
| `bxc/packages/x` | x.com GraphQL | — |
| `bxc/packages/xai` | — | `api.x.ai/v1` (OpenAI-compatible) |
| `bxc grok` CLI | — | `whoami`, `models`, `chat`, `tts`, `stt`, `raw` |
| `bxc-mcp` | `bxc_x_client`, `bxc_xcom_profile` | `bxc_grok_chat`, `bxc_grok_models`, `bxc_grok_whoami` + browser tools |

**Keyless auth:** `bxc grok` uses the same OIDC JWT as Grok Build (`~/.grok/auth.json`, field `key`) — no `XAI_API_KEY` required after `grok login`. Optional fallback: `XAI_API_KEY` (metered developer key).

## How they coexist on the VPS

```text
Grok Build  → ~/.grok/auth.json, grok CLI, aphrody-mcp (Context7, aphrody tools)
bxc.service → :3000 browser engine
bxc-mcp     → ~/.local/bin/bxc-mcp (stdio, 40 tools)
xAI API     → XAI_API_KEY → https://api.x.ai/v1
```

## MCP inventory (typical)

| Server | Binary | Domain |
| --- | --- | --- |
| aphrody | `aphrody-mcp` | Docs, RE, Gemini bridge, etc. |
| bxc | `bxc-mcp` | Web automation, X client tool |

Configure in `~/.config/aphrody/mcp.json` and Grok `config.toml` — do not duplicate secrets into committed JSON.

## Related docs

- Twitter/X: [`../x/bxc-integration.md`](../x/bxc-integration.md)
- bxc repo: `/home/ubuntu/bxc/README.md`