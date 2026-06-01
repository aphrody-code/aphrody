---
name: google-design
description: >
  Apply Google's design language to a UI. Load whenever the user wants something to look or
  feel like a Google product, including requests like "apply Google design style", "make it
  look like a Google product", "Google-style UI", "design.google guidance", "Google design
  principles", "Google Sans typography", "Product Sans", "Gemini visual language", "the
  Gemini sparkle / gradient look", "Material You expressiveness", or any question about how
  Google approaches color, type, motion, and brand across its products (Search, Gemini,
  Workspace, Pixel). This is the language layer above the Material 3 spec; for exact M3 token
  values use the sibling skill m3-docs.
---

# google-design

Equip yourself to produce UI that reads as a genuine Google product. Google's design language
sits on top of Material Design 3: M3 gives the tokens and components; this skill gives the
**intent, restraint, and brand grammar** that make a screen feel like Google rather than
generically Material. Ground every answer in the in-repo corpus; attribute Google sources.

For the full link map of articles, see `docs/design/google-design-llms.txt` (llms.txt index
of design.google). For exact M3 numeric values (type scale, color roles, motion curves), defer
to the `m3-docs` and `m3-design-guide` sibling skills.

## The Google design mindset

Google design is **clarity, deference, and depth**, expressed through restraint:

- **Bold simplicity.** Strong, confident layouts built from few elements. One clear primary
  action per surface. Generous whitespace. The content is the interface; chrome recedes.
- **Deference to content.** Color, motion, and depth serve the content and the user's task,
  never decoration for its own right. A surface earns an accent only when it carries meaning.
- **Depth through tonal hierarchy, not borders.** Layer UI with `surface-container-*` roles
  and tonal elevation before reaching for shadows or outlines. Hierarchy is built from tone.
- **Material You expressiveness.** Personal, adaptive color derived from a seed (the user's
  wallpaper or a brand color) via the HCT system. The same component re-themes automatically;
  the design never hard-codes a hex. Expressiveness comes from dynamic color + purposeful
  motion + rounded, friendly shape, not from ornament.
- **Accessibility is the default, not a pass.** The tonal palette guarantees contrast by
  construction (see m3-docs for ratios). Never encode meaning with color alone.
- **Human, warm, and calm.** Rounded corners, soft motion, friendly copy. Google reads as
  approachable and trustworthy, not cold or aggressive.

Practical translation in this monorepo: build with `@aphrody-code/m3-react` (`Md*`) components,
theme with `@aphrody-code/m3-tokens` dynamic color (`--md-sys-color-*` runtime roles), and
animate with `@aphrody-code/m3-motion`. Never introduce a non-Google design system's spec as
the source of truth; reference others only as contrast.

## Typography: Google Sans and Product Sans

Source: design.google "Google Sans: evolving Google's typeface"
(<https://design.google/library/google-sans-flex-font>, fetched 2026-05-29). Distilled in
`docs/design/references/google-sans-family.md`.

- **Product Sans** is the wordmark/logotype typeface behind the Google logo (2015 identity
  refresh). It is a brand asset, not a UI text face. Do not set body or UI copy in it.
- **Google Sans** (2018) is the geometric brand display typeface — confident, circular, for
  large text, headers, and product lockups. It originally paired with **Roboto** for small
  text (a two-typeface system).
- **Google Sans Text** (2020) is the small-size variant: taller, more condensed, less
  circular, looser spacing, proportions aligned to Roboto. Use it for body and dense UI.
- **Google Sans Mono** (2020) and **Google Sans Code** (2025, the code display face in Gemini)
  cover monospace/editorial and source code respectively.
- **Google Sans Flex** is the variable-font version; Google Sans and Flex went open-source in
  2025 (now on Google Fonts). Prefer the variable font: one file, continuous `wght`/`wdth`/
  optical-size axes for adaptive hierarchy.
- The family spans 20+ writing systems (Arabic, CJK, Ge'ez, Thai...), one of the largest type
  families in the world — relevant for global/i18n products.

Guidance: use **Google Sans / Google Sans Flex for display, headlines, and brand moments**;
**Google Sans Text (or Roboto / Roboto Flex) for body and UI**; **Google Sans Code for code**.
Map onto the M3 type scale roles (display/headline/title for the brand face, body/label for the
text face). Load via Google Fonts with `font-display: swap`; subset for performance.

## The Gemini visual language and the sparkle

Source: design.google "Illustrating the Gemini app"
(<https://design.google/library/gemini-ai-visual-design>, fetched 2026-05-29) and the AI sparkle
research (<https://design.google/library/ai-sparkle-icon-research-pozos-schmidt>). Distilled in
`docs/design/references/gemini-visual-language.md` and the dark-mode token analysis in
`docs/design/google/ANALYSE.md`.

- **Gradients are the central device.** They act as "context builders": a crisp, opaque leading
  edge that diffuses toward a soft tail, forming directional pointers that guide attention and
  personify the AI's reasoning. Brand goals: intuitive, immersive, accessible, aspirational, and
  above all **trustworthy**.
- **The circle is the foundational shape.** Simplicity, harmony, comfort. The Gemini logo is born
  from the negative space of four circles. Buttons and containers use rounded corners for
  continuity with the Google ecosystem.
- **Heritage references:** the four Google brand dots (red/yellow/green/blue); Material shapes,
  softened and blurred for an ethereal quality.
- **Intentional motion.** Every animation has a defined start and end — directional flow that
  mirrors the user's action. Radial gradient waves visualize voice; animated icons signal new
  features. Quality words: warm, spatial, rounded.
- **The sparkle icon** (`auto_awesome` in Material Symbols) is the most ubiquitous Gemini brand
  element — it marks AI-generated affordances.

Implementation grammar (from `docs/design/google/ANALYSE.md`): build the structure with M3 roles
and components, and keep the brand gradient as the **single** hard-coded value, exposed once as a
custom property. The canonical Gemini "sparkle" gradient is
`#4285f4 -> #9b72cb -> #d96570` (expose as `--gemini-sparkle`); everything else resolves through
`--md-sys-color-*`. The live transposition is the "Gemini AI Mode" section of `examples/showcase`.

```css
:root {
  --gemini-sparkle: linear-gradient(90deg, #4285f4 0%, #9b72cb 50%, #d96570 100%);
}
```

Use the sparkle gradient sparingly: the AI affordance (assist chip + `auto_awesome` icon), a halo
on the active AI surface, active citation chips. Never paint general UI with it.

## Color and Material You

- Derive the whole scheme from one seed via the HCT/dynamic-color system (TonalSpot is the
  default Material You variant). On the web, generate the ~47 `--md-sys-color-*` roles at runtime
  with `@aphrody-code/m3-tokens/dynamic-color` (`applyDynamicColor(seed, {dark})`).
- The Google product look (e.g. Search dark mode) maps to roles, not hexes: page background ->
  `background`/`surface`; bars and cards -> `surface-container` / `surface-container-high`; hover
  -> a state layer; separators -> `outline-variant`; focus outline -> `outline`; primary text ->
  `on-surface`; secondary text -> `on-surface-variant`; result links -> `primary`; visited ->
  `tertiary`. See the full mapping table in `docs/design/google/ANALYSE.md`.
- Provide light and dark by injecting the right scheme; roles handle dark mode automatically. The
  only intentional hex is `--gemini-sparkle`.

## Motion

Google motion is purposeful and directional, never decorative. Use the M3 easing/duration model
(see m3-docs for curves and the 16 duration tokens): Standard easing for micro-interactions,
Emphasized Decelerate for entering content, Emphasized Accelerate for exiting. Match duration to
the spatial size of the change. In React, animate with `@aphrody-code/m3-motion` transition
patterns (fade-through, shared-axis, container transform). For Gemini surfaces, lean into
directional gradient motion and radial voice waves as accents — still bounded by the easing model.

## Hardware / Pixel adjacency

Google's hardware design language (Pixel, Nest) shares the same DNA: soft geometry, calm color,
material honesty, Material You wallpapers seeding the on-device theme. For software that lives
next to Pixel hardware, keep the dynamic-color seed flow and the rounded, friendly shape language
consistent. Reference: design.google hardware article
(<https://design.google/library/google-design-hardware-isabelle-olsson>).

## Rules

- **Attribution is mandatory.** When you state a Google design fact, cite the in-repo reference
  (`docs/design/references/*.md`, `docs/design/google/ANALYSE.md`) or the design.google URL from
  `docs/design/google-design-llms.txt`, with the 2026-05-29 fetch date where it is a source claim.
- **No verbatim dumps.** Distill; never paste article bodies.
- **Google sources only as authority.** Other design systems are contrast, never the spec.
- **One hex only.** Outside `--gemini-sparkle`, every color is a `--md-sys-color-*` role.
- **Real components only.** Use `Md*` exports that actually exist in `@aphrody-code/m3-react`.

## References and going deeper

- `docs/design/google-design-llms.txt` — complete llms.txt link map of design.google (themes:
  AI/Gemini, Material/Material You, color, typography, motion, brand/hardware, layout, XR,
  accessibility, principles). Use it to find the article for a topic.
- `docs/design/references/gemini-visual-language.md` — distilled Gemini visual language facts.
- `docs/design/references/google-sans-family.md` — the Google Sans family.
- `docs/design/references/transparent-screens-glimmer.md` — Android XR (Glimmer) design notes.
- `docs/design/google/ANALYSE.md` — Google Search/Gemini dark-mode grammar -> M3 role mapping +
  the `--gemini-sparkle` value; the showcase transposition.
- Sibling skill `m3-docs` — the Material Design 3 specification with exact token values.
- Sibling skill `m3-design-guide` — actionable M3 build directives (color roles, type, elevation,
  shape, motion, state layers, the decision checklist).
