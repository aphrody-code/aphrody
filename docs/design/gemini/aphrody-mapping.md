<!-- SPDX-License-Identifier: Apache-2.0 -->
# Gemini web app — theme import (pixel-exact)

The Gemini web app (<https://gemini.google.com/app>) ships a Material 3 token
system under the `--gem-sys-*` namespace, plus a canvas/code layer under
`--lumi-sys-*`. This note records the **dark theme** as captured live on
2026-05-22 by reading the cascaded computed values of those custom properties
(functional token values — colours and corner radii).

- Full token sheet: [`tokens-system.css`](tokens-system.css) (175 system tokens).
- Captured body context: `dark-theme zero-state-theme`; body background
  `#0f0f0f` (= `--lumi-sys-color--surface`), body text `#e3e3e3`.
- The app also defines ~1700 `--mat-*` Angular Material component tokens; those
  are *derived* from the system layer and are intentionally not vendored.

## Defining palette at a glance (dark)

| Role | Gemini value |
|---|---|
| primary | `#a8c7fa` |
| primary-container | `#1f3760` |
| secondary | `#7fcfff` |
| tertiary | `#6dd58c` |
| surface | `#131314` |
| surface-container | `#1e1f20` |
| surface-container-high(est) | `#282a2c` / `#333537` |
| on-surface | `#e3e3e3` |
| on-surface-variant | `#c4c7c5` |
| outline / outline-variant | `#8e918f` / `#444746` |
| error | `#f2b8b5` |
| brand text gradient | `#4285f4 → #9b72cb` |
| brand blue / green / red / yellow | `#3186ff` / `#0ebc5f` / `#ff4641` / `#ffcc00` |

The corner-radius scale (`--gem-sys-shape--corner-*`) matches the M3 Expressive
10-step scale now in [`m3-tokens` `shape.rs`](../../../crates/m3-tokens/src/shape.rs)
(none 0 → medium 12 → large 16 → large-increased 20 → extra-large 28 →
extra-large-increased 32 → extra-extra-large 48 → full 9999), confirming our
token update against a shipping Google product.

## Mapping to aphrody `m3-tokens` `ColorRoles`

Drop-in dark scheme for `crates/m3-tokens/src/color.rs` (ARGB, opaque). The
values are the captured `--gem-sys-color--*` roles; `#035` shorthand is expanded
to `#003355`, `background`/`on_background` mirror `surface`/`on_surface`.

```rust
/// Gemini web app dark scheme, captured pixel-exact 2026-05-22.
pub const GEMINI_DARK: ColorRoles = ColorRoles {
    primary: 0xFFA8C7FA,
    on_primary: 0xFF062E6F,
    primary_container: 0xFF1F3760,
    on_primary_container: 0xFFD3E3FD,
    secondary: 0xFF7FCFFF,
    on_secondary: 0xFF003355,
    secondary_container: 0xFF004A77,
    on_secondary_container: 0xFFC2E7FF,
    tertiary: 0xFF6DD58C,
    on_tertiary: 0xFF0A3818,
    tertiary_container: 0xFF0F5223,
    on_tertiary_container: 0xFFC4EED0,
    error: 0xFFF2B8B5,
    on_error: 0xFF601410,
    error_container: 0xFF601410,
    on_error_container: 0xFFF9DEDC,
    surface: 0xFF131314,
    on_surface: 0xFFE3E3E3,
    surface_variant: 0xFF444746,
    on_surface_variant: 0xFFC4C7C5,
    outline: 0xFF8E918F,
    outline_variant: 0xFF444746,
    background: 0xFF131314,
    on_background: 0xFFE3E3E3,
    inverse_surface: 0xFFE3E3E3,
    inverse_on_surface: 0xFF303030,
    inverse_primary: 0xFF0B57D0,
    scrim: 0xFF000000,
    shadow: 0xFF000000,
    // expanded surface tones
    surface_dim: 0xFF131314,
    surface_bright: 0xFF1E1F20,
    surface_container_lowest: 0xFF0E0E0E,
    surface_container_low: 0xFF1B1B1B,
    surface_container: 0xFF1E1F20,
    surface_container_high: 0xFF282A2C,
    surface_container_highest: 0xFF333537,
};
```

> Wiring this `const` into `color.rs` is deferred until the in-flight edits to
> that file settle (it is being revised concurrently); the snippet is exact and
> ready to paste. Field order/names follow the current 36-field `ColorRoles`.
