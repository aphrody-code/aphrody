<!-- SPDX-License-Identifier: Apache-2.0 -->
# Backend engines — Ollama vs llama.cpp vs vLLM (decision)

aphrody fronts all three behind one OpenAI surface ([`aphrody-serve`](./local-llm.md)),
so the engine is a swappable `--base-url`. This page records, for **our needs**
(local open-weight on an RTX 4070 12 GB / WSL2, powering a CLI + GUI + OpenAI
server; single-to-moderate concurrency; easy model swap; cross-platform per
CLAUDE.md §0), which engine to use when — and the standing default.

## Our needs, ranked

1. **Dev UX & model management** — pull/swap models with zero friction.
2. **VRAM efficiency on 12 GB** — run the biggest useful model that fits.
3. **Single-stream latency** — CLI/GUI is one user at a time; TTFT matters.
4. **Cross-platform** — Linux #1, Windows #2, wasm #3.
5. **Throughput under concurrency** — only for batch/agent/RAG fan-out.

## Comparison

| Dimension | **Ollama** | **llama.cpp** | **vLLM** |
|---|---|---|---|
| Model mgmt / hot-swap | ★★★ `pull`/`run`, keep-alive, multi-loaded | ★ manual GGUF, 1 server/model | ★ manual HF, restart to swap |
| VRAM efficiency @12 GB | ★★★ GGUF Q4 → runs 12B | ★★★ GGUF Q4 | ★★ AWQ/GPTQ INT4 + pre-alloc KV → ~7B comfortably, ~13–14B feasible |
| Single-stream latency | ★★★ | ★★★ (lightest) | ★★ (batching overhead) |
| Concurrent throughput | ★★ `OLLAMA_NUM_PARALLEL` (default 1, opt-in) | ★★ `-np N` + continuous batching (on by default) | ★★★ PagedAttention (paged KV cache) |
| New archs / features | ★★ registry + GGUF import | ★★ huge GGUF ecosystem | ★★★ HF day-1, LoRA, tools, multimodal |
| OpenAI-compat completeness | ★★ `/v1` chat/cmpl/embed/models/responses; tools+vision+JSON+seed (no logprobs/rerank) | ★★★ `/v1` chat/cmpl/embed/models/responses + rerank + Anthropic `/v1/messages` (tools via `--jinja`) | ★★★ tools, logprobs, rerank, structured outputs |
| KV reuse across requests | – | – | ★★★ via **LMCache** |
| Footprint / deps | ★★ Go daemon | ★★★ tiny C++ binary, no Python | ★ heavy (torch+CUDA, ~9.5 GB env) |
| Cross-platform (Lin/Win/wasm) | ★★★ Lin/Win/Mac | ★★★ widest (incl. CPU/edge) | ★ Linux / WSL2, NVIDIA (no native Windows) |
| Startup time | ★★★ on-demand | ★★ per-process | ★ slow weight load |

## Decision (autonomous)

- **Default = Ollama.** Best UX + hot-swap + VRAM-efficient GGUF (runs our kept
  `gemma4:12b` in ~7.6 GB) + cross-platform + zero-config. It is already
  `aphrody-serve`'s default backend (`127.0.0.1:11434`). For a single-user
  CLI/GUI this is strictly the right call.
- **llama.cpp = portability/control tier.** Reach for `llama-server` when you
  want a single static binary, the widest hardware, a specific GGUF, or no
  daemon. Same family as our `whisper.cpp` (STT). It's also the path if we ever
  want in-process Rust inference (`llama-cpp-2`, deferred — it breaks wasm #3).
- **vLLM (+ LMCache) = throughput tier.** Opt-in for **concurrency**: agent
  fan-out, RAG batch, serving many clients, or KV reuse on long-context/RAG.
  Not the default on 12 GB — its server-grade throughput (PagedAttention +
  continuous batching) is overkill for one user, it is single-model-per-process,
  and on 12 GB you run a quantized (AWQ/GPTQ INT4) ~7–14B model with a capped
  `--max-model-len`. No native Windows build, but officially runs under WSL2 on NVIDIA.

### How aphrody switches

```bash
aphrody serve                                   # default → Ollama :11434
aphrody serve --base-url http://127.0.0.1:8000  # → vLLM (throughput/RAG)
aphrody serve --base-url http://127.0.0.1:8080  # → llama-server (portable/control)
```

This is the whole point of the proxy-first architecture: the engine is a policy
choice, not a code change. The comparison **validates the Ollama default** while
keeping vLLM/llama.cpp one flag away.

### Empirical note (this box, 2026-06-28)

Measured on the RTX 4070 / WSL2:

- **Ollama** and **llama.cpp** serve cleanly — verified streaming through `aphrody serve`.
- **vLLM 0.23 + torch 2.11+cu130** *initializes* fully (engine init 66 s, torch.compile +
  CUDA-graph capture, **5.9 GiB KV cache**, **~10.4 GB total for a 1.5B model** at
  `--gpu-memory-utilization 0.80` — empirical proof of aggressive KV pre-allocation: it
  fills the VRAM budget regardless of model size, which is exactly what buys its ~27×
  concurrency headroom) **but its OpenAI HTTP endpoint hung** (requests timed out,
  `status 000`). So the `aphrody serve → vLLM` swap is **unverified pending a
  vLLM/cu130/WSL2 debug** (bleeding-edge torch 2.11+cu130 + vLLM 0.23 APIServer↔EngineCore).
  Net: vLLM is the **opt-in** throughput tier, not a zero-config default — exactly how
  aphrody treats it.

See also: [`local-llm.md`](./local-llm.md) · [`local-llm-ecosystem.md`](./local-llm-ecosystem.md).
