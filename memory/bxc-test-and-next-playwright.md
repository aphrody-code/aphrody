---
name: bxc-test-and-next-playwright
description: "bxc ships two Playwright-compatible test packages over its native CDP layer — @aphrody-code/bxc-test (test/expect/Page/Locator) and @aphrody-code/next-playwright (Next.js instant() port) — plus the bxc-native src/next module. Where they live, how they differ, and the Network.deleteCookies fix."
metadata: 
  node_type: memory
  type: reference
  originSessionId: e87d3ad8-df91-4692-835f-a6350089539d
---

Shipped 2026-06-04 in bxc (`/home/ubuntu/bxc`, commit `11a923d`). Three Next/
Playwright test surfaces, all on bxc's **in-process CDP** layer (`src/cdp/
domains/*`) — no Chromium bundling, no `@playwright/test` runtime dep:

- **`packages/test` → `@aphrody-code/bxc-test`** — Playwright-shaped
  `test`/`expect`/`TestPage`/`BxcLocator` over CDP DOM + `bun:test`. Web-first
  auto-retry matchers, `getBy*` selectors, `defineConfig` fixture seam. `static`
  profile = fully offline (`Bun.serve({port:0})`). 15 tests. Honest caveats:
  static has **no JS engine** (`Runtime.evaluate` → undefined) and **no layout**
  (`getBoxModel` → zero box) → "visible" = attached & not hidden; `getByRole` is
  a role→CSS heuristic, not a real a11y tree (roadmap = `fast`/Lightpanda
  profile).
- **`packages/next-playwright` → `@aphrody-code/next-playwright`** — faithful
  port of vercel `packages/next-playwright` (`16.3.0-canary.39`, MIT). The
  `instant()` cookie protocol is copied **byte-for-byte** (constant
  `next-instant-navigation-testing`, value `JSON.stringify([0,"p"+Math.random()])`,
  nesting guard via `context().cookies()`, `resolveURL`+error text,
  acquire/release `finally`). Reimplemented: `src/context.ts` `CdpCookieContext`
  (maps `addCookies`/`cookies`/`clearCookies` → `Network.setCookies`/`getCookies`/
  `deleteCookies`) + `adaptPage(testPage)`; `src/step.ts` pluggable bxc reporter
  (`setStepReporter`, direct-exec default). 8 offline tests.
- **`src/next/` → `@aphrody-code/bxc/next`** (pre-existing core, bunlight→bxc
  rebrand) — bxc-native, **page-direct** `instant()` variant: the page itself
  exposes the cookie ops, nesting guarded by an in-process `inFlight` lock; plus
  `withPlaywrightPage()`. Use for quick bxc scripts; use the package for the
  exact vercel surface in `bun test`.

**Real bug fixed in passing:** bxc's `static` Network domain lacked
`Network.deleteCookies`, so `Page.clearCookies({name})` (and the `instant()`
release) was broken offline. Implemented in `src/cdp/domains/Network.ts`
(name + optional domain/path scope) with domain tests.

Plan/compat tables: `bxc/docs/test-package-plan.md` (§8 = next-playwright).
aphrody ref: `docs/nextjs-canary-reference.md` (port marked SHIPPED).
Build: agent 1 built bxc-test; orchestrator assembled next-playwright + the
Network fix. Run: `bun test packages/test/test/ packages/next-playwright/test/`.
See [[bun-test-runner-pattern]].
