<!-- SPDX-License-Identifier: Apache-2.0 -->

# aphrody-terminal-browser

LLM-to-DOM bridge for `aphrody-terminal`. Parses `aphrody-browser-*` OSC
sequences from the terminal event bus and dispatches them to a pluggable
browser backend.

## Backends

| Backend | Binary | Transport | Capability |
|---|---|---|---|
| `bxc` | `bxc` | stdin/stdout JSON-RPC 2.0 | Fast scrape, static + light JS, no GPU |
| `agent-browser` | `agent-browser` | stdin/stdout JSON-RPC 2.0 | Full Chromium via CDP |
| `edge` | `msedge` | process spawn | DOM snapshot only (Windows fallback) |

`Active::probe()` selects bxc first, then agent-browser, then edge.

## OSC sequence table

| Sequence | Payload | Dispatched call |
|---|---|---|
| `\e]aphrody-browser-nav;<url>\a` | Raw URL | `navigate(url)` |
| `\e]aphrody-browser-eval;<b64-js>\a` | base64 JS source | `eval_js(src)` |
| `\e]aphrody-browser-dom;<b64-selector>\a` | base64 CSS selector | `query_selector(sel)` |
| `\e]aphrody-browser-screenshot;viewport\a` | Literal `viewport` | `screenshot(Viewport)` |
| `\e]aphrody-browser-screenshot;element:<sel>\a` | `element:` prefix + selector | `screenshot(Element)` |
| `\e]aphrody-browser-screenshot;fullpage\a` | Literal `fullpage` | `screenshot(Fullpage)` |
| `\e]aphrody-browser-intercept;<b64-json-rule>\a` | base64 JSON rule | `intercept(rule)` |
| `\e]aphrody-browser-extract;<b64-json-schema>\a` | base64 JSON schema | `extract(schema)` |
| `\e]aphrody-browser-record;<id>;start\a` | `<id>;start` | `record(id, Start)` |
| `\e]aphrody-browser-record;<id>;stop\a` | `<id>;stop` | `record(id, Stop)` |

Both BEL (`\x07`) and ST (`\x1b\\`) terminators are accepted.

## Usage

```rust
use aphrody_terminal_browser::{Active, osc::parse_aphrody_browser_osc};

let mut backend = Active::probe().await?;
if let Some(req) = parse_aphrody_browser_osc(raw_osc_bytes) {
    let response = backend.dispatch(req).await?;
}
```
