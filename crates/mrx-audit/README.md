<!-- SPDX-License-Identifier: Apache-2.0 -->

# mrx-audit

Parallel monorepo audit engine for the **MRX** (Monorepo Real-time X-platform
mapper) family.

A single `run()` produces both a hardening-style `path.json` audit report and
an exhaustive `monorepo-map.json` snapshot, written atomically (`.tmp` +
rename). Designed to be invoked directly from `mrx-cli` or repeatedly from
`mrx-watch` on every debounced FS event without re-initialising thread pools.

## Install

```toml
[dependencies]
mrx-audit = "1.0.0-canary"
```

## Quick start

```rust
use std::path::Path;
use mrx_audit::run;

let result = run(
    Path::new("."),
    Path::new("path.json"),
    Path::new("monorepo-map.json"),
)?;

println!("status: {:?}", result.status);
println!("findings: {}", result.total_findings);
println!("workspaces: {} | submodules: {}", result.workspaces, result.submodules);
println!("scan took {} ms", result.duration_ms);
# Ok::<(), anyhow::Error>(())
```

The return type is `mrx_core::RunResult`.

## Public API

Full surface lives in [`src/lib.rs`](src/lib.rs):

- `run(root, audit_out, map_out) -> anyhow::Result<RunResult>` — the only
  public entry point. Everything else (file walking, per-workspace bucketing,
  hashing, atomic write) is encapsulated.

## Pipeline

In order, every `run()` invocation:

1. Parses `.gitmodules` and enriches each entry via `git submodule status`.
2. Calls `mrx_detect::detect_root` to classify the repo root.
3. Walks `apps/` and `packages/` in parallel with `ignore::WalkBuilder`
   (ripgrep's engine — respects `.gitignore`, `.ignore`, global excludes).
4. For each file in parallel: pattern matching, language detection by
   extension, size aggregation, per-workspace stat bucketing.
5. Computes a `blake3` content hash over the root config files
   (`turbo.json`, `package.json`, `bun.lock`, `Cargo.toml`, `.gitmodules`,
   plus pnpm / yarn / nx / lerna / deno variants).
6. Atomically writes both JSON outputs.

## Audit rules

Three pattern findings are emitted to `path.json` (each becomes `Findings`
when any match occurs, `Production Ready` otherwise):

- **`absolute_paths`** — files containing `/home/ubuntu`.
- **`system_paths`** — files containing `/var/www`.
- **`fragile_relative_paths`** — files containing `../../../../`.

Scanning is skipped for `node_modules/`, `.next/`, `.turbo/`, `.bun-cache/`,
`target/`, `dist/`, `build/`, `.git/`, `.cache/`, and noisy glob patterns
(`*.log`, `*.tmp`, `*.swp`, `*.swx`, `*.lock`, `*.min.js`, `*.min.css`). Files
larger than 1 MB or non-textual extensions skip the content-pattern pass.

## License

Apache-2.0

## Related

- [`mrx-core`](../mrx-core) — types serialised to `path.json` / `monorepo-map.json`.
- [`mrx-detect`](../mrx-detect) — root and per-workspace classification.
