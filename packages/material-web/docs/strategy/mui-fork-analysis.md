<!-- SPDX-License-Identifier: Apache-2.0 -->

# Should we fork MUI / MUI X into a bun-native, full-Material-3 system?

> Strategic analysis. Read-only research, no code changes. Surveys the MUI GitHub
> org (state at 2026-05-29), weighs a full fork against the current "native Lit M3
>
> - React wrappers + migration codemods" strategy, and proposes a concrete plan.

---

## 1. Executive summary — verdict

**DO NOT FORK the MUI / MUI X component or styling layer. FORK SELECTIVELY (or
rather, BORROW) a small set of low-level, framework-agnostic, MIT-licensed
algorithms only. Keep the current "tune / wrap + migrate" strategy as the
backbone.**

Three findings drive this:

1. **The architecture is the wrong shape to fork.** MUI's value is welded to an
   **Emotion CSS-in-JS runtime** (`@mui/styled-engine` hard-depends on
   `@emotion/cache`, `@emotion/serialize`, `@emotion/sheet`) and to the
   `sx` / `createTheme` / `ThemeProvider` design language. Our system is the
   opposite by design: **semantic color roles emitted as `--md-sys-*` runtime CSS
   custom properties, consumed inside Lit Shadow DOM, no per-render JS style
   injection.** Forking MUI's component layer means either dragging Emotion in
   (which fights M3's token model and our bun-native, zero-runtime-CSS-in-JS
   philosophy) or rewriting every component's styling — at which point it is no
   longer a fork, it is a rewrite that happens to start from someone else's tree.

2. **A large, legally clean fork is not even possible.** MUI X is **dual-licensed
   per package**. The community packages are MIT, but `x-data-grid-pro`,
   `x-data-grid-premium`, `x-charts-pro`, `x-charts-premium`,
   `x-tree-view-pro`, `x-scheduler-premium`, and `x-license` ship under a
   **commercial license** (`"license": "SEE LICENSE IN LICENSE"`). Those cannot
   be relicensed, forked, or shipped in a free product. The Premium features the
   README already declares out of scope (row grouping, pivot, Excel export,
   recurrence/DnD scheduler) are exactly the commercial code — so a "full" fork
   was never on the table.

3. **The cost/benefit is upside-down.** A fork creates a permanent obligation to
   track a ~740 MB, very actively developed React monorepo, only to bend it
   toward a design system it was not built for. The current approach already
   delivers the consumer-facing win (a migration path off MUI) at a fraction of
   the maintenance surface, with cleaner M3 conformance and licensing.

The one defensible move is to **borrow specific MIT, framework-agnostic
algorithms** (data-grid sort/filter/pagination logic, virtualization math,
charts scales/geometry, date arithmetic, a11y interaction patterns from Base UI)
as _reference and seed code_ to power our own `md-*` components — with attribution
— never to vendor MUI's styled component layer.

---

## 2. Repo inventory (MUI GitHub org, observed 2026-05-29)

`gh repo list mui` + `gh api` on package manifests. Stars rounded.

| Repo                                            | What it is                                                                                                                                                | Lang       | License                 | Repo size | Health                                           | Fork-relevant?                                          |
| ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ----------------------- | --------- | ------------------------------------------------ | ------------------------------------------------------- |
| `material-ui`                                   | Monorepo: `@mui/material`, `@mui/system`, `@mui/styled-engine` (Emotion), `@mui/utils`, `@mui/lab`, `@mui/icons-material`, Joy UI, `@mui/material-nextjs` | JavaScript | **MIT**                 | ~742 MB   | Very active (98k★)                               | Component layer: **no**. Icons/codemod: reference only  |
| `mui-x`                                         | Monorepo: Data Grid, Charts, Date/Time Pickers, Tree View, Scheduler — **community + Pro + Premium**                                                      | TypeScript | **Mixed** (per-package) | ~184 MB   | Very active (5.7k★)                              | **Community packages only**, and only as algorithm seed |
| `base-ui`                                       | Unstyled, accessible headless primitives (from Radix/Floating UI/MUI authors)                                                                             | TypeScript | **MIT**                 | ~40 MB    | Active (9.8k★)                                   | **Yes** — a11y behavior reference                       |
| `pigment-css`                                   | Zero-runtime CSS-in-JS (build-time extraction)                                                                                                            | TypeScript | **MIT**                 | —         | Alpha, slow                                      | No (we have no CSS-in-JS need)                          |
| `toolpad`                                       | Low-code dashboard builder                                                                                                                                | TypeScript | **MIT**                 | —         | **Not actively maintained** (per its own README) | No                                                      |
| `material-ui-docs`                              | Docs mirror (read-only, PRs closed)                                                                                                                       | JavaScript | MIT                     | —         | Mirror                                           | No                                                      |
| `material-ui-pickers`                           | Legacy v1–v4 pickers                                                                                                                                      | TypeScript | MIT                     | —         | **Archived**                                     | No                                                      |
| `mui-public`                                    | Org tooling / infra monorepo                                                                                                                              | TypeScript | MIT                     | —         | Active (infra)                                   | No                                                      |
| `mui-design-kits`                               | Figma kit tracker                                                                                                                                         | —          | —                       | —         | Tracker                                          | No                                                      |
| `tech-challenge-*`, `workshop-*`, `hackathon-*` | Hiring / workshop repos                                                                                                                                   | mixed      | mixed/none              | —         | N/A                                              | No                                                      |
| `mui-fork-argos-1/2`                            | Their forks of the Argos visual-test tool                                                                                                                 | TypeScript | MIT                     | —         | CI infra                                         | No                                                      |

### 2.1 MUI X package license split (the load-bearing legal fact)

`gh api` on each `packages/<pkg>/package.json`:

| Package                             | License field              | Free to fork?       |
| ----------------------------------- | -------------------------- | ------------------- |
| `x-data-grid`                       | `"MIT"`                    | **Yes**             |
| `x-charts`                          | `"MIT"`                    | **Yes**             |
| `x-date-pickers`                    | `"MIT"`                    | **Yes**             |
| `x-tree-view`                       | `"MIT"`                    | **Yes**             |
| `x-scheduler`                       | `"MIT"`                    | **Yes**             |
| `x-virtualizer`                     | `"MIT"`                    | **Yes**             |
| `x-internals`                       | `"MIT"`                    | **Yes**             |
| `x-data-grid-pro`                   | `"SEE LICENSE IN LICENSE"` | **No — commercial** |
| `x-data-grid-premium`               | `"SEE LICENSE IN LICENSE"` | **No — commercial** |
| `x-charts-pro` / `x-charts-premium` | commercial                 | **No**              |
| `x-tree-view-pro`                   | commercial                 | **No**              |
| `x-scheduler-premium`               | commercial                 | **No**              |
| `x-license`                         | commercial                 | **No**              |

The MUI X repo root has **no single license** (`license: null` on the repo) —
precisely because it is a mixed tree. Any fork must surgically exclude every
`*-pro` / `*-premium` / `x-license` package, or it is a license violation.

### 2.2 The Emotion dependency (the architectural fact)

`@mui/material` → `@mui/system` → `@mui/styled-engine`, and
`@mui/styled-engine` directly declares:

```
"@emotion/cache", "@emotion/serialize", "@emotion/sheet"
```

So MUI components are inseparable from a runtime CSS-in-JS engine that injects
`<style>` tags per render into the light DOM. That is the exact opposite of our
model (compile-time Sass → `--md-sys-*` custom properties → Shadow DOM, zero
per-render style injection). This is not a detail; it is the reason a component
fork degenerates into a rewrite.

---

## 3. Option A — FORK ALL (full bun-native, full-M3 fork of MUI + MUI X)

**What it would actually require:**

- Vendor `material-ui` (~742 MB) and the MIT subset of `mui-x` into the monorepo,
  excluding all Pro/Premium packages and `x-license` (legal hard stop).
- Replace MUI's build (pnpm + Babel macros + codegen + Emotion runtime) with a
  bun-native pipeline — i.e. rip out Babel/Emotion and re-plumb every package.
- Rip out Emotion `styled`/`sx` and re-skin every component to `--md-sys-*` tokens
  and M3 shape/elevation/state-layer semantics — Material 2 → Material 3 is a
  spec change, not a re-theme.
- Re-test the whole React surface, then **track upstream forever**: MUI ships
  multiple releases a month; every merge re-introduces Emotion and M2 patterns we
  just removed.

**Verdict: reject.** Reasons:

- **Licensing:** the most valuable X features are commercial and cannot be
  forked into a free product. A "full" fork is legally impossible by definition.
- **Impedance mismatch:** Emotion `sx`/`theme` vs `--md-sys-*` Shadow-DOM tokens
  is a fundamental design-language conflict. The real-world migration wall is
  already `sx`/Emotion styling (see `migration/10-case-study-rpbey.md`) — forking
  would internalize that wall instead of leaving it behind.
- **Scale & maintenance:** absorbing a ~740 MB monorepo that releases constantly,
  to permanently fight its core abstraction, is an unbounded maintenance liability
  for a small team. It violates the repo's own "no dead weight / measured" ethos
  (`docs/STACK.md`).
- **Bun-native:** MUI's Babel-macro + Emotion build is one of the harder things to
  make bun-native; we would inherit that complexity rather than shed it.

---

## 4. Option B — TUNE / WRAP + MIGRATE (the current approach)

Native Lit `md-*` web components self-contained on `--md-sys-*`, React wrappers
(`@aphrody/m3-react`), MUI→M3 codemod kit (`migration/`), consumer lint
plugin (`eslint-plugin-m3`).

| Axis                              | Assessment                                                                                                                                                                                                                                                      |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **M3 spec correctness**           | **High.** Components are written to M3 (HCT color roles, shape scale, state layers, motion tokens) from the start — no Material-2 substrate to fight.                                                                                                           |
| **Maintenance**                   | **Bounded.** We own a focused `md-*` set; no obligation to track a foreign monorepo. Upstream `@material/web` is in maintenance mode, so the fork is _the_ live M3 web implementation — a moat, not a liability.                                                |
| **Bundle size**                   | **Smaller.** No Emotion runtime; deep imports per family (~280 B gzip vs MUI+Emotion baseline). Token CSS is static.                                                                                                                                            |
| **Licensing cleanliness**         | **Clean.** Apache-2.0 / our own code; no commercial-license entanglement, no Emotion (MIT but a runtime dependency we don't want).                                                                                                                              |
| **Migration value for consumers** | **This is the actual product.** Consumers get _off_ MUI: codemods (`transforms/orchestrator.ts` + `icons.ts`, 96% icon auto-map vs 4253-glyph table), `mui-m3-map.json`, the lint plugin, and the rpbey case study quantifying the real effort (the `sx` wall). |
| **Effort**                        | **Already largely done** and incremental: add a component → 3-point wiring → regenerate wrappers.                                                                                                                                                               |

**Verdict: keep as the backbone.** It is the only option that is simultaneously
M3-correct, license-clean, bun-native, and shippable by a small team.

---

## 5. Option C — FORK SELECTIVELY (the recommended nuance)

Do **not** fork MUI's styled component layer. **Do** treat a few MIT,
framework-agnostic, logic-only pieces as _seed/reference_ to harden our own
`md-*` components where the hard part is an algorithm, not the markup or styling.

The discriminator: **borrow logic that has no opinion about styling or framework;
never borrow anything that ships JSX + Emotion.**

### 5.1 Worth borrowing / forking selectively (all MIT)

| Source (MIT)                                | What to extract                                                                                                         | Powers which `md-*`                      | Why it's safe to borrow                                                                                                                                                                                          |
| ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mui-x/packages/x-data-grid`                | Sort comparators, multi-column sort precedence, filter operator semantics, pagination model, column resize/reorder math | `md-table`                               | Pure data transforms; no DOM/Emotion. Re-implement to taste, attribute MIT.                                                                                                                                      |
| `mui-x/packages/x-virtualizer`              | Windowing math (visible range from scrollTop + row heights, overscan)                                                   | `md-table` large datasets                | Framework-agnostic geometry.                                                                                                                                                                                     |
| `mui-x/packages/x-charts`                   | Scale/axis math, data→geometry mapping, tick generation, stacking                                                       | the 8 `md-*-chart`                       | Math, not rendering; we draw with our own SVG/tokens.                                                                                                                                                            |
| `mui-x/packages/x-date-pickers` (community) | Date arithmetic, range/validation logic, locale/`Intl` adapters                                                         | `md-date-picker` family                  | Logic only; rendering stays M3.                                                                                                                                                                                  |
| `mui-x/packages/x-internals`                | Small shared utilities used by the community packages                                                                   | shared                                   | MIT helpers; cherry-pick, don't vendor wholesale.                                                                                                                                                                |
| `base-ui`                                   | a11y interaction _patterns_ (focus management, roving tabindex, dismissable layers, anchor positioning conventions)     | overlays, menus, listbox-like components | MIT, explicitly "unstyled"; read as a reference for ARIA/keyboard behavior. Prefer native platform APIs (`<dialog>`, Popover API, CSS Anchor Positioning) where they exist (see `migration/05-gap-analysis.md`). |

How to borrow: copy/adapt the _function_, not the package; rewrite it in our TS
style; add an SPDX/attribution note crediting MUI (MIT requires preserving the
copyright + permission notice). This is "selective fork" in the only sense that
makes sense here — vendoring algorithms, not components.

### 5.2 Do NOT borrow / fork

| Source                                  | Why not                                                                                                                                                                              |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `@mui/material` component layer         | JSX + Emotion `styled`/`sx`; Material 2 semantics; would need full re-skin = rewrite.                                                                                                |
| `@mui/system` / `@mui/styled-engine`    | The Emotion runtime itself — directly opposed to `--md-sys-*` + Shadow DOM.                                                                                                          |
| `pigment-css`                           | A CSS-in-JS solution to a problem we don't have (we use Sass→tokens). Alpha, slow.                                                                                                   |
| Joy UI (`@mui/joy`)                     | A _different_ design system; irrelevant to M3.                                                                                                                                       |
| Any `*-pro` / `*-premium` / `x-license` | **Commercial license. Hard stop.** Out of scope per the README anyway.                                                                                                               |
| `toolpad`                               | Not maintained; out of scope.                                                                                                                                                        |
| `@mui/icons-material` (as a dependency) | We already map `@mui/icons-material` → Material Symbols via the icon codemod (96%, validated against 4253 glyphs). Use it as a _naming reference_ for the codemod, not a dependency. |

---

## 6. Legal / licensing constraints (explicit)

- **MIT (material-ui, base-ui, pigment-css, MUI X community packages):** free to
  fork/borrow **with attribution** — preserve the copyright notice and the MIT
  permission text in any file derived from theirs. Our repo is Apache-2.0;
  MIT-into-Apache-2.0 is compatible (keep the MIT notice for the borrowed parts).
- **Commercial (`x-*-pro`, `x-*-premium`, `x-license`):** **do not fork, vendor,
  copy, or relicense.** These require a paid MUI X license and explicitly forbid
  redistribution. They map 1:1 to the "MUI X Premium = out of scope" line already
  in the README. Treat them as off-limits at the algorithm level too — do not
  reverse-engineer Premium-only behaviors (row grouping, pivot, Excel export,
  recurrence/DnD scheduler) from their source.
- **Emotion:** MIT, so not a _legal_ problem — but a _runtime/architecture_ one
  we are choosing to avoid.
- **Trademark:** "MUI", "Material UI", "MUI X" are MUI's marks; "Material",
  "Material Design" are Google's. Don't name forks in a way that implies
  endorsement. Our `@aphrody/*` + `@material/web` (the existing Google fork)
  naming is already clear of this.

---

## 7. Recommended next-step plan (fits the bun / M3 philosophy)

1. **Codify the decision.** Land this doc; add a one-line "no MUI component/styled
   fork; algorithm-only borrowing under MIT attribution" note to `CLAUDE.md`
   coverage section so future work doesn't re-litigate it.
2. **Keep investing where the product actually is — migration.** The measured
   wall is `sx`/Emotion (rpbey case study), not component coverage. Prioritize:
   an `sx`-prop → Tailwind/`--md-sys-*` codemod (the missing piece in
   `transforms/orchestrator.ts`), and broaden `mui-m3-map.json`.
3. **Harden `md-table` and charts with borrowed MIT logic.** Where our
   data-grid/charts need more (sort precedence edge cases, virtualization at
   scale, axis/tick math), adapt the corresponding **community** `mui-x`
   algorithm into our own TS with an MIT attribution header — do not add a
   dependency on `@mui/x-*`.
4. **Use Base UI as an a11y oracle.** When building/auditing overlays, menus, and
   listbox-style components, cross-check ARIA/keyboard behavior against `base-ui`
   (MIT) but implement on native platform primitives (`<dialog>`, Popover API,
   CSS Anchor Positioning) per `migration/05-gap-analysis.md`.
5. **Pin the licensing guardrail in the lint plugin (optional).** Consider an
   `eslint-plugin-m3` rule (or a CI check) that fails on any import of
   `@mui/x-*-pro`/`-premium` in our own packages, so the commercial line can
   never be crossed by accident.
6. **Do not vendor either MUI monorepo.** No `material-ui/` or `mui-x/` tree in
   our repo. Borrowed algorithms live as our own files with attribution, keeping
   the bun-native, no-dead-weight invariant from `docs/STACK.md` intact.

---

## Sources

- MUI org repo list + per-package manifests: `gh repo list mui`,
  `gh api repos/mui/<repo>/contents/packages/...` (observed 2026-05-29).
- License split: `package.json` `"license"` fields per MUI X package (MIT vs
  `"SEE LICENSE IN LICENSE"`).
- Emotion coupling: `@mui/styled-engine` dependencies (`@emotion/cache`,
  `@emotion/serialize`, `@emotion/sheet`).
- In-repo context: `docs/STACK.md`, `README.md`, `CLAUDE.md`,
  `docs/06-landscape-recommendations.md`, `migration/05-gap-analysis.md`,
  `migration/10-case-study-rpbey.md`, `migration/mui-m3-map.json`.
