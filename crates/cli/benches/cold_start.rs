// SPDX-License-Identifier: Apache-2.0
//
// Cold-start latency benchmarks for the `aphrody` binary.
//
// We spawn a fresh OS process per iteration and let criterion measure the
// wall-clock window from `Command::output()` to exit. This is the latency a
// user perceives when typing `aphrody version` in a shell, so it is the
// number quoted in `docs/PERFORMANCE.md` and tracked in
// `docs/PERFORMANCE-HISTORY.md`.
//
// The dominant cost in a cold-start invocation is NOT the body of the
// `Version` subcommand (which prints two short lines from compile-time
// constants) — it is dynamic-linker resolution, mimalloc init, clap parse,
// and on Windows `CreateProcessW`. Profiling the inner function would give a
// nanosecond-scale measurement that does not correspond to anything a user
// observes ; the process-spawn approach is the one that matters.
//
// We use `iter_custom` so the bench wraps an outer loop of `iters` spawns
// inside a single criterion sample — without it criterion's default harness
// would conflate sample setup with the spawn itself.
//
// Acceptance criterion (PLAN.md, R-A R1.5):
//   - Report p50 / p95 / p99 of `aphrody version`.
//   - HTML report produced by `cargo bench -p aphrody --bench cold_start`.
//   - Ledger row added to `docs/PERFORMANCE-HISTORY.md` on each release.

use std::hint::black_box;
use std::process::Command;
use std::time::Instant;

use criterion::{Criterion, criterion_group, criterion_main};

/// Resolve the freshly-built `aphrody` binary path.
///
/// `CARGO_BIN_EXE_<name>` is injected by Cargo at compile time for any
/// `[[bench]]` declared in the same crate as a `[[bin]]`. This avoids both a
/// `which` lookup (which would race with an outdated `~/.local/bin/aphrody`)
/// and an `assert_cmd` dependency at bench time.
const APHRODY_BIN: &str = env!("CARGO_BIN_EXE_aphrody");

/// Spawn `aphrody version` and wait for it to exit successfully.
///
/// Returns nothing — the bench measures wall-clock time, not the output.
/// We assert exit-success to fail fast if the binary regresses to a non-zero
/// exit (otherwise we'd record the latency of a broken binary).
fn spawn_version() {
    let output = Command::new(black_box(APHRODY_BIN))
        .arg("version")
        .output()
        .expect("aphrody version: spawn must succeed (built binary missing?)");
    assert!(
        output.status.success(),
        "aphrody version exited non-zero ({:?}); stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Cold-start latency of `aphrody version`.
///
/// Sample size is intentionally low (20) because each iteration burns
/// 1–5 ms of OS bookkeeping (`CreateProcessW` on Windows is the costliest
/// path). With criterion's default of 100 samples × 20 iterations = 2000
/// process spawns per bench run, which is ~10 seconds on Linux and ~30 s on
/// Windows — fast enough for `cargo bench` to stay interactive.
fn bench_version_cold_start(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_start");
    group.sample_size(20);

    group.bench_function("aphrody_version", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                spawn_version();
            }
            start.elapsed()
        });
    });

    group.finish();
}

criterion_group!(benches, bench_version_cold_start);
criterion_main!(benches);
