---
name: m3-spec-check
description: >
  Audit a file, component, or directory for Material Design 3 conformance.
  Use when the user says: "check this is M3-compliant", "audit Material Design conformance",
  "spec-check this component", "spec-check this page", "is this on-spec for M3",
  "validate my Material UI", "does this follow MD3".
---

# m3-spec-check

Audits source files against the M3 specification. Runs the lint plugin when available; falls back to grep heuristics. Outputs a findings report grouped by severity with `file:line` and the suggested M3 fix.

---

## Step 1 — Run the lint plugin (preferred)

Check whether `@aphrody-code/eslint-plugin-m3` is installed:

```bash
# In the project root:
ls node_modules/@aphrody-code/eslint-plugin-m3 2>/dev/null || echo "NOT_FOUND"
```

If found, run via oxlint (fastest) by writing or checking for an `.oxlintrc.json` at the project root with this `jsPlugins` section, then executing:

```json
{
  "plugins": ["@aphrody-code/m3"],
  "jsPlugins": [
    {
      "name": "@aphrody-code/eslint-plugin-m3",
      "path": "./node_modules/@aphrody-code/eslint-plugin-m3/index.js"
    }
  ],
  "rules": {
    "m3/valid-icon-name": "error",
    "m3/no-sx-prop": "error",
    "m3/no-mui-prop-on-md": "error",
    "m3/no-mui-import": "error",
    "m3/prefer-icon-token": "warn",
    "m3/no-hardcoded-color": "error"
  }
}
```

Run:

```bash
bunx oxlint --config .oxlintrc.json <target-path>
```

The `jsPlugins` path is resolved **relative to the `.oxlintrc.json` file**, not the CWD. Verify the path before running.

If ESLint is preferred or oxlint is not available:

```bash
bunx eslint --rulesdir node_modules/@aphrody-code/eslint-plugin-m3/lib/rules <target-path>
```

If the plugin is not installed, proceed with the grep heuristics below.

---

## Step 2 — Grep heuristics (fallback or supplement)

Run these against the target path. Flag every match as a finding.

### COLOR

Hardcoded colors in style attributes or CSS-in-JS — must be replaced with `var(--md-sys-color-<role>)`:

```bash
rg --with-filename --line-number \
  '(style=.*|sx=.*|styled\.|css`).*#[0-9a-fA-F]{3,8}|rgb\(|rgba\(' \
  <target-path>
```

Content color on an `md-*` element or M3 component not using `on-*` role:

```bash
rg --with-filename --line-number \
  'color:\s*var\(--md-sys-color-(?!on-)' \
  <target-path>
# Review matches: if the element is a container (background), the content child
# must use the matching on-* role.
```

Duplicated color declarations for dark mode (dark mode must be handled by role swaps, not duplicate CSS):

```bash
rg --with-filename --line-number \
  'prefers-color-scheme.*dark|data-theme.*dark' \
  <target-path>
# Each match: verify it only sets --md-sys-color-* values, not component-level colors.
```

### TYPE

Arbitrary pixel font sizes (should be typescale tokens):

```bash
rg --with-filename --line-number \
  'font-size:\s*[0-9]+px' \
  <target-path>
```

Font weights outside 400/500 (M3 only uses Regular and Medium):

```bash
rg --with-filename --line-number \
  'font-weight:\s*(?!400|500)[0-9]+' \
  <target-path>
```

### SHAPE / ELEVATION

Ad-hoc border-radius values not using a shape token:

```bash
rg --with-filename --line-number \
  'border-radius:\s*(?![0-9]*var\(--md-sys-shape)[0-9]' \
  <target-path>
```

Ad-hoc box-shadow not using an elevation token or `surface-container-*` role:

```bash
rg --with-filename --line-number \
  'box-shadow:' \
  <target-path>
# Review each: shadows are allowed for Level 3+ (FAB, dialogs) but must still
# be paired with surface-container-* background, not an arbitrary color.
```

### SPACING / LAYOUT

Spacing values not on a 4dp grid (i.e., not multiples of 4px):

```bash
rg --with-filename --line-number \
  'padding:\s*[0-9]*[13579]px|margin:\s*[0-9]*[13579]px' \
  <target-path>
# Odd-pixel values are always off-grid.
```

Touch targets below 48px — look for small fixed sizes on interactive elements:

```bash
rg --with-filename --line-number \
  '(width|height):\s*(1[0-9]|2[0-9]|3[0-9]|4[0-7])px' \
  <target-path>
# Flag any interactive element (button, a, [role=button]) with these sizes.
```

### STATE LAYERS

Missing hover/focus/active state layer on interactive components. Check for `::before` or `:hover` handling:

```bash
rg --with-filename --line-number \
  ':hover|:focus-visible|:active' \
  <target-path>
# If an interactive component has none of these, it is missing state layers.
```

Wrong state layer opacity — must be hover:8%, focus:10%, pressed:10%, dragged:16%:

```bash
rg --with-filename --line-number \
  ':hover.*opacity|:focus.*opacity|:active.*opacity' \
  <target-path>
# Compare found values against: hover=0.08, focus=0.10, active/pressed=0.10
```

Disabled state incorrectly using state layers (disabled uses direct opacity, not layers):

```bash
rg --with-filename --line-number \
  ':disabled|disabled.*opacity|\.disabled.*opacity' \
  <target-path>
# Content must be at 38% opacity; container at 12% opacity.
```

### ICONS

Non-variable Material Symbols instance (frozen axes = no FILL/wght effect):

```bash
rg --with-filename --line-number \
  'Material Symbols.*@[0-9]' \
  <target-path>
# A URL with @24,400,0,0 or similar is a frozen instance. Replace with variable range.
```

Icon names not in snake_case (M3 icon names are always snake_case ligatures):

```bash
rg --with-filename --line-number \
  '<md-icon>[A-Z]' \
  <target-path>
```

`--md-icon-*` axes set via hardcoded font-feature-settings instead of the CSS custom properties:

```bash
rg --with-filename --line-number \
  "font-feature-settings.*'FILL'\|\"FILL\"" \
  <target-path>
# Must use --md-icon-fill, --md-icon-wght, --md-icon-grad, --md-icon-opsz instead.
```

### ACCESSIBILITY

Check for interactive elements without accessible name:

```bash
rg --with-filename --line-number \
  '<(button|a|input|select|textarea)[^>]*>' \
  <target-path>
# Each match: verify aria-label, aria-labelledby, or visible text label is present.
```

Color-alone state (no icon or text to convey state):

```bash
rg --with-filename --line-number \
  'color.*error\|color.*success\|color.*warning' \
  <target-path>
# Review: if state is only communicated by color, add an icon or text indicator.
```

Missing focus ring on focusable custom elements:

```bash
rg --with-filename --line-number \
  ':focus-visible' \
  <target-path>
# Every focusable md-* or custom element must have an entry. Absence is a finding.
```

### MIGRATION RESIDUE

`sx` prop (MUI-specific, no equivalent on md-\* elements):

```bash
rg --with-filename --line-number \
  '\bsx=' \
  <target-path>
```

MUI prop names on md-\* elements (M3 equivalents differ):

```bash
rg --with-filename --line-number \
  '<md-[a-z-]+[^>]+(checked=|title=|open=|activeTabIndex=)' \
  <target-path>
# checked -> selected; title -> text (md-dialog); open -> opened (md-dialog);
# value -> activeTabIndex (md-tabs).
```

MUI package imports:

```bash
rg --with-filename --line-number \
  "from '@mui/|from \"@mui/" \
  <target-path>
```

---

## Step 3 — Build the findings report

Organize findings into three severity levels:

**ERROR** — spec violation that breaks M3 correctness or breaks accessibility:

- Hardcoded colors / non-token color
- Wrong `on-*` content roles
- Missing touch target (< 48dp on interactive element)
- Missing accessible name
- Contrast below 4.5:1 (text) or 3:1 (non-text/large)
- `@mui/*` imports on an M3 project
- MUI prop names on `md-*` elements
- `sx` prop on `md-*` elements

**WARN** — M3 recommendation not followed, may degrade experience:

- Arbitrary font sizes instead of typescale tokens
- Font weight outside 400/500
- Ad-hoc border-radius instead of shape tokens
- Off-grid spacing (not a multiple of 4px)
- Ad-hoc box-shadow without surface-container pairing
- State layer opacity at wrong value
- Frozen Material Symbols instance
- Color-alone state communication

**INFO** — worth noting, low urgency:

- Dark mode handled by component-level duplication instead of role swaps
- Missing `prefers-reduced-motion` guard on animations
- Icon PascalCase name (possible typo vs snake_case)

### Output format

```
=== M3 SPEC CHECK: <target-path> ===

[ERROR] COLOR — hardcoded color
  src/components/Badge.tsx:14
  Found: color: #6750A4
  Fix:   color: var(--md-sys-color-primary, #6750A4)

[ERROR] MIGRATION RESIDUE — sx prop on md-element
  src/views/Home.tsx:37
  Found: <md-button sx={{ mt: 2 }}>
  Fix:   Remove sx; use CSS custom property or wrapper class

[WARN] TYPE — arbitrary font size
  src/components/Label.css:8
  Found: font-size: 13px
  Fix:   font-size: var(--md-sys-typescale-label-medium-size, 0.75rem)

[WARN] STATE LAYER — hover opacity incorrect
  src/components/Chip.css:22
  Found: :hover { opacity: 0.15; }
  Fix:   Hover state layer must be 0.08 (8%)

[INFO] ICONS — frozen Material Symbols instance
  src/index.html:5
  Found: fonts.googleapis.com/...@24,400,0,0
  Fix:   Load variable range; drive axes via --md-icon-fill/wght/grad/opsz

Total: 2 error(s), 2 warning(s), 1 info
```

---

## Step 4 — Next actions

After reporting:

- To auto-fix violations: invoke `m3-correct` with the findings list. It applies token replacements, state layer corrections, and prop renames.
- For a full-codebase autonomous audit with commits per finding: use the `md-spec-checker` agent.
- For the authoritative list of color roles, shape tokens, and typescale tokens: see `m3-design-guide`.

---

## Quick reference — correct M3 values

**State layer opacities**: hover 8% / focus 10% / pressed 10% / dragged 16% / disabled-content 38% / disabled-container 12%.

**Touch target**: minimum 48x48 dp on any interactive element.

**Contrast**: text >= 4.5:1, large text (>=18pt regular or 14pt bold) and non-text elements >= 3:1.

**Shape tokens**: `extra-small` 4px / `small` 8px / `medium` 12px / `large` 16px / `extra-large` 28px / `full` 9999px.

**Spacing grid**: multiples of 4dp (4, 8, 12, 16, 24, 32...).

**Font weights**: 400 (Regular) and 500 (Medium) only.

**Migration prop renames** (MUI -> M3): `checked` -> `selected`, `title` -> `text` (md-dialog), `open` -> `opened` (md-dialog), `value` -> `activeTabIndex` (md-tabs).
