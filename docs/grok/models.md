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

## Grok Build CLI optionnel

Le binaire `grok` n'est pas garanti sur le VPS. Lorsqu'il est installé, sa
configuration locale provient de `~/.grok/config.toml` :

| Setting | Typical value |
| --- | --- |
| Default agent model | `grok-build` |
| Web search | `grok-4.20-multi-agent` |
| Subagents | `grok-build` |

Override via `GROK_WEB_SEARCH_MODEL` et `GROK_REASONING_EFFORT` dans
l'environnement local, ou via `~/.grok/config.toml`. Aucun `vps-env.sh` n'est
fourni par ce dépôt.

## Smoke model for API tests

Use **`grok-3-mini`** for cheap chat completion smoke tests; use **`grok-4`** when you need the largest context window.
