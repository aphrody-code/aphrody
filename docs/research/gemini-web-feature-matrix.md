<!-- SPDX-License-Identifier: Apache-2.0 -->
# Gemini web app — feature-exploitation matrix (`crates/gemini-web`)

Honest verification of which `gemini.google.com/app` capabilities the
`gemini-web` crate exploits today, captured live 2026-05-21 (build
`boq_assistant-bard-web-server_20260520.03_p0`, account-owner session). Account
anonymised as `<user>`; no token/cookie values appear here.

## Verdict summary

| Feature | Web mechanism | Crate support | Status |
|---|---|---|---|
| Text chat (send/reply) | `StreamGenerate` POST, `f.req=[null,"<inner_list>"]` | `client.send/ask` + `parse_stream_response` | **FAIT** — verified live (reply parsed, cid/rid/rcid threaded) |
| Conversation threading | `[cid,rid,rcid]` in inner_list, echoed in response | `ConversationMetadata` round-trip | **FAIT** — verified live |
| Config read | `batchexecute` `ESY5D` | `client.get_config_flag` | **FAIT** — verified live (`bard_activity_enabled=true`) |
| Model selection (Flash-Lite / Flash / Pro) | `x-goog-ext-525001261-jspb` request header | `transport.send(model)` raw header + `rpc_ids::GeminiModel` | **FAIT (mechanism)** — Flash-Lite token verified live; Flash/Pro tokens page-sourced |
| **Deep Think** (extended reasoning) | "Niveau de réflexion → Extended" — a request flag, NOT a model token | not wired | **INCOMPLET** — flag slot identified, needs capture + wiring |
| **Nano Banana** (image generation) | image model/tool; reply carries generated-image descriptors | `ChatReply.web_image_urls` + `candidate[12]` parsing (response side) | **INCOMPLET** — response-side extraction present; request-side trigger not wired |
| **Veo 3** (video generation) | separate async tool; generation job + poll | not wired | **NON_FAIT** — async generate+poll flow required |
| **Deep Research** | separate async multi-step agent mode; job + poll | not wired | **NON_FAIT** — async agent+poll flow required |

## Evidence (live capture)

### Model picker (this account)
The in-app model dropdown exposes exactly: **3.1 Flash-Lite** (fastest), **3.5
Flash** (general), **3.1 Pro** (advanced code/math), plus a **"Niveau de
réflexion"** submenu with **Standard** and **Extended**. Veo, image generation
and Deep Research are NOT in the model picker — they are separate tools reached
from the left sidebar / `+` menu, each an async generative flow.

### Model-selector header (real format)
Selecting a model sets the `x-goog-ext-525001261-jspb` request header on the
`StreamGenerate` POST. The captured Flash-Lite value:

```text
[1,null,null,null,"1d44b34bcaa1c04d",null,null,0,[4,5,6,8],null,null,3,null,null,6,1,"<client-uuid>"]
```

- index 4 = the 16-hex model token (Flash-Lite = `1d44b34bcaa1c04d`, verified live).
- trailing element = a per-request client UUID (`crateGenerated`).
- Flash (`fbb127bbb056c959`) and Pro (`9d8ca3786ebdfbea`) tokens are present in
  the page model-config blob but their label↔token binding is page-sourced, not
  yet send-verified per model.

The earlier reconstructed `[1,null,null,null,"<token>"]` short form was WRONG;
`rpc_ids::GeminiModel::header()` now emits the full captured shape.

### Deep Think
"Extended" reasoning is a separate selection from the model token (it lives in
the reasoning-level submenu, not the model list). It is therefore a request
flag, captured alongside the model header — wiring it requires one more capture
to lock the flag position, then a `ReasoningLevel` field on the send path.

### Veo 3 / Nano Banana / Deep Research
These are surfaced in the page as experiment-gated tools, e.g. the live config
held `[45701617,null,null,null,"Veo 3.1 Fast",null,"TTwIIc"]`. They are NOT
plain header switches: image/video/research are asynchronous generation jobs
(submit → poll for completion → fetch media/result). Exploiting them needs a
distinct request shape per tool plus a polling loop — out of scope for the
text-chat transport and explicitly NOT claimed as working.

## Tools menu inventory (live, 2026-05-21)

The `+` menu in the input bar exposes these generative modes (each a selectable
chip that re-routes the next `StreamGenerate`): **Images**, **Vidéos**,
**Musique**, **Canvas**, **Deep Research**, **Apprentissage guidé** — plus the
attachment sources (Fichiers, Drive, Photos, Notebooks). Selecting one shows a
chip in the input bar (e.g. "Deep Research" with a "Sources / Fichiers" sub-row
and the placeholder "Que souhaitez-vous rechercher ?").

- **Deep Research** is a two-step async agent: the first turn returns a research
  *plan*; confirming it runs the multi-step investigation (minutes), streamed.
- **Vidéos (Veo)** is async video generation (minutes); the rendered video URL
  arrives in a late streamed chunk.

## Shipped (this work)

`aphrody-mcp` now exposes four Gemini tools backed by `gemini-web` (cookie auth,
no API key): `gemini_chat` (3.5 Flash + model switch), `gemini_image` (Nano
Banana), `gemini_video` (Veo), `gemini_deep_research` (Pro). The SDK
`ChatReply` now carries `generated_image_urls` + `generated_video_urls`
(structurally extracted from `candidate[12]`). Video/research use prompt-routing
(an explicitly phrased prompt + media extraction) rather than the mode-chip flag,
which is the documented enhancement for guaranteed routing + plan-confirmation
+ poll-to-completion.

## Next steps to fully exploit (concrete)
1. **Deep Think**: capture the `StreamGenerate` request with "Extended" selected;
   diff the model header / inner_list vs Standard to locate the flag; add
   `ReasoningLevel::{Standard,Extended}` to `send`.
2. **Nano Banana**: capture an image-gen send; confirm the trigger (image model
   token vs a tool flag); the response-side image extraction is already in
   `parse_stream_response`.
3. **Veo 3 / Deep Research**: capture the submit RPC + the poll RPC + the
   completion payload; implement an async `generate_video` / `deep_research`
   with a poll loop and a typed job handle.
