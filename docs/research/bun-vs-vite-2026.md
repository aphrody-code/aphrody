<!-- SPDX-License-Identifier: Apache-2.0 -->
# Bun vs Vite as the frontend build/dev toolchain — 2026, with real benchmarks

Research-only document. Compares **Bun** (oven-sh/bun) and **Vite** (vitejs/vite)
as the build / dev-server / HMR layer for the `aphrody-ts` sibling repo's
"M3 fusion" frontend (Material Web / Lit web components + `@aphrody-code/m3-react`
React 19 wrappers + CSS tokens), and recommends a direction. Includes a
**reproducible benchmark with real millisecond numbers** measured on this host,
plus an honest account of the method and its biases.

Author: aphrody-code. Last updated: 2026-05-23.

Versions under test (verified, not assumed):

- **Bun** `1.3.14` (revision `1.3.14-canary.1+bbd3e624a`), already on `PATH`;
  source clone at `var/bun` is `1.4.0` (in development, `HEAD` 2026-05-23).
- **Vite** `8.0.14` (npm `latest`; `vite/8.0.14 win32-x64`), source clone
  `var/vite` `HEAD` = `release: v8.0.14` (2026-05-21). The Vite 8.0 line went
  stable 2026-03-12; broad press coverage landed mid-May 2026.
- **`@vitejs/plugin-react`** `6.0.2`, **React** `19.2.6`, **Node** `26.1.0`,
  **npm** `11.13.0`.
- Repos: `https://github.com/oven-sh/bun`, `https://github.com/vitejs/vite`.

---

## 0. aphrody-ts constraints that frame the decision

Hard inputs, not preferences (from `aphrody-ts/CLAUDE.md` and the repo layout):

1. **`aphrody-ts` is already an all-Bun workspace** — Bun is the runtime,
   bundler, test runner, and package manager. `bun.lock`, `bunfig.toml`, the
   `catalog:` protocol, and `workspace:*` deps are all Bun-native.
2. **`apps/console` already ships on `Bun.serve`** — `bun --hot src/server.ts`
   for dev, an `import("./index.html")` HTML route for the frontend, and
   `bun:ffi` (`@aphrody-code/native`) to drive the whole aphrody CLI in-process.
   This is exactly Bun's fullstack-dev-server model.
3. **The frontend is a fusion**: Material Web (`@material/web`, Lit) +
   `@aphrody-code/m3-react` (React 19 + `@lit/react`) + Tailwind v4 `@theme`
   tokens + `--md-sys-color-*` CSS variables.
4. **Lint/format is oxc** (`oxlint` / `oxfmt`) — the same Rust toolchain family
   that Vite 8 now builds on (oxc). There is no Babel/ESLint footprint to keep.
5. **Latency is a project-wide objective** (memory: *latency-minimal-objective*):
   cold start, rebuild, install, all matter.

The question is therefore narrow: **keep the native-Bun toolchain, or introduce
Vite (on Node or on Bun) for the frontend dev/build loop?**

---

## 1. Roles and overlap — what is actually comparable

These are not the same kind of tool. Bun is a **platform**; Vite is a **build
tool that runs on a platform**.

| Capability | Bun `1.3`/`1.4` | Vite `8` |
|---|---|---|
| JS/TS runtime | Yes (native, JavaScriptCore) | No — needs Node or Bun under it |
| Package manager | Yes (`bun install`, `bun.lock`) | No — pairs with bun/pnpm/npm |
| Test runner | Yes (`bun test`) | No — pairs with Vitest |
| Bundler (production) | Yes (`bun build`, native Zig) | Yes (Rolldown, Rust) |
| Dev server + HMR | Yes (`bun ./index.html`, since 1.3) | Yes (its core feature) |
| TS/JSX transform | Yes (native) | Yes (oxc, via Rolldown) |
| HTML entry bundling | Yes (HTMLRewriter pipeline) | Yes (`index.html` as entry) |
| Plugin ecosystem | Young; reads esbuild-style plugins | Very large; Rollup/Vite plugins |

**The genuinely comparable surface** is the middle of the table: *bundling +
dev server + HMR + TS/JSX/CSS handling*. Everything Bun does above that (runtime,
PM, test) is not something Vite competes with — if you adopt Vite you still run
it **on** a runtime and still need a package manager, and in 2026 the common
recommendation is to run Vite **on Bun** and keep Bun as PM/test, rather than to
"replace Bun with Vite". So three configurations are on the table:

- **A. Native Bun** — `bun build` + `bun ./index.html` (status quo for `console`).
- **B. Vite on Node** — classic `vite` / `vite build`, Node runtime, Bun as PM.
- **C. Vite on Bun** — `bunx vite` / Vite driven by the Bun runtime; Bun as
  runtime + PM + test, Vite as the dev/build layer.

---

## 2. Architecture internals

### Bun (native, Zig)

- Single statically-linked binary; the bundler, transpiler, CSS parser, dev
  server, and HMR runtime are all built in. **Zero `node_modules` cost** to get
  bundling + dev + HMR — they ship inside the `bun` binary already on `PATH`.
- `bun build` is a one-shot bundler (tree-shaking, minify, code splitting,
  `--target browser|bun|node`). It has **no persistent on-disk build cache**;
  every build is effectively "cold", which matters for the benchmark below.
- The fullstack dev server (`bun ./index.html`, since Bun 1.3, Oct 2025) scans
  the HTML with `HTMLRewriter`, bundles the referenced `<script>`/`<link>`
  assets, transpiles TS/JSX/TSX, downlevels CSS, and serves a manifest from
  `Bun.serve`. **The HTML response is gated on the bundle being built**, so the
  first byte of HTML already implies a built graph.
- **HMR**: client API modeled on Vite's `import.meta.hot` (`bun:beforeUpdate`,
  also aliased as `vite:*` for compatibility). The HMR socket is `/_bun/hmr`;
  patches are pushed to the **browser runtime client** (`/_bun/client/...`), not
  to arbitrary socket subscribers. The dev server rebuilds the whole entry graph
  on change (single-graph model), then serves a new hashed bundle.

### Vite 8 (Rust, via VoidZero stack)

Vite 8 is the inflection point: it **drops the old esbuild+Rollup dual-bundler
and ships Rolldown (Rust) as the single unified bundler**. In `var/vite`,
`packages/vite/package.json` lists `"rolldown": "1.0.2"` as a hard `dependency`
and the build script is literally `rolldown --config rolldown.config.ts`;
`esbuild` and `rollup` are now only dev/peer deps. Rolldown embeds **oxc** (the
Rust parser/resolver/transformer/minifier, same team) — Vite's `oxc.ts` plugin
imports `transformSync` and `viteTransformPlugin` from `rolldown/utils` and
`rolldown/experimental`. So Vite, Rolldown, and oxc are now one end-to-end Rust
toolchain.

- **Dev server**: still serves **unbundled native ESM** by default — it
  transpiles each requested module on demand (oxc) and lets the browser fetch
  the import graph. Confirmed empirically: `GET /src/main.tsx` returns
  transpiled ESM with an injected `/@vite/client` HMR shim. (Vite 8 adds an
  experimental **Full Bundle Mode** that pre-bundles for dev — claimed 3x faster
  startup, 40% faster reloads, 10x fewer requests — but it is opt-in.)
- **Build**: `vite build` runs Rolldown end-to-end. Default minifier is **oxc**
  (Vite reports it 30–90x faster than terser, ~0.5–2% worse compression);
  `build.minify: 'esbuild'` is deprecated. `vite build` implies a production
  context (sets the production define / `NODE_ENV`) automatically — a fairness
  point for the benchmark.
- **HMR**: module-graph-driven. The dev server only computes an update for a
  module that is **in the graph that a client has actually requested**; it then
  broadcasts a precise `{"type":"update","updates":[{"type":"js-update",…}]}`
  over the `vite-hmr` WebSocket. This granularity (per-module, accepts
  boundaries) is the maturity Vite is known for.
- SSR / code splitting / CSS modules / asset handling are all first-class and
  battle-tested across SvelteKit, Nuxt, Astro, React Router, Storybook (Vite is
  downloaded ~65M times/week).

**Install footprint** (measured, this project): the Vite 8 toolchain pulls
**71 MB** of `node_modules`, of which the Rolldown native binding alone
(`@rolldown/binding-win32-x64-msvc`) is **23 MB**. Native Bun adds **0 bytes** of
`node_modules` for the same bundling/dev/HMR capability.

---

## 3. Ecosystem and frameworks (relevant to the M3 fusion)

| Concern | Bun | Vite 8 |
|---|---|---|
| React 19 + Fast Refresh | Built-in JSX + HMR; React Refresh integration is newer/less granular | `@vitejs/plugin-react` v6, oxc-powered React Refresh, very mature |
| Lit / web components | Standard ESM classes bundle fine; **HMR granularity for custom elements is weak** (often a full reload) | Same baseline; richer community HMR plugins (`vite-plugin-web-components-hmr`) but custom-element HMR is imperfect everywhere |
| `@material/web` (Lit) | Works (plain ESM) | Works (plain ESM) |
| Tailwind v4 `@theme` | Native `bun-plugin-tailwind`; v4 ships its own bundler integration (no PostCSS chain) | First-class via `@tailwindcss/vite` |
| CSS Modules | Historically a gap (bun#16916); improving | First-class |
| Plugin breadth | Young; reads many esbuild-style plugins | Huge; Rollup + Vite plugin ecosystems, full API compat in v8 |
| Framework presets | Few | Many (SvelteKit/Nuxt/Astro/Router/Storybook) |

For the **specific** fusion (Lit web components wrapped for React via
`@lit/react`, themed by Tailwind v4 `@theme` + CSS variables): both can *build*
it without drama (it is standard ESM + CSS). The differentiators are (a) Vite's
deeper React Fast Refresh and per-module HMR boundaries, vs (b) Bun's
zero-dependency, single-binary simplicity that the repo is already standardized
on. Note that **custom-element HMR is imperfect on both** — neither hot-swaps a
re-registered `customElements.define` cleanly, so the Lit half tends to full-reload
regardless of tool.

---

## 4. Benchmark — real numbers

### 4.1 Method

- **Fixture** (`var/bench-bun-vite/`, gitignored, reproducible via `gen.mjs`):
  a synthetic React 19 + TypeScript app of **150 `.tsx` components** that import
  each other (a real module graph, not a flat fan-out), plus a barrel that
  statically re-exports all of them, an entry `main.tsx`, and an `index.html`.
  `bun build` reports **159 modules** bundled (150 components + entry + barrel +
  react/react-dom). This is representative of a *mid-size component library*,
  which is the shape of the M3 fusion — it is **not** a multi-MB real app.
- **Tooling**: identical pinned versions (§intro). Bun build run with
  `NODE_ENV=production` because `vite build` sets production automatically while
  `bun build` does not — without this the comparison is unfair (see 4.5).
- **Timing**: end-to-end wall-clock as a developer experiences it — process
  spawn → exit (build) or spawn → first successful HTTP fetch (dev server).
  Reported as **median of N runs**, with min/max and the raw samples in the
  harness output.
- **Host**: Windows 11 (build 28020), Intel Core i7-11370H (4 cores / 8
  threads), 15.8 GB RAM. Node 26.1.0. **Single machine, single OS.**
- **Harnesses** (in `var/bench-bun-vite/`): `bench.mjs` (build),
  `bench-dev.mjs` (dev cold start), `bench-hmr.mjs` (edit-to-update).

### 4.2 Production build (7 runs, median ms)

| Build | Bun `bun build --minify` | Vite `vite build` (Rolldown+oxc) | Bun advantage |
|---|---:|---:|---:|
| Cold | **72 ms** (71–75) | **391 ms** (384–416) | ~5.4x |
| Warm | **72 ms** (70–74) | **398 ms** (382–433) | ~5.5x |

Both tools show **no warm-cache advantage on this build shape**: Bun has no
on-disk build cache, and Vite's only deps to pre-bundle are react/react-dom
(cheap), so the bundler work dominates either way. The published Vite-8 case
studies (Linear 46s→6s, Ramp −57%, Beehiiv −64%, "10–30x") are about
*Vite-7→Vite-8* on large real apps, **not** Vite-vs-Bun; they do not contradict
this result.

### 4.3 Dev server cold start (7 runs, median ms)

Measured as spawn → first successful HTTP fetch (the "page is loadable" moment).

| Milestone | Bun `bun ./index.html` | Vite `vite` | Bun advantage |
|---|---:|---:|---:|
| HTML served | **120 ms** (119–122) | **274 ms** (268–342) | ~2.3x |
| + entry module transpiled | **121 ms** (121–124) | **339 ms** (333–404) | ~2.8x |

Bun bundles the entry up front (HTMLRewriter), so HTML and the usable module
land together (~120 ms). Vite serves HTML fast, then pays ~65 ms more to
on-demand-transpile the first module. Bun's server self-reports "ready in
~13–23 ms"; the gap to the measured ~120 ms is **process spawn + runtime init +
first-request bundling** — the honest end-to-end number, not the internal timer.

### 4.4 Edit-to-update latency (6 runs, median ms) — asymmetric milestones

The two servers expose the update signal differently, and a raw WebSocket client
gets **no** update from either (both only update modules a real client has
loaded). So this row compares the **closest available milestone for each**, and
they are **not the same thing**:

| Tool | Milestone measured | Median | Range |
|---|---|---:|---:|
| Vite | file-write → HMR `update` pushed on `vite-hmr` WS (graph pre-populated via HTTP) | **78 ms** | 66–96 |
| Bun | file-write → rebuilt entry bundle served (new content hash at `/`) | **13.6 ms** | 7–21 |

Caveat (important): Vite's number includes computing the precise per-module
update and notifying the client; Bun's number is rebuild-to-serveable, polled,
and **excludes** browser-side patch application. A symmetric in-browser
measurement would need a real headless browser driving both, which adds CDP
round-trip variance that would dominate a sub-100 ms signal — so it was **not**
attempted rather than reported dishonestly. Directionally, Bun's native rebuild
is much faster; Vite's HMR is more *granular* (it patches one module; Bun rebuilds
the entry graph and reloads).

### 4.5 Output bundle size (production)

| Bundle | Raw | gzip |
|---|---:|---:|
| Vite (`dist-vite`, oxc minify) | 279.3 KB | 64.3 KB |
| Bun (`dist-bun-prod`, `NODE_ENV=production`) | 276.7 KB | 63.0 KB |
| Bun **without** `NODE_ENV=production` | 494.1 KB | 125.2 KB |

In production mode the outputs are **within ~1%** — neither tool produces a
meaningfully smaller bundle here. The last row is the trap: a default
`bun build --minify` leaves React in dev mode and nearly doubles the bundle;
`vite build` avoids this automatically. Any Bun build script for the M3 fusion
**must** set `NODE_ENV=production` (or `--define`).

### 4.6 Package manager (warm cache, this project)

Bun is also the PM, so this is part of the toolchain comparison:

| Install (23 pkgs, warm cache) | Time |
|---|---:|
| `bun install` | **1.2 s** |
| `npm install` | 7.8 s |

~6.5x. The point: if you adopt **Vite on Node**, you would still not use npm —
you'd keep Bun (or pnpm) as PM. Vite does not improve installs; Bun already wins
that axis.

### 4.7 Bias / threats to validity (read this)

- **Single host, Windows only.** aphrody's #1 target is Linux; Bun's I/O and
  process-spawn characteristics differ on Linux, and Vite's chokidar/file-watch
  path differs too. These numbers are Windows-11 numbers.
- **Synthetic fixture.** 150 small components is a *library*-shaped graph. A real
  app with large vendor deps, CSS Modules, SSR, dynamic imports, and many assets
  would stress Vite's mature pipeline and Bun's younger one differently — likely
  narrowing Bun's lead and exposing feature gaps.
- **Build has no warm-cache story** for either tool on this graph; on a large app
  Vite's incremental/prebundle behavior matters more.
- **Edit-to-update rows are not the same milestone** (4.4) — treat as directional.
- **Process-spawn overhead** is included in every number (it is part of the real
  developer cost), which slightly favors the single-binary tool (Bun).
- Versions are pinned and exact; Bun `1.4`-line behavior may differ from the
  tested `1.3.14`.

---

## 5. Analysis for the aphrody-ts M3 fusion

What the data and architecture say, mapped to this repo:

1. **Bun already wins every speed axis measured** (build ~5.4x, dev start
   ~2.3–2.8x, rebuild faster, install ~6.5x) **and ties on output size**, on the
   #2 platform. There is no performance reason to add Vite.
2. **The repo is structurally all-Bun** (`bun.lock`, `catalog:`, `workspace:*`,
   `bun test`, `Bun.serve` + `bun:ffi` in `console`). Introducing Vite adds a
   second build model, a 71 MB native-binding `node_modules` footprint, and a
   config surface (`vite.config.ts`, plugins) to maintain — against the repo's
   minimalism and the project's latency objective.
3. **Where Vite is genuinely better is the dev *experience* on the React half**:
   per-module HMR with proper Fast Refresh boundaries, and the enormous plugin
   ecosystem. But the fusion's other half is **Lit/Material Web custom elements,
   whose HMR is imperfect on both tools** — so the part of the app where Vite's
   HMR granularity shines is only the React-wrapper layer, and even there Bun's
   React HMR is improving and "fast enough" given sub-150 ms full reloads.
4. **CSS story is fine on Bun** for this stack: Tailwind v4 ships its own bundler
   integration (no PostCSS chain) and Bun has a native Tailwind plugin; the fusion
   uses `@theme` + CSS variables, not CSS Modules (Bun's weak spot).
5. **"Vite 8 is now Rust too"** removes the old "Vite = slow JS toolchain"
   argument, but it does **not** make Vite faster than Bun here — Bun's native
   bundler still measured several times quicker, and Vite carries the runtime +
   install-footprint overhead Bun does not.

The one real risk of staying on Bun: **bundler maturity / feature breadth**. If
the M3 fusion later needs something Bun's bundler does not do well (advanced CSS
Modules, a Vite-only plugin, SSR/streaming for a framework, mature library-mode
`d.ts` bundling), Bun could force a workaround. That risk is **contained** by the
fact that Vite-on-Bun is a drop-in escape hatch (config C) that keeps Bun as
runtime/PM/test.

---

## 6. Recommendation

**Stay on the native Bun toolchain (config A) for `aphrody-ts`'s frontend
build/dev/HMR. Do not adopt Vite now.**

Rationale: on the measured axes Bun is faster everywhere and ties on bundle size,
the repo is already an all-Bun workspace (so native Bun is zero-friction and
zero-extra-`node_modules`), the fusion's CSS needs sit inside Bun's strengths
(Tailwind v4, CSS variables) and outside its weak spot (CSS Modules), and the
only place Vite is clearly better — per-module React Fast Refresh — is partly
neutralized by the Lit/custom-element half where HMR is imperfect on every tool.
Adding Vite would trade the repo's single-binary minimalism and latency edge for
a maturity insurance policy it does not yet need.

**Hold Vite-on-Bun (config C) as the documented escape hatch**, not the default:
adopt it *only* if a concrete blocker appears (a needed Vite-only plugin, real
CSS-Modules requirements, SSR/streaming for a framework, or library-mode type
bundling Bun can't match). Reassess against the Bun `1.4` line, which is already
in `var/bun` and may further close any feature gap. Do **not** consider Vite-on-
**Node** (config B): it adds a second runtime for no benefit the repo lacks.

### First concrete step

In `aphrody-ts`, harden the **production-mode footgun** found in §4.5 before
anything else: ensure every frontend production path sets `NODE_ENV=production`
(or `--define process.env.NODE_ENV='"production"'`) for `bun build`, so the React
half is not shipped in dev mode (which nearly doubled the bundle: 494 KB → 277 KB
raw). Concretely: add a `build` script to the relevant `apps/*` `package.json`
that runs `NODE_ENV=production bun build ./src/main.tsx --outdir dist --minify
--target browser`, and assert the absence of `react-dom`'s dev-only strings in a
`bun test` size guard. Then leave the dev loop on `bun --hot` / `bun ./index.html`
and revisit Vite-on-Bun only if a named feature gap blocks the fusion.

---

## Sources

All accessed 2026-05-23. Versions verified against local clones (`var/bun`,
`var/vite`) and the `npm`/`bun` registries; benchmark numbers measured locally
(harnesses in `var/bench-bun-vite/`).

- Bun repo and docs: `https://github.com/oven-sh/bun`, Bun fullstack dev server
  `https://bun.com/docs/bundler/fullstack`, Bun hot reloading
  `https://bun.com/docs/bundler/hot-reloading`, Bun bundler plugins
  `https://bun.sh/docs/bundler/plugins`, Bun 1.3 release
  `https://bun.com/blog/bun-v1.3` (dev server + HMR, Oct 2025).
- Vite repo and docs: `https://github.com/vitejs/vite` (`v8.0.14`), "Vite 8.0 is
  out!" `https://vite.dev/blog/announcing-vite8`, Vite 8 beta (Rolldown-powered)
  `https://vite.dev/blog/announcing-vite8-beta`, build options
  `https://vite.dev/config/build-options`, v7→v8 migration
  `https://vite.dev/guide/migration`, Rolldown+oxc integration
  `https://deepwiki.com/vitejs/vite/3.3-rolldown-and-oxc-integration`.
- Vite 8 coverage / benchmark claims: InfoQ
  `https://www.infoq.com/news/2026/05/vite-v8-rust/` (10–30x; Linear 46s→6s;
  Ramp −57%; Beehiiv −64%; release framing), DevClass
  `https://www.devclass.com/development/2026/03/17/vite-team-boasts-10-30x-faster-builds-with-rust-powered-rolldown/5209472`.
- Bun-vs-Vite landscape (2026): PkgPulse
  `https://www.pkgpulse.com/guides/bun-vs-vite-2026`, DEV "Why use Vite when Bun
  is also a bundler?" `https://dev.to/this-is-learning/why-use-vite-when-bun-is-also-a-bundler-vite-vs-bun-2723`.
- Lit / web-component HMR: `https://github.com/web-padawan/awesome-lit`,
  `https://github.com/fi3ework/vite-plugin-web-components-hmr`.
- Bun CSS / Tailwind: `https://github.com/oven-sh/bun/issues/16916` (CSS
  Modules), `https://github.com/joshunrau/bun-plugin-tailwindcss`.
- Local fixtures and harnesses (gitignored): `var/bench-bun-vite/gen.mjs`,
  `bench.mjs`, `bench-dev.mjs`, `bench-hmr.mjs`; clones `var/bun`, `var/vite`.
