<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody (Python)

A keyless Python client and CLI for Google's AI Ultra stack — **Gemini, Cloud
Code, Vertex AI, the Gemini web app, image generation and a local voice loop** —
that authenticates with the credentials already present on the machine.
**No API key is ever required.**

aphrody reads the Antigravity desktop client's `gemini:antigravity` token from
the Windows Credential Manager (or a token file on other platforms), keeps it
fresh by re-reading the store the Antigravity app maintains, and uses it as a
Bearer credential for Vertex AI (`google-genai`) and the Cloud Code `v1internal`
API.

It also drives the **Gemini web app** (`gemini.google.com`) directly: the
`aphrody web` command speaks the same Boq `batchexecute` protocol the browser
uses, authenticated by your stored Google session cookies — again no API key,
and not even an OAuth token, just the cookies.

## How it stays keyless

Path A — OAuth (Vertex AI + Cloud Code + image generation):

```
Windows Credential Manager (gemini:antigravity)   [Linux: var/secrets/antigravity-token.json]
        │  OAuth access_token + refresh_token  (scope: cloud-platform, aicode, …)
        ▼
aphrody.auth.credentials.load_token()  ──keyless re-read on expiry──▶  credential store
        ├─▶ Credentials (keyless refresh) ─▶ google-genai (Vertex AI) ─▶ Gemini / Gemini Image
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
and cookies are read at runtime, stored only under the git-ignored secrets dir
(`var/secrets/` inside the repo, else `~/.aphrody/`; override with
`APHRODY_SECRETS_DIR`), mode `0600`, and never embedded, logged, or committed.
The Antigravity client is a *confidential* desktop client, so aphrody refreshes
by re-reading the OS credential store (which the app keeps current) rather than
calling the token endpoint with a secret it does not have.

## Install

```console
$ uv sync                      # inside the aphrody workspace
$ uv run aphrody <command>
```

Optional extras:

| Extra | Pulls | Enables |
|-------|-------|---------|
| `aphrody[cookies]` | `browser-cookie3` (LGPL, isolated) | `aphrody cookies extract` from Chrome |
| `aphrody[voice]` | `google-antigravity[voice]` (faster-whisper, kokoro-onnx, numpy, websockets) | `aphrody voice` |
| `aphrody[dev]` | `pytest`, `pytest-httpx` | the test suite |

## CLI

```console
$ aphrody whoami                       # signed-in Google account (email + name)
$ aphrody token                        # token status (scopes, expiry) — never prints the token
$ aphrody chat "Summarize OAuth in one line."          # Vertex AI (OAuth, keyless)
$ aphrody web  "Summarize OAuth in one line."          # Gemini web app (cookies, keyless)
$ aphrody models                       # account tier / models (Cloud Code, with a tier fallback)
$ aphrody image gen "a banana spaceship, studio render" --out ship.png --size 4K   # Nano Banana Pro
$ aphrody image icon gen "rocket launch" --style rounded --out rocket.png --ico     # Material 3 icon + .ico
$ aphrody voice                        # local voice-to-voice loop (see below)
$ aphrody cookies status               # stored cookie metadata (names/domains, never values)
$ aphrody cookies load cookies.json    # import a Cookie-Editor JSON export
$ aphrody cookies extract              # pull cookies from local Chrome (needs aphrody[cookies])
```

Output is always UTF-8 (JSON for structured results), so the CLI is safe to
script and to drive from sub-agents regardless of the host console code page.

### `chat` vs `web`

Both generate text with no API key, by different routes:

- **`chat`** → Vertex AI via the OAuth token (`gemini-2.5-flash` by default; pass
  `--model`). Best for programmatic, system-style prompting. Project resolves via
  `APHRODY_VERTEX_PROJECT` / `GOOGLE_CLOUD_PROJECT`.
- **`web`** → the consumer Gemini web app via your session cookies. You get
  exactly what the browser gives you (the same model picker, grounding, persona).

### Voice (`aphrody voice`)

A fully local, keyless **voice-to-voice loop**, served to the browser:

```
browser mic ─ws→ Whisper STT ─→ Antigravity Agent (keyless) ─→ Kokoro TTS ─ws→ browser speaker
```

- **STT**: local faster-whisper (`base` by default, `--whisper-model`).
- **Brain**: the `google.antigravity` Agent — keyless, driven by the Antigravity
  token; it streams the reply sentence-by-sentence with barge-in/interrupt.
- **TTS**: local Kokoro ONNX; the model + voices auto-download to
  `~/.aphrody/models/` on first run (`--voice-name`, default `ff_siwis`, French).
- Serves a WebSocket on `ws://127.0.0.1:8789` and a localized web UI on
  `http://127.0.0.1:8790` (`--host`, `--port`, `--ui-port`, `--ui false`).

Requirements: install the `aphrody[voice]` extra, and make the Antigravity agent
**localharness** binary discoverable. That binary ships with the Antigravity
desktop app (e.g. `…/Antigravity/resources/bin/language_server.exe` on Windows);
point `ANTIGRAVITY_HARNESS_PATH` at it if it is not auto-discovered.

## Library

```python
from aphrody import AphrodyClient, GeminiWebClient
from aphrody.vertex import GeminiVertex

# Cloud Code / userinfo over the raw authenticated client (OAuth):
with AphrodyClient.from_credential_manager() as client:
    print(client.userinfo()["email"])

# Text generation over Vertex AI (OAuth, keyless):
print(GeminiVertex().generate("Say hello in one word."))

# Text generation over the Gemini web app (cookies, keyless):
with GeminiWebClient() as web:
    print(web.generate("Say hello in one word."))
```

## Layout

| Module | Purpose |
|--------|---------|
| `aphrody._paths` | resolve the private secrets dir (`var/secrets/` ▸ `~/.aphrody`) |
| `aphrody.auth.tokens` | `OAuthToken` value type + expiry logic |
| `aphrody.auth.credential_store` | Windows Credential Manager + token-file source/cache |
| `aphrody.auth.oauth` | refresh / tokeninfo / userinfo |
| `aphrody.auth.credentials` | resolve a valid token → keyless google-auth `Credentials` |
| `aphrody.auth.cookies` | Google cookie jar store + `Cookie` header builder (web path) |
| `aphrody.endpoints` | public hosts, client ids, scopes, method paths |
| `aphrody.client` | `AphrodyClient` — Bearer HTTP + Cloud Code methods |
| `aphrody.vertex` | `GeminiVertex` — google-genai over Vertex AI |
| `aphrody.gemini_web` | `GeminiWebClient` — cookie-auth Boq client for gemini.google.com |
| `aphrody.images` | Nano Banana Pro (Gemini 3 Pro Image) generation/edit/compose, 1K/2K/4K |
| `aphrody.prompts` | Nano Banana Pro prompt template library + enhancer (no deps) |
| `aphrody.optimize` | lossless PNG (oxipng) + WebP/AVIF re-encode — `aphrody[images]` |
| `aphrody.batch` | declarative concurrent bulk generation from a JSON spec |
| `aphrody.icons` | Material 3 icon generation + SVG→PNG→Windows `.ico` — `aphrody[icons]` |
| `aphrody.voice_server` | local Whisper↔Agent↔Kokoro voice loop + web UI |
| `aphrody.cli` | the `aphrody` command-line interface |

## License

Apache-2.0. Mirrors the native Rust `crates/antigravity-sdk` surface.
