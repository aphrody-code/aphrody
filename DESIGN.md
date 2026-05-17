---
version: alpha
name: Aphrody
description: >-
  Aphrody is the cross-platform CLI's visual layer. Material Design 3 baseline
  cascade + a Gemini-derived brand layer (spectrum-shift gradient, four-color
  sparkle, rounded foundational shapes) painted in Google Sans Flex variable
  type. Token values match crates/m3-tokens/src/{color,typography,shape,
  elevation,motion,state,gemini_brand,google_sans_flex}.rs byte-for-byte; bxc
  and any DESIGN.md-aware agent can pixel-perfect from this single file.
colors:
  primary: "#6750A4"
  on-primary: "#FFFFFF"
  primary-container: "#EADDFF"
  on-primary-container: "#21005D"
  secondary: "#625B71"
  on-secondary: "#FFFFFF"
  secondary-container: "#E8DEF8"
  on-secondary-container: "#1D192B"
  tertiary: "#7D5260"
  on-tertiary: "#FFFFFF"
  tertiary-container: "#FFD8E4"
  on-tertiary-container: "#31111D"
  error: "#B3261E"
  on-error: "#FFFFFF"
  error-container: "#F9DEDC"
  on-error-container: "#410E0B"
  surface: "#FEF7FF"
  on-surface: "#1D1B20"
  surface-variant: "#E7E0EC"
  on-surface-variant: "#49454F"
  surface-container-lowest: "#FFFFFF"
  surface-container-low: "#F7F2FA"
  surface-container: "#F3EDF7"
  surface-container-high: "#ECE6F0"
  surface-container-highest: "#E6E0E9"
  outline: "#79747E"
  outline-variant: "#CAC4D0"
  background: "#FEF7FF"
  on-background: "#1D1B20"
  inverse-surface: "#322F35"
  inverse-on-surface: "#F5EFF7"
  gemini-brand-blue: "#4285F4"
  gemini-brand-purple: "#9168C0"
  gemini-brand-pink: "#EC4899"
  gemini-brand-yellow: "#FAE366"
  gemini-brand-green: "#BFF28D"
  google-dot-blue: "#4285F4"
  google-dot-red: "#EA4335"
  google-dot-yellow: "#FBBC04"
  google-dot-green: "#34A853"
typography:
  display-large:
    fontFamily: Google Sans Flex
    fontSize: 57px
    fontWeight: 400
    lineHeight: 1.12
    letterSpacing: -0.25px
    fontVariation: '"wght" 400, "opsz" 96, "GRAD" 30, "ROND" 60'
  display-medium:
    fontFamily: Google Sans Flex
    fontSize: 45px
    fontWeight: 400
    lineHeight: 1.16
    letterSpacing: 0px
    fontVariation: '"wght" 400, "opsz" 72, "GRAD" 20, "ROND" 50'
  display-small:
    fontFamily: Google Sans Flex
    fontSize: 36px
    fontWeight: 400
    lineHeight: 1.22
    letterSpacing: 0px
    fontVariation: '"wght" 400, "opsz" 48, "ROND" 40'
  headline-large:
    fontFamily: Google Sans Flex
    fontSize: 32px
    fontWeight: 400
    lineHeight: 1.25
    letterSpacing: 0px
    fontVariation: '"wght" 500, "opsz" 36, "ROND" 30'
  headline-medium:
    fontFamily: Google Sans Flex
    fontSize: 28px
    fontWeight: 400
    lineHeight: 1.29
    letterSpacing: 0px
    fontVariation: '"wght" 500, "opsz" 28, "ROND" 25'
  headline-small:
    fontFamily: Google Sans Flex
    fontSize: 24px
    fontWeight: 400
    lineHeight: 1.33
    letterSpacing: 0px
    fontVariation: '"wght" 500, "opsz" 24, "ROND" 20'
  title-large:
    fontFamily: Google Sans Flex
    fontSize: 22px
    fontWeight: 400
    lineHeight: 1.27
    letterSpacing: 0px
    fontVariation: '"wght" 500, "opsz" 22, "ROND" 15'
  title-medium:
    fontFamily: Google Sans Flex
    fontSize: 16px
    fontWeight: 500
    lineHeight: 1.50
    letterSpacing: 0.15px
    fontVariation: '"wght" 500, "opsz" 16'
  title-small:
    fontFamily: Google Sans Flex
    fontSize: 14px
    fontWeight: 500
    lineHeight: 1.43
    letterSpacing: 0.1px
    fontVariation: '"wght" 500, "opsz" 14'
  body-large:
    fontFamily: Google Sans Flex
    fontSize: 16px
    fontWeight: 400
    lineHeight: 1.50
    letterSpacing: 0.5px
    fontVariation: '"wght" 400, "opsz" 16, "ROND" 20'
  body-medium:
    fontFamily: Google Sans Flex
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.43
    letterSpacing: 0.25px
    fontVariation: '"wght" 400, "opsz" 14'
  body-small:
    fontFamily: Google Sans Flex
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.33
    letterSpacing: 0.4px
    fontVariation: '"wght" 400, "opsz" 12'
  label-large:
    fontFamily: Google Sans Flex
    fontSize: 14px
    fontWeight: 500
    lineHeight: 1.43
    letterSpacing: 0.1px
    fontVariation: '"wght" 500, "opsz" 14'
  label-medium:
    fontFamily: Google Sans Flex
    fontSize: 12px
    fontWeight: 500
    lineHeight: 1.33
    letterSpacing: 0.5px
    fontVariation: '"wght" 500, "opsz" 12'
  label-small:
    fontFamily: Google Sans Flex
    fontSize: 11px
    fontWeight: 500
    lineHeight: 1.45
    letterSpacing: 0.5px
    fontVariation: '"wght" 500, "opsz" 11'
rounded:
  none: 0px
  xs: 4px
  sm: 8px
  md: 12px
  lg: 16px
  xl: 28px
  full: 9999px
  gemini-prompt-bar: 28px
  gemini-message: 24px
  gemini-chip: 18px
spacing:
  xs: 4px
  sm: 8px
  md: 16px
  lg: 24px
  xl: 32px
  2xl: 48px
  3xl: 64px
components:
  button-filled:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.label-large}"
    rounded: "{rounded.full}"
    padding: "{spacing.lg}"
  button-outlined:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.primary}"
    typography: "{typography.label-large}"
    rounded: "{rounded.full}"
    padding: "{spacing.lg}"
  button-text:
    backgroundColor: "transparent"
    textColor: "{colors.primary}"
    typography: "{typography.label-large}"
    rounded: "{rounded.full}"
    padding: "{spacing.md}"
  card-elevated:
    backgroundColor: "{colors.surface-container-low}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.md}"
    padding: "{spacing.md}"
  card-filled:
    backgroundColor: "{colors.surface-container}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.md}"
  text-field-outlined:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface}"
    typography: "{typography.body-large}"
    rounded: "{rounded.xs}"
  dialog:
    backgroundColor: "{colors.surface-container-high}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.xl}"
  chip-assist:
    backgroundColor: "{colors.surface-container-low}"
    textColor: "{colors.on-surface}"
    typography: "{typography.label-large}"
    rounded: "{rounded.sm}"
  fab:
    backgroundColor: "{colors.primary-container}"
    textColor: "{colors.on-primary-container}"
    rounded: "{rounded.lg}"
  gemini-prompt-bar:
    backgroundColor: "{colors.surface-container}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.gemini-prompt-bar}"
    padding: "{spacing.md}"
  gemini-message-user:
    backgroundColor: "{colors.surface-container-high}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.gemini-message}"
    padding: "{spacing.lg}"
  gemini-message-assistant:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.gemini-message}"
    padding: "{spacing.lg}"
  gemini-suggestion-chip:
    backgroundColor: "{colors.surface-container-low}"
    textColor: "{colors.on-surface}"
    typography: "{typography.body-medium}"
    rounded: "{rounded.lg}"
    padding: "{spacing.md}"
---
<!-- SPDX-License-Identifier: Apache-2.0 -->
# Aphrody Design System
## Overview
Aphrody is a cross-platform CLI that wears a desktop-and-web identity
shaped by Google's Material Design 3 baseline plus a Gemini-derived brand
layer. The personality is **calm, precise, and quietly expressive** —
the surface stays out of the way of the engineer until a moment of
delight is earned. Rounded foundational shapes (echoing Google's
four-color dot lineage), a single signature gradient
(blue → purple → pink), and the variable Google Sans Flex face give the
product a recognizable Gemini-family fingerprint while staying inside
the standard M3 token grammar so MWC3 / Angular Material / Material Web
components drop in without bespoke theming.
**Audience:** systems engineers, infrastructure designers, AI
researchers, and the curious. Density should default to "engineer
desktop" (compact, informative, keyboard-first), with breathing room
preserved on hero / empty-state surfaces.
**Emotional target:** trustworthy, energetic on demand, never
performative. Motion is purposeful and rare.
## Colors
The Aphrody palette is rooted in the M3 baseline (`#6750A4` primary,
`#FEF7FF` surface). Five **Gemini brand** colors layer on top to mark
moments of identity (the prompt-send affordance, sparkle, avatar ring,
streaming shimmer). The classic **Google four-color dots** seed the
sparkle gradient and any rare four-stop accent surface.
- **Primary (`{colors.primary}`):** Deep violet, the M3 canonical seed.
  Use for the dominant action, key navigation indicators, focus rings.
- **Secondary (`{colors.secondary}`):** Muted lavender-grey for second-tier
  surfaces and supporting affordances.
- **Tertiary (`{colors.tertiary}`):** Dusty rose, reserved for accent
  hover states and high-affinity highlights.
- **Surface family (`{colors.surface-container-lowest}` →
  `{colors.surface-container-highest}`):** The five M3 surface tints
  used to layer cards, sheets, and floating affordances. Always pair
  with `{colors.on-surface}` for text.
- **Gemini spectrum (`{colors.gemini-brand-blue}` →
  `{colors.gemini-brand-purple}` → `{colors.gemini-brand-pink}`):**
  The signature gradient. Applied to prompt-send, sparkle, avatar
  ring, focus-state pill border, and streaming-message shimmer. Do
  not use as a fill on body content.
- **Four-color dot lineage
  (`{colors.google-dot-blue}` / `{colors.google-dot-red}` /
  `{colors.google-dot-yellow}` / `{colors.google-dot-green}`):**
  Source of the radial sparkle gradient. Echo with caution; over-use
  dilutes the Google identity.
## Typography
Google Sans Flex is the single product face. The variable axes
(`wght`, `opsz`, `wdth`, `GRAD`, `slnt`, `ROND`) let one font ship every
display, headline, title, body, and label cut in the M3 type scale.
Body and label stay neutral; display and headline lean rounder + slightly
heavier grade for a warm spatial quality that mirrors Gemini's
foundational shapes per the design.google article.
- **Display & headline:** Higher `opsz` (24–96), `GRAD ≥ 20`,
  `ROND ≥ 20`. This is where the Gemini "warm, spatial, rounded"
  signature shows up.
- **Title & body:** `ROND = 0` for readability at small sizes.
- **Label:** `wght = 500` always.
Static fallbacks ship under `assets/fonts/google-sans-flex/static/`
for runtimes that reject variable fonts. License: SIL Open Font
License v1.1 (`assets/fonts/google-sans-flex/OFL.txt`).
## Layout
The default layout is a **two-pane CLI / four-pane desktop** grid:
- **CLI surfaces (Linux native, terminal):** single column, `{spacing.md}`
  default gutter, optional right rail (Help / Doctor) at `{spacing.2xl}`.
- **Desktop GUI (wry + tao via crates/gui):** three-pane navigation rail
  + content + side sheet, gutters at `{spacing.lg}` between panes.
- **Gemini chat surface
  (crates/aphrody-wasm/examples/gemini-clone-pixel-perfect.html):**
  72 px collapsed rail + chat column with max-width 920 px, prompt bar
  pinned to bottom.
Spacing scale (`xs`, `sm`, `md`, `lg`, `xl`, `2xl`, `3xl`) is the only
permitted set; never inject ad-hoc px values.
## Elevation & Depth
M3 elevation levels 0–5. Aphrody uses **at most three levels per
surface** at any time:
- **Level 0:** Flat baseline (cards on inverse-surface backgrounds).
- **Level 1:** Default elevated card resting state.
- **Level 2:** Hovered card or affordance.
- **Level 3:** Dropped menu / select / dialog scrim.
- **Level 4:** Floating action button while pressed.
- **Level 5:** Modal dialog or bottom sheet.
Elevation tokens map to box-shadow strings via
`crates/m3-tokens/src/elevation.rs::LEVELS`. Never roll your own
shadow stack.
## Shapes
Corner radius is the M3 scale plus three Gemini overrides:
- `{rounded.none}` for rigid containers (debugger panes, code blocks).
- `{rounded.xs}`, `{rounded.sm}` for text fields and small chips.
- `{rounded.md}` (12 px) for elevated cards — the workhorse radius.
- `{rounded.lg}` for FAB and large containers.
- `{rounded.xl}` for dialogs and bottom sheets.
- `{rounded.full}` for pill-shaped buttons + buttons states.
- **Gemini overrides** (`{rounded.gemini-prompt-bar}` = 28 px,
  `{rounded.gemini-message}` = 24 px, `{rounded.gemini-chip}` = 18 px)
  ONLY on Gemini-branded surfaces.
The Gemini "foundational shape" guidance from
design.google/library/gemini-ai-visual-design — sparkle echoing four-
color dots, prompt-bar echoing pill, message echoing rounded square —
is encoded directly here so any DESIGN.md-aware agent renders the
shapes correctly without re-reading the article.
## Components
Token references (`{components.button-filled}` etc.) in the YAML front
matter are the normative API. The component list mirrors the
`crates/shadcn-bridge/src/*.rs` modules so any Rust consumer can pull
both the rendered DOM and the token table from the same source of
truth.
- **Button (filled / outlined / text / tonal):** all four M3 variants
  use `{rounded.full}`. Filled is the dominant CTA; tonal is the second
  layer; text is for low-emphasis surfaces.
- **Card (elevated / filled / outlined):** elevated is default; filled
  for content carousels; outlined for dense lists where elevation noise
  is unwelcome.
- **Text field (outlined / filled):** outlined for forms; filled for
  dense inline editing. Always pair with `{typography.body-large}` for
  the input and `{typography.label-small}` for the floating label.
- **Dialog:** `{rounded.xl}` (28 px) at level 5 elevation. Headline in
  `{typography.headline-small}`.
- **Tabs, snackbar, select, checkbox, radio, switch, slider, lists,
  nav-bar / drawer / rail, app-bar, FAB, chip, progress, badge,
  tooltip, icon-button, segmented-button, bottom-sheet, date-picker,
  time-picker, search-bar:** see
  `crates/shadcn-bridge/src/<name>.rs` and the v2 pixel-perfect HTML
  demo at `crates/aphrody-wasm/examples/m3-shadcn-pixel-perfect-v2.html`
  for the canonical rendering.
- **Gemini atoms:** `gemini-sparkle`, `gemini-prompt-bar`,
  `gemini-message-user`, `gemini-message-assistant`,
  `gemini-suggestion-chip`, `gemini-avatar-ring`. Tokens defined above;
  full DOM constructor at
  `crates/shadcn-bridge/src/gemini.rs`; rendered preview at
  `crates/aphrody-wasm/examples/gemini-clone-pixel-perfect.html`.
## Do's and Don'ts
**Do:**
- Use `{colors.gemini-spectrum-shift}` only on Gemini-branded surfaces
  (prompt-send, avatar ring, streaming shimmer, focus pill border).
- Compose the variable font axes (`wght`, `opsz`, `ROND`, `GRAD`) to
  follow the M3 type scale; never bake static cuts in front-end code
  except as `font-display: swap` fallbacks.
- Pair every `surface` with its matching `on-surface` token; do not
  introduce custom text colors.
- Anchor every component token at one of the references defined above;
  do not redefine values inline in CSS or in Rust.
- Cap motion at the M3 baseline (`short3` = 150 ms,
  `medium2` = 300 ms). Use `cubic-bezier(0.2, 0, 0, 1)`
  (`standard` easing) by default.
**Don't:**
- Don't use the spectrum-shift gradient as a body fill, page
  background, or large surface paint — it is a **moment** color, not a
  **state** color.
- Don't ship the four-color dot lineage outside the sparkle gradient
  or designated brand-anchor surfaces; Google reserves the dots for
  identity-level uses.
- Don't introduce a third typeface; if a code surface needs
  monospace, defer to `Google Sans Mono` via the GUI crate's terminal
  pane, not here.
- Don't increase corner radius beyond `{rounded.xl}` (28 px) for
  general components; the Gemini overrides above are exhaustive.
- Don't roll your own elevation shadow strings; consume
  `crates/m3-tokens/src/elevation.rs::LEVELS` instead.
- Don't author DESIGN.md by hand — re-run
  `/design-google-ingest` to refresh the design.google ledger and
  edit `DESIGN.md` only via the curated token tables in
  `crates/m3-tokens/src/`.
---
_Spec: <https://github.com/google-labs-code/design.md> · Generators:
`scripts/design-google-curate.ts`, `crates/m3-tokens/src/`. Validate
with `bun x @google/design.md lint DESIGN.md`._
