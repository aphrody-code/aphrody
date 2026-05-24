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
  subprocess), depending on the sibling repo
  `aphrody = { path = "../../../../aphrody/crates/cli" }`. Mirrors
  `crates/aphrody-app`. The Assistant sends `["chat","--prompt", <msg>]`
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
bun run build      # ng build -> dist/desktop/browser
bun run start      # ng serve  -> http://localhost:1420
bun run tauri dev  # full desktop shell (requires the sibling aphrody repo)
```

`src-tauri/rust-toolchain.toml` pins the same nightly as the sibling aphrody
repo, required because the in-process CLI dependency uses Edition 2024.
