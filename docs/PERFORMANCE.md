<!-- SPDX-License-Identifier: Apache-2.0 -->
# Performance

Performance claims for `aphrody` with reproducible recipes. Complements
[`../BENCHMARKS.md`](../BENCHMARKS.md) (mrx scanner focus) by covering the
wider workspace: CLI cold start, criterion micro-benchmarks in `backend`,
A2A coordination latency, WASM artifact size, and memory footprint.

Numbers are honest priors — measured on a 2024-class x86_64 laptop (~4.0 GHz,
AES-NI, NVMe SSD), Windows 11 Insider Canary, Rust nightly 1.97 release
profile (`lto = "fat"`, `codegen-units = 1`). Your numbers will differ. That
is the point of running the recipes below yourself.

## 1. Headline claims (honest)

- `aphrody doctor` cold start: **< 100 ms** on a warm filesystem.
- `aphrody --version`: **< 30 ms** cold, **< 10 ms** warm.
- `mrx scan` on this repo (~19,213 files, 482 MB): **~1.4 s** warm cache,
  **~3 s** cold (see [`../BENCHMARKS.md`](../BENCHMARKS.md) for the full table).
- AES-256-GCM decrypt of a 1 KiB Chromium-format payload: **< 50 µs**.
- A2A `file_jsonl` inbox poll with 100 envelopes (~50 KB): **< 5 ms**.
- WASM bundle (`aphrody-wasm` release, `wasm-pack --target web`): **~91 KB**
  gzip-compressed.

These are floors and ceilings the team verifies in CI, not marketing wins.

## 2. How to reproduce

```bash
git clone https://github.com/aphrody-code/aphrody
cd aphrody
cargo build --release -p aphrody --locked --offline

# CLI startup (use /usr/bin/time on Linux, Measure-Command on PowerShell)
/usr/bin/time -f '%e s' ./target/release/aphrody --version
/usr/bin/time -f '%e s' ./target/release/aphrody doctor

# Criterion micro-benches (warm-up + sample + plot)
cargo bench -p backend

# Quick smoke pass (one sample per bench, no plots)
cargo bench -p backend -- --quick
```

Criterion writes HTML reports under `target/criterion/<bench_name>/report/`.
Open `index.html` for distribution, regression vs previous run, violin plots.

## 3. Bench results (criterion runs)

The five micro-benches in
[`crates/backend/benches/backend_bench.rs`](../crates/backend/benches/backend_bench.rs)
exercise the algorithmic primitives that dominate hot paths in the backend
crate. Expected ranges on the reference laptop:

| Bench name | Expected range | Notes |
|---|---:|---|
| `vfs_resolve` | ~200 ns/op | HashMap lookup + prefix match + `PathBuf::push` chain. Zero syscalls. |
| `dns_dedup_sort` | ~5 µs/op | 20-entry domain list: lowercase, filter, sort, dedup. |
| `aes_gcm_decrypt_1kb` | ~10-50 µs/op | RustCrypto `aes-gcm` decryption of a 1 KiB Chromium-format ciphertext (3-byte prefix + 12-byte nonce + payload + tag). Lower bound assumes AES-NI; pure software path lands higher. |
| `sha256_hash_1mb` | ~3-5 ms/op | `sha2` crate, ~250 MB/s sustained throughput. |
| `serde_json_parse_crtsh` | ~10 µs/op | `serde_json::from_slice` on a 20-entry crt.sh-shaped payload. |

Hardware without AES-NI (or ARM without crypto extensions) will see
`aes_gcm_decrypt_1kb` drift up. File an issue if your numbers are an order
of magnitude off — that usually points at a build-flag regression.

## 4. mrx scan performance

`mrx` walks polyglot monorepos and emits two JSON snapshots in one shot.
Numbers below are reproduced from [`../BENCHMARKS.md`](../BENCHMARKS.md) and
the [aggressive scan audit](audits/2026-05-17-mrx-aggressive.md):

- Target repo: **19,213 files, 482 MB**, 53 workspaces, 9 submodules.
- Warm cache (median of 3 consecutive runs): **1.4 s** wall clock.
- Cold cache (first run, fs cache empty): **~3 s** wall clock.
- Output artifacts: `path.json` and `monorepo-map.json`, ~50 KB each.
- Throughput: ~14,000 files/s, ~351 MB/s including content classification
  and a blake3 root-config hash.

Methodology:

```bash
cargo build --release -p mrx-cli
for i in 1 2 3; do
    /usr/bin/time -f '%e s' ./target/release/mrx --root . scan \
        --out path.json --map monorepo-map.json
done
```

Take the median of the warm runs (#2 and #3) — the first run pays the cold
filesystem cost.

## 5. A2A coordination performance

The A2A protocol is file-based (JSONL mailbox) with an optional HTTP listener
for liveness checks. Latency numbers on localhost:

- Inbox poll (`file_jsonl` backend): **O(file_size)**. A 100-envelope inbox
  (~50 KB JSONL) parses in **< 5 ms**.
- HTTP listener round-trip (port 8788, `/ping`): **2-10 ms** on localhost.
- Heartbeat poll (single `stat()` on `heartbeat-*.txt`): **< 1 ms**.
- 3-deep handshake (round-trip × 3): **~30 ms** localhost, **~100-300 ms**
  across machines depending on network RTT.

The listener is a Bun script and the Rust side reads files directly — no IPC
channel, no broker, no message bus. Cross-host coordination is bounded by
SSH round-trip plus disk fsync.

## 6. WASM bundle

`wasm-pack build --target web --release crates/aphrody-wasm` produces:

| Artifact | Size | Role |
|---|---:|---|
| `aphrody_wasm_bg.wasm` | **~91 KB gzip** (~250 KB uncompressed) | Rust core compiled to wasm32 |
| `aphrody_wasm.js` | ~10 KB | wasm-bindgen JS glue |
| `aphrody_wasm.d.ts` | ~3 KB | TypeScript types |

Optimisations are pinned in
[`crates/aphrody-wasm/Cargo.toml`](../crates/aphrody-wasm/Cargo.toml) under
`[package.metadata.wasm-pack.profile.release]`: `wasm-opt` is enabled with
SIMD and bulk-memory flags. As a sanity check, a no-opt build of the same
crate is ~1.2 MB uncompressed — roughly 10× worse than the release artifact.

## 7. Memory footprint

Resident set size measured via `ps -o rss=` (Linux) or
`Get-Process | Select-Object WorkingSet` (PowerShell):

- `aphrody --version`: **~5 MB RSS** with the `mimalloc` global allocator.
- `aphrody doctor`: **~12 MB RSS** (the diagnostic path loads `reqwest` +
  `rustls` for the network probe).
- Long-running A2A listener: **~15 MB RSS**, stable across an 8-hour grind
  loop (no leaks observed).

## 8. Comparison to alternatives (honest)

Apples-to-oranges — different tools do different things — but useful for
calibrating expectations:

| Tool | Cold start | Notes |
|---|---:|---|
| `just` | ~10 ms | Smaller surface, no diagnostic subcommand. |
| `taskfile` | ~50 ms | Go runtime overhead. |
| `gh` | ~80-150 ms | Go + HTTP client init. |
| `aphrody --version` | ~30 ms | Rust + `mimalloc`; competitive with `just`. |
| `aphrody doctor` | < 100 ms | Wider scope than the others; comparable to `gh auth status`. |

`aphrody` is competitive with `just` for task-runner use, faster than `gh`
for environment diagnostics, and the only tool here that also ships an A2A
coordination protocol.

## 9. Profiling tips

```bash
# CPU flamegraph (Linux: perf_events; Windows: requires WPR)
cargo install flamegraph
cargo flamegraph -p backend --bench backend_bench

# Heap profile (Linux/valgrind)
valgrind --tool=massif ./target/release/aphrody doctor
ms_print massif.out.*

# Async runtime introspection
# Build the cli with `--features tokio-console-instrument`, then
tokio-console http://127.0.0.1:6669
```

For a quick "is this hot?" answer without setup, `cargo bench -p backend --
--quick` finishes in seconds and tells you whether a refactor moved any of
the five micro-benches outside its expected band.

## 10. Performance regressions

Criterion benches run in the extended job of
[`.github/workflows/cross-platform.yml`](../.github/workflows/cross-platform.yml).
The job stores the previous baseline under `target/criterion/` and criterion
reports percent change automatically. Open an issue tagged `perf` if you see
a **>20% regression** on any of the five benches versus the baseline — that
threshold catches real algorithmic regressions while tolerating noise from
shared CI runners.

For deeper investigations, cross-reference
[`../BENCHMARKS.md`](../BENCHMARKS.md) (mrx-focused, with throughput tables)
and [`audits/2026-05-17-mrx-aggressive.md`](audits/2026-05-17-mrx-aggressive.md)
(audit of the aggressive scan mode and its cost model).
