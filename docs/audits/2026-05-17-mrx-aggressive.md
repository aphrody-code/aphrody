<!-- SPDX-License-Identifier: Apache-2.0 -->

# mrx Aggressive Audit — 2026-05-17

Tool: `mrx` CLI (`mrx-cli` crate, release build, `x86_64-pc-windows-msvc`)
Build exit code: **0** (Finished `release` profile in 0.84 s, all deps cached offline)
Binary: `target/x86_64-pc-windows-msvc/release/mrx.exe`
Root scanned: `C:\src\aphrody`

## CLI surface (`mrx --help`)

Three subcommands exposed:

| Subcommand | Purpose |
|-----------|---------|
| `scan`    | One-shot audit + monorepo map; writes `path.json` + `monorepo-map.json`; exit 0 always |
| `check`   | Same as `scan`, but exit 1 when findings are detected (CI gate mode) |
| `watch`   | Long-running daemon; re-runs audit on FS events (notify v8, debounced) |

Note: there is no `detect` or `audit` subcommand at the CLI layer. Detection
(`mrx-detect`) and audit logic (`mrx-audit`) are library crates invoked internally
by `scan`/`check`. The task description maps these to the respective library
outputs captured below.

---

## scan

Command: `RUST_LOG=info mrx --root C:\src\aphrody scan`
Exit code: 0
Log line: `scan complete status=Findings submodules=0 workspaces=6 duration_ms=14`

### `monorepo-map.json` — root detection (mrx-detect output)

| Attribute | Value |
|-----------|-------|
| Task runners | `turbo` |
| Package managers | `bun` |
| Lockfiles | `bun.lock`, `Cargo.lock` |
| `has_cargo_workspace` | `true` |
| `has_bun_workspaces` | `true` |
| `has_turbo` | `true` |

### File stats

| Metric | Value |
|--------|-------|
| Total files scanned | 119 |
| Bytes scanned | 16 437 373 |
| Scan duration | 14 ms |
| Languages detected | CSS (1 file), JSON (16), Markdown (5), TypeScript (30) |

### Workspaces detected

6 workspaces, all `node` kind (from `packages/`):
`google-core`, `n2b`, `n2b-plugin`, `n2b-shims`, `n2b-types`, `ui`

No `apps/` directory exists at the repo root — that scan dir produced zero results.

### Scanner blind spot (drift finding)

The scanner strictly walks `apps/` and `packages/`. The 18 Rust crates under
`crates/` (77 `.rs` source files) are **completely invisible** to `mrx`. The
monorepo is hybrid Bun + Cargo, but `mrx scan` produces language stats showing
only TypeScript, JSON, CSS, Markdown — zero Rust. This means no path-hardening
checks, no file-count aggregation, no language stats for the entire Rust side
of the workspace.

---

## detect

`mrx-detect::detect_root` is called inside every `scan`/`check` invocation.
The library performs pure `stat()` calls — no clones, no subprocesses.

Detected correctly:
- `bun.lock` present → `bun` package manager
- `turbo.json` present → `turbo` task runner
- `Cargo.toml` with `[workspace]` → `has_cargo_workspace = true`
- `package.json` with `"workspaces"` array → `has_bun_workspaces = true`
- pnpm, yarn, npm, nx, lerna, deno: all absent, correctly reported false/empty

Drift signal — workspace runtime mismatch:
All 6 detected workspaces report `runtimes: ["node"]`. The `detect_workspace_runtimes`
function adds `"bun"` only when the individual workspace directory contains
`bunfig.toml` or `bun.lock`. None of the `packages/*` dirs carry their own
lockfile, so the root-level Bun setup is not propagated to workspace-level
runtime detection. This causes a misleading report: the project uses Bun
everywhere, but every workspace is labelled as `node`.

---

## audit

Command: `RUST_LOG=info mrx --root C:\src\aphrody check`
Exit code: **1** (findings detected)

### `path.json` findings

| Finding key | Pattern checked | Status | Matches |
|-------------|----------------|--------|---------|
| `absolute_paths` | `/home/ubuntu` | Production Ready | 0 |
| `system_paths` | `/var/www` | Production Ready | 0 |
| `fragile_relative_paths` | `../../../../` | **Findings** | 1 |

**1 total finding:**

```
packages\n2b-plugin\src\ffi.ts
```

Context: `ffi.ts` uses `../../../../..` depth-5 traversal in an FFI path
candidate list to locate `target/release/bun_ffi.dll` from the package source
directory. This is intentional (FFI multi-path fallback), but mrx correctly
flags it as fragile — the path breaks if the package is ever relocated or
published standalone.

### mrx-audit code hygiene findings (source audit)

1. **`unwrap_or_else` / `unwrap_or` used 10 times across mrx-audit and mrx-cli.**
   All usages are either default-value forms (`unwrap_or(false)`, `unwrap_or("")`)
   or error-recovery forms (`unwrap_or_else(|_| fallback)`). None are panic-able
   naked `.unwrap()`. Acceptable under the style guide, but worth documenting.

2. **`#[forbid(unsafe_code)]` missing in all 4 lib crates.**
   `mrx-cli/src/main.rs` carries the attribute (line 15). The library crates
   (`mrx-core`, `mrx-detect`, `mrx-audit`, `mrx-watch`) do not. No unsafe code
   is present in any of them, but the lint guard is absent — a future contributor
   could add `unsafe` without a compile-time gate.

3. **`mrx-core` docstring claims "only serde + chrono" but `Cargo.toml` omits `chrono`.**
   The lib doc at line 5 states "Kept dependency-free (only serde + chrono)."
   The `mrx-core` `Cargo.toml` lists only `serde` as a dependency. `chrono` is
   consumed in `mrx-audit` (which imports `mrx-core::*`) and is declared in
   `mrx-audit/Cargo.toml`. The doc comment is stale/misleading.

4. **`mrx-audit` package description claims rayon but does not depend on it.**
   `Cargo.toml` description: "parallel monorepo audit engine (ignore + rayon + blake3)".
   Module-level doc says "Per-file in parallel via rayon". Actual implementation
   uses `ignore::WalkBuilder::build_parallel()` with its own thread pool — rayon
   is not a dependency, not imported, not used. The description is false.

5. **Windows path separator mismatch — `file_count` and `bytes` are zero for all workspaces.**
   `workspace_key(rel)` returns forward-slash keys (`packages/n2b-plugin`).
   The workspace DashMap is populated with `ws_rel.display().to_string()` keys,
   which on Windows produces backslash paths (`packages\n2b-plugin`). The two
   never match. Every workspace in `monorepo-map.json` reports `file_count: 0`
   and `bytes: 0`. This is a silent data-integrity bug — scan exits 0 with
   structurally valid but numerically wrong output.

---

## verdict

The `mrx` binary builds cleanly (exit 0, 0.84 s offline) and correctly detects
the hybrid Bun + Turbo + Cargo workspace shape of the aphrody repo. The
path-hardening audit surfaces 1 legitimate finding (`packages/n2b-plugin/src/ffi.ts`
uses a depth-5 `../../../../..` FFI path that is intentional but fragile). Beyond
the live finding, the dogfooding run exposed structural gaps in the mrx crates
themselves that reduce trust in the output on Windows.

**Top 5 actionable findings:**

1. **Windows path separator bug in `workspace_key`** (`mrx-audit/src/lib.rs:459`):
   replace `format!("{top}/{name}")` with `Path::new(top).join(name).display().to_string()`
   to use the OS separator, fixing `file_count=0` for all workspaces on Windows.

2. **Scanner is blind to `crates/`**: 77 Rust source files (all `mrx-*`, `cli`,
   `backend`, `base`, etc.) are never walked. Add `crates` to `SCAN_DIRS` and
   handle `Cargo.toml`-only workspace detection for Rust-only packages to make
   the monorepo map complete.

3. **`#[forbid(unsafe_code)]` missing in `mrx-core`, `mrx-detect`, `mrx-audit`,
   `mrx-watch`**: add to each `src/lib.rs` top-of-file to match `mrx-cli`.

4. **Stale `rayon` claim in `mrx-audit` description**: update `Cargo.toml` description
   and module docstring to reference `ignore::WalkBuilder::build_parallel()` instead
   of rayon, which is never imported.

5. **`packages/*` workspaces all labelled `node` despite Bun root**: `detect_workspace_runtimes`
   should propagate parent-level Bun detection (e.g., check for root `bunfig.toml`
   or carry a `has_bun: bool` context) so that child workspaces that rely on the
   root lockfile are correctly labelled `bun`.
