<!-- SPDX-License-Identifier: Apache-2.0 -->
# Material 3 styles

Reference digest of the Material Design 3 *Styles* section, paraphrased in our own
words from the live SPA at <https://m3.material.io/styles> (color, typography,
elevation, shape, motion, icons). Numeric/dp values were transcribed from the
fetched pages and cross-checked against `crates/m3-tokens`. Not a copy of the
spec; consult the source for canonical wording and imagery.

The six top-level areas linked from the index are **Color**, **Elevation**,
**Icons**, **Motion**, **Shape**, and **Typography** (plus a **Spacing** page).

---

## Color system

M3 color is a token-driven system where every UI element is "painted by number":
each element maps to a semantic **color role**, and each role resolves to a tone
drawn from a generated palette. The pipeline runs source color → 5 key colors →
tonal palettes → roles → UI.

### HCT color space

The system manipulates color in **HCT** (Hue, Chroma, Tone):

- **Hue** — the red/orange/…/violet identity, on a 0–360 circular degree scale
  (0 and 360 coincide).
- **Chroma** — colorfulness vs. neutral grey; runs 0 (neutral) up to roughly 120
  at its practical ceiling. Achievable chroma varies by hue and tone.
- **Tone** — perceived lightness (luminance), 0 (black) to 100 (white). Tone drives
  contrast, so HCT lets you change hue/chroma while holding tone (and thus contrast)
  steady — something HSL and RGB cannot do cleanly.

### Key colors and tonal palettes

A single source color (from wallpaper quantization, in-app content, or a hand-pick)
is fed through the Material Color Utilities (MCU) algorithm to derive **five key
colors**: primary, secondary, tertiary, neutral, and neutral variant. Each key color
expands into a **tonal palette** — a family at one hue/chroma sampled across tone
stops. Stops are 0–100 in increments of 10, plus the extra near-white stops 95, 98,
and 99 (some palettes carry additional values). Roles are aliases into these palettes;
e.g. the `primary` role takes primary tone 40 in light theme, `on primary` takes
primary tone 100.

### Color roles

Roles are the connective tissue between palette tones and UI elements; they are
tokenized and guarantee accessible pairings (paired fills and "on" colors hold a
minimum 3:1 contrast). Naming conventions:

- **Surface** — neutral backgrounds and large low-emphasis areas.
- **Primary / Secondary / Tertiary** — accent roles ranked by needed emphasis
  (primary = most prominent like a FAB; secondary = recessive like filter chips;
  tertiary = contrasting accents and special highlights).
- **Container** — fill color for foreground elements (buttons), never for text/icons.
- **On…** — text/icon color sitting on top of the paired parent (e.g. `on primary`).
- **…variant** — lower-emphasis sibling of a role (e.g. `outline variant`).

Role groups documented:

- **Accent groups** (primary/secondary/tertiary), each with role, on-role,
  container, and on-container (4 roles each = 12).
- **Error** — static by default even under dynamic color, still flips light/dark:
  error, on error, error container, on error container.
- **Surface** — surface, on surface, on surface variant, plus five emphasis-ranked
  containers: surface container lowest / low / (default) / high / highest.
- **Inverse** — inverse surface, inverse on surface, inverse primary (e.g. snackbars).
- **Outline** — outline (text-field borders, 3:1 boundaries) and outline variant
  (dividers, decorative).
- **Add-on roles** (most products skip): fixed and fixed-dim accents (hold the same
  tone across light/dark), their on-fixed / on-fixed-variant pairs, and the
  bright/dim surface roles. `surface dim`/`surface bright` keep relative brightness
  across themes rather than inverting like `surface`.

The role count quoted by the spec ranges from "26 standard" up to "45" once add-on
roles are included.

### Schemes

- **Static / baseline** — the default hand-picked scheme (M3 reference Purple seed),
  shipping light + dark; the recommended M2→M3 migration starting point.
- **Dynamic** — generated at runtime from user wallpaper (user-generated) or in-app
  content (content-based), yielding personalized, accessible, auto-dark schemes.

### Contrast levels

Roles support three tokenized contrast levels: **standard** (default, mixed contrast),
**medium** (≥3:1), and **high** (≥7:1). Applied automatically across light and dark.

---

## Typography

A **type scale** is a curated set of styles for product-wide consistency. M3 ships
**one** scale with two style sets of 15 each — **baseline** and **emphasized**
(emphasized added in the M3 Expressive update; heavier weight, for selection/headline
emphasis) — for 30 tokens total. Both sets span Display Large down to Label Small.

### The 15 baseline roles

Five families, each Large/Medium/Small:

| Role            | size (dp) | line-height (dp) | tracking (dp) | weight |
|-----------------|-----------|------------------|---------------|--------|
| Display Large   | 57        | 64               | -0.25         | 400    |
| Display Medium  | 45        | 52               | 0             | 400    |
| Display Small   | 36        | 44               | 0             | 400    |
| Headline Large  | 32        | 40               | 0             | 400    |
| Headline Medium | 28        | 36               | 0             | 400    |
| Headline Small  | 24        | 32               | 0             | 400    |
| Title Large     | 22        | 28               | 0             | 400    |
| Title Medium    | 16        | 24               | 0.15          | 500    |
| Title Small     | 14        | 20               | 0.1           | 500    |
| Body Large      | 16        | 24               | 0.5           | 400    |
| Body Medium     | 14        | 20               | 0.25          | 400    |
| Body Small      | 12        | 16               | 0.4           | 400    |
| Label Large     | 14        | 20               | 0.1           | 500    |
| Label Medium    | 12        | 16               | 0.5           | 500    |
| Label Small     | 11        | 16               | 0.5           | 500    |

(These dp values are mirrored exactly in `crates/m3-tokens/typography.rs`.)

### Fonts and customization

Two typeface slots: a **brand** face for large styles (Display/Headline) and a
**plain** face for small styles (Body/Label). Roboto is the spec default for both
(Roboto Flex suggested as a variable-axis swap). The scale derives from the
**Major Second** ratio (≈1.125) anchored to a 14 base size. Font-size units: `sp` on
Android, `rem` on web (sp/16 = rem at the 16px browser default).

---

## Elevation

Elevation is z-axis distance between surfaces, in dp. M3 tokenizes **six levels,
0–5**. Tokens carry only the dp height — no intrinsic shadow or color; each platform
chooses how to render it. Elevation can be expressed via **tonal surface color**
(preferred in M3) or **shadow** (used only when extra separation/affordance is needed),
a shift from M2 which applied shadows at every level.

| Level | dp |
|-------|-----|
| 0     | 0   |
| 1     | 1   |
| 2     | 3   |
| 3     | 6   |
| 4     | 8   |
| 5     | 12  |

Default resting levels per the component table: level 0 (filled/tonal/outlined
buttons, cards, chips, tabs, nav rail, lists), level 1 (banners, modal bottom sheets,
elevated buttons/cards/chips, modal nav drawer), level 2 (scrolled app bar, menus,
navigation bar, rich tooltip, toolbar), level 3 (FAB/extended FAB, modal dialogs,
date/time pickers, search). Hover/focus typically raises a component one level.
Surface tint color is deprecated — use the 0–5 level tokens.

---

## Shape

Shape controls corner roundedness on rectangular containers. The **current** M3
shape scale (Expressive update) is a **size-based scale with ten styles**:

| # | Style                  | dp            |
|---|------------------------|---------------|
| 1 | None                   | 0             |
| 2 | Extra small            | 4             |
| 3 | Small                  | 8             |
| 4 | Medium                 | 12            |
| 5 | Large                  | 16            |
| 6 | Large increased        | 20            |
| 7 | Extra large            | 28            |
| 8 | Extra large increased  | 32            |
| 9 | Extra extra large      | 48            |
| 10| Full                   | fully rounded |

Shapes can be **symmetric** (all corners equal) or **asymmetric** (per-corner, used
for grouped items like menus and split buttons via "inner corner" tokens). The family
can be customized from **rounded** to **cut** (straight chamfer). Optical-roundness
guidance for nesting: inner radius = outer radius − padding. M3 also adds a shape
library with morphing (Android Compose API; web not yet available), driving the
standard button group and loading indicator.

> Note: the previous M3 scale (still embedded in `m3-tokens`) was a **7-step** scale
> (none/extra-small/small/medium/large/extra-large/full at 0/4/8/12/16/28/9999dp).
> See Discrepancies.

---

## Motion

Motion tokens split into **easing** (timing curves) and **duration** (ms values).

### Easing sets

Two sets of three. The CSS columns from the spec:

| Token                          | CSS cubic-bezier                  |
|--------------------------------|-----------------------------------|
| emphasized                     | N/A (fall back to standard)       |
| emphasized.decelerate          | cubic-bezier(0.05, 0.7, 0.1, 1.0) |
| emphasized.accelerate          | cubic-bezier(0.3, 0.0, 0.8, 0.15) |
| standard                       | cubic-bezier(0.2, 0.0, 0, 1.0)    |
| standard.decelerate            | cubic-bezier(0, 0, 0, 1)          |
| standard.accelerate            | cubic-bezier(0.3, 0, 1, 1)        |

The **emphasized** base curve has no single cubic-bezier on web — it is a multi-segment
path interpolator on Android, and the spec says use **standard** as the web fallback.
The **emphasized** set is the expressive default; **standard** is for small/utility
transitions. (Linear is not in the spec's two easing tables but is a common token.)

### Duration tokens (16)

| Group       | Tokens → ms                                    |
|-------------|------------------------------------------------|
| short       | short1 50, short2 100, short3 150, short4 200  |
| medium      | medium1 250, medium2 300, medium3 350, medium4 400 |
| long        | long1 450, long2 500, long3 550, long4 600     |
| extra-long  | extra-long1 700, extra-long2 800, extra-long3 900, extra-long4 1000 |

Short = small utility moves; medium = mid-screen traversals; long = large expressive
transitions (often with emphasized easing); extra-long = rare ambient/non-input motion.

---

## Icons — Material Symbols

Material Symbols are **variable icon fonts**: created at seven weights across three
styles (outlined, rounded, sharp). Key design parameters:

- **Sizes (optical):** baseline 24dp; additional 20dp (dense/desktop), 40dp and 48dp
  (display/headline, larger screens). Design on a 24dp grid at 100% scale.
- **Layout:** content stays within a 20×20dp **live area** with 2dp padding to the
  24dp **trim area**; nothing exits the trim area.
- **Grid keylines:** square 18dp, circle 20dp diameter, vertical rectangle 20×16dp,
  horizontal rectangle 16×20dp. Place icons "on pixel."
- **Corners:** default 2dp radius. Outlined style → square interior corners; rounded
  style → rounded interior + exterior; sharp style → corners reduced 2dp→0dp.
- **Weight axis:** stroke is 2dp at regular (**400**); the weight axis ranges
  **thin (100) → bold (700)**. Complex icons may use 1.5dp strokes as optical
  correction. (Symbols also expose **fill** and **grade** axes as variable-font axes.)

---

## → aphrody m3-tokens mapping

Grounded in the real exported names read from `crates/m3-tokens/src/*.rs`.

| M3 style area              | aphrody item (file → symbol)                                                             |
|----------------------------|-------------------------------------------------------------------------------------------|
| Color roles                | `color.rs` → `struct ColorRoles` (36 fields) + consts `BASELINE`, `BASELINE_DARK`, `APHRODY`, `APHRODY_DARK`; `export_css`, `color_vars`, `FUSION_ALIAS_MAP` |
| Tonal palettes / key colors| `tonal.rs` → `struct Palette`, `struct Tone`, `TONE_VALUES` (13 stops), `PRIMARY_BASELINE` … `NEUTRAL_VARIANT_BASELINE`, `ALL_BASELINE` (6 palettes) |
| HCT / dynamic color        | `hct.rs`, `dynamic.rs` (HCT space + scheme generation; not deeply inspected here)         |
| Typography (15 roles)      | `typography.rs` → `struct TypeStyle`, `DISPLAY_LARGE` … `LABEL_SMALL`, `ALL` (15), `NAMES`, `export_css` |
| Elevation (levels 0–5)     | `elevation.rs` → `LEVEL_COUNT` (6), `LEVELS` [(dp, box-shadow)], `dp(level)`, `shadow(level)` |
| Shape (corner scale)       | `shape.rs` → `struct CornerRadius`, `NONE`/`EXTRA_SMALL`/`SMALL`/`MEDIUM`/`LARGE`/`EXTRA_LARGE`/`FULL`, `ALL` (7), `NAMES`, `export_css` |
| Motion durations           | `motion.rs` → `struct Duration`, `DURATION_SHORT1` … `DURATION_EXTRA_LONG4`, `ALL_DURATIONS` (16), `PRIMARY_DURATIONS_MS` |
| Motion easings             | `motion.rs` → `struct Easing`, `EASING_EMPHASIZED…`, `EASING_STANDARD…`, `EASING_LINEAR`, `ALL_EASINGS` (7) |
| Icons (Material Symbols)    | *no crate module* — `m3-tokens` does not model the Symbols axes (weight/fill/grade/optical size); also `state.rs`, `adaptive.rs`, `gemini_brand.rs`, `google_sans_*` exist beyond the core five style areas |

### Discrepancies vs m3-tokens

Described only — no code edited.

1. **Shape scale is stale (7 vs 10 steps).** `shape.rs` implements the older M3
   7-step scale (`none/extra-small/small/medium/large/extra-large/full` =
   0/4/8/12/16/28/9999dp). The current spec (corner-radius-scale page) defines a
   **10-step** scale and adds three styles: **large increased (20dp)**,
   **extra large increased (32dp)**, and **extra extra large (48dp)**. The crate is
   missing those three tokens. The crate's `extra-large` (28dp) and `full` still match.
   The `FULL` sentinel uses `9999` dp where the spec just says "fully rounded."

2. **Motion `emphasized` easing cubic-bezier diverges from spec.** `motion.rs`
   `EASING_EMPHASIZED` is `cubic-bezier(0.2, 0.0, 0, 1.0)` — identical to
   `EASING_STANDARD`. That matches the spec's *guidance* (web has no single bezier for
   emphasized; use standard as fallback), but it is not the true emphasized path
   interpolator. Worth a code comment so it is not mistaken for the canonical curve.

3. **Tonal palette tones partly unverified.** `tonal.rs` carries `TODO: verify`
   markers on several mid-range tones (50/60/70/95/99) of the secondary, tertiary,
   neutral, and neutral-variant palettes — values were interpolated, not confirmed
   against `material-color-utilities` output. The verified anchors (primary tone 40 =
   `#6750A4`, error palette) are correct.

4. **No Icons / Material Symbols module.** The Symbols axes (weight 100–700, fill,
   grade, optical sizes 20/24/40/48dp) have no representation in `m3-tokens`. Out of
   scope for a token crate, but the mapping table has a gap here by design.

5. **Color role count.** `ColorRoles` exposes 36 fields (the standard set incl.
   expanded tone-based surfaces). The spec's full role inventory reaches ~45 once the
   add-on fixed / fixed-dim / on-fixed accent roles are counted; those add-on roles are
   intentionally not modeled. Not a bug — a documented scope choice.
