<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody (Python)

A keyless Python client and CLI for Google's AI Ultra stack — **Gemini, Cloud
Code and Vertex AI** — that authenticates with the OAuth credentials already
present on the machine. **No API key is ever required.**

aphrody reads the Antigravity desktop client's `gemini:antigravity` token from
the Windows Credential Manager (or a local cache on other platforms), keeps it
fresh by re-reading the store the Antigravity app maintains, and uses it as a
Bearer credential for Vertex AI (`google-genai`) and the Cloud Code `v1internal`
API.

It also drives the **Gemini web app** (`gemini.google.com`) directly: the
`aphrody web` command speaks the same Boq `batchexecute` protocol the browser
uses, authenticated by your stored Google session cookies — again no API key,
and not even an OAuth token, just the cookies.

## How it stays keyless

Path A — OAuth (Vertex AI + Cloud Code):

```
Windows Credential Manager (gemini:antigravity)
        │  OAuth access_token + refresh_token  (scope: cloud-platform, aicode, …)
        ▼
aphrody.auth.credentials.load_token()  ──keyless re-read on expiry──▶  credential store
        ├─▶ Credentials (keyless refresh) ─▶ google-genai (Vertex AI) ─▶ Gemini
        └─▶ Bearer header ─▶ cloudcode-pa.googleapis.com  (loadCodeAssist, …)
```

Path B — Cookies (the Gemini web app):

```
Google session cookies (__Secure-1PSID, __Secure-1PSIDTS, SAPISID, …)
        ▼
aphrody.auth.cookies (private store)  ─▶  GET /app → scrape SNlM0e + cfb2h
        └─▶ POST …BardFrontendService/StreamGenerate ─▶ gemini.google.com reply
```

The OAuth client id, endpoints and cookie names are public; the user's tokens
and cookies are read at runtime, stored only under `~/.aphrody/` (mode `0600`),
and never embedded, logged, or committed. The Antigravity client is a
confidential desktop client, so aphrody refreshes by re-reading the OS
credential store (which the app keeps current) rather than calling the token
endpoint with a secret it does not have.

## CLI

```console
$ aphrody whoami                       # signed-in Google account
$ aphrody token                        # token status (scopes, expiry) — never prints the token
$ aphrody chat "Summarize OAuth in one line."          # Vertex AI (OAuth)
$ aphrody web "Summarize OAuth in one line."           # Gemini web app (cookies)
$ aphrody models                       # account tier / models (Cloud Code)
$ aphrody image "a banana spaceship, studio render" --out ship.png
$ aphrody cookies status               # stored cookie metadata (never values)
$ aphrody cookies load cookies.json    # import a Cookie-Editor export
```

Run inside the workspace with `uv run aphrody <command>`.

## Library

```python
from aphrody import AphrodyClient
from aphrody.vertex import GeminiVertex

# Cloud Code / userinfo over the raw authenticated client:
with AphrodyClient.from_credential_manager() as client:
    print(client.userinfo()["email"])

# Text generation over Vertex AI (proven, recommended path):
print(GeminiVertex().generate("Say hello in one word."))
```

## Layout

| Module | Purpose |
|--------|---------|
| `aphrody.auth.tokens` | `OAuthToken` value type + expiry logic |
| `aphrody.auth.credential_store` | Windows Credential Manager + cross-platform cache |
| `aphrody.auth.oauth` | refresh / tokeninfo / userinfo |
| `aphrody.auth.credentials` | resolve a valid token → keyless google-auth `Credentials` |
| `aphrody.auth.cookies` | Google cookie jar store + `Cookie` header builder (web path) |
| `aphrody.endpoints` | public hosts, client ids, scopes, method paths |
| `aphrody.client` | `AphrodyClient` — Bearer HTTP + Cloud Code methods |
| `aphrody.vertex` | `GeminiVertex` — google-genai over Vertex AI |
| `aphrody.gemini_web` | `GeminiWebClient` — cookie-auth Boq client for gemini.google.com |
| `aphrody.images` | nano-banana image generation/editing (Gemini Image) |
| `aphrody.cli` | the `aphrody` command-line interface |

## License

Apache-2.0. Mirrors the native Rust `crates/antigravity-sdk` surface.
