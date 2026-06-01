<!-- SPDX-License-Identifier: Apache-2.0 -->

# Material 3 foundations

> **Paraphrase note.** Everything below is an original-wording digest of the
> Material Design 3 _Foundations_ section, written for aphrody's design layer.
> No sentence reproduces source copy; values (opacities, contrast ratios,
> target sizes) are factual constants quoted verbatim because they are not
> copyrightable. Treat this as a reference index, not a copy of the upstream
> guidelines.
>
> Source: <https://m3.material.io/foundations> (fetched 2026-05-22).
>
> **Out of scope (cross-linked, not duplicated):**
>
> - Layout (overview, scaffold, grids/spacing, breakpoints, RTL, canonical
>   examples) → see [`m3-layout.md`](m3-layout.md).
> - Material A–Z terminology → see [`m3-glossary.md`](m3-glossary.md).

---

## 1. Overview & principles (accessibility-first)

The Foundations section frames accessibility as a baseline design value rather
than an afterthought: requirements are baked into every component so inclusive
behaviour ships by default. Designing for a wide spectrum of abilities up front
(low vision, blindness, hearing, cognitive, motor, and situational limits like a
broken arm) avoids costly redesigns and reduces design/tech debt.

Three guiding principles drive accessible work:

- **Honour individuals** — default experiences rarely fit everyone; offer
  customisation so people can adapt the UI to their own shifting needs.
- **Learn before, not after** — research the breadth of user abilities up front;
  unanticipated outcomes become a learning foundation rather than a failure.
- **Requirements as a starting point** — WCAG minimums are creative
  opportunities, not ceilings. Dark mode, TTS, and STT all began as access
  solutions that ended up benefiting everyone.

## 2. Designing (accessible implementation)

A three-stage framework drawn from WCAG and industry practice turns a visual UI
into a linear, text-based experience that maps cleanly to code:

- **Accessibility markup** — document focus order and keyboard semantics in the
  spec itself (e.g. Tab moves focus, Space/Enter toggles a switch).
- **Implementation** — lean on native platform controls and semantic HTML so
  assistive tech, shortcuts, and structure work without bespoke wiring.
- **Color & flow** — colour and contrast support navigation and comprehension.

Rule of thumb: prefer standard platform elements (native dialogs) over custom
look-alikes, which need extra testing to behave with assistive technology.

## 3. Content design

UX writing and information design keep UIs legible. The baseline house style is
**AP (Associated Press) Style** unless a component note overrides it. Sub-areas:

| Area                      | Digest                                                                                                |
| ------------------------- | ----------------------------------------------------------------------------------------------------- |
| **Writing / style guide** | Clear, concise UI text anyone can parse; AP-style conventions; updated first-person-pronoun guidance. |
| **Alt text**              | Off-screen description for screen readers (and the fallback when an image fails to load).             |
| **Global writing**        | Write for translation and a worldwide audience; avoid idioms and locale-bound phrasing.               |
| **Notifications**         | Content rules for notification copy (concise, actionable).                                            |

### Alt-text rules worth pinning

- Describe **meaning and context**, not exhaustive visual detail.
- Recommended length: **≤ 140 characters** (longer text gets truncated by some
  readers).
- Never prefix with "image of" — the reader already announces "image".
- Purely decorative images take an empty alt (`alt=""`) so readers skip them.
- Tailor alt text to its surrounding context; don't duplicate the caption.
- Charts/graphs: summarise the takeaway, not every data point — a useful formula
  is "summary of [data type] + [reason for the chart]". Link to source data and
  prefer interactive charts for dense analysis visualisations.

## 4. Interaction

Four interaction articles: **gestures**, **inputs**, **selection**, and
**states**.

- **Gestures** — touch/pointer motions (tap, drag, swipe, long-press) that
  trigger actions.
- **Inputs** — input modalities (touch, mouse, keyboard, voice) a component must
  support.
- **Selection** — single/multi-select patterns and their visual treatment.
- **States** — visual status indicators; combinable (e.g. selected + hover) and
  applied consistently across components.

### 4.1 States & the state layer (the load-bearing detail)

A **state layer** is a semi-transparent overlay that signals interaction status.
Only one applies at a time. Its colour is taken from the component's **content
(`on-*`) colour**, never a fixed grey — only the **opacity** changes per state.
The overlay sits between the container and the content layers. The state layer
is **40 dp** while the interactive target is **48 dp**.

The six interaction states M3 names: **enabled, disabled, hover, focused,
pressed, dragged**. The four that composite an overlay carry fixed opacities:

| State   | State-layer opacity | Trigger                               |
| ------- | ------------------- | ------------------------------------- |
| Hover   | **+8 %** (0.08)     | Pointer rests over the element        |
| Focus   | **+10 %** (0.10)    | Keyboard / voice / programmatic focus |
| Pressed | **+10 %** (0.10)    | Tap or mouse-button down              |
| Dragged | **+16 %** (0.16)    | Press-and-move                        |

> **Verified against source:** the M3 _State layers_ page lists exactly these
> four overlay values — Hover +8 %, Focus +10 %, Press +10 %, Drag +16 %.
> _Enabled_ adds no overlay; _disabled_ reduces content/container opacity rather
> than compositing an overlay (its values live on per-component spec tables and
> the color-contrast guidance, not in the 4-overlay diagram).

## 5. Accessibility & usability

### 5.1 Color & contrast (the numbers)

Contrast ratio expresses how far two colours differ in relative luminance, on a
1:1 → 21:1 scale (W3C). Minimums:

| Element                                                 | Minimum contrast vs background |
| ------------------------------------------------------- | ------------------------------ |
| Large text (≥ 14 pt bold / 18 pt regular) and graphics  | **3:1**                        |
| Small / body text                                       | **4.5:1**                      |
| Non-text UI (e.g. button containers clustered together) | **3:1**                        |

- **Disabled states are exempt** from contrast minimums.
- **Clustered** elements (a row of buttons) each need 3:1 against the background
  so users can tell them apart.
- **Standalone** prominent elements (a FAB) are exempt from the 3:1
  container-vs-background rule because their prominence already distinguishes
  them.

### 5.2 Usability

Usability = intuitive and easy for everyone (distinct from accessibility, which
specifically serves disability and assistive tech). M3 leans on the Nielsen
Norman five aspects: **efficiency, errors, learnability, memorability,
satisfaction**.

Design tactics for a strong visual hierarchy: colour & contrast, containment &
grouping, motion (sparingly), shape & shape-morph, size (largest element = main
CTA), and typography. Design around **primary goals** (one primary task per
page), then **test and iterate** with real users.

### 5.3 Target sizes

- Standard interactive target: **48 dp** (state layer 40 dp).
- XR icon buttons: **56 dp** target with a **4 dp** offset (see §8).

## 6. Design tokens

Tokens replace hard-coded values with named, reusable design decisions shared
across design files, tools, and code. A token = a code-like **name** (e.g.
`md.ref.palette.secondary90`) + an **associated value** (e.g. `#E8DEF8`); a value
may itself point to another token.

**Name anatomy** — dot-separated, general → specific:
`md` (design system) · `ref` / `sys` / `comp` (class) · role description.

**Three token tiers:**

| Tier          | Prefix | Role                                                                                                              |
| ------------- | ------ | ----------------------------------------------------------------------------------------------------------------- |
| **Reference** | `ref`  | Every available raw value (a hex, a font, a measurement). Context-independent.                                    |
| **System**    | `sys`  | The theming layer — assigns purpose/roles; points at reference tokens and can swap them per context (light/dark). |
| **Component** | `comp` | Per-element component properties; should point at sys/ref tokens, not raw values.                                 |

Example chain: `md.comp.fab.primary.container.color` → a `sys` colour role →
a `ref` palette token → a resolved hex. Changing the hex never changes the name.

**Contexts** are conditions (dark theme, dense layout, RTL, form factor) that
make a token resolve to a different value — like a tag overriding the default.

> aphrody implements the **state** slice of the sys tier in
> [`crates/m3-tokens/src/state.rs`](../../crates/m3-tokens/src/state.rs); see §10.

## 7. Customization (dynamic color & branding)

M3 lets brand colour and a user's personal colour preference coexist. **Dynamic
color** derives an accessible scheme from a source colour while preserving brand
identity; it also handles contrast, legibility, interaction states, and works on
non-Material components.

- Build a **custom color scheme** + a **custom theme** as a fallback for users
  who don't enable dynamic color.
- The **Material Theme Builder** (Figma plugin) generates colour + type tokens
  and exports to multiple code formats (incl. the cross-platform DSP format).
- **Five key colour groups** seed the tonal palettes: **primary, secondary,
  tertiary, neutral, neutral-variant** — each input expands into roles like
  `primary`, `on-primary`, `primary-container`.

## 8. Adaptive surfaces — watches & XR (brief)

### Watches (Wear OS, M3 Expressive)

- **Round-screen shape system** — edge-hugging containers/buttons for balance on
  circular displays.
- **Shape morph + physics-based motion** — controls change corner radius to
  signal interaction; transitions feel fluid.
- **Rich color** — dynamic color across **26 standard color roles** in six groups
  (primary, secondary, tertiary, error, surface, outline).
- **New type roles** — arc text for curved titles, numeral text for big stylised
  figures; baseline scale optimised for compact round screens.

### XR (Android XR)

Principles: reuse **familiar Material patterns**, **prioritise comfort** (centre
the field of view; design seated/standing/reclined), **embrace depth** (elevation

- 3D models), and **design for accessibility** (screen readers, voice, resizable
  text, multimodal input). UI groups onto floating **spatial panels**; feedback via
  spatial audio, haptics, and visual cues. XR icon buttons use a **56 dp** target +
  **4 dp** offset. Key terms: home space vs full space, passthrough, orbiters,
  spatial elevation, spatial environments.

## 9. Building for all

Teams can't embody every user, so collaborate with communities, experts, and
researchers — especially those usually overlooked. Keep an evolving list of
experience dimensions in mind across research, testing, and marketing: age,
culture, disability, education/literacy, ethnicity, gender, geography, physical
attributes, race, religion, sexual orientation, socioeconomic status, and tech
proficiency.

---

## 10. → aphrody

| M3 foundation                              | aphrody artifact                                                                             | Notes                                                                                                                                                                 |
| ------------------------------------------ | -------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Interaction states / state-layer opacities | [`crates/m3-tokens/src/state.rs`](../../crates/m3-tokens/src/state.rs)                       | `HOVER 0.08`, `FOCUS 0.10`, `PRESSED 0.10`, `DRAGGED 0.16` (+ `DISABLED_CONTENT 0.38`, `DISABLED_CONTAINER 0.12`); CSS export `--md-sys-state-*-state-layer-opacity`. |
| Design tokens (ref/sys/comp tiers)         | [`m3-design-tokens.md`](m3-design-tokens.md), [`aphrody-m3-tokens.md`](aphrody-m3-tokens.md) | Token taxonomy + the `m3-tokens` crate.                                                                                                                               |
| Layout                                     | [`m3-layout.md`](m3-layout.md)                                                               | Scaffold, grids/spacing, breakpoints, RTL, canonical examples.                                                                                                        |
| Material A–Z terms                         | [`m3-glossary.md`](m3-glossary.md)                                                           | Glossary cross-link (state layer, on-color, dynamic color, tonal palette…).                                                                                           |
| Motion (usability tactic)                  | [`m3-motion.md`](m3-motion.md)                                                               | Expressive / physics-based motion detail.                                                                                                                             |

## Discrepancies

- **None on the four overlay opacities.** `state.rs` (Hover 0.08, Focus 0.10,
  Pressed 0.10, Dragged 0.16) matches the M3 _State layers_ page exactly.
- **Scope addition, not a conflict:** `state.rs` also defines
  `DISABLED_CONTENT = 0.38` and `DISABLED_CONTAINER = 0.12`. These two are _not_
  in the 4-overlay state-layers diagram (which covers only hover/focus/press/
  drag). M3 documents disabled opacity on per-component spec tables and the
  color-contrast guidance instead. The crate's own doc-comment already flags
  that disabled tokens "differ in nature" (they scale content/container opacity
  rather than compositing an overlay), so the grouping is intentional and
  internally documented — no code change recommended.
