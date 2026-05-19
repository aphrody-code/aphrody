<!-- SPDX-License-Identifier: Apache-2.0 -->
# DEV-ENV — Aphrody developer environment reference

> Workspace: 40+ crates Rust + Bun packages | Targets: Linux #1, Win11, WASM
> Sccache 0.15.0+ | Cargo nightly-2026-05-17 | Bun 1.3.14

## TL;DR — Quick bootstrap

**Windows (PowerShell)** :
```pwsh
pwsh -File scripts/setup-dev-env.ps1            # prompt + apply
pwsh -File scripts/setup-dev-env.ps1 -Check     # dry-run
pwsh -File scripts/setup-dev-env.ps1 -Apply     # silent apply
pwsh -File scripts/setup-dev-env.ps1 -Reset     # remove aphrody-specific vars
```

**Linux / macOS / git-bash** :
```bash
bash scripts/setup-dev-env.sh            # writes ~/.bashrc.d/aphrody.sh
bash scripts/setup-dev-env.sh --check    # dry-run
bash scripts/setup-dev-env.sh --reset    # remove
```

After apply : restart your shell, then verify with `cargo dev-fast`.

## Environment variables (User scope)

| Variable                              | Value (Win)                                       | Value (Unix)                              | Purpose                                                |
|---------------------------------------|---------------------------------------------------|-------------------------------------------|--------------------------------------------------------|
| `CARGO_INCREMENTAL`                   | `0`                                               | `0`                                       | Required for sccache (mutually exclusive)              |
| `SCCACHE_DIR`                         | `C:\sccache`                                      | `~/.cache/sccache`                        | Fast NVMe path for compile cache                       |
| `SCCACHE_CACHE_SIZE`                  | `10G`                                             | `10G`                                     | Cache size cap                                         |
| `CARGO_BUILD_JOBS`                    | `7`                                               | `7`                                       | 8 cores - 1 for linker + sccache                       |
| `CARGO_NET_GIT_FETCH_WITH_CLI`        | `true`                                            | `true`                                    | Use system git for fetch (faster on Win)               |
| `CARGO_TERM_COLOR`                    | `always`                                          | `always`                                  | Color output in CI logs                                |
| `RUST_BACKTRACE`                      | `1`                                               | `1`                                       | Backtraces by default                                  |
| `BUN_RUNTIME_TRANSPILER_CACHE_PATH`   | `%USERPROFILE%\.bun-transpile-cache`              | `~/.cache/bun-transpile`                  | Bun TS transpile cache (resolved per-user by `scripts/setup-dev-env.ps1`) |
| `APHRODY_A2A_ENDPOINT`                | `http://localhost:8788/jsonrpc`                   | (same)                                    | Default A2A coord JSON-RPC endpoint                    |
| `APHRODY_LIVE_BACKEND`                | `gemini-oauth`                                    | (same)                                    | Default gemini-live backend                            |

## User-managed (script does NOT touch)

| Variable                              | Expected                                          | Notes                                                  |
|---------------------------------------|---------------------------------------------------|--------------------------------------------------------|
| `CARGO_HOME`                          | `~/.cargo` (default)                              | If set to a non-existent path, cargo falls back        |
| `RUSTUP_HOME`                         | `~/.rustup` (default)                             | Same                                                   |
| `BUN_INSTALL`                         | `~/.bun`                                          | Where bun binaries live                                |
| `GITHUB_PERSONAL_ACCESS_TOKEN`        | `gho_***` (per `gh auth token`)                   | Rotate via `gh auth refresh` if leaked                 |

## Known drift on yohan@aphrody (2026-05-18 audit)

- `CARGO_HOME=D:\cargo` and `RUSTUP_HOME=D:\rustup` are set but **D: drive does not exist**
  on this machine (only C:\ with ~50 GB free). Cargo falls back to default
  `~/.cargo` which is why everything still works — but the env vars are stale.
  If you intentionally moved cargo to D:, plug the drive back in. Otherwise
  remove the env vars : `[Environment]::SetEnvironmentVariable('CARGO_HOME', $null, 'User')`.
- `BUN_INSTALL=D:\bun` — same status.

## .cargo/config.toml — 2026 best practices applied

See [`.cargo/config.toml`](../../.cargo/config.toml) for the full file. Key flags :

- **rustc-wrapper = "sccache"** : portable (PATH-resolved), not a hardcoded
  Windows path. Works on every host with sccache installed.
- **rustflags `-Z threads=8`** : parallel rustc frontend (nightly-only). The
  single biggest local-iteration speedup.
- **rustflags `-Z share-generics=y`** : reduces mono duplication across crates.
- **`[profile.dev.package."*"] opt-level = 1`** (in root `Cargo.toml`) : deps
  compile with -O1 in dev. Removes the "debug build is 10x slower" problem.
- **`[profile.dev.build-override] opt-level = 3`** (root) : build scripts
  compile fully optimized. Speeds up the very first cold build dramatically.
- **`[registries.crates-io] protocol = "sparse"`** : new resolver, fewer
  network roundtrips, faster `cargo update`.
- **`[net] git-fetch-with-cli = true`** : delegates to system git, much
  faster on Windows where the built-in libgit2 path is slow.

## Cargo aliases — cheat sheet

| Alias                | Resolves to                                                                | Use-case                                |
|----------------------|----------------------------------------------------------------------------|-----------------------------------------|
| `cargo dev-fast`     | check workspace, message-format=short, offline                             | Sub-5s warm iteration                   |
| `cargo lint-fast`    | clippy workspace, -D warnings, offline                                     | Pre-push lint gate                      |
| `cargo bench-fast`   | bench workspace, no-default-features                                       | Quick perf-regression check             |
| `cargo build-fast`   | build --release -p aphrody                                                 | Local binary refresh                    |
| `cargo test-fast`    | nextest run --no-fail-fast                                                 | Full test suite, parallel               |
| `cargo doc-fast`     | doc --workspace --no-deps                                                  | Docs without dep recompile              |
| `cargo dup-deps`     | tree --workspace --duplicates                                              | Catch version drift                     |
| `cargo fmt-check`    | fmt --all -- --check                                                       | Sub-second fmt gate                     |
| `cargo outdated-w`   | outdated --workspace --root-deps-only                                      | Find outdated deps                      |
| `cargo upd-workspace`| update --workspace                                                         | Lockfile refresh, workspace only        |
| `cargo smoke`        | check -p aphrody --locked --offline                                        | Sub-30s warm binary check               |
| `cargo future-report`| report future-incompatibilities                                            | Warn on edition deprecations            |
| `cargo ci-offline`   | clippy workspace --locked --offline -- -D warnings                         | CI lint gate (hermetic)                 |
| `cargo xt-offline`   | nextest run --workspace --locked --offline                                 | CI test gate (hermetic)                 |
| `cargo build-dist`   | build --profile dist --workspace --locked                                  | Release-grade build                     |

Run `cargo --list` for the full set.

## Sccache verification

```bash
sccache --show-stats                 # current hits/misses
sccache --stop-server                # force restart
RUSTC_WRAPPER="" cargo dev-fast      # bypass sccache for one invocation
```

Expected hit rate after a warm workspace : >70%. Cold-fill takes ~3-5 min on
this machine size.

## Bun cache

```bash
bun pm cache rm                      # clear bun install cache
bun --bun tsc --noEmit               # use bun's TS transpiler (faster)
```

The `BUN_RUNTIME_TRANSPILER_CACHE_PATH` env var caches the transformed JS so
repeated `bun run X.ts` invocations skip re-parsing.

## Recommendations

1. **Run the setup script once after cloning** : `pwsh -File scripts/setup-dev-env.ps1 -Apply`.
2. **Restart your shell** after the apply — User scope vars don't propagate
   to running shells.
3. **Periodically purge sccache** if disk runs low : `rm -rf $SCCACHE_DIR/*`.
4. **If on Linux** : install `mold` linker for an extra ~2-3x linker speedup
   (not yet wired in `.cargo/config.toml` — open follow-up).
5. **If on Windows** : ensure `link.exe` from VS 2026 Insiders is on PATH
   (the workspace `.cargo/config.toml` hardcodes the absolute path as a
   fallback if PATH doesn't reach the MSVC bin dir).

## Related docs

- [`BUILD-SPEED.md`](BUILD-SPEED.md) — measured sccache + cargo speedups
- [`PIPELINE-OPTIMIZATION.md`](PIPELINE-OPTIMIZATION.md) — CI pipeline tuning
- [`PROFILES.md`](PROFILES.md) — Cargo profile reference (dev/release/dist/asan/release-fast)
- [`PUBLISH-LADDER.md`](PUBLISH-LADDER.md) — crates.io publish order
- [`CHEATSHEET.md`](CHEATSHEET.md) — daily cargo commands
