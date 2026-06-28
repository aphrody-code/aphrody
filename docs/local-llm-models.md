<!-- SPDX-License-Identifier: Apache-2.0 -->
# Model families — Gemma vs DeepSeek vs Qwen (for a 12 GB 4070)

Which open-weight family to run locally for aphrody (agentic CLI + OpenAI server +
RAG, single RTX 4070 12 GB, French-speaking user). Researched against current
official sources (mid-2026) — notable updates vs older knowledge: **Gemma 4
(2026-04) is now Apache-2.0**; DeepSeek flagships are **V3.2 (685B) / V4-Pro (1.6T)**;
Qwen is on **Qwen3.x**, Apache-2.0.

## Comparison (12 GB, Q4_K_M, Ollama)

| Criterion | **Gemma (Google)** | **DeepSeek** | **Qwen (Alibaba)** |
|---|---|---|---|
| Fits 12 GB @ Q4 | `gemma4:12b` ~7.6 GB (text+img+audio, 256K ctx); `gemma4:e2b` ~7.2 GB | **R1 distills only**: `deepseek-r1:8b` ~5–7 GB, `:14b` ~9 GB | `qwen3:8b` ~5 GB, `qwen3:14b` ~9 GB, `qwen2.5-coder:14b` ~9 GB |
| Doesn't fit | 26B ~18 GB, 31B ~20 GB, gemma3:27b | 32B distill ~20 GB, 70B, all flagships (671B–1.6T) | 30B-A3B MoE ~17–21 GB, 32B ~20 GB |
| Tool-use / agentic | good (G3/4 fn-calling + JSON) | **weakest** (reasoning-only, no advertised tools) | **best** (native fn-calling, MCP, Qwen-Agent) |
| Coding | decent (CodeGemma on old G2) | strong-for-size (reasoning-oriented) | **best fit** (`qwen2.5-coder:14b`) |
| Multilingual / French | **strong** (140+ langs) | weakest (EN/ZH lean) | **strong** (119+ langs) |
| Reasoning / math | solid (12B) | **best** (`deepseek-r1:8b` ≈ AIME-level) | strong (Thinking + Qwen-Math) |
| License | **G4 Apache-2.0**; G2/3/3n custom *Gemma Terms* (flow-down, Prohibited-Use, revocable) | MIT weights; **Llama-based distills carry Meta license** | **Apache-2.0** across Qwen3 (cleanest) |
| Context (12 GB-real) | 256K native (KV limits real use to tens of K) | ~32K (distill native) | 128K native |
| Modalities @fit | **text + image + audio IN** (12B, unique) | text-only | text (vision via `qwen3-vl`, +VRAM) |
| GGUF / Ollama | official QAT GGUFs · `ollama pull gemma4` | distills · `ollama pull deepseek-r1:8b\|14b` | first-party GGUF · `ollama pull qwen3:8b\|14b` |

## Verdicts

- **Gemma 4** — best multimodal-on-one-card (text+image+audio at 12B), 256K context,
  clean Apache-2.0; not a coding/agentic specialist; only G4 is truly open.
- **DeepSeek** — unmatched math/reasoning per GB via the R1 distills; but the entire
  flagship line (671B–1.6T) is unusable on 12 GB, distills are text-only, weakest at
  French, and **not** built for function-calling.
- **Qwen3** — best agentic all-rounder that fits 12 GB: native function-calling + MCP,
  strong French, strong code, Apache-2.0; its standout 30B-A3B/Coder-30B MoEs don't fit.

## Recommendation for aphrody (role-routed, all `ollama pull`)

| Job | Model | Pull | Why |
|---|---|---|---|
| Agentic chat / `aphrody run` | **Qwen3-14B** (or 8B) | `ollama pull qwen3:14b` | best 12 GB fn-calling/MCP, strong FR, Apache-2.0 |
| Coding | **Qwen2.5-Coder-14B** | `ollama pull qwen2.5-coder:14b` | strongest dedicated coder that fits (Qwen3-Coder is 30B) |
| Deep reasoning / math | **DeepSeek-R1-0528-Qwen3-8B** | `ollama pull deepseek-r1:8b` | SOTA-for-size reasoning; only distills fit |
| Multimodal / long-context | **Gemma 4 12B** (resident) | `ollama pull gemma4:12b` | only fits-12 GB model with image+audio in |

Skip on this card: any 30B-A3B/35B-A3B MoE (~17–21 GB + Ollama MoE GPU-util bug),
Gemma 26B/31B, DeepSeek 32B+, every 70B/235B+ flagship.

**Pairs with the engine choice** ([`local-llm-backends.md`](./local-llm-backends.md)):
all picks are plain GGUFs on the **Ollama** default backend, hot-swapped behind
`aphrody serve` (one resident at a time on 12 GB — load per route). Current box
state: slimmed to **`gemma4:12b` only**; the fleet above is opt-in.

See also: [`local-llm.md`](./local-llm.md) · [`local-llm-backends.md`](./local-llm-backends.md) · [`local-llm-ecosystem.md`](./local-llm-ecosystem.md).
