<!-- SPDX-License-Identifier: Apache-2.0 -->
# TanStack for aphrody — which libraries help the vanilla-TS/Lit UI, and how

Research-only document (read-only investigation; no source modified). Evaluates
the **TanStack** ecosystem (https://github.com/TanStack) against aphrody's
front-end, decides per-library adopt/defer/reject, and gives a ranked plan with
a concrete first integration.

Author: aphrody-code. Last updated: 2026-05-24.

---

## 0. The verdict in one screen

aphrody's UI is **decided**: Vanilla TypeScript + Lit / Material Web custom
elements, built by **Bun** into a static `dist/`, served two ways —
`apps/console` (`Bun.serve` + browser, live today) and the future Tauri shell
(`apps/desktop-ui`, offline webview, no SSR). It calls the CLI via
`runCaptured(args) -> {code, stdout, stderr}` (bun:ffi in web, `invoke`/Channels
in Tauri), and most output is **structured/tabular** (`doctor --json`, `scan`,
RE, forensics), **streamed** (long commands, `agy-loop`), or driven by
**forms** (command / config input). License is Apache-2.0 with a **zero-GPL**
policy ([CLAUDE.md] §0.1, [ui-framework.md], [gui-options-2026.md]).

The single most important finding: **TanStack is not "a React thing" for the
parts aphrody needs.** Every relevant library is a *framework-agnostic core*
(`*-core`) plus thin adapters, and TanStack now ships **first-party Lit
adapters** for Query, Table, Virtual, Form, and Store — exactly aphrody's shell.
All packages are **MIT** (Apache-2.0-compatible, no GPL). So the usual
disqualifier ("the adapter is React-only") does **not** apply here.

What aphrody should adopt:

- **Now**: **TanStack Virtual** (`@tanstack/lit-virtual`) for streamed/long
  output panes and big file/RE/forensic listings; **TanStack Store**
  (`@tanstack/store`, core) to replace the console's ad-hoc DOM state.
- **When a real grid is built**: **TanStack Table** (`@tanstack/lit-table`, on
  the **v9** plugin line once it stabilises) for sortable/filterable
  `doctor --json` / `scan` / forensic tables.
- **Optional / situational**: **TanStack Query** (`@tanstack/lit-query`) only
  for the genuinely *async, cacheable, refetchable* commands (status polling,
  `doctor`, listings) — its value over a hand-written wrapper is real but
  smaller when the "server" is an in-process FFI call; **TanStack Form**
  (`@tanstack/lit-form`) only when config/command forms grow validation-heavy;
  **TanStack Pacer** (core) for debounce/throttle of live inputs and stream
  flushes.
- **Reject for this shell**: **TanStack Router** and **TanStack Start**
  (Router = React/Solid-only adapters + weight aphrody doesn't need for a
  handful of webview views; Start = a full SSR server framework, the exact
  thing Tauri forbids). **TanStack DB** (heavy differential-dataflow client
  database, React/Solid/Vue-first, no first-party Lit adapter, solves a
  sync/collections problem aphrody does not have). **TanStack Config** is a
  build-tooling repo for *publishing TanStack itself*, irrelevant to a Bun repo.

---

## 1. Method and what was actually measured

Versions, licenses, and dependency trees were read from the **npm registry**
on 2026-05-24 (`npm view <pkg> version license`, `npm pack`). **Bundle sizes
were measured locally, not taken from a calculator**: each package was
`npm pack`-ed, extracted, linked into a `node_modules`, and bundled with the
**actual aphrody toolchain** — `bun build <entry> --minify --target=browser`
(Bun 1.3.14) — then gzipped (`gzip -9`). This is the production path the
front-end uses ([bun-vs-vite-2026.md] confirms Bun is the bundler), so the
numbers reflect what would really ship to the webview, not a synthetic
"min+gz" of a different toolchain. Probe artifacts live in the gitignored
`var/tanstack-probe/` (clones/tarballs only; nothing committed).

Caveats on the sizes (read honestly):

- They are **whole-package** bundles (`export *` of the entry). Real code
  imports a subset and **tree-shakes further down** — e.g. `@tanstack/store`
  used as just `Store` + `Derived` is smaller than the 2.2 KB whole-package
  figure; a Table built from only the row/sort/filter models is smaller than
  the 15.7 KB v8 whole-core figure.
- For the **Lit adapters**, `lit`, `@lit/reactive-element`, `@lit/context`,
  `@lit-labs/observers` were marked **external**, because aphrody already ships
  Lit (Material Web depends on it — [ui-framework.md] §1). So the adapter
  figures are the **incremental** cost over the Lit aphrody already pays.
- Sizes are Bun-on-Windows; Bun output is platform-stable, but treat them as
  ±a few hundred bytes, not exact-to-the-byte truth.

---

## 2. Per-library table (verdict at a glance)

Sizes = measured gzip via `bun build --minify --target=browser` (this host,
2026-05-24). "Lit/vanilla core?" = is there a first-party Lit adapter **or** a
usable framework-agnostic core (yes is required for aphrody's shell).

| Library | npm pkg (version) | aphrody need it covers | Lit/vanilla core? | Size (gzip, measured) | License | Verdict |
|---|---|---|---|---|---|---|
| **Virtual** | `@tanstack/virtual-core` 3.15.0 / `@tanstack/lit-virtual` 3.13.26 | Virtualize streamed output, logs, big file/RE/forensic listings (60 fps, full markup control) | **Yes** — first-party `lit-virtual` (ReactiveController); core is FW-agnostic | core ~6.4 KB; lit adapter +core (Lit ext) ~6.5 KB (adapter glue ~0.1 KB) | MIT | **Adopt now** |
| **Store** | `@tanstack/store` 0.11.0 (core); `@tanstack/lit-store` 0.13.2 | Framework-agnostic reactive signals to replace the console's ad-hoc DOM state (`Store`/`Derived`/`Effect`) | **Yes** — designed FW-agnostic; `subscribe()` works in vanilla; optional `lit-store` controller | core ~2.2 KB (whole pkg) | MIT | **Adopt now (via core)** |
| **Table** | `@tanstack/table-core` 8.21.3 (v9 `9.0.0-alpha.50`) / `@tanstack/lit-table` 8.21.3 | Sortable/filterable/paginated tables for `doctor --json`, `scan`, RE/forensic rows | **Yes** — first-party `lit-table` (Lit 3) | v8 core ~15.7 KB; **v9 ~6–7 KB** (plugin/tree-shakeable, per TanStack) | MIT | **Adopt when a grid is built — prefer v9 line** |
| **Query** | `@tanstack/query-core` 5.100.14 / `@tanstack/lit-query` 0.2.6 | Cache/loading/retry/invalidation for *async* commands (status, `doctor`, listings) | **Yes** — first-party `lit-query` 0.2.x (`QueryController`); core `QueryObserver.subscribe()` is vanilla | core ~12.4 KB; lit adapter +core (Lit ext) ~15.9 KB | MIT | **Optional — adopt only for genuinely async/cacheable commands** |
| **Form** | `@tanstack/form-core` 1.32.0 / `@tanstack/lit-form` 1.24.1 | Validation/state for config & command-builder forms | **Yes** — first-party `lit-form` (`TanStackFormController`) | core ~13.4 KB (pulls `pacer-lite`) | MIT | **Defer — adopt when forms get validation-heavy** |
| **Pacer** | `@tanstack/pacer` 0.21.1 (core) | Debounce/throttle/rate-limit/queue/batch live inputs & stream flushes | **Yes** — FW-agnostic core; **no** Lit adapter (not needed) | full barrel ~9.2 KB; single primitive (e.g. `debounce`) far less | MIT | **Optional — tiny, import per-primitive** |
| **Router** | `@tanstack/react-router` / `solid-router` | View routing in the webview | **No** — adapters are **React/Solid only**; no Lit/vanilla adapter | n/a (heavy) | MIT | **Reject for this shell** |
| **Start** | `@tanstack/react-start` (+ Solid) | Full-stack meta-framework | **No** — **SSR server framework** (Vite + server functions); React/Solid only | n/a | MIT | **Reject (Tauri = no SSR)** |
| **DB** | `@tanstack/db` 0.6.7 | Client-first reactive DB: collections, live queries, optimistic mutations | **No first-party Lit adapter**; React/Solid/Vue-first; differential-dataflow engine is heavy | n/a (large) | MIT | **Reject (solves a problem aphrody lacks)** |
| **Config** | `@tanstack/config` 0.22.2 | Build/publish tooling for libraries | N/A — it's TanStack's own monorepo build config | n/a | MIT | **Reject (irrelevant to a Bun repo)** |

Dependency hygiene (verified): `query-core`, `table-core`, `virtual-core`,
`store` declare **zero runtime dependencies** — clean, self-contained, ideal
for an offline bundle. `form-core` pulls `@tanstack/pacer-lite` (0.2.2, no
further deps); `pacer` pulls `@tanstack/devtools-event-client` (0.4.3, no
further deps, dev-tooling glue, tree-shakeable in prod). The Lit adapters peer-
depend on `lit >=2.8.0 <4` — **the same range Material Web uses**
([ui-framework.md] §1), so there is no second Lit copy and no version conflict.

---

## 3. Why each verdict — mapped to aphrody's concrete surfaces

### 3.1 Virtual — adopt now (the clearest win)

aphrody's output panes are exactly TanStack Virtual's use case: "render massive
lists, grids, and tables at 60 FPS while giving developers full control over
markup and styles" ([TanStack Virtual]). Today `apps/console` dumps stdout into
a single `<pre>` via `output.append(pre)` with no windowing
(`C:\src\aphrody-ts\apps\console\src\app.ts`, lines 47–51). A long `scan`, a
streamed `agy-loop` run, a big `re strings`, or a forensic file listing will
grow that pane unbounded — jank on WebKitGTK (Linux #1, the weakest engine —
[ui-framework.md] §3). `@tanstack/lit-virtual` solves this with the
`@lit/context`-style **ReactiveController** pattern (a `Virtualizer` wrapped for
Lit), and the measured incremental cost over the Lit aphrody already ships is
**~0.1 KB of adapter glue on top of ~6.4 KB of core** — negligible for the
offline budget. The core is dependency-free. There is no DOM-native equivalent:
hand-rolling a correct windowing virtualizer (variable row heights, scroll
restoration, overscan) is precisely the wheel TanStack already de-risked.

First binding: a `lit` output-list element that virtualizes streamed lines from
the Tauri `Channel` / `Bun.serve` stream, keyed by line index.

### 3.2 Store — adopt now (replace ad-hoc state)

The console wires state by hand: `runButton.disabled = true/false`,
`output.replaceChildren()`, version text mutated imperatively
(`C:\src\aphrody-ts\apps\console\src\app.ts`). As the UI grows (multiple panes,
tabs, a command palette, persisted presets), that hand-wiring is where bugs
live. `@tanstack/store` is "first and foremost a framework-agnostic signals
implementation … can be used in vanilla JavaScript or TypeScript"
([TanStack Store]) with `Store` / `Derived` / `Effect` and a plain
`store.subscribe(cb)` that returns an unsubscribe — drives `requestUpdate()` in
a `LitElement` or a manual re-render in vanilla. Measured **~2.2 KB gz**
whole-package (less when tree-shaken), zero deps. A `@tanstack/lit-store`
controller (0.13.2) exists if a tighter Lit binding is wanted, but the core
alone is enough for vanilla. This is the lightest possible "real reactivity"
upgrade and stays inside the standards lane the UI doc prescribes
([ui-framework.md] §4.1, "graduating to Lit-as-app-framework"). Alternative
considered: Lit's own `@lit-labs/signals` / `SignalWatcher` — viable and also
fine; Store is picked because it's the shared substrate the *other* TanStack
libs (Form, Table v9, Query) build on, so adopting it once pays off if any of
those land later.

### 3.3 Table — adopt when a real grid exists, on the v9 line

Structured outputs (`doctor --json`, `scan`, RE section/symbol lists, forensic
artifact tables) are the canonical TanStack Table case: "headless UI for
building powerful tables & datagrids … while retaining 100% control over markup
and styles" ([TanStack Table]). A first-party `@tanstack/lit-table` exists
(Lit 3). The catch is **version**: the stable line is **v8** (`8.21.3`, ~15.7 KB
core measured, last functional release on the v8 series ~a year old); **v9** is
in active alpha (`9.0.0-alpha.50`, 2026-05-21) and is a ground-up refactor where
"features are … tree-shakeable and treated as plugins — import only what you
use … ~6–7kb compared to 15–20kb for the same table in v8" ([TanStack Table V9
RFC]). For an offline bundle, the v9 size story matters. **Recommendation:
don't build a grid until it's actually needed, and when it is, target the v9
plugin line** (and re-check the `lit-table` v9 adapter's status at that point —
the Lit adapter must have caught up to v9, since today's published `lit-table`
tracks v8). Until then, simple read-only tables can be rendered directly with
Lit templates; reach for Table-core only when sorting/filtering/pagination/
column-resize is genuinely required.

### 3.4 Query — optional, smaller payoff for in-process commands

This is the most nuanced call. TanStack Query manages *server-state*: cache,
dedup, background refetch, retry, invalidation, stale-while-revalidate. aphrody's
"server" is usually an **in-process FFI call** (`runCaptured` over bun:ffi) or a
Tauri `invoke` — not a network round-trip. That removes some of Query's headline
value (there's no network latency to cache around). **But not all of it**: many
aphrody commands *are* async, slow, and re-runnable — `doctor`, `scan`, status
polling, listings, anything the UI shows repeatedly — and for those, Query's
caching ("don't re-run `doctor` on every tab focus"), `staleTime`,
de-duplication, retry-with-backoff, and `invalidateQueries` (re-run after a
mutating command) are real ergonomics wins over the console's bespoke
`fetch`→`try/catch`→`finally{disabled=false}` block. The core is genuinely
vanilla: `new QueryClient()` + `new QueryObserver(client, opts)` +
`observer.subscribe(result => …)` ([QueryObserver]); the first-party
`@tanstack/lit-query` (0.2.6, MIT, `lit >=2.8.0 <4`) wraps that as a
`QueryController` for `LitElement`. Measured incremental ~15.9 KB gz (adapter +
core, Lit external) — the heaviest of the "adopt-able" set. **Verdict: adopt
selectively**, only for the async/cacheable/refetchable commands, not as a
blanket wrapper around every `runCaptured`. For one-shot fire-and-render
commands, a thin hand-written async wrapper (what the console has) is lighter
and sufficient. Streaming (`agy-loop`, chat) is **not** Query's job — that's a
`Channel`/`EventSource` feeding a Store + Virtual list.

### 3.5 Form — defer until forms get heavy

Config screens and a command-builder are real aphrody surfaces, and
`@tanstack/lit-form` (1.24.1, `TanStackFormController`) is a clean first-party
fit: "headless, performant, type-safe form state … for TS/JS … and Lit"
([TanStack Form]). But today's form is a single `<input>` + submit
(`app.ts` lines 96–101). Form-core is ~13.4 KB gz and pulls `pacer-lite`. For
small forms, Material Web's form controls + a few lines of validation are
lighter. **Adopt when** a form grows multi-field with cross-field validation,
async validation, field arrays, or dirty/submit lifecycle — i.e. a real
`aphrody` config editor — not before.

### 3.6 Pacer — optional, tiny, per-primitive

Pacer is "a framework-agnostic, purpose-built library … to control async event
timing without the complexity of reactive programming patterns" (debounce,
throttle, rate-limit, queue, batch — [TanStack Pacer]). aphrody has natural
uses: debounce a live filter box over a virtualized list, throttle
stream-render flushes so a fast `agy-loop` doesn't repaint per token, rate-limit
re-runs. There is **no Lit adapter and none is needed** — the core is plain
functions/classes usable anywhere. Import **per-primitive** (a lone `debounce`
is a fraction of the ~9.2 KB whole-barrel). It's a "nice to have utility," not a
foundation; adopt opportunistically. (A 20-line `debounce`/`throttle` is also a
fine DOM-native alternative if pulling the dep isn't warranted.)

### 3.7 Router — reject for the Tauri/console shell

TanStack Router is excellent, but its adapters are **React and Solid only** —
there is **no Lit or vanilla adapter** — which is disqualifying for aphrody's
vanilla/Lit shell per the project's own rule ([ui-framework.md]: pick the layer
that "adds the least between those custom elements and the webview"). It is also
heavy relative to the need: a Tauri tool UI has a handful of views (run,
output, config, maybe RE/forensics tabs), which Lit's `@lit-labs/router` (the
in-lane choice from [ui-framework.md] §4.1) or even a tiny hash/switch handles
without a routing framework. Adopting Router would also drag in React/Solid,
contradicting the shell decision. **Reject.**

### 3.8 Start — reject (Tauri forbids SSR)

TanStack Start is a "full-stack React framework … full-document SSR, streaming,
server functions, bundling" ([TanStack Start]), built on Router + Vite, with a
Node/server runtime. Tauri serves a **static** `dist/` in a webview with **no
SSR** ([ui-framework.md] §0, [bun-vs-vite-2026.md]), aphrody's backend is the
Rust CLI (not a TS server), and the toolchain is Bun (not Vite). Start is the
precise category Tauri rules out and adds a server aphrody doesn't have.
**Reject** — same reasoning that declines Next/Nuxt/SvelteKit-SSR.

### 3.9 DB — reject (no problem to solve, no Lit adapter, heavy)

TanStack DB is "a reactive, client-first store … collections, live queries and
optimistic mutations … powered by differential dataflow" ([TanStack DB]),
designed to sit *in front of TanStack Query* for sync-heavy apps with a
server-backed dataset. aphrody has **no synced collection model** — it runs CLI
commands and shows their output; there is no client database to keep consistent
with a backend. There is **no first-party Lit adapter** (React/Solid/Vue-first),
and the differential-dataflow engine is a large payload against the offline
budget. It is a powerful tool aimed at a problem aphrody does not have.
**Reject.** (If a future surface maintained a large, queryable, locally-synced
dataset — e.g. a persistent index of scanned artifacts — revisit; not now.)

### 3.10 Config — reject (not an application dependency)

`@tanstack/config` is the build/publish tooling TanStack uses to ship its own
packages (Vite/Rollup/Vitest/publint presets). It has nothing to do with
building an aphrody UI under Bun. **Reject** (mentioned only to close the list).

---

## 4. Pitfalls and things that would have killed adoption (checked, cleared)

The brief flagged the classic risk: **most TanStack adapters are React / Solid /
Vue / Svelte / Angular, and a missing vanilla/Lit path is fatal for our shell.**
This was checked per-library, not assumed:

- **First-party Lit adapters DO exist** for the libraries aphrody needs:
  `@tanstack/lit-query` (0.2.6), `@tanstack/lit-table` (8.21.3),
  `@tanstack/lit-virtual` (3.13.26), `@tanstack/lit-form` (1.24.1),
  `@tanstack/lit-store` (0.13.2) — all MIT, all peering `lit >=2.8.0 <4`. They
  use Lit's **ReactiveController** pattern (`QueryController`,
  `TanStackFormController`, the virtualizer controller), which is the idiomatic
  way to bind external state to a `LitElement` and composes cleanly with
  Material Web. This is what flips TanStack from "React ecosystem" to "usable by
  aphrody."
- **Cores are framework-agnostic and dependency-free** (`query-core`,
  `table-core`, `virtual-core`, `store` declare `{}` dependencies), so even
  without an adapter they're drivable from vanilla via `subscribe()`
  (`QueryObserver.subscribe`, `store.subscribe`, the `Virtualizer` API). Store
  and Pacer are explicitly documented for vanilla use.
- **Router and Start have NO Lit/vanilla adapter** — Router is React/Solid-only,
  Start is an SSR server framework. These two *are* red-flagged and rejected for
  exactly this reason; do not try to shoehorn them into the webview.
- **`lit-table` tracks v8 today, but Table is mid-migration to v9.** If/when
  aphrody builds a grid, confirm the `lit-table` **v9** adapter is published and
  stable before committing — adopting v8 now risks a migration, and v9's
  tree-shakeable plugin model is the size win that justifies Table at all in an
  offline bundle.
- **`lit-query` is pre-1.0 (0.2.x).** It works and is first-party, but it's the
  youngest adapter; treat its API as potentially-moving and pin it. The
  `query-core` underneath is mature (v5).
- **License**: every TanStack package inspected is **MIT** — clean under
  aphrody's Apache-2.0 / zero-GPL policy ([CLAUDE.md] §0.1). No GPL/LGPL anywhere
  in the relevant trees. (MIT → Apache-2.0 is a standard permissive-into-
  permissive combination; no contamination.)
- **No double Lit, no version skew**: the adapters' `lit >=2.8.0 <4` peer range
  overlaps Material Web's `lit ^2.8.0 || ^3.0.0` ([ui-framework.md] §1), so Bun
  dedupes to one Lit. Verified at the manifest level.
- **Bundle discipline**: the offline Tauri budget is the constraint
  ([ui-framework.md] §0, latency-minimal objective). Adopt **cores per-feature**
  and let Bun tree-shake; do **not** import barrels wholesale. Avoid the devtools
  packages in production builds (`@tanstack/*-devtools`) — they're the heavy,
  dev-only siblings.
- **Streaming is not Query**: resist wrapping `agy-loop`/chat streams in Query.
  Streams are a `Channel`/`EventSource` → Store → Virtual list; Query is for
  discrete async results.

---

## 5. Ranked recommendation and the first concrete integration

### Tier 1 — adopt now (highest value, lowest cost)

1. **TanStack Virtual** (`@tanstack/lit-virtual` + `@tanstack/virtual-core`) —
   virtualize the output pane(s): streamed `agy-loop`/chat lines, long
   `scan`/`re strings` dumps, file/forensic listings. ~6.5 KB gz incremental,
   zero deps, no DOM-native equivalent worth hand-rolling. **Biggest UX win on
   the weakest engine (WebKitGTK, Linux #1).**
2. **TanStack Store** (`@tanstack/store`, core) — replace the console's ad-hoc
   imperative state (button-disabled, output children, version) with
   `Store`/`Derived`/`Effect` signals driving Lit `requestUpdate()`. ~2.2 KB gz,
   zero deps, framework-agnostic.

### Tier 2 — adopt when the surface appears

3. **TanStack Table** (`@tanstack/lit-table`, **v9 line**) — when a real
   sortable/filterable grid for `doctor --json` / `scan` / forensic rows is
   built. Wait for v9 (~6–7 KB, plugin/tree-shakeable) + its Lit adapter to
   stabilise; render trivial tables with plain Lit until then.
4. **TanStack Query** (`@tanstack/lit-query`) — *selectively*, only for async,
   cacheable, refetchable commands (status/`doctor`/listings), not every
   `runCaptured`. ~15.9 KB gz incremental.
5. **TanStack Form** (`@tanstack/lit-form`) — when a config/command-builder form
   becomes multi-field and validation-heavy.
6. **TanStack Pacer** (core, per-primitive) — debounce filters, throttle stream
   flushes, as needed. Tiny.

### Tier 3 — never (for this shell)

7. **TanStack Router** — React/Solid-only adapters; use `@lit-labs/router` or a
   trivial switch.
8. **TanStack Start** — SSR server framework; Tauri = static, no SSR.
9. **TanStack DB** — heavy synced-collections engine; no problem to solve here,
   no Lit adapter.
10. **TanStack Config** — TanStack's own build tooling; irrelevant.

### First integration (concrete, in `apps/console`, then reused by Tauri)

Introduce a single **`<aphrody-output>` Lit element** that owns command output,
combining Tier-1 libs:

1. A module-level **`@tanstack/store`** holds run state:
   `Store<{ status: 'idle'|'running'|'done'|'error'; code: number|null; lines:
   string[] }>`, plus a `Derived` for the status badge text. The form's submit
   handler sets `status='running'`; the bun:ffi / Tauri `Channel` callback
   appends to `lines`; the button's `disabled` is a `Derived` of `status`.
   (Deletes the manual `runButton.disabled` / `replaceChildren` wiring in
   `app.ts`.)
2. `<aphrody-output>` renders the `lines` array through a
   **`@tanstack/lit-virtual`** virtualizer (`VirtualizerController`), so a
   10k-line `scan` or a fast `agy-loop` stream stays at 60 fps on WebKitGTK with
   bounded DOM nodes.
3. Optionally throttle the per-line append with one **`@tanstack/pacer`**
   `throttle` so a token-fast stream flushes at a fixed cadence instead of per
   chunk.

This lands the two highest-value libraries on the surface that needs them most
(streaming + large output), proves the Lit-adapter/Material-Web coexistence,
stays well under the offline budget (~9 KB gz combined incremental, tree-shaken
lower), and is reused verbatim by the Tauri `apps/desktop-ui` (same Lit element,
swap `fetch`/bun:ffi for `invoke`/`Channel`). Tables/Query/Form get layered on
later, per surface, on the same Store substrate.

---

## 6. Sources

In-repo (read-only, verified 2026-05-24): [`docs/tauri/ui-framework.md`](../tauri/ui-framework.md)
(UI decision: vanilla TS + Lit/Material Web, offline Tauri, no SSR, bundle
budget); [`docs/research/bun-vs-vite-2026.md`](bun-vs-vite-2026.md) (Bun is the
bundler); [`docs/research/gui-options-2026.md`](gui-options-2026.md) (Apache-2.0
/ zero-GPL policy, FFI bridge, M3 fusion); [`CLAUDE.md`](../../CLAUDE.md) §0.1,
§2 (autonomy, polyglot policy); the live console source
`C:\src\aphrody-ts\apps\console\src\app.ts` (ad-hoc state to be replaced) and
`apps/console/package.json` (Apache-2.0, Bun, no UI framework).

Local measurements (Bun 1.3.14 `bun build --minify --target=browser` + `gzip -9`,
this host, 2026-05-24; tarballs via `npm pack`; gitignored `var/tanstack-probe/`):
`query-core` 5.100.14 ~12.4 KB; `table-core` 8.21.3 ~15.7 KB; `virtual-core`
3.15.0 ~6.4 KB; `store` 0.11.0 ~2.2 KB; `form-core` 1.32.0 ~13.4 KB; `pacer`
0.21.1 ~9.2 KB; `lit-virtual` 3.13.26 (+core, Lit ext) ~6.5 KB; `lit-query`
0.2.6 (+core, Lit ext) ~15.9 KB. Versions/licenses via `npm view` (all **MIT**):
`query-core` 5.100.14, `table-core` 8.21.3 (+ `9.0.0-alpha.50`), `virtual-core`
3.15.0, `store` 0.11.0, `form-core` 1.32.0, `pacer` 0.21.1, `lit-query` 0.2.6,
`lit-table` 8.21.3, `lit-virtual` 3.13.26, `lit-form` 1.24.1, `lit-store`
0.13.2, `db` 0.6.7, `config` 0.22.2. Dependency trees (`{}` for the four cores)
read from each package's `package.json`.

External (accessed 2026-05-24):

- TanStack libraries index, MIT licensing — [TanStack Libraries]
  (https://tanstack.com/libraries); LICENSE files e.g.
  https://github.com/TanStack/query/blob/main/LICENSE,
  https://github.com/TanStack/table/blob/main/LICENSE.
- TanStack Query: framework-agnostic `query-core`, Lit adapter shipped —
  [TanStack Query] (https://tanstack.com/query/latest);
  [query-core npm] (https://www.npmjs.com/package/@tanstack/query-core);
  [QueryObserver] (https://tanstack.com/query/latest/docs/reference/QueryObserver)
  (`observer.subscribe(result => …)` vanilla usage);
  [Lit TanStack query adapter] (https://github.com/TanStack/query/discussions/6390).
- TanStack Table: core-plus-adapter, Lit adapter, v9 alpha tree-shakeable
  (~6–7 KB vs v8 ~15–20 KB) — [TanStack Table] (https://tanstack.com/table/latest);
  [Vanilla TS/JS] (https://tanstack.com/table/latest/docs/vanilla);
  [Lit Table] (https://tanstack.com/table/v8/docs/framework/lit/lit-table);
  [@tanstack/lit-table npm] (https://www.npmjs.com/package/@tanstack/lit-table);
  [TanStack Table V9 RFC] (https://github.com/TanStack/table/discussions/5834);
  [V9 alpha release] (https://newreleases.io/project/github/TanStack/table/release/v9.0.0-alpha.34).
- TanStack Virtual: FW-agnostic `virtual-core` + Lit adapter, 60 fps, full
  markup control — [TanStack Virtual] (https://tanstack.com/virtual/latest);
  [Installation] (https://tanstack.com/virtual/latest/docs/installation);
  [@tanstack/lit-virtual npm] (https://www.npmjs.com/package/@tanstack/lit-virtual);
  [Architecture Overview] (https://deepwiki.com/TanStack/virtual/1.2-getting-started).
- TanStack Store: framework-agnostic signals, vanilla usage, `Store`/`Derived`/
  `Effect` — [TanStack Store] (https://tanstack.com/store/latest);
  [Quick Start] (https://tanstack.com/store/latest/docs/quick-start);
  [store repo] (https://github.com/tanstack/store).
- TanStack Form: headless FW-agnostic core, Lit (`TanStackFormController`) —
  [TanStack Form] (https://tanstack.com/form/latest);
  [form repo] (https://github.com/TanStack/form);
  [form-core npm] (https://www.npmjs.com/package/@tanstack/form-core).
- TanStack Pacer: framework-agnostic timing primitives, vanilla core —
  [TanStack Pacer] (https://tanstack.com/pacer/latest);
  [Pacer Overview] (https://tanstack.com/pacer/latest/docs/overview).
- TanStack Router / Start: React/Solid adapters; Start = SSR server framework —
  [TanStack Start Overview] (https://tanstack.com/start/latest/docs/framework/react/overview);
  [TanStack Start: New Meta Framework (React or SolidJS)] (https://www.infoq.com/news/2025/11/tanstack-start-v1/);
  [Router SSR] (https://tanstack.com/router/latest/docs/guide/ssr).
- TanStack DB: client-first reactive DB, differential dataflow, sits in front of
  Query — [TanStack DB] (https://tanstack.com/db/latest);
  [DB Overview] (https://tanstack.com/db/latest/docs/overview);
  [TanStack DB 0.1 blog] (https://tanstack.com/blog/tanstack-db-0.1-the-embedded-client-database-for-tanstack-query).

[CLAUDE.md]: ../../CLAUDE.md
[ui-framework.md]: ../tauri/ui-framework.md
[bun-vs-vite-2026.md]: bun-vs-vite-2026.md
[gui-options-2026.md]: gui-options-2026.md
[TanStack Libraries]: https://tanstack.com/libraries
[TanStack Query]: https://tanstack.com/query/latest
[QueryObserver]: https://tanstack.com/query/latest/docs/reference/QueryObserver
[TanStack Table]: https://tanstack.com/table/latest
[TanStack Table V9 RFC]: https://github.com/TanStack/table/discussions/5834
[TanStack Virtual]: https://tanstack.com/virtual/latest
[TanStack Store]: https://tanstack.com/store/latest
[TanStack Form]: https://tanstack.com/form/latest
[TanStack Pacer]: https://tanstack.com/pacer/latest
[TanStack Start]: https://tanstack.com/start/latest/docs/framework/react/overview
[TanStack DB]: https://tanstack.com/db/latest
