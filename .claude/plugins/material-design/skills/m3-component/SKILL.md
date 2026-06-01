---
name: m3-component
description: >
  Scaffold a self-contained Material Design 3 md-* web component and/or its React wrapper.
  Use when the user says: "create an M3 component", "scaffold a Material Design 3 <X> component",
  "build an md-* web component", "make a new Material component", "wrap a component in React",
  "add md-<x> to the monorepo".
---

# m3-component

Scaffolds a Material Design 3 component end-to-end. Auto-detects context and takes the right path:

- **Mode A** — inside the `material-web` monorepo (detected by `packages/material-web/` + `packages/react/md-elements.txt`): full 3-point wiring via the `add-md-component` skill.
- **Mode B** — any other project (generic React / Lit / plain web): generates a standalone, spec-compliant component skeleton.

---

## Detect mode

Before doing anything, check whether both of these paths exist relative to the project root:

- `packages/material-web/` (directory)
- `packages/react/md-elements.txt` (file)

If both exist: **Mode A**. Otherwise: **Mode B**.

---

## Mode A — material-web monorepo

Defer to the `add-md-component` skill, which owns the canonical procedure for this repo. The steps below summarize it; `add-md-component/SKILL.md` is authoritative.

### Inputs to resolve

- `tag`: kebab-case, must start with `md-` (e.g. `md-rating`).
- `ComponentName`: public class = `Md` + PascalCase of tag without `md-` (e.g. `md-rating` -> `MdRating`; internal class `Rating`).
- `events` (optional): list any custom events the component dispatches.

If any input is ambiguous, infer from the closest existing component and proceed. Do not stop to ask.

### File structure

Create exactly three files under `packages/material-web/<comp>/`:

```
<comp>/<comp>.ts                     # @customElement + export class + HTMLElementTagNameMap
<comp>/internal/<comp>.ts            # LitElement subclass (logic)
<comp>/internal/<comp>-styles.ts     # css`` styles consuming --md-sys-* with fallbacks
```

### 3-point central wiring (do all three, in order)

**Point 1 — bundle export**

Add to `packages/material-web/aphrody-components.ts`:

```ts
export * from "./<comp>/<comp>.js";
```

**Point 2 — register the tag**

Append the tag on a new line in `packages/react/md-elements.txt`.

**Point 3 — regenerate React wrappers**

```bash
cd packages/react && bun run generate.mjs
```

Check stdout: the tag must NOT appear under "Unresolved". If it does, the `@customElement … export class` lines are not consecutive or the export is missing from `aphrody-components.ts`.

### Build verification (strict, this exact order — do not skip or reorder)

```bash
bunx turbo run build
# compiled .js files are gitignored; turbo build refreshes them first
# components that import each other read the compiled .js, not .ts

cd packages/material-web && bunx tsc -p tsconfig.json --noEmit
# strict + noUnusedLocals: catches TS6133/TS7011 that --skipLibCheck hides
# use bun run typecheck:fast (tsgo) for speed during iteration; tsc remains canonical

cd packages/material-web && bun run build:aphrody
# self-contained dist-aphrody bundle
```

Do not claim success until `tsc --noEmit` is clean and the tag is resolved by `generate.mjs`.

### Gotchas

- **Decorator + export class must be on consecutive lines.** `generate.mjs` regex: `/@customElement\(\s*['"]([a-z0-9-]+)['"]\s*\)\s*\nexport class (\w+)/`. A blank line or comment between them breaks resolution.
- **Never hand-edit** `packages/react/wrappers/*.ts` or `packages/react/index.ts` — both are auto-generated and will be overwritten on the next `generate.mjs` run.
- **Auto-formatter** (`post-edit-format.sh`) rewrites quotes and indentation after every `Edit`/`Write`. Single or double quotes in `@customElement(...)` are both accepted by the regex. If an edit keeps reverting, use `sed -i` to bypass the hook.
- **Custom events**: if the component fires events, add them to `EVENTS_BY_TAG` in `packages/react/generate.mjs` before running point 3. Verify the event string against the actual `new CustomEvent('...')` call — do not guess.

### Lit best practices (enforced by strict build)

Apply all of the following. Violations cause `TS6133`/`TS7011` or `tsc --noEmit` errors:

- `@property` only for public reflected API; `@state` for all internal state.
- Derived values computed in `willUpdate(changedProperties)`, never in `render()`.
- `styleMap` / `classMap` for dynamic bindings; never string concatenation.
- Custom events with `{ bubbles: true, composed: true }` to cross the shadow root.
- Observers and listeners cleaned up in `disconnectedCallback`.
- Form controls: `static formAssociated = true` + `attachInternals()` + `ElementInternals`.
- `static shadowRootOptions = { ...LitElement.shadowRootOptions, delegatesFocus: true }` for focusable wrappers around native controls.
- No implicit `any`; annotate all return types.

---

## Mode B — generic M3 project

Produce a standalone, spec-compliant component. The output path depends on whether the project uses Lit, React, or plain CSS.

### Design constraints (apply regardless of stack)

**Color**

- All colors via `var(--md-sys-color-<role>, <fallback>)`. Never hardcode hex/rgb.
- Content on a surface uses the matching `on-*` role (e.g. text on `primary` uses `on-primary`).
- Dark mode is handled automatically when the host sets `--md-sys-color-*` roles via `:root` / `[data-theme=dark]` — never duplicate color declarations for dark.

**Type**

- Font/size/weight/line-height from `--md-sys-typescale-<role>-<size>-<property>`. Only weights 400 and 500 are used in M3.
- Never use arbitrary `px` sizes where a typescale token exists.

**Shape**

- Border radius from `var(--md-sys-shape-corner-<token>, <fallback>)`. Tokens: `extra-small` 4px, `small` 8px, `medium` 12px, `large` 16px, `extra-large` 28px, `full` 9999px.

**Elevation / surface hierarchy**

- Use `surface-container-*` roles (`lowest / low / (default) / high / highest`) for elevation hierarchy, not ad-hoc `box-shadow` values.
- Tonal elevation: apply `background-color: var(--md-sys-color-surface-container-high)` for elevated surfaces instead of hardcoded shadows.

**State layers**

- Every interactive surface needs a state layer overlay using the `on-*` color of that surface at the correct opacity:
  - hover: 8%
  - focus: 10%
  - pressed: 10%
  - dragged: 16%
- Disabled: content at 38% opacity; container at 12% opacity. No state layers on disabled elements.

**Spacing**

- Use multiples of 4dp (4, 8, 12, 16, 24, 32...). Minimum touch target: **48x48 dp** for any interactive element, even if the visual is smaller (pad with `min-width: 48px; min-height: 48px` and center the visual inside).

**Icons**

- Use Material Symbols ligatures (`font-family: 'Material Symbols Outlined'`).
- Load the variable font ranges, not a frozen `@24,400,0,0` instance; drive axes via `--md-icon-fill`, `--md-icon-wght`, `--md-icon-grad`, `--md-icon-opsz`.

**Accessibility**

- Explicit `role` and `aria-*` attributes.
- Visible focus ring (`:focus-visible` at minimum; `md-focus-ring` in the monorepo).
- Never convey state by color alone; pair with icon or text.
- Contrast: text >= 4.5:1, large text and non-text elements >= 3:1.

---

### Lit skeleton (md-\* web component)

Three files, mirroring the monorepo's self-contained pattern:

**`<comp>/<comp>.ts`** — public element, registers the tag:

```ts
// Copyright 2026 Google LLC
// SPDX-License-Identifier: Apache-2.0
import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";
import { CompInternal } from "./internal/comp.js";
import { styles } from "./internal/comp-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-comp": MdComp;
  }
}

@customElement("md-comp")
export class MdComp extends CompInternal {
  static override styles: CSSResultOrNative[] = [styles];
}
```

**`<comp>/internal/comp.ts`** — LitElement logic:

```ts
import { LitElement, html } from "lit";
import { property, state } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";

export class CompInternal extends LitElement {
  /** Public API property — reflected as attribute. */
  @property({ type: Boolean, reflect: true }) disabled = false;

  /** Internal reactive state — never exposed as attribute. */
  @state() private _hovered = false;

  // Derived values: computed once in willUpdate, not recomputed in render.
  private _computedLabel = "";

  override willUpdate(changed: Map<string | number | symbol, unknown>) {
    if (changed.has("disabled")) {
      this._computedLabel = this.disabled ? "unavailable" : "available";
    }
  }

  override render() {
    return html`
      <div
        class=${classMap({ comp: true, "comp--disabled": this.disabled })}
        role="region"
        aria-disabled=${this.disabled ? "true" : "false"}
        @mouseenter=${() => {
          this._hovered = true;
        }}
        @mouseleave=${() => {
          this._hovered = false;
        }}
      >
        <slot></slot>
      </div>
    `;
  }

  // Dispatch events that must cross the shadow root.
  private _dispatch(type: string, detail?: unknown) {
    this.dispatchEvent(
      new CustomEvent(type, {
        bubbles: true,
        composed: true,
        detail,
      }),
    );
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    // Clean up any observers or external event listeners here.
  }
}
```

**`<comp>/internal/comp-styles.ts`** — styles with M3 tokens + fallbacks:

```ts
import { css } from "lit";

export const styles = css`
  :host {
    display: inline-flex;
    outline: none;
    /* Shape */
    border-radius: var(--md-sys-shape-corner-medium, 12px);
    /* Ensure minimum touch target */
    min-width: 48px;
    min-height: 48px;
  }

  .comp {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    /* Surface + on-surface content color */
    background-color: var(--md-sys-color-surface-container, #f3edf7);
    color: var(--md-sys-color-on-surface, #1c1b1f);
    /* Type scale */
    font-family: var(--md-sys-typescale-body-large-font, Roboto, sans-serif);
    font-size: var(--md-sys-typescale-body-large-size, 1rem);
    font-weight: var(--md-sys-typescale-body-large-weight, 400);
    line-height: var(--md-sys-typescale-body-large-line-height, 1.5rem);
    /* Motion */
    transition:
      background-color 200ms cubic-bezier(0.2, 0, 0, 1),
      color 200ms cubic-bezier(0.2, 0, 0, 1);
  }

  /* State layer: hover */
  .comp::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background-color: var(--md-sys-color-on-surface, #1c1b1f);
    opacity: 0;
    pointer-events: none;
    transition: opacity 200ms cubic-bezier(0.2, 0, 0, 1);
  }

  :host(:hover) .comp::before {
    opacity: 0.08;
  }

  :host(:focus-visible) .comp::before {
    opacity: 0.1;
  }

  :host(:active) .comp::before {
    opacity: 0.1;
  }

  /* Disabled state */
  .comp--disabled {
    color: color-mix(in srgb, var(--md-sys-color-on-surface, #1c1b1f) 38%, transparent);
    background-color: color-mix(in srgb, var(--md-sys-color-on-surface, #1c1b1f) 12%, transparent);
    pointer-events: none;
  }

  /* Focus ring */
  :host(:focus-visible) {
    outline: 3px solid var(--md-sys-color-secondary, #625b71);
    outline-offset: 2px;
  }
`;
```

---

### React skeleton (M3 component using tokens + Tailwind)

For projects using React with `@aphrody-code/m3-tokens/m3-tokens.css` imported globally:

```tsx
import { CSSProperties } from "react";

interface M3CompProps {
  label: string;
  disabled?: boolean;
  onClick?: () => void;
}

/**
 * Minimal M3-compliant React component.
 * Requires --md-sys-color-* CSS custom properties on :root (import m3-tokens.css).
 */
export function M3Comp({ label, disabled = false, onClick }: M3CompProps) {
  const style: CSSProperties = {
    // All color via CSS custom properties — never hardcode.
    backgroundColor: "var(--md-sys-color-primary-container)",
    color: "var(--md-sys-color-on-primary-container)",
    // Shape token with fallback.
    borderRadius: "var(--md-sys-shape-corner-medium, 12px)",
    // Minimum touch target.
    minWidth: 48,
    minHeight: 48,
    // Type scale token.
    fontFamily: "var(--md-sys-typescale-label-large-font, Roboto, sans-serif)",
    fontSize: "var(--md-sys-typescale-label-large-size, 0.875rem)",
    fontWeight: "var(--md-sys-typescale-label-large-weight, 500)",
    lineHeight: "var(--md-sys-typescale-label-large-line-height, 1.25rem)",
    // Layout.
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    padding: "10px 24px",
    border: "none",
    cursor: disabled ? "default" : "pointer",
    position: "relative",
    overflow: "hidden",
    // Motion (M3 Standard easing, Short 4 = 200ms).
    transition: "background-color 200ms cubic-bezier(0.2, 0, 0, 1)",
    // Disabled opacity on container.
    opacity: disabled ? 0.12 : 1,
  };

  return (
    <button
      style={style}
      disabled={disabled}
      onClick={onClick}
      aria-disabled={disabled}
      // State layer implemented via CSS class or ::before pseudo-element in a
      // stylesheet — see the Lit skeleton above for the CSS pattern.
      className="m3-comp"
    >
      {/* Content opacity on disabled is separate from container. */}
      <span style={{ opacity: disabled ? 38 / 100 : 1 }}>{label}</span>
    </button>
  );
}
```

Add state layer CSS alongside it (or in a CSS module):

```css
.m3-comp::before {
  content: "";
  position: absolute;
  inset: 0;
  background-color: var(--md-sys-color-on-primary-container);
  opacity: 0;
  pointer-events: none;
  transition: opacity 200ms cubic-bezier(0.2, 0, 0, 1);
  border-radius: inherit;
}
.m3-comp:hover::before {
  opacity: 0.08;
}
.m3-comp:focus-visible::before {
  opacity: 0.1;
}
.m3-comp:active::before {
  opacity: 0.1;
}
```

---

## Verification

After scaffolding:

1. Run the appropriate build verification from the mode section above.
2. Run `m3-spec-check` on the new file(s) to confirm token usage, state layers, touch targets, and a11y are correct.
3. Consult `m3-design-guide` for the authoritative list of `--md-sys-color-*` roles and `--md-sys-shape-corner-*` tokens when deciding which tokens to use.
