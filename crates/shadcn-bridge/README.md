<!-- SPDX-License-Identifier: Apache-2.0 -->

# shadcn-bridge

Rust + `wasm-bindgen` bridge that re-creates the [shadcn-ui](https://ui.shadcn.com/)
component prop surface on top of Google's official
[Material Web Components 3](https://github.com/material-components/material-web)
(`@material/web`) custom elements.

> **Why**: per `project_aphrody_ultimate_goals` (memory entry 2026-05-17), the
> aphrody web stack is migrating off React/TS shadcn toward **MWC3 native**
> custom elements, driven by Rust compiled to `wasm32-unknown-unknown`. This
> crate is the binding surface for that migration — every shadcn call-site
> ports to a single `create_<name>(&Props)` Rust function that emits the
> equivalent MWC3 DOM subtree.

## Mapping table

| shadcn primitive | MWC3 element(s)                                                                              |
| ---------------- | -------------------------------------------------------------------------------------------- |
| `Button`         | `<md-filled-button>`, `<md-outlined-button>`, `<md-text-button>`, `<md-tonal-button>`        |
| `Input`          | `<md-outlined-text-field>`, `<md-filled-text-field>`                                          |
| `Card`           | bespoke `<div class="md-elevated-card">` carrying an M3 elevation token                       |
| `Dialog`         | `<md-dialog>`                                                                                 |
| `Tabs`           | `<md-tabs>` + `<md-primary-tab>` / `<md-secondary-tab>`                                      |
| `Toast`          | `<md-snackbar>` (or bespoke `<div class="md-snackbar">` fallback for unsupporting browsers)  |
| `Select`         | `<md-outlined-select>` + `<md-select-option>`                                                |
| `Checkbox`       | `<md-checkbox>`                                                                              |
| `RadioGroup`     | one `<md-radio>` per option, grouped by shared `name`                                        |
| `Switch`         | `<md-switch>`                                                                                |
| `Slider`         | `<md-slider>`                                                                                |
| `Avatar`         | bespoke `<div class="md-avatar">` wrapping `<img>` + M3 elevation                            |

## API shape

Each module exposes three artefacts:

1. A `<Name>Props` struct (`serde`-friendly) mirroring the shadcn prop surface
   (PascalCase → snake_case).
2. A pure-Rust `create_<name>(props: &<Name>Props) -> Result<HtmlElement, JsValue>`
   entrypoint that builds the DOM subtree via `web_sys`.
3. A `#[wasm_bindgen]`-annotated JS-facing wrapper that accepts a JSON string
   (so JS callers do not need to import `serde-wasm-bindgen`).

## Usage (Rust)

```rust,no_run
use shadcn_bridge::button::{ButtonProps, create_button};

let props = ButtonProps {
    label: "Save".into(),
    variant: "filled".into(),
    disabled: false,
    on_click_id: Some("save-btn".into()),
};

// `el` is a `web_sys::HtmlElement` ready to mount under any parent node.
let el = create_button(&props).expect("DOM available");
```

## Usage (JS, post-`wasm-pack build`)

```js
import init, { create_button } from "./pkg/shadcn_bridge.js";

await init();

const el = create_button(JSON.stringify({
    label: "Save",
    variant: "filled",
    disabled: false,
    on_click_id: "save-btn",
}));

document.body.appendChild(el);
```

## Build targets

This crate is built into the aphrody web bundle as `wasm32-unknown-unknown`.
Native targets (`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) also
compile — the wasm-only DOM logic is `#[cfg(target_arch = "wasm32")]`-gated so
host builds exercise only the `*Props` structs (covered by `cargo test`).

```bash
# WASM (the primary deliverable):
cargo check -p shadcn-bridge --target wasm32-unknown-unknown --locked

# Native host check (Props structs + tests only):
cargo check -p shadcn-bridge --locked
cargo test  -p shadcn-bridge --locked
```

## License

Apache-2.0 — see `LICENSE` at the repo root.
