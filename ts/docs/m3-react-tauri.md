<!-- SPDX-License-Identifier: Apache-2.0 -->
# m3-react in Tauri — the React surface for aphrody's desktop shell

How `@aphrody-code/m3-react` (React 19 + Material Web) plugs into a Tauri v2
webview, the Gemini / Google design patterns it applies, and when to reach for
React vs the vanilla `apps/desktop-ui` shell.

Last updated: 2026-05-24. Author: aphrody-code.

> **Scope and decision context.** The canonical Tauri frontend decision
> (`C:\src\aphrody\docs\tauri\ui-framework.md`) is unambiguous: the **default
> shell is vanilla TS + Lit/Material Web** (`apps/desktop-ui`), because it is the
> lightest layer between the framework-agnostic `md-*` custom elements and the
> webview, with the fastest cold start on WebKitGTK (Linux is target #1). **React
> 19 is explicitly demoted as the *shell* framework** (heaviest runtime; `@lit/react`
> wrappers are redundant now that React 19 scores 100% on Custom Elements
> Everywhere). This document does **not** relitigate that. It covers the
> complementary case the decision leaves open: a **React *surface*** —
> `apps/desktop-react` — for screens that are genuinely React (an embedded React
> host, a docs/marketing microsite, a contributor who wants React DX for a
> specific view), built the exact same Tauri-ready way as the vanilla shell.

---

## 0. TL;DR

- **`apps/desktop-react`** is a Tauri-ready React 19 app, the React counterpart
  to the vanilla `apps/desktop-ui`. It consumes the published lib
  `@aphrody-code/m3-react` (Material Web wrappers + Gemini interaction
  primitives) + `@aphrody-code/theme` tokens + the same offline fonts, builds
  with Bun (`NODE_ENV=production bun build`) to a self-contained `dist/`, and
  calls aphrody commands through a **transport-abstract client** (`src/transport.ts`):
  Tauri `invoke`/`Channel` on desktop, `fetch /api/run` on web.
- It applies the **Gemini "long reflection" + streaming** affordances and the
  **design.google View Transitions / scroll-reveal** patterns via
  `@aphrody-code/m3-react/interactions` — all reduced-motion aware.
- **Bundle cost is the headline tradeoff**: the React `dist/` JS is **773.5 KB**
  minified vs the vanilla `desktop-ui` **266.3 KB** (+507 KB, ~2.9x) for the same
  fonts and tokens. That delta is exactly why vanilla stays the default shell and
  React is a per-surface opt-in.

---

## 1. The two desktop frontends, side by side

| | `apps/desktop-ui` (vanilla) | `apps/desktop-react` (React surface) |
|---|---|---|
| Framework runtime in webview | none (only the Lit the `md-*` components pull in, ~5-7 KB) | React 19 + react-dom + `@lit/react` |
| Components | `md-*` imported directly, used in HTML | `@aphrody-code/m3-react` wrappers (curated subset) |
| Interaction patterns | hand-wired DOM | `@aphrody-code/m3-react/interactions` (shared lib) |
| Tokens | `@aphrody-code/theme/tokens.css` | same |
| Fonts | embedded Roboto Flex + Material Symbols woff2 | same files (copied; identical hashes) |
| Build | `bun build src/index.html` | `bun build src/index.html` (identical flags) |
| Output | static `dist/` for Tauri `frontendDist` | static `dist/` for Tauri `frontendDist` |
| `dist/` JS (minified) | **266.3 KB** | **773.5 KB** |
| Role | **default shell** (per ui-framework.md) | **React-specific surfaces only** |

Both are real, both build green, both are self-contained (no CDN). They differ
only in the layer between the custom elements and the webview — which is the
whole point of the framework decision.

---

## 2. How it branches to Tauri

The Tauri shell decision is settled in `C:\src\aphrody\docs\tauri\`: a Rust crate
(`crates/aphrody-app`, build-excluded from the lean workspace) hosts the system
webview and exposes `#[tauri::command]`s that call the `cli` library in-process
(Path (a)). The frontend's only job is to (1) build to a static `dist/` and (2)
talk to those commands. `desktop-react` does both:

### 2.1 Build wiring (`tauri.conf.json` `build` block)

Per `docs/tauri/bun-in-tauri.md` §4.2, the Tauri crate points at the Bun build
output:

```jsonc
{
  "build": {
    "beforeDevCommand": { "script": "bun run dev", "wait": true },
    "beforeBuildCommand": "bun run build",     // NODE_ENV=production bun build … (set in package.json)
    "devUrl": "http://localhost:1420",          // pin the Bun dev server to this port
    "frontendDist": "../desktop-react/dist"     // relative to the src-tauri / crate dir
  }
}
```

`desktop-react`'s `build` script is `NODE_ENV=production bun build src/index.html
--outdir dist --minify --sourcemap=none --asset-naming='assets/[name]-[hash].[ext]'`
— native Bun (style A2, not Vite-on-Bun), which sidesteps the two biggest Bun+Tauri
footguns (`--bun`/shebang runtime substitution and Vite/esbuild child-spawn) per
`bun-in-tauri.md` §3.1-3.2. `NODE_ENV=production` is mandatory so React isn't
shipped in dev mode (it nearly doubles the bundle otherwise). `tauri-codegen`
embeds the `dist/` into the binary at release — one self-contained executable,
no Node, no runtime server, no bundled browser.

### 2.2 Calling commands — the transport abstraction (`src/transport.ts`)

The same React bundle ships to two hosts, so the call path is abstracted behind
one interface and selected at runtime by **feature detection** (presence of the
Tauri-injected globals — no separate build):

- **Desktop (Tauri):** `invoke('aphrody_exec', { args })` reaches the Rust command
  that wraps `aphrody::run_async` in-process and returns the captured
  `{ code, stdout, stderr }` (Path (a) + the shared `aphrody-capture` crate,
  `aphrody-integration.md` §7). Streaming uses a Tauri `Channel<string>` handed
  to `aphrody_exec_stream` — Tauri's `Channel` maps directly onto aphrody's
  streaming commands (chat / agy-loop / SSE), a capability the FFI string-return
  path does not have.
- **Web (browser / `Bun.serve` console):** `POST /api/run` with `{ args }`
  returning `{ code, stdout, stderr }` — **exactly the contract
  `apps/console/src/{app.ts,server.ts}` already serves**. Streaming falls back to
  a chunked `fetch` body reader (`/api/run?stream=1`).

Tauri's JS API is reached through the injected `window.__TAURI__` global
(`app.withGlobalTauri: true`, `architecture.md` §4), so **`desktop-react` adds no
`@tauri-apps/api` dependency**: the web/console build stays Tauri-free, and the
desktop build reads the global the shell injects. One bundle, two hosts.

### 2.3 Tokens, fonts, components — same sources as everything else

- **Tokens**: `import '@aphrody-code/theme/tokens.css'` (the `aphrody design
  tokens --fusion` output), identical to `desktop-ui` and `console`. Light + dark
  via `--md-sys-color-*`; the sheet flips on `class="dark"` on `<html>` **and**
  honours `@media (prefers-color-scheme: dark)`. (Note: in an OS-dark
  environment the media query keeps surfaces dark regardless of the `.dark`
  class — this is the shared token-sheet behaviour, identical across all aphrody
  web surfaces, not specific to this app.)
- **Fonts**: the same Roboto Flex + Material Symbols woff2 vendored under
  `src/fonts/` (copied from `desktop-ui`, identical content hashes), declared via
  local `@font-face` and rewritten by Bun to hashed `dist/assets/*.woff2`. Zero
  CDN — required because a Tauri webview has no network.
- **Components**: `src/components.tsx` re-exports the *named* subset of
  `@aphrody-code/m3-react` the demo uses, so Bun tree-shakes the bundle to only
  those `md-*` elements + the interaction helpers (the lib's full ~70-component
  barrel is never pulled in). `md-icon` has no React wrapper, so it is registered
  via a side-effect import (`@material/web/icon/icon.js`) and rendered as a raw
  custom element — React 19 drives custom elements natively (100% Custom Elements
  Everywhere), so no `@lit/react` wrapper is needed for it.

---

## 3. Gemini / Google design patterns applied (with RE sources)

The patterns below are reproduced on standard platform APIs in
`@aphrody-code/m3-react/interactions` (`apps/m3-react/src/interactions.tsx`) and
wired into the demo (`apps/desktop-react/src/app.tsx`). They are grounded in a
live runtime analysis of the two Google surfaces plus the in-repo reverse
engineering of Google's own desktop app.

### 3.1 Why a webview at all — the google.exe RE

The strongest cross-check that "Tauri (system webview) + web tech" is the right
model for an aphrody desktop frontend is that **Google ships its own Windows
desktop app the same way**. The authorized RE map of the operator's
`%LOCALAPPDATA%\Google\Google` install
(`C:\src\aphrody\docs\research\google-local-install-map.md`) classifies
`latest\google.exe` (22.27 MB, signer "Google LLC") as family **`web_view2`**:
it is a **Microsoft Edge WebView2 host** that renders its launcher / Search /
Lens / account UI as **local web content** (`html\` ships `main.html`,
`lens_overlay.html`, `login_page.html`, `onboarding_v2.html`, `settings.html`,
`variables.css`, plus `GoogleSans-v12.ttf` / `Roboto.ttf`) —
`google-local-install-map.md` §1, §2.1. It is explicitly **not** the Antigravity
IDE (that is a separate Electron app) — `google-local-install-map.md` §1, and the
in-repo memory `magika-webview2-re`.

Two takeaways for aphrody's frontend:

1. **WebView2 = Tauri-on-Windows is consistent with Google's own choice.** Tauri
   binds the OS webview (WebView2 on Windows, WebKitGTK on Linux, WKWebView on
   macOS); Google's desktop app uses the same WebView2 runtime as a host for
   local HTML/CSS/JS. Rendering `md-*` + tokens in that host is exactly what
   `google.exe`'s `html\` surface does (down to bundling **Google Sans + Roboto**
   locally and theming via a `variables.css` token sheet — the same shape as
   `@aphrody-code/theme`). aphrody's offline-font + local-token approach mirrors
   this verbatim.
2. **Offline-first asset bundling is the norm, not a workaround.** Google's app
   vendors its fonts and CSS tokens into the install rather than hitting
   `fonts.googleapis.com`; `desktop-react` (and `desktop-ui`) do the same with
   embedded woff2 + `tokens.css`, which is mandatory for a Tauri webview that has
   no network.

### 3.2 Gemini "long reflection" (thinking) + streaming answer reveal

Source: a live runtime analysis (2026-05-23) of `gemini.google.com`, captured in
the module header of `apps/m3-react/src/interactions.tsx`. The thinking state is
built from `gem-shimmer-sweep` (skeleton shimmer), `animateGradient` /
`gradientScroll` (the moving brand gradient), `input-area-spin` (an animated
gradient ring around the composer while processing), and answers stream in
token-by-token with each block `fade-in-up`-ing on arrival. The underlying
transport is Gemini's `StreamGenerate` POST — reverse-engineered in
`C:\src\aphrody\crates\gemini-web` and documented in
`C:\src\aphrody\docs\research\gemini-web-feature-matrix.md` (text chat verified
live; reply parsed token-by-token; `cid/rid/rcid` threading).

Applied in `desktop-react`:

- **`ThinkingIndicator`** (brand-gradient dot + shimmer-sweep bar) shows while a
  command is awaiting its first output chunk — the analogue of Gemini's
  pre-first-token reflection state.
- **`GradientBorder` (`active`)** wraps the command input in the brand-gradient
  "input-area-spin" ring while a command is running — Gemini's composer
  processing affordance.
- **`StreamingText`** renders the command's output progressively (token-by-token,
  `fade-in-up`, blinking caret that hides on completion) as chunks arrive from
  the transport's `streamCommand()` async iterable — the analogue of Gemini's
  streamed answer. The Gemini brand gradient stops
  (`#4285f4 → #9168c0 → #d96570`) are used verbatim.

### 3.3 design.google navigation + reveal motion

Source: the same live analysis, design.google half: navigation runs through the
**View Transitions API** (`view-transition-name: root`); content reveals are
**IntersectionObserver**-driven (no CSS scroll timelines); hover is a 0.165s
colour transition on the decelerate curve `cubic-bezier(0, 0.4, 0.2, 1)`. Applied
in `desktop-react`:

- **`useViewTransition()`** wraps the light/dark theme swap and the entry of each
  new output block, so view changes animate the design.google way (with a
  reduced-motion fallback to a plain synchronous update). `styles.css` sets
  `:root { view-transition-name: root }` so the root cross-fade applies.
- **`Reveal`** (IntersectionObserver fade+lift) is available from the same lib for
  longer scrolling surfaces; the demo is single-screen so it is exported but used
  sparingly.

All of the above no-op to a static, fully-visible state under
`prefers-reduced-motion: reduce` (verified in the lib's keyframe guards).

---

## 4. React vs vanilla — bundle comparison and when to use which

### 4.1 Measured bundle sizes (Bun production build, 2026-05-24)

Both apps were built with the identical flags (`NODE_ENV=production bun build
src/index.html --outdir dist --minify --sourcemap=none`). The fonts are shared,
content-hashed, and byte-identical, so they are excluded from the "code" total.

| Artifact | `desktop-ui` (vanilla) | `desktop-react` (React) | Delta |
|---|---:|---:|---:|
| `dist/*.js` (minified) | 266.3 KB (272,682 B) | 773.5 KB (792,051 B) | **+507.2 KB (~2.9x)** |
| `dist/*.css` | 12.0 KB (12,281 B) | 14.6 KB (14,975 B) | +2.6 KB |
| `index.html` | 9.2 KB (inline layout CSS) | 1.2 KB (CSS extracted) | — |
| **code total (JS+CSS)** | **278.3 KB** | **788.1 KB** | **+509.8 KB** |
| `material-symbols-outlined.woff2` | 3.85 MB | 3.85 MB | identical (shared) |
| `roboto-flex-latin.woff2` | 318.8 KB | 318.8 KB | identical (shared) |

The +507 KB JS delta is **React 19 + react-dom + `@lit/react` + the wrapped
component surface**, on top of the Material Web/Lit both apps share. (These are
unzipped minified bytes; gzip/brotli over the wire compresses both substantially,
but the *relative* ~2.9x gap stands — it is the framework runtime, not content.)
Because Tauri embeds `dist/` into the binary, this delta is paid in binary size
and parse/instantiate time on every cold start — heaviest on WebKitGTK (Linux #1),
which is precisely the cost the ui-framework.md decision optimizes against for the
default shell.

### 4.2 When to use which

**Use vanilla `desktop-ui` (the default shell)** when:
- it is the main aphrody desktop window / the Tauri shell itself;
- cold-start latency on Linux #1 matters (it always does for the shell);
- the UI is mostly streamed text output + standard `md-*` controls (the CLI
  console workload) — a VDOM buys little there;
- you want the smallest binary and the fewest moving parts.

**Use React `desktop-react` (a React surface)** when:
- the screen is genuinely React — e.g. embedding an existing React component
  tree, a docs/marketing microsite, or a view a contributor is building with
  React DX and hooks;
- you specifically want `@aphrody-code/m3-react`'s typed wrappers + the
  interaction hooks in a React composition model;
- the ~2.9x JS overhead is acceptable for that surface's value (it is a
  *surface*, not the shell, so it does not tax the shell's cold start).

This mirrors the ui-framework.md ranking exactly: **vanilla/Lit is primary;
React + m3-react is kept for React-specific surfaces and must not dictate the
shell's framework.** `desktop-react` is the concrete, build-green realization of
"keep m3-react for any React surface" — wired to Tauri the same way, so adopting
it for a screen is a drop-in, not a re-architecture.

---

## 5. Files

`apps/desktop-react/`:

- `package.json` — React surface app; deps `@aphrody-code/m3-react`,
  `@aphrody-code/theme`, `@material/web`, `react`, `react-dom`; native Bun build.
- `src/index.html` — Bun bundler entry (`class="dark"`, pre-hydration background).
- `src/main.tsx` — React root + ordered side-effect imports (fonts → tokens →
  typescale → `md-icon` → app).
- `src/app.tsx` — the demo: command panel + Gemini-style streamed output zone +
  theme toggle, all via View Transitions / thinking / streaming primitives.
- `src/transport.ts` — the transport-abstract client (Tauri `invoke`/`Channel`
  vs `fetch /api/run`), no `@tauri-apps/api` dependency.
- `src/components.tsx` — curated tree-shakeable re-exports from
  `@aphrody-code/m3-react`.
- `src/styles.css` — layout-only sheet (all colour from tokens).
- `src/globals.d.ts` — `declare module '*.css'`.
- `src/fonts/` — embedded Roboto Flex + Material Symbols woff2 + `fonts.css` +
  upstream licenses (Apache-2.0 / SIL OFL-1.1).

Verification (2026-05-24): `tsc --noEmit`, `oxlint`, `oxfmt --check`, and the
production `bun build` all pass; the built `dist/` was served and driven in a
real webview — React mounts, `md-*` components upgrade (shadow DOM), the
transport detects `web`, a preset command streams token-by-token into the
Gemini-style output zone with the correct exit-code badge, and the dist contains
no CDN references (fonts resolve to local hashed `./assets/*.woff2`).

---

## 6. Sources

In-repo (read-only, Rust core repo `C:\src\aphrody`):
- `docs/tauri/ui-framework.md` — the frontend-framework decision (vanilla primary,
  React demoted as shell, kept for React surfaces).
- `docs/tauri/aphrody-integration.md` — Path (a) in-process command integration,
  `Channel` streaming, the `aphrody-capture` crate.
- `docs/tauri/architecture.md` / `docs/tauri/bun-in-tauri.md` — IPC mechanics,
  `withGlobalTauri`, the Bun build contract and footguns.
- `docs/research/google-local-install-map.md` — `google.exe` = WebView2 host RE
  (family `web_view2`, local HTML/Google Sans/Roboto/`variables.css`).
- `docs/research/gemini-web-feature-matrix.md` + `crates/gemini-web` —
  Gemini `StreamGenerate` / token-by-token streaming RE.

In-repo (this repo `C:\src\aphrody-ts`):
- `apps/m3-react/src/interactions.tsx` — the Gemini/design.google interaction
  primitives (module header documents the live runtime analysis, 2026-05-23).
- `apps/desktop-ui/` — the vanilla shell this app parallels.
- `apps/console/src/{app.ts,server.ts}` — the `/api/run` web transport contract.
