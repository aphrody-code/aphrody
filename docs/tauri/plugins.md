<!-- SPDX-License-Identifier: Apache-2.0 -->
# Tauri v2 plugins & packages for aphrody — catalogue + curated set

Research deliverable (read-only). Catalogues the official Tauri v2 plugins and
the notable community plugins/packages, then recommends a curated set for
`crates/aphrody-app` (the build-excluded Tauri shell that calls
`aphrody::run_async` in-process — see [`aphrody-integration.md`](aphrody-integration.md)
and the verdict in [`README.md`](README.md)).

Sources: the official plugins monorepo cloned at `var/tauri-plugins` (gitignored,
`git clone --depth 1 https://github.com/tauri-apps/plugins-workspace`), each
plugin's `Cargo.toml` / `README.md` / `permissions/`, plus crates.io and the
`tauri-apps/awesome-tauri` index (verified 2026-05-24). Tauri core read at
`var/tauri` tag v2.11.2.

The single most important framing for aphrody: **aphrody is already a powerful
system CLI in Rust** (forensics, reverse-engineering, filesystem scan via
walkdir/ignore/rayon, process, messaging on Slack/Telegram/Matrix/Discord/X,
voice, chrome/cookies, DNS, HTTP). All of that system interaction is **already
covered by the CLI core and reached through `#[tauri::command]` -> `aphrody::run_async`**.
A Tauri plugin must therefore earn its place by providing something **only the
webview shell can do** (a native window, tray, OS notification, global hotkey,
clipboard, native file dialog, updater, autostart, single-instance, deep-link,
OS info, window-state, log bridge) — **not** by duplicating fs/shell/process/scan/
network that the Rust CLI already owns.

---

## A. Catalogue — official Tauri v2 plugins

License for every official plugin is **`Apache-2.0 OR MIT`** (workspace-inherited,
`var/tauri-plugins/Cargo.toml:38` -> `license = "Apache-2.0 OR MIT"`). Versions
read from each plugin's `Cargo.toml` in `var/tauri-plugins/plugins/<name>/`
(2026-05-24). **Zero GPL/LGPL** in the Rust tree (the LGPL is only in the
dynamically-linked system WebKitGTK/GTK3 C libs on Linux — see §D and
[`risks.md`](risks.md) §3).

Cross-platform columns are the canonical support tables from each plugin's
`README.md` (`L`=Linux, `W`=Windows, `M`=macOS, `A`=Android, `i`=iOS;
`-` = unsupported). "Capability ID" is the plugin's permission namespace
(`<plugin>:<permission>`); `allow-*`/`deny-*` identifiers read from
`var/tauri-plugins/plugins/<name>/permissions/`.

| Plugin (crate) | Role | L | W | M | A | i | Capability namespace + key permissions | Version |
|----------------|------|---|---|---|---|---|----------------------------------------|---------|
| **fs** (`tauri-plugin-fs`) | Scoped file system access from the webview (read/write/meta, watch), path-scoped. | Y | Y | Y | Y | Y | `fs:` — `allow-read-file`, `allow-write-file`, `allow-mkdir`, per-base-dir `allow-{appdata,appconfig,home,…}-{read,write,meta}[-recursive]`; default `fs:default` | 2.5.1 |
| **dialog** (`tauri-plugin-dialog`) | Native OS open/save/message/ask/confirm dialogs. | Y | Y | Y | Y | Y | `dialog:` — `allow-open`, `allow-save`, `allow-message`, `allow-ask`, `allow-confirm` | 2.7.1 |
| **shell** (`tauri-plugin-shell`) | Spawn child processes / open URLs with the default app. | Y | Y | Y | Y | Y | `shell:` — `allow-execute`, `allow-spawn`, `allow-open`, `allow-kill`, `allow-stdin-write` (scoped command allowlist) | 2.3.5 |
| **opener** (`tauri-plugin-opener`) | Open files/URLs in the default app; reveal in file manager. Successor to shell-open. | Y | Y | Y | Y | Y | `opener:` — `allow-open-url`, `allow-open-path`, `allow-reveal-item-in-dir`, `allow-default-urls` | 2.5.4 |
| **notification** (`tauri-plugin-notification`) | Native desktop + mobile notifications (channels, actions, permission state). | Y | Y | Y | Y | Y | `notification:` — `allow-notify`, `allow-request-permission`, `allow-is-permission-granted`, mobile channels/actions; default `notification:default` | 2.3.3 |
| **global-shortcut** (`tauri-plugin-global-shortcut`) | System-wide hotkey registration. | Y | Y | Y | - | - | `global-shortcut:` — `allow-register`, `allow-register-all`, `allow-unregister`, `allow-unregister-all`, `allow-is-registered` | 2.3.1 |
| **clipboard-manager** (`tauri-plugin-clipboard-manager`) | Read/write system clipboard (text, html, image). | Y | Y | Y | Y | Y | `clipboard-manager:` — `allow-read-text`, `allow-write-text`, `allow-read-image`, `allow-write-image`, `allow-write-html`, `allow-clear` | 2.3.2 |
| **os** (`tauri-plugin-os`) | OS info: platform, arch, version, family, hostname, locale. | Y | Y | Y | Y | Y | `os:` — `allow-platform`, `allow-version`, `allow-os-type`, `allow-arch`, `allow-family`, `allow-hostname`, `allow-locale`, `allow-exe-extension` | 2.3.2 |
| **process** (`tauri-plugin-process`) | Exit/relaunch the app process. | Y | Y | Y | - | - | `process:` — `allow-exit`, `allow-restart` | 2.3.1 |
| **updater** (`tauri-plugin-updater`) | In-app self-update: poll, download, verify signature, install. | Y | Y | Y | - | - | `updater:` — `allow-check`, `allow-download`, `allow-install`, `allow-download-and-install` | 2.10.1 |
| **window-state** (`tauri-plugin-window-state`) | Persist + restore window position/size across launches. | Y | Y | Y | - | - | `window-state:` — `allow-save-window-state`, `allow-restore-state`, `allow-filename` | 2.4.1 |
| **single-instance** (`tauri-plugin-single-instance`) | Enforce one running instance; forward argv/cwd to the first. **Desktop-only.** | Y | Y | Y | - | - | (no JS commands — Rust-side `init` callback; pairs with `deep-link` via the `deep-link` feature) | 2.4.2 |
| **autostart** (`tauri-plugin-autostart`) | Launch app at OS login. | Y | Y | Y | - | - | `autostart:` — `allow-enable`, `allow-disable`, `allow-is-enabled` | 2.5.1 |
| **deep-link** (`tauri-plugin-deep-link`) | Register app as default handler for a custom URL scheme. | Y | Y | Y | Y | Y | `deep-link:` — `allow-register`, `allow-unregister`, `allow-is-registered`, `allow-get-current` | 2.4.9 |
| **store** (`tauri-plugin-store`) | Simple persistent key-value JSON store. | Y | Y | Y | Y | Y | `store:` — `allow-get`, `allow-set`, `allow-save`, `allow-load`, `allow-delete`, `allow-clear`, `allow-entries`, `allow-keys`, `allow-values` | 2.4.3 |
| **sql** (`tauri-plugin-sql`) | SQLite / Postgres / MySQL client (sqlx) exposed to the webview. | Y | Y | Y | Y | - | `sql:` — `allow-load`, `allow-execute`, `allow-select`, `allow-close` | 2.4.0 |
| **http** (`tauri-plugin-http`) | Fetch-like HTTP client in Rust (reqwest), URL-scoped. | Y | Y | Y | Y | Y | `http:` — `allow-fetch`, `allow-fetch-send`, `allow-fetch-read-body`, `allow-fetch-cancel` (scope = allowed URL patterns) | 2.5.9 |
| **stronghold** (`tauri-plugin-stronghold`) | Encrypted secret/key vault (IOTA Stronghold engine). | Y | Y | Y | Y | Y | `stronghold:` — `allow-initialize`, `allow-save-store-record`, `allow-get-store-record`, `allow-create-client`, … | 2.3.1 |
| **log** (`tauri-plugin-log`) | Unified logging: webview `console` + Rust `log` -> stdout/file/webview targets. | Y | Y | Y | Y | Y | `log:` — `allow-log`; default `log:default` | 2.8.0 |
| **positioner** (`tauri-plugin-positioner`) | Move windows to well-known anchor positions (corners, tray-relative). | Y | Y | Y | - | - | `positioner:` — `allow-move-window`, `allow-move-window-constrained`, `allow-set-tray-icon-state` | 2.3.1 |
| **upload** (`tauri-plugin-upload`) | Stream file upload/download over HTTP with progress. | Y | Y | Y | Y | Y | `upload:` — `allow-upload`, `allow-download` | 2.4.0 |
| **websocket** (`tauri-plugin-websocket`) | WebSocket client (tokio-tungstenite) for the webview. | Y | Y | Y | Y | Y | `websocket:` — `allow-connect`, `allow-send` | 2.4.2 |
| **cli** (`tauri-plugin-cli`) | Parse the app's own argv (clap) inside the Tauri app. | Y | Y | Y | - | - | `cli:` — `allow-cli-matches` | 2.4.1 |
| **localhost** (`tauri-plugin-localhost`) | Serve embedded assets over a `localhost` HTTP server instead of the custom protocol. | Y | Y | Y | Y | Y | (Rust-side `Builder` only) | 2.3.2 |
| **persisted-scope** (`tauri-plugin-persisted-scope`) | Persist runtime-granted fs/asset scopes across restarts. | Y | Y | Y | Y | Y | (Rust-side `init` only; complements `fs`) | 2.3.7 |
| **barcode-scanner** (`tauri-plugin-barcode-scanner`) | Camera QR/EAN/barcode scan. **Mobile-only.** | - | - | - | Y | Y | `barcode-scanner:` — `allow-scan`, `allow-cancel`, `allow-request-permissions` | 2.4.4 |
| **biometric** (`tauri-plugin-biometric`) | Fingerprint/Face biometric auth prompt. **Mobile-only.** | - | - | - | Y | Y | `biometric:` — `allow-authenticate`, `allow-status` | 2.3.2 |
| **nfc** (`tauri-plugin-nfc`) | Read/write NFC tags. **Mobile-only.** | - | - | - | Y | Y | `nfc:` — `allow-scan`, `allow-write`, `allow-is-available` | 2.3.5 |
| **haptics** (`tauri-plugin-haptics`) | Vibration / haptic feedback. **Mobile-only.** | - | - | - | Y | Y | `haptics:` — `allow-vibrate`, `allow-impact-feedback`, `allow-notification-feedback`, `allow-selection-feedback` | 2.3.2 |
| **geolocation** (`tauri-plugin-geolocation`) | GPS position + tracking. **Mobile-only** (desktop has `desktop.rs` stub but README marks L/W/M unsupported). | - | - | - | Y | Y | `geolocation:` — `allow-get-current-position`, `allow-watch-position`, `allow-check-permissions`, `allow-request-permissions` | 2.3.2 |

**Built into Tauri core (NOT separate plugins)** — available without adding a
plugin dep, gated by `core:` capabilities:

| Surface | Role | Capability namespace |
|---------|------|----------------------|
| Tray icon | System tray icon + menu (`tauri::tray`, backed by `tray-icon` crate, MIT/Apache). | `core:tray:` (e.g. `core:tray:default`) |
| Menu | Native window/app menus (`tauri::menu`, backed by `muda` crate, MIT/Apache). | `core:menu:` |
| Window | Create/resize/show/title/decorations (`tauri::window`). | `core:window:` (e.g. `allow-set-title`, `allow-set-size`) |
| Webview | Print, zoom, eval, devtools. | `core:webview:` (e.g. `allow-print`) |
| Event | Backend<->frontend pub/sub. | `core:event:` |
| App / Path / Image / Resources | App metadata, path resolution, image handles, resource table. | `core:app:`, `core:path:`, `core:image:`, `core:resources:` |

Evidence: `var/tauri/examples/api/src-tauri/capabilities/run-app.json` grants
`core:window:allow-set-title`, `core:webview:allow-print`, etc.; tray/menu via
`muda 0.19` / `tray-icon 0.23` in `var/tauri/crates/tauri/Cargo.toml`
([`architecture.md`](architecture.md) §1, §3).

---

## B. Catalogue — notable community plugins & packages

From `tauri-apps/awesome-tauri` (verified 2026-05-24) + crates.io. **Every entry
below is MIT or MIT-OR-Apache** (no GPL found). "Maint." = latest crates.io
publish date observed.

| Package | Role | Cross-platform | License | Latest / maint. |
|---------|------|----------------|---------|------------------|
| **tauri-specta** (`tauri-specta`) | Generates **typed TS bindings** (commands + events) from `#[tauri::command]` Rust fns via the `specta` type system. | N/A (build-time codegen) | MIT | 2.0.0-rc.25, 2026-05-08 |
| **specta** (`specta`) | The underlying Rust->TS type exporter that tauri-specta builds on. | N/A | MIT | 2.0.0-rc.25, 2026-05-07 |
| **taurpc** (`taurpc`) | Alternative typesafe IPC: a tRPC-style typed router over Tauri commands/events. | N/A | MIT OR Apache-2.0 | 0.7.1, 2026-02-12 |
| **window-vibrancy** (`window-vibrancy`) | Acrylic/Mica (Windows) + vibrancy (macOS) window backdrop. Official tauri-apps crate, **NOT a plugin** (called from Rust setup). | W, M only — **Linux UNSUPPORTED** (compositor-controlled) | MIT OR Apache-2.0 | 0.7.1, 2025-11-12 |
| **tauri-plugin-system-info** | Detailed CPU/RAM/disk/network system info to the webview. | L/W/M (desktop) | MIT | 2.0.9, 2025-02-14 |
| **tauri-plugin-network** | Read network info + scan the local network. | L/W/M | MIT | 2.0.4, 2024-10-03 |
| **tauri-plugin-clipboard** (CrossCopy) | Richer clipboard: text/image/html/rtf/files **+ clipboard-change monitoring** (the official clipboard-manager lacks change events). | L/W/M (+mobile partial) | MIT | 2.1.11, 2024-10-17 |
| **tauri-plugin-prevent-default** | Disable browser default shortcuts/context-menu (e.g. F5, Ctrl+P) in the webview. | L/W/M | MIT | 5.0.0, 2026-04-18 |
| **tauri-plugin-context-menu** | Native right-click context menus. | L/W/M | MIT | active |
| **tauri-plugin-theme** | Dynamically switch the app's light/dark theme at runtime. | L/W/M | MIT | 1.0.0, 2025-03-09 |
| **tauri-plugin-tracing** | `tracing`-crate structured logging, JS->Rust bridge, file rotation, flamegraphs. | L/W/M | MIT | active |
| **sentry-tauri** | Capture JS errors, Rust panics, native minidumps to Sentry. | L/W/M | MIT | active |
| **vite-plugin-tauri** | Integrate Tauri into a Vite build (frontend-side). | N/A (build tooling) | MIT | active |
| **tauri-plugin-serialport / serialplugin** | Cross-platform serial-port comms. | L/W/M | MIT | active |
| **tauri-plugin-blec** | Bluetooth LE client (btleplug). | L/W/M + mobile | MIT | active |
| **tauri-plugin-screenshots** | Capture screenshots of windows/monitors. | L/W/M | MIT | active |

Notes: `tauri-plugin-clipboard`, `tauri-plugin-system-info`, and
`tauri-plugin-network` have not published in 2025-2026 (last 2024-2025); treat
their "maintained" status as stale-but-functional and re-audit before adoption.
`tauri-specta`, `specta`, `taurpc`, and `tauri-plugin-prevent-default` are all
freshly published in 2026 — clearly active.

---

## C. Curated set for aphrody (ADD / SKIP)

Decision rule (restated): **ADD** only what the webview shell needs and the CLI
cannot already do; **SKIP** anything that duplicates the Rust CLI's system reach
(route those through `#[tauri::command]` -> `aphrody::run_async` instead).

### ADD — shell-only capabilities the CLI cannot provide

| Plugin | Why ADD (shell-only, not in the CLI) |
|--------|--------------------------------------|
| **(core) tray-icon + menu** | A desktop app needs a tray + native menus; this is pure webview-shell territory with no CLI equivalent. Built into core (`core:tray:`/`core:menu:`), no extra crate. |
| **(core) window / webview** | Window create/title/size/decorations + webview print/zoom/devtools — only the shell owns the window. `core:window:`/`core:webview:`. |
| **window-state** | Persist/restore window geometry across launches. Desktop UX, no CLI analogue. Cheap, L/W/M. |
| **positioner** | Anchor windows (tray-relative popovers, corners). Pairs with tray. Desktop UX only. |
| **single-instance** | One running GUI instance, forward argv to the first — essential for a tray/deep-link desktop app. Enable its `deep-link` feature so a second launch with a URL is forwarded (see deep-link note below). |
| **notification** | Native OS notifications (job done, agy-loop finished, message arrived). The CLI's `aphrody notify` targets chat backends (Slack/Telegram/…), **not** the local desktop notification center — these are complementary, not duplicate. |
| **global-shortcut** | System-wide hotkeys (e.g. summon the aphrody window, quick-capture). No CLI equivalent; inherently a windowing-layer feature. L/W/M only (no mobile). |
| **clipboard-manager** | Read/write the OS clipboard from the UI (paste a binary path to triage, copy a forensics finding). The CLI does not touch the GUI clipboard. Start with text-only permissions. |
| **dialog** | Native file open/save + message/confirm dialogs. The UI needs an OS file picker to choose a target binary/dir to hand to a CLI command; the CLI has no GUI picker. |
| **os** | Quick OS/arch/version/locale for the UI chrome (badges, conditional UI). Tiny, ubiquitous. (The CLI's `doctor`/`version` give richer data via `run_async`; `os` is just for trivial UI-side branching without a command round-trip.) |
| **process** | Let the UI relaunch/exit the app cleanly (e.g. after an update). Shell lifecycle, not CLI. |
| **updater** | In-app signed self-update of the GUI bundle. This is the GUI distribution mechanism; the CLI ships via `scripts/deploy.*` and has no in-app updater. ADD once code-signing keys exist. |
| **log** | Bridge webview `console.*` + Rust `log` into one sink (file + devtools). Useful for the GUI's own diagnostics; distinct from the CLI's tracing output. |
| **tauri-specta** (package, build-dep) | Typed TS bindings for the `#[tauri::command]` surface — see §E. High-value for the transport-abstract client + `m3-react`/Lit frontend. |

### ADD — conditional / platform-scoped

| Plugin | Condition |
|--------|-----------|
| **deep-link** | ADD if aphrody wants an `aphrody://` URL scheme (open a triage report, deep-link into a chat thread). On Linux/Windows the OS spawns a **new** process with the URL as argv (only macOS/iOS/Android emit live events), so it **must** be paired with `single-instance` + its `deep-link` feature to forward the URL to the running instance (`var/tauri-plugins/plugins/deep-link/README.md`). |
| **autostart** | ADD if aphrody should launch at login (e.g. a background tray agent for `hermes`/messaging). Otherwise SKIP — most users do not want a forensics tool auto-starting. Default OFF. |
| **stronghold** | ADD only if the GUI must hold its **own** secrets at rest. aphrody already stores credentials in `secrets/` + OS Credential Manager (antigravity-sdk) via the CLI; prefer routing secret ops through the CLI. SKIP unless the GUI needs an independent encrypted vault. If added, hash the password with argon2id (`README.md` example). |
| **(mobile) biometric / haptics / barcode-scanner / nfc / geolocation** | DEFER — mobile is **out of aphrody's priority order** (Linux #1, Windows #2, wasm #3, macOS best-effort; mobile not listed, [`risks.md`](risks.md) §2). Revisit only if a mobile shell ships. |
| **window-vibrancy** (package) | OPTIONAL eye-candy on Windows/macOS (Mica/acrylic/vibrancy). **No effect on Linux #1** (compositor-controlled, upstream-unsupported). ADD as a `#[cfg(any(windows, target_os="macos"))]` setup call only; never rely on it for Linux. Low priority. |

### SKIP — already covered by the Rust CLI (route via `run_async`, do not duplicate)

| Plugin | Why SKIP (duplicates CLI reach) |
|--------|--------------------------------|
| **fs** | aphrody already does scoped filesystem work in Rust (walkdir/ignore/rayon scan, forensics, chrome/cookies). Exposing a second, webview-facing fs ACL **enlarges the attack surface** of a security tool for no gain. Do all fs through `#[tauri::command]` -> `aphrody::run_async` (which keeps the Rust-side guards). The native file **picker** still comes from `dialog` (ADD) — that returns a path the command then handles. |
| **shell** | The CLI **is** the shell surface. A webview-facing `shell:execute` with a command allowlist is a notorious RCE footgun; aphrody's commands already run real subprocesses in Rust with full control. Never grant `shell:` to the webview. |
| **opener** | "Open this path/URL in the default app" overlaps the CLI's capabilities and adds a webview-reachable launcher. If the GUI needs to reveal a file in the file manager, prefer a dedicated narrow command over the broad `opener:` ACL. Borderline — ADD only the single `allow-reveal-item-in-dir` if a real need appears; otherwise SKIP. |
| **http** | aphrody has a first-class Rust HTTP stack (reqwest + rustls ring provider, [`README.md`](README.md) §0). A second webview-facing HTTP client with its own URL-scope ACL is redundant and widens egress surface. Make network calls in commands. |
| **upload / websocket** | Same reasoning: file upload/download and WS belong in Rust commands where aphrody already has the networking + streaming (Tauri `Channel`) story. SKIP the plugins. |
| **sql** | aphrody already uses SQLite (FTS5 `aphrody-fsindex`) inside Rust. A webview-facing SQL client (`sql:execute` with raw queries) is an injection surface. Query via commands. |
| **store** | A persistent KV store is trivially a JSON file the Rust side owns, or a thin command. The plugin adds a webview-reachable persistence ACL for little benefit; prefer Rust-side state or a small command. SKIP (low value, not harmful). |
| **cli** | aphrody's argv is parsed by the **CLI library** (clap) before/around Tauri; the GUI does not need to re-parse its own argv through a plugin. SKIP. |
| **localhost** | Tauri's default custom-protocol asset serving is fine and avoids opening a real localhost port (smaller surface). Only needed for quirky webview requirements aphrody does not have. SKIP. |
| **persisted-scope** | Only relevant if you ship the **fs** plugin with runtime-granted scopes — and we are skipping `fs`. SKIP. |
| **system-info / network** (community) | Duplicates the CLI's system + network reconnaissance (which is far richer: `doctor`, DNS recon, scan). Route through commands. SKIP. |
| **clipboard (CrossCopy)** | Only ADD over the official `clipboard-manager` if you specifically need **clipboard-change events**; otherwise the official one suffices. Default SKIP. |

**Net curated set:** core **tray/menu/window/webview** + plugins
**window-state, positioner, single-instance, notification, global-shortcut,
clipboard-manager, dialog, os, process, updater, log**, the **tauri-specta**
build-dep, and conditionally **deep-link (+single-instance feature)**,
**autostart**, **stronghold**, **window-vibrancy** (Win/mac only). Everything
fs/shell/http/sql/upload/ws/scan/process-control is **SKIPPED at the plugin
layer and routed through `aphrody::run_async`** to keep one Rust-side guard and a
minimal webview ACL.

---

## D. Concrete `Cargo.toml` + `capabilities` for `crates/aphrody-app`

This is the build-excluded host-only shell (Option 1 in
[`aphrody-integration.md`](aphrody-integration.md) §4). Versions match §A
(2026-05-24); pin to caret-minor per the workspace convention.

### `crates/aphrody-app/Cargo.toml`

```toml
[package]
name = "aphrody-app"
version = "0.1.0"
edition = "2024"
license = "Apache-2.0"
publish = false          # host-only GUI shell, never published / never in the CLI distribution

[lib]
# mobile shells link this as a static/cdylib; keep rlib for desktop.
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2.5", features = [] }

[dependencies]
tauri = { version = "2.10", features = ["tray-icon", "image-png"] }
aphrody = { path = "../cli" }                  # Path (a): in-process Rust->Rust
aphrody-capture = { path = "../aphrody-capture" }  # shared stdout/stderr capture (Phase 0 pre-work)

# --- ADD: shell-only capability plugins (see §C) ---
tauri-plugin-window-state     = "2.4"
tauri-plugin-positioner       = "2.3"
tauri-plugin-single-instance  = { version = "2.4", features = ["deep-link"] }
tauri-plugin-notification     = "2.3"
tauri-plugin-global-shortcut  = "2.3"
tauri-plugin-clipboard-manager = "2.3"
tauri-plugin-dialog           = "2.7"
tauri-plugin-os               = "2.3"
tauri-plugin-process          = "2.3"
tauri-plugin-updater          = "2.10"
tauri-plugin-log              = "2.8"

# --- ADD (conditional): uncomment when the feature is wanted ---
# tauri-plugin-deep-link      = "2.4"
# tauri-plugin-autostart      = "2.5"
# tauri-plugin-stronghold     = "2.3"   # + argon2 for password hashing

# --- typed Rust->TS bindings (build-time; see §E) ---
specta       = "=2.0.0-rc.25"
specta-typescript = "=0.0.9"
tauri-specta = { version = "=2.0.0-rc.25", features = ["derive", "typescript"] }

serde = { version = "1", features = ["derive"] }

# Windows/macOS-only eye-candy; NO effect on Linux #1.
[target.'cfg(any(windows, target_os = "macos"))'.dependencies]
window-vibrancy = "0.7"

# Keep this crate OUT of the lean default build (mirror the 2026-05-23 UI
# extraction). In the workspace root Cargo.toml, exclude from default-members:
#   default-members = [ ...everything except "crates/aphrody-app"... ]
# so `cargo ci-offline` / cross-target binary checks never pull wry/tao/gtk-rs.
```

Notes:
- All official plugin versions are caret-minor against §A; the rc-pinned
  `=2.0.0-rc.25` on specta/tauri-specta is deliberate (release-candidates: pin
  exact, re-pin via PR — matches CLAUDE.md toolchain-pin discipline). The cloned
  workspace already references `specta = "^2.0.0-rc.16"`
  (`var/tauri-plugins/Cargo.toml:24`), confirming the line is current.
- `tauri` feature `tray-icon` pulls the core `tray-icon`/`muda` path; no separate
  plugin needed for tray/menu.
- These deps add the **wry/tao/gtk-rs/webkit2gtk** tree (Risk R4,
  [`risks.md`](risks.md) §4) — scoped to this build-excluded crate, plus matching
  `deny.toml` / `cargo vet` entries (Phase 0 pre-work).

### `crates/aphrody-app/capabilities/main.json` (minimal ACL — default-deny)

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-capability",
  "description": "Minimal surface for the aphrody desktop shell. Default-deny; only the shell-layer permissions the UI needs. ALL fs/shell/http/scan/process work goes through the aphrody_exec command, NOT through plugin ACLs.",
  "windows": ["main"],
  "local": true,
  "permissions": [
    "core:default",

    "core:window:allow-set-title",
    "core:window:allow-set-size",
    "core:window:allow-minimize",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:webview:allow-print",
    "core:tray:default",
    "core:menu:default",
    "core:event:default",

    "window-state:allow-save-window-state",
    "window-state:allow-restore-state",

    "positioner:allow-move-window",
    "positioner:allow-set-tray-icon-state",

    "notification:default",

    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-is-registered",

    "clipboard-manager:allow-read-text",
    "clipboard-manager:allow-write-text",

    "dialog:allow-open",
    "dialog:allow-save",
    "dialog:allow-message",
    "dialog:allow-confirm",

    "os:allow-platform",
    "os:allow-version",
    "os:allow-arch",
    "os:allow-locale",

    "process:allow-restart",
    "process:allow-exit",

    "updater:allow-check",
    "updater:allow-download-and-install",

    "log:default"
  ]
}
```

Deliberately **absent**: every `fs:`, `shell:`, `http:`, `sql:`, `upload:`,
`websocket:`, `store:`, `opener:` permission. The webview cannot touch the
filesystem, spawn processes, or open arbitrary network connections **directly** —
it can only call the app's own `#[tauri::command]`s (e.g. `aphrody_exec(args)`),
which run Rust-side under aphrody's existing guards. Enable
`removeUnusedCommands: true` in `tauri.conf.json`
(`var/tauri/examples/api/src-tauri/tauri.conf.json:13`) to tree-shake any
ungranted command out of the binary and the IPC surface.

### Frontend usage (JS) — examples for the granted plugins

```js
// All system work goes through the in-process Rust command, NOT a plugin:
import { invoke } from "@tauri-apps/api/core";
const { code, stdout, stderr } = await invoke("aphrody_exec", {
  args: ["re", "triage", "/path/to/binary"],
});

// Shell-only capabilities use their plugins:
import { open as openDialog, save } from "@tauri-apps/plugin-dialog";
const file = await openDialog({ multiple: false }); // native picker -> path -> feed to aphrody_exec

import { writeText, readText } from "@tauri-apps/plugin-clipboard-manager";
await writeText(stdout);

import { sendNotification, isPermissionGranted, requestPermission }
  from "@tauri-apps/plugin-notification";
if (!(await isPermissionGranted())) await requestPermission();
await sendNotification({ title: "aphrody", body: "Triage complete" });

import { register } from "@tauri-apps/plugin-global-shortcut";
await register("CmdOrCtrl+Shift+A", () => { /* summon window */ });

import { platform, version } from "@tauri-apps/plugin-os";
const os = `${await platform()} ${await version()}`;
```

---

## E. tauri-specta — recommendation: ADOPT

**Verdict: ADD `tauri-specta` (2.0.0-rc.25, MIT) as a build-time dependency.**

Why it fits aphrody:
- It generates **typed TypeScript bindings** (both commands **and** events) from
  the `#[tauri::command]` Rust functions in `crates/aphrody-app`, using the
  `specta` type system. This makes the UI's `invoke("aphrody_exec", …)` calls and
  their `{ code, stdout, stderr }` results fully typed in TS — directly serving
  the transport-abstract client and the M3 `m3-react`/Lit frontend that the
  integration plan targets ([`aphrody-integration.md`](aphrody-integration.md) §5).
- It is **freshly maintained in 2026** (rc.25 published 2026-05-08; `specta`
  rc.25 2026-05-07) and **MIT-licensed** — passes the license gate. The cloned
  official plugins workspace itself depends on `specta` (`var/tauri-plugins/Cargo.toml:24`),
  i.e. the Tauri ecosystem already standardises on it.
- It is **build-time only** — it does not enter the runtime binary or the IPC
  attack surface, so it carries none of the webview-plugin security cost. The
  generated `.ts` file is committed/consumed by the Bun frontend build.
- Workflow: derive `specta::Type` on command arg/return structs, register the
  commands with `tauri_specta::Builder`, and on `cargo build` (or a small bin)
  export `bindings.ts` for the frontend. Pin exact (`=2.0.0-rc.25`) because it is
  a release candidate; re-pin via PR.

**Why not the alternatives:**
- **taurpc** (0.7.1, MIT/Apache, 2026-02-12) is a heavier tRPC-style typed
  *router* abstraction. aphrody's command surface is intentionally tiny (mostly
  one `aphrody_exec` + a few streaming `Channel` commands), so a full RPC router
  is overkill — tauri-specta's thin codegen is the better fit. Keep taurpc on the
  radar only if the command surface grows into a large typed API.
- **tauri-plugin-graphql** — type-safe IPC over GraphQL; far more machinery than a
  CLI-wrapping shell needs. SKIP.

Caveat: rc status means the API can still shift between rc bumps; the exact pin +
PR-gated re-pin contains that. The payoff (no hand-written, drift-prone TS types
across the Rust<->webview boundary) is worth it for a typed UI client.

---

## F. Security (ACL v2) + Linux #1 notes

**ACL / capabilities (v2) — minimise the surface.**
- Tauri v2 is **default-deny**: a command is unreachable unless a capability
  grants it (`RuntimeAuthority`, `var/tauri/crates/tauri/src/ipc/authority.rs:27-35`;
  [`architecture.md`](architecture.md) §3). The curated `main.json` in §D grants
  **only** shell-layer permissions and **no** fs/shell/http/sql — so the webview
  cannot reach the filesystem/network/process layer except through the app's own
  commands, which keep aphrody's Rust-side guards.
- **Origin gating**: commands are bound to `Local` vs `Remote { url }`
  (`authority.rs:57-67`). Keep everything `"local": true` and load only the
  embedded first-party frontend — never a remote origin — so no remote page can
  invoke commands. This is the load-bearing mitigation for a security tool
  (Risk R8, [`risks.md`](risks.md) §4).
- **Tree-shake**: `removeUnusedCommands: true`
  (`var/tauri/examples/api/src-tauri/tauri.conf.json:13`) drops ungranted
  commands from the binary and IPC surface.
- **Scoped permissions**: if `fs`/`http`/`opener` are ever added against this
  advice, they **must** carry tight path/URL scopes (`allow`/`deny` globs;
  asset-protocol scope example `var/tauri/examples/api/src-tauri/tauri.conf.json:31-37`)
  and never a blanket `allow-*`. The strong default is to not add them at all.
- **stronghold**: if adopted, the master password must be hashed with **argon2id**
  before reaching the engine (`var/tauri-plugins/plugins/stronghold/README.md`
  shows the `rust-argon2` `Config` with `Variant::Argon2id`). Never pass a raw
  password.

**Linux #1 specifics.**
- All ADDed official plugins are **fully supported on Linux** (§A tables:
  window-state, positioner, single-instance, notification, global-shortcut,
  clipboard-manager, dialog, os, process, updater, log all show Linux `Y`). No
  Linux gaps in the curated desktop set.
- **window-vibrancy is unsupported on Linux** (compositor-controlled, confirmed on
  the upstream repo) — gate it `#[cfg(any(windows, target_os="macos"))]` and never
  let the UI depend on it for Linux rendering.
- The Linux webview is **WebKitGTK on GTK3** ([`risks.md`](risks.md) §1): the
  M3/Material-Web (Lit) frontend must be verified headlessly on WebKitGTK
  (evergreen-web, but lags Chromium on some `:has()`/container-query/Houdini
  edges). Security of rendered content rests on the **user's patched system
  WebKitGTK** (Risk R1) — acceptable because aphrody renders only its own embedded
  content behind the local-origin ACL.
- The **mobile-only** plugins (biometric/haptics/nfc/barcode-scanner/geolocation)
  are not Linux-relevant and are deferred regardless.
- License on Linux: the gtk-rs/webkit2gtk **Rust bindings are MIT**; the
  underlying GTK3/WebKitGTK **C libraries are LGPL but dynamically linked** at
  runtime (distro package), so **no GPL/LGPL contamination** of aphrody's
  Apache-2.0 binary — identical to any Linux app linking libc/GTK
  ([`risks.md`](risks.md) §3).

---

## Sources

- Official plugins monorepo: `tauri-apps/plugins-workspace`, cloned at
  `var/tauri-plugins` (gitignored). Per-plugin `Cargo.toml`/`README.md`/`permissions/`;
  workspace license `var/tauri-plugins/Cargo.toml:38` (`Apache-2.0 OR MIT`).
- Tauri core: `var/tauri` tag v2.11.2 — ACL `authority.rs`, capabilities example
  `examples/api/src-tauri/capabilities/run-app.json`, `tauri.conf.json` flags
  (cited inline). Companion docs [`architecture.md`](architecture.md),
  [`risks.md`](risks.md), [`aphrody-integration.md`](aphrody-integration.md),
  [`README.md`](README.md).
- crates.io (versions/licenses/dates, 2026-05-24): tauri-specta 2.0.0-rc.25 (MIT),
  specta 2.0.0-rc.25 (MIT), taurpc 0.7.1 (MIT OR Apache-2.0), window-vibrancy 0.7.1
  (MIT OR Apache-2.0), tauri-plugin-window-state 2.4.1, tauri-plugin-system-info
  2.0.9 (MIT), tauri-plugin-network 2.0.4 (MIT), tauri-plugin-clipboard 2.1.11
  (MIT), tauri-plugin-prevent-default 5.0.0 (MIT), tauri-plugin-theme 1.0.0 (MIT).
  - https://crates.io/crates/tauri-specta — https://github.com/specta-rs/tauri-specta
  - https://crates.io/crates/specta — https://github.com/specta-rs/specta
  - https://crates.io/crates/taurpc — https://github.com/MatsDK/TauRPC
  - https://github.com/tauri-apps/window-vibrancy
  - https://crates.io/crates/tauri-plugin-window-state
- Community index: `tauri-apps/awesome-tauri` README (Plugins section), verified
  2026-05-24 — https://github.com/tauri-apps/awesome-tauri ;
  official feature catalogue https://v2.tauri.app/plugin/
