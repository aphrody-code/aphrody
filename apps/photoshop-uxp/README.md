<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody — Photoshop UXP bridge panel

A UXP plugin that runs **inside** Adobe Photoshop and exposes its *entire*
internal surface to aphrody. It is the in-app counterpart to the headless cloud
Photoshop API (`aphrody_firefly::photoshop`): where the cloud API offers a
handful of REST operations, this panel drives the **running** Photoshop through
`batchPlay` — the universal ActionDescriptor executor that can perform any menu
command, filter, adjustment, or recorded action — plus an `eval` escape hatch
for the full UXP DOM.

> **Policy note.** aphrody is otherwise 100% Rust (CLAUDE.md §2 bans JS). This
> panel is the **one explicitly-authorized JS artifact**: a UXP plugin must be
> JS, and the maintainer lifted the ban for it. It lives outside the Cargo
> workspace and never participates in the Rust build. All logic that *can* be
> Rust is Rust (the bridge server, the MCP tools); this panel is a thin client.

## Architecture

```
aphrody-mcp (Rust)                         Photoshop (this UXP panel, JS)
  photoshop_live_info     ┐                  ┌ op "info"     → app/doc/layers
  photoshop_live_batchplay├─ ws://localhost:8765 ─┤ op "batchPlay"→ action.batchPlay
  photoshop_live_exec     ┘   (bridge server)   └ op "eval"     → arbitrary UXP JS
```

- The Rust side (`crates/google_mcp/src/photoshop_bridge.rs`) is a local
  WebSocket **server** bound to `127.0.0.1:8765`, started lazily on the first
  `photoshop_live_*` MCP call.
- This panel is the WebSocket **client**: it connects, auto-reconnects, and
  executes each `{ id, op, args }` command, replying `{ id, ok, result }`.
- Write operations run inside `core.executeAsModal` as UXP requires.

## MCP tools (aphrody side)

| Tool | Op | Does |
|---|---|---|
| `photoshop_live_info` | `info` | app version, active document, layer tree |
| `photoshop_live_batchplay` | `batchPlay` | run an ActionDescriptor array — **anything** |
| `photoshop_live_exec` | `eval` | run UXP JS with `app`/`photoshop`/`constants`/`core`/`batchPlay` in scope |

### Example — fill the active layer with red via batchPlay

```json
{ "commands": [
  { "_obj": "fill", "using": { "_enum": "fillContents", "_value": "color" },
    "color": { "_obj": "RGBColor", "red": 255, "green": 0, "blue": 0 },
    "opacity": { "_unit": "percentUnit", "_value": 100 } }
] }
```

Capture the descriptor for any Photoshop action with the **Alchemist** plugin
or legacy **ScriptListener**, then replay it through `photoshop_live_batchplay`.

## Install (your machine)

1. Install **Adobe UXP Developer Tools** (free, from Creative Cloud).
2. *Add Plugin* → select `apps/photoshop-uxp/manifest.json` → *Load*.
3. In Photoshop: **Plugins ▸ aphrody** to open the panel.
4. Start `aphrody-mcp` (it binds the bridge on first `photoshop_live_*` call).
   The panel shows **Connected** once the socket is up.

## Optional: syntax check with bun

```bash
cd apps/photoshop-uxp && bun run check
```

(UXP plugins need no build step; this just parses `main.js`.)
