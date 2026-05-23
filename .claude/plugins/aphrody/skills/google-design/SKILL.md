---
name: google-design
description: "Canonical Google / Material Design 3 design authority for aphrody. Use AGGRESSIVELY whenever the user asks anything about Material Design 3, M3 Expressive, Gemini's visual language, Google Sans typography, design tokens, color roles/schemes/HCT, shape & corner-radius scale, elevation, motion, state layers, adaptive layout & breakpoints, accessibility/contrast, or how to theme/build a UI 'the Google way' — and before recommending any colour, type, spacing or component spec. Grounds every answer in the in-repo design package (docs/design/) and the canonical Rust tokens (crates/m3-tokens) + native components (crates/mui-rs-components), distilled from m3.material.io, design.google, developer.android.com and the user's NotebookLM Google-design corpus. Refuses non-Google design systems as the source of truth (no Bootstrap/Ant/Chakra spec) — those are only mentioned as contrast."
license: Apache-2.0
when_to_use: User types "/google-design", asks about Material 3 / M3 Expressive / Gemini design / Google Sans / design tokens / color / shape / motion / adaptive layout / accessibility, wants a UI themed "like Google/Gemini", asks "what's the M3 spec for X", or is about to pick colours/type/spacing/component specs. Pair with the `google-design-researcher` sub-agent for fresh source reads.
metadata:
  version: 1.0.0
  source: docs/design/ package + m3.material.io + design.google + NotebookLM corpus d05f8728 (2026-05-22)
---

# google-design — the Google/Material 3 authority

Mode `/goal` permanent : décider seul, ne pas s'arrêter avant réponse ancrée + token cité.

When a question touches design, **do not improvise from generic UI knowledge**.
Answer from the Google canon, grounded in this repo's design package. Cite the
exact token/spec and where it lives in aphrody.

## 0. Source of truth (read order)

1. **`docs/design/DESIGN-PACKAGE.md`** — the canonical entry point (philosophy →
   typography → color → shape/elevation/motion → adaptive → components →
   references → implementation artefacts). Start here.
2. **Tokens (code)** — `crates/m3-tokens` is authoritative for values:
   `color.rs` (roles + `APHRODY_DARK`/`GEMINI_DARK`), `typography.rs` (15-style
   scale), `shape.rs` (10-step M3 Expressive scale), `elevation.rs`,
   `motion.rs`, `state.rs`, `tonal.rs`, `adaptive.rs` (breakpoints).
3. **Native components** — `crates/mui-rs-components` (every M3 component as a
   real `Widget`), rendered via `crates/mui-rs-renderer` (parley text +
   `gradient` + `shadow`).
4. **Reference digests** — `docs/design/{m3-styles,m3-foundations,m3-layout,
   m3-components,m3-glossary}.md`, `gemini/`, `references/`,
   `notebook-google-design-corpus.md`.

If a value is missing or stale, dispatch the **`google-design-researcher`**
sub-agent (Google-sources-only) to fetch it, then write it into `m3-tokens`.

## 1. The non-negotiable canon

- **Color** is HCT-derived: a source color → 5 key colors → tonal palettes
  (13 tones) → `--md-sys-color-*` roles → schemes (light/dark). Contrast: Δtone
  40 ⇒ ratio ≥ 3.0, Δ50 ⇒ ≥ 4.5. Default aphrody surface is **dark-first**.
- **Typography** is the **Google Sans** family (open-source 2025): Google Sans
  Text (UI/body), Google Sans (display), Google Sans Code (code, the Gemini code
  face). In `mui-rs` use `TextStyle::ui/display/code` — never hard-code a font.
- **Shape** is the **10-step** scale: none 0 · xs 4 · s 8 · m 12 · l 16 ·
  large-increased 20 · xl 28 · xl-increased 32 · xxl 48 · full 9999 (dp).
- **Elevation** levels 0/1/3/6/8/12 dp; in `mui-rs` use `shadow::draw_elevation`
  (dark occlusion shadows, not light surfaces).
- **Motion** has defined start/end (directional). Ambient/incoming ≈ slow
  (~2 s); direct input feedback is instant.
- **State layers**: hover 8% · focus 10% · pressed 10% · dragged 16%.

## 2. The Gemini / Expressive layer

- Gemini's identity: directional **gradients** (sharp opaque lead → diffuse
  tail) on a **circle** foundation; rounded containers; soft, optimistic, warm.
  Brand sweep blue `#4285F4` → purple `#9B72CB` → cyan `#1BA1E3`. In `mui-rs`:
  `gradient::brand_linear` / `gradient::directional`.
- Gemini dark theme imported pixel-exact: `docs/design/gemini/` +
  `GEMINI_DARK` in `m3-tokens`.
- M3 **Expressive** (2025): richer shape scale, dynamic motion, expanded color
  roles. material-web ships no Expressive in stable yet (lives in `labs/gb/`).
- **Additive/transparent displays** (Jetpack Compose Glimmer): dark surface +
  bright content, desaturated palette, optical-size type, ~0.6° legibility floor
  — see `references/transparent-screens-glimmer.md`.

## 3. How to answer

```
For <design question>: the M3 spec is <value/role/token>.
In aphrody: <m3-tokens symbol or mui-rs component> (<file:path>).
<one-line rationale or Gemini/Expressive nuance>.
```

- **Always** name the concrete token/role and its repo location.
- **Prefer** reusing an existing token to inventing one ("true > new",
  Reinfurt). If a new token is genuinely needed, derive it from the HCT/tonal
  system, don't pick a raw hex.
- **Never** present a non-Google design system as the spec; mention them only
  as contrast when explicitly asked.
- For component specs (size dp, radius, color role), cross-check
  `docs/design/m3-components-spec.md` and the live page via the sub-agent.

## 4. Building UI the Google way (mui-rs)

- Theme: `m3_tokens::color::{APHRODY_DARK, GEMINI_DARK}` → `mui_rs::Theme`.
- Text: `TextStyle::ui(size, weight, color)` (Google Sans stack).
- Surfaces: `surface` → `surface_container*` tiers; shadows via `shadow`.
- Brand moments: `gradient::brand_linear` for hero/sparkle/text masks.
- Components: pick from `crates/mui-rs-components` (full M3 set, real render).

## 5. Refresh loop

`/design-google-ingest` (scrape design.google) and the
`google-design-researcher` sub-agent keep the package current. The NotebookLM
corpus (`notebook-google-design-corpus.md`) is the curated reading list.

## Related

- Skills: `design-md`, `design-review`, `color-expert`, `creative-director`,
  `best-stack-2026` (for the Rust crate behind a UI need).
- Agents: `google-design-researcher` (Google-sources-only reader),
  `design-google-curator` (assembles DESIGN.md from scrapes).
