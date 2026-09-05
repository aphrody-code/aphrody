<!-- SPDX-License-Identifier: Apache-2.0 -->
# Gemini web app (`gemini.google.com`) — consolidated protocol reference

The definitive reverse-engineering reference for the Gemini consumer web app, as
implemented by the `crates/gemini-web` SDK. Captured live 2026-05-21 against
build `boq_assistant-bard-web-server_2026051x` (account-owner session,
anonymised `<user>`). No secret/token/cookie values appear here.

## 1. Auth model

- **Cookies**: the signed-in Google session jar (`SAPISID`, `__Secure-1PSID`,
  `__Secure-3PSID`, `HSID`, `SSID`, `SID`, `APISID`, `__Secure-1P/3PSIDTS`, …).
  Exported (Cookie-Editor format) to `~/.aphrody/google-cookies.json`. Replayed
  as the `Cookie:` header on every request. `SAPISIDHASH` (SHA-256 over
  `<ts> <SAPISID> <origin>`) is available for the public APIs gateway but is not
  needed by the page RPCs (which use the `at` token).
- **`at` token** (anti-CSRF): `WIZ_global_data.SNlM0e` (~42 chars), scraped from
  the app HTML at bootstrap. Short-lived (~10 min) — refresh by re-fetching the
  page. Threaded in the `f.req` form body as `&at=<token>`.
- **`bl`** (build label): `WIZ_global_data.cfb2h` — `boq_assistant-bard-web-server_<date>`.
- **`f.sid`** (session id): `WIZ_global_data.FdrFJe`.

## 2. Transport

Two endpoints, both POST `application/x-www-form-urlencoded`, both Boq:

| Purpose | Path |
|---|---|
| Config / sync / lists | `/_/BardChatUi/data/batchexecute` |
| **Send / generate** | `/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate` |

Common query: `bl`, `f.sid`, `hl`, `_reqid` (monotonic, +100000 stride), `rt=c`.

### batchexecute envelope
- Request: `f.req=[[["<rpcid>","<inner_json_string>",null,"generic"]]]&at=<token>`
- Response: `)]}'` + length-prefixed chunks of `[["wrb.fr","<rpcid>","<inner_json>",…]]`.
- RPCs observed: `ESY5D` (config flag, e.g. `bard_activity_enabled`), `CNgdBe`
  (locale), `L5adhe` (UI prefs), `MaZiqc` (per-turn sync + conversation list),
  `VxUbXb` (incremental stream read), `PCck7e` (title gen), `aPya6c` (init/sync).

### StreamGenerate (send) — the real message path
- Body: `f.req=[null,"<inner_list_json>"]&at=<token>` (the `[null,"<json>"]`
  framing, NOT the batchexecute `[[[…]]]` envelope).
- `inner_list = [[prompt,0,null,null,null,null,0], [language], [cid,rid,rcid]]`
  (sparse context slots appended by the UI are optional for text-only sends).
- Response: Boq `)]}'` + `wrb.fr` chunks. Reply text at `inner[4][0][1][0]`;
  threading metadata at `inner[1]` = `[cid, rid]`; chosen candidate id at
  `inner[4][0][0]`. Generated images under `candidate[12]`; generated videos
  delivered as a late streamed chunk (Veo, async).

## 3. Model selection

Header `x-goog-ext-525001261-jspb` on the `StreamGenerate` POST:

```text
[1,null,null,null,"<token>",null,null,0,[4,5,6,8],null,null,3,null,null,<n>,1,"<client-uuid>"]
```

| Model          | token (`index 4`)  | `n` (`index 14`) | verified |
|----------------|--------------------|------------------|----------|
| 3.1 Flash-Lite | `1d44b34bcaa1c04d` | 6                | yes      |
| 3.5 Flash      | `56fdd199312815e2` | 1                | yes      |
| 3.1 Pro        | `e6fa609c3fa255c0` | 3                | inferred |

The trailing UUID is a *stable* per-client id (reuse one per session). The model
is locked per-conversation; switching needs a new chat.

## 4. Modes (`+` menu)

Selectable generative modes (each a chip re-routing the next send): **Images**
(Nano Banana), **Vidéos** (Veo, async minutes), **Musique**, **Canvas**, **Deep
Research** (two-step: plan → confirm → async multi-step), **Apprentissage
guidé**. **Deep Think** = the "Niveau de réflexion → Extended" reasoning level
(separate from the model token). The `gemini-web` SDK reaches image/video/
research via explicit prompt-routing + media extraction; mode-chip flag capture
(for guaranteed routing + plan-confirm + poll-to-completion) is the documented
enhancement.

## 5. SDK mapping (`crates/gemini-web`)

| Protocol element | SDK |
|---|---|
| cookie jar + `at`/`bl`/`f.sid` | `auth.rs`, `bootstrap.rs` (`SessionTokens`) |
| Boq envelope codec | `boq.rs` (`encode_f_req`, `parse_envelopes`) |
| batchexecute RPC | `transport.rs::rpc_raw` |
| StreamGenerate send | `transport.rs::stream_generate` + `payload.rs` |
| model header | `models.rs::GeminiModel::header` |
| reply + media parse | `payload.rs::parse_stream_response` → `types::ChatReply` |
| high-level client | `client.rs::GeminiWebClient` (`ask`/`send`/`get_config_flag`) |

Consumers: `aphrody-chat::GeminiWebBackend` (native 3.5 Flash chat) and the
`aphrody-mcp` tools `gemini_chat` / `gemini_image` / `gemini_video` /
`gemini_deep_research`.
