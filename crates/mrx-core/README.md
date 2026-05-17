<!-- SPDX-License-Identifier: Apache-2.0 -->

# mrx-core

Foundation crate for the **MRX** (Monorepo Real-time X-platform mapper) family.
It exposes the shared, dependency-light data model that every downstream mrx
crate (`mrx-detect`, `mrx-audit`, `mrx-watch`, `mrx-cli`) serialises to disk as
`path.json` (audit report) and `monorepo-map.json` (workspace snapshot).

Kept intentionally thin: only `serde` is pulled in so the type system can flow
freely through the rest of the pipeline without dragging in heavy build deps.

## Install

```toml
[dependencies]
mrx-core = "1.0.0-canary"
```

## Quick start

```rust
use mrx_core::{RootKind, Workspace, WorkspaceKind, LangStat};

let mut root = RootKind::default();
root.task_runners.push("turbo".into());
root.package_managers.push("bun".into());
root.has_cargo_workspace = true;

let ws = Workspace {
    path: "crates/cli".into(),
    kind: WorkspaceKind::Rust,
    runtimes: vec!["cargo".into()],
    name: Some("aphrody".into()),
    version: Some("1.0.0-canary".into()),
    file_count: 0,
    bytes: 0,
    languages: Default::default(),
};
assert_eq!(ws.kind, WorkspaceKind::Rust);
```

## Public API

Full surface lives in [`src/lib.rs`](src/lib.rs). Highlights:

- **Audit report types** — `AuditReport`, `Scope`, `FindingGroup`,
  `InfraExceptions`, `SubmoduleSection`, `Submodule`, `Status`.
- **Monorepo map types** — `MonorepoMap`, `RootKind`, `MapStats`, `LangStat`,
  `Workspace`, `WorkspaceKind`.
- **Run summary** — `RunResult` returned by `mrx_audit::run` and surfaced by
  the CLI.

`WorkspaceKind` is a tri-state (`Node`, `Rust`, `Hybrid`) that lets a single
directory be both a `package.json` and `Cargo.toml` member — exactly the shape
napi-rs and Tauri crates take.

## Cross-platform

Per-platform path separators are normalised to forward slashes when used as
map keys downstream. `mrx-audit::process_file` converts `display()` output via
`replace('\\', "/")` so workspace bucketing stays consistent between Windows
`ReadDirectoryChangesW` events and Linux `inotify` payloads — `mrx-core`
itself stays separator-agnostic.

## License

Apache-2.0

## Related

- [`mrx-detect`](../mrx-detect) — root-shape detection (Bun, Turbo, pnpm, Cargo).
- [`mrx-audit`](../mrx-audit) — parallel audit engine consuming these types.
- [`mrx-watch`](../mrx-watch) — long-running watcher re-running audits on FS events.
- [`mrx-cli`](../mrx-cli) — `mrx` binary entry point.
