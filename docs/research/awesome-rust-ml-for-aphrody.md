<!-- SPDX-License-Identifier: Apache-2.0 -->

# Awesome-Rust-MachineLearning, curated for aphrody

Curated comparison of the highest-value Rust ML crates for the aphrody
workspace. This is an original analysis written for aphrody's actual needs
(file-type classification, embeddings, vector recall, tokenization), not a
reproduction of the source list.

## Attribution and source

- Upstream catalogue: **Awesome-Rust-MachineLearning** by vaaaaanquish et al.
  - Source URL: <https://github.com/vaaaaanquish/Awesome-Rust-MachineLearning>
  - Raw README ingested: <https://raw.githubusercontent.com/vaaaaanquish/Awesome-Rust-MachineLearning/main/README.md>
  - License of the upstream list: that repository is published under its own
    terms (an awesome-list of links). The RAW upstream file is kept locally,
    gitignored, at `var/data/awesome-rust-ml/README.upstream.md` and is NOT
    committed verbatim to avoid redistributing a third-party document.
- Date of analysis: 2026-05-21. Crate versions/licenses below were
  fact-checked against crates.io on that date (per CLAUDE.md §2.5).

> Caveat on the upstream list: it is link-curated but partly stale for the
> 2026 inference landscape. It still leads with `tensorflow/rust`, `tch-rs`
> and `tract`, and predates the rise of `candle`, the `ort` 2.x line,
> `fastembed`, `burn`, and `lancedb`. The recommendations below correct for
> that and prioritise crates that are (a) Apache-2.0 compatible, (b) actively
> maintained in 2026, and (c) buildable on aphrody's platform priorities
> (Linux #1, Windows #2, WASM #3).

## License gate (aphrody is Apache-2.0)

aphrody must avoid GPL/AGPL contamination. Every crate recommended below is
`MIT`, `Apache-2.0`, or `MIT OR Apache-2.0` — all compatible. Crates that are
GPL, that wrap GPL native libraries, or that drag in `native-tls`/OpenSSL on
the Linux server target are flagged and rejected.

---

## 1. Inference runtime: candle vs ort/tract vs tch

| Crate | Version (2026-05-21) | License | Native dep | aphrody fit |
|-------|----------------------|---------|------------|-------------|
| `ort` | `2.0.0-rc.12` | MIT OR Apache-2.0 | ONNX Runtime (prebuilt fetched at build, or system) | **Already used** (via `magika`, see §6) |
| `candle-core` | `0.10.2` | MIT OR Apache-2.0 | none (pure Rust + optional CUDA/Metal) | Strong fit for custom transformer inference |
| `tract` / `tract-onnx` | `0.22.x` | MIT OR Apache-2.0 | none (pure Rust) | Best fit for hermetic-offline + WASM ONNX |
| `tch` (tch-rs) | `0.x` | MIT OR Apache-2.0 | libtorch (large C++ runtime) | Rejected: heavyweight, non-hermetic |
| `wonnx` | `0.x` | MIT OR Apache-2.0 | none (WebGPU) | Niche: GPU ONNX in browser, immature |

Analysis:

- **`ort`** is the pragmatic production choice and is already in the graph.
  It wraps Microsoft's ONNX Runtime (v1.24 in the rc.12 line). Trade-off: the
  `download-binaries` feature fetches a prebuilt runtime at build time, which
  is **not hermetic-offline and not WASM-capable**. aphrody already isolates
  this correctly behind an opt-in feature (see §6).
- **`candle`** (Hugging Face) is the best choice for running open-weight
  transformer models (Llama, BERT, embedding models) directly in Rust without
  a C++ runtime. Pure Rust core, optional CUDA/Metal acceleration. It is the
  recommended path if aphrody ever needs in-process LLM/embedding inference
  beyond what `fastembed` provides.
- **`tract`** (Sonos) is the right tool when hermetic-offline or `wasm32`
  builds are mandatory: 100% Rust, no prebuilt runtime download, self-contained
  ONNX/TF inference. It is the natural fallback for a future WASM-target
  classification path where `ort` cannot go.
- **`tch`** is rejected for aphrody: it links libtorch (hundreds of MB of C++),
  breaks the hermetic build, and conflicts with the "no C/C++ in distribution"
  policy (CLAUDE.md §2).

Recommendation: keep `ort` for the magika path; reach for `candle` for any new
in-process model inference; keep `tract` in mind as the WASM/hermetic fallback.

## 2. Embeddings: fastembed

| Crate | Version | License | Native dep |
|-------|---------|---------|------------|
| `fastembed` | `5.13.4` | Apache-2.0 | ONNX Runtime via `ort` (shared with magika) |

`fastembed` is the highest-leverage addition for aphrody. It produces dense
sentence embeddings (BGE, E5, Nomic, etc.) from a single crate, runs on `ort`
(the runtime aphrody already ships for magika, so no new native dependency
family), and is Apache-2.0. This directly feeds `aphrody-memory`'s vector
recall: today the HNSW module is a hand-rolled brute-force placeholder and the
LanceDB backend stores vectors that must be produced somewhere. `fastembed` is
the natural local, offline-capable embedding producer to close that gap.

Recommendation: strong candidate for `aphrody-memory` to generate embeddings
locally rather than calling a remote provider. Not yet wired (see §6).

## 3. Tokenizers: tokenizers vs tiktoken-rs

| Crate | Version | License | Use |
|-------|---------|---------|-----|
| `tokenizers` (Hugging Face) | `0.22.2` | Apache-2.0 | WordPiece/BPE/Unigram for transformer models |
| `tiktoken-rs` | `0.11.0` (0.9.x widely used) | MIT | GPT/OpenAI token counting & BPE |

- **`tokenizers`** is the canonical choice when pairing with `candle`/`fastembed`
  models — it loads the exact `tokenizer.json` shipped with HF models.
- **`tiktoken-rs`** is the right tool for *counting* tokens against
  OpenAI/GPT and Anthropic-adjacent budgets (context-window accounting in
  `aphrody-chat`, `aphrody-sdk`, terminal LLM crates). Cheaper and narrower
  than `tokenizers`.

Recommendation: `tiktoken-rs` for context-budget accounting in the LLM-facing
crates; `tokenizers` only if/when aphrody runs HF models locally. Neither is
wired today.

## 4. ONNX: ort (already used)

Covered in §1. `ort 2.0.0-rc.12`, MIT OR Apache-2.0. This is aphrody's only
real neural-inference dependency today, pulled transitively by `magika` and
declared as an optional direct dep in `crates/cli/Cargo.toml`
(`ort = { version = "=2.0.0-rc.12", optional = true }`) behind the `magika`
feature. The pin is exact (`=2.0.0-rc.12`) because the 2.x line is a release
candidate (production-ready but not API-stable). Verified building offline
(see §6 test results).

## 5. Vector search: lancedb vs qdrant vs in-process ANN

| Crate | Version | License | Shape |
|-------|---------|---------|-------|
| `lancedb` | `0.29` (workspace pin) | Apache-2.0 | Embedded serverless vector DB on Arrow/Lance | **Already used** |
| `qdrant-client` | `1.18.0` | Apache-2.0 | Client for an external Qdrant server |
| `instant-distance` | n/a | Apache-2.0 | Pure-Rust HNSW, **not in offline registry cache** |
| `hnsw` (rust-cv) | n/a | MIT | Pure-Rust HNSW index |

- **`lancedb`** is already the embedded vector backend in `aphrody-memory`
  (`lancedb = { workspace = true }`, `default-features = false` to drop the
  aws/azure/gcs cloud transitives). Apache-2.0, embedded, no server process —
  the right call for a single-binary CLI.
- **`qdrant-client`** is Apache-2.0 and the choice only if aphrody needs to
  talk to an *external* Qdrant cluster. For aphrody's single-binary model,
  embedded LanceDB is preferred over running a separate vector server.
- **`instant-distance`** would be the pure-Rust HNSW upgrade for the current
  brute-force `aphrody-memory::hnsw` module, but it is documented as **not
  present in this workspace's offline registry cache** (see the comment in
  `crates/aphrody-memory/Cargo.toml`), so it cannot be added under the
  hermetic-offline build today without first populating the cache.

Recommendation: keep LanceDB. Pair it with `fastembed` (§2) for the embedding
half. Defer the HNSW swap until the offline cache carries `instant-distance`.

## 6. What aphrody already uses (workspace grep, 2026-05-21)

Grepped the workspace `Cargo.toml` files for `candle`, `ort`, `tokenizers`,
`fastembed`, `tract`, `tch`, `lancedb`, `qdrant`, `magika`:

| Crate | Status | Where |
|-------|--------|-------|
| `magika` | **WIRED** (opt-in `magika` feature) | `crates/aphrody-re/Cargo.toml` (`magika = "1.1.0"`), impl in `crates/aphrody-re/src/magika.rs` |
| `ort` | **WIRED** (transitive via `magika` + optional direct dep) | `crates/cli/Cargo.toml` (`=2.0.0-rc.12`, `magika` feature) |
| `lancedb` | **WIRED** | workspace `Cargo.toml` (`0.29`), `crates/aphrody-memory/Cargo.toml` |
| `arrow-array` / `arrow-schema` | **WIRED** (LanceDB backend) | `crates/aphrody-memory/Cargo.toml` |
| `candle` | not used | — (recommended for future in-process inference) |
| `tract` | not used | — (recommended as WASM/hermetic ONNX fallback) |
| `fastembed` | not used | — (recommended for `aphrody-memory` embeddings) |
| `tokenizers` | not used | — |
| `tiktoken-rs` | not used | — (recommended for LLM context budgeting) |
| `qdrant-client` | not used | — (only if external Qdrant ever needed) |
| `tch` | not used | rejected (libtorch / non-hermetic) |

The magika path is the only neural-ML code currently compiled into aphrody.
`MagikaClass` / `Classifier` (`crates/aphrody-re/src/magika.rs`) projects
Magika's `FileType` + `TypeInfo` into an owned, serde-serializable result and
runs the embedded ONNX model through `ort`. The feature is correctly host-only
and opt-in because `ort`'s prebuilt-runtime download is incompatible with the
hermetic-offline and `wasm32` builds.

## 7. Build test results ("test everything")

Command run on Windows (host), offline:

```
cargo check -p aphrody-re --features magika --offline
```

Result: **GREEN, exit 0.** The full ML chain compiled from the offline
registry cache:

- `ort-sys v2.0.0-rc.12` compiled
- `ort v2.0.0-rc.12` checked
- `ndarray v0.17.2`, `num-*`, `matrixmultiply v0.3.10` checked (tensor I/O)
- `magika v1.1.0` checked
- `aphrody-re v1.0.0-canary` checked
- Finished in ~12s, no errors (only unrelated workspace `profile package spec
  ... did not match any packages` warnings from the root profile config).

This confirms the existing ONNX/ort + magika ML path builds cleanly offline on
the host. No `Cargo.toml` was modified and no new dependency was added during
this research task.

## 8. Recommendation summary

1. **Keep**: `ort` (2.0.0-rc.12) + `magika` (1.1.0) for file-type
   classification; `lancedb` (0.29) for embedded vector storage. All
   Apache-2.0-compatible, all green offline.
2. **Add when needed (highest value first)**:
   - `fastembed` (5.13.4, Apache-2.0) — local embeddings for `aphrody-memory`,
     reuses the `ort` runtime already shipped, no new native family.
   - `candle-core` (0.10.2, MIT OR Apache-2.0) — in-process transformer
     inference without a C++ runtime, if aphrody runs open-weight models.
   - `tiktoken-rs` (0.11.0, MIT) — token budgeting in the LLM-facing crates.
   - `tract` (0.22.x, MIT OR Apache-2.0) — pure-Rust ONNX fallback for the
     `wasm32` / hermetic-offline target where `ort` cannot run.
3. **Defer**: swapping the brute-force `aphrody-memory::hnsw` for
   `instant-distance` until it lands in the offline registry cache.
4. **Reject**: `tch`/libtorch (non-hermetic, C++), and any GPL/AGPL ML crate
   (aphrody is Apache-2.0). No native-tls/OpenSSL drift was found among the
   recommended crates — all use the workspace rustls path or have no TLS dep.

## References (crates.io, fact-checked 2026-05-21)

- candle-core — <https://crates.io/crates/candle-core> (0.10.2, MIT OR Apache-2.0)
- ort — <https://crates.io/crates/ort> (2.0.0-rc.12, MIT OR Apache-2.0)
- tract-onnx — <https://crates.io/crates/tract-onnx> (0.22.x, MIT OR Apache-2.0)
- fastembed — <https://crates.io/crates/fastembed> (5.13.4, Apache-2.0)
- tokenizers — <https://crates.io/crates/tokenizers> (0.22.2, Apache-2.0)
- tiktoken-rs — <https://crates.io/crates/tiktoken-rs> (0.11.0, MIT)
- lancedb — <https://crates.io/crates/lancedb> (0.29 pinned; Apache-2.0)
- qdrant-client — <https://crates.io/crates/qdrant-client> (1.18.0, Apache-2.0)
- magika — <https://crates.io/crates/magika> (1.1.0, Apache-2.0)
- tch — <https://crates.io/crates/tch> (MIT OR Apache-2.0; rejected for non-hermetic libtorch)
