<!-- SPDX-License-Identifier: Apache-2.0 -->
# IEVR ML environment audit + refactor — 2026-05-17

Read-only audit of the local ML stack with a refactor proposal aimed at **IEVR
binary analysis** (static + optional dynamic). Mission constraint: **local-only,
no cloud APIs**.

## Current state

### Python
- **Version**: 3.14.5 at `C:\Users\<user>\AppData\Local\Programs\Python\Python314\python.exe`
- **`python3` alias**: hijacked by Windows Store stub (not usable for our work).
- **Secondary interpreter**: `C:\winclean\.venv\Scripts\python.exe` (peer repo venv, off-limits).
- **Pip packages installed**: 167 total. RE-relevant subset:
  - `angr 9.2.214` + `claripy 9.2.214` + `cle 9.2.214` + `archinfo 9.2.214` — symbolic execution + binary loader.
  - `capstone 5.0.6` — multi-arch disassembler.
  - `keystone-engine 0.9.2` — multi-arch assembler.
  - `unicorn 2.1.4` — CPU emulator.
  - `pyghidra 3.0.2` + `ghidra-stubs 12.0.4` + `ghidradrgn 12.0` + `ghidragdb 12.0` + `ghidralldb 12.0` + `ghidratrace 12.0` — Ghidra Python bridge + stubs.
  - `cxxheaderparser 1.7.0`, `bitstring 4.4.0`, `bitarray 3.8.1` — binary-format scaffolding.
- **Missing for ML**: `torch`, `transformers`, `numpy`, `pandas`, `scikit-learn`, `jupyter`, `faiss-cpu`/`faiss-gpu`, `frida`, `binaryai`, `asm2vec`, `palmtree`.
- **Conda**: not installed.
- **pyenv**: not installed.

### GPU
- **Device**: NVIDIA GeForce RTX 3050 Laptop (4 GB VRAM).
- **Driver**: 596.36 (CUDA runtime 13.2 advertised by driver).
- **CUDA toolkit (`nvcc`)**: **not installed** — only driver-level CUDA available.
- **PyTorch CUDA**: N/A — torch not installed; `python -c "import torch"` → `ModuleNotFoundError`.
- **Current GPU load at audit time**: 70 MiB / 4096 MiB (idle, DWM + Chrome compositing only).

### Storage
- **C:** 471 GB used / **39 GB free** — primary OS + IEVR install (per `iecode-public-endpoints.md`) lives here. Tight margin for large embedding caches.
- **D:** 37 GB used / **69 GB free** — recommended target for ML model cache + embeddings index.
- **ML cache target (recommended)**: `D:\ievr-ml\cache\` (POSIX-style mount: `/d/ievr-ml/cache/`).

### Existing ML-for-binary tools (binary on PATH)
- **Ghidra binary**: not on PATH and not found at `C:\Program Files\ghidra` or `C:\Tools\ghidra*`. `pyghidra` bridge is installed — Ghidra runtime must be installed separately to make `pyghidra` actually work.
- **Radare2**: `C:\ProgramData\chocolatey\bin\radare2.exe` (also `r2.exe`). Standalone `C:\Tools\radare2\radare2-6.1.4-w64\` build present.
- **Frida**: not installed (no CLI on PATH, no `frida-tools` pip pkg).
- **BinaryAI / asm2vec / palmtree**: none installed.
- **angr**: installed (symbolic execution available immediately).

## Refactor proposal: IEVR-focused stack

### Static analysis pipeline (primary)
1. **Install Ghidra runtime** (11.x or 12.x to match `ghidra-stubs 12.0.4` already in pip) at `D:\ievr-ml\ghidra\` — keeps the 4 GB Ghidra tree off C:.
2. **Ghidra headless batch disasm** — `analyzeHeadless` on IEVR `.exe` + `.dll`, projects written to `D:\ievr-ml\projects\`. Driven by `pyghidra` so we stay in one Python world.
3. **CFG + function metadata export** — Python plugin emits per-function JSON (mnemonics, basic blocks, xrefs, calling convention) under `D:\ievr-ml\cfg\<sha256>\funcs.jsonl`.
4. **PyTorch embedding step** — install `torch==2.x` CPU build first (GPU optional; 4 GB VRAM is tight for transformer-class models). Use an asm2vec-style or PalmTree-style pretrained checkpoint to embed each function vector (256–768 dims). Cache embeddings at `D:\ievr-ml\embeddings\<sha256>.npy`.
5. **Similarity index** — `faiss-cpu` `IndexFlatIP` or `scikit-learn` `NearestNeighbors` on the embedding matrix. Index file at `D:\ievr-ml\index\ievr-funcs.faiss`.
6. **Query CLI** — small wrapper: `ievr-ml query --snippet <asm-or-decompile.txt> --top-k 10` → returns top-k matching IEVR functions with xrefs back to Ghidra.

### Dynamic analysis pipeline (optional, opt-in)
1. `pip install frida-tools` (~30 MB) — locally-running CLI, no telemetry.
2. Frida JS hooks generated from static-pipeline candidates (export targeted symbols).
3. Capture call traces + arg dumps to `D:\ievr-ml\traces\<run-id>.jsonl`.
4. Re-feed traces into the embedding pipeline (runtime-confirmed call graphs sharpen similarity ranking).

### GPU strategy
- 4 GB VRAM is **enough for inference** of small embedding models (asm2vec, fastText-class, 100–200 MB checkpoints) but **not for training** transformer-class models. Use CPU for any training/fine-tuning; reserve GPU for batch embedding only.
- Install order: `torch` CPU first (smoke test the pipeline), then `torch+cu13` only after verifying disk budget on D:.
- Install `nvcc` (CUDA Toolkit 13.x) only if we end up writing custom CUDA kernels — not needed for off-the-shelf PyTorch.

### Disk strategy
- All caches under `D:\ievr-ml\` (≈30–40 GB budget):
  - `ghidra/` — runtime (4 GB).
  - `projects/` — Ghidra .gpr/.rep (5–10 GB).
  - `cfg/` — per-binary JSONL (1–2 GB).
  - `embeddings/` — float32 vectors (1–5 GB).
  - `index/` — faiss index (<1 GB).
  - `models/` — pretrained checkpoints (1–2 GB).
- Leave C: free for OS + Visual Studio + actively-played IEVR installs.

## Out-of-scope (by design)
- **Cloud APIs** (OpenAI, Anthropic, Tencent BinaryAI public endpoint, etc.) — mission rule: no external data transfer for game RE work.
- **Anti-tamper bypass** — legal grey area. Pipeline operates strictly on **unpacked** binaries (post-DRM-stripping done by separately-authorized tooling).
- **Cloud GPU rentals** — local-only; the 4 GB RTX 3050 is the ceiling.
- **node-based ML stacks** (per repo `feedback_bun_only` policy) — Python ML, Rust pipeline glue, Bun only for surrounding scripting if needed.

## Action items
- [ ] Install Ghidra 12.x runtime to `D:\ievr-ml\ghidra\` (pip stubs already match).
- [ ] Create `D:\ievr-ml\{cache,projects,cfg,embeddings,index,models,traces}\` skeleton.
- [ ] `pip install torch --index-url https://download.pytorch.org/whl/cpu` (CPU first, ≈250 MB).
- [ ] `pip install numpy pandas faiss-cpu transformers` (≈400 MB combined).
- [ ] `pip install frida-tools` (optional, ≈30 MB; deferred until static pipeline is green).
- [ ] Acquire one pretrained binary-embedding checkpoint (PalmTree or asm2vec-style) and stage under `D:\ievr-ml\models\`.
- [ ] Stand up `pyghidra` headless smoke-test on a single IEVR `.dll`, dump first 10 function CFGs as JSONL.
- [ ] Wire the smoke-test output through a stub embedder → write `embeddings/<sha256>.npy`.
- [ ] Build first faiss index from the stub embeddings, run a self-query (sanity check: top-1 hit = self).
