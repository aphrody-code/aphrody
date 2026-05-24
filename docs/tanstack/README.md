<!-- SPDX-License-Identifier: Apache-2.0 -->
# TanStack in aphrody

How aphrody uses the TanStack ecosystem in its UI. The full evaluation — with
locally-measured bundle sizes and sources — is `docs/research/tanstack-for-aphrody.md`;
this is the actionable summary.

## Why it fits

aphrody's UI shell is Vanilla TS + Lit / Material Web (see
`docs/tauri/ui-framework.md`). TanStack is not React-only: every relevant
library is a framework-agnostic `*-core` with a first-party **Lit adapter**
(`lit-virtual`, `lit-store`, `lit-table`, `lit-query`, `lit-form`), all **MIT**
(zero GPL), peer-depending on `lit 2.8-3.x` — the same range Material Web uses,
so there is no second copy of Lit in the bundle.

## Adopt now

| Library | Package (gz) | aphrody use |
|---|---|---|
| **Virtual** | `@tanstack/lit-virtual` (~6.5 KB) | windowed rendering of long / streamed output — agy-loop, scan, RE / forensic listings, logs. The biggest UX win on WebKitGTK (Linux #1). |
| **Store** | `@tanstack/store` (~2.2 KB, 0 deps) | reactive run state for the vanilla shell, replacing the ad-hoc imperative DOM updates in `apps/console`. |

## Adopt when the surface appears

| Library | Notes |
|---|---|
| **Table** | `@tanstack/lit-table` on the v9 line (~6-7 KB, tree-shakeable; avoid v8 at ~15.7 KB) for sortable `doctor --json` / `scan` / forensic grids. |
| **Query** | `@tanstack/lit-query` (~15.9 KB) only for genuinely async / cacheable commands — its payoff shrinks because the "server" is an in-process FFI / Tauri `invoke` call, and it must NOT wrap streams. |
| **Form / Pacer** | opportunistically (command / config forms; stream throttling). |

## Reject for this shell

Router (React / Solid adapters only, no Lit), Start (an SSR server framework —
Tauri forbids SSR), DB (a heavy differential-dataflow client database, no Lit
adapter, solves a sync problem aphrody does not have), Config (TanStack's own
build tooling).

## First integration: `<aphrody-output>`

A single Lit custom element, used by `apps/console` today and the Tauri
`apps/desktop-ui` later, composing the adopt-now libraries:

- **Store** holds the run state (`{ status, code, lines }`), updated as a
  command streams.
- **lit-virtual** windows the line list, so a 100k-line output stays smooth.
- an optional **Pacer** throttle coalesces high-frequency stream chunks.

It consumes the transport-abstracted command client (Tauri `invoke` / Channels
on the desktop, `fetch /api/run` in the Bun-served web console).

## Setup

```sh
# in the aphrody-ts repo
bun add @tanstack/lit-virtual @tanstack/store
```

Import only the adapter you use; the cores have zero runtime dependencies, so
the cost is just the table above.

## See also

- `docs/research/tanstack-for-aphrody.md` — the full evaluation (per-library
  verdicts, measured sizes, pitfalls).
- `docs/tauri/ui-framework.md` — the Vanilla-TS + Lit / Material Web decision.
- `docs/plans/tauri-app.md` — the Tauri adoption plan (P2 frontend).
- the sibling `aphrody-ts` repo's `docs/design/UI-STACK.md` — the unified UI stack.
