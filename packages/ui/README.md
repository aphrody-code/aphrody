# @aphrody-code/ui

Material Design 3 component library for aphrody. **Status: refactor in progress** — migrating from
the shadcn-ui registry surface to thin React wrappers around
[`@material/web`](https://github.com/material-components/material-web) v2 custom elements.

## Why the refactor

The legacy `src/components/*.tsx` files (still shipped under the `@aphrody-code/ui/legacy`
subpath export) targeted a Bun JSX runtime called `Html`. They worked, but they are not React
and they don't compose with the rest of the monorepo. The new `components/` folder ships real
React 19 components with strict TypeScript types and CSS that bridges M3 system tokens to
component-local custom properties.

## Components migrated

| shadcn name | M3 wrapper | File | Status |
|---|---|---|---|
| button      | `<md-*-button>` family | [`components/button.tsx`](./components/button.tsx) | done |
| input       | `<md-*-text-field>`    | -- | TODO |
| select      | `<md-select>`          | -- | TODO |
| checkbox    | `<md-checkbox>`        | -- | TODO |
| radio       | `<md-radio>`           | -- | TODO |
| switch      | `<md-switch>`          | -- | TODO |
| card        | `<md-card>`            | -- | TODO |
| dialog      | `<md-dialog>`          | -- | TODO |
| tabs        | `<md-tabs>`            | -- | TODO |
| navigation  | `<md-navigation-*>`    | -- | TODO |
| snackbar    | `<md-snackbar>`        | -- | TODO |

Variant mapping reference: [`docs/research/SHADCN_M3_MAPPING.md`](../../docs/research/SHADCN_M3_MAPPING.md).

## Design tokens

Tokens scraped from <https://m3.material.io> live at [`tokens/m3.json`](./tokens/m3.json).
They are produced by [`scripts/scrape-m3-tokens.ts`](../../scripts/scrape-m3-tokens.ts), which
drives the local [bxc](https://github.com/aphrody-code/bxc) fork to read the live CSS variables
plus the documentation tables.

Regenerate:

```bash
# Requires bxc cloned at C:/worktree/bxc (or BXC_ROOT=...)
bun run scripts/scrape-m3-tokens.ts --profile=fast
```

The JSON file is schema-validated at write time with Zod; the resulting `M3Tokens` type is
exported from the script.

## Usage

```tsx
import { Button } from "@aphrody-code/ui/button";
import "@aphrody-code/ui/button.css";

export function Example() {
  return (
    <>
      <Button>Save</Button>
      <Button variant="outline" size="lg">Cancel</Button>
      <Button variant="destructive" onClick={() => alert("zap")}>Delete</Button>
      <Button variant="link" href="https://m3.material.io">Spec</Button>
    </>
  );
}
```

The wrapper attaches Material Web custom elements at module load via side-effect imports
(`@material/web/button/filled-button.js`, etc.), so it must run in a DOM-capable environment.

## Adding a component (template)

1. Add `components/<name>.tsx` and `components/<name>.css`.
2. Side-effect-import each Material Web element you wrap.
3. Declare its JSX intrinsic type next to the existing button declarations.
4. Map any shadcn-style variants/sizes to M3 properties via inline class names + CSS vars.
5. Re-export from [`components/index.ts`](./components/index.ts) (and import its CSS there).
6. Add a `components/<name>.test.tsx` with at least three cases: render, event, edge.
7. Tick the row in this README.

## Tests

```bash
bun test packages/ui
```

Tests run under `@happy-dom/global-registrator` so the custom elements register without a
real browser. The first dynamic `import("./button.tsx")` triggers Material Web's element
registration in the happy-dom realm.

## Legacy

The pre-refactor surface remains importable for transition code:

```ts
import { Button as LegacyButton } from "@aphrody-code/ui/legacy";
```

It will be deleted once the React migration covers every legacy component.
