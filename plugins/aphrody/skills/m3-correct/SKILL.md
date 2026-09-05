---
name: m3-correct
description: User-invoked fix skill. Auto-correct Material Design 3 violations in a codebase. Use when asked to "fix M3 violations", "auto-correct Material Design issues", "make this M3-compliant", "clean up MUI residue", "fix hardcoded colors", "fix sx on md components", or "fix wrong icon names".
---

# m3-correct

Fix M3 violations in a target codebase. Work category by category; verify after each category. Never blindly mass-replace — choose the semantically correct role per usage context.

## Step 0 — Audit first

Run the `m3-spec-check` skill (or invoke the `md-spec-checker` agent) to get a prioritized findings list before touching any file. The findings list drives everything below. Do not guess; work from evidence.

## Step 1 — Machine-checkable issues via lint

Install `@aphrody-code/eslint-plugin-m3` if not already present:

```bash
bun add -D @aphrody-code/eslint-plugin-m3
```

Add (or create) `.oxlintrc.json` at the project root:

```json
{
  "jsPlugins": ["./node_modules/@aphrody-code/eslint-plugin-m3/index.js"],
  "rules": {
    "m3/valid-icon-name": "error",
    "m3/no-sx-prop": "error",
    "m3/no-mui-prop-on-md": "error",
    "m3/no-mui-import": "warn",
    "m3/prefer-icon-token": "warn",
    "m3/no-hardcoded-color": "warn"
  }
}
```

> `jsPlugins` paths resolve relative to the `.oxlintrc.json` file, not the working directory.

Run:

```bash
bunx oxlint --config .oxlintrc.json src
```

Collect every error/warning — these are your fix targets.

## Step 2 — Fix by category

Work through each category in the order below. Apply, then re-run lint to confirm the category is clean before moving on.

### 2a. Hardcoded colors (`m3/no-hardcoded-color`)

Replace `#hex` / `rgb()` values in `style` or `sx` on `md-*` components with the semantically correct `var(--md-sys-color-<role>)`.

Intent-to-role mapping (choose by what the color does, not its visual value):

| Intent                         | Role token                                   |
| ------------------------------ | -------------------------------------------- |
| Primary text on surface        | `var(--md-sys-color-on-surface)`             |
| Secondary / muted text         | `var(--md-sys-color-on-surface-variant)`     |
| Accent / brand / active        | `var(--md-sys-color-primary)`                |
| Error / destructive            | `var(--md-sys-color-error)`                  |
| Card / sheet background (low)  | `var(--md-sys-color-surface-container-low)`  |
| Card / sheet background (high) | `var(--md-sys-color-surface-container-high)` |
| Subtle border / divider        | `var(--md-sys-color-outline-variant)`        |
| Strong border                  | `var(--md-sys-color-outline)`                |
| Overlay / scrim                | `var(--md-sys-color-scrim)`                  |

When intent is ambiguous, consult the `m3-design-guide` skill for role semantics before choosing.

### 2b. `sx` prop on `md-*` wrappers (`m3/no-sx-prop`)

`sx={{...}}` on a `md-*` component has no effect: `md-*` are web components with Shadow DOM; MUI Emotion does not reach them.

- Layout / spacing (margin, padding, display, gap, width, height) -> Tailwind utility classes on the host element. Use the 4 dp grid (multiples of 1 = 4 px, 2 = 8 px, 3 = 12 px, 4 = 16 px, etc.).
- Internal styling (colors, typography, state) -> `--md-sys-*` CSS custom properties set on the component or an ancestor. Shadow DOM is not reachable by Tailwind.

Example:

```tsx
// Before
<MdButton sx={{ mt: 2, color: '#6750A4' }}>Save</MdButton>

// After
<MdButton
  className="mt-2"
  style={{ '--md-filled-button-container-color': 'var(--md-sys-color-primary)' }}
>Save</MdButton>
```

### 2c. Renamed props (`m3/no-mui-prop-on-md`)

Replace MUI prop names left on `md-*` wrappers with their M3 equivalents:

| MUI prop             | M3 prop          | Component        |
| -------------------- | ---------------- | ---------------- |
| `checked`            | `selected`       | Switch, Checkbox |
| `title`              | `text`           | Tooltip          |
| `open`               | `opened`         | Drawer           |
| `value` (Tabs)       | `activeTabIndex` | Tabs             |
| LinearProgress 0-100 | 0-1 (normalize)  | LinearProgress   |

### 2d. Wrong icon names (`m3/valid-icon-name`)

`<MdIcon>` / `<md-icon>` content must be a valid Material Symbols snake_case ligature name from the official 4253-glyph set.

- PascalCase MUI remnants: `Delete` -> `delete`, `ArrowBack` -> `arrow_back`, `Brightness4` -> `brightness_4`
- Remove style suffixes: `DeleteOutlined` -> `delete`, `SaveRounded` -> `save`
- Brand logos (GitHub, YouTube, X, etc.) are absent from Material Symbols by Google policy — keep them as SVG elements and flag with a TODO.

Collect all icon names after fixing:

```bash
rg -oN '<md-icon>([a-z_]+)</md-icon>' -r '$1' src | sort -u
```

Use that list with `ensureMaterialSymbols({ iconNames: [...] })` (see the migrate-mui skill for font loading details).

### 2e. `fontVariationSettings` inline on icons (`m3/prefer-icon-token`)

Replace inline `fontVariationSettings` with the inheritable axis tokens:

| Axis | Token                       |
| ---- | --------------------------- |
| FILL | `--md-icon-fill` (0..1)     |
| wght | `--md-icon-wght` (100..700) |
| GRAD | `--md-icon-grad` (-50..200) |
| opsz | `--md-icon-opsz` (20..48)   |

These tokens are CSS custom properties — they cascade from any ancestor and are animatable.

### 2f. Residual `@mui/*` imports (`m3/no-mui-import`)

Once a component is fully ported: remove the corresponding `@mui/material` or `@mui/icons-material` import and replace with the `@aphrody-code/m3-react` equivalent. Do not remove an import until the component using it is fully replaced — partial removal breaks the build.

## Step 3 — Re-verify

```bash
bunx oxlint --config .oxlintrc.json src
```

All `error` rules must be clean. `warn` rules should be clean or have an explicit justified exception comment.

Then re-run the `m3-spec-check` skill (or the `md-spec-checker` agent) to confirm no spec violations remain.

## Step 4 — Build and typecheck

```bash
bun run build
bun run typecheck
```

Both must exit 0. Fix any new type errors introduced by prop renames or import changes before committing.

## Notes

- Never mass-replace all `#hex` -> a single role. The same hex value can mean primary in one place and error in another — context determines the role.
- Tailwind utilities apply to the custom element host; `--md-sys-*` tokens apply inside the Shadow DOM. These are complementary, not interchangeable.
- For autonomous whole-codebase correcting, point the `md-corrector` agent at the repo root.
- For role-choice questions, use the `m3-design-guide` skill.
