<!-- SPDX-License-Identifier: Apache-2.0 -->
# Material Web in the Tauri v2 webview — offline, cross-engine playbook

Author: aphrody-code. Last updated: 2026-05-24.

How aphrody renders its Material Design 3 fusion (Material Web `md-*`
components + `@aphrody-code/theme` tokens) inside the **Tauri v2** desktop
webview, with **zero network access at runtime**. This is the implementation
companion to the framework decision in
[`../../aphrody/docs/tauri/ui-framework.md`](../../aphrody/docs/tauri/ui-framework.md)
(Vanilla TS + Lit/Material Web) and the shell decision in
[`../../aphrody/docs/tauri/README.md`](../../aphrody/docs/tauri/README.md)
(Tauri v2, system webview, Rust backend calling `aphrody::run_async`
in-process).

The reference frontend produced alongside this doc is
[`apps/desktop-ui`](../apps/desktop-ui) — a static, framework-runtime-free
Material Web page that builds with Bun into a self-contained `dist/` ready to
drop into Tauri's `frontendDist`.

---

## 0. The decision in one screen

- **Frontend layer**: Vanilla TypeScript driving Material Web `md-*` custom
  elements directly. No React/Vue/Svelte runtime in the webview — the only JS
  framework shipped is the Lit that the imported `md-*` components already pull
  in. Matches `apps/console` and the `ui-framework.md` ruling.
- **Components**: npm `@material/web` `^2.4.1` (the same dependency
  `apps/m3-react` declares), imported as ES modules with explicit `.js`
  extensions. **Not** the `packages/material-web` fork (that is a vendored,
  out-of-gate dev artifact — see §6).
- **Tokens**: `@aphrody-code/theme/tokens.css` (`--md-sys-color-*`, light
  `:root` + dark `.dark`). Theme switch = toggling `class="dark"` on `<html>`.
- **Fonts**: **embedded, self-hosted woff2** — Material Symbols Outlined (icon
  font for `md-icon`) and Roboto Flex (text). No `fonts.googleapis.com`. This
  is the single most important offline requirement (§2).
- **Build**: `bun build src/index.html --outdir dist` — Bun's HTML bundler
  concatenates the CSS, bundles the TS, and content-hashes + copies the woff2
  into `dist/assets/`, rewriting every `url()` to a relative path. The output
  references nothing external.
- **Tauri wiring**: point `build.frontendDist` at that `dist/`, ship a CSP that
  permits Lit's adopted stylesheets and the embedded fonts via `'self'`, no
  CDN hosts (§3).

---

## 1. The `apps/desktop-ui` skeleton

```
apps/desktop-ui/
  package.json          # @aphrody-code/desktop-ui; deps: @material/web ^2.4.1, @aphrody-code/theme
  tsconfig.json         # strict, moduleResolution: bundler, noEmit (tsc is the type gate)
  src/
    index.html          # build entry; the demo markup (md-* components, light+dark)
    app.ts              # registers md-* elements, imports tokens + fonts, wires interactions
    globals.d.ts        # `declare module "*.css"` for the side-effect CSS imports
    fonts/
      fonts.css                          # local @font-face (Material Symbols + Roboto Flex)
      material-symbols-outlined.woff2    # vendored icon font  (Apache-2.0)
      roboto-flex-latin.woff2            # vendored text font  (SIL OFL-1.1)
      LICENSE-material-symbols.txt
      LICENSE-roboto-flex.txt
```

### How components are consumed

`app.ts` imports each component module for its registration side effect. We
bind the exported element class to a referenced array (the same import *shape*
`apps/m3-react` uses) rather than a bare `import "...js"`:

```ts
import { MdFilledButton } from "@material/web/button/filled-button.js";
import { MdOutlinedTextField } from "@material/web/textfield/outlined-text-field.js";
import { MdIcon } from "@material/web/icon/icon.js";
// ...
const REGISTERED = [MdFilledButton, MdOutlinedTextField, MdIcon /* ... */] as const;
void REGISTERED.length;
```

Importing the module runs `customElements.define('md-filled-button', ...)`, so
the tags work declaratively in `index.html`. The components are
framework-agnostic, so the page needs no wrapper layer (the `m3-react`
`createComponent` wrappers exist only to bridge React's prop/event model and
are not on this path).

The demo (`index.html`) exercises a representative cross-section — buttons
(`md-filled-button`/`md-outlined-button`/`md-text-button`), `md-icon` +
`md-icon-button`, `md-outlined-text-field`, `md-checkbox`/`md-switch`/
`md-slider`, `md-chip-set`/`md-assist-chip`, `md-list`/`md-list-item`/
`md-divider`, `md-linear-progress`/`md-circular-progress`, and `md-tabs`/
`md-primary-tab` — all themed by `--md-sys-color-*` and rendered in both light
and dark via the header `md-switch` toggling `class="dark"`.

### Build

```sh
bun install                               # wires the workspace member
bun run --cwd apps/desktop-ui build       # NODE_ENV=production bun build src/index.html --outdir dist --minify
bun run --cwd apps/desktop-ui typecheck   # tsc --noEmit  (the type gate; there is no emit step)
bun run --cwd apps/desktop-ui lint        # oxlint
bun run --cwd apps/desktop-ui fmt:check   # oxfmt --check src
```

`dev` (`bun src/index.html`) starts Bun's HMR dev server for iteration; Tauri
points `devUrl` at it during `tauri dev` and `frontendDist` at `dist/` for the
release build.

### Measured bundle (verified 2026-05-24)

| Artifact | Size | Notes |
|---|---|---|
| `dist/index.html` | ~9.4 KB | entry; relative `<link>`/`<script>` to hashed assets |
| `dist/index-*.js` | ~273 KB | Material Web + Lit + app (150 modules), minified; ~70 KB gzipped |
| `dist/index-*.css` | ~12.3 KB | tokens + fonts.css + M3 type scale, concatenated |
| `dist/assets/roboto-flex-latin-*.woff2` | ~326 KB | text font, latin subset, all axes |
| `dist/assets/material-symbols-outlined-*.woff2` | ~3.94 MB | icon font (variable; full glyph set) |
| **Total** | **~4.4 MB** | dominated by the icon font (see §2.4 to shrink) |

The only `http(s)` string in the whole output is `http://www.w3.org/2000/svg`
(the SVG namespace passed to `createElementNS` inside components — an
identifier, **not** a fetch). There are **no** `href`/`src`/`@import`/`url()`
references to any external host.

---

## 2. Offline fonts — the core of the prep

A Tauri app ships with **no network**. The conventional Material Web setup puts
a `<link rel="stylesheet" href="https://fonts.googleapis.com/...">` in the HTML
for both the text face and the **Material Symbols** icon font. In an offline
webview those requests fail silently, with two visible consequences:

1. Text falls back to a system sans (cosmetic — acceptable but off-brand).
2. **`md-icon` renders empty boxes or the raw ligature text** (e.g. the literal
   word `settings` instead of the gear glyph). The icon font is *functional*,
   not decorative — without it the UI is broken.

So both faces must be **embedded**.

### 2.1 Which fonts, and why those families

- **Icons — `Material Symbols Outlined`.** `md-icon`'s style sets
  `font-family: var(--md-icon-font, Material Symbols Outlined)` (verified in
  `@material/web/icon/icon-styles.css`). The vendored `@font-face` family name
  **must be exactly `"Material Symbols Outlined"`** so the default resolves with
  no per-component override. Source: npm
  [`material-symbols`](https://www.npmjs.com/package/material-symbols)
  (**Apache-2.0**, 0 deps), file `material-symbols-outlined.woff2` — the
  variable icon font (`FILL`/`wght`/`GRAD`/`opsz` axes).
- **Text — `Roboto Flex`, aliased to `Roboto` and `Google Sans Text`.** The M3
  type scale defaults to `Roboto` via `--md-ref-typeface-brand`/`-plain`
  (verified in `@material/web/typography/md-typescale-styles.css`), and the
  aphrody tokens / `apps/console` reference `"Google Sans Text"` first. We
  vendor one variable face and declare it under all three family names, so the
  embedded font satisfies every reference with no token edits. Source: npm
  [`@fontsource-variable/roboto-flex`](https://www.npmjs.com/package/@fontsource-variable/roboto-flex)
  (**SIL OFL-1.1**, 0 deps), latin subset.

Both licenses are clean for aphrody's Apache-2.0 distribution (Apache-2.0 +
OFL-1.1; no GPL). The `LICENSE-*.txt` files travel next to the woff2 in
`src/fonts/`.

### 2.2 The local `@font-face` sheet

`src/fonts/fonts.css` declares the faces with **relative** `url()`s:

```css
@font-face {
  font-family: "Roboto Flex";   /* + duplicate rules aliasing "Roboto" and "Google Sans Text" */
  src: url("./roboto-flex-latin.woff2") format("woff2-variations");
  font-weight: 100 1000;
  font-stretch: 25% 151%;
  font-style: oblique 0deg 10deg;
  font-display: swap;
}
@font-face {
  font-family: "Material Symbols Outlined";
  src: url("./material-symbols-outlined.woff2") format("woff2");
  font-weight: 100 700;
  font-display: block;          /* hide the glyph slot until ready — never flash the ligature word */
}
```

`app.ts` imports it (`import "./fonts/fonts.css";`) before the components paint.

### 2.3 How Bun makes the woff2 self-contained

Bun's HTML bundler "handles CSS imports and `<link>` tags by concatenating CSS
files and rewriting asset paths with content-addressable hashes"
([Bun — HTML & static sites](https://bun.com/docs/bundler/html-static)). The
relative `url("./material-symbols-outlined.woff2")` is copied to
`dist/assets/material-symbols-outlined-<hash>.woff2` and the reference in the
bundled CSS is rewritten to `url(./assets/material-symbols-outlined-<hash>.woff2)`.
Result: the woff2 ship inside `dist/`, addressed relative to the document — they
load from `tauri://localhost/assets/...` with no network and no asset-protocol
plumbing.

> Why vendor the woff2 into `src/fonts/` instead of `bun add`-ing the font
> packages as runtime deps? Two reasons: (1) the `material-symbols` package is
> ~13 MB unpacked (every weight/fill) — vendoring only the one Outlined woff2
> keeps the dependency surface and `node_modules` lean; (2) it pins the exact
> bytes that ship, independent of registry availability, which matches
> aphrody's reproducible-offline posture. The packages remain the upstream of
> record (cited above); refresh by re-copying from a pinned install.

### 2.4 Shrinking the icon font (optional, deferred)

The 3.94 MB Outlined woff2 carries the full Material Symbols glyph set. If the
app uses a known, fixed icon list, subset it with
[`glyphhanger`](https://github.com/zachleat/glyphhanger) or
[`subset-font`](https://www.npmjs.com/package/subset-font) down to the used
codepoints (typically <50 KB), and replace the vendored file. Not done here so
the demo can use arbitrary icons; revisit when the icon inventory is frozen.

---

## 3. CSP and capabilities for Tauri v2

Material Web is built on **Lit**, which styles components with **constructable /
adopted stylesheets** (`document.adoptedStyleSheets` + `CSSStyleSheet`
populated from `css` template literals). Under a Content-Security-Policy this
counts as inline styling, so `style-src` must allow it. The webview also needs
to load the bundled JS/CSS/fonts from its own origin.

### 3.1 The CSP (offline-adapted)

Tauri's documented CSP example
([Tauri — CSP](https://v2.tauri.app/security/csp)) lists CDN font hosts:

```jsonc
// Upstream Tauri example — NOT what aphrody ships (note the CDN hosts):
"style-src": "'unsafe-inline' 'self' https://fonts.googleapis.com",
"font-src":  ["https://fonts.gstatic.com"]
```

For aphrody the fonts are embedded, so **drop the CDN hosts** and keep
everything on `'self'`. The aphrody `tauri.conf.json` CSP:

```jsonc
{
  "app": {
    "security": {
      "csp": {
        "default-src": "'self'",
        "script-src":  "'self'",
        "style-src":   "'self' 'unsafe-inline'",
        "font-src":    "'self'",
        "img-src":     "'self' data: blob:",
        "connect-src": "ipc: http://ipc.localhost"
      }
    }
  }
}
```

Directive rationale:

- **`style-src: 'self' 'unsafe-inline'`** — required. Lit's adopted stylesheets
  and the components' inline `<style>` need `'unsafe-inline'`; this matches the
  upstream Tauri example (which also carries `'unsafe-inline'`). A strict
  nonce/hash CSP for styles is impractical with Lit because each component
  injects its own sheet at definition time. `'self'` covers the bundled
  `dist/index-*.css`.
- **`font-src: 'self'`** — the woff2 are same-origin assets in `dist/assets/`.
  No `fonts.gstatic.com`. (If you ever fall back to subsetting served via the
  asset protocol, add `asset: http://asset.localhost` here.)
- **`script-src: 'self'`** — the bundled `dist/index-*.js` is same-origin. Add
  `'wasm-unsafe-eval'` **only** if you later load a wasm module in the webview
  (aphrody's wasm runs in the Rust backend, so it is not needed here).
- **`connect-src: ipc: http://ipc.localhost`** — Tauri IPC transport
  (`invoke`, `Channel`). This is Tauri's standard value.
- **`default-src: 'self'`** — deny everything else. No remote content is loaded
  by default, which is the security posture the shell decision relies on
  (`tauri/README.md` §5, R1: the Linux WebKitGTK CVE stream is mitigated because
  aphrody renders only its own first-party content behind the ACL + strict CSP).

### 3.2 Asset protocol — not needed for the fonts

Because the woff2 are bundled into `dist/` and embedded by `tauri-codegen` at
build time, they are served from the app origin (`tauri://localhost`) and need
**no** `assetProtocol` configuration. Leave `assetProtocol.enable = false`
unless the app must read arbitrary files from disk at runtime (it does not for
this UI). This keeps the attack surface minimal — the broad `"allow": ["**/*"]`
asset scope from Tauri's docs is explicitly *not* used.

### 3.3 Capabilities

The frontend is pure rendering + IPC; it needs only the `core:default`
capability set plus whatever custom `#[tauri::command]`s the shell exposes
(e.g. `aphrody_exec`, streaming `Channel` commands). A `capabilities/main.json`
that grants only the app's own commands is sufficient — no plugin permissions
are required for Material Web itself (it touches no Tauri API).

---

## 4. Cross-webview reality — WebKitGTK first (Linux #1)

The shell fixes the engine per OS: **WebKitGTK** (`webkit2gtk-4.1`) on Linux,
**WebView2** (Chromium) on Windows, **WKWebView** on macOS. The frontend must
render on all three; Material Web's mechanism is standards-based, so the bar is
"does the engine implement the relevant web standards."

### 4.1 Custom elements + Shadow DOM — native everywhere

Custom Elements v1 and Shadow DOM are implemented at 100% parity across all
evergreen engines including the WebKit family
([Custom Elements Everywhere](https://custom-elements-everywhere.com/)). The
`md-*` components' core mechanism is native on WebKitGTK with no polyfill.

### 4.2 `ElementInternals` + form-associated custom elements — present on the line Ubuntu ships

Several Material Web form controls (`md-checkbox`, `md-switch`, `md-radio`,
`md-text-field`, `md-slider`, `md-select`) are **form-associated custom
elements** and use `ElementInternals` to participate in `<form>`. WebKit enabled
this by default from Safari Technology Preview 162
([WebKit — ElementInternals and form-associated custom elements](https://webkit.org/blog/13711/elementinternals-and-form-associated-custom-elements/)),
and it is present in current WebKitGTK. The version that matters is the engine
the distro ships: Tauri/wry bind the **`webkit2gtk-4.1`** C API, and
**Ubuntu 24.04+ ships the 4.1 line** (the 4.0 dev packages were removed). aphrody's
#1 target is **Ubuntu 26.04**, newer still — so `ElementInternals` and
form-associated elements are available, and **no `element-internals-polyfill`
is required at runtime** on the Linux target. (That polyfill appears in Material
Web's *devDependencies* for legacy engines only; it is not bundled by
`apps/desktop-ui`, and is not in the `dist/`.)

> If a future target must run on an older engine (e.g. an LTS with a stale
> WebKitGTK, or an embedded WebView), gate the polyfill behind a runtime feature
> check (`'attachInternals' in HTMLElement.prototype`) and load it only when
> absent. It is **not** needed for the three first-class engines on current OS
> versions (WebView2 = Chromium, WKWebView = current Safari, WebKitGTK 4.1 on
> Ubuntu 24.04/26.04).

### 4.3 The genuine WebKitGTK risks (framework-independent)

- **WebKitGTK lags Chromium on bleeding-edge CSS/JS** — some `:has()` /
  container-query edge cases, Houdini. *Mitigation*: the M3 fusion already
  targets evergreen-baseline CSS (as `apps/console` does); headlessly test the
  components on WebKitGTK. Keeping zero framework runtime minimizes the JS
  surface that could differ across engines.
- **Variable-font axis support**: all three engines render variable woff2 and
  honor `font-variation-settings`. `md-icon` sets the FILL/wght axes via CSS;
  Roboto Flex's extra axes are optional. No engine-specific fallback is needed
  for the embedded faces, but verify icon weight changes render on WebKitGTK if
  the design leans on `--md-icon-*` axis tokens.
- **`font-display: block`** on the icon font (per §2.2) avoids the
  flash-of-ligature-text on the slower-to-rasterize engine; since the font is
  local the block period is effectively zero on all three.

### 4.4 WebView2 (Windows) and WKWebView (macOS)

Both are evergreen Chromium/WebKit and render Material Web with no special
handling. The only cross-engine knobs that matter here live *below* the
frontend (Tauri's IPC fast-path thresholds — 8 KB JSON on WebView2, 1 KB raw on
macOS) and affect command payloads, not rendering.

---

## 5. Wiring `dist/` into Tauri `frontendDist`

In the Tauri shell crate's `tauri.conf.json` (the shell lives in the Rust repo
as the build-excluded `crates/aphrody-app` — `tauri/README.md` §6):

```jsonc
{
  "build": {
    // path is relative to tauri.conf.json (in crates/aphrody-app/);
    // point it at the sibling aphrody-ts build output.
    "frontendDist": "../../../aphrody-ts/apps/desktop-ui/dist",
    "devUrl": "http://localhost:3000",
    "beforeBuildCommand": "bun run --cwd ../../../aphrody-ts/apps/desktop-ui build",
    "beforeDevCommand": "bun run --cwd ../../../aphrody-ts/apps/desktop-ui dev"
  },
  "app": {
    "withGlobalTauri": true,        // expose window.__TAURI__ for plain-JS invoke (no ESM import needed)
    "security": { "csp": { /* §3.1 */ } }
  }
}
```

- `tauri-codegen` embeds the built `dist/` into the binary at release, so the
  shipped app is one self-contained executable — no Node, no runtime server, no
  network for the UI.
- The exact relative `frontendDist` path depends on where the shell crate sits
  relative to this repo; pin it once the shell crate exists. The frontend source
  stays in `aphrody-ts` (consistent with the 2026-05-23 extraction).
- IPC from the frontend: with `withGlobalTauri: true`, call
  `window.__TAURI__.core.invoke('aphrody_exec', { args })`; for streaming
  (chat / agy-loop / SSE) consume a Tauri `Channel<T>` via a plain JS callback.
  The current `app.ts` demo uses no IPC (it proves rendering); add the `invoke`
  calls when the shell command surface lands.

### 5.1 Same `dist/` also serves the browser path

Because the output is a plain static bundle, the same `dist/` can be served by
any static server for a browser build, mirroring how `apps/console` keeps a
`Bun.serve` path alongside. Tauri does not use a runtime server — it serves
`frontendDist` directly from the embedded files.

---

## 6. The `@material/web` resolution gotcha (read before `bun install` surprises)

In this workspace, `@material/web` can resolve two ways:

1. **npm `@material/web@2.4.1`** — ships `.js` + `.d.ts` only. Clean types,
   the documented dependency, what `apps/m3-react` and `apps/desktop-ui` use.
2. **`packages/material-web`** — the **vendored fork** (aphrody's extended
   Material Web, 94 `md-*` tags, ships `.ts` source). It is **not** a Bun
   workspace member, has its own `.git`, and is `gitignore`d. On this machine a
   stale **`bun link @material/web`** (a global symlink in
   `~/.bun/install/global/node_modules/@material/web -> packages/material-web`)
   shadows the hoisted top-level `node_modules/@material/web`, pointing it at the
   fork.

The fork's `.ts` source type-checks under `noUncheckedIndexedAccess`-style
strictness and trips `tsc` (e.g. `internal/events/dispatch-hooks.ts`), whereas
the npm package's `.d.ts` is `skipLibCheck`'d. **`apps/desktop-ui` is pinned to
the npm package** (`"@material/web": "^2.4.1"` + a nested real copy under
`apps/desktop-ui/node_modules/`, the same physical state `apps/m3-react` is in),
so its build and `tsc --noEmit` are clean and reproducible. A fresh `bun install`
on a machine **without** the stale global link resolves the npm package by
default.

Takeaways:

- Do **not** rely on the global `bun link` for first-party apps — it is an
  un-committed dev artifact for working *on* the fork.
- If `tsc` in an app suddenly reports errors inside `packages/material-web/**`,
  the app is resolving the fork; ensure it resolves npm `@material/web` (pin the
  version; if a stale link shadows it, materialize a nested copy as `desktop-ui`
  does, or `bun unlink @material/web` globally).
- Per `CLAUDE.md`, `packages/**` are third-party: not linted, not formatted, not
  type-gated by this repo — never edit them to satisfy an app's build.

---

## 7. Verification checklist (what was run, 2026-05-24)

- `bun install` — wires `apps/desktop-ui` as a workspace member. OK.
- `bun run --cwd apps/desktop-ui build` — `dist/` produced; contains
  `index.html`, hashed JS, hashed CSS, and **both** woff2 under `assets/`
  (verified the icon + text fonts are present). OK.
- `dist/` audit — **no** `fonts.googleapis.com` / `fonts.gstatic.com` / external
  `href`/`src`/`url()`; only `http://www.w3.org/2000/svg` (a namespace string).
  Self-contained. OK.
- `bun run --cwd apps/desktop-ui typecheck` — `tsc --noEmit`, **0 errors**
  (resolving npm `@material/web`). OK.
- `oxlint` — **0 warnings/errors** on `apps/desktop-ui`. OK.
- `oxfmt --check src` — all files correctly formatted. OK.
- Bundle sizes — table in §1 (total ~4.4 MB, icon font ~3.94 MB; shrinkable per
  §2.4). Reported.

Not run (out of scope — no Tauri build here): launching the Tauri shell on
WebKitGTK/WebView2/WKWebView. The cross-webview analysis in §4 is grounded in
the engine standards and the `webkit2gtk-4.1` version the Ubuntu target ships;
a headless WebKitGTK render of `dist/` is the next verification step when the
`crates/aphrody-app` shell exists.

---

## 8. Sources

In-repo (verified 2026-05-24): `apps/desktop-ui/**`;
`@material/web@2.4.1` (`icon/icon-styles.css` → `--md-icon-font` default;
`typography/md-typescale-styles.css` → `Roboto` typeface default);
`apps/theme/tokens.css` (`:root` + `.dark`); `apps/console/src/{app.ts,index.html}`
(the build model); `apps/m3-react/{package.json,src/index.ts}` (the
`@material/web` dependency + import shape). Framework decision:
[`../../aphrody/docs/tauri/ui-framework.md`](../../aphrody/docs/tauri/ui-framework.md);
shell decision: [`../../aphrody/docs/tauri/README.md`](../../aphrody/docs/tauri/README.md).

External (verified 2026-05-24):

- Bun HTML bundler (concatenates CSS, rewrites `url()` assets with content
  hashes, copies to outdir) — [Bun — HTML & static sites](https://bun.com/docs/bundler/html-static),
  [Bun — `Bun.build`](https://bun.com/docs/bundler).
- npm [`material-symbols`](https://www.npmjs.com/package/material-symbols)
  (Apache-2.0) — the `material-symbols-outlined.woff2` variable icon font.
- npm [`@fontsource-variable/roboto-flex`](https://www.npmjs.com/package/@fontsource-variable/roboto-flex)
  (SIL OFL-1.1) — the Roboto Flex variable text font.
- Tauri v2 CSP (`style-src` `'unsafe-inline'`, `connect-src ipc:`,
  `font-src`/`img-src`/`script-src`; `'wasm-unsafe-eval'` only for wasm) —
  [Tauri — Content Security Policy](https://v2.tauri.app/security/csp).
- Tauri v2 asset protocol + `assetProtocol` scope — [Tauri — Asset Protocol](https://v2.tauri.app/security/asset-protocol).
- Custom Elements / Shadow DOM 100% engine parity —
  [Custom Elements Everywhere](https://custom-elements-everywhere.com/).
- `ElementInternals` + form-associated custom elements in WebKit (default from
  STP 162) — [WebKit blog](https://webkit.org/blog/13711/elementinternals-and-form-associated-custom-elements/).
- Material Web maintenance mode (Apache-2.0; thin standards-based layer
  preferred) — [Material Web](https://material-web.dev/),
  [maintenance-mode discussion](https://github.com/material-components/material-web/discussions/5642).
