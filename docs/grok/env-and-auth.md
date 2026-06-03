<!-- SPDX-License-Identifier: Apache-2.0 -->
# xAI / Grok credentials

## Global setup (VPS)

Store the API key only in **`~/.bash_secrets`** (mode `0600`), sourced from `~/.bashrc`:

```bash
export XAI_API_KEY="xai-…"   # metered — do not commit
```

Do **not** add `XAI_API_KEY` to `~/aphrody/.env` in git-tracked copies unless required for a specific service; prefer `bash_secrets` + audit.

## Classification (aphrody policy)

| Kind | Mechanism |
| --- | --- |
| Preferred for Grok Build | `~/.grok/auth.json` (no API key in prompts) |
| Paid metered | `XAI_API_KEY` → `api.x.ai` |
| Free / keyless elsewhere | Context7, Microsoft Learn via aphrody-mcp |

Run:

```bash
bash ~/awesome-grok-build/scripts/aphrody-env-audit.sh
```

## Rotation

If a key was pasted in chat or logs, rotate it in the [xAI console](https://console.x.ai/) and update `~/.bash_secrets` only.