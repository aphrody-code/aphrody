---
description: Scrapes Material Design 3 design tokens from m3.material.io (or any URL) via the aphrody CLI (auto-start bxc Bun daemon). Writes a normalized JSON bundle to packages/ui/tokens/m3.json.
argument-hint: [url]
allowed-tools: Bash, Read, Write, Glob
model: sonnet
---

# /tokens — Material Design 3 token scraper (via aphrody CLI)

Scrape the canonical M3 design tokens from `$ARGUMENTS` (default:
`https://m3.material.io/foundations/design-tokens`) via the native
`aphrody tokens` subcommand and emit a normalized JSON file at
`packages/ui/tokens/m3.json` consumed by the `pixel-perfect` skill and the
`aphrody-code/ui#aphrody` components.

## URL resolution

- `$ARGUMENTS` empty → use
  `https://m3.material.io/foundations/design-tokens`.
- `$ARGUMENTS` is a bare path → prepend `https://m3.material.io/`.
- Otherwise treat as a full URL.

## Steps

1. **Sanity recon** (optional but recommended)
   - Run `aphrody bxc recon <url>` to verify the URL responds
     (`httpStatus: 200`) and capture the framework breakdown. Warn (do
     not fail) if `frameworks` is empty — m3.material.io ships its tokens
     through shadow DOM which the recon stage cannot see.

2. **Token scrape**
   - Run `aphrody tokens --url <url> --output packages/ui/tokens/m3.json --force`.
   - The CLI handles : daemon auto-start, `/api/scrape` against the
     `:root` selector, regex parse of `--md-*` custom properties, JSON
     write with 2-space indentation.
   - Capture stdout. The CLI prints
     `M3 tokens written to <path> (<N> entries)`.

3. **Validate output**
   - Read back `packages/ui/tokens/m3.json` and parse it as JSON. Verify
     the top-level shape `{source_url, tokens: {...}}`.
   - Count tokens: `Object.keys(tokens).length`.
   - **If the count is below 80**, the page renders tokens in shadow DOM
     and the naïve `:root` scrape didn't capture them. Fall back to:

     ```bash
     # Use the bxc Bun MCP `extract_structured` tool (richer) via the
     # bxc-scrapper MCP server.
     # Or invoke pixel-perfect skill on a component first to seed tokens.
     ```

     Report the partial count and **do not overwrite** the file.

4. **Audit summary**
   - Print:
     - Source URL.
     - Token counts per top-level CSS prefix
       (`--md-sys-color: 56`, `--md-sys-typescale: 90`, …).
     - Output file size
       (`bun -e "console.log(require('node:fs').statSync('packages/ui/tokens/m3.json').size)"`).
   - End with a single line:
     `tokens: wrote <N> tokens to packages/ui/tokens/m3.json`.

## Anti-stub

- Do not fabricate token values that `aphrody tokens` did not return.
- If the CLI exits non-zero (e.g. bun missing), report stderr verbatim
  and stop.
- Never edit `packages/ui/tokens/m3.json` by hand from inside this
  command; the file is always rewritten in full by `aphrody tokens`.
