<!-- SPDX-License-Identifier: Apache-2.0 -->
# xAI API endpoints (OpenAI-compatible)

Official base URL: **`https://api.x.ai/v1`**

Source in-repo: `var/ragflow/conf/models/xai.json`, `var/ragflow/conf/llm_factories.json`.

## REST routes

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/v1/models` | List available models |
| POST | `/v1/chat/completions` | Chat (primary) |
| POST | `/v1/tts` | Text-to-speech (`eve`, etc.) |
| POST | `/v1/stt` | Speech-to-text |

Authorization header:

```http
Authorization: Bearer $XAI_API_KEY
```

## Example (chat)

```bash
curl -sS https://api.x.ai/v1/chat/completions \
  -H "Authorization: Bearer $XAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "grok-3-mini",
    "messages": [{"role": "user", "content": "Reply with exactly: OK"}],
    "max_tokens": 16
  }'
```

## Example (models)

```bash
curl -sS https://api.x.ai/v1/models \
  -H "Authorization: Bearer $XAI_API_KEY"
```

## Grok Build CLI (separate surface)

Not hosted at `api.x.ai` for agent turns — uses xAI session via `~/.grok/`:

- Config: `~/.grok/config.toml`, `~/.grok/settings.json`
- Docs: [awesome-grok-build/docs/grok-unlocked-setup.md](https://github.com/aphrody-code/awesome-grok-build/blob/main/docs/grok-unlocked-setup.md) (community kit)

## aphrody-router note

Native `aphrody` Rust router allows **anthropic**, **gemini**, **antigravity** only — not provider id `xai`. Use Grok CLI or direct HTTP for Grok models.