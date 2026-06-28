# aphrody-local

A **real local aphrody** — the Python brain over this machine's open-weight
stack. It complements the Rust `aphrody-serve` OpenAI server: Python owns the
ML / RAG / data surface (chat, retrieval-augmented generation, model/engine
selection, agents), while Rust owns the fast cross-platform CLI + HTTP server.

```
 aphrody.py (CLI/REPL/SDK, Python 3.12, ~/aphrody-py-env)
   │  openai SDK  →  http://127.0.0.1:8088/v1  (aphrody-serve, Rust)
   │                  └─ or direct Ollama :11434/v1 fallback
   │  RAG:
   │    • local  → fastembed (ONNX, torch-free) embed + bge rerank
   │    • ragflow→ ragflow-sdk → RAGFlow Docker stack (OpenAI-API-Compatible LLM)
   ▼
 RTX 4070 12GB · CUDA 13.3 · WSL2
```

## Install

```bash
uv venv ~/aphrody-py-env --python 3.12
~/aphrody-py-env/bin/python -m pip install -U openai typer rich httpx numpy
# optional extras
~/aphrody-py-env/bin/python -m pip install fastembed        # local RAG
~/aphrody-py-env/bin/python -m pip install ragflow-sdk pypdf # RAGFlow + PDFs
```

## Use

```bash
PY=~/aphrody-py-env/bin/python

$PY aphrody.py chat "say hi in 3 words"      # stream a reply
$PY aphrody.py engines                        # discover live local engines
$PY aphrody.py models                         # list models on the resolved engine
$PY aphrody.py doctor                         # config + engine + RAG health

# RAG (local fastembed backend by default)
$PY aphrody.py rag ingest ./notes -d mynotes
$PY aphrody.py rag query "what did I write about X?" -d mynotes --sources
```

## Configuration (env)

| Var | Default | Meaning |
|-----|---------|---------|
| `APHRODY_BASE_URL` | auto-discover | OpenAI `/v1` base (else first live engine, else Ollama) |
| `APHRODY_API_KEY` | `local` | bearer token (ignored by local engines) |
| `APHRODY_MODEL` | first available | model id |
| `APHRODY_RAG_BACKEND` | `local` | `local` (fastembed) or `ragflow` |
| `APHRODY_EMBED_MODEL` | `BAAI/bge-small-en-v1.5` | fastembed embedding model |
| `RAGFLOW_BASE_URL` | `http://127.0.0.1:9380` | RAGFlow server |
| `RAGFLOW_API_KEY` | — | API key from the RAGFlow UI |

## RAGFlow

RAGFlow is heavyweight (Docker Compose: ES/Infinity + MySQL + Redis + MinIO +
server; ~16 GB RAM, ~50 GB disk). Point its model providers at the local engine
("OpenAI-API-Compatible", base `http://host.docker.internal:8088/v1`) for a
fully-local pipeline. See [`../../docs/aphrody-py-local.md`](../../docs/aphrody-py-local.md).
The default `local` backend needs no Docker.
