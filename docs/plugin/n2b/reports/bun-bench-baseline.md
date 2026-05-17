# Bun Benchmark Baseline Report

**Date:** 2026-04-18  
**Bun version:** 1.3.13-canary.1 (bc7da9ed)  
**Platform:** x64-linux, Intel Core (Haswell, no TSX), clk ~3.48 GHz  
**System Bun:** `/home/ubuntu/.bun/bin/bun`

---

## Phase 1 — Inventory

### Bench directory structure (`bench/`)

| Directory | Runtime | Type | Est. duration | Notes |
|-----------|---------|------|---------------|-------|
| `async/` | Bun/Node/Deno | sync vs async vs await | ~5s | No external deps |
| `gzip/` | Bun/Node/Deno | zlib vs libdeflate | ~7s | Needs `@babel/standalone` |
| `json5/` | Bun only | Bun.JSON5 vs js library | ~11s | Needs `json5` package |
| `crypto/random.mjs` | Bun only | randomInt/randomBytes/Fill | ~8s | No external deps |
| `crypto/aes-gcm-throughput.mjs` | Bun only | AES-GCM 4KB/1MB | ~7s | No external deps |
| `deepEqual/map.js` | Bun only | expect().toEqual on Maps | ~4s | No external deps |
| `deepEqual/set.js` | Bun only | expect().toEqual on Sets | ~4s | No external deps |
| `emitter/microbench.mjs` | Bun only | EventEmitter emit/on | ~5s | No external deps |
| `sqlite/` | Bun/Node/Deno | SELECT queries, northwind DB | >30s | Needs `better-sqlite3` + northwind.sqlite download |
| `ffi/` | Bun/Node | Rust FFI calls | >30s | Needs Rust build (Cargo) |
| `fetch/` | Bun only | 100x fetch to example.com | Variable | Network-dependent, skip |
| `install/` | Bun | bun install benchmark | >60s | Next.js T3 app, very long |
| `bundle/` | Bun/esbuild/swc | Bundler throughput | >30s | Needs Node + esbuild |
| `express/` | Bun/Node | HTTP server throughput | Requires running server | Needs wrk/k6 |
| `websocket-server/` | Bun/Node | WebSocket throughput | Requires running server | Needs external client |
| `postgres/` | Bun | PostgreSQL queries | Requires running PG | Skip (no DB) |
| `grpc-server/` | Bun | gRPC server | Requires running server | Skip |
| `stress/` | Various | Stress tests | Very long | Skip |
| `stream-file-upload-client/` | Bun | Streaming uploads | Requires server | Skip |
| `react-hello-world/` | Bun/Node | React SSR | Requires deps | Node comparison bench |
| `sourcemap/` | Bun | Source map gen | Medium | Needs deps |
| `glob/` | Bun | Glob matching | ~10s | Has braces/micromatch deps |
| `yaml/` | Bun | YAML parsing | ~5s | Standalone |
| `scanner/` | Bun | JS lexer scan | ~5s | Standalone |
| `modules/` | Bun/Node | CommonJS/ESM load | ~10s | Multiple sub-benches |

### Benches suitable for CI (< 30s, hot paths)

1. `async/bun.js` — tests async overhead (core event loop path)
2. `gzip/bun.js` — tests zlib vs libdeflate (compression path)
3. `crypto/random.mjs` — tests CSPRNG (crypto path)
4. `crypto/aes-gcm-throughput.mjs` — tests cipher throughput (crypto path)
5. `json5/json5.mjs` — tests Bun.JSON5 native vs JS (parsing path)
6. `deepEqual/map.js` + `set.js` — tests expect().toEqual (test runner path)
7. `emitter/microbench.mjs` — tests EventEmitter (event path)

---

## Phase 2 — Baseline Measurements

### Benchmarks executed (5 selected)

#### 1. `async/bun.js` — Async overhead

```
hyperfine --warmup 2 --runs 5 'bun bun.js'
mean: 4.989s ± 0.040s
```

| Metric | sync | async | await 1 |
|--------|------|-------|---------|
| avg/iter | 79.21 ps | 65.19 ns | 217.76 ns |
| p99 | 120 ps | 135.92 ns | 419.10 ns |

Key observation: `async` overhead is 65 ns/iter vs 79 ps for sync — ~820x slower per call. `await 1` resolves a non-Promise value and costs 217 ns, ~3.3x more than plain async. This is the fundamental cost of microtask scheduling in JavaScriptCore.

#### 2. `gzip/bun.js` — Compression throughput (babel.min.js, ~1.4 MB)

```
hyperfine --warmup 2 --runs 5 'bun bun.js'
mean: 6.821s ± 0.037s
```

| Operation | zlib (avg) | libdeflate (avg) | Speedup |
|-----------|-----------|-----------------|---------|
| roundtrip | 49.82 ms | 42.61 ms | +14% |
| gzipSync | 44.69 ms | 39.09 ms | +13% |
| gunzipSync | 4.52 ms | 2.85 ms | +37% |

Key observation: libdeflate is consistently faster. Decompression (gunzipSync) shows the largest gain: libdeflate is 37% faster than zlib. The `zlib` path is the fallback for versions < 1.1.21 — on 1.3.13 both backends are available.

#### 3. `crypto/random.mjs` — CSPRNG

```
hyperfine --warmup 2 --runs 5 'bun random.mjs'
mean: 8.231s ± 0.122s (highest stddev ratio = 1.5%)
```

| API | avg/iter | Notes |
|-----|---------|-------|
| randomInt (sync) | 21.60 ns | Fast path |
| randonBytes - 32 | 1.26 µs | ~58x slower than randomInt |
| randomBytes - 256 | 1.34 µs | Minimal scaling with size |
| randomFillSync - 32 | 1.18 µs | Similar to randomBytes |
| randomFillSync - 256 | 1.21 µs | Flat with size |
| randomFill - 32 (async) | 22.04 µs | ~18x async overhead vs sync |
| randomFill - 256 (async) | 21.64 µs | Dominated by async overhead |
| randomInt (async) | 777.47 ns | callback overhead |

Key observation: `randomBytes` cost is essentially flat between 32 and 256 bytes (1.26 µs vs 1.34 µs), suggesting the per-call fixed cost (Buffer allocation, entropy pool lock) dominates over data size. The async `randomFill` at ~22 µs is ~18x the sync counterpart — pure scheduling overhead.

#### 4. `json5/json5.mjs` — Bun.JSON5 native vs js library

```
hyperfine --warmup 2 --runs 5 'bun json5.mjs'
mean: 10.844s ± 0.090s
```

| Operation | Bun.JSON5 | json5 (JS) | Speedup |
|-----------|----------|-----------|---------|
| parse small (97B) | 2.28 µs | 18.34 µs | **8x** |
| parse large (1.1MB) | 30.30 ms | 210.31 ms | **6.9x** |
| stringify small (61B) | 1.07 µs | 4.94 µs | **4.6x** |
| stringify large (782KB) | 12.02 ms | 89.85 ms | **7.5x** |

Key observation: Bun.JSON5 native implementation is consistently 5-8x faster than the pure-JS `json5` package. The parse-large benchmark (30 ms for 1.1 MB) is the most expensive single operation — at ~36 MB/s this is notably slower than `JSON.parse` which runs at >1 GB/s for equivalent JSON. The JSON5 syntax adds overhead proportional to input size.

#### 5. `deepEqual/map.js` — expect().toEqual on Maps (10k entries)

```
hyperfine --warmup 2 --runs 5 'bun map.js'
mean: 3.927s ± 0.022s (lowest stddev = 0.6%)
```

| Benchmark | avg/iter | p99 |
|-----------|---------|-----|
| deepEqual Map (10k entries) | 616.63 µs | 732.88 µs |
| deepEqual CustomMap (10k entries) | 628.15 µs | 719.27 µs |
| deepEqual Set (10k entries) | 458.57 µs | 536.61 µs |
| deepEqual CustomSet (10k entries) | 461.52 µs | 565.53 µs |

Key observation: Map equality check (616 µs) is ~34% slower than Set (458 µs) for the same 10k-entry collection. CustomMap subclass adds only ~2% overhead vs plain Map, suggesting class identity check is cheap. At 616 µs per call, a test suite with hundreds of large Map assertions will have measurable latency.

---

## Phase 3 — Anomalies and Observations

### Anomaly 1 — randomBytes flat scaling (REGRESSION CANDIDATE)

`randomBytes(32)` = 1.26 µs, `randomBytes(256)` = 1.34 µs. An 8x size increase yields only +6% time. This means the fixed per-call cost (~1.2 µs) completely dominates. For small random buffer needs, the allocation overhead is the bottleneck, not entropy generation. The zig-engineer should examine `src/bun.js/api/crypto.zig` for unnecessary Buffer allocations on each call.

### Anomaly 2 — gzip zlib gunzipSync high variance

`gunzipSync` with zlib: avg 4.52 ms, min 3.53 ms, max 7.72 ms — a 2.2x spread. libdeflate: avg 2.85 ms, min 2.54 ms, max 4.67 ms. The zlib backend shows notably higher variance on decompression, suggesting memory pressure or OS scheduler interaction in the zlib C binding.

### Anomaly 3 — JSON5 parse large at 36 MB/s

For a native Zig implementation, 30.3 ms for 1.1 MB is slow relative to native JSON.parse throughput. The JSON5 parser likely makes allocation-heavy passes or uses a less-optimized SIMD path. This is the strongest optimization candidate.

### Anomaly 4 — async await 1 costs 217 ns

`await 1` (a non-thenable) takes 217 ns vs 65 ns for `async () => {}`. The 3x cost to resolve a non-Promise with `await` suggests JSC's fast-path for non-Promise awaitables is not being hit, or the microtask flush has overhead even for trivially resolved values.

---

## Phase 4 — Skipped Benches

| Bench | Reason skipped |
|-------|---------------|
| `sqlite/` | Requires `northwind.sqlite` download via `bash src/download.sh` (network) |
| `ffi/` | Requires Rust Cargo build of native library |
| `fetch/` | Network-dependent (external HTTP to example.com), non-deterministic |
| `install/` | Full Next.js T3 app install — multi-minute duration |
| `bundle/` | Requires Node.js + esbuild (no Node on this VPS) |
| `express/` | HTTP server bench — requires external load generator (wrk/k6) |
| `websocket-server/` | Requires concurrent client process |
| `postgres/` | No PostgreSQL instance running |
| `grpc-server/` | Requires running gRPC server + client |
| `stress/` | Long-running soak tests |
| `stream-file-upload-client/` | Requires running server |

---

## Summary Table

| Bench | Runtime | Key metric | hyperfine mean ± σ | Anomaly? |
|-------|---------|-----------|-------------------|---------|
| `async/bun.js` | Bun 1.3.13 | async: 65 ns/iter, await 1: 218 ns/iter | 4.989s ± 0.040s | await 1 = 3.3x async |
| `gzip/bun.js` | Bun 1.3.13 | gunzipSync zlib: 4.52 ms, libdeflate: 2.85 ms | 6.821s ± 0.037s | zlib 2.2x variance |
| `crypto/random.mjs` | Bun 1.3.13 | randomBytes(32): 1.26 µs, (256): 1.34 µs | 8.231s ± 0.122s | flat scaling — alloc bottleneck |
| `json5/json5.mjs` | Bun 1.3.13 | Bun.JSON5.parse large: 30.3 ms/1.1MB = 36 MB/s | 10.844s ± 0.090s | slow vs JSON.parse |
| `deepEqual/map.js` | Bun 1.3.13 | Map 10k: 616 µs, Set 10k: 458 µs | 3.927s ± 0.022s | Map 34% slower than Set |

---

## Optimization Candidates (priority order)

1. **JSON5 parse throughput** (`json5/json5.mjs`, `Bun.JSON5.parse large` 30 ms/1.1 MB) — 36 MB/s is far below native JSON.parse throughput. The zig-engineer should profile `src/` for the JSON5 parser hotspot (allocation pattern, SIMD usage).

2. **randomBytes fixed per-call cost** (`crypto/random.mjs`) — 1.2 µs baseline regardless of size. Per-call Buffer allocation overhead is the suspect. Pooling or stack-allocating small buffers could give 2-5x improvement for `randomBytes(32)` use cases.

3. **gzip zlib variance** (`gzip/bun.js`) — zlib gunzipSync has min 3.5 ms / max 7.7 ms spread. libdeflate should be the default backend since it is both faster and more consistent.

4. **await non-Promise overhead** (`async/bun.js`, `await 1` 218 ns) — JSC fast path for non-thenable await not being used optimally. JSC's `@operationResolveNonPromise` should be cheaper than 218 ns on modern hardware.

---

## Reproduce Commands

```bash
# async
cd /home/ubuntu/rsbun/bun/bench/async
bun bun.js
# or: hyperfine --warmup 2 --runs 5 'bun bun.js'

# gzip (requires @babel/standalone installed)
cd /home/ubuntu/rsbun/bun/bench/gzip
BUN_CONFIG_REGISTRY=https://registry.npmjs.org bun install
bun bun.js
# or: hyperfine --warmup 2 --runs 5 'bun bun.js'

# crypto random
cd /home/ubuntu/rsbun/bun/bench/crypto
bun random.mjs
# or: hyperfine --warmup 2 --runs 5 'bun random.mjs'

# json5 (requires json5 installed)
cd /home/ubuntu/rsbun/bun/bench/json5
BUN_CONFIG_REGISTRY=https://registry.npmjs.org bun install
bun json5.mjs
# or: hyperfine --warmup 2 --runs 5 'bun json5.mjs'

# deepEqual Map/Set
cd /home/ubuntu/rsbun/bun/bench/deepEqual
bun map.js && bun set.js
# or: hyperfine --warmup 2 --runs 5 'bun map.js'

# AES-GCM cipher throughput
cd /home/ubuntu/rsbun/bun/bench/crypto
bun aes-gcm-throughput.mjs
```

Note: Top-level `bun install` in `bench/` fails because `bun.lock` references an internal Artifactory registry (Bun CI infra). Use `BUN_CONFIG_REGISTRY=https://registry.npmjs.org bun install` per sub-bench instead.
