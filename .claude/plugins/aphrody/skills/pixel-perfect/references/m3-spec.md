# Material Design 3 — distilled contract

Reference subset of the M3 spec used by the `pixel-perfect` skill. Derived
from `docs/research/SHADCN_M3_MAPPING.md` (agent Explore, 2026-05-17) and
cross-checked against `https://m3.material.io` (sections cited inline).
Sources: `m3.material.io/{components,styles,foundations}` and
`github.com/material-components/material-web`.

This file is intentionally factual and short. The skill loads it in full;
do not exceed 400 lines.

---

## 1. Color system (HCT, dynamic color)

M3 colors are defined in HCT (Hue/Chroma/Tone) and expressed as CSS
custom properties. Every component MUST consume colors through these
tokens; raw hex is forbidden.

Source: `m3.material.io/styles/color/system/overview`.

### Surface / on-surface

| Token | Role |
|---|---|
| `--md-sys-color-background` | App background |
| `--md-sys-color-on-background` | Text on background |
| `--md-sys-color-surface` | Default surface |
| `--md-sys-color-on-surface` | Text/icons on surface |
| `--md-sys-color-surface-variant` | Tonal surface variant |
| `--md-sys-color-on-surface-variant` | Text on variant |
| `--md-sys-color-surface-container-lowest` | Card/elev 1 |
| `--md-sys-color-surface-container-low` | Card/elev 2 |
| `--md-sys-color-surface-container` | Card/elev 3 |
| `--md-sys-color-surface-container-high` | Card/elev 4 |
| `--md-sys-color-surface-container-highest` | Card/elev 5 |

### Accent roles

| Token | Role |
|---|---|
| `--md-sys-color-primary` / `-on-primary` | Primary action |
| `--md-sys-color-primary-container` / `-on-primary-container` | Primary tonal surface |
| `--md-sys-color-secondary` / `-on-secondary` | Secondary action |
| `--md-sys-color-secondary-container` / `-on-secondary-container` | Secondary tonal |
| `--md-sys-color-tertiary` / `-on-tertiary` | Tertiary accent |
| `--md-sys-color-tertiary-container` / `-on-tertiary-container` | Tertiary tonal |
| `--md-sys-color-error` / `-on-error` | Error |
| `--md-sys-color-error-container` / `-on-error-container` | Error tonal |
| `--md-sys-color-outline` | Outlined component border |
| `--md-sys-color-outline-variant` | Subdued divider |
| `--md-sys-color-inverse-surface` / `-inverse-on-surface` | Snackbar |
| `--md-sys-color-inverse-primary` | Inverse accent |
| `--md-sys-color-scrim` | Scrim under modals |
| `--md-sys-color-shadow` | Shadow color |

A component is *color-compliant* iff every `color`, `background-color`,
`border-color`, `fill`, and `stroke` declaration resolves to one of the
tokens above.

## 2. Typography (adaptive type scale)

Source: `m3.material.io/styles/typography/type-scale-tokens`.

Each scale token expands to `font`, `font-weight`, `letter-spacing`, and
`line-height` properties.

### Display / Headline / Title / Body / Label

| Family | Sizes |
|---|---|
| Display | `large`, `medium`, `small` |
| Headline | `large`, `medium`, `small` |
| Title | `large`, `medium`, `small` |
| Body | `large`, `medium`, `small` |
| Label | `large`, `medium`, `small` |

Tokens:

- `--md-sys-typescale-display-large-font`
- `--md-sys-typescale-display-large-size`
- `--md-sys-typescale-display-large-line-height`
- `--md-sys-typescale-display-large-tracking`
- `--md-sys-typescale-display-large-weight`

…and the same five suffixes for every {family, size} combination above
(15 combinations × 5 props = 75 tokens). The skill verifies that any
font-sizing declaration in a component sets at least the `-size` and
`-weight` from the matching token row.

## 3. Shape (corner radius)

Source: `m3.material.io/styles/shape/overview`.

| Token | Default value (dp) |
|---|---|
| `--md-sys-shape-corner-none` | 0 |
| `--md-sys-shape-corner-extra-small` | 4 |
| `--md-sys-shape-corner-small` | 8 |
| `--md-sys-shape-corner-medium` | 12 |
| `--md-sys-shape-corner-large` | 16 |
| `--md-sys-shape-corner-large-end` | 16 (asymmetric) |
| `--md-sys-shape-corner-extra-large` | 28 |
| `--md-sys-shape-corner-full` | 9999 |

Rule: every `border-radius` in an M3 component MUST be one of these
tokens (or a Material Web internal that resolves to one).

## 4. Motion

Source: `m3.material.io/styles/motion/overview`.

### Durations

| Token | ms |
|---|---|
| `--md-sys-motion-duration-short1` | 50 |
| `--md-sys-motion-duration-short2` | 100 |
| `--md-sys-motion-duration-short3` | 150 |
| `--md-sys-motion-duration-short4` | 200 |
| `--md-sys-motion-duration-medium1` | 250 |
| `--md-sys-motion-duration-medium2` | 300 |
| `--md-sys-motion-duration-medium3` | 350 |
| `--md-sys-motion-duration-medium4` | 400 |
| `--md-sys-motion-duration-long1` | 450 |
| `--md-sys-motion-duration-long2` | 500 |
| `--md-sys-motion-duration-long3` | 550 |
| `--md-sys-motion-duration-long4` | 600 |
| `--md-sys-motion-duration-extra-long1` | 700 |
| `--md-sys-motion-duration-extra-long2` | 800 |
| `--md-sys-motion-duration-extra-long3` | 900 |
| `--md-sys-motion-duration-extra-long4` | 1000 |

### Easings (cubic-bezier)

| Token | Value |
|---|---|
| `--md-sys-motion-easing-linear` | `linear` |
| `--md-sys-motion-easing-standard` | `cubic-bezier(0.2, 0, 0, 1)` |
| `--md-sys-motion-easing-standard-accelerate` | `cubic-bezier(0.3, 0, 1, 1)` |
| `--md-sys-motion-easing-standard-decelerate` | `cubic-bezier(0, 0, 0, 1)` |
| `--md-sys-motion-easing-emphasized` | `cubic-bezier(0.2, 0, 0, 1)` |
| `--md-sys-motion-easing-emphasized-accelerate` | `cubic-bezier(0.3, 0, 0.8, 0.15)` |
| `--md-sys-motion-easing-emphasized-decelerate` | `cubic-bezier(0.05, 0.7, 0.1, 1)` |

Rule: every `transition-duration` / `animation-duration` resolves to a
duration token; every `transition-timing-function` /
`animation-timing-function` resolves to an easing token. Inline
`cubic-bezier(...)` literals fail the audit.

## 5. Elevation (light & dark)

Source: `m3.material.io/styles/elevation/overview`.

Surface tint + shadow encoded together. Tokens:

| Level | Token |
|---|---|
| 0 | `--md-sys-elevation-level0` |
| 1 | `--md-sys-elevation-level1` |
| 2 | `--md-sys-elevation-level2` |
| 3 | `--md-sys-elevation-level3` |
| 4 | `--md-sys-elevation-level4` |
| 5 | `--md-sys-elevation-level5` |

Default dp values (z): 0, 1, 3, 6, 8, 12.

Box-shadow rule: components MUST consume `--md-sys-elevation-level{n}`,
either directly (`box-shadow: var(--md-sys-elevation-level3)`) or via
the Material Web `<md-elevation>` internal element.

## 6. State layers (interaction overlays)

Opacity multipliers applied on the matching `on-*` color:

| State | Opacity token | Default |
|---|---|---|
| Hover | `--md-sys-state-hover-state-layer-opacity` | 0.08 |
| Focus | `--md-sys-state-focus-state-layer-opacity` | 0.10 |
| Pressed | `--md-sys-state-pressed-state-layer-opacity` | 0.10 |
| Dragged | `--md-sys-state-dragged-state-layer-opacity` | 0.16 |

Components MUST emit a state layer via Material Web's `<md-ripple>` or
equivalent — naked hover styles via `:hover { opacity: 0.X }` fail.

## 7. Spacing / sizing

M3 spacing is on a 4 dp grid. There is no `--md-sys-spacing-*` token
family in the upstream spec yet (as of 2026-05-17), so the skill checks
that every `margin` / `padding` / `gap` / `top` / `left` etc. is either:

- a multiple of `4px` in literal form, or
- a CSS custom property whose computed value is a multiple of 4.

Components MAY define `--md-comp-<name>-*` custom properties for
per-component sizing; those are allowed if they ultimately resolve to
a 4 dp multiple.

## 8. Component → Material Web element table

Subset used most often by the audit (full table:
`docs/research/SHADCN_M3_MAPPING.md` §2).

| Family | Element(s) | Variants |
|---|---|---|
| button | `<md-filled-button>`, `<md-outlined-button>`, `<md-text-button>`, `<md-elevated-button>`, `<md-tonal-button>`, `<md-fab>` | filled / outlined / text / elevated / tonal / fab |
| checkbox | `<md-checkbox>` | n/a |
| radio | `<md-radio>` | n/a |
| switch | `<md-switch>` | n/a |
| text-field | `<md-outlined-text-field>`, `<md-filled-text-field>` | outlined / filled |
| select | `<md-outlined-select>`, `<md-filled-select>` | outlined / filled |
| slider | `<md-slider>` | n/a |
| dialog | `<md-dialog>` | n/a |
| navigation | `<md-navigation-drawer>`, `<md-navigation-bar>`, `<md-navigation-rail>` | drawer / bar / rail |
| tabs | `<md-tabs>` + `<md-primary-tab>` / `<md-secondary-tab>` | primary / secondary |
| badge | `<md-badge>` | n/a |
| progress | `<md-linear-progress>`, `<md-circular-progress>` | linear / circular |
| menu | `<md-menu>` + `<md-menu-item>` | n/a |
| chip | `<md-assist-chip>`, `<md-filter-chip>`, `<md-input-chip>`, `<md-suggestion-chip>` | assist / filter / input / suggestion |
| divider | `<md-divider>` | n/a |
| snackbar | `<md-snackbar>` | n/a |
| icon | `<md-icon>` | n/a |
| icon-button | `<md-icon-button>`, `<md-outlined-icon-button>`, `<md-filled-icon-button>`, `<md-filled-tonal-icon-button>` | standard / outlined / filled / tonal |

## 9. Canonical reference URLs (per family)

Used by the visual-diff step of the workflow.

| Family | URL |
|---|---|
| button | https://m3.material.io/components/buttons/overview |
| checkbox | https://m3.material.io/components/checkbox/overview |
| radio | https://m3.material.io/components/radio-button/overview |
| switch | https://m3.material.io/components/switch/overview |
| text-field | https://m3.material.io/components/text-fields/overview |
| select | https://m3.material.io/components/menus/overview |
| slider | https://m3.material.io/components/sliders/overview |
| dialog | https://m3.material.io/components/dialogs/overview |
| navigation | https://m3.material.io/components/navigation-drawer/overview |
| tabs | https://m3.material.io/components/tabs/overview |
| badge | https://m3.material.io/components/badges/overview |
| progress | https://m3.material.io/components/progress-indicators/overview |
| menu | https://m3.material.io/components/menus/overview |
| chip | https://m3.material.io/components/chips/overview |
| divider | https://m3.material.io/components/divider/overview |
| snackbar | https://m3.material.io/components/snackbar/overview |
| icon-button | https://m3.material.io/components/icon-buttons/overview |

## 10. Disallowed patterns

Any of the following short-circuits the audit to FAIL:

1. Importing from `@radix-ui/*` or `class-variance-authority` in a
   component that claims M3 compliance.
2. Tailwind utility classes that set colors (`bg-*`, `text-*`,
   `border-*`) bypassing `--md-sys-color-*`.
3. Inline `style={{ color: '#...' }}` literals.
4. `cubic-bezier(...)`, `ease-in-out`, or numeric `transition: 0.3s`
   instead of motion tokens.
5. `box-shadow:` literals instead of elevation tokens.
6. Hard-coded font stacks (`font-family: 'Roboto', sans-serif`) — M3
   uses the type scale font tokens.
