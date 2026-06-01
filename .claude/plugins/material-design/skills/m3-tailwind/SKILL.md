---
name: m3-tailwind
description: >
  Wire Material Design 3 tokens into Tailwind v4 and shadcn/ui with @aphrody-code/m3-theme
  and @aphrody-code/m3-tokens. Use when asked to "use M3 with Tailwind", "M3 + shadcn",
  "Tailwind v4 @theme with --md-sys-* tokens", "style the host of an md-* with Tailwind",
  "fusion tokens", or "map shadcn color aliases to M3 roles".
---

# m3-tailwind

How M3 tokens meet Tailwind v4 and shadcn/ui in this monorepo. Two layers:

## 1. Runtime tokens for Tailwind — `@aphrody-code/m3-tokens/m3-tokens.css`

The lib only emits the ~49 `--md-sys-color-*` color roles at runtime; typescale,
shape, elevation and motion are resolved at Sass compile-time. To consume the full
token set from Tailwind `@theme`, import the runtime sheet (62 vars):

```css
/* app.css */
@import "tailwindcss";
@import "@aphrody-code/m3-tokens/m3-tokens.css"; /* :root { --md-sys-* } */

@theme inline {
  --color-primary: var(--md-sys-color-primary);
  --color-surface: var(--md-sys-color-surface);
  --color-on-surface: var(--md-sys-color-on-surface);
  /* … map the roles you use → Tailwind color utilities */
}
```

Now `bg-surface text-on-surface` etc. follow the M3 theme (and dynamic-color, dark
mode) automatically. Details + limits: `migration/06-tailwind-material-web.md`,
`migration/09-coverage-tailwind.md`.

## 2. M3 + shadcn fusion sheet — `@aphrody-code/m3-theme`

`@aphrody-code/m3-theme` ships ONE generated stylesheet (`tokens.css`, light + dark)
that maps the **shadcn/ui** color aliases (`background`, `foreground`, `card`,
`popover`, `muted`, `accent`, `border`, `ring`, `destructive`, …) onto the M3
`--md-sys-color-*` roles, and also emits a Tailwind `@theme inline` block. Use it
when a project already speaks shadcn/Tailwind and you want it themed by M3.

```css
@import "@aphrody-code/m3-theme/tokens.css"; /* :root (M3) + :root (shadcn aliases) + @theme + .dark */
```

It is self-contained: `packages/m3-theme/generate.ts` builds `tokens.css` from the
M3 baseline palette + the shadcn alias map (no external CLI). Regenerate after
editing the palette/aliases:

```bash
cd packages/m3-theme && bun run generate    # bun generate.ts && oxfmt tokens.css
```

## The hard rule — host only, never shadow DOM

Tailwind classes and `--md-sys-*` cascade only reach the **host** of an `md-*` element (margin, padding, layout, and the color roles the component reads). The component's internal shadow DOM is NOT reachable by Tailwind selectors. So:

- Layout / spacing around components -> Tailwind utilities on the host (4 dp grid: `p-2` = 8px, `gap-4` = 16px).
- Component internals -> drive via `--md-sys-*` / component `--md-*` custom properties (which DO pierce the shadow boundary as inherited custom properties).

This is also enforced by `@aphrody-code/eslint-plugin-m3` (`m3/no-sx-prop`, `m3/no-hardcoded-color`). See the `m3-lint` skill.

## 3. Tailwind Oxide Rust compilation & performance engine

Under the hood, Tailwind CSS v4 delegates utility scanning to a native Rust engine (`tailwind-oxide`) to achieve O(N) lookup speeds and sub-millisecond incremental builds. Key design choices we can adopt in `bun-rs` for `material-web` components:

- **State-Machine Extractor (FSM):** Instead of generic regex matching, custom state machines (with states encoded as Rust typestates, e.g., `ArbitraryPropertyMachine<State>`) process inputs at the byte level. Speculative cursors are cloned cheaply to run parallel enums/variant matching without backtrack penalties.
- **Branchless Byte Classifiers:** Proc-macros (like `ClassifyBytes`) generate O(1) static lookup arrays matching byte values (0..255) to class classifications, reducing conditional branches in loops to zero.
- **SIMD Whitespace Skipping:** Skips template whitespace/indentation 16 bytes at a time by loading strides into vectors and performing branchless all-whitespace checks.
- **Lock-free Parallel Walkers:** Ignores heavy global locks during directories scans by accumulating matches in thread-local buffers (cap 256) and only syncing via a `Mutex` when full.
- **AST Optimization Pass:** Prunes the output CSS tree recursively by removing unused variables/keyframes and merging identical declarations, keeping runtime selectors clean and GC friendly.

## 4. Running the Tailwind CLI

To run the Tailwind CLI in this monorepo (e.g. for consuming projects, custom themes, or showcase builds):

```bash
# Initializing/compiling via bun (v4 CLI is installed as a monorepo devDependency)
bunx tailwindcss -i input.css -o output.css --minify
```
