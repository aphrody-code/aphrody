<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: 2026 aphrody contributors -->

# Evaluation: `googleapis/js-genai` (Google Gen AI SDK) for aphrody

**Date:** 2026-05-26
**Scope:** Read-only cross-reference. Maps the canonical API surface of the
official Google Gen AI SDK for TS/JS (`googleapis/js-genai`, the successor to
`@google/generative-ai`) against aphrody's Rust Gemini crates, and extracts an
actionable, prioritized list of shapes / ids / patterns to adopt — **without**
adopting its API-key auth model.

**Sources (web):**
- Repo: <https://github.com/googleapis/js-genai/tree/main> (README, `src/models.ts`, `LICENSE`)
- Gemini API REST reference: <https://ai.google.dev/api/generate-content>
- Model catalog: <https://ai.google.dev/gemini-api/docs/models>

**Sources (aphrody, read-only cross-ref):** `crates/gemini-runtime/`,
`crates/gemini-web/`, `crates/google_mcp/`, `crates/aphrody-images/`,
`crates/antigravity-sdk/`, `crates/cli/src/agy_backend.rs`,
`crates/aphrody-gateway/src/byok/gemini.rs`.

---

## 0. TL;DR — the keyless reality

aphrody is **token-Google-AI-Ultra-only** (agy / Antigravity OAuth Bearer +
Gemini-web cookies). It holds **no API key** in its default path. js-genai is
built around two auth modes — **API key** (`generativelanguage.googleapis.com`,
`x-goog-api-key`) and **Vertex AI OAuth** (`aiplatform.googleapis.com`,
`Authorization: Bearer`). The agy OAuth token is **rejected** by the public
`generativelanguage` host (`401 ACCESS_TOKEN_TYPE_UNSUPPORTED` /
`403 ACCESS_TOKEN_SCOPE_INSUFFICIENT`, documented at
`crates/antigravity-sdk/src/endpoints.rs:67-74` and
`crates/cli/src/agy_backend.rs:27-31`), so aphrody reaches Gemini via two
keyless transports instead:

1. **Cloud Code modelbackend** — `POST cloudcode-pa.googleapis.com/v1internal:generateContent`,
   envelope `{ model, project, request }` (the path agy.exe uses).
   `crates/antigravity-sdk/src/endpoints.rs:130-136`.
2. **Regional Vertex AI** — `POST {loc}-aiplatform.googleapis.com/v1/projects/{project}/locations/{loc}/publishers/google/models/{model}:generateContent`
   (the agy token *is* accepted here). `crates/antigravity-sdk/src/endpoints.rs:69-74`.

**Conclusion:** adopt js-genai's **request/response shapes, model ids, and
patterns** (they are identical across all transports — same `GenerateContentRequest`
JSON, same method *suffixes* `:generateContent` / `:streamGenerateContent?alt=sse`
/ `:embedContent` / `:predict` / `:predictLongRunning`). Do **not** adopt its
API-key header injection for the default path. A BYOK API-key adapter already
exists and is correctly isolated (`crates/aphrody-gateway/src/byok/gemini.rs`).

---

## 1. js-genai canonical API surface

The SDK exposes one root `GoogleGenAI` handle with submodules
`ai.models`, `ai.chats`, `ai.files`, `ai.caches`, `ai.live`, `ai.batches`,
`ai.operations`, `ai.tunings`. The method → REST-suffix mapping below is taken
verbatim from `src/models.ts` (the path strings are the same regardless of
Gemini-API vs Vertex backend; only the host + the `{model}` prefix differ).

| Capability | js-genai method | REST suffix (`v1beta`/`v1`) | aphrody status |
|---|---|---|---|
| Text generation | `models.generateContent` | `{model}:generateContent` | **Covered** (3 transports) |
| Streaming | `models.generateContentStream` | `{model}:streamGenerateContent?alt=sse` | **Partial** — only BYOK (`byok/gemini.rs`) + `gemini` CLI NDJSON; not on agy/Vertex keyless |
| Function calling / tools | `tools` + `toolConfig` in config | (fields of `:generateContent`) | **Missing** in keyless request struct |
| Structured output (JSON) | `config.responseMimeType` + `responseSchema` | (fields of `generationConfig`) | **Missing** (sent as opaque `Value`) |
| System instructions | `config.systemInstruction` | `systemInstruction` field | **Covered** (`agy_backend.rs`, `models.rs`) |
| Embeddings | `models.embedContent` | `{model}:embedContent` (GeminiAPI) / `{model}:predict` (Vertex) | **Missing** (no Gemini embed path; local-only `aphrody-embed`) |
| Token counting | `models.countTokens` / `computeTokens` | `{model}:countTokens` / `{model}:computeTokens` | **Missing** |
| Image generation | `models.generateImages` | `{model}:predict` (Imagen) | **Different** — web-surface only (Nano Banana via prompt) |
| Image edit / upscale | `models.editImage` / `upscaleImage` | `{model}:predict` | **Missing** (Firefly/Photoshop cover edit) |
| Video generation | `models.generateVideos` | `{model}:predictLongRunning` + `operations.get` poll | **Different** — web-surface only (Veo via prompt) |
| Deep Research | `deep-research-*` agent model | `{model}:generateContent` (agentic) | **Different** — web-surface only (Pro prompt) |
| Files API | `ai.files.upload` / `get` / `list` / `delete` | `/upload/v1beta/files`, `/v1beta/files/*` | **Missing** |
| Context caching | `ai.caches.create` / `get` / `list` | `/v1beta/cachedContents` + `cachedContent` ref | **Missing** |
| Batch | `ai.batches.create` | `{model}:batchGenerateContent` | **Missing** |
| Live API | `ai.live.connect` | WebSocket `BidiGenerateContent` | **Missing** (native voice is separate, `aphrody-voice`) |

**License:** js-genai is **Apache-2.0** (`LICENSE` line 1 = "Apache License",
Version 2.0). Compatible with aphrody (Apache-2.0). Per CLAUDE.md §5 / §2 we do
**not** vendor the JS — we port shapes/ids/patterns to Rust only.

---

## 2. Canonical Gemini model ids (source of truth) + validation of aphrody ids

From <https://ai.google.dev/gemini-api/docs/models> (catalog, May 2026) and
js-genai samples. **Exact code strings:**

### Text / multimodal
| Model code | Channel | Notes |
|---|---|---|
| `gemini-3.5-flash` | **Stable** | "Most intelligent for agentic/coding". aphrody chat default. ✅ |
| `gemini-3-flash-preview` | Preview | Frontier-class preview. ✅ in `FALLBACK_MODELS` |
| `gemini-3.1-flash-lite` | Stable | Replaces the older flash-lite preview |
| `gemini-3.1-flash-lite-preview` | Preview | |
| `gemini-3.1-pro-preview` | Preview | Advanced + agentic |
| `gemini-3-pro-preview` | **SHUT DOWN 2026-03-09** | ⚠ **still listed** in aphrody `FALLBACK_MODELS` |
| `gemini-2.5-pro` | Stable | ✅ |
| `gemini-2.5-flash` | Stable | ✅ |
| `gemini-2.5-flash-lite` | Stable | ✅ |
| `gemini-2.0-flash` / `gemini-2.0-flash-lite` | **Deprecated** | ⚠ used in `antigravity-sdk` test fixtures & docstrings only |

### Generative media / embeddings / agents
| Model code | Purpose |
|---|---|
| `nano-banana-pro-preview` / `gemini-3-pro-image-preview` | Pro image (Nano Banana Pro) — aphrody uses `gemini-3-pro-image-preview` ✅ |
| `nano-banana-2-preview` / `gemini-3.1-flash-image-preview` | Flash image — aphrody uses `gemini-3.1-flash-image-preview` ✅ |
| `gemini-2.5-flash-image` | Fastest image — aphrody final fallback ✅ |
| `imagen-4` | Imagen text-to-image (predict path) — **not used** by aphrody |
| `veo-3.1-generate-preview` / `veo-3.1-lite-generate-preview` | Veo video (predictLongRunning) — **not used** (web-surface Veo instead) |
| `gemini-embedding-001` / `gemini-embedding-2` | Text / multimodal embeddings — **not used** (no Gemini embed path) |
| `deep-research-preview-04-2026` / `deep-research-max-preview-04-2026` | Deep Research agent — **not used** as a model id |
| `antigravity-preview-05-2026` | **General-purpose managed agent** — directly relevant to aphrody's agy backend; worth probing via `fetchAvailableModels` |
| `lyria-3-pro-preview` | Music (out of scope) |

### Validation verdict on aphrody ids
- `gemini-3.5-flash` (the chat default in `agy_backend.rs:24`, `web_search.rs:29`,
  `FALLBACK_MODELS[1]`): **VALID, stable, canonical.** No change.
- Web tokens (`gemini-web/src/models.rs`): the web app labels are "3.5 Flash",
  "3.1 Flash-Lite", "3.1 Pro" — these are **opaque per-build header tokens**, not
  REST ids, so they cannot be cross-validated against the REST catalog (they map
  to the picker, not to `generativelanguage`). The labels are internally
  consistent. **No divergence** to fix (the web tokens are a separate namespace).
- Image ids (`aphrody-images/src/models.rs:18-29`): all three **VALID**.
- **DIVERGENCE FOUND:** `gemini-3-pro-preview` in `FALLBACK_MODELS`
  (`gemini-runtime/src/lib.rs:87`) is **shut down** per the catalog. See §3 P0.

---

## 3. Gap analysis — aphrody vs js-genai (with file:line)

### Covered (parity)
- **`generateContent` text turns** — keyless via Cloud Code
  (`agy_backend.rs:136-194` → `antigravity-sdk/src/client.rs`) and Vertex
  (`endpoints.rs:69-74`). Request/response structs in
  `antigravity-sdk/src/models.rs:258-336` match js-genai's `GenerateContentRequest`
  / `GenerateContentResponse` for the text subset.
- **System instructions** — `agy_backend.rs:161-166` merges system turns into
  `system_instruction`, exactly the js-genai `config.systemInstruction` semantic.
- **Google-search grounding** — `gemini-runtime/src/web_search.rs` issues
  `{"tools":[{"google_search":{}}]}` and parses `groundingMetadata.groundingChunks`,
  matching js-genai's built-in Google Search tool. (Note: this path uses an API
  key — see §5.)
- **Image / video / deep-research** — covered *functionally* via the keyless
  Gemini **web surface** (`gemini_tools.rs`, `aphrody-images`), not via the
  js-genai predict endpoints.

### Missing or divergent
| Gap | js-genai shape | aphrody location & nature |
|---|---|---|
| **Stale model id** | `gemini-3-pro-preview` shut down | `gemini-runtime/src/lib.rs:87` lists it as a fallback (+ test `lib.rs:621` asserts its presence) |
| **Tools / function-calling in keyless requests** | `tools[]`, `toolConfig` fields | `antigravity-sdk/src/models.rs:261-274` `GenerateContentRequest` has **no `tools`/`tool_config` field**; `Part` (`:218-222`) models **only `text`** (no `functionCall` / `functionResponse` / `inlineData` / `fileData`) |
| **Structured output** | `generationConfig.responseMimeType="application/json"` + `responseSchema` | `models.rs:268-269` sends `generation_config` as opaque `serde_json::Value` — works, but no typed builder and no helper to set responseSchema; agy chat loop never sets it |
| **Streaming on keyless** | `:streamGenerateContent?alt=sse` (chunked `GenerateContentResponse`) | only `byok/gemini.rs:120` (API-key) and `gemini` CLI NDJSON (`gemini-runtime/src/lib.rs`) stream; the agy/Vertex keyless path is **request/response only** → higher perceived latency (cf. project latency objective) |
| **Embeddings (Gemini)** | `models.embedContent` → `{model}:embedContent` | no crate calls `:embedContent`; `aphrody-embed` is **local ONNX** only. No keyless cloud embeddings via Vertex `:predict` |
| **Files API** | `ai.files.upload/get/list/delete` | absent. Large media must be inlined |
| **Context caching** | `ai.caches.*` + `cachedContent` ref | absent. Directly relevant to the latency/cost objective for repeated long prompts |
| **Token counting** | `countTokens` / `computeTokens` | absent. `BackendResponse` hard-codes `prompt_tokens: 0` (`agy_backend.rs:189-192`) |
| **Veo via predict** | `generateVideos` → `:predictLongRunning` + `operations.get` | aphrody Veo is web-surface prompt only (`gemini_tools.rs:107-119`); no operation-polling model |
| **Batch / Live** | `batches`, `live` | absent (Live overlaps `aphrody-voice` but via a different protocol) |

---

## 4. Benefits to capture — prioritized (changes described, NOT applied)

### P0 — correctness (do first)
1. **Drop / replace the shut-down `gemini-3-pro-preview`.**
   - **File:** `crates/gemini-runtime/src/lib.rs:87` (the `FALLBACK_MODELS` tuple)
     and the assertion at `:621` (`assert!(ids.contains(&"gemini-3-pro-preview"))`).
   - **Change:** replace `("gemini-3-pro-preview", …)` with the live
     `("gemini-3.1-pro-preview", "gemini-3.1-pro-preview")` (the current Pro
     preview), and update the test to assert the new id. Keep `gemini-3.5-flash`
     as `FALLBACK_MODELS[1]` (still canonical/stable).
   - **Why:** a fallback to a dead id silently fails turns that degrade to it.

2. **Audit deprecated `gemini-2.0-flash` references.**
   - **Files:** `antigravity-sdk/src/models.rs` docstrings/fixtures
     (`:141`, `:438-444`, `:543`), `byok/gemini.rs:275` test URL.
   - **Change:** these are docstrings/tests, not runtime defaults — bump the
     sample ids to `gemini-2.5-flash` for hygiene so docs don't teach a
     deprecated id. Non-blocking, but cheap.

### P1 — high-value capability parity (keyless-compatible)
3. **Extend the keyless `GenerateContentRequest` to carry `tools` + `toolConfig`
   and enrich `Part`.**
   - **File:** `crates/antigravity-sdk/src/models.rs:261-274` (add
     `#[serde(skip_serializing_if="Option::is_none")] tools: Option<Value>` and
     `tool_config: Option<Value>`); `Part` (`:216-222`) gains optional
     `function_call`, `function_response`, `inline_data`, `file_data` variants.
   - **Pattern to port:** js-genai `FunctionDeclaration` + `FunctionCallingConfigMode.ANY`.
     The Cloud Code `v1internal:generateContent` and Vertex `:generateContent`
     both accept these fields verbatim (same proto), so this is **keyless-safe**.
   - **Why:** unlocks server-side function-calling on the agy tier; today the
     chat loop can only do text (`agy_backend.rs:185-193` returns empty
     `tool_calls`).

4. **Typed structured-output helper (`responseMimeType` + `responseSchema`).**
   - **File:** add a builder on `GenerateContentRequest` (or a thin wrapper in
     `agy_backend.rs`) that sets
     `generation_config = {"responseMimeType":"application/json","responseSchema":<schema>}`.
   - **Pattern to port:** js-genai `config.responseSchema` (JSON-Schema subset).
     Keyless-safe (it's a `generationConfig` field).
   - **Why:** deterministic JSON from agents/skills without brittle text parsing —
     directly improves the autonomy objective.

5. **Streaming on the keyless agy/Vertex path.**
   - **File:** `crates/antigravity-sdk/src/client.rs` (add a
     `generate_content_stream` calling the `:streamGenerateContent?alt=sse`
     suffix on the Cloud Code/Vertex host) consumed by `agy_backend.rs`.
   - **Pattern to port:** js-genai `generateContentStream` — SSE frames, each a
     `GenerateContentResponse` delta; accumulate `candidates[0].content.parts[].text`.
     aphrody already has an SSE chunk parser to mirror in `byok/gemini.rs`.
   - **Why:** time-to-first-token drops sharply (latency objective). Keyless-safe.

### P2 — new surfaces (mostly Vertex-keyless)
6. **Keyless Gemini embeddings via Vertex `:predict` / `:embedContent`.**
   - Add `embed_content(model, texts)` to `antigravity-sdk` hitting
     `{loc}-aiplatform.googleapis.com/.../models/gemini-embedding-001:predict`
     (Vertex form; the agy token is accepted there). Complements local
     `aphrody-embed` with a cloud option. Model id: `gemini-embedding-001`.
7. **Context caching + Files API** (`ai.caches` / `ai.files` equivalents on the
   Vertex host) — only if a long-prompt reuse workload appears; high payoff for
   cost/latency but larger surface.
8. **Token counting** (`countTokens`) so `BackendResponse.prompt_tokens`
   (`agy_backend.rs:189`) stops being hard-coded `0`.

### Implications for the future GUI views (image / video / deep-research)
The desktop app (`apps/desktop`) calls aphrody in-process. For new GUI views,
the **exact keyless call** to wire is:

- **Chat / structured / tools view:** `AgyBackend::complete` →
  `cloudcode-pa.googleapis.com/v1internal:generateContent` (envelope
  `{model,project,request}`), `model = "gemini-3.5-flash"`. For streaming, the
  P1#5 `:streamGenerateContent?alt=sse` variant.
- **Image view:** today → `gemini_tools::image` / `aphrody-images`
  (`ImageClient::generate_single`) over the **web surface** (cookies, no key),
  model fallback `gemini-3-pro-image-preview → gemini-3.1-flash-image-preview →
  gemini-2.5-flash-image`. (A future Vertex `imagen-4:predict` path would need a
  Vertex project but is keyless via the agy token.)
- **Video view:** today → `gemini_tools::video` (web-surface Veo, async, re-poll).
  The js-genai-faithful alternative is `veo-3.1-generate-preview:predictLongRunning`
  + `operations.get` polling on the Vertex host (keyless via agy token) — adopt
  this shape if the web surface proves unreliable for headless GUI use.
- **Deep-research view:** today → `gemini_tools::deep_research` (Pro prompt,
  web surface). js-genai exposes it as a *model id*
  (`deep-research-preview-04-2026`) on `:generateContent`; if that id is entitled
  on the agy tier (probe via `fetchAvailableModels`,
  `endpoints.rs:125`), the GUI could call it through the Cloud Code path for a
  structured agentic result instead of free-text.

---

## 5. Keyless constraint — explicit mapping

| js-genai auth mode | Endpoint | Compatible with aphrody keyless? |
|---|---|---|
| **API key** (`x-goog-api-key`) | `generativelanguage.googleapis.com/v1beta` | **NO** for the default path (agy token rejected here). Only used by the opt-in BYOK adapter (`byok/gemini.rs`) and `web_search.rs` when a user supplies a key. |
| **Vertex AI OAuth** (`Authorization: Bearer`) | `{loc}-aiplatform.googleapis.com/v1/...` | **YES** — the agy OAuth token is accepted (`endpoints.rs:67-74`). This is the keyless home for embeddings/imagen/veo predict paths. |
| **(aphrody-specific) Cloud Code** | `cloudcode-pa.googleapis.com/v1internal:*` | **YES** — the canonical agy.exe path; carries the Google One AI Ultra tier. Primary keyless transport for `:generateContent`. |
| **(aphrody-specific) Gemini web** | `gemini.google.com/_/BardChatUi/...` (cookies) | **YES** — separate batchexecute/StreamGenerate RPC namespace, not a js-genai surface, but covers chat/image/video/deep-research keylessly today. |

**Rule:** when porting any js-genai method, keep the **path suffix and JSON
body** identical, but route it to the Cloud Code or Vertex host with the OAuth
Bearer, never to `generativelanguage` with `x-goog-api-key` (except inside the
isolated BYOK adapter, which is correct as-is).

---

## 6. License / risk

- **js-genai:** Apache-2.0 (`LICENSE` line 1, Version 2.0) — **compatible** with
  aphrody (Apache-2.0, CLAUDE.md). No GPL contamination.
- **No vendoring:** per CLAUDE.md §2/§5 we do **not** add the JS package or
  re-vendor it. We port the **shapes, ids, and patterns** to Rust crates only.
  No new Cargo dependency results from this evaluation.
- **Supply-chain:** all proposed P0/P1 changes are pure-Rust edits to existing
  crates (`gemini-runtime`, `antigravity-sdk`); no new crates ⇒ no
  `cargo deny`/`vet` delta beyond what those crates already clear.
