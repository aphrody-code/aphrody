<!-- SPDX-License-Identifier: Apache-2.0 -->

# Performance History — `aphrody` Bench Ledger

Long-term ledger of criterion bench results across releases. Manual entry per
release, supplemented by the `.github/workflows/bench.yml` CI artifact archive.

## 1. Purpose

Track criterion bench results across releases of the `backend`, `cli`, and
`google_mcp` (`aphrody-mcp` binary) crates. Detect regressions early — before
users notice in real workloads. The ledger is maintained by hand on every
release tag and supplemented by the `bench.yml` CI artifact archive (30-day
retention). Together they give two views of bench health: a stable per-release
record (this file) and a per-PR delta stream (CI artifacts).

Two benches dominate the **pillar R1** acceptance criterion (PLAN §R-A R1.5):

- `cargo bench -p aphrody --bench cold_start` — `aphrody version` cold-start
  p50/p95/p99. Target v2.0.0 : **< 5 ms p50** on the reference machine.
- `cargo bench -p google_mcp --bench initialize_handshake` — `aphrody-mcp`
  stdio MCP handshake p50/p95. Target v2.0.0 : **< 20 ms p50** on the
  reference machine.

## 2. How to update

- After every `v*` tag, run on the reference machine:
  ```bash
  cargo bench -p backend --locked 2>&1 | tee bench-backend-vX.Y.Z.txt
  cargo bench -p aphrody --bench cold_start --locked 2>&1 | tee bench-cold-start-vX.Y.Z.txt
  cargo bench -p google_mcp --bench initialize_handshake --locked 2>&1 | tee bench-mcp-init-vX.Y.Z.txt
  ```
- Extract the criterion summary lines (mean ± stddev) for each bench and add
  a row to the matching table in section 4 below (one table per bench
  surface: backend primitives, cold-start, MCP handshake).
- Compare against the previous row; flag any bench whose mean moved by more
  than 20 % with `[REGRESSION]` in the Notes column.
- Investigate every flagged regression with `cargo flamegraph` BEFORE tagging
  the next version. Do not paper over flagged rows.

## 3. Reference hardware

The ledger uses a fixed reference machine to keep numbers comparable across
releases:

- CPU: 2024-class x86_64 (e.g., AMD Ryzen 9 7950X / Intel Core i9-14900K)
- Clock: ~4.0–5.5 GHz boost
- Memory: 32 GB DDR5
- Storage: NVMe SSD
- OS: Ubuntu 26.04 LTS (host of the `bench.yml` CI runner)

Your local numbers will differ. The shape — relative change between
consecutive rows — is what matters.

## 4. Bench ledger (chronological, oldest first)

### 4.1 Backend primitives (`cargo bench -p backend`)

| Date | Version | vfs_resolve | dns_dedup_sort | aes_gcm_decrypt_1kb | sha256_hash_1mb | serde_json_parse_crtsh | Notes |
|---|---|---|---|---|---|---|---|
| 2026-05-17 | 1.0.0-canary | ~200 ns | ~5 us | ~30 us | ~4 ms | ~10 us | baseline (criterion benches added in YOLO #41) |
| _next_ | _v0.1.0_ | TBD | TBD | TBD | TBD | TBD | _ledger row added at first stable release_ |

### 4.2 Cold-start latency (`cargo bench -p aphrody --bench cold_start`)

Measures wall-clock latency of a fresh `aphrody version` process spawn
(dominant cost: dynamic linker + mimalloc init + clap parse + on Windows
`CreateProcessW`). The benchmark uses `iter_custom` with a sample size of 20
to keep the bench run interactive (~10 s Linux, ~30 s Windows).

| Date | Version | aphrody_version p50 | p95 | p99 | Notes |
|---|---|---|---|---|---|
| 2026-05-19 | 1.0.0-canary | TBD | TBD | TBD | bench shipped (R-A R1.5 phase 1) — first measurement at v0.1.0 tag |
| _next_ | _v0.1.0_ | TBD | TBD | TBD | _target: p50 < 5 ms on reference hardware_ |

### 4.3 MCP initialize handshake (`cargo bench -p google_mcp --bench initialize_handshake`)

Measures wall-clock latency of a full `aphrody-mcp` stdio MCP handshake :
fresh process spawn → `initialize` JSON-RPC request → response →
`notifications/initialized` notification → client drop (kills child). Sample
size 10 because each iteration burns ~30 ms Linux / ~80 ms Windows.

| Date | Version | stdio_handshake p50 | p95 | Notes |
|---|---|---|---|---|
| 2026-05-19 | 1.0.0-canary | TBD | TBD | bench shipped (R-A R1.5 phase 2) — first measurement at v0.1.0 tag |
| _next_ | _v0.1.0_ | TBD | TBD | _target: p50 < 20 ms on reference hardware_ |

## 5. Investigation playbook

When a row shows `[REGRESSION]`:

1. Verify it is not CI runner noise — re-run `bench.yml` three times and
   take the median.
2. `git bisect` between the last good version and the regressing one.
3. `cargo flamegraph` to find the slow function in the hot path.
4. Apply profile-guided optimisation (`cargo pgo`) if the regression is in
   a hot inner loop that benefits from PGO.
5. If the regression is genuine and structural, open an issue; consider
   yanking the affected version per `docs/cargo/PUBLISH-LADDER.md` section 5.

## 6. CI artifact archive

`.github/workflows/bench.yml` uploads a `bench-output.txt` artifact per run,
retained 30 days. To pull a historical bench from CI:

```bash
gh run download <run-id> -n bench-output-<sha>
```

Cross-reference the artifact `sha` with the release tag for end-to-end
traceability between a published version and its measured bench numbers.

## 7. Compared to alternatives

We do NOT (yet) use `criterion-compare-action`. It requires baseline
persistence across runs and a sustained CI baseline store, which is more
infrastructure than we currently maintain. When that lands (planned Q3 2026
per `docs/ROADMAP.md`), the manual ledger will be supplemented, not
replaced — the manual version captures release-version-level snapshots; the
CI version captures per-PR deltas. Both views matter.

## 8. Reference

- `crates/backend/benches/backend_bench.rs` — backend primitives bench source
- `crates/cli/benches/cold_start.rs` — `aphrody version` cold-start bench
  (R-A R1.5 phase 1)
- `crates/google_mcp/benches/initialize_handshake.rs` — `aphrody-mcp` stdio
  MCP handshake bench (R-A R1.5 phase 2)
- `docs/PERFORMANCE.md` — bench claims with reproduction recipes
- `docs/PLAN.md` §R-A R1.5 — acceptance criterion (p50 cold-start < 5 ms,
  p50 MCP initialize < 20 ms on reference hardware)
- `.github/workflows/bench.yml` — CI bench gate
- `docs/ROADMAP.md` — planned `criterion-compare-action` wiring
- `docs/cargo/PUBLISH-LADDER.md` — yank policy referenced in section 5
