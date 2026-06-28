<!-- SPDX-License-Identifier: Apache-2.0 -->
# JARVIS — aphrody's North Star

> **J**ust **A** **R**ather **V**ery **I**ntelligent **S**ystem.
>
> A fully-local, private, voice-driven, agentic, multimodal personal AI running on
> a single RTX 4070 (12 GB, WSL2). **No cloud key required for the core loop.**
>
> Status: the organs already exist as production-grade Rust crates; the **wiring
> between them is the work**.

This is the product target for the open-weight refactor ([`local-llm.md`](./local-llm.md)):
local LLM serving + STT + RAG + agent loop + voice I/O are the *substrate*; JARVIS
is the assistant on top.

---

## 1. Vision

JARVIS is **one always-on agent** that **hears you, thinks with a local open-weight
model, acts on your machine under hard safety gates, speaks back, and remembers** —
entirely on this box, with zero data leaving WSL.

aphrody is uniquely positioned because every organ already ships as a real Rust crate:

- a streaming agentic turn loop — `aphrody-engine` (`run_turn`),
- a provider-agnostic model seam — `aphrody-model-client::ModelClient`,
- a local OpenAI server already serving Ollama / gemma4 / Qwen3 over `/v1` — `aphrody-serve`,
- local STT — `aphrody-voice` (whisper.cpp),
- four production vector-memory backends — `aphrody-memory`,
- defense-in-depth command safety — `aphrody-guard`.

The open-weight stack closes the loop: **Qwen3 = best agentic tool-caller at 12 GB**
([`local-llm-models.md`](./local-llm-models.md)), gemma4 for multimodal chat,
whisper.cpp for ears, plus a **local TTS to add** for the mouth. What's missing is
not capability — it's the **handful of seams** that connect ears → brain → hands →
mouth → memory into a single daemon.

---

## 2. Reference architecture — Barth's *jarvis-OS*

We are not designing in a vacuum. **[Grominet95/jarvis-OS](https://github.com/Grominet95/jarvis-OS)**
("Barth's Jarvis", [demo video](https://www.youtube.com/watch?v=oP8kKUKfq3w)) is a
working, local-first personal AI with an architecture we explicitly learn from. It
is a FastAPI server hosting a text chat **and** a real-time voice pipeline bridged
via LiveKit, organised in **four layers**:

| Layer | Barth's jarvis-OS | The pattern worth stealing |
|---|---|---|
| **L1 — Instant voice agent** | LiveKit WebRTC pipeline: STT (Whisper/Deepgram) → LLM → TTS (Piper/ElevenLabs), native VAD + noise-cancellation + barge-in. Tools: web search, Gmail, Calendar, Spotify, vision, code-exec. Interfaces: voice, text, Telegram. | A **streaming STT→LLM→TTS loop with turn-detection** is the heart. The LLM is *your choice* (any OpenAI-compatible endpoint). |
| **L2 — Memory + initiative** | Local SQLite. Memory = **atomic facts**, each *dated*, *sourced* to the exchange that produced it, **reinforced** when re-heard, **archived** when contradicted — *never deleted*. Nightly **AutoDream** + `ConsolidationAgent` (≈03:00) re-read sessions to extract missed facts. Proactive: monitors mail/weather/calendar → proposes drafts, reminders, digests. | **Atomic-fact memory with provenance + reinforcement** (not blob summaries). **Nightly consolidation** as a scheduled background pass. **Proactive initiative** = event/schedule → propose action. |
| **L3 — Learning + security** | User creates/validates new **Skills**. A **security validation portal** evaluates *risk + cost* before execution; critical/kernel actions are **forbidden by design**. | A **risk/cost gate in front of every action**, with a hard-forbidden tier — defense-in-depth, not a single allow/deny. |
| **L4 — Ecosystem** | **Skills** (software integrations: CAD, 3D-printing…), **Presets** (event sequences, e.g. "streamer mode"), **Views** (visualizations: weather, 3D models). | Modularity as three distinct extension points: *capabilities* (Skills), *macros* (Presets), *surfaces* (Views). |

**Stack Barth uses:** LiveKit (real-time voice), Electron (desktop cockpit),
MediaPipe (vision/gestures), SQLite (local privacy DB).

### 2.1 aphrody already maps onto all four layers — often more maturely

The striking finding: aphrody has a real equivalent for nearly every Barth layer,
and the Rust core is frequently *more* production-grade.

| Barth layer / feature | aphrody equivalent | Fit |
|---|---|---|
| L1 LiveKit STT→LLM→TTS + VAD | whisper.cpp (STT) + `aphrody-serve`→engine (LLM) + **TTS gap**; `aphrody-voice` traits (`Transcriber`/`Speaker`/`Sink`) | **adopt a streaming voice loop** (see §6) |
| L1 tools (Gmail/Cal/Spotify/files) | `aphrody-tools` + `aphrody-toolcall` + MCP (`google_mcp`, `aphrody-mcp` OAuth 2.1) | ✅ framework |
| L1 Telegram / multi-channel | `aphrody-messaging` (Telegram/Matrix/Slack/Discord/X, bidirectional) | ✅ |
| L1 camera / gestures (MediaPipe) | `aphrody-capture` (screen→PNG) + gemma4 vision; webcam/MediaPipe = gap | partial |
| L2 SQLite local memory | `aphrody-memory` (JSONL/HNSW/SQLite/LanceDB, pure-Rust, `#![forbid(unsafe_code)]`) | ✅ (stronger) |
| L2 atomic-fact provenance/reinforce | `mem0`/`honcho` summarization in `aphrody-memory` + eviction — **needs the dated/sourced/reinforced fact model** | partial → adopt |
| L2 **AutoDream** nightly consolidation | the **`dream` skill** (`bun-agent:bun-dreamer`) + `aphrody-cron` (At/Cron schedules) | ✅ *already exists* — wire to memory |
| L2 proactive initiative | `aphrody-cron` + `aphrody-events` + `aphrody-supervisor` | partial (framework, not wired) |
| L3 security validation portal | **`aphrody-guard`** (command-safety Allow/Prompt/Forbidden + process hardening) + `aphrody-skills/permissions` (allow/ask/deny, scope tags) | ✅ strong match; add **risk/cost scoring** |
| L3 skill create/validate | `aphrody-skills` (+ forge) | ✅ |
| L4 Skills ecosystem | `aphrody-skills` + `aphrody-marketplace` | ✅ |
| L4 Presets (macros) | `aphrody-cron` action sequences | gap (new concept to formalize) |
| L4 Views (surfaces) | `apps/web` (React + Material 3) | partial |
| Electron cockpit | `apps/web` (web GUI, repoint to local `/v1`) | ✅ (web, not Electron) |

**Strategic read:** aphrody is ~70 % structurally there and often more production-grade
(Rust `aphrody-guard` ≈ his sandbox; the `dream` skill ≈ AutoDream;
`aphrody-marketplace` ≈ his Skills ecosystem). The decisive new work is the **local
brain seam** (§8) and a **streaming voice loop** (§6); the rest is wiring existing organs.

---

## 3. Organ map (honest: scaffold vs production)

Verified against the real tree (workflow `w79pxax2h`, 2026-06-28).

| Organ | Key crates / files | What works **today** | Maturity | The gap |
|---|---|---|---|---|
| **Voice** (ears + mouth) | `aphrody-voice/src/stt/local_whisper.rs`, `stt/whisper_api.rs`, `elevenlabs.rs`; `cli/src/agent_cmd.rs` (`hermes`) | Cloud STT (Whisper API, ElevenLabs) + cloud TTS (ElevenLabs, streaming). Local whisper.cpp STT compiles behind `local-whisper`. Full voice→voice loop in `hermes`. Object-safe `Transcriber`/`Speaker`/`Sink` traits → headless-testable. | **Partial.** Cloud path production; local STT real but feature-gated off; **no local TTS at all**; no mic I/O. | No local TTS (Piper/Kokoro); no `cpal` mic capture; no wake-word; no barge-in; no streaming STT (`local_whisper.rs` buffers whole clip); `local-whisper` not wired into any CLI command. |
| **Brain** (agentic loop) | `aphrody-engine/src/turn.rs` (`run_turn`), `session.rs`, `actor.rs`; `aphrody-model-client/src/lib.rs` (`ModelClient`); `aphrody-agent-runtime/src/model.rs` (`ModelChoice`); `aphrody-toolcall` | Real streaming multi-turn loop: SSE `ModelStreamEvent::{TextDelta,ReasoningDelta,ToolCall,Completed}`, tool dispatch via `ToolRegistry`, result re-injection, `max_tool_iterations` cap, 13+ events to `EventSink` + JSONL rollout. Interactive actor with steering + interrupt. `StubModelClient` for offline tests. | **Production loop, single provider.** | **Only `impl ModelClient` is `GeminiClient`.** `ModelChoice` = `Gemini` \| `Stub` only — **no local variant**. The whole agentic brain talks **only to cloud Gemini**. |
| **Memory** | `aphrody-memory/src/lib.rs` (`MemoryBackend` + `jsonl`/`hnsw`/`sqlite`/`lancedb`, `mem0`, `honcho`, `eviction`); `aphrody-embed`; `aphrody-context`; `aphrody-session` | **Strongest organ.** 4 production vector backends (pure-Rust HNSW, SQLite BLOB, LanceDB ANN >100k rows, durable JSONL). `mem0`/`honcho` summarization, eviction. `aphrody-context` token budgeting + summarization triggers. | **Production — but orphaned.** | `MemoryBackend` **not called by `aphrody-engine/session.rs`**. No recall-before-think / write-after-act. `aphrody-embed` not pointed at the local `/v1/embeddings` that `aphrody-serve` already exposes. |
| **Perception** (eyes) | `aphrody-capture` (Win GDI screen→PNG), `google_mcp` (`screen_capture`), `aphrody-fsindex` (SQLite FTS5), `aphrody-re` (PE/ELF triage), `aphrody-agent-proto` (`InputItem::Image`) | On-demand screen capture → base64 PNG MCP tool. FS metadata index. Binary triage. Image protocol fields exist. | **Partial / scaffold.** | `turn.rs` renders images as text `[image: path]` — **media never reaches the model**. No local vision inference wired (gemma/paligemma in py venv, no FFI). Screen capture Windows-only GDI; no webcam; no continuous perception loop; no machine-state telemetry. |
| **Hands** (action + safety) | `aphrody-tools`, `aphrody-toolcall`, `aphrody-agent-tools/src/shell.rs`, `aphrody-guard/src/command_safety.rs` + `harden.rs`, `aphrody-skills/src/permissions.rs`, `aphrody-task-runner`, `aphrody-mcp` | **Production.** Vendor-neutral tool registry → Anthropic/Gemini/MCP formats. Direct-argv shell (no `sh -c`), timeout + 64 KiB cap. Command-safety classifier (catches `rm -rf`/`dd`/`git push --force`). Permission engine (allow/ask/deny, scope tags, layered). DAG task runner. MCP OAuth 2.1 client. | **Production execution + safety.** | No interactive approval UX at CLI surface (Gated gate is channel-only). No sandbox (seccomp/containers); no per-tool resource caps; no persistent audit trail. No **risk/cost scoring** (Barth L3). |
| **Ambient** (proactive / always-on) | `aphrody-cron`, `aphrody-events`, `aphrody-messaging`, `aphrody-supervisor`, `aphrody-terminal-backend` (WS PTY + systemd notify), `aphrody-serve`, `apps/web` | Scheduler (Every/At/Cron, JSON persist), pub-sub bus (NDJSON/memory/counter sinks), bidirectional channels, multi-agent supervisor (fan-in events), OpenAI `/v1` HTTP, React Web GUI. | **Partial — components real, daemon absent.** | **No `aphrody daemon`** co-hosting scheduler + bus + messaging + voice. No event→action glue (cron emits `Event`, nothing routes it). No proactive trigger rule store. `aphrody-serve` is an **OpenAI relay via `GatewayAdapter`, not wired to `aphrody-engine`** — zero tool-calling over HTTP. |

---

## 4. The JARVIS loop (minimum lovable, end-to-end)

The smallest loop that *feels* like JARVIS, with the **exact crate** and the **missing wire**:

```
 [1 LISTEN]      [2 TRANSCRIBE]     [3 THINK]              [4 ACT]            [5 SPEAK]        [6 REMEMBER]
   mic     →   whisper.cpp   →   local agentic LLM   →   tools (gated)  →   local TTS   →   vector store
```

1. **Listen** — capture mic audio. Crate: *none yet*. **WIRE:** add `cpal` mic capture (`aphrody-voice/src/audio.rs`), 16 kHz mono PCM.
2. **Transcribe** — speech → text. Crate: `aphrody-voice/src/stt/local_whisper.rs`. **WIRE:** turn on `--features local-whisper` and select it as the default `Transcriber`.
3. **Think** — local agentic turn with tool-calling. Crates: `aphrody-engine::run_turn` + `aphrody-model-client::ModelClient`, model served by `aphrody-serve` (`/v1`, Ollama/Qwen3). **WIRE (keystone):** there is **no local `impl ModelClient`** and **no `ModelChoice::Local`**. Add `LocalOpenAiClient` (SSE→`ModelStreamEvent`) hitting `http://127.0.0.1:8080/v1`, plus Qwen3 function-calling → `ToolCall` mapping.
4. **Act** — execute tool calls under safety. Crates: `turn.rs` → `ToolSafety` → `aphrody-guard::command_safety` → `ToolExecutor::handle`. **Already wired** — works the moment stage 3 emits real `ToolCall`s. Run JARVIS v1 in `Gated` autonomy.
5. **Speak** — text → audio out. Crate: `aphrody-voice` (`Speaker`). **WIRE:** **no local TTS exists** — add `aphrody-voice/src/tts/piper.rs` (or Kokoro), wire as default `Speaker`, push PCM to the same `cpal` device as stage 1.
6. **Remember** — persist + retrieve. Crates: `aphrody-memory::MemoryBackend` + `aphrody-embed` (via local `/v1/embeddings`). **WIRE:** `session.rs` never calls `MemoryBackend`. Add recall-before-think + write-after-act (store via `mem0`/`honcho`, with Barth's dated/sourced/reinforced fact model).

**Net:** stage 4 is done; stages 1, 2, 3, 5, 6 each need one concrete wire. **Stage 3's
local `ModelClient` is the linchpin** — nothing downstream is "JARVIS" until the brain runs locally.

---

## 5. Biggest gaps (ranked by leverage)

1. **Local `ModelClient` + `ModelChoice::Local` (the keystone).** Without it the entire `aphrody-engine` agentic loop (tools, sessions, memory, autonomy, rollouts) only talks to cloud Gemini. One new `impl ModelClient` against local `/v1` unlocks every other organ at once. *Highest leverage by far.*
2. **Wire `aphrody-serve` → `aphrody-engine`.** Serve is an OpenAI relay through `GatewayAdapter` — no tool-calling over HTTP. Route `/v1/chat/completions` through `run_turn` so the local server itself becomes agentic (and the Web GUI / any OpenAI client gets tools for free).
3. **Local TTS provider (Piper / Kokoro).** The mouth is 100 % cloud (ElevenLabs). No local `Speaker` impl exists.
4. **Mic capture + default-on local STT.** No `cpal`; `local-whisper` is off and unwired.
5. **Memory ↔ engine wiring.** `aphrody-memory` is the most production-ready organ yet `session.rs` never calls it. Recall-before-think / write-after-act turns a stateless agent into a personal one.
6. **`aphrody daemon` (always-on).** Cron + bus + messaging + supervisor exist but are never co-hosted with event→action glue. Blocks proactive/ambient JARVIS + AutoDream.
7. **Wake word + barge-in + streaming STT/TTS.** Natural-conversation polish; defer until the batch loop works.

---

## 6. Voice transport — decision

Barth uses **LiveKit** ([Apache-2.0, self-hostable](https://docs.livekit.io/agents/),
Python/Node SDKs) for the L1 voice loop: an `AgentSession(stt=…, llm=…, tts=…, turn_detection=…)`
that handles VAD, barge-in, and noise-cancellation, where the **LLM is any
OpenAI-compatible endpoint** — i.e. `aphrody serve` plugs straight in, with whisper as
the STT plugin and Piper as the TTS plugin.

aphrody is **Rust-primary** (CLAUDE.md §0/§2); LiveKit Agents is a Python/Node runtime.
So the call is:

- **Default (single box) = pure-Rust local loop.** `cpal` mic → whisper.cpp →
  `run_turn` (local model) → Piper → `cpal` out. No WebRTC, no extra runtime, fully
  aphrody-native — the right shape for a localhost JARVIS on this RTX 4070. This is §4
  / Phases 2–4.
- **Multi-device / remote = LiveKit transport (reference pattern, opt-in).** When you
  want to reach JARVIS from a phone or browser over the network with production-grade
  VAD/noise-cancellation/barge-in, stand up a LiveKit agent under `py/` or `ts/` that
  uses `openai.LLM(base_url="http://127.0.0.1:8080/v1")` (= `aphrody serve`) + a whisper
  STT plugin + a Piper TTS plugin. The brain, tools, memory, and safety stay in the Rust
  core; LiveKit is only the audio transport. Exactly Barth's separation.

Both share the same brain (`aphrody-serve` → `aphrody-engine`), so the transport is a
deployment choice, not a re-architecture — the same principle as the swappable
`--base-url` engine ([`local-llm-backends.md`](./local-llm-backends.md)).

---

## 7. Roadmap — today → JARVIS v1

Each phase is independently demoable.

- **Phase 0 — Local brain (keystone).** Add `LocalOpenAiClient: ModelClient` (SSE→`ModelStreamEvent`) + `ModelChoice::Local` + `aphrody run --local`. **Accept:** `aphrody run --local "list files then summarize"` streams from Qwen3 on this box and **executes a real shell tool call** via the engine — no cloud key.
- **Phase 1 — Agentic local server.** Route `aphrody-serve` `/v1/chat/completions` through `aphrody-engine` instead of the `GatewayAdapter` relay. **Accept:** an OpenAI client (or `apps/web`) hitting `:8080` gets tool calls executed server-side and sees `ToolCallBegin/End` events.
- **Phase 2 — Voice-in (local STT).** Add `cpal` mic capture + default-on `local_whisper`. **Accept:** speak into the mic → correct transcript, fully offline.
- **Phase 3 — Voice-out (local TTS).** Add `piper`/`kokoro` `VoiceProvider`, set as default `Speaker`, play via `cpal`. **Accept:** type a question → hear a spoken local answer, no ElevenLabs key.
- **Phase 4 — Full local voice loop.** Wire a new `aphrody jarvis` to mic→whisper→`run_turn`(local)→tools(Gated)→piper. **Accept:** *speak a question, get a spoken answer from the local model, with at least one tool call executed under a safety gate.*
- **Phase 5 — Memory-augmented (+ atomic facts).** Call `aphrody-memory` (sqlite/hnsw + `aphrody-embed` via local `/v1/embeddings`) for recall-before-think / write-after-act in `session.rs`, with Barth's dated/sourced/reinforced fact model. **Accept:** tell JARVIS a fact in one session, ask in a fresh session, it recalls correctly.
- **Phase 6 — Always-on daemon (+ AutoDream + initiative).** `aphrody daemon` boots cron + event bus + messaging + the voice loop; cron `Event` → connector glue; a nightly `dream`-skill consolidation pass over the day's sessions; risk/cost scoring on the `aphrody-guard` gate (Barth L3). **Accept:** a scheduled job fires, JARVIS speaks a proactive notification and/or DMs a channel — unattended; the nightly pass updates memory.

---

## 8. First build (single highest-leverage step)

**Build `LocalOpenAiClient: ModelClient` + `ModelChoice::Local`, exposed as `aphrody run --local`.**

Why this and nothing else first: `aphrody-engine` is already a production streaming
agentic loop with tool dispatch, sessions, safety gates, autonomy modes, and rollout
recording — but the **only `impl ModelClient` is `GeminiClient`** and the **only
`ModelChoice` variants are `Gemini`/`Stub`** (both verified in-tree). That single
missing seam is what keeps the whole brain cloud-bound. `aphrody-serve` already serves
Qwen3/gemma4 over local `/v1`, so the model endpoint exists; the work is purely an
SSE-stream adapter (`/v1/chat/completions` deltas → `ModelStreamEvent::{TextDelta,
ToolCall, Completed}`) plus Qwen3 function-calling → `ToolCall` mapping.

The moment it lands, **every other organ becomes reachable locally at once**: Hands
(tools already wired to the engine), Memory (engine sessions become local),
Voice/Ambient (all flow through `run_turn`). It is the smallest diff with the largest
unlock — and it makes the entire JARVIS loop possible without a cloud key. Home it in
`aphrody-providers`/`aphrody-router`, default it to Qwen3 in `Gated` autonomy.

---

## See also

- [`local-llm.md`](./local-llm.md) — local serving substrate (the `aphrody serve` stack)
- [`local-llm-models.md`](./local-llm-models.md) — Qwen3 / Gemma4 / DeepSeek picks at 12 GB
- [`local-llm-backends.md`](./local-llm-backends.md) — Ollama / llama.cpp / vLLM engine choice
- [`aphrody-py-local.md`](./aphrody-py-local.md) — the Python orchestrator + RAG (`py/aphrody-local`)
- Reference: [Grominet95/jarvis-OS](https://github.com/Grominet95/jarvis-OS) · [LiveKit Agents](https://docs.livekit.io/agents/)
