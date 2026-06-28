<!-- SPDX-License-Identifier: Apache-2.0 -->
# Local-LLM ecosystem — community SDKs & tools

Because [`aphrody-serve`](./local-llm.md) speaks the **OpenAI protocol**, the
entire local-LLM tooling ecosystem works against it with no glue: point any
OpenAI client at `http://127.0.0.1:8088/v1`. This page curates the **best**
community SDKs/tools (sourced from the Ollama README, fetched 2026-06-28) and
flags how aphrody uses each.

## Point any OpenAI client at aphrody-serve

```bash
aphrody-serve --port 8088            # serving local open-weight models
```

| Client | Config |
|---|---|
| **openai SDK** (py/js) | `base_url="http://127.0.0.1:8088/v1"`, `api_key="local"` |
| **aichat** | `~/.config/aichat/config.yaml`: `clients: [{type: openai-compatible, name: aphrody, api_base: http://127.0.0.1:8088/v1, models: [{id: gemma4:12b}]}]` → `aichat -m aphrody:gemma4:12b "hi"` |
| **Continue** (IDE) | `config.json` model: `{provider: "openai", apiBase: "http://127.0.0.1:8088/v1", model: "gemma4:12b"}` |
| **Open WebUI** | env `OPENAI_API_BASE_URL=http://127.0.0.1:8088/v1`, `OPENAI_API_KEY=local` |
| **LiteLLM** | model `openai/gemma4:12b`, `api_base=http://127.0.0.1:8088/v1` |

## Direct integration targets (prioritised for aphrody)

| Project | Lang | Use in aphrody |
|---|---|---|
| **[ollama-rs](https://github.com/pepperoni21/ollama-rs)** 0.3.5 | Rust | SDK for a future `aphrody models` mgmt command + native Ollama features (see below). |
| **[aichat](https://github.com/sigoden/aichat)** | Rust | Design reference for `aphrody chat`/`run` (roles/sessions/RAG/tools); also a live verifier of `aphrody-serve`. |
| **[Open WebUI](https://github.com/open-webui/open-webui)** | — | Instant GUI on `aphrody-serve`; UX target for the `apps/web` GUI (M5). |
| **[LiteLLM](https://github.com/BerriAI/litellm)** | py | Reference for `/v1` endpoint + error coverage (M2/M3 parity). |
| **[RAGFlow](https://github.com/infiniflow/ragflow)** | py | RAG engine for the Python `aphrody.py` (under design). |

### ollama-rs (v0.3.5, MIT) — the Rust Ollama SDK

`cargo add ollama-rs` (feature `stream` for streaming). Tokio-based.

- **Client**: `Ollama::default()` (localhost:11434) / `Ollama::new(host, port)`.
- **Chat/Generate**: `send_chat_messages_with_history(...)`, `generate(GenerationRequest::new(model, prompt))`, `*_stream` variants.
- **Embeddings**: `generate_embeddings(GenerateEmbeddingsRequest::new(model, input))`.
- **Model management** (what the OpenAI proxy can't do): `list_local_models()`, `show_model_info()`, `create_model()`, `delete_model()`, `copy_model()`.
- **Tools/function-calling**: `Coordinator` + `#[ollama_rs::function]` macro.

→ **aphrody plan**: keep the OpenAI relay engine-agnostic for serving; adopt
ollama-rs only for the *native* surface (an `aphrody models pull/list/rm` CLI
command and structured model info). Not a server dependency.

### aichat (Apache-2.0/MIT) — the Rust LLM CLI to beat

Features: Shell Assistant, CMD + REPL, **Roles**, **Sessions**, **RAG**,
**Function Calling/MCP**, **Macros**, **AI Agents** ("instructions + tools +
documents"), and a built-in **local server** (`aichat --serve` → `/v1/chat/completions`,
`/v1/embeddings`, `/v1/rerank`, `/playground`, `/arena`). Install:
`cargo install aichat` or prebuilt release binary.

→ **aphrody plan**: aichat's roles/sessions/RAG/function-calling model is the
blueprint for `aphrody chat`/`run` (M4/M7). Its `--serve` arena/playground is a
UX idea for `aphrody serve`. Use it now as a real OpenAI client to regression-test
`aphrody-serve`.

## SDKs by language

- **Rust**: [ollama-rs](https://github.com/pepperoni21/ollama-rs) · [langchain-rust](https://github.com/Abraxas-365/langchain-rust) · [vtcode](https://github.com/vinhnx/vtcode) (terminal coding agent).
- **Unified gateways** (aphrody-serve's peers): [LiteLLM](https://github.com/BerriAI/litellm) · [any-llm](https://github.com/mozilla-ai/any-llm) · [Portkey](https://portkey.ai) · [Semantic Kernel](https://github.com/microsoft/semantic-kernel).
- **TS/JS**: official ollama-js · [LangChain.js](https://js.langchain.com/docs/integrations/chat/ollama/) · [LlamaIndexTS](https://ts.llamaindex.ai).
- **Python**: official ollama-python · [LangChain](https://python.langchain.com/docs/integrations/chat/ollama/) · [LlamaIndex](https://docs.llamaindex.ai) · [Haystack](https://github.com/deepset-ai/haystack-integrations).
- **Other**: [OllamaSharp](https://github.com/awaescher/OllamaSharp) (.NET) · [Ollama4j](https://github.com/ollama4j/ollama4j) (Java) · [Spring AI](https://github.com/spring-projects/spring-ai) · [ollama-swift](https://github.com/mattt/ollama-swift) · [Ollama-hpp](https://github.com/jmont-dev/ollama-hpp) (C++).

## Tools by category

- **Terminal/CLI**: **[aichat](https://github.com/sigoden/aichat)** · [gollama](https://github.com/sammcj/gollama) (TUI model manager) · [oterm](https://github.com/ggozad/oterm) · [ParLlama](https://github.com/paulrobello/parllama) · [tenere](https://github.com/pythops/tenere) · shell copilots [tlm](https://github.com/yusufcanb/tlm) / [ShellOracle](https://github.com/djcopley/ShellOracle) / [cmdh](https://github.com/pgibler/cmdh).
- **Code editors**: [Continue](https://github.com/continuedev/continue) · [Cline](https://github.com/cline/cline) · [Void](https://github.com/voideditor/void) · [twinny](https://github.com/rjmacarthy/twinny) · [gptel](https://github.com/karthink/gptel) (Emacs).
- **Chat UIs**: [Open WebUI](https://github.com/open-webui/open-webui) · [LibreChat](https://github.com/danny-avila/LibreChat) · [Lobe Chat](https://github.com/lobehub/lobe-chat) · [AnythingLLM](https://github.com/Mintplex-Labs/anything-llm) · [big-AGI](https://github.com/enricoros/big-AGI).
- **Agents**: [crewAI](https://github.com/crewAIInc/crewAI) · [Strands Agents](https://github.com/strands-agents/sdk-python) (AWS) · [any-agent](https://github.com/mozilla-ai/any-agent) · [AutoGPT](https://github.com/Significant-Gravitas/AutoGPT).
- **RAG**: **[RAGFlow](https://github.com/infiniflow/ragflow)** · [R2R](https://github.com/SciPhi-AI/R2R) · [MaxKB](https://github.com/1Panel-dev/MaxKB) · [Minima](https://github.com/dmayboroda/minima).
- **Observability**: [Langfuse](https://langfuse.com/docs/integrations/ollama) · [OpenLIT](https://github.com/openlit/openlit) (OTel + GPU) · [Opik](https://www.comet.com/docs/opik) · [MLflow Tracing](https://mlflow.org/docs/latest/llms/tracing/).
- **DB/Embeddings**: [pgai](https://github.com/timescale/pgai) (Postgres vectors) · [chromem-go](https://github.com/philippgille/chromem-go).
- **Infra**: [Harbor](https://github.com/av/harbor) (containerised LLM toolkit) · Cloud: [Google Cloud Run](https://cloud.google.com/run/docs/tutorials/gpu-gemma2-with-ollama), [Fly.io](https://fly.io/docs/python/do-more/add-ollama/), [Koyeb](https://www.koyeb.com/deploy/ollama).

## See also

[`local-llm.md`](./local-llm.md) — the server & backend setup.
