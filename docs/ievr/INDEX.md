<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody contributors
-->

# IEVR Documentation Sub-Index

## 1. Where to start

aphrody-side meta-documentation for the Inazuma Eleven Victory Road (IEVR)
reverse-engineering effort. The actual RE workspace lives in a separate repo
at `C:/src/ievr-re/`; this directory tracks strategy, format references,
anti-tamper safety notes, and community sources. To extract assets or run a
debugger, jump to `C:/src/ievr-re/` after reading `PLAN.md` and
`eac-considerations.md`.

## 2. Strategy + planning

- [`PLAN.md`](PLAN.md) — 7-phase workplan (P0 inventory through P6 ML similarity) with schedule
- [`legal-checklist.md`](legal-checklist.md) — DMCA 1201(f) + EU + Steam ToS checklist _in flight_
- [`community-sources.md`](community-sources.md) — existing IE modding tools + community resources _in flight_
- [`glossary.md`](glossary.md) — RE terminology reference _in flight_

## 3. Inventories (P0 outputs)

- [`binaries-inventory.md`](binaries-inventory.md) — Steam install + 975 files + 921 CPKs _in flight_
- [`re-toolchain.md`](re-toolchain.md) — RE tools available on the system _in flight_
- [`ml-env-audit.md`](ml-env-audit.md) — ML environment audit + refactor proposal

## 4. Format references

- [`cpk-format.md`](cpk-format.md) — CRI Middleware CPK archive container
- [`cri-toolchain.md`](cri-toolchain.md) — CPK + USM + ADX/HCA tooling
- [`usm-format.md`](usm-format.md) — CRI Sofdec video container _in flight_
- [`adx-hca-format.md`](adx-hca-format.md) — CRI audio formats (ADX, HCA, AWB) _in flight_
- [`asset-formats.md`](asset-formats.md) — general game asset format reference _in flight_
- [`level5-engine-notes.md`](level5-engine-notes.md) — Level-5 custom engine observations

## 5. Anti-tamper + safety

- [`eac-considerations.md`](eac-considerations.md) — EAC impact on RE; what is safe vs banned
- [`cri-known-keys.md`](cri-known-keys.md) — CRI encryption key reference _in flight_

## 6. Per-phase guidance

For each `PLAN.md` phase, the docs most worth re-reading:

- **P0 (inventory)** — `binaries-inventory.md`, `re-toolchain.md`, `ml-env-audit.md`, `asset-formats.md`
- **P1 (setup)** — `cri-toolchain.md`, `re-toolchain.md`
- **P2 (static analysis)** — `level5-engine-notes.md`, `cpk-format.md`, `eac-considerations.md`
- **P3 (asset extraction)** — `cri-toolchain.md`, `cpk-format.md`, `usm-format.md`, `adx-hca-format.md`
- **P4 (dynamic analysis)** — `eac-considerations.md` (CRITICAL caveats), `legal-checklist.md`
- **P5 (documentation)** — `community-sources.md`, `glossary.md`
- **P6 (ML similarity)** — `ml-env-audit.md`, `level5-engine-notes.md`

## 7. Related (outside docs/ievr/)

- [`../PROTOCOL.md`](../PROTOCOL.md) — aphrody A2A protocol (peer coord with winclean Claude)
- [`../posts/2026-05-ai-json.md`](../posts/2026-05-ai-json.md) — cross-Claude coordination background
- [`../INDEX.md`](../INDEX.md) — global aphrody documentation index

## 8. Active work

The peer Claude in `C:/winclean/` runs the heavy IEVR extraction pipeline:
binary inventory pass is complete; next work item is MlCore #14. The aphrody
side owns documentation, format specifications, and cross-references. All
coordination flows through the shared mailbox at
`C:/winclean/.coord/inbox-from-{aphrody,winclean}.jsonl`, with HTTP listener
on `:8788` exposing `/ping`, `/msg`, `/inbox`, `/ai.json`. Before pushing
changes that touch shared format specs, drain the inbox and emit a `fact`.
