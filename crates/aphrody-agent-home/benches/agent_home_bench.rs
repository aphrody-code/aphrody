// SPDX-License-Identifier: Apache-2.0
//! Criterion micro-benchmarks for aphrody-agent-home (AH-18).
//!
//! Three groups:
//!   * `cold_load/mmap` vs `cold_load/read` — the zero-copy mmap path vs a
//!     plain heap read of the same file (the plan §3 "map once" claim).
//!   * `cache/hit` — a second load served from the content-addressed cache.
//!   * `assemble/system_prompt` — assembling the borrowed system-prompt view
//!     from a fully seeded workspace.

use std::hint::black_box;

use aphrody_agent_home::{AgentHome, BootstrapBudget, FileCache, HomeOptions, MappedBytes, OnboardOptions};
use criterion::{criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

/// Seed a realistic workspace and return its tempdir (kept alive by caller).
fn seeded_workspace() -> (TempDir, std::path::PathBuf) {
    let td = TempDir::new().expect("tempdir");
    let ws = td.path().join("workspace");
    AgentHome::onboard(&OnboardOptions::new(&ws)).expect("seed");
    (td, ws)
}

fn bench_cold_load(c: &mut Criterion) {
    let td = TempDir::new().expect("tempdir");
    // A representative bootstrap file (~4 KiB of markdown).
    let path = td.path().join("SOUL.md");
    let body = "You are a focused engineering companion. ".repeat(100);
    std::fs::write(&path, &body).expect("write");

    let mut group = c.benchmark_group("cold_load");
    group.bench_function("mmap", |b| {
        b.iter(|| {
            let m = MappedBytes::load(black_box(&path)).expect("mmap");
            black_box(m.len())
        });
    });
    group.bench_function("read", |b| {
        b.iter(|| {
            let v = std::fs::read(black_box(&path)).expect("read");
            black_box(v.len())
        });
    });
    group.finish();
}

fn bench_cache_hit(c: &mut Criterion) {
    let td = TempDir::new().expect("tempdir");
    let path = td.path().join("SOUL.md");
    std::fs::write(&path, "persona body that is cached").expect("write");
    let cache = FileCache::new();
    // Prime the cache.
    cache.load("SOUL.md", &path).expect("prime");

    c.bench_function("cache/hit", |b| {
        b.iter(|| {
            let hit = cache.load(black_box("SOUL.md"), black_box(&path)).expect("hit");
            black_box(hit.from_cache)
        });
    });
}

fn bench_assemble(c: &mut Criterion) {
    let (_td, ws) = seeded_workspace();
    let budget = BootstrapBudget::default();

    c.bench_function("assemble/system_prompt", |b| {
        // Re-open per iteration to include the file-load + parse cost (the
        // realistic per-session cost the runtime pays).
        b.iter(|| {
            let home = AgentHome::open(HomeOptions {
                workspace: Some(ws.clone()),
                ..HomeOptions::default()
            })
            .expect("open");
            let view = home.system_prompt(black_box(&budget));
            black_box(view.render().len())
        });
    });

    // Also bench just the assembly off an already-open home (the hot path when
    // the home is shared via arc-swap and only re-rendered).
    let home = AgentHome::open(HomeOptions {
        workspace: Some(ws.clone()),
        ..HomeOptions::default()
    })
    .expect("open");
    c.bench_function("assemble/render_only", |b| {
        b.iter(|| {
            let view = home.system_prompt(black_box(&budget));
            black_box(view.render().len())
        });
    });
}

criterion_group!(benches, bench_cold_load, bench_cache_hit, bench_assemble);
criterion_main!(benches);
