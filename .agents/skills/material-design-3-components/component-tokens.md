# @material/web Component Token Prefixes

Quick reference for all `@material/web` component CSS custom property prefixes.
Generated from [`tokens/_index.scss`](https://github.com/material-components/material-web/blob/main/tokens/_index.scss).

Use these prefixes to override individual component tokens:

```css
/* Example: customize filled button */
md-filled-button {
  --md-filled-button-container-color: var(--md-sys-color-primary);
  --md-filled-button-container-shape: var(--md-sys-shape-corner-full);
}
```

## System Tokens

| Prefix | Category |
|--------|----------|
| `--md-sys-color-*` | Color roles (primary, secondary, surface, etc.) |
| `--md-sys-typescale-*` | Typography scale (display, headline, title, body, label) |
| `--md-sys-shape-*` | Shape corners (none, extra-small, small, medium, large, extra-large, full) |
| `--md-sys-elevation-*` | Elevation levels (0–5) |
| `--md-sys-motion-*` | Motion easing and duration tokens |
| `--md-sys-state-*` | State layer opacity (hover, focus, pressed, dragged) |
| `--md-ref-palette-*` | Reference palette tones (primary0–100, secondary0–100, etc.) |
| `--md-ref-typeface-*` | Typeface references (brand, plain) |

## Component Tokens

| Prefix | Component |
|--------|-----------|
| `--md-comp-assist-chip-*` | Assist chip |
| `--md-comp-badge-*` | Badge |
| `--md-comp-checkbox-*` | Checkbox |
| `--md-comp-circular-progress-*` | Circular progress |
| `--md-comp-dialog-*` | Dialog |
| `--md-comp-divider-*` | Divider |
| `--md-comp-elevated-button-*` | Elevated button |
| `--md-comp-elevated-card-*` | Elevated card |
| `--md-comp-elevation-*` | Elevation |
| `--md-comp-fab-*` | FAB |
| `--md-comp-fab-branded-*` | Branded FAB |
| `--md-comp-filled-button-*` | Filled button |
| `--md-comp-filled-card-*` | Filled card |
| `--md-comp-filled-field-*` | Filled field |
| `--md-comp-filled-icon-button-*` | Filled icon button |
| `--md-comp-filled-select-*` | Filled select |
| `--md-comp-filled-text-field-*` | Filled text field |
| `--md-comp-filled-tonal-button-*` | Filled tonal button |
| `--md-comp-filled-tonal-icon-button-*` | Filled tonal icon button |
| `--md-comp-filter-chip-*` | Filter chip |
| `--md-comp-focus-ring-*` | Focus ring |
| `--md-comp-full-screen-dialog-*` | Full screen dialog |
| `--md-comp-icon-*` | Icon |
| `--md-comp-icon-button-*` | Icon button |
| `--md-comp-input-chip-*` | Input chip |
| `--md-comp-item-*` | Item |
| `--md-comp-linear-progress-*` | Linear progress |
| `--md-comp-list-*` | List |
| `--md-comp-list-item-*` | List item |
| `--md-comp-menu-*` | Menu |
| `--md-comp-menu-item-*` | Menu item |
| `--md-comp-navigation-bar-*` | Navigation bar |
| `--md-comp-navigation-drawer-*` | Navigation drawer |
| `--md-comp-outlined-button-*` | Outlined button |
| `--md-comp-outlined-card-*` | Outlined card |
| `--md-comp-outlined-field-*` | Outlined field |
| `--md-comp-outlined-icon-button-*` | Outlined icon button |
| `--md-comp-outlined-segmented-button-*` | Outlined segmented button |
| `--md-comp-outlined-select-*` | Outlined select |
| `--md-comp-outlined-text-field-*` | Outlined text field |
| `--md-comp-primary-tab-*` | Primary tab |
| `--md-comp-radio-*` | Radio |
| `--md-comp-ripple-*` | Ripple |
| `--md-comp-secondary-tab-*` | Secondary tab |
| `--md-comp-slider-*` | Slider |
| `--md-comp-suggestion-chip-*` | Suggestion chip |
| `--md-comp-switch-*` | Switch |
| `--md-comp-text-button-*` | Text button |

## Resources

- @material/web theming guide: https://material-web.dev/theming/material-theming/
- Component API docs: https://material-web.dev/components/
- Source: https://github.com/material-components/material-web/tree/main/tokens
