<!-- SPDX-License-Identifier: Apache-2.0 -->
# The Aphrody RAG/LLM Pattern — unified contract for downstream bots

**Status:** canonical. **Verified:** 2026-06-04 against `rpbey` + `shenron` source.

`rpbey` (Beyblade) and `shenron` (Dragon Ball) independently converged on the
**same** retrieval architecture. This document promotes that convergence to a
**contract**: the parts both bots already share become normative, the parts
that diverge get one canonical resolution, and the heavy jobs get a
resource-aware execution profile so they exploit the VPS without thrashing it.

It complements — does not replace — [`RAG.md`](./RAG.md) (aphrody's own Python
RAPTOR/GraphRAG ingestion pipeline) and [`rpbey-rag/`](./rpbey-rag/README.md)
(rpbey's per-surface deep dive). This file is the **cross-repo seam**.

---

## 1. The shared pattern (already true in both repos — now normative)

| Layer | Contract | rpbey | shenron |
|---|---|---|---|
| **Embedding model** | `Xenova/multilingual-e5-small`, **384 dims**, ONNX q8, multilingual (FR/EN/JP) | `apps/embed-sidecar/server.ts:22` | `apps/bot/src/lib/embeddings.ts:27` |
| **E5 prefix convention** | `query:` for queries, `passage:` for documents; mean-pool; **L2-normalize** (so dot product = cosine) | yes | yes |
| **Process isolation** | embeddings run in a **sidecar** HTTP service, never in the main bot process | port `7077` | port `5007` |
| **Env override** | `EMBED_MODEL`, `EMBED_PORT`, `EMBED_URL` | yes | yes (`EMBED_PORT`) |
| **Retrieval** | **hybrid**: lexical (BM25 / FTS5) ∪ dense (cosine top-K) fused by **RRF, k=60** | `lib/search-rank.ts` | `apps/bot/src/lib/rag.ts:186` |
| **Degradation** | best-effort: sidecar timeout → drop dense → **lexical-only never crashes** | 2-tier | 3-tier (`rag.ts:224`) |
| **Corpus ingestion** | `bxc recon <url> --profile <p>` (HTML→Markdown) + `bxc scrape … --json`, with `bun run bxc` PATH fallback | `apps/bot` scrapers | `apps/bot/scripts/rag-recon.ts:85` |
| **Generation** | extractive-first (zero-hallucination); LLM is an **optional** layer behind a seam (§4) | LLM removed 2026-06-01 | B-series roadmap |

**Normative rules** (a downstream bot is "pattern-conformant" iff):

1. It embeds with `multilingual-e5-small` @ 384d, e5 prefixes, L2-normalized.
2. Embeddings/reranking live in a sidecar reachable over the §3 HTTP contract.
3. Retrieval is hybrid lexical∪dense via RRF (k=60), degrading to lexical-only.
4. Corpus comes through `bxc` (so crawling, impersonation, caching are shared).
5. Generation, if present, goes through the §4 OpenAI-compatible seam — no
   bot hardcodes a provider SDK in its retrieval path.

---

## 2. The three divergences and their canonical resolution

### 2.1 Vector store — Redis VSIM vs SQLite blob
- rpbey: Redis vector set `rpbey:search:vec` (`VADD`/`VSIM`, FP32), ~17k items × 384d ≈ 26 MB.
- shenron: SQLite `rag_vectors(rowid, vec BLOB)` + in-memory `Float32Array`, brute-force cosine, ~1k chunks ≈ 1.6 MB.

**Resolution — both are valid backends behind one interface.** The store is a
strategy, not the contract. Pick by corpus size:
- **< ~5k chunks** → SQLite blob + in-memory brute-force (shenron). Zero infra, ~30 ms/query, rebuild is a single file.
- **≥ ~5k chunks or multi-process readers** → Redis VSIM (rpbey). Shared across web+bot, no per-process vector load.
Both expose the same logical op: `topK(queryVec, k) -> [{id, score}]`. Document which backend a repo uses in its `rag_meta`/corpus header.

### 2.2 Reranking — present (shenron) vs absent (rpbey)
- shenron: cross-encoder `Xenova/bge-reranker-base`, top-15, sigmoid → [0,1].
- rpbey: no rerank (RRF order is final).

**Resolution — rerank is part of the canonical sidecar (§3) and recommended.**
It is the cheapest precision win available (one extra sidecar call, best-effort,
falls back to RRF order on timeout). rpbey SHOULD adopt `/rerank`; the sidecar
contract below mandates the endpoint exists even if a caller skips it.

### 2.3 Generation — removed (rpbey) vs roadmap (shenron)
- rpbey: local LLM removed 2026-06-01; extractive synthesis; Gemini only for X-metagame RAG.
- shenron: B1–B5 roadmap (Gemini via aphrody → distilled SFT → local GGUF).

**Resolution — generation is optional and lives behind the §4 seam.** A
conformant bot ships extractive answers by default and may enable an LLM rewrite
by pointing the seam at the aphrody gateway. No bot couples retrieval to a
provider.

---

## 3. Canonical embedding/rerank sidecar HTTP contract

Both sidecars already speak a near-identical dialect; this freezes it. A sidecar
MUST implement (loopback-only, JSON):

```
GET  /health
  -> 200 {"ok":true,"model":"Xenova/multilingual-e5-small","dim":384,"rerank":"Xenova/bge-reranker-base"}

POST /embed       {"texts": string[], "kind": "query" | "passage"}
  -> 200 {"vectors": number[][]}        # L2-normalized, length == texts.length, each dim==384
  # server applies the e5 prefix from `kind`; caller sends raw text.
  # MUST cap batch at 64; caller chunks larger inputs.

POST /rerank      {"query": string, "passages": string[]}
  -> 200 {"scores": number[]}           # [0,1], length == passages.length
  # MUST cap at 64 passages; truncate each to ~400 chars / 512 tokens.
```

Client rules (normative): embed timeout **3 s**, rerank timeout **6 s**, both
best-effort (on failure, skip that signal — never throw into the request path).
Sidecar lazy-loads models on first call and warms up at boot with a 1-text
`/embed`. `EMBED_PORT` defaults: rpbey 7077, shenron 5007 (env-overridable).

A future shared implementation (`@aphrody-code/embed-sidecar`) can satisfy this
contract for both; until then, each repo's sidecar is conformant as-is.

---

## 4. LLM generation seam (OpenAI-compatible)

Generation is wired through one seam, already named in [`rpbey-rag/llm.md`](./rpbey-rag/llm.md):

```
RPBEY_LLM_URL   # OpenAI-compatible base URL (e.g. aphrody gateway, llama.cpp --api, vLLM)
RPBEY_LLM_MODEL # model id
```

- **Default (no URL set):** extractive synthesis from retrieved passages. Deterministic, zero-hallucination, citations intact. This is the shipped behaviour in both bots today.
- **aphrody gateway:** point the seam at the aphrody backend (`aphrody antigravity chat` / Gemini) for grounded rewriting. shenron B1 targets exactly this.
- **Local model:** point the seam at a llama.cpp/vLLM `--api` server (shenron B5: `shenron-llm.service`, GGUF q4_k_m). Same contract, no code change.

**Training/distillation** (shenron B3–B4) reuses the seam in reverse: batched
gateway calls produce an SFT dataset (`data/llm/*-sft.jsonl`), fine-tune a 2–3B
model (LoRA, rented GPU), serve the GGUF behind the same seam. The dataset
schema is `{messages:[{role,content}], meta:{sources:[...]}}` — sources are the
RAG passages, so every training example is grounded and auditable.

---

## 5. Resource-aware execution profile (exploit RAM/CPU without thrashing)

The VPS is **12 cores / 45 GiB**, frequently shared by several agents (Claude,
Grok, the winclean C# peer) plus the always-on `bxc-crawler`. Heavy RAG jobs
(embedding rebuilds, dataset distillation) MUST yield to interactive work.

**Canonical wrapper — run any heavy RAG/embedding/training job through this:**

```bash
# scripts/rag-nice.sh — CPU+IO de-prioritized, core-capped, single-flight
#   usage: rag-nice.sh <lockname> <cmd...>
set -euo pipefail
LOCK="/tmp/rag-${1}.lock"; shift
# single-flight: never two heavy RAG jobs at once (LTO-style discipline)
exec 9>"$LOCK"; flock -n 9 || { echo "rag job '${LOCK}' already running"; exit 0; }
# leave ~1/3 of cores for interactive agents; cap ONNX threads
CORES=$(nproc); export OMP_NUM_THREADS=$(( CORES/3>0 ? CORES/3 : 1 ))
export ORT_NUM_THREADS="$OMP_NUM_THREADS" UV_THREADPOOL_SIZE="$OMP_NUM_THREADS"
exec nice -n 15 ionice -c2 -n7 "$@"
```

**Sidecar systemd hardening (both repos already cap memory — make it uniform):**

```ini
# *-embed.service
[Service]
MemoryMax=3G            # ONNX e5+reranker fit well under this
CPUWeight=30            # yields to interactive (default 100)
IOWeight=30
Nice=10
IPAddressDeny=any
IPAddressAllow=localhost
ProtectSystem=strict
```

**Scheduling rules:**
- Embedding rebuilds are **off-peak timers**, never inline. Persistent=true so a
  missed slot runs once on boot, not on every wake. shenron rebuild ≈ 85 s for
  ~1k chunks; rpbey vector rebuild every 2 h — both belong behind `rag-nice.sh`.
- **Incremental over full rebuild** where possible: hash each chunk
  (`sha256(content)`), re-embed only changed rows, bump `rag_meta.built_at`.
  This is shenron PLAN A7 and the single biggest CPU saving — a content edit
  re-embeds 1 row, not 1041.
- **Batch = 64** everywhere (the sidecar cap). Larger batches blow ONNX memory;
  smaller wastes the warm model.
- **Single-flight**: never run two embedding rebuilds concurrently (the `flock`
  above). Mirrors the repo rule "one `cargo build --release` at a time".
- Prefer **Redis/SQLite reads** to re-embedding: a query embed is one 384d
  vector; cache query embeddings for hot queries (TTL 1 h).

**RAM budget (measured):** e5-small ONNX ≈ 500 MB resident, bge-reranker ≈ 400 MB
→ a sidecar with both ≈ 1 GB steady, 3 GB ceiling. Redis vector set ≈ 4 bytes ×
384 × N items (26 MB @ 17k). Keep corpora in Redis/SQLite, models in the sidecar,
nothing vector-heavy in the bot heap.

---

## 6. Conformance checklist (per downstream bot)

- [ ] e5-small @ 384d, e5 prefixes, L2-normalized embeddings.
- [ ] Sidecar implements §3 (`/health` `/embed` `/rerank`), loopback-only, memory-capped.
- [ ] Hybrid retrieval = lexical ∪ dense via RRF(k=60), degrading to lexical-only.
- [ ] `/rerank` available (bge-reranker-base), used best-effort.
- [ ] Corpus ingested via `bxc recon`/`scrape` (shared crawler, not a bespoke fetcher).
- [ ] Generation (if any) behind the §4 `*_LLM_URL` OpenAI-compatible seam; extractive default.
- [ ] Heavy jobs wrapped in `rag-nice.sh`; rebuilds are off-peak `Persistent=true` timers; incremental where the store allows.

rpbey gaps vs this contract: **adopt `/rerank`** (2.2). shenron gaps: **incremental
rebuild** (5, PLAN A7) and **generation seam** (B1, not yet shipped). Everything
else already conforms.
