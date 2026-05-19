---
name: m3-component
version: "1.0.0"
description: Scaffolds a new Material Design 3 component wrapper under packages/ui/components/<name>/ — generates the .tsx wrapper around the matching `<md-*>` Material Web element, the .css custom-property bridge, the bun:test test suite, and updates components/index.ts. Mirrors the Button POC exactly. Use whenever the user asks to "add a new M3 component", "scaffold Input", "wrap md-card", "create the dialog wrapper", or when refactoring a shadcn primitive into its M3 equivalent. Skip for non-M3 / non-UI work.
argument-hint: <component-name>
allowed-tools: Read, Edit, Write, Glob, Grep, Bash
---

# m3-component — scaffold a Material Web 3 wrapper

Generates a production-ready React wrapper for a Material Design 3
component, following the **Button POC** exactly (see
`packages/ui/components/button.{tsx,css,test.tsx}`). Zero placeholder, zero
TODO comments — if a variant/size has no M3 equivalent, fail loud rather
than emit a stub.

The shadcn ↔ M3 mapping table is in
`docs/research/SHADCN_M3_MAPPING.md` (see `references/mapping.md` for the
extracted lookup table this skill needs).

## When to use

- User typed `/m3-component <name>` (where `<name>` is one of input,
  select, card, dialog, tabs, navigation, snackbar, checkbox, radio,
  switch, badge, avatar, progress, dropdown-menu, …)
- User asks to "add the Input wrapper" / "scaffold the Card component"
- After `pixel-perfect` audit identified a missing component
- During the planned 13-component migration sprint

## When to skip

- The component already exists at `packages/ui/components/<name>.tsx` →
  defer to `pixel-perfect` skill for an audit instead
- The user requested a non-M3 wrapper (plain HTML, custom CSS-only) →
  this skill is M3-specific
- The shadcn name has no clean M3 mapping (e.g. `aspect-ratio`,
  `scroll-area`, `resizable`) — flag it and stop, don't fabricate

## Workflow

1. Validate `<name>` exists in `references/mapping.md`. If missing → stop
   with the list of supported names.
2. Read `packages/ui/components/button.tsx` as the source-of-truth
   template (variant-map, useLayoutEffect+setAttribute, addEventListener).
3. Read `packages/ui/components/button.css` for the
   `--md-sys-*` → `--aph-<name>-*` bridge pattern.
4. Read `packages/ui/components/button.test.tsx` for the bun:test +
   happy-dom pattern.
5. Generate the 3 files at `packages/ui/components/<name>.{tsx,css,test.tsx}`:
   - `.tsx`: side-effect-import every Material Web element you wrap, declare
     JSX intrinsic types, map shadcn variants to M3 tags via a
     `Record<Variant, ElementType>` table.
   - `.css`: declare `--aph-<name>-*` custom properties bridged to the
     correct `--md-sys-*` tokens (color/typescale/shape/elevation per the
     M3 component spec).
   - `.test.tsx`: at least 3 cases — variant→tag mapping, primary event
     (click/change/submit), pass-through of HTML attributes (disabled,
     name, value, form, href as relevant).
6. Update `packages/ui/components/index.ts` to re-export `<Component>` and
   side-effect-import `<name>.css`.
7. Tick the migration tracker row in `packages/ui/README.md`.
8. Run `bun test packages/ui/components/<name>.test.tsx` and report exit
   code. **Do not mark success unless 3/3 tests pass.**

## Anti-stub clause

- Never emit a wrapper that doesn't render an actual `<md-*>` element
- Never copy the Button variant list verbatim into another component —
  match the M3 spec for THAT component (Card has elevated/filled/outlined,
  Dialog has standard/full-screen, etc.)
- Never write a `// TODO: implement variant X` comment — either implement
  the variant via the matching `<md-*>` tag, or remove it from the
  variant union type entirely
- If `@material/web` doesn't ship an element for the requested wrapper,
  stop and report (don't ship a stub div)

## Pre-requisites

- `packages/ui/components/button.tsx` exists (POC reference)
- `@material/web` v2.4+ resolves (`npm:@material/web@^2.4.1` in
  `packages/ui/package.json`)
- `happy-dom` and `bun:test` available
