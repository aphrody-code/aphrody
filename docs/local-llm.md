<!-- SPDX-License-Identifier: Apache-2.0 -->
# Local open-weight LLMs — `aphrody serve`

aphrody's open-weight cap: **one OpenAI-compatible surface in front of any local
inference engine.** You run open-weight models (Llama, Qwen, Mistral, Gemma, …)
on local hardware; aphrody exposes them at `/v1/...` so every OpenAI client
(`openai` SDK, `curl`, LangChain, LlamaIndex, Continue, …) just works — no cloud,
no API key, no data leaving the box.

> **Status (2026-06-28):** **M0 shipped & verified.** The `aphrody-serve` crate
> streams real GPU tokens from a local engine through an OpenAI-compatible API.
> Remaining milestones (CLI `serve`/`chat` rewire, GUI, cloud carve-out) are
> tracked in [`PLAN.md`](./PLAN.md). Design rationale: [`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md).

## Architecture

```
 OpenAI clients (curl / openai SDK / LangChain / Continue / apps/web GUI)
        │  POST /v1/chat/completions, GET /v1/models, ...
        ▼
 ┌───────────────────────────────────────────────┐
 │  aphrody-serve  (axum, crates/aphrody-serve)   │   ← thin, portable HTTP layer
 │  OpenAI wire  ──►  aphrody_gateway::GatewayAdapter
 └───────────────────────────────────────────────┘
        │  OpenAI-compatible HTTP (the one seam)
        ▼
 ┌──────────┬──────────────┬───────────────┬──────────────┐
 │ Ollama   │ llama.cpp     │ vLLM (+LMCache)│ gemma (JAX)  │   ← GPU inference engines
 │ :11434   │ llama-server  │ :8000          │ research/FT  │
 └──────────┴──────────────┴───────────────┴──────────────┘
                     NVIDIA RTX 4070 12 GB · CUDA 13.3 · WSL2
```

The single integration point is **`aphrody_gateway::GatewayAdapter`**
(`crates/aphrody-gateway/src/lib.rs`). Its `OpenAiProxyAdapter`
(`openai_proxy.rs`) posts to any `OPENAI_BASE_URL`, so **any engine that speaks
the OpenAI protocol is a drop-in backend** — you only change `--base-url`.

## Quickstart

```bash
# 1. Build the server (uses the §7 sccache bypass + native Linux target)
cargo build -p aphrody-serve --config "build.rustc-wrapper=''" \
  --target x86_64-unknown-linux-gnu

# 2. Run it (defaults: 127.0.0.1:8080 → Ollama on 127.0.0.1:11434)
./target/x86_64-unknown-linux-gnu/debug/aphrody-serve --port 8088

# 3. Talk to it like OpenAI
curl -s http://127.0.0.1:8088/v1/models | jq .

curl -sN http://127.0.0.1:8088/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{"model":"llama3.2:1b",
       "messages":[{"role":"user","content":"Count from 1 to 5."}],
       "stream":true}'
```

> `Content-Type: application/json` is **required** — `curl -d` defaults to
> form-encoding, which the JSON extractor rejects with 422.

### Verified output (M0 acceptance)

```text
GET /v1/models
{"object":"list","data":[
  {"id":"gemma4-db-full:latest","object":"model","owned_by":"local",...},
  {"id":"gemma4:12b",...},{"id":"llama3.2:1b",...}]}

POST /v1/chat/completions (stream:true)
data: {"choices":[{"delta":{"content":"Here"},"finish_reason":null,...}],"object":"chat.completion.chunk",...}
data: {"choices":[{"delta":{"content":"'s"},...}],...}
...
data: [DONE]
```

Using the official OpenAI Python SDK:

```python
from openai import OpenAI
client = OpenAI(base_url="http://127.0.0.1:8088/v1", api_key="local")
r = client.chat.completions.create(
    model="gemma4:12b",
    messages=[{"role": "user", "content": "Hi"}],
)
print(r.choices[0].message.content)
```

## Endpoints

| Route                       | Status | Behaviour                                                      |
|-----------------------------|:------:|---------------------------------------------------------------|
| `GET  /healthz`             |  ✅    | Liveness probe → `ok`.                                         |
| `GET  /v1/models`           |  ✅    | Lists the backend's models (maps Ollama `/api/tags` → OpenAI). |
| `POST /v1/chat/completions` |  ✅    | Chat, **streaming (SSE) and non-streaming**.                  |
| `POST /v1/completions`      |  ✅    | Legacy text completion (transparent relay).                   |
| `POST /v1/embeddings`       |  ✅    | Embeddings (transparent relay; needs an embedding model — `ollama pull nomic-embed-text`). |

## Configuration

`aphrody-serve` is 12-factor; every flag has an env fallback.

| Flag         | Env                  | Default                   | Meaning                              |
|--------------|----------------------|---------------------------|--------------------------------------|
| `--host`     | `APHRODY_SERVE_HOST` | `127.0.0.1`               | Bind address.                        |
| `--port`     | `APHRODY_SERVE_PORT` | `8080`                    | Bind port.                           |
| `--base-url` | `OPENAI_BASE_URL`    | `http://127.0.0.1:11434`  | Backend engine root.                 |
| `--api-key`  | `OPENAI_API_KEY`     | `ollama`                  | Bearer token (ignored by Ollama).    |

Point it at a different engine simply by changing `--base-url`.

## Backends

All engines run isolated so nothing collides with the `ml-env` PyTorch 2.11
setup (see the `gpu-ai-stack` project memory / `~/GPU-AI-SETUP.md`).

### Ollama — default, live

Systemd `ollama.service` on `127.0.0.1:11434`, OpenAI-compatible under `/v1`.

```bash
ollama pull qwen2.5:7b-instruct-q4_K_M     # add a model
ollama list                                 # what's available
aphrody-serve                               # --base-url default already points here
```

### llama.cpp — `llama-server`

CUDA build at `~/llama.cpp/build/bin`. Its `llama-server` is OpenAI-compatible.

```bash
~/llama.cpp/build/bin/llama-server \
  -m ~/models/Qwen2.5-7B-Instruct-Q4_K_M.gguf \
  --host 127.0.0.1 --port 8080 -ngl 99
aphrody-serve --base-url http://127.0.0.1:8080
```

### vLLM (+ LMCache) — high throughput

Isolated env `~/vllm-env` (vLLM 0.23.0, torch 2.11+cu130). Best for batched /
concurrent serving; **memory-hungry** — on a 12 GB card use a small or quantized
(AWQ/GPTQ) model and cap KV memory.

```bash
~/vllm-env/bin/vllm serve Qwen/Qwen2.5-3B-Instruct \
  --host 127.0.0.1 --port 8000 \
  --max-model-len 8192 --gpu-memory-utilization 0.85
aphrody-serve --base-url http://127.0.0.1:8000 --api-key dummy
```

**LMCache** (`~/vllm-env`, KV-cache offload to CPU/disk → faster TTFT for
long-context/RAG/multi-turn) plugs into vLLM **V1** via its KV connector:

```bash
LMCACHE_CHUNK_SIZE=256 LMCACHE_LOCAL_CPU=True LMCACHE_MAX_LOCAL_CPU_SIZE=5 \
~/vllm-env/bin/vllm serve <model> --host 127.0.0.1 --port 8000 \
  --kv-transfer-config '{"kv_connector":"LMCacheConnectorV1","kv_role":"kv_both"}'
```

Confirm connector name / env keys against the LMCache docs
(`https://docs.lmcache.ai/getting_started/quickstart.html`) — they track the
installed vLLM version.

### gemma — native JAX (research / fine-tuning)

Isolated env `~/gemma-env` (`gemma` + `jax[cuda12]` 0.10.2, GPU detected). This
is the DeepMind reference implementation for **inference and fine-tuning** of
Gemma 2/3/3n/4 — not an HTTP server, so it is **not** a drop-in `/v1` backend.
Use it for LoRA/full fine-tunes and native sampling; serve the resulting weights
through Ollama/llama.cpp/vLLM.

```python
# ~/gemma-env/bin/python
from gemma import gm
model  = gm.nn.Gemma4_E4B()
params = gm.ckpts.load_params(gm.ckpts.CheckpointPath.GEMMA4_E4B_IT)
sampler = gm.text.ChatSampler(model=model, params=params, multi_turn=True)
print(sampler.chat("Hello"))
```

## Getting models — Hugging Face CLI

The `hf` CLI (1.21.0, uv-tool install) downloads weights into the shared cache.

```bash
# Gated repos (official Llama/Gemma) need a token: hf auth login
hf download unsloth/Qwen2.5-7B-Instruct-GGUF \
  Qwen2.5-7B-Instruct-Q4_K_M.gguf --local-dir ~/models
```

Ungated GGUF mirrors (unsloth, bartowski) need no auth.

## Hardware

NVIDIA RTX 4070 12 GB (Ada, sm_89) · Windows driver / CUDA 13.3 projected into
WSL2 · `nvcc` 13.3. The 12 GB VRAM ceiling drives model-size choices: 7B-Q4 GGUF
fits comfortably with room for context; 12B-Q4 (the `gemma4` family) fits via
Ollama's offloading; for vLLM prefer ≤3–7B or AWQ-quantized.

## Roadmap

| Milestone | Deliverable | Acceptance |
|-----------|-------------|------------|
| **M0** ✅ | `aphrody-serve`: `/v1/chat/completions` (stream+non), `/v1/models`, `/healthz` | curl streams local GPU tokens |
| **M1** ✅ | model discovery (Ollama `/api/tags` + `/v1/models` fallback) | `/v1/models` across engines |
| **M2** ✅ | `/v1/completions` + error parity (transparent relay) | `openai` SDK round-trip |
| **M3** ✅ | `/v1/embeddings` (transparent relay) | vector via an embedding model |
| M4 | CLI `aphrody chat`/`run` → `GatewayAdapter` (local default) | chat works with no cloud creds |
| M5 | GUI (`apps/web`) repointed to local `/v1` | browser streams local tokens |
| M6 | `cloud-providers` feature OFF by default | `--no-default-features` builds CLI+serve |
| M7 | local agent loop (tool-calling via Ollama) | `aphrody run` does a tool round-trip |
| M8 | in-process engine (candle), host-only | serves with Ollama stopped |

Community SDKs & tools that work against this server: [`local-llm-ecosystem.md`](./local-llm-ecosystem.md).

Full plan & rationale: [`PLAN.md`](./PLAN.md) · [`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md) · [`ARCHITECTURE.md`](./ARCHITECTURE.md) · project role in [`../CLAUDE.md`](../CLAUDE.md).
