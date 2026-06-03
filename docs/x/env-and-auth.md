<!-- SPDX-License-Identifier: Apache-2.0 -->
# X.com environment and authentication

This is **Twitter/X** (platform x.com), not **xAI** (Grok). For xAI see [`../grok/env-and-auth.md`](../grok/env-and-auth.md).

## Cookie session (primary for `bxc x` / `@aphrody-code/x`)

| Variable | Purpose |
| --- | --- |
| `X_AUTH_TOKEN` | `auth_token` cookie |
| `X_CT0` | `ct0` CSRF cookie |
| `X_HANDLE` | Optional default handle |
| `APHRODY_X_DEBUG` | Verbose client logs |

Store on VPS: `~/.bash_secrets` (sourced from `~/.bashrc`, mode `0600`). **Never commit.**

Session files (mode `0600`):

| Path | Role |
| --- | --- |
| `~/.aphrody/x-session.json` | `auth_token` + `ct0` for `bxc x` / `@aphrody-code/x` |
| `~/.bxc/cookies/xcom.json` | Full cookie jar (bxc shortcut `xcom`) |
| `~/.aphrody/cookies/xcom.json` | Mirror |

Import from DevTools: `python3 ~/aphrody/scripts/build-x-cookie-jar.py <export.json> --handle <screen_name>`

## Developer API (aphrody messaging / official X API)

| Variable | Class |
| --- | --- |
| `X_BEARER_TOKEN` | Direct token |
| `X_API_KEY` / `X_API_SECRET` | OAuth consumer |
| `X_ACCESS_TOKEN` / `X_ACCESS_TOKEN_SECRET` | OAuth 1.0a user |
| `X_CLIENT_ID` / `X_CLIENT_SECRET` | OAuth 2.0 |

Run `bash ~/awesome-grok-build/scripts/aphrody-env-audit.sh` for paid vs direct classification.

## Rate limits

Response headers: `x-rate-limit-limit`, `x-rate-limit-remaining`, `x-rate-limit-reset`.

The TS client sleeps when `remaining === 0`. Hard caps (e.g. daily tweet limit) return GraphQL error codes — see [architecture.md](architecture.md).