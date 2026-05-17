<!-- SPDX-License-Identifier: Apache-2.0 -->

# mrx-detect

Root-shape and per-workspace runtime detection for the **MRX**
(Monorepo Real-time X-platform mapper) family.

Given a directory path, it answers two questions in a single `stat()`-only pass:

1. **What does this repo root look like?** — Bun? Turborepo? pnpm? Cargo
   workspace? Several at once? (Yes — modern repos routinely combine
   Turbo + Bun + Cargo for napi-rs crates.)
2. **What runtime drives this individual workspace?** — `bun`, `cargo`,
   `turbo`, `node`, `deno`.

Both calls are pure I/O detection — no git clone, no package install, no
network. Safe to call from a watcher loop on every FS event.

## Install

```toml
[dependencies]
mrx-detect = "1.0.0-canary"
```

## Quick start

```rust
use std::path::Path;
use mrx_detect::{detect_root, detect_workspace_runtimes};

let root = Path::new(".");
let kind = detect_root(root);
println!("task runners: {:?}", kind.task_runners);
println!("package managers: {:?}", kind.package_managers);
println!("cargo workspace? {}", kind.has_cargo_workspace);

let ws_runtimes = detect_workspace_runtimes(&root.join("packages/foo"));
println!("workspace runtimes: {:?}", ws_runtimes);
```

## Public API

Full surface lives in [`src/lib.rs`](src/lib.rs):

- `detect_root(root: &Path) -> RootKind` — populates lockfiles, package
  managers, task runners, and `has_*` flags in one pass.
- `detect_workspace_runtimes(dir: &Path) -> Vec<String>` — emits the runtime
  list (`bun`, `turbo`, `cargo`, `node`, `deno`) for a single workspace
  directory.

The returned `RootKind` is defined in [`mrx-core`](../mrx-core) so producers
and consumers share the same type without re-marshalling.

## Detection coverage

Lockfiles recognised: `bun.lock`, `bun.lockb`, `package-lock.json`,
`pnpm-lock.yaml`, `yarn.lock`, `Cargo.lock`, `deno.lock`.

Configs / manifests recognised: `bunfig.toml`, `turbo.json` / `turbo.jsonc`,
`pnpm-workspace.yaml`, `lerna.json`, `nx.json`, `deno.json` / `deno.jsonc`,
root `Cargo.toml` with `[workspace]`, root `package.json` with
`"workspaces": [...]`.

Detection is additive — a Turbo-on-Bun-with-Cargo-napi repo yields
`task_runners = ["turbo"]`, `package_managers = ["bun"]`, plus
`has_cargo_workspace = true` and `has_bun_workspaces = true` simultaneously.

## License

Apache-2.0

## Related

- [`mrx-core`](../mrx-core) — shared data model (`RootKind`, `Workspace`, etc.).
- [`mrx-audit`](../mrx-audit) — calls `detect_root` + `detect_workspace_runtimes`
  during each audit pass.
