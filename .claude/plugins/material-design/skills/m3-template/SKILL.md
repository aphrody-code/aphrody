---
name: m3-template
description: >
  Action skill: produces a complete, ready-to-use M3 page or screen scaffold.
  Invoke when the user says "scaffold an M3 page", "scaffold an M3 screen",
  "give me a Material Design template", "give me a Material Design layout",
  "create a dashboard in M3", "create a settings screen in M3",
  "create a list-detail screen in M3", "create a feed screen in M3",
  "M3 page boilerplate", or any similar request to generate a concrete screen.
---

# M3 Template -- Procedure for Claude

This skill instructs Claude to produce a complete adaptive M3 screen scaffold.
Follow each step in order before emitting code.

---

## Step 0 -- Resolve target parameters

Determine these four before writing any code. Infer the most reasonable default
if the user has not specified; do not ask unless resolution is genuinely
ambiguous.

1. **Canonical layout**: list-detail / supporting-pane / feed / dashboard /
   settings / auth. Default: settings if not implied by context.
2. **Target size classes**: which breakpoints must be supported. Default:
   Compact through Expanded (mobile to tablet/desktop).
3. **Stack**: React with `@aphrody-code/m3-react`, or plain web components
   (`md-*` + Lit), or plain HTML. Default: React if the project uses React;
   plain HTML otherwise.
4. **Primary content**: what lives in each pane (brief description). Infer
   from context.

---

## Step 1 -- Lay out the adaptive shell

Build the outer scaffold first. It must contain:

- **Navigation component**, swapped per size class:
  - Compact (< 600 px): `md-navigation-bar` (bottom, full width)
  - Medium (600 -- 839 px): `md-navigation-rail` (left side)
  - Expanded+ (>= 840 px): `md-navigation-drawer` permanent (left side)
- **Top app bar** (`md-top-app-bar`): present on Compact and Medium; on
  Expanded+ it may be omitted if the drawer header carries the title.
- **Content area**: one pane on Compact, two panes on Expanded+.
- **FAB** (`md-fab` or `md-extended-fab`): include if the layout has a primary
  create/add action.

Use **Tailwind utilities** on `<div>` wrappers and host elements for
host-level layout (flex, grid, gap, width, padding). Use `min-h-screen`,
`flex`, `flex-col`, `flex-row`, `gap-*`, `p-4`, etc. on the 4 dp grid (Tailwind
spacing unit 1 = 4px = 4 dp).

Use **`@media` or `@container` queries** in a `<style>` block (or a CSS module)
to swap the navigation pattern at 600 px and 840 px. Toggle a CSS class
(`nav-compact` / `nav-medium` / `nav-expanded`) on the shell element rather
than conditionally mounting components in JS.

---

## Step 2 -- Use only M3 components, tokens, and icons

Constraints that must hold for every line of output:

- Components: `md-*` custom elements (plain web) or `Md*` React wrappers
  (from `@aphrody-code/m3-react`). Never use `<button>`, `<input>`, etc.,
  directly when an `md-*` equivalent exists.
- Color: always `var(--md-sys-color-*)`. Never hardcode a color value
  (`#hex`, `rgb(...)`, `oklch(...)`, named CSS color). Never use a Tailwind
  color utility (`bg-blue-500`) for component color -- only for layout.
- Typography: `var(--md-sys-typescale-*)` or the `md-*` component's own
  typography. Never hardcode `font-size` or `font-weight`.
- Icons: `<md-icon>icon_name</md-icon>` with Material Symbols ligature names
  (snake_case). Never inline SVG.
- No `sx` prop. No Emotion. No MUI.
- No `style=""` inline overrides except when setting a `--md-sys-color-*`
  token on a subtree (e.g., for a local dark-on-light inversion).

---

## Step 3 -- Canonical pane composition

Implement the pane layout for the chosen canonical layout:

### List-detail

- **Compact**: one pane at a time. Show the list by default; navigate to
  detail on item tap (back button in top app bar returns to list). Use
  `md-list` + `md-list-item` for the list; `md-divider` between sections.
- **Expanded+**: two panes side by side. List pane ~360 px fixed or
  `min-w-[280px] max-w-xs`; detail pane fills remaining space. Use
  `md-navigation-drawer` or a sidebar `<aside>` for the list pane if it is
  persistent.

### Supporting pane

- **Compact**: single pane (main content only). Supporting panel collapses
  into a bottom sheet (`md-dialog` with `type="alert"` or a custom sheet) or
  is omitted.
- **Expanded+**: main pane ~2/3 width, supporting pane ~1/3 width. Use
  `md-card` (outlined) for the supporting pane container. Split with a
  `md-divider` (vertical) or a gap.

### Feed

- **Compact**: single column of `md-card` items, full width minus 16 px
  margin on each side.
- **Medium**: two-column `grid grid-cols-2 gap-4`.
- **Expanded+**: three-column `grid grid-cols-3 gap-6` or auto-fill with
  `grid-cols-[repeat(auto-fill,minmax(280px,1fr))]`.

### Dashboard

- **Compact**: single column, stacked metric cards.
- **Expanded+**: mixed grid -- summary `md-card` spanning 2 cols, charts/
  tables filling remaining cells. Use `md-card` (elevated or filled).

### Settings

- **Compact**: single column. Group settings under `<section>` with a
  `md-divider` and a `<p>` category label styled `label-large` via
  `var(--md-sys-typescale-label-large-*)`.
- **Expanded+**: two-column. Left column: category list (use `md-list`);
  right column: selected category's settings (use `md-list`, `md-switch`,
  `md-slider`, `md-select`).

### Auth (login / sign-up)

- **Compact**: centered card full-width, `max-w-sm mx-auto`, containing
  `md-filled-text-field` inputs and an `md-filled-button` CTA.
- **Expanded+**: split -- illustration/brand left half, form right half.

---

## Step 4 -- Make it adaptive with media/container queries

Add a `<style>` block (or CSS module) covering the M3 breakpoints. Show/hide
or swap components by toggling `display: none` / `display: flex` and
`visibility` on the navigation components. Keep all three navigation
components in the DOM and let CSS decide which one is visible -- this avoids
hydration/mount cost on resize.

```css
/* default = Compact */
md-navigation-rail,
md-navigation-drawer {
  display: none;
}
md-navigation-bar {
  display: flex;
}

@media (min-width: 600px) {
  md-navigation-bar {
    display: none;
  }
  md-navigation-rail {
    display: flex;
  }
}

@media (min-width: 840px) {
  md-navigation-rail {
    display: none;
  }
  md-navigation-drawer {
    display: flex;
  }
}
```

For pane splits, add `.pane-container` layout rules inside
`@media (min-width: 840px)` to switch from `flex-col` to `flex-row`.

---

## Step 5 -- Verify build compatibility

After emitting code, state which command confirms the template compiles:

- React stack: `cd examples/showcase && bun run build` (or `bun run
typecheck`).
- Plain HTML / web components: open in a browser or run a Bun static server
  (`Bun.serve`).

Remind the caller to run `bun install` first if `@aphrody-code/m3-react` or
`@material/web` are not yet in `node_modules`.

---

## Skeleton examples

### Example A -- Settings screen (React, Compact through Expanded)

```tsx
// packages/react usage; host layout = Tailwind; theming = --md-sys-color-*

import {
  MdNavigationBar,
  MdNavigationRail,
  MdNavigationDrawer,
  MdNavigationTab,
  MdTopAppBar,
  MdList,
  MdListItem,
  MdSwitch,
  MdDivider,
  MdIcon,
} from "@aphrody-code/m3-react";

export function SettingsPage() {
  return (
    <div className="flex min-h-screen" style={{ background: "var(--md-sys-color-surface)" }}>
      {/* Navigation -- all three rendered; CSS picks the visible one */}
      <MdNavigationDrawer className="nav-drawer hidden md-expanded:flex" opened>
        <MdNavigationTab>
          <MdIcon slot="active-icon">settings</MdIcon>
          <MdIcon slot="icon">settings</MdIcon>
          Settings
        </MdNavigationTab>
      </MdNavigationDrawer>

      <MdNavigationRail className="nav-rail hidden md-medium:flex">
        <MdNavigationTab>
          <MdIcon slot="active-icon">settings</MdIcon>
          <MdIcon slot="icon">settings</MdIcon>
          Settings
        </MdNavigationTab>
      </MdNavigationRail>

      {/* Main content */}
      <div className="flex flex-col flex-1">
        <MdTopAppBar className="nav-top-bar">
          <MdIcon slot="navigationIcon">arrow_back</MdIcon>
          Settings
        </MdTopAppBar>

        <main className="flex-1 p-4 md:p-6 max-w-2xl mx-auto w-full">
          <section>
            <p
              style={{
                color: "var(--md-sys-color-primary)",
                font: "var(--md-sys-typescale-label-large-font)",
                fontSize: "var(--md-sys-typescale-label-large-size)",
              }}
              className="px-4 py-2"
            >
              Account
            </p>
            <MdList>
              <MdListItem>
                <MdIcon slot="start">person</MdIcon>
                Profile
              </MdListItem>
              <MdDivider />
              <MdListItem>
                <MdIcon slot="start">lock</MdIcon>
                Privacy
                <MdSwitch slot="end" />
              </MdListItem>
            </MdList>
          </section>
        </main>
      </div>

      {/* Bottom navigation bar (Compact only) */}
      <MdNavigationBar className="nav-bar fixed bottom-0 inset-x-0">
        <MdNavigationTab active>
          <MdIcon slot="active-icon">settings</MdIcon>
          <MdIcon slot="icon">settings</MdIcon>
          Settings
        </MdNavigationTab>
      </MdNavigationBar>
    </div>
  );
}
```

```css
/* Adaptive navigation visibility */
.nav-rail,
.nav-drawer {
  display: none;
}
.nav-bar {
  display: flex;
}

@media (min-width: 600px) {
  .nav-bar {
    display: none;
  }
  .nav-top-bar {
    display: none;
  }
  .nav-rail {
    display: flex;
  }
}

@media (min-width: 840px) {
  .nav-rail {
    display: none;
  }
  .nav-drawer {
    display: flex;
  }
}
```

---

### Example B -- List-detail screen (plain web components, Compact and Expanded)

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <script type="module" src="/node_modules/@material/web/aphrody-components.js"></script>
    <link rel="stylesheet" href="/styles.css" />
  </head>
  <body>
    <div class="shell">
      <!-- Navigation bar (Compact) -->
      <md-navigation-bar class="nav-bar">
        <md-navigation-tab active>
          <md-icon slot="active-icon">inbox</md-icon>
          <md-icon slot="icon">inbox</md-icon>
          Inbox
        </md-navigation-tab>
      </md-navigation-bar>

      <!-- Navigation rail (Medium+) -->
      <md-navigation-rail class="nav-rail">
        <md-navigation-tab active>
          <md-icon slot="active-icon">inbox</md-icon>
          <md-icon slot="icon">inbox</md-icon>
          Inbox
        </md-navigation-tab>
      </md-navigation-rail>

      <!-- Content -->
      <div class="content-area">
        <!-- List pane -->
        <aside class="list-pane">
          <md-top-app-bar>Inbox</md-top-app-bar>
          <md-list>
            <md-list-item type="button">
              <md-icon slot="start">mail</md-icon>
              Message from Alice
              <span slot="supporting-text">Hey, wanted to check in...</span>
            </md-list-item>
            <md-divider></md-divider>
            <md-list-item type="button">
              <md-icon slot="start">mail</md-icon>
              Project update
              <span slot="supporting-text">The build passed on CI...</span>
            </md-list-item>
          </md-list>
        </aside>

        <!-- Detail pane (hidden on Compact) -->
        <main class="detail-pane">
          <p
            style="color: var(--md-sys-color-on-surface-variant);
                  padding: 24px;
                  font: var(--md-sys-typescale-body-large-font);
                  font-size: var(--md-sys-typescale-body-large-size)"
          >
            Select a message to read.
          </p>
        </main>
      </div>
    </div>
  </body>
</html>
```

```css
/* styles.css */
:root {
  color-scheme: light dark;
  background: var(--md-sys-color-surface);
  color: var(--md-sys-color-on-surface);
}

.shell {
  display: flex;
  flex-direction: column;
  min-height: 100dvh;
}

.content-area {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.nav-rail {
  display: none;
}
.nav-bar {
  display: block;
}
.detail-pane {
  display: none;
}

@media (min-width: 600px) {
  .nav-bar {
    display: none;
  }
  .nav-rail {
    display: block;
  }
  .shell {
    flex-direction: row;
  }
  .content-area {
    flex-direction: row;
  }
}

@media (min-width: 840px) {
  .detail-pane {
    display: flex;
    flex: 1;
  }
  .list-pane {
    width: 360px;
    flex-shrink: 0;
    border-right: 1px solid var(--md-sys-color-outline-variant);
  }
}
```

---

## Rules summary (never violate)

- No hardcoded colors. All color from `var(--md-sys-color-*)`.
- No `sx` prop, no Emotion, no MUI imports.
- Layout on `<div>` wrappers via Tailwind or plain CSS. Never on `md-*`
  internals (Shadow DOM boundary).
- Icons via `<md-icon>` with Material Symbols ligature names only.
- Navigation components: all three in DOM, CSS toggles visibility.
- Touch targets >= 48 px on Compact (built into `md-*` components).
- bun only for installs and builds -- never npm or pnpm.

---

## Related skills

- `responsive-adaptive`: breakpoint values, grid/margin rules, decision checklist.
- `m3-design-guide`: color tokens, typography tokens, elevation, theming.
- `m3-spec-check`: validate that output conforms to M3 spec before shipping.
