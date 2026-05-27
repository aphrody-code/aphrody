<!-- SPDX-License-Identifier: Apache-2.0 -->

# aphrody desktop

A pixel-faithful Gemini-style desktop client for the **aphrody** CLI, built on
the same stack Gemini ships: **Tauri 2 + Angular 21 + Angular Material 21**.

The UI mirrors gemini.google.com/app (deep-black-to-blue glow, narrow icon
rail, gradient hero, the 32px composer pill with a mic/model selector,
streaming assistant turns) but is 100% aphrody — its own navigation
(Assistant, Reverse, Forensics, Réseau, Diagnostic, Commandes) and its own
backend.

## Architecture

- **Frontend** (`src/`): standalone Angular components + signals. Material 3 is
  themed via `mat.theme()` (emits `--mat-sys-*` system tokens), then the exact
  Gemini dark/light palette is layered on top of those tokens in
  `src/styles.scss`, so every Material component renders in Gemini's colours.
- **Backend** (`src-tauri/`): the `aphrody_exec` `#[tauri::command]` runs the
  aphrody CLI **in-process** via `aphrody::run_captured` (Rust → Rust, no
  subprocess), depending on the in-repo crate
  `aphrody = { path = "../../../crates/cli" }` (the app was repatriated into
  this repo on 2026-05-24; it is the only non-Rust surface here). Mirrors the
  in-process pattern of `crates/aphrody-app`. The Assistant sends
  `["chat","--prompt", <msg>, "--model", "gemini-3.5-flash"]`
  (gemini-runtime → Gemini 3.5 Flash); the tool views run `re`, `dns`,
  `forensics`, `doctor`, etc.
- **Web/dev fallback**: outside Tauri, `AphrodyService` tries `POST /api/run`
  and otherwise returns a labelled offline stub, so `ng serve` is fully usable.

## Fonts (vendored, offline)

Tauri has no network at runtime, so fonts are vendored under
`src/assets/fonts/` and declared with local `@font-face`:

- **Google Sans Flex** (the real Gemini face) — latin + latin-ext woff2 subsets,
  OFL (`google-sans-flex-OFL.txt`).
- **Material Symbols Outlined** (the `mat-icon` ligature font) — Apache-2.0
  (`material-symbols-LICENSE.txt`).

The built `dist/` references **no** network resource.

## Commands (run under bun)

```bash
bun install
bun run build       # ng build  -> dist/desktop/browser (frontend only)
bun run start       # ng serve  -> http://localhost:1420
bun run tauri dev   # dev shell: live frontend over the ng serve devUrl
bun run tauri build # PRODUCTION shell: embeds dist/, loads no dev server
```

### Production vs dev (read before shipping)

Tauri decides dev-vs-production purely by `tauri::is_dev()`, which is
`!cfg!(feature = "custom-protocol")` (tauri 2.11 `src/lib.rs:308`). So:

- A **bare `cargo build` / `cargo run`** — _even `--release`_ — leaves
  `custom-protocol` **off**, so the webview loads `build.devUrl`
  (`http://localhost:1420`). With no `bun run start` running you get
  **"localhost refused to connect"**. This is a dev artifact, not shippable.
- The **production binary** has `custom-protocol` **on**, so the webview serves
  the embedded `build.frontendDist` over the `tauri://` protocol — no network.
  Produce it with either:
  - `bun run tauri build` (also makes installers), or
  - `cargo build --release --features custom-protocol` (binary only, no Node).

`src-tauri/rust-toolchain.toml` pins the same nightly as the core aphrody
workspace, required because the in-process CLI dependency uses Edition 2024.

## Native CLI integration (`aphrody gui`)

The app is launched from the core CLI by **resolve + spawn**, never by
embedding a webview in `aphrody` (the wry/tao/GTK stack stays out of the core
supply-chain — `apps/desktop` is a self-rooted workspace excluded from the
core, cf. `CLAUDE.md` §2/§7).

- **Binary name** — the build emits **`aphrody-gui`** (`.exe` on Windows), set
  via `[[bin]] name = "aphrody-gui"` in `src-tauri/Cargo.toml`; `tauri
build`/bundle is locked to the same name via `mainBinaryName` in
  `tauri.conf.json` (the package stays `desktop`, `[lib] desktop_lib` so
  `main.rs` keeps calling `desktop_lib::run()`). `productName` stays `aphrody`.
  Use a **production** build (see above) — `aphrody gui` will happily launch a
  dev-mode binary, which then shows the localhost error.
- **Launch** — `aphrody gui` resolves the binary in this order (first match
  wins) and spawns it **detached** (the window outlives the launcher; the
  terminal is never blocked):
  1. `$APHRODY_GUI_BIN` (explicit path to an existing file),
  2. an `aphrody-gui[.exe]` sibling of the running `aphrody` (the deployed
     case),
  3. an in-tree build at
     `apps/desktop/src-tauri/target/[<triple>/]{release,debug}/aphrody-gui[.exe]`
     — the `<triple>` segment is present when a `build.target` is forced (the
     repo `.cargo/config.toml` pins `x86_64-pc-windows-msvc`),
  4. `aphrody-gui` on `PATH`.
     `aphrody gui --print-path` resolves and prints the path without launching
     (scriptable); `aphrody gui -- <args…>` forwards trailing args to the GUI.
- **Deploy** — `scripts/deploy.{ps1,sh}` build only the core workspace, which
  excludes this app. Pass `-IncludeGui` / `--include-gui` to copy an
  already-built `aphrody-gui` next to `aphrody` in `~/.local/bin`, so the
  sibling lookup (step 2) succeeds after install. Build the production GUI
  first: `cd apps/desktop && bun install && bun run tauri build`.
