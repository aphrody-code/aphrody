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

## Smoke sécurisé

```bash
# Métadonnées uniquement ; aucun contenu n'est généré.
bash ~/aphrody/scripts/test-xai-grok-bxc.sh

# Chat minimal, seulement sur opt-in explicite et potentiellement facturable.
RUN_PAID_XAI_SMOKE=1 XAI_SMOKE_MODEL=grok-3-mini \
  bash ~/aphrody/scripts/test-xai-grok-bxc.sh
```

Le runner place le Bearer dans un fichier de configuration `curl` temporaire
de mode `0600`, jamais dans les arguments du processus, puis le supprime sur
toute sortie. Utiliser ce runner plutôt qu'un `curl -H` interactif.

## Grok Build CLI (separate surface)

Not hosted at `api.x.ai` for agent turns — uses xAI session via `~/.grok/`:

- Config: `~/.grok/config.toml`, `~/.grok/settings.json`

## aphrody-router note

Native `aphrody` Rust router allows **anthropic**, **gemini**, **antigravity** only — not provider id `xai`. Use Grok CLI or direct HTTP for Grok models.
