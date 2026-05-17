<!-- SPDX-License-Identifier: Apache-2.0 -->

# mrx-watch

Long-running cross-platform file-system watcher for the **MRX**
(Monorepo Real-time X-platform mapper) family.

Watches a repository root for relevant changes, debounces the firehose, then
re-runs `mrx_audit::run` to refresh `path.json` and `monorepo-map.json` in
place. The audit itself runs in `tokio::task::spawn_blocking` so the watcher
loop keeps draining events while the scan is in flight, and overlapping
triggers are deduplicated via an in-flight flag.

## Install

```toml
[dependencies]
mrx-watch = "1.0.0-canary"
```

## Quick start

```rust
use std::path::Path;
use mrx_watch::run;

// Blocks until SIGINT / SIGTERM (Unix) or Ctrl+C (Windows).
run(
    Path::new("."),
    Path::new("path.json"),
    Path::new("monorepo-map.json"),
    750, // debounce window in ms
)?;
# Ok::<(), anyhow::Error>(())
```

`run` performs one immediate audit pass on startup so the JSON outputs exist
before the first event arrives, then attaches watchers to `apps/`, `packages/`
(recursive) and to `.gitmodules`, `turbo.json`, `package.json`, `bun.lock`,
`Cargo.toml`, `pnpm-workspace.yaml` (non-recursive).

## Public API

Full surface lives in [`src/lib.rs`](src/lib.rs):

- `run(root, audit_out, map_out, debounce_ms) -> anyhow::Result<()>` — builds
  a multi-threaded Tokio runtime (`thread_name = "mrx-watch"`), runs the
  initial audit, then drives the debounced watch loop until shutdown.

## Backend

Backed by [`notify`](https://crates.io/crates/notify) v8 plus
[`notify-debouncer-full`](https://crates.io/crates/notify-debouncer-full). Per
operating system:

| OS      | Backend                  |
|---------|--------------------------|
| Linux   | `inotify`                |
| macOS   | `FSEvents`               |
| Windows | `ReadDirectoryChangesW`  |
| `*BSD`  | `kqueue`                 |
| other   | polling fallback         |

Events are debounced by `notify-debouncer-full`, then coalesced and filtered
by a noisy-path predicate before triggering a re-audit. Skipped path segments:
`node_modules`, `.next`, `.turbo`, `.bun-cache`, `target`, `dist`, `build`,
`.git`, `.cache`. Skipped extensions: `.log`, `.tmp`, `.swp`, `.swx`.

Throughput notes: every backend has a per-process inotify-watch / handle
budget — keep the watch root scoped to the monorepo, and prefer increasing
`debounce_ms` (default `750`) over widening the watched set when an editor
triggers thousands of writes per second (e.g. webpack bundle output).

## License

Apache-2.0

## Related

- [`mrx-core`](../mrx-core) — shared data model.
- [`mrx-cli`](../mrx-cli) — `mrx watch` subcommand wraps this entry point.
