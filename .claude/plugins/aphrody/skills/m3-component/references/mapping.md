# shadcn → Material Web 3 lookup table

Extracted from `docs/research/SHADCN_M3_MAPPING.md` for fast skill lookup.

| shadcn name | M3 tag(s) | Module import path | Variants |
|---|---|---|---|
| button | `md-filled-button`, `md-outlined-button`, `md-text-button`, `md-tonal-button`, `md-elevated-button`, `md-fab` | `@material/web/button/{filled,outlined,text,filled-tonal,elevated}-button.js`, `@material/web/fab/fab.js` | default→filled, outline→outlined, ghost→text, secondary→tonal, destructive→filled+error, link→text, elevated→elevated |
| checkbox | `md-checkbox` | `@material/web/checkbox/checkbox.js` | default |
| radio | `md-radio` | `@material/web/radio/radio.js` | default |
| switch | `md-switch` | `@material/web/switch/switch.js` | default |
| input | `md-outlined-text-field`, `md-filled-text-field` | `@material/web/textfield/{outlined,filled}-text-field.js` | default→outlined, filled→filled |
| textarea | `md-outlined-text-field type="textarea"` | `@material/web/textfield/outlined-text-field.js` | (set `type="textarea"` prop) |
| select | `md-outlined-select`, `md-filled-select` | `@material/web/select/{outlined,filled}-select.js` | default→outlined, filled→filled |
| slider | `md-slider` | `@material/web/slider/slider.js` | default |
| card | (use `<div>` + `md-elevation` + `md-ripple`) | `@material/web/elevation/elevation.js`, `@material/web/ripple/ripple.js` | elevated, filled, outlined |
| dialog | `md-dialog` | `@material/web/dialog/dialog.js` | default (use `type="alert"` for alert-dialog) |
| tabs | `md-tabs`, `md-primary-tab`, `md-secondary-tab` | `@material/web/tabs/{tabs,primary-tab,secondary-tab}.js` | primary→primary, secondary→secondary |
| badge | `md-badge` | `@material/web/labs/badge/badge.js` | default |
| dropdown-menu | `md-menu`, `md-menu-item`, `md-sub-menu` | `@material/web/menu/{menu,menu-item,sub-menu}.js` | default |
| progress | `md-linear-progress`, `md-circular-progress` | `@material/web/progress/{linear,circular}-progress.js` | linear→linear, circular→circular |
| separator | `md-divider` | `@material/web/divider/divider.js` | default |
| sonner / toast | (composite via `md-snackbar` — not in @material/web yet, custom impl required) | n/a | (flag as unsupported, fall back to sonner npm) |
| navigation-menu | `md-navigation-bar`, `md-navigation-rail`, `md-navigation-drawer` | `@material/web/labs/navigationbar/{,...}.js` | bar→bar, rail→rail, drawer→drawer |
| icon-button | `md-icon-button`, `md-filled-icon-button`, `md-outlined-icon-button`, `md-filled-tonal-icon-button` | `@material/web/iconbutton/{icon-button,filled-icon-button,outlined-icon-button,filled-tonal-icon-button}.js` | default→icon, filled→filled, outlined→outlined, tonal→filled-tonal |

## Unsupported by Material Web 3 (flag and stop)

These shadcn components have **no clean M3 equivalent** — `m3-component`
must refuse to scaffold them. The user should either keep the legacy
shadcn version or write a custom impl:

- `accordion` (M3 uses list-expand patterns, not a dedicated component)
- `alert` (use `md-dialog` with `type="alert"` or build with primitives)
- `aspect-ratio` (CSS-only, no M3 component)
- `avatar` (no `md-avatar`, use `<md-icon>` inside `<div>`)
- `breadcrumb` (no M3 component, use `md-text-button` + separators)
- `calendar` (no `md-date-picker`, only experimental)
- `carousel` (no M3 component)
- `chart` (out of scope)
- `collapsible` (use `<details>` or build with primitives)
- `combobox` (no M3 combobox, build from `md-menu` + `md-outlined-text-field`)
- `command` (no M3 equivalent)
- `context-menu` (use `md-menu` + manual position)
- `direction` (RTL is a layout concern, not a component)
- `drawer` (use `md-navigation-drawer`)
- `empty` (composition, no M3 element)
- `field` (composition with `md-outlined-text-field`)
- `hover-card` (no M3 equivalent)
- `input-group` / `input-otp` (composition)
- `item` (composition)
- `kbd` (CSS-only)
- `menubar` (composition)
- `pagination` (composition with icon-buttons)
- `popover` (use `md-menu` or custom)
- `resizable` (no M3 component)
- `scroll-area` (native CSS)
- `sheet` (use `md-dialog` full-screen)
- `sidebar` (use `md-navigation-rail` or `md-navigation-drawer`)
- `skeleton` (CSS-only)
- `spinner` (use `md-circular-progress`)
- `table` (no M3 component, native HTML)
- `toggle` / `toggle-group` (no `md-toggle`; use `md-icon-button` with state)
- `tooltip` (no `md-tooltip`, custom impl)
