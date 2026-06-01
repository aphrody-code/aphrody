---
name: m3-docs
description: Answer Material Design 3 specification questions with attribution. Model-invocable and user-invocable. Use when asked "what does the M3 spec say about X", "look up Material Design docs", "official M3 reference for color / type / shape / motion / layout / components", "cite the Material guideline", or "find the m3.material.io page for X".
---

# m3-docs

Answer Material Design 3 (Material You) specification questions authoritatively, with the
exact canonical value and a citation. M3 is Google's design system; **only Google sources
are authoritative**. Never present a non-Google system as M3 spec.

## Source priority

Resolve every question in this order. Stop at the first tier that answers it.

### 1. Local repo corpus (fastest, always try first)

| File                                                          | Covers                                                                                                                                                                                                                                                                                                                                              |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/home/ubuntu/aphrody/packages/material-web/docs/01-md3-spec-foundations.md`   | The canonical, self-contained reference: HCT color model, dynamic color, ~47 color roles + light/dark tones, 3-tier token taxonomy, full 15-style type scale (px / line-height / weight / tracking), elevation levels, shape scale, motion easing + 16 durations, state-layer opacities, window size classes, accessibility ratios. **Start here.** |
| `/home/ubuntu/aphrody/packages/material-web/docs/design/m3-foundations.md`     | Core design principles                                                                                                                                                                                                                                                                                                                              |
| `/home/ubuntu/aphrody/packages/material-web/docs/design/m3-styles.md`          | Color, typography, elevation, shape, iconography                                                                                                                                                                                                                                                                                                    |
| `/home/ubuntu/aphrody/packages/material-web/docs/design/m3-design-tokens.md`   | Token levels (ref / sys / comp), CSS custom property format                                                                                                                                                                                                                                                                                         |
| `/home/ubuntu/aphrody/packages/material-web/docs/design/m3-components-spec.md` | Per-component anatomy, tokens, states                                                                                                                                                                                                                                                                                                               |
| `/home/ubuntu/aphrody/packages/material-web/docs/design/m3-components.md`      | Component catalog mapping                                                                                                                                                                                                                                                                                                                           |
| `/home/ubuntu/aphrody/packages/material-web/docs/design/m3-layout.md`          | Window size classes, canonical layouts, breakpoints, navigation patterns                                                                                                                                                                                                                                                                            |
| `/home/ubuntu/aphrody/packages/material-web/docs/design/m3-motion.md`          | Easing/duration tokens, transition patterns, spring model (Expressive)                                                                                                                                                                                                                                                                              |
| `/home/ubuntu/aphrody/packages/material-web/docs/design/m3-glossary.md`        | Term definitions                                                                                                                                                                                                                                                                                                                                    |

Migration docs hold applied spec knowledge:

- `migration/02-theme-token-migration.md` — MUI theme -> M3 tokens walkthrough
- `migration/06-tailwind-material-web.md` — Shadow DOM + Tailwind + `--md-sys-*` tokens
- `migration/11-material-symbols.md` — icon axes, font loading, subsetting

Read the relevant file before answering. If the corpus has the answer, cite the file path
and section. Do not fetch the web when the corpus suffices.

### 2. The m3.material.io link map (find the right page)

`docs/design/m3-material-io-llms.txt` is a complete llms.txt-format index of
`https://m3.material.io` (enumerated from its sitemap, fetched 2026-05-29), grouped into
Get started / Foundations / Styles / Components / Develop / Blog, one line per page with a
one-line description. Use it to **locate the exact canonical URL** for a topic before
fetching, and to answer "which page documents X" questions directly.

### 3. Live m3.material.io fetch (freshness / gaps only)

Fetch only from Google-authoritative domains, and only when tiers 1-2 are insufficient or a
freshness check is required. Resolve the URL from the link map above, then fetch.

| Topic                                       | Canonical URL                                                                                               |
| ------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| M3 overview                                 | `https://m3.material.io/`                                                                                   |
| Color: how the system works (HCT, palettes) | `https://m3.material.io/styles/color/system/how-the-system-works`                                           |
| Color roles                                 | `https://m3.material.io/styles/color/roles`                                                                 |
| Design tokens                               | `https://m3.material.io/foundations/design-tokens/overview`                                                 |
| Typography (type scale)                     | `https://m3.material.io/styles/typography/applying-type`                                                    |
| Elevation                                   | `https://m3.material.io/styles/elevation/applying-elevation`                                                |
| Shape scale                                 | `https://m3.material.io/styles/shape/corner-radius-scale`                                                   |
| Motion (easing/duration)                    | `https://m3.material.io/styles/motion/easing-and-duration/tokens-specs`                                     |
| State layers                                | `https://m3.material.io/foundations/interaction/states/state-layers`                                        |
| Breakpoints / window size classes           | `https://m3.material.io/foundations/layout/breakpoints/overview`                                            |
| Material Web (web components)               | `https://material-web.dev/`                                                                                 |
| material-color-utilities                    | `https://github.com/material-foundation/material-color-utilities`                                           |
| material-tokens reference (concrete values) | `https://github.com/material-foundation/material-tokens/blob/main/tokens.md`                                |
| Jetpack Compose M3                          | `https://developer.android.com/develop/ui/compose/designsystems/material3`                                  |
| M3 Expressive announcement                  | `https://blog.google/products-and-platforms/platforms/android/material-3-expressive-android-wearos-launch/` |

> Web caveat for fetching: m3.material.io is an Angular SPA rendered client-side, so a plain
> fetch returns no page body. For concrete numeric values, prefer the local corpus or the
> `material-tokens` GitHub source (plain text). For deep or multi-source reads, invoke the
> `md-design-researcher` agent rather than fetching manually. `docs_auto_search` (aphrody MCP,
> `mcp__plugin_aphrody_aphrody__docs_auto_search`) is a general fallback.

## Quick-reference: canonical M3 values

Distilled from `docs/01-md3-spec-foundations.md`. Cite that file when you use these.

### Color roles (~47, the runtime `--md-sys-color-*` set)

Accent families (each x primary / secondary / tertiary / error):
`primary`, `on-primary`, `primary-container`, `on-primary-container` (same shape for
secondary/tertiary/error). Surfaces & neutrals: `surface`, `surface-dim`, `surface-bright`,
`surface-container-lowest`, `surface-container-low`, `surface-container`,
`surface-container-high`, `surface-container-highest`, `surface-variant` (legacy),
`on-surface`, `on-surface-variant`, `outline`, `outline-variant`, `background`,
`on-background`. Inverse & utility: `inverse-surface`, `inverse-on-surface`,
`inverse-primary`, `surface-tint` (= primary), `scrim`, `shadow`.
Rule: content on a container always uses the matching `on-*` role. `primary` light tone 40 /
dark tone 80; `on-primary` light 100 / dark 20.

### Type scale (Roboto; weights 400 / 500 only)

| Style           | Size | Line height | Weight | Tracking |
| --------------- | ---- | ----------- | ------ | -------- |
| Display Large   | 57px | 64px        | 400    | -0.25px  |
| Display Medium  | 45px | 52px        | 400    | 0        |
| Display Small   | 36px | 44px        | 400    | 0        |
| Headline Large  | 32px | 40px        | 400    | 0        |
| Headline Medium | 28px | 36px        | 400    | 0        |
| Headline Small  | 24px | 32px        | 400    | 0        |
| Title Large     | 22px | 28px        | 400    | 0        |
| Title Medium    | 16px | 24px        | 500    | 0.15px   |
| Title Small     | 14px | 20px        | 500    | 0.1px    |
| Body Large      | 16px | 24px        | 400    | 0.5px    |
| Body Medium     | 14px | 20px        | 400    | 0.25px   |
| Body Small      | 12px | 16px        | 400    | 0.4px    |
| Label Large     | 14px | 20px        | 500    | 0.1px    |
| Label Medium    | 12px | 16px        | 500    | 0.5px    |
| Label Small     | 11px | 16px        | 500    | 0.5px    |

### Elevation (6 levels)

Level 0 = 0dp; 1 = 1dp; 2 = 3dp; 3 = 6dp; 4 = 8dp; 5 = 12dp. Rendered via tonal elevation
(surface tint, preferred) or shadow. On the web, prefer `surface-container-*` roles for
hierarchy.

### Shape scale (classic 7-step)

None 0dp; Extra Small 4dp; Small 8dp; Medium 12dp; Large 16dp; Extra Large 28dp; Full 9999dp
(pill). Token: `--md-sys-shape-corner-<token>`. Expressive intermediates (20/32/48dp) and
shape morphing are not on `@material/web`.

### Motion easing (cubic-bezier) + durations

| Easing                | Curve                             |
| --------------------- | --------------------------------- |
| Standard              | `cubic-bezier(0.2, 0, 0, 1)`      |
| Standard Decelerate   | `cubic-bezier(0, 0, 0, 1)`        |
| Standard Accelerate   | `cubic-bezier(0.3, 0, 1, 1)`      |
| Emphasized Decelerate | `cubic-bezier(0.05, 0.7, 0.1, 1)` |
| Emphasized Accelerate | `cubic-bezier(0.3, 0, 0.8, 0.15)` |
| Linear                | `cubic-bezier(0, 0, 1, 1)`        |

(Emphasized full is a 2-segment spline, not a single bezier; on the web approximate with
Emphasized Decelerate for enter, Emphasized Accelerate for exit.)

Durations (16): Short 1-4 = 50/100/150/200ms; Medium 1-4 = 250/300/350/400ms;
Long 1-4 = 450/500/550/600ms; Extra Long 1-4 = 700/800/900/1000ms. Duration grows with the
spatial size of the animation.

### State layers (opacity of the `on-*` content color)

Enabled 0%; Hover 8%; Focus 10%; Pressed 10%; Dragged 16%. Disabled (separate mechanism):
content 38%, container 12%.

### Window size classes (width, dp)

Compact < 600 (phones; bottom nav bar); Medium 600-839 (small tablets/foldables; nav rail);
Expanded 840-1199 (tablets/desktop; rail or drawer); Large 1200-1599 (desktop; permanent
drawer + multi-pane); Extra-large >= 1600 (very large screens). Spacing grid is 4dp
(prefer 8dp for larger gaps). Touch targets >= 48x48dp, >= 8dp apart.

### Accessibility (WCAG)

Normal text 4.5:1; large text (>= 18pt regular / 14pt bold) and non-text UI 3:1. The tonal
palette guarantees these by construction; contrast levels Standard (0.0) / Medium (0.5) /
High (1.0) raise it further.

## Rules

**Attribution is mandatory.** Every answer ends with its source (local file path + section,
or the resolved m3.material.io URL).

**Cite the link map for "which page" questions.** When asked where something is documented,
answer with the URL from `docs/design/m3-material-io-llms.txt`.

**Never treat non-Google systems as M3 authority.** Bootstrap, Tailwind UI, Ant Design,
Chakra UI, Apple HIG, and MUI's own design opinions are contrast examples only; they do not
define M3 behavior.

**Do not reproduce article bodies verbatim.** Distill facts, quote the specific values
(token names, dp, ratios, curves), and attribute.

**Flag the web-vs-native gap when relevant:**

- M3 Expressive (motion physics/springs, shape morphing, split buttons, button groups,
  docked toolbars, FAB menus, loading indicators, emphasized type) is Android/Compose-only
  as of 2026; it is NOT in `@material/web`.
- `@material/web` is in maintenance mode (no new feature development).
- On the web, M3 is fully usable via its tokenized classic foundations (color roles, type
  scale, shape scale, motion easing/duration, state layers as CSS custom properties), minus
  Expressive additions.

**Do not invent token names.** Forms: `--md-sys-color-<role>`,
`--md-sys-typescale-<role>-<size>-<property>`, `--md-sys-shape-corner-<scale>`,
`--md-sys-motion-*`, `--md-comp-<component>-<property>`. If unsure, check the local corpus or
the `material-tokens` reference.

## Answer format

1. State the fact / spec value directly.
2. Provide relevant token names or a code snippet when applicable.
3. Note any web limitation if applicable.
4. Cite: `Source: <path or URL>, <section>`.

## References

- `docs/design/m3-material-io-llms.txt` — complete llms.txt index of m3.material.io (the
  link map). Referenced by path to avoid duplication; keep that file as the single source.
- Sibling skill `google-design` — Google's broader design language (Gemini, Google Sans,
  Material You expressiveness) when the question is about "the Google way" rather than the M3
  spec.
