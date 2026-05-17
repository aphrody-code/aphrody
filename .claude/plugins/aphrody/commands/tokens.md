---
description: Scrapes Material Design 3 design tokens from m3.material.io (or any URL provided) and writes a normalized JSON bundle to packages/ui/tokens/m3.json.
argument-hint: [url]
allowed-tools: mcp__bxc-scrapper__bxc_recon, mcp__bxc-scrapper__bxc_scrape, mcp__bxc-scrapper__extract_structured, Read, Write, Bash, Glob
---

# /tokens — Material Design 3 token scraper

Scrape the canonical M3 design tokens from `$ARGUMENTS` (default:
`https://m3.material.io/foundations/design-tokens`) and emit a single
normalized JSON file at `packages/ui/tokens/m3.json` consumed by the
`pixel-perfect` skill and the `aphrody-code/ui#aphrody` components.

## URL resolution

- `$ARGUMENTS` empty → use
  `https://m3.material.io/foundations/design-tokens`.
- `$ARGUMENTS` is a bare path → prepend `https://m3.material.io/`.
- Otherwise treat as a full URL.

## Steps

1. **Recon**
   - Call `mcp__bxc-scrapper__bxc_recon` on the URL. Confirm
     `frameworks` includes the Google Wiz framework (sanity check; warn
     if not, do not fail).

2. **Scrape token table**
   - Call `mcp__bxc-scrapper__bxc_scrape` with selector
     `pre, code, table, section[data-token-table]` to harvest the raw
     token definitions.

3. **Normalize via extract_structured**
   - Call `mcp__bxc-scrapper__extract_structured` with the page HTML
     (re-fetched if needed) and the following Zod schema serialized as
     JSON:

     ```json
     {
       "type": "object",
       "properties": {
         "color":     { "type": "object", "additionalProperties": { "type": "string" } },
         "typescale": { "type": "object", "additionalProperties": { "type": "object", "additionalProperties": { "type": ["string","number"] } } },
         "shape":     { "type": "object", "additionalProperties": { "type": "string" } },
         "motion":    {
           "type": "object",
           "properties": {
             "duration": { "type": "object", "additionalProperties": { "type": "string" } },
             "easing":   { "type": "object", "additionalProperties": { "type": "string" } }
           },
           "required": ["duration", "easing"]
         },
         "elevation": { "type": "object", "additionalProperties": { "type": "string" } },
         "state":     { "type": "object", "additionalProperties": { "type": ["string","number"] } }
       },
       "required": ["color","typescale","shape","motion","elevation","state"]
     }
     ```

   - The tool returns a typed JSON. Validate the top-level keys are
     present; if any are missing, abort and report which key is empty
     (do not write a partial bundle to disk).

4. **Write**
   - Ensure `packages/ui/tokens/` exists (`mkdir -p` via `Bash`,
     portable to Windows: `bun -e "import('node:fs').then(fs => fs.mkdirSync('packages/ui/tokens', { recursive: true }))"`).
   - `Write` the JSON to `packages/ui/tokens/m3.json` with 2-space
     indentation and a trailing newline.

5. **Audit summary**
   - Print:
     - Source URL
     - Token counts per top-level key
       (`color: 56`, `typescale.<family>: 15`, etc.)
     - Output file size (`bun -e "console.log(require('node:fs').statSync('packages/ui/tokens/m3.json').size)"`).
   - End with a single line: `tokens: wrote <N> tokens to packages/ui/tokens/m3.json`.

## Anti-stub

- If `extract_structured` returns less than 80 tokens total, do not
  write the file — that indicates a partial scrape (the M3 token page
  has > 400 tokens as of 2026-05-17). Report the count and stop.
- If the MCP server returns `BXC_UNAVAILABLE`, do not fabricate
  defaults; report the error and exit.
- Never edit `packages/ui/tokens/m3.json` by hand from inside this
  command; the file is always rewritten in full.
