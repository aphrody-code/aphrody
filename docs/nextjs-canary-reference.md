<!-- SPDX-License-Identifier: Apache-2.0 -->

# Next.js Canary Reference (`16.3.0-canary.39`)

Reference for the exact Next.js canary our public sites pin, the AI-agent surface
it ships, the Rust/SWC/Turbopack stack it carries (cross-referenced against
aphrody's root `Cargo.toml`), and a port spec for `@next/playwright`.

Source of truth: a `--depth 1 --branch v16.3.0-canary.39` clone of
`github.com/vercel/next.js` (verified `lerna.json` → `"version": "16.3.0-canary.39"`,
`packages/next/package.json` → `16.3.0-canary.39`). Everything below was read from
that clone; paths are quoted as `file:line`.

## 1. Pinned versions and delta

| Site | Pin | Notes |
| --- | --- | --- |
| **rpbey** (primary) | `next@16.3.0-canary.39` | `rpbey/package.json` + `apps/web` |
| **shenron** | `next@16.3.0-canary.37` | `shenron/package.json` (catalog + apps/site) |

Delta `.37 → .39` (from `git diff --stat v16.3.0-canary.37 HEAD`), scoped to what
matters here:

- **Bundled docs**: only 5 `.mdx` changed — `docs/01-app/02-guides/instant-navigation.mdx`,
  `.../01-directives/use-cache.mdx`, `.../route-segment-config/instant.mdx`,
  `.../next-config-js/rewrites.mdx`. Corpus shape (429 `.mdx`, `01-app`/`02-pages`/
  `03-architecture`/`04-community`) is unchanged → shenron's `.37` agent docs are
  structurally identical, only content-stale on those 5 pages.
- **Agent surface**: `packages/next/src/server/lib/generate-agent-files.ts` and
  `packages/next/AGENTS.md` are **byte-identical** across `.37`/`.39`.
- **`@next/playwright`**: only the version field bumped — **no API change**. The
  port spec in §5 applies to both pinned sites.

## 2. Agent / AI / skills surface inventory

| Path (in clone) | What it is | Our sites consume? |
| --- | --- | --- |
| `AGENTS.md` (21 KB) | Contributor/agent guide for the Next.js monorepo itself | No (contributor-facing, not shipped) |
| `CLAUDE.md` → symlink to `AGENTS.md` | Same file | No |
| `.github/AGENTS.md` | CI/workflow-authoring rules for agents | No |
| `packages/next/AGENTS.md` | 6-line managed block: "This is NOT the Next.js you know… Read the relevant guide in `dist/docs/`" | Indirectly — this block is the template injected into our sites' `AGENTS.md` |
| `.cursor/commands/gt-workflow.md` | Cursor slash-command (Graphite worktree workflow) | No |
| `.cursor/worktrees.json` | Cursor worktree setup hook | No |
| `packages/next/src/server/lib/generate-agent-files.ts` | The generator that writes/upserts the `<!-- BEGIN:nextjs-agent-rules -->` block into a project's `AGENTS.md`/`CLAUDE.md` during `next dev` | **Yes** — this produced rpbey/shenron `AGENTS.md` |
| `packages/next/src/telemetry/detect-agent.ts` | Detects 12 agents (cursor, cursor-cli, claude, cowork, devin, replit, gemini, codex, antigravity, augment-cli, opencode, github-copilot) via env vars (`CLAUDECODE`, `CURSOR_TRACE_ID`, `AI_AGENT`, …) | Trigger only — gates the generator |
| `docs/**` (429 `.mdx`) | The LLM docs corpus → copied to `dist/docs/**` (as `.md`) at build | **Yes** (bundled into `node_modules/next/dist/docs/`) |
| `packages/create-next-app/helpers/generate-agent-files.ts`, `packages/next-codemod/lib/agents-md.ts` | Sibling copies of the same generator (kept in sync per the file header) | No (scaffold/codemod paths) |
| `llms.txt` / `llms-full.txt` | **Absent** at this tag — superseded by the bundled-docs mechanism | N/A (rpbey explicitly dropped its `llms-full.txt` dump) |

No MCP server config and no plugin manifest ship in the Next.js repo at this tag.

### Verified bundled-docs / AGENTS pattern

1. `packages/next/taskfile.js:38-46` — the `copy_docs` task globs `docs/**/*`,
   renames `.mdx → .md`, and `.target('dist/docs')`. That is the `dist/docs/`
   our `AGENTS.md` points at.
2. `generate-agent-files.ts:25-29` emits the block referencing
   `` `node_modules/next/dist/docs/` `` verbatim — exactly what
   `rpbey/AGENTS.md:4` and `shenron/AGENTS.md:4` contain. The pattern is **current
   and matches the real clone**; no drift in the block itself.
3. `next dev` calls `writeAgentFiles` (`start-server.ts:510-529`) only when
   `detectAgent()` returns non-null and `agentRules !== false` in `next.config`.

## 3. Rust-stack inventory (Next ↔ aphrody)

Next's Rust workspace = `/tmp/next.js/Cargo.toml` (members: `crates/next-*`,
`turbopack/crates/*`, `turbopack/xtask`; native bindings crate
`crates/next-napi-bindings`, published as `@next/swc` `16.3.0-canary.39`).

| Crate | Version in Next | Role | In aphrody root `Cargo.toml`? | Useful for bxc? |
| --- | --- | --- | --- | --- |
| `turbopack-*` (≈30 crates) | path deps (`turbopack/crates/*`) | Incremental bundler / dev server / ecmascript+css pipeline | **No** — only referenced in comments (`Cargo.toml:237,253`); tuono\* excluded | Indirect — too heavy to vendor; invoke via `next` CLI, not embed |
| `turbo-tasks-*` (`turbo-tasks`, `-backend`, `-fs`, `-hash`, …) | path deps | Turbopack's incremental computation engine | No | No (Turbopack-internal) |
| `swc_core` | **65.0.3** (`Cargo.toml:213`) | JS/TS parse + transform core | **Yes** but at **63** (`aphrody/Cargo.toml:717`) — **version drift** | Yes — transpile TS/JSX from a Bun/Rust bridge |
| `swc_plugin_backend_wasmtime` | 9.0.0 | Wasm SWC plugin host | Yes, `9` (`:719`) | Maybe (plugin sandbox) |
| `swc_emotion` / `swc_relay` | **4.0.0** | Emotion / Relay transforms | aphrody has `3` (`:720-721`) — drift | Low |
| `styled_jsx` / `styled_components` / `modularize_imports` / `preset_env_base` | `4.0.0` / `4.0.0` / `4.0.0` / `7.0.0` | Next's SWC transform set | aphrody: `styled_jsx 3`, `preset_env_base 7` (`:723,727`) — partial drift | Low |
| `browserslist-rs` | 0.19.0 | Target-browser query | Yes, `0.19` (`:728`) | Yes (CSS targets) |
| `lightningcss` | **1.0.0-alpha.70** (`:280`) | CSS transform/minify (Parcel) | aphrody has **1.0.0-alpha.60** (`:731`) — **drift, 10 alphas behind** | **Yes** — strong bxc candidate (native CSS minify) |
| `lightningcss-napi` | **0.4.6** (`:286`) | N-API binding for lightningcss | aphrody `0.4` (`:732`) | Yes (Bun N-API bridge) |
| `mdxjs` | `1.0.3` crates.io + git fork `vercel-labs/mdxjs-rs-turbopack` (`:224,369`) | MDX→JS | aphrody `1` from crates.io (`:735`) — Next uses a **turbopack fork** for the path dep | Maybe (MDX) |
| `napi` / `napi-derive` / `napi-build` | `2` | Node-API native binding glue (`@next/swc`) | Yes, `2` (`:593-595`) | Yes — the Bun↔Rust bridge mechanism |
| **oxc** | **Not present** in Next at this tag (grep of `Cargo.toml` finds none) | — | aphrody **declares** `oxc_* 0.131` (`:558-562`) | Yes for bxc lint/parse, but **not** a Next dependency |

Notes:
- aphrody's CLAUDE.md "déclaré dans Cargo.toml, pas de re-vendoring" holds for
  `swc_core`, `lightningcss`, `browserslist-rs`, `mdxjs`, `napi`, `oxc` — all
  present as `[workspace.dependencies]`. The **versions lag** Next's canary
  (swc 63 vs 65, lightningcss alpha.60 vs alpha.70, swc_emotion/relay 3 vs 4).
- Genuinely useful "native next module in bxc": **`lightningcss` + `lightningcss-napi`**
  (CSS minify/transform), **`swc_core`** (TS/JSX transpile), and the **`napi`**
  bridge pattern. Turbopack itself is not a vendor target — call `next build`/`next dev`
  as a subprocess from bxc instead.

## 4. `@next/playwright` API + bxc port plan

Package `@next/playwright` (`packages/next-playwright`, `16.3.0-canary.39`,
MIT). It is **tiny and intentionally thin** — two source files:

- `src/index.ts` (149 lines) — exports `instant()`.
- `src/step.ts` (32 lines) — internal `step()` helper.

`package.json`: `peerDependencies: { "@playwright/test": ">=1.0.0" }` (optional);
devDeps `@playwright/test 1.58.2`, `typescript 6.0.2`; build is `tsc -d`
(commonjs, `target es2019`, `types: ["node"]`). **No runtime dep on Playwright** —
it uses structural typing.

### Exact API

```ts
export async function instant<T>(
  page: PlaywrightPage,             // structural: { url(), context() }
  fn: () => Promise<T>,
  options?: { baseURL?: string }
): Promise<T>
```

Behaviour (`src/index.ts`):
1. Reads `page.context().cookies()`; throws if an `instant()` scope is already
   active (nesting unsupported) — `index.ts:62-69`.
2. Resolves the host from `options.baseURL ?? page.url()` (`resolveURL`,
   `index.ts:105-148`; descriptive throw on `about:blank`/fresh page).
3. **Acquire**: `page.context().addCookies([{ name: 'next-instant-navigation-testing',
   value: JSON.stringify([0, 'p'+Math.random()]), domain, path: '/' }])` wrapped in
   a labeled step (`index.ts:76-85`).
4. Runs `fn()`, then **Release** in `finally`: `clearCookies({ name: INSTANT_COOKIE })`
   (`index.ts:88-96`).

`step()` (`src/step.ts`): if `@playwright/test` is present and
`test.step` exists, wrap the body so it shows as a labeled step in the
Playwright UI; on the "can only be called from a test" error (e.g. under Jest)
or when the package is absent, fall back to running the body directly.

The contract is a **single cookie** (`next-instant-navigation-testing`). Next.js
side reads it (CookieStore change event → `navigation-testing-lock.ts`) to serve
only cached/prefetched content during the scope. Requires Cache Components; in
production builds gated behind `experimental.exposeTestingApiInProductionBuild`
(README §"Enabling in production builds").

There is **no** dev/build/start harness, no fixtures, no `next` test helpers in
this package — it is purely the `instant()` navigation-testing primitive.

### bxc port — SHIPPED (`@aphrody-code/next-playwright`)

> **Status (2026-06-04): DONE.** The port lives at
> `bxc/packages/next-playwright` (`@aphrody-code/next-playwright`), built on
> `@aphrody-code/bxc-test`. 8 offline tests green (`tsc --noEmit` + `oxlint`
> clean). The cookie protocol (constant `next-instant-navigation-testing`, value
> shape, nesting guard, `resolveURL` + error text, acquire/release `finally`) is
> copied byte-for-byte; `src/context.ts` is the `CdpCookieContext` adapter over
> `Network.setCookies/getCookies/deleteCookies` + `adaptPage()`; `src/step.ts` is
> the bxc-runner step seam (`setStepReporter`, direct-execution default). The
> port surfaced & fixed a real bxc gap: the `static` Network domain lacked
> `Network.deleteCookies` (`src/cdp/domains/Network.ts`). The bxc-native,
> page-direct variant remains at `bxc/src/next/` (`@aphrody-code/bxc/next`).
> Plan: `bxc/docs/test-package-plan.md` §8.

The original plan that drove this (kept for the record):

**Reuse as-is (copy, MIT):**
- The cookie protocol and constant `next-instant-navigation-testing` — it is the
  load-bearing contract with Next.js and must match byte-for-byte.
- The structural `PlaywrightPage`/`PlaywrightBrowserContext` typing approach
  (keeps the port runner-agnostic).
- The nesting guard, `resolveURL` logic, and the acquire/release `finally`
  semantics — copy verbatim.

**Reimplement on bxc:**
- `step()` must target **bxc's own test runner** instead of `@playwright/test`:
  swap the `require('@playwright/test').test.step` probe for bxc's step API
  (with the same "run body directly if not in a test" fallback).
- The cookie/context calls must go through bxc's browser-automation layer
  (`mcp__bxc__browser_*` / CDP), since bxc drives Chrome over CDP rather than
  Playwright's `BrowserContext`. Provide a thin adapter exposing
  `addCookies` / `cookies` / `clearCookies` over CDP `Network.setCookie` /
  `Network.getCookies` / `Network.deleteCookies`.
- Build with Bun (`bun build`/`tsc`) instead of the Next monorepo's `tsc -d`
  taskfile path.

**Effort:** small. The Next side (cookie semantics) is fixed; the port is ~one
adapter file mapping the two cookie ops onto bxc's CDP surface plus a `step()`
shim. No need to depend on `@playwright/test` at all.
