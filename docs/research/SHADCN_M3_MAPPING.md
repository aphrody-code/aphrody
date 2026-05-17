# shadcn-ui ↔ Material Design 3 — Cartographie de refactor

> Document de recherche pour OBJECTIF #2 — refactor aphrody-code/ui@aphrody
> (fork shadcn-ui/ui) en library Material Design 3 NATIF via wrappers
> Material Web Components 3.
>
> Source : agent Explore (2026-05-17).

---

## 1. shadcn-ui — Inventaire (55 composants)

### Core Inputs & Forms (12)
button, button-group, checkbox, input, input-group, input-otp, native-select,
radio-group, select, switch, textarea, label

### Navigation & Layout (6)
breadcrumb, menubar, navigation-menu, pagination, sidebar, tabs

### Display & Content (15)
accordion, alert, alert-dialog, avatar, badge, card, carousel, chart, empty,
kbd, progress, separator, skeleton, spinner, table

### Overlays & Popovers (8)
context-menu, dialog, drawer, dropdown-menu, hover-card, popover, sheet, tooltip

### Selection & Grouping (8)
collapsible, combobox, command, direction, field, item, toggle, toggle-group

### Specialized (5)
aspect-ratio, calendar, resizable, scroll-area, sonner (toast)

### Architecture
- Registry-based v4 : `registry/bases/base/ui/{component}.tsx`
- Base UI + Radix UI + CVA (class-variance-authority)
- Tailwind CSS pour styling
- Composable slot-based architecture

---

## 2. Mapping shadcn ↔ M3 (`md-*` tags)

| shadcn/ui | Material Design 3 | Notes |
|---|---|---|
| button | `<md-filled-button>`, `<md-outlined-button>`, `<md-text-button>`, `<md-elevated-button>`, `<md-tonal-button>`, `<md-fab>` | M3 a 5+ variantes vs 6 shadcn |
| checkbox | `<md-checkbox>` | Direct |
| radio-group | `<md-radio>` | Direct |
| switch | `<md-switch>` | Direct |
| select | `<md-select>`, `<md-menu>` | Pattern dropdown |
| input | `<md-outlined-text-field>`, `<md-filled-text-field>` | 2 variantes |
| textarea | `<md-outlined-text-field type="textarea">` | Inclus dans text-field |
| slider | `<md-slider>` | Direct |
| card | `<md-card>` | Direct |
| dialog | `<md-dialog>` | Direct |
| navigation-menu | `<md-navigation-drawer>`, `<md-navigation-bar>`, `<md-navigation-rail>` | 3 layouts |
| tabs | `<md-tabs>`, `<md-tab>` | Direct |
| badge | `<md-badge>` | Direct |
| avatar | `<md-avatar>` | + `<md-icon>` |
| dropdown-menu | `<md-menu>`, `<md-menu-item>` | Direct |
| toast / sonner | `<md-snackbar>` | Direct |
| progress | `<md-linear-progress>`, `<md-circular-progress>` | 2 variantes |
| separator | `<md-divider>` | Direct |
| table | n/a (guidelines only) | Build via primitives |
| popover | n/a (menu/dialog patterns) | Composite |
| pagination | n/a (guidelines + `<md-icon-button>`) | Composite |
| hover-card | n/a (menu/popover patterns) | Composite |
| breadcrumb | n/a (guidelines + `<md-button>`) | Composite |
| collapsible | n/a (menu/list expand) | Composite |
| scroll-area | n/a (native scroll) | Native |

---

## 3. Design Tokens M3 à scraper

### Color System (HCT-based, Dynamic Color)
- `--md-sys-color-primary`, `-secondary`, `-tertiary`
- `--md-sys-color-error`, `-on-error`
- `--md-sys-color-surface`, `-on-surface`
- `--md-sys-elevation-*` (shadows)

### Typography (Adaptive Type Scale)
- `--md-sys-typescale-display-{large,medium,small}`
- `--md-sys-typescale-headline-{large,medium,small}`
- `--md-sys-typescale-title-{large,medium,small}`
- `--md-sys-typescale-body-{large,medium,small}`
- `--md-sys-typescale-label-{large,medium,small}`

### Shape
- `--md-sys-shape-corner-{extra-large,large,medium,small,extra-small}`

### Motion (springs)
- Durations : `--md-sys-motion-duration-{short1,short2,medium1,medium2,long1,long2}`
- Easings : `--md-sys-motion-easing-{linear,emphasized,emphasized-decelerate,emphasized-accelerate,standard,standard-decelerate,standard-accelerate}`

### State / Spatial
- Opacity disabled/hover/focus/pressed
- Spacing : multiples de 4px

---

## 4. Stratégie refactor — Button (exemple)

```tsx
import {
  MdFilledButton, MdOutlinedButton, MdTextButton,
  MdTonalButton, MdElevatedButton,
} from '@material/web/button/all.js';

const variantMap = {
  default: MdFilledButton,
  outline: MdOutlinedButton,
  ghost: MdTextButton,
  secondary: MdTonalButton,
  destructive: MdFilledButton, // + error color via --md-sys-color-error
  link: MdTextButton,
} as const;

export function Button({ variant = 'default', size, ...props }) {
  const Component = variantMap[variant];
  return <Component style={sizeStyles[size]} {...props} />;
}
```

Sizes via CSS vars `--md-sys-size-*` ou inline styles (pas de prop size native M3).

---

## 5. Priorités refactor

### Haute (high-frequency)
1. **Button**
2. **Input** + **Select**
3. **Card**
4. **Dialog**
5. **Tabs** + **Navigation**

### Moyenne
6. Checkbox, Radio, Switch
7. Badge, Avatar, Progress
8. Dropdown-Menu, Popover (composite)
9. Toast/Snackbar

### Basse
10. Carousel, Chart, Calendar
11. Pagination, Breadcrumb, Separator (composites)
12. Resizable, Scroll-Area

---

## 6. Ressources

- [Material Design 3 Components](https://m3.material.io/components)
- [Material Web GitHub](https://github.com/material-components/material-web)
- [Material Web Theming](https://material-web.dev/theming/material-theming/)
- [Design Tokens — M3](https://m3.material.io/foundations/design-tokens)
- [Typography — M3](https://m3.material.io/styles/typography/type-scale-tokens)

---

## 7. Pipeline d'intégration aphrody

1. **bxc scraping** → extrait tokens M3 + assets CDN depuis m3.material.io
2. **JSON tokens** → `packages/ui/tokens/m3.json`
3. **Refactor branche `aphrody`** sur `aphrody-code/ui` :
   - Pour chaque component HAUTE priorité : wrapper Material Web 3
   - Mapping variants shadcn → M3
   - Tests visuels via skill `pixel-perfect`
4. **Push** `aphrody-code/ui#aphrody` après chaque batch
