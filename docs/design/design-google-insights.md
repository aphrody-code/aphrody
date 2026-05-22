<!-- SPDX-License-Identifier: Apache-2.0 -->
# Google Design — distilled insights → aphrody

Actionable principles drawn from four Google Design library articles (read
2026-05-22), each paired with what aphrody's design system (`m3-tokens`,
`mui-rs`) does about it. Summaries are in our own words; see the source links
for the full pieces.

## 1. Typography — Google Sans is now open (2025)

Source: *Evolving Google's Typeface* — <https://design.google/library/google-sans-flex-font>

Takeaways:
- The Google Sans family (Sans, **Text**, **Code**, **Flex**) was open-sourced
  in 2025. Text is tuned for UI/body sizes; Code is the monospace Gemini uses to
  render code; Flex is a variable font whose **optical-size** axis adjusts
  counters and letter-spacing for legibility at a given size.
- A single, consistent typeface across surfaces measurably improves perceived
  quality; font fragmentation is a subtle but real friction.

aphrody applies it:
- `mui-rs-renderer::text` now centralises three stacks — `FONT_UI`
  (Google Sans Text → Google Sans → Roboto → system), `FONT_DISPLAY`,
  `FONT_CODE` (Google Sans Code → mono) — and `TextStyle::ui/display/code`.
  Every component switched from ad-hoc `"Roboto, Segoe UI…"` to `FONT_UI`.
- `m3-tokens` already carries `google_sans_flex` / `google_sans_code` modules;
  the variable-font opsz axis is the next lever (wire `wdth/opsz` into the type
  scale).

## 2. Gemini's visual language — directional gradients on a circle

Source: *Illustrating the Gemini App* — <https://design.google/library/gemini-ai-visual-design>

Takeaways:
- Gradients are treated as **directional energy**: a sharp, near-opaque leading
  edge that diffuses to a soft tail, used as a pointer to steer attention.
- The **circle** is the foundational shape (the Gemini logo is the negative
  space of four circles); containers echo it through rounded corners.
- Motion has a defined **start and end** — directional flow that mirrors the
  user's action and signals "thinking".

aphrody applies it:
- New `mui-rs-renderer::gradient` module: `brand_linear` (blue→purple→cyan
  brand sweep) and `directional` (lead-opaque→transparent energy gradient),
  plus the captured brand colours. These return vello `Gradient` brushes any
  `Scene::fill` can use.
- Rounded-corner identity is already in `m3-tokens` shape (10-step) and every
  `mui-rs` container.

## 3. Designing for additive/transparent displays (Jetpack Compose Glimmer)

Source: *Designing for Transparent Screens* — <https://design.google/library/transparent-screens>

Takeaways (counter-intuitive but broadly useful for dark UIs):
- On additive displays black = transparent, so the system is **dark surface +
  bright content** by default; light surfaces bleed (halation) and hurt
  legibility. Treat "black" as a clean container, not a colour.
- Palette is **neutral/desaturated** by default; saturated colour is spent
  sparingly to point at what matters (buttons), because vivid hues wash out.
- Depth is conveyed with **dark, rich shadows** (occlusion), not light surfaces;
  system controls use an exaggerated depth level.
- Typography uses **optical sizing**, bold weight and generous letter-spacing;
  legibility floor expressed as a visual angle (~0.6°).
- Motion: incoming/ambient content arrives **slowly (~2 s)** to invite rather
  than startle; direct input gets **instant** focus-ring feedback.

aphrody applies it:
- Validates aphrody's **dark-first** default (`APHRODY_DARK`, Gemini's
  `#131314` surface / `#e3e3e3` content). The `shadow` module's dark elevation
  blurs match the "occlusion, not light" guidance.
- Forward work: an "ambient/glass" theme variant (neutral desaturated palette),
  and a motion policy split — long (~2 s) for ambient entrances vs. short for
  input feedback — wired through `m3-tokens::motion` durations.

## 4. "True is better than new" (David Reinfurt)

Source: *True Design is Better than New Design* — <https://design.google/library/david-reinfurt-teaches-design>

Takeaways:
- Intuition and individual perspective are legitimate design drivers.
- A little **productive dissonance** — something not perfectly smooth — is what
  makes design memorable; invisibility isn't the only goal.
- The best results come from **coöperation**, not lowest-common-denominator
  compromise.

aphrody applies it:
- Bias toward **reusing and refining** proven tokens/components over chasing
  novelty (e.g. the 10-step shape scale was confirmed against shipping Gemini
  before adopting, not invented).
- The aphrody ↔ winclean peer model is coöperation by design (distinct repos,
  shared coord channel) rather than a merged compromise.

---

See also: [`m3-styles.md`](m3-styles.md), [`m3-foundations.md`](m3-foundations.md),
[`gemini-theme.md`](gemini-theme.md), [`aphrody-m3-tokens.md`](aphrody-m3-tokens.md).
