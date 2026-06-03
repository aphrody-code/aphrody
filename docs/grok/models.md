<!-- SPDX-License-Identifier: Apache-2.0 -->
# xAI / Grok model ids

## RAGFlow vendor (`var/ragflow/conf/models/xai.json`)

| Model | max_tokens | Types |
| --- | --- | --- |
| `grok-4` | 256000 | chat |
| `grok-3` | 131072 | chat |
| `grok-3-fast` | 131072 | chat |
| `grok-3-mini` | 131072 | chat |
| `grok-3-mini-mini-fast` | 131072 | chat |
| `grok-2-vision` | 32768 | vision |
| `eve` | — | tts |

## Extended aliases (`llm_factories.json`)

Also referenced in deployment configs:

- `grok-4-0709`, `grok-3-beta`, `grok-3-mini-beta`
- `grok-4-fast-reasoning`, `grok-4-fast-non-reasoning`
- `grok-code-fast-1`
- `grok-2-image-1212`

Prefer **`GET /v1/models`** at runtime for the authoritative list tied to your API key.

## Grok Build CLI (this VPS)

From `~/.grok/config.toml` / `awesome-grok-build` templates:

| Setting | Typical value |
| --- | --- |
| Default agent model | `grok-build` |
| Web search | `grok-4.20-multi-agent` |
| Subagents | `grok-build` |

Override via `GROK_WEB_SEARCH_MODEL`, `GROK_REASONING_EFFORT` in `vps-env.sh`.

## Smoke model for API tests

Use **`grok-3-mini`** for cheap chat completion smoke tests; use **`grok-4`** when you need the largest context window.