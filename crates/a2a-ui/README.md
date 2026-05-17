# SPDX-License-Identifier: Apache-2.0
# a2a-ui

WASM channel viewer for the A2A file-based `.coord` mailbox protocol
(`ai.json` v1, `inbox-from-*.jsonl`).

Consumes a JSON array of A2A envelopes and renders a live channel view in
the browser using `wasm-bindgen` + DOM APIs.  No JavaScript framework
dependency — pure Rust compiled to WASM.

## Quick start

### 1. Prerequisites

- Rust nightly (pinned via `rust-toolchain.toml` at repo root).
- `wasm-pack` — install once:

  ```sh
  cargo binstall wasm-pack
  # or
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
  ```

- A static file server.  `bun` works well:

  ```sh
  # Install bun once: https://bun.sh
  bun x serve .
  ```

### 2. Build

Run from the workspace root or from `crates/a2a-ui/`:

```sh
# From workspace root
wasm-pack build crates/a2a-ui --target web --out-dir crates/a2a-ui/pkg

# Or from the crate directory
cd crates/a2a-ui
wasm-pack build --target web
```

This emits `pkg/a2a_ui.js` + `pkg/a2a_ui_bg.wasm` (and an npm
`package.json`).

### 3. Run the example

```sh
cd crates/a2a-ui/examples
bun x serve .
# Open http://localhost:3000/channel-viewer.html
```

The page fetches `envelopes.json` (three sample envelopes bundled in
`examples/`), passes the raw JSON to the WASM module, and renders a styled
channel view.

## Public API

```rust
/// One-time panic hook install — call before any other function.
pub fn init_panic_hook();

/// Parse `json` as Vec<Envelope> and inject DOM nodes into `container_id`.
pub fn render_envelope_list(container_id: &str, json: &str) -> Result<(), JsValue>;
```

### Envelope format

```json
{
  "v": 1,
  "ts": "2026-05-17T20:02:27Z",
  "from": "aphrody",
  "to": "winclean",
  "kind": "fact",
  "topic": "short subject line",
  "body": "free-form content"
}
```

Legacy envelopes (pre-v1) that use `type` instead of `kind` and `subject`
instead of `topic` are handled transparently via `#[serde(alias)]`.

## Workspace integration

The crate is not yet in `workspace.members` — the orchestrator wires that
post-batch.  Until then, use `--manifest-path` directly:

```sh
cargo check --manifest-path crates/a2a-ui/Cargo.toml \
    --target wasm32-unknown-unknown --locked
```

## Cross-compilation gates

The crate compiles as a pure `rlib` on native targets (Linux, Windows) so
that `cargo check --workspace` never breaks the non-WASM CI path.  All DOM
and browser APIs are gated behind `#[cfg(target_arch = "wasm32")]`.
