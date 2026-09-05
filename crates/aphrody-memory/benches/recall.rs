// SPDX-License-Identifier: Apache-2.0
//
// Recall benchmark — top-10 semantic search over 100k events on `HnswBackend`.
//
// PLAN R-B R3.7 acceptance criterion (cf. `docs/PLAN.md`):
//
//   "Recall benchmark : 100k events, query top-10 semantic, p95 < 100 ms"
//   verify: `cargo bench -p aphrody-memory bench_recall_100k`
//
// The criterion group is named `bench_recall_100k` so the PLAN verify command
// (which is a benchmark-id substring filter, not a Rust symbol) selects this
// bench precisely.
//
// The brute-force cosine implementation in `crates/aphrody-memory/src/hnsw.rs`
// scans every embedded record per query. With 128-dim vectors and 100 000
// records that is ~12.8 M f32 multiply-adds per query — comfortably under the
// 100 ms budget on a 2024-class x86_64 CPU.
//
// ## Why `HnswBackend` and not `SqliteBackend` / Tier-1 providers
//
// `MemoryBackend` (tier-2, embedding-aware) is the only trait in this crate
// that exposes a semantic-search method (`search(&Embedding, top_k)`). Of the
// three implementations only `HnswBackend` keeps the entire index in process
// memory ; `SqliteBackend` performs a full table scan over a `BLOB` column
// (deserialise -> cosine -> sort) which dominates the bench with SQLite-specific
// overhead unrelated to the recall algorithm. `LanceDbBackend` carries a real
// ANN index but pulls Arrow/IO transitives and requires Tokio's multi-thread
// runtime — not the latency we want to publish as the headline number.
//
// The Tier-1 providers (`Mem0Provider`, `HonchoProvider`, `SqliteLocalProvider`)
// expose keyword search only ; "semantic" there is a property of the remote
// service, not of this crate, so a local bench cannot measure it honestly.
//
// ## Methodology
//
// - Setup (100k inserts) runs once, *outside* the criterion timed window, via
//   `runtime.block_on(build_corpus())`. Each criterion sample runs `iters`
//   in-memory `search()` calls and reports the total elapsed time ; criterion
//   then derives p50 / mean / std-dev / outliers (and the p95 cited in the
//   PLAN) internally.
// - Sample size is 20. Each sample loops `iters` searches where `iters` is
//   tuned by criterion so a sample lasts long enough for stable percentiles.
// - The query embedding is generated with the same deterministic SplitMix64
//   PRNG that seeds the corpus, picking a seed outside the corpus range
//   (`0x_DEADBEEF_CAFE_F00D`) so the query is not trivially identical to any
//   stored record. Reproducibility is exact across runs (no `rand` thread-RNG).
//
// ## Verify
//
// ```bash
// cargo check --benches -p aphrody-memory --locked
// cargo bench -p aphrody-memory bench_recall_100k -- --warm-up-time 1 --measurement-time 3
// # -> target/criterion/bench_recall_100k/recall_100k_top10_d128/report/index.html
// ```

use std::hint::black_box;
use std::time::Instant;

use criterion::{Criterion, criterion_group, criterion_main};
use tokio::runtime::{Builder, Runtime};

use aphrody_memory::{Embedding, HnswBackend, MemoryBackend, MemoryRecord};

/// Number of events seeded into the in-memory index for the headline bench.
///
/// The PLAN verify is `100k` ; reducing this is a regression.
const RECORD_COUNT: usize = 100_000;

/// Embedding dimensionality. 128 matches the size emitted by small SBERT
/// distillates (e.g. `all-MiniLM-L6-v2` projected to 128 via mean-pooled
/// truncation) — representative of the smaller end of production deployments.
/// The cost scales linearly in this constant.
const EMBED_DIM: usize = 128;

/// Top-K passed to every `search()` call — matches the PLAN verb (top-10).
const TOP_K: usize = 10;

/// Build a fresh single-thread Tokio runtime for the bench.
///
/// `current_thread` is intentional : `HnswBackend::search` is CPU-bound and a
/// multi-thread runtime would only add scheduler overhead between criterion's
/// timer and the search body.
fn make_runtime() -> Runtime {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current-thread runtime must build")
}

/// Generate a deterministic embedding from a u64 seed.
///
/// SplitMix64 — branchless, allocation-free, no dependency on `rand`. Each
/// dimension is mapped from a fresh u64 to `[-1.0, 1.0]` via the signed-i64
/// route so the distribution is centred (cosine similarity is undefined on
/// zero vectors ; centring around zero is OK because we ship 128 dims and the
/// probability of an all-zero output is effectively zero).
fn synthetic_embedding(seed: u64, dim: usize) -> Embedding {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    (0..dim)
        .map(|_| {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            // Reinterpret as signed and normalise into [-1.0, 1.0]. The
            // `as i64 as f64` chain preserves sign without UB.
            (z as i64 as f64 / i64::MAX as f64) as f32
        })
        .collect()
}

/// Synthesize the bench corpus : one `HnswBackend` loaded with `RECORD_COUNT`
/// events, plus one query embedding never present in the corpus.
///
/// The temp directory is returned so the caller keeps it alive — `HnswBackend`
/// performs one disk flush in `put_many` and the on-disk file would otherwise
/// be reaped before the bench finishes. The flush is intentional : it
/// exercises the same code path a production caller hits during a bulk import
/// (R3.4 migration tool), so the bench captures realistic init cost even
/// though that cost lives outside the timed window.
async fn build_corpus() -> (tempfile::TempDir, HnswBackend, Embedding) {
    let dir = tempfile::tempdir().expect("tempdir must succeed");
    let mut backend = HnswBackend::open(dir.path())
        .await
        .expect("HnswBackend::open must succeed on a fresh tempdir");

    let batch: Vec<(String, MemoryRecord)> = (0..RECORD_COUNT)
        .map(|i| {
            let rec = MemoryRecord::new("bench", format!("event-{i}"))
                .with_embedding(synthetic_embedding(i as u64, EMBED_DIM));
            // Match the convention "<ns>/<uuid>" used by the production code.
            let key = format!("bench/{}", rec.id);
            (key, rec)
        })
        .collect();
    backend
        .put_many(batch)
        .await
        .expect("bulk insert must succeed (no I/O failure on tempdir)");

    // Query seed deliberately outside the corpus range so the query is not a
    // trivial nearest-neighbour of itself.
    let query = synthetic_embedding(0x_DEAD_BEEF_CAFE_F00D, EMBED_DIM);

    (dir, backend, query)
}

/// Top-10 semantic-search recall bench over 100 000 events.
///
/// The bench loops `iters` calls to `HnswBackend::search` and reports the total
/// elapsed wall-clock to criterion, which derives the percentile distribution
/// (including the p95 cited in the PLAN) from the sample of 20 such windows.
fn bench_recall_100k(c: &mut Criterion) {
    let runtime = make_runtime();
    // Setup runs once, outside the timed loop. `_dir` keeps the tempdir alive
    // until the bench function returns.
    let (_dir, backend, query) = runtime.block_on(build_corpus());

    let mut group = c.benchmark_group("bench_recall_100k");
    // 20 samples x N iters per sample ; criterion auto-tunes iters.
    group.sample_size(20);
    // Throughput in elements per second — useful for the regression dashboard.
    group.throughput(criterion::Throughput::Elements(RECORD_COUNT as u64));

    group.bench_function("recall_100k_top10_d128", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            runtime.block_on(async {
                for _ in 0..iters {
                    let results = backend
                        .search(black_box(&query), black_box(TOP_K))
                        .await
                        .expect("search must succeed on a healthy backend");
                    // Touch the result so the optimiser cannot elide the call.
                    black_box(&results);
                }
            });
            start.elapsed()
        });
    });

    group.finish();
}

criterion_group!(benches, bench_recall_100k);
criterion_main!(benches);
