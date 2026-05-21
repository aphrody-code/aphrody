// SPDX-License-Identifier: Apache-2.0
# BUILD-SPEED — Aphrody monorepo compilation cache guide

> Workspace: 57 members (100 % Rust) | Host: Windows 11 28020 x86-64 | Cores: 8 logical
>
> **NOTE (2026-05-21).** Les passages `turbo.json` / `~/.bun/…` ci-dessous
> sont **obsolètes** (Bun/Turbo bannis sous la politique 100 % Rust). Le guide
> sccache/cargo reste valide.

## Baseline (before 2026-05-18 optimizations)

| Run        | Time      | Notes                            |
|------------|-----------|----------------------------------|
| warm check | ~40 s     | cargo check --workspace --offline|
| cold check | ~8-12 min | estimated, full recompile        |

## After optimizations

| Run                          | Time    | Delta   | Notes                                     |
|------------------------------|---------|---------|-------------------------------------------|
| warm (cargo incremental)     | 3.2 s   | -92 %   | Cargo skips unchanged crates              |
| sccache cold-fill (1st run)  | 3m 42s  | n/a     | populates 7 GiB local sccache cache       |
| sccache warm (post-clean)    | ~3 min  | ~-40 %  | estimated from avg compiler 1.3s x 465    |
| cold (no cache, no sccache)  | ~8-12 min | --    | baseline before any optimization          |

Times are indicative; vary with thermal/disk state.

## Changes applied (2026-05-18)

### .cargo/config.toml
- `rustc-wrapper` enabled: `sccache.exe` (WinGet 0.15.0, stable path).
- `jobs = 7` (8 logical cores - 1 for linker/sccache).
- `link-arg=/INCREMENTAL:NO` on `x86_64-pc-windows-msvc`: fat LTO and strip
  are incompatible with MSVC incremental linking; removing it cuts link overhead.
- `ci-fast` alias: `cargo check --workspace --all-targets --locked --offline`
  (no clippy overhead — use for rapid sccache-warm iteration).
- `xt-fast` alias: `cargo nextest run --workspace --locked --offline`.

### Cargo.toml
- `[profile.dev] debug = "line-tables-only"` (was `"limited"`).
  Produces the minimum debug info needed for file:line backtraces.
  Reduces .pdb/.dwo artifact size and speeds incremental linking.

### turbo.json
- `globalDependencies` extended: `Cargo.toml`, `Cargo.lock`,
  `.cargo/config.toml`, `rust-toolchain.toml` — ensures Turbo invalidates
  all JS tasks when the Rust build config changes.
- `parallelism = 0` (auto, uses available cores).
- `remoteCache.enabled = false` (explicit; no TURBO_TOKEN configured yet).

### bunfig.toml
- `[install.cache] dir = "~/.bun/install/cache"` — explicit global cache dir,
  prevents drift when running from different working directories.

## sccache quick-start

Install (WinGet, already done on this machine):

```
winget install Mozilla.sccache
```

Linux / CI one-liner:

```bash
cargo install sccache --locked
export RUSTC_WRAPPER=sccache
```

CI (GitHub Actions — mozilla-actions handles install + S3/GHA cache backend):

```yaml
- uses: mozilla-actions/sccache-action@v0.0.5
  with:
    version: "v0.15.0"
```

To disable sccache temporarily:

```bash
RUSTC_WRAPPER="" cargo check --workspace
```

## sccache stats (after a warm run)

```
sccache --show-stats
```

Key metrics: `cache_hits`, `cache_misses`, `requests_executed`.
Aim for >70 % hit rate on a fully warm workspace.

## Profile cheat-sheet

| Alias         | Profile      | LTO    | Use-case                    |
|---------------|--------------|--------|-----------------------------|
| cargo ci-fast | (check only) | none   | fastest warm iteration      |
| cargo ci-offline | clippy    | none   | lint gate pre-push          |
| cargo xt-fast | dev          | none   | nextest, sccache-warm tests |
| cargo build-dist | dist      | fat    | final distributable binary  |
| cargo release-fast | release-fast | thin | CI release build        |

## Tips

- `-Z threads=8` in `[build] rustflags`: nightly parallel frontend; each crate
  compiles its HIR→MIR phases on up to 8 threads. Combined with sccache this
  is the biggest single win.
- `-Z share-generics=y`: reduces monomorphization duplication across crates.
- `[profile.dev.package."*"] opt-level = 1`: third-party deps compile at -O1
  in dev mode — eliminates the "debug build is 10x slower at runtime" problem.
- `[profile.dev.build-override] opt-level = 3`: build scripts (prost, bindgen,
  tonic-build) compile fully optimized — speeds up the very first build.
- sccache cache dir default: `~/.cache/sccache` (Linux) / `%LOCALAPPDATA%sccache` (Win).
  Set `SCCACHE_DIR` to a fast NVMe path for best results.
- `cargo clean` nukes `target/` (currently 25 GB). Prefer targeted clean:
  `cargo clean -p aphrody` to remove only the CLI crate artifacts.

## sccache cache state (2026-05-18 after first fill)

```
Cache location: Local disk C:UsersyohanAppDataLocalMozillasccachecache
Cache size: 7 GiB (of 10 GiB max)
Compile requests: 668  |  executed: 465  |  misses: 465  |  hit rate: 0% (first fill)
Avg compiler time: 1.3 s/crate
Non-cacheable (incremental mode): 72 crates -- disable via CARGO_INCREMENTAL=0 for sccache
```

Note: sccache and Cargo incremental are mutually exclusive for most crates.
For maximum sccache benefit run with `CARGO_INCREMENTAL=0`.
For local hot iteration, Cargo incremental (the default) is faster.
