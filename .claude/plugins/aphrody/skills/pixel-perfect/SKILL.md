---
name: pixel-perfect
version: "1.0.0"
description: Validates a Material Design 3 component implementation against the upstream M3 spec. Use when the user asks to audit M3 tokens, compare a shadcn/aphrody-code/ui component to its Material Web 3 reference, check that CSS custom properties match the `--md-sys-*` contract, or verify visual fidelity of a refactored component. Skip for non-M3 work, raw Tailwind theming, or arbitrary visual regression unrelated to Material Design.
argument-hint: <component-name-or-path>
allowed-tools: Read, Edit, Write, Glob, Grep, Bash, mcp__aphrody__bxc_scrape, mcp__aphrody__bxc_recon, mcp__aphrody__vision_analyze, mcp__aphrody__extract_structured
---

# pixel-perfect — Material Design 3 component auditor

Audit a candidate component (typically under
`packages/ui/components/<name>/`) for fidelity to the Material Design 3
contract. The skill compares: (1) the component's declared CSS custom
properties, (2) its DOM tag usage against the canonical
`<md-*>` element from `material-web`, and (3) optionally a screenshot
of the rendered component against the M3 reference page on
`m3.material.io`.

Cross-references:
- `references/m3-spec.md` — distilled M3 contract (color/typescale/shape/
  motion/elevation tokens) extracted from the docs/research mapping.
- `references/validation-checklist.md` — exact per-axis checklist used in
  the workflow below.

## When to use

Trigger automatically when the user asks any of:

- "audit this M3 component" / "is this Material 3-compliant?"
- "check the design tokens for `<name>`"
- "compare our `Button` to `<md-filled-button>`"
- "validate the `--md-sys-color-*` mapping"
- after the `n2b-ultra` agent finishes migrating a UI package

## When to skip

Skip if:

- the project area is not under `packages/ui/` or
  `aphrody-code/ui#aphrody`
- the component does not claim M3 compliance (raw Tailwind / shadcn
  baseline without M3 retrofit)
- the user is doing a one-off Tailwind tweak, color picking, or generic
  visual diff unrelated to M3

## Workflow (must follow in order)

1. **Locate the component**
   - Resolve `$ARGUMENTS` to either a component name (resolved via
     `Glob` on `packages/ui/components/$ARGUMENTS/**`) or a direct file
     path. If ambiguous, use the first match alphabetically and record
     the choice in the report.
   - Read every file in the component dir (`.tsx`, `.ts`, `.css`,
     `spec.json` if present).

2. **Map shadcn → M3**
   - Open `references/m3-spec.md` and find the entry that matches the
     component's variant family (button, input, card, dialog, ...).
   - Confirm the component imports the matching Material Web element
     (`@material/web/<family>/<element>.js`) and exposes the M3 variants
     listed in the spec (`filled`, `outlined`, `text`, `tonal`,
     `elevated`, `fab` for buttons, etc.).
   - If the wrapper still pulls Radix, CVA, or Base UI primitives,
     flag this as a P0 violation.

3. **Token audit**
   - Walk every CSS rule in the component and verify each color, type,
     shape, and motion declaration resolves to a `--md-sys-*` token from
     `references/validation-checklist.md`. Hard-coded hex / px /
     `cubic-bezier(...)` literals are P1 violations.
   - Run `Grep -n "--md-sys-" packages/ui/tokens/m3.json` to confirm
     each referenced token exists in the scraped token bundle. Missing
     tokens are P0 — invoke `/tokens` to refresh the bundle.

4. **Visual diff (optional but recommended for HIGH-priority components)**
   - Call `mcp__aphrody__bxc_recon` on the canonical M3 reference
     URL listed in `references/m3-spec.md` for the family.
   - Call `mcp__aphrody__vision_analyze` on the resulting
     `screenshot_path` to extract elements/colors/fonts.
   - Compare extracted colors against the component's declared tokens.
     Any delta-E > 5 in primary/secondary/error is a P1 violation.

5. **Emit the report**
   - Write `packages/ui/components/<name>/spec.report.md` containing:
     - resolved component path
     - M3 family + Material Web element used
     - token audit table (token, found?, source line)
     - violations grouped by severity (P0 must-fix, P1 should-fix,
       P2 nice-to-have)
     - copy-paste fixes for each P0 (concrete diff snippets)
   - Print a one-line summary to stdout:
     `pixel-perfect: <name> — <P0> P0 / <P1> P1 / <P2> P2`.

## Outputs

- Always: `packages/ui/components/<name>/spec.report.md`.
- On P0 violations: a non-zero exit (raise to caller via tool return).
- When `/tokens` had to be re-run: a note at the top of the report with
  the new token count.

## Anti-stub rule

If any audit axis cannot be evaluated (e.g. `vision_analyze` returned
`BXC_UNAVAILABLE`), the report must mark that axis `unaudited (reason:
<error>)` — never silently pass. Do not invent token values, do not
fabricate Material Web imports, do not write `TODO` lines.
