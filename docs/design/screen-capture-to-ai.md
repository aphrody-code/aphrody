<!-- SPDX-License-Identifier: Apache-2.0 -->
# Screen / window capture → AI (aphrody)

A feature that captures the whole screen (or a single window) and hands the
image straight to a vision model — either to the MCP client's model (via an
`aphrody-mcp` tool) or directly to the internal Gemini 3.5 Flash backend.

## 1. Capture layer — `crates/aphrody-capture`

Pure capture + PNG encode, returns `Vec<u8>` (PNG). Backends, in order of
preference:

| Backend | Platform | Status | Notes |
|---|---|---|---|
| **Win32 GDI** (`windows` crate) | Windows | implemented | `BitBlt` of the virtual screen / a window HDC → `GetDIBits` (BGRA) → `image` PNG. No external dep (the `windows` crate is already cached; only feature flags added — offline-safe). |
| `xcap` 0.x | Win/macOS/Linux | deferred | The clean cross-platform path, but **not in the offline registry cache** — cannot be added under the workspace's offline build policy. Adopt when the registry is refreshed. |
| CDP / WebView2 | any (browser only) | exists | `aphrody-terminal-browser` already screenshots browser pages; not general OS capture. |

Surface (`#[cfg(windows)]`; other targets return `CaptureError::Unsupported`):
- `capture_primary_screen() -> Result<Vec<u8>>` — the primary monitor as PNG.
- `capture_virtual_screen() -> Result<Vec<u8>>` — all monitors (the full
  desktop bounding box).
- `capture_window_by_title(substr) -> Result<Vec<u8>>` — first top-level window
  whose title contains `substr` (case-insensitive), via `FindWindow`/`PrintWindow`.
- `list_windows() -> Vec<WindowInfo>` — enumerate visible top-level windows
  (title + handle) so the caller can pick one.

BGRA→RGBA + top-down row handling is pure logic and unit-tested; the GDI calls
are `unsafe` Win32 wrapped in RAII guards that delete DCs/bitmaps on drop.

## 2. Transmission to the AI

### Path A — via MCP (implemented): the calling model has vision
`aphrody-mcp` tools (in `google_mcp`):
- `screen_capture { window?: string, save_path?: string }` → captures the
  screen (or the named window) and returns
  `{ mime: "image/png", base64: "<…>", width, height, saved_path? }`.
  The MCP client (Claude / any vision model) decodes the base64 and *sees* the
  screenshot. This needs no Gemini-upload RE and works today.
- Privacy: capture is local; the base64 is returned only to the MCP client the
  user already trusts. Never written off-box.

### Path B — direct to internal Gemini 3.5 Flash (designed)
Feed the PNG to the `gemini-web` SDK as a vision input. The Gemini web
`StreamGenerate` `inner_list` has a file-data slot
(`[prompt, 0, null, req_file_data, …]`): images are first uploaded (Scotty-style
resumable upload to `push.clients6.google.com` / the `/upload/` endpoint),
yielding an upload id that is referenced in `req_file_data` as
`[[uploadId], filename, mime]`. This upload wire is **not yet byte-confirmed**
(it needs the same live capture treatment as the send path). Until then, Path A
covers the "transmit to the AI" requirement; Path B is the follow-up
(`gemini-web` `upload_image()` + `ask_with_image()`), tracked here.

### Path C — UI integration
The Google-app-desktop bar (`docs/UI-GOOGLE-APP-DESKTOP.md`) gets a capture
affordance (the `+` menu / a hotkey): capture → Path A or B → reply in the bar.

## 3. CLI
`aphrody capture screen --out shot.png` / `aphrody capture window "<title>" --out
shot.png` (feature `capture`, host-only) — capture to disk, and with `--ask
"<prompt>"` route through the gemini-web vision path (Path B) once wired.

## 4. Security / privacy
- Capture is strictly local; bytes leave the machine only on an explicit
  `--ask` (to Google, the user's own session) or when the MCP client forwards
  them — same trust boundary as any MCP tool result.
- No always-on capture; every shot is an explicit call.
- Window titles enumerated by `list_windows` may leak app names — returned only
  to the local caller.
