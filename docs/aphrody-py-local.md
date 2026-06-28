<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody-local — the Python brain for local open-weight AI

`aphrody-local` (`py/aphrody-local/`, entry `aphrody.py`) is the **Python**
half of aphrody's open-weight cap. It deliberately **complements** the Rust
`aphrody-serve` (see [`local-llm.md`](./local-llm.md)) rather than duplicating
it: Rust owns the fast, cross-platform CLI + OpenAI HTTP server; Python owns the
ML / RAG / data surface where its ecosystem dominates (CLAUDE.md §2).

> **Status (2026-06-28):** scaffolded & chat-path verified. Streams real GPU
> tokens through the local OpenAI server. RAG: local fastembed backend + real
> `ragflow-sdk` client (RAGFlow Docker stack documented, not auto-started).

## Why a separate package

- `py/aphrody/` is the existing antigravity/forensics/research package (Python
  3.14, its own deps). `aphrody-local` is a clean, self-contained package with
  a narrow purpose and a light dependency set, so it can live in a dedicated
  **Python 3.12** venv (`~/aphrody-py-env`) — 3.14 is too new for several ML
  wheels (fastembed/onnxruntime, ragflow-sdk).
- It never touches `~/ml-env` (torch 2.11) — the local RAG path is **torch-free**
  (ONNX via fastembed), so no PyTorch clash.

## Architecture

```
 aphrody.py  (Typer CLI / SDK)
   ├─ aphrody_local.client.LocalAI ──(openai SDK)──► OpenAI /v1 base
   │     resolve order: aphrody-serve :8088 → ollama :11434 → vllm :8000 → llamacpp :8080
   ├─ aphrody_local.engines        ── discovery + model listing
   └─ aphrody_local.rag
         ├─ base.RagBackend (Protocol) + Chunk / RagAnswer / RagUnavailable
         ├─ local_backend.LocalRagBackend   (fastembed embed → cosine → rerank → answer)
         └─ ragflow_backend.RagflowBackend  (ragflow-sdk → RAGFlow Docker stack)
```

The LLM is always reached over the **OpenAI protocol**, so chat/answer synthesis
runs on whatever engine `aphrody-serve` fronts (Ollama/vLLM/llama.cpp). RAG
backends are chosen by `APHRODY_RAG_BACKEND` and share one `RagBackend` interface
so callers don't care which is active.

## RAG decision: local-first, RAGFlow-optional

| | Local backend (default) | RAGFlow backend |
|---|---|---|
| Deps | `fastembed` (ONNX, ~tens of MB) | RAGFlow Docker stack + `ragflow-sdk` |
| Footprint | venv only, no daemon | ES/Infinity + MySQL + Redis + MinIO; ~16 GB RAM, ~50 GB disk |
| Embeddings | `BAAI/bge-small-en-v1.5` (ONNX) | RAGFlow-managed (point at local embed model) |
| Rerank | `bge-reranker` cross-encoder (ONNX) | RAGFlow multi-recall + fused rerank |
| Doc understanding | text/markdown/PDF (basic) | deep layout/table/figure parsing |
| Best for | personal-scale, instant, offline | large heterogeneous corpora |

Both implement `RagBackend`; neither **fakes** results — if `fastembed` is
missing or RAGFlow is unreachable, you get a `RagUnavailable` with a fix, not an
empty answer.

### Running RAGFlow fully local (no cloud keys)

```bash
git clone https://github.com/infiniflow/ragflow && cd ragflow/docker
docker compose -f docker-compose.yml up -d        # ES + MySQL + Redis + MinIO + server
# UI at http://localhost:9380 → Model Providers → "OpenAI-API-Compatible":
#   base_url = http://host.docker.internal:8088/v1   (aphrody-serve)
#   set chat model + a local embedding/rerank model
# create an API key, then:
export APHRODY_RAG_BACKEND=ragflow RAGFLOW_API_KEY=ragflow-xxxxx
```

> `host.docker.internal` lets the RAGFlow containers reach `aphrody-serve` on the
> WSL host. On a 23 GB box this is feasible but heavy; prefer the local backend
> for day-to-day use.

## Quickstart

```bash
uv venv ~/aphrody-py-env --python 3.12
~/aphrody-py-env/bin/python -m pip install -U openai typer rich httpx numpy fastembed
PY=~/aphrody-py-env/bin/python
cd py/aphrody-local

$PY aphrody.py chat "say hi in 3 words"   # streams from the local engine
$PY aphrody.py doctor                      # config + engines + RAG health
$PY aphrody.py rag ingest ./README.md -d demo && $PY aphrody.py rag query "what is aphrody-local?" -d demo --sources
```

## Roadmap

| Step | Deliverable |
|------|-------------|
| ✅ P0 | package + CLI; chat streams via local OpenAI engine |
| ✅ P0 | RAG abstraction + local fastembed backend + real ragflow-sdk client |
| P1 | REPL mode (sessions, history) + roles |
| P1 | tool/function-calling agent loop over the local engine |
| P2 | RAGFlow live wiring verified end-to-end (Docker) |
| P2 | PyO3 bridge so the Rust `aphrody` CLI can call this RAG surface |

See also: [`local-llm.md`](./local-llm.md) · [`RAG.md`](./RAG.md) · [`../CLAUDE.md`](../CLAUDE.md).
