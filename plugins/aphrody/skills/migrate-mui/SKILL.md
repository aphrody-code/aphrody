---
name: migrate-mui
description: Migrate a React + MUI (@mui/material@9 + @mui/icons-material + MUI X) codebase to material-web (@aphrody-code/m3-react + Material Symbols). Use when asked to "migrate MUI to material-web / M3", "port a MUI app", "run the migration codemods", "convert @mui/icons-material to Material Symbols", or "estimate migration effort".
---

# migrate-mui

Migrate a React + MUI codebase to `@aphrody-code/m3-react` (material-web wrappers) + Material Symbols. Component coverage is solved (~120 `md-*`); the real cost is the **`sx`/Emotion styling model** — ~30 % of effort, irreducibly manual.

Work in order; verify after each step.

## Detect context

Check whether the material-web monorepo codemods are available:

```bash
ls migration/codemods/transforms/orchestrator.ts 2>/dev/null && echo "CODEMODS AVAILABLE" || echo "MANUAL MODE"
```

Follow the matching path below.

---

## Path A — Codemods available (`migration/codemods/` present)

### Step 0 — Scope the target

Run from the app source directory:

```bash
rg -l "@mui/" -g '*.tsx' -g '*.ts' . | wc -l          # files on MUI
rg -oN "sx=\{\{" -g '*.tsx' . | wc -l                  # sx sites = manual wall
rg -oN "@mui/icons-material" -g '*.tsx' . | wc -l       # icon import sites
```

These numbers drive effort estimation. Never run codemods on the live source first — copy to a throwaway sandbox, transform, diff, then apply.

Filenames with `(`, `)`, `[`, `]` (Next.js route groups) break jscodeshift globbing — rename them in the sandbox (e.g. `(auth)/page.tsx` -> `f1.tsx`).

### Step 1 — Components (jscodeshift orchestrator)

```bash
cd migration/codemods
bunx jscodeshift -t transforms/orchestrator.ts --parser=tsx --extensions=tsx '<glob>'
```

Maps `@mui/material` imports + JSX -> `@aphrody-code/m3-react` wrappers. Variant-aware for Button/TextField/Select. Converts slots (`DialogTitle`, `CardHeader`, ...). Drops `sx`/`color`/`size` with `MIGRATION-TODO` markers. Layout (`Box`/`Stack`/`Grid`) -> `<div>` + TODO.

### Step 2 — Icons (96 % automatic)

```bash
bunx jscodeshift -t transforms/icons.ts --parser=tsx --extensions=tsx '<glob>'
```

PascalCase -> snake_case, validated against the 4253 official Material Symbols glyph names. Brand logos (GitHub, X, YouTube, ...) are kept and flagged — they are absent from Material Symbols by Google policy, keep as SVG.

Measured on an anonymized 30-file application corpus: 108/113 icon usages auto-converted (96 %), 5 brand logos flagged.

### Step 3 — Verify transform output

```bash
# Every transformed file must still parse:
for f in <sandbox>/*.tsx; do
  bun build "$f" --target=browser >/dev/null 2>&1 || echo "SYNTAX: $f"
done
rg -c "MIGRATION-TODO" <sandbox>/*.tsx   # remaining manual work by file
```

The machine-readable map `migration/mui-m3-map.json` is the authoritative source for all component, prop, event, and icon mappings. Regenerate after editing: `cd migration/codemods && bun run scripts/export-map-json.ts`.

---

## Path B — Codemods not available (manual migration)

Use the same component/prop/event knowledge inline. Work file by file:

### Component name map (selected)

Replace `@mui/material` imports with `@aphrody-code/m3-react`:

| MUI component            | M3 React wrapper                                     |
| ------------------------ | ---------------------------------------------------- |
| `Button` (contained)     | `MdFilledButton`                                     |
| `Button` (outlined)      | `MdOutlinedButton`                                   |
| `Button` (text)          | `MdTextButton`                                       |
| `TextField` (outlined)   | `MdOutlinedTextField`                                |
| `TextField` (filled)     | `MdFilledTextField`                                  |
| `Checkbox`               | `MdCheckbox`                                         |
| `Switch`                 | `MdSwitch`                                           |
| `Slider`                 | `MdSlider`                                           |
| `Select` (outlined)      | `MdOutlinedSelect` + `MdSelectOption`                |
| `Dialog`                 | `MdDialog`                                           |
| `IconButton`             | `MdIconButton`                                       |
| `Tabs` / `Tab`           | `MdTabs` / `MdPrimaryTab` or `MdSecondaryTab`        |
| `LinearProgress`         | `MdLinearProgressIndicator`                          |
| `CircularProgress`       | `MdCircularProgressIndicator`                        |
| `Chip`                   | `MdChip` / `MdFilterChip` / `MdInputChip`            |
| `Fab`                    | `MdFab`                                              |
| `Card`                   | `MdElevatedCard` / `MdFilledCard` / `MdOutlinedCard` |
| `Snackbar`               | `MdSnackbar`                                         |
| `Tooltip`                | `MdTooltip`                                          |
| `Divider`                | `MdDivider`                                          |
| `List` / `ListItem`      | `MdList` / `MdListItem`                              |
| `Menu` / `MenuItem`      | `MdMenu` / `MdMenuItem`                              |
| `Box` / `Stack` / `Grid` | `<div>` + Tailwind utilities                         |

### Renamed props

| MUI prop                   | M3 prop          | Component        |
| -------------------------- | ---------------- | ---------------- |
| `checked`                  | `selected`       | Switch, Checkbox |
| `title`                    | `text`           | Tooltip          |
| `open`                     | `opened`         | Drawer           |
| `value` (Tabs)             | `activeTabIndex` | Tabs             |
| LinearProgress value 0-100 | normalize to 0-1 | LinearProgress   |

### Slots

MUI sub-components become slotted content:

| MUI slot component   | M3 equivalent           |
| -------------------- | ----------------------- |
| `DialogTitle`        | `<div slot="headline">` |
| `DialogContent`      | `<div slot="content">`  |
| `DialogActions`      | `<div slot="actions">`  |
| `CardHeader` (title) | `<div slot="headline">` |

### Icon migration (manual)

Convert icon component names: PascalCase -> snake_case, remove style suffix.

Rules:

- Word boundaries + digit boundaries: `ArrowBack` -> `arrow_back`, `Brightness4` -> `brightness_4`
- Style suffix removal: `DeleteOutlined` -> `delete`, `SaveRounded` -> `save`
- Brand logos (GitHub, YouTube, X, ...): keep as SVG + TODO

Replace: `<CloseIcon />` -> `<MdIcon>close</MdIcon>`; import `md-icon` side-effect: `import '@aphrody-code/material-web/icon/icon.js'`

Verify names against `migration/codemods/data/material-symbols-names.json` when present, or at `https://fonts.google.com/icons`.

### Events

MUI `onChange(e, value)` -> native DOM `input`/`change` event; read `e.target.value`. Namespaced events on fork components: `table:sort`, `stepper:change`, etc. (check component docs).

---

## Step 4 — Material Symbols font

Load variable ranges (not a frozen instance) so the axis tokens work:

```ts
import { ensureMaterialSymbols } from "@aphrody-code/material-web/icon/material-symbols.js";

// Development / prototype: full font, variable ranges
ensureMaterialSymbols();

// Production: subset to only the icons used
ensureMaterialSymbols({ iconNames: ["home", "search", "settings", "close"] });
```

Collect the icon set after migration:

```bash
rg -oN '<md-icon>([a-z_]+)</md-icon>' -r '$1' src | sort -u
```

The variable-range URL is:

```
https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200&display=block
```

Never use the frozen `@24,400,0,0` URL — it makes `--md-icon-fill` and `--md-icon-wght` tokens have no effect.

Self-hosted alternative:

```ts
import { ensureMaterialSymbolsFontFace } from "@aphrody-code/material-web/icon/material-symbols.js";
ensureMaterialSymbolsFontFace("/fonts/MaterialSymbolsOutlined.woff2");
```

Files: `google/material-design-icons` repo, `variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].woff2` (Apache-2.0).

## Step 5 — The sx wall (manual, ~30 % of effort)

`sx={{...}}` and `theme.palette/spacing/alpha` are Emotion runtime — no equivalent in material-web. Each occurrence becomes:

- Layout / spacing (margin, padding, display, gap, width) -> Tailwind utility classes on the host element (4 dp grid: 1=4px, 2=8px, 3=12px, 4=16px, ...).
- Internal styling -> `--md-sys-*` CSS custom properties. The Shadow DOM is NOT reachable by Tailwind (see `migration/06-tailwind-material-web.md`).

The codemod only flags these with `MIGRATION-TODO`. This is the irreducible manual part. Count `MIGRATION-TODO` markers to track progress.

## Step 6 — Lint continuously

Install and run `@aphrody-code/eslint-plugin-m3` throughout the migration to catch what the codemods leave:

```bash
bun add -D @aphrody-code/eslint-plugin-m3
```

`.oxlintrc.json`:

```json
{
  "jsPlugins": ["./node_modules/@aphrody-code/eslint-plugin-m3/index.js"],
  "rules": {
    "m3/no-sx-prop": "error",
    "m3/valid-icon-name": "error",
    "m3/valid-color-role": "error",
    "m3/no-mui-prop-on-md": "error",
    "m3/no-mui-import": "warn",
    "m3/prefer-icon-token": "warn",
    "m3/require-icon-button-label": "warn",
    "m3/no-hardcoded-color": "warn"
  }
}
```

> `jsPlugins` paths resolve relative to the `.oxlintrc.json` file.

8 rules; verified on oxlint 1.67. Two are auto-fixable (`valid-icon-name`,
`no-mui-prop-on-md`) — run `oxlint --fix` (or `eslint --fix`) to rename leftover
MUI prop names and snake_case the icon glyphs in one pass. This plugin is the
continuous complement to the codemods.

## Step 7 — Theme -> tokens

Replace a hand-rolled MUI `createTheme` with a single seed via `@aphrody-code/m3-tokens` dynamic-color:

```ts
import { applyDynamicColor } from "@aphrody-code/m3-tokens/dynamic-color";

// Re-theme the whole document from a brand seed color:
applyDynamicColor("#6750A4", { dark: false });
```

This derives all ~47 `--md-sys-color-*` roles (light + dark) via `SchemeTonalSpot` from `material-color-utilities`. See `migration/02-theme-token-migration.md`.

## Finish

Run `m3-correct` and `m3-spec-check` skills to catch remaining issues. For autonomous whole-codebase migration, invoke the `md-migrator` agent.

```bash
bun run build
bun run typecheck
```

Both must exit 0.

## Key facts

- bun only (no npm/pnpm). Codemods are CommonJS jscodeshift; run `bun run test` in `migration/codemods` (fixtures must stay green).
- Component coverage is solved: ~120 `md-*` tags, covering the full `@mui/material` v9 surface + MUI X Community (Data Grid, charts, date/time pickers, tree, scheduler).
- The real migration wall is `sx`/Emotion styling — measured at ~30 % of effort on the anonymized reference corpus.
- MUI X Premium (row grouping, pivot, Excel export, recurring events) is out of scope (commercial MUI offering).
