<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody — Material 3 design tokens

The aphrody GUI design system, expressed as Material 3 tokens. Derived from the
**Material 3 Design Kit** (Figma community file `1035203688168086460`, duplicated
as the local "Aphrody" file and exported to `.fig`) and the aphrody brand
(rust `#CE422B`, dark surfaces). Use these for the GUI built in
`apps/photoshop-uxp` / the Figma file and any future native UI.

> Source assets: the kit's 89 raster assets are extracted to
> `var/aphrody-fig/typed/` (gitignored). The kit's component catalog is the
> standard M3 set (Buttons, Chips, Cards, Dialogs, Navigation, FAB, Text fields,
> Switches, Sliders, Lists, Snackbars, Tabs, Top app bars, …).

## Color roles — dark scheme (default)

aphrody ships dark-first. Rust is the brand seed; surfaces are near-black inks.

| M3 role | Hex | Use |
|---|---|---|
| `primary` | `#E8836B` | primary actions, active states (rust seed lightened for dark-theme contrast) |
| `on-primary` | `#3A0E05` | text/icons on primary |
| `primary-container` | `#7A2517` | filled containers, emphasised chips |
| `on-primary-container` | `#FFDAD1` | text on primary-container |
| `brand-rust` | `#CE422B` | the literal brand accent (logo dot, Run button, dividers) |
| `secondary` | `#E7BDB2` | secondary accents |
| `tertiary` | `#E8C98A` | highlight / "hair gold" accent |
| `on-tertiary` | `#3F2E00` | text on tertiary |
| `error` | `#FFB4AB` | errors |
| `surface` | `#141414` | base app surface |
| `surface-dim` | `#0F0F0F` | window background (ink) |
| `surface-bright` | `#1E1E1E` | raised surfaces |
| `surface-container-lowest` | `#0C0C0C` | terminal/card panels |
| `surface-container` | `#202020` | title bar, input bar |
| `surface-container-high` | `#242424` | hovered/active fills |
| `on-surface` | `#F0F0F0` | primary text |
| `on-surface-variant` | `#8A8A8A` | muted text, labels, placeholders |
| `outline` | `#3A3A3A` | borders, dividers |
| `outline-variant` | `#2A2A2A` | subtle separators |
| `success` | `#7CCF8A` | terminal "ok" lines (aphrody extension, not core M3) |

## Type scale (M3, Roboto / Roboto Mono for code)

| Style | Size / line-height | Weight |
|---|---|---|
| Display Large | 57 / 64 | 400 |
| Display Medium | 45 / 52 | 400 |
| Display Small | 36 / 44 | 400 |
| Headline Large | 32 / 40 | 400 |
| Headline Medium | 28 / 36 | 400 |
| Headline Small | 24 / 32 | 400 |
| Title Large | 22 / 28 | 400 |
| Title Medium | 16 / 24 | 500 |
| Title Small | 14 / 20 | 500 |
| Body Large | 16 / 24 | 400 |
| Body Medium | 14 / 20 | 400 |
| Body Small | 12 / 16 | 400 |
| Label Large | 14 / 20 | 500 |
| Label Medium | 12 / 16 | 500 |
| Label Small | 11 / 16 | 500 |
| **Code** (aphrody) | 14 / 22 | 400 — **Roboto Mono** | terminal panel |

## Shape scale

| Token | Radius |
|---|---|
| `corner-none` | 0 |
| `corner-extra-small` | 4 |
| `corner-small` | 8 |
| `corner-medium` | 12 |
| `corner-large` | 16 |
| `corner-extra-large` | 28 |
| window / panels (aphrody) | 18 |

## Elevation (dark, tonal)

M3 dark uses surface-tint overlays rather than heavy shadows. aphrody panels
sit on `surface-container-*` tones (above) with an optional 1px `outline-variant`
border instead of a drop shadow; the mascot panel uses a 10%-opacity
`brand-rust` spotlight fill.

## Provenance

- Kit: Material 3 Design Kit (Google), Figma community, Apache-2.0 usage.
- Brand rust `#CE422B` + dark inks: aphrody (see the GUI built in the Figma
  "Aphrody" file and `apps/photoshop-uxp`).
- Full canonical M3 spec: <https://m3.material.io>. These tokens are the
  aphrody-tuned subset; defer to the spec for components not listed.
