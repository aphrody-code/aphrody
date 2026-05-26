<!-- SPDX-License-Identifier: Apache-2.0 -->
# Keyless Google-AI audit

**Date:** 2026-05-26
**Scope:** every Google-AI surface reachable from the `aphrody` binary and the
`aphrody-mcp` server.
**Question:** is aphrody *fully keyless* for Google AI — authenticating only
with the Google AI Ultra OAuth token (the agy / Antigravity credential) or
signed-in Google session cookies, never an API key?

## Verdict

**YES.** No production code path reachable from `aphrody` (or `aphrody-mcp`)
requires or injects a Google API key (`x-goog-api-key`, `GOOGLE_API_KEY`,
`GEMINI_API_KEY`). Every Google-AI surface authenticates with either:

- the **OAuth Bearer token** reused from the agy / Antigravity credential store
  (Google One AI Ultra tier), routed to **Cloud Code `v1internal:generateContent`**
  and regional **Vertex `:generateContent`** — the public
  `generativelanguage.googleapis.com` host rejects this token
  (`403 ACCESS_TOKEN_SCOPE_INSUFFICIENT`), so it is never used there; or
- **signed-in Google session cookies** (Gemini web / Nano Banana / Veo /
  NotebookLM).

The only Google-API-key code in the tree is **dormant**: it lives in crates or
constructors that the `aphrody` binary never instantiates (see "Dormant code"
below). The acquisition of the token itself is keyless too — OAuth 2.0
Authorization-Code + **PKCE S256** with a **public client_id and no
client_secret**.

## Surface → auth → proof

| # | Surface | Auth mechanism | Keyless | Proof (file:line) |
|---|---|---|---|---|
| 1 | `aphrody chat` (default) | OAuth Bearer (agy) → Cloud Code `v1internal:generateContent` | yes | `crates/cli/src/commands.rs:1872`; `crates/cli/src/agy_backend.rs:56` |
| 2 | `aphrody chat --web` | Google session cookies + SAPISIDHASH | yes | `crates/gemini-web/src/auth.rs:80` |
| 3 | `aphrody chat --stub` | none (deterministic offline) | n/a | `crates/cli/src/commands.rs:1852` |
| 4 | `AgyBackend` (Cloud Code) | `Authorization: Bearer <token>` per request | yes | `crates/antigravity-sdk/src/client.rs:96,131` |
| 5 | agy login / refresh | OAuth 2.0 PKCE S256, public client_id, no secret | yes | `crates/antigravity-sdk/src/oauth.rs:19,151`; `auth.rs:30` |
| 6 | `aphrody hermes` / `aphrody agent` | same stub/web/agy order (Google = keyless) | yes | `crates/cli/src/agent_cmd.rs:464-495` |
| 7 | MCP `gemini_chat` / `gemini_image` / `gemini_video` / `gemini_deep_research` | Gemini-web cookies | yes | `crates/google_mcp/src/gemini_tools.rs:22,72,107,126` |
| 8 | `aphrody image generate` (Nano Banana) | cookies (`from_default_cookies`) | yes | `crates/cli/src/image_cmd.rs:6,62` |
| 9 | `aphrody notebooklm …` | Google cookies **or** Bearer OAuth, never a key | yes | `crates/notebooklm/src/auth.rs:69-108` |
| 10 | `aphrody gemini …` (passthrough) | spawns external `gemini` binary; aphrody injects nothing | yes (aphrody side) | `crates/cli/src/commands.rs:810-836` |

## Token source (exact)

The token is **never embedded** in the binary; it is read at runtime:

- **Windows** — generic credential `gemini:antigravity` in the Windows
  Credential Manager, read via `CredReadW` (`CRED_TYPE_GENERIC`). Blob is
  UTF-8 JSON `{"token":{"access_token":"ya29.…","refresh_token":"1//…",…}}`.
  `crates/antigravity-sdk/src/auth.rs:121-183`.
- **Linux / macOS** — `~/.config/aphrody/antigravity-token.json` (mode `0600`),
  same JSON envelope. `crates/antigravity-sdk/src/oauth.rs:479-532`.
- **Acquisition** — OAuth 2.0 Authorization-Code + PKCE S256, loopback
  `127.0.0.1:9109`, public client_id, no client_secret; refresh via
  `https://oauth2.googleapis.com/token` with the refresh_token only.
  `crates/antigravity-sdk/src/oauth.rs:194-235`.

## Dormant code (present, never reached)

These contain Google-API-key code but are **never constructed by the binary**;
they are kept for completeness so a future change does not reintroduce a key by
copying from them:

- `aphrody-chat` `AntigravityBackend::from_env` (`GOOGLE_API_KEY`) and
  `GeminiBackend` — `crates/aphrody-chat/src/backend.rs:618-619,173-288`. The
  CLI only ever builds `AgyBackend` / `GeminiWebBackend` / `StubBackend`.
- `aphrody-router` `GeminiProvider` (`x-goog-api-key`) —
  `crates/aphrody-router/src/lib.rs:917,977`. Pulled transitively via
  `aphrody-chat`, but instantiated only in `aphrody-router/tests/`.
- `aphrody-gateway` (`GOOGLE_ANTIGRAVITY_API_KEY`, BYOK `x-goog-api-key`) — not
  a dependency of `crates/cli` at all.
- `gemini-runtime::WebSearchConfig::with_api_key` — `web_search.rs:227`;
  constructed only in `tests/` and doc examples.

## Non-Google vendors (out of scope, all opt-in)

Third-party providers use their own keys/tokens, by nature outside the Google
AI Ultra token: Adobe Firefly (IMS OAuth S2S), ElevenLabs (`ELEVENLABS_API_KEY`),
Whisper/OpenAI (`OPENAI_API_KEY`), Mem0 (`MEM0_API_KEY`), Honcho
(`HONCHO_API_KEY`), X (`X_AUTH_TOKEN` cookie), Discord/Slack/Telegram/Matrix
(bot tokens), Context7 (optional). All are feature-gated / opt-in.

## Hardening recommendations (defense-in-depth, not blockers)

1. Feature-gate or remove the dormant Google-API-key code in `aphrody-chat`
   and `aphrody-router` so `cargo tree -p aphrody` carries no `x-goog-api-key`
   symbol.
2. Drop `aphrody-router` from `aphrody-chat`'s deps if only `AnthropicBackend`
   is wanted, or isolate the Gemini-key router behind an explicit `byok`
   feature.
3. Add a CI guard that fails if a **non-test** path in `crates/cli`
   (transitively) constructs a Google-API-key auth, to lock the invariant. To
   avoid false positives on the dormant code above, this must assert at the
   construction boundary, not by a naive source grep.
4. For `aphrody gemini` passthrough, document that auth is delegated to the
   external `gemini` binary; if a strict end-to-end keyless invariant is
   required, unset `GEMINI_API_KEY` before the spawn.
