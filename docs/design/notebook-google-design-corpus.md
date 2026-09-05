<!-- SPDX-License-Identifier: Apache-2.0 -->
# NotebookLM corpus — "Interface de l'Application de Recherche Google"

Source inventory of the user's NotebookLM notebook
`d05f8728-e5de-430a-be03-0d830b4de348`, read 2026-05-22 from the live notebook
page. Only the **source metadata** (titles/categories) is recorded here — the
Deep Research Report bodies and third-party article texts are not reproduced
(copyright); their knowledge is distilled into the `google-design` skill and the
`docs/design/` package.

## What the notebook aggregates

- **3 Deep Research Reports** (the notebook's synthesis backbone):
  1. *Architectural Analysis of the Material 3 Design System — Evolutionary
     Foundations, Component Taxonomy, and Adaptive Systems.*
  2. *The Architecture of Material Design 3 — Systemic Foundations, Interactive
     Components, and the Expressive Evolution.*
  3. *The Sovereign Desktop and Spatial AI Ecosystem — Google's integration of
     Gemini, Material Expressive, and Jetpack Compose Glimmer.*
- **M3 component pages** (material.io): all-buttons, buttons, button-groups,
  app-bars (top/bottom), bottom-sheets, carousel, breakpoints, components index,
  and the rest of the M3 catalogue.
- **Foundations**: Accessibility (Material Design), breakpoints / adaptive.
- **Android / Compose**: *API defaults — Jetpack Compose* (Android Developers).
- **Ecosystem / adoption**: *Adopt Material Design 3 / Material You* (mui/
  material-ui issue #29345); mobile-UX best-practices articles.
- **Design philosophy**: David Reinfurt — *"the point is to get disoriented,
  not oriented"* (rethinking how design is taught).

## Mapping to the aphrody design package

| Notebook theme | aphrody artefact |
|---|---|
| M3 foundations / glossary | [`m3-foundations.md`](m3-foundations.md), [`m3-glossary.md`](m3-glossary.md) |
| M3 styles (color/type/shape/motion) | [`m3-styles.md`](m3-styles.md), `crates/m3-tokens` |
| M3 components + taxonomy | [`m3-components.md`](m3-components.md), [`m3-components-spec.md`](m3-components-spec.md), `crates/mui-rs-components` |
| Adaptive / breakpoints / layout | [`m3-layout.md`](m3-layout.md), `crates/m3-tokens/src/adaptive.rs` |
| Gemini / Expressive / Glimmer | [`gemini/`](gemini/), [`references/`](references/), `mui-rs-renderer::{gradient,text}` |
| Reinfurt philosophy | [`references/reinfurt-true-not-new.md`](references/reinfurt-true-not-new.md) |

This corpus is the reading list for the [`google-design`](../../plugins/aphrody/skills/google-design/SKILL.md)
skill and the `google-design-researcher` sub-agent.
