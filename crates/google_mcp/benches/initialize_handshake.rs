// SPDX-License-Identifier: Apache-2.0
//
// MCP `initialize` handshake latency benchmark for the `aphrody-mcp` binary.
//
// We spawn a fresh `aphrody-mcp` stdio child per iteration, run the full
// MCP handshake (`initialize` → `notifications/initialized`), and let
// criterion measure the wall-clock window. This is the latency a downstream
// client (Claude Code, Gemini CLI, third-party MCP host) observes when first
// reaching out to our server, so it is the number quoted in
// `docs/PERFORMANCE.md` and tracked in `docs/PERFORMANCE-HISTORY.md`.
//
// The dominant cost is process spawn + tokio runtime warm-up + serde_json
// parse of the `tools/list` payload (15 tool descriptors). Profiling the
// inner handshake function alone would give a microsecond-scale measurement
// that does not match the user-perceived first-call latency; the
// process-spawn approach is the one that matters.
//
// We use `iter_custom` so the bench wraps an outer loop of `iters`
// spawn+handshake+drop cycles inside a single criterion sample — otherwise
// criterion's default harness would conflate sample setup with the spawn.
//
// Acceptance criterion (PLAN.md, R-A R1.5):
//   - Report p50 / p95 of `aphrody-mcp` initialize handshake.
//   - HTML report produced by `cargo bench -p google_mcp --bench initialize_handshake`.
//   - Ledger row added to `docs/PERFORMANCE-HISTORY.md` on each release.

use std::collections::HashMap;
use std::hint::black_box;
use std::time::Instant;

use criterion::{Criterion, criterion_group, criterion_main};
use gemini_runtime::McpClient;
use tokio::runtime::Builder;

/// Resolve the freshly-built `aphrody-mcp` binary path.
///
/// `CARGO_BIN_EXE_<name>` is injected by Cargo at compile time for any
/// `[[bench]]` declared in the same crate as a `[[bin]]`. This avoids both a
/// `which` lookup (which would race with an outdated `~/.local/bin/aphrody-mcp`)
/// and an `assert_cmd` dependency at bench time.
const APHRODY_MCP_BIN: &str = env!("CARGO_BIN_EXE_aphrody-mcp");

/// Spawn `aphrody-mcp`, run the full MCP initialize handshake, drop the
/// client (which kills the child via `kill_on_drop(true)`).
///
/// We assert handshake success to fail fast if the server regresses to a
/// non-zero exit or a protocol error (otherwise we'd record the latency of
/// a broken server).
async fn spawn_and_handshake() {
    let env: HashMap<String, String> = HashMap::new();
    let mut client = McpClient::from_stdio(black_box(APHRODY_MCP_BIN), &[], &env)
        .await
        .expect("aphrody-mcp: stdio spawn must succeed (built binary missing?)");
    let info = client
        .initialize()
        .await
        .expect("aphrody-mcp: initialize handshake must succeed");
    // Touch the returned ServerInfo so the optimiser cannot elide the call.
    black_box(&info);
}

/// MCP initialize handshake latency for `aphrody-mcp` stdio.
///
/// Sample size is intentionally low (10) because each iteration burns
/// ~30 ms on Linux and ~80 ms on Windows (spawn + tokio init + JSON-RPC
/// roundtrip). With criterion's default of 100 samples × 10 iterations =
/// 1000 spawn+handshake cycles per bench run, which is ~30 s on Linux and
/// ~80 s on Windows — fast enough for `cargo bench` to stay interactive.
fn bench_initialize_handshake(c: &mut Criterion) {
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current-thread runtime must build");

    let mut group = c.benchmark_group("aphrody_mcp_initialize");
    group.sample_size(10);

    group.bench_function("stdio_handshake", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            runtime.block_on(async {
                for _ in 0..iters {
                    spawn_and_handshake().await;
                }
            });
            start.elapsed()
        });
    });

    group.finish();
}

criterion_group!(benches, bench_initialize_handshake);
criterion_main!(benches);
