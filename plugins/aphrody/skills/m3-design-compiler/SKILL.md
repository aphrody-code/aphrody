---
name: m3-design-compiler
description: >
  Turn a natural-language design brief into a Material Design 3 React scaffold with
  @aphrody-code/m3-design. Use when asked to "scaffold a screen from a description",
  "generate an M3 layout from a brief", "design compiler", "brief to JSX", or "make an
  M3 prototype/dashboard/deck from this prompt".
---

# m3-design-compiler

`@aphrody-code/m3-design` is the natural-language design compiler of the monorepo:
a deterministic brief -> M3 React scaffold generator. It parses a free-text brief,
runs an HCT tonal-palette engine, compiles a layout tree of `@aphrody-code/m3-react`
(`Md*`) components, and self-critiques the result. Good for agents/tooling that need
a self-contained, type-checked React artifact from one prompt (no visual form).

## API

```ts
import { DesignCompiler, parseBrief } from "@aphrody-code/m3-design";

const result = new DesignCompiler().compile(
  "a compact analytics dashboard, dark blue, with a list-detail layout and live streaming",
  // seedColor?  — optional hex; default is the M3 baseline #6750A4 (parsed from the brief if a color is named)
  // designSystemId? — optional token overrides from a shipped design-system fixture
);
// result: GenerationResult — index.tsx layout code, theme.css (HCT palette), token JSON, critique scores
```

`parseBrief(rawBrief, seedColor?)` exposes the structured directives the compiler
extracts, when you only want the parse:

- **Output kind**: `deck` | `prototype` | `dashboard` | `mobile` | `editorial`.
- **Adaptive platform**: `desktop-web` | `ios` | `responsive-web`.
- **Layout**: `scaffold` | `list-detail` | `supporting-pane` | `feed`.
- **Density**: `high` (compact) | `default` | `low`.
- **Seed color**: detected hex or color reference (default `#6750A4`).
- **Feature toggles**: streaming, thinking/shimmer indicators, central action-router.

## What it emits

- `index.tsx` — a layout tree built from real `Md*` wrappers (Scaffold, NavigationRail,
  cards, lists, etc.), aliased to friendly names.
- `theme.css` — an HCT tonal palette (18 tone steps) mapped to `--md-sys-*` roles.
- W3C design-token JSON.
- A self-critique payload (scores + rationale) for autonomous iteration.

## When to use vs `m3-template`

- `m3-design-compiler` — programmatic, deterministic, brief-in / artifact-out;
  ideal for batch/agent generation and when you want the HCT theme + critique.
- `m3-template` (skill) — interactive scaffolding of a canonical M3 page with the
  adaptive shell; reach for it when a human is iterating in-repo.

Verify in-repo: `cd packages/m3-design && bun test` (parser + generator suites).
