<!-- SPDX-License-Identifier: Apache-2.0 -->
# xAI / Grok credentials

## Global setup (VPS)

Store the API key only in **`~/.bash_secrets`** (mode `0600`), sourced from `~/.bashrc`:

```bash
export XAI_API_KEY="xai-…"   # metered — do not commit
```

Do **not** add `XAI_API_KEY` to `~/aphrody/.env` in git-tracked copies unless required for a specific service; prefer `~/.bash_secrets` plus the local diagnostic below.

## Classification (aphrody policy)

| Kind | Mechanism |
| --- | --- |
| Preferred for Grok Build | `~/.grok/auth.json` (no API key in prompts) |
| Paid metered | `XAI_API_KEY` → `api.x.ai` |
| Free / keyless elsewhere | Context7, Microsoft Learn via aphrody-mcp |

Diagnostic local sans afficher de valeur :

```bash
test -n "${XAI_API_KEY:-}" && echo "XAI_API_KEY: present" || echo "XAI_API_KEY: absent"
test ! -e "$HOME/.bash_secrets" || test "$(stat -c %a "$HOME/.bash_secrets")" = 600
test ! -e "$HOME/.grok/auth.json" || test "$(stat -c %a "$HOME/.grok/auth.json")" = 600
command -v grok >/dev/null && grok --version || echo "grok CLI: absent (optional)"
```

Le smoke du dépôt effectue un `GET /v1/models` non génératif. Il ne lance un
chat, potentiellement facturable, qu'avec `RUN_PAID_XAI_SMOKE=1` :

```bash
bash ~/aphrody/scripts/test-xai-grok-bxc.sh
RUN_PAID_XAI_SMOKE=1 bash ~/aphrody/scripts/test-xai-grok-bxc.sh
```

## Rotation

If a key was pasted in chat or logs, rotate it in the [xAI console](https://console.x.ai/) and update `~/.bash_secrets` only.
