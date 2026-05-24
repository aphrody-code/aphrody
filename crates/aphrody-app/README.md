<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody-app — the aphrody desktop shell (Tauri v2)

A native desktop window that runs the **full aphrody CLI in-process** and renders
it in a Material 3 webview. The React frontend (French UI) lives in the sibling
`aphrody-ts` repo (`apps/desktop-react`); this crate is the Rust shell.

## Architecture — Path (a): in-process, no FFI hop

The webview never spawns a subprocess and never crosses an FFI boundary. Each
command is a `#[tauri::command]` that calls `aphrody::run_captured(argv)`
(the CLI library, crate `aphrody`, dir `crates/cli`) directly:

```
webview  --invoke('aphrody_exec', { args })-->  Rust #[command]
                                                  └─ spawn_blocking
                                                       └─ aphrody::run_captured(["aphrody", ...args])
                                                            └─ { code, stdout, stderr }  --> back to the webview
```

`run_captured` redirects the process-global stdout/stderr to a temp file
(`aphrody-stdio-capture`) and returns the captured text + exit code. The redirect
is process-global, so `aphrody_exec` serialises every run behind a `Mutex`.

Two commands are exposed:

| Command | Returns | Use |
|---|---|---|
| `aphrody_exec(args: string[])` | `{ code, stdout, stderr }` | run any aphrody command |
| `aphrody_meta()` | `{ app_version, target_os, target_arch, family }` | header / about info |

## Why it is build-EXCLUDED

This crate pulls the `tauri` / `wry` / `tao` / `gtk-rs` / `webkit2gtk` tree, which
must never enter the core `Cargo.lock` or `cargo ci-offline`. So it is listed in
the root `Cargo.toml` `[workspace.exclude]` and carries an empty `[workspace]`
table + its own `Cargo.lock`. The core `aphrody` binary and CI are unaffected.

## Capability surface (default-deny)

`capabilities/main.json` is v2 default-deny: it grants only **shell-layer**
permissions (window, os, dialog, clipboard, notification, process, window-state,
global-shortcut, log). It grants **no** `fs` / `shell` / `http` / `sql`
permission — every filesystem / process / network action goes through
`aphrody_exec`, which keeps aphrody's own Rust-side guards. The window loads only
the embedded first-party frontend (`"local": true`), never a remote origin.

Plugin rationale: `docs/tauri/plugins.md`. Integration design:
`docs/tauri/aphrody-integration.md`.

## Build / run

The frontend is built by Bun in `aphrody-ts` and copied into `dist/` (gitignored)
before `cargo` embeds it. Use the helper script (it does all three steps):

```pwsh
pwsh scripts/tauri.ps1 -Action run            # Windows: build frontend + app, launch
```
```bash
scripts/tauri.sh run                           # Linux/macOS
```

Linux #1 needs the WebView runtime dev packages:
`sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev`.

The script shares the core `target/` dir (`CARGO_TARGET_DIR`) so the aphrody CLI
deps already compiled for the core build are reused.

## Frontend

Default is `apps/desktop-react` (React 19, French UI, modeled on the
gemini.google.com/app maquette). The vanilla `apps/desktop-ui` (Lit / Material
Web) is selectable with `-Frontend desktop-ui` / `FRONTEND=desktop-ui`. Both
consume `@aphrody-code/theme` tokens and the transport-abstract command client.
