<!-- SPDX-License-Identifier: Apache-2.0 -->
# Photoshop UXP panel → aphrody (user-owned, out-of-policy)

A literal in-app Photoshop plugin panel must be **UXP** (HTML/JS) or a legacy
C++ `.8bf`. aphrody's repo is Rust-only (CLAUDE.md §2), so this JS artifact is
**not committed as source** — it is documented here for you to assemble and
install yourself. The panel is a thin client: every real operation runs in
Rust, in `aphrody-mcp` (`photoshop_*` + `gemini_*` + `firefly_to_photoshop`).

## Architecture

```
Photoshop  ──UXP panel (HTML/JS)──▶  aphrody-mcp (Rust, stdio/HTTP)
                                       ├─ gemini_image / gemini_chat   (Gemini web)
                                       ├─ firefly_generate             (Firefly v3)
                                       ├─ firefly_to_photoshop          (bridge)
                                       └─ photoshop_* (manifest/rendition/ops)
```

The panel never holds credentials or calls Adobe directly — it asks
`aphrody-mcp`, which owns the IMS token (`FIREFLY_CLIENT_ID/SECRET`) and the
Gemini cookie jar.

## `manifest.json` (UXP v5+)

```json
{
  "id": "dev.aphrody.photoshop.panel",
  "name": "aphrody",
  "version": "1.0.0",
  "main": "index.html",
  "manifestVersion": 5,
  "host": [{ "app": "PS", "minVersion": "24.0.0" }],
  "entrypoints": [
    { "type": "panel", "id": "aphrodyPanel", "label": { "default": "aphrody" },
      "minimumSize": { "width": 230, "height": 200 } }
  ],
  "requiredPermissions": {
    "network": { "domains": ["http://localhost:8765"] },
    "localFileSystem": "request"
  }
}
```

## Bridge contract (panel → aphrody-mcp)

Run `aphrody-mcp` behind a tiny local HTTP shim (or use the existing
`aphrody-mcp` HTTP transport) on `localhost:8765`. The panel POSTs an MCP
`tools/call`:

```
POST http://localhost:8765/  { "method":"tools/call",
  "params": { "name":"firefly_to_photoshop",
              "arguments": { "prompt":"<user prompt>", "output_url":"<presigned PUT>",
                             "format":"psd" } } }
```

and places the returned `photoshop_job` output into the document (a generated
PSD imported as a layer, a rendition saved, etc.).

## Install (your machine)

1. Save the `manifest.json` above + an `index.html` (your panel UI) in a folder.
2. Load it with Adobe UXP Developer Tools (`Add Plugin` → select the folder →
   `Load`).
3. Start `aphrody-mcp` and its local HTTP shim before opening the panel.

## Why this split

The capability is fully delivered by aphrody (Rust). The panel is only chrome.
Keeping the JS out of the repo preserves the Rust-only invariant; you own and
install the panel, and can iterate on it without touching aphrody's build.
