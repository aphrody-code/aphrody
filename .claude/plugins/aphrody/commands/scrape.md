---
description: Recon + targeted scrape on a URL via the bxc-scrapper MCP server. Returns headers, CDN, framework detection, asset inventory, screenshot path, and (optionally) selector-scoped text extraction.
argument-hint: <url> [css-selector]
allowed-tools: mcp__bxc-scrapper__bxc_recon, mcp__bxc-scrapper__bxc_scrape, Read, Bash
---

# /scrape — bxc recon + scrape

Run a two-stage scrape against `$ARGUMENTS` and present the result.

`$ARGUMENTS` is parsed as `<url> [selector]`:

- The first token is the URL (must start with `http://` or `https://`).
- The remainder (if any) is treated as the CSS selector for the scrape
  stage. If absent, default to `body` so the recon stage is the only
  payload returned.

## Steps

1. **Recon**
   - Call `mcp__bxc-scrapper__bxc_recon` with `{ "url": "<url>" }`.
   - Parse the returned JSON: `{headers, cdn, frameworks, assets, css,
     screenshot_path}`.
   - If the tool returns `BXC_UNAVAILABLE`, report it verbatim with the
     hint: "start the bxc daemon (`bun run dev` in `packages/bxc`) or
     install the `bxc-engine` binary on PATH". Stop.

2. **Selector scrape** (only if a selector was provided)
   - Call `mcp__bxc-scrapper__bxc_scrape` with
     `{ "url": "<url>", "selector": "<selector>" }`.
   - Capture the array of text extractions.

3. **Report**
   - Print a one-screen summary:
     - **URL** + final URL (after redirects, from `headers`).
     - **CDN** + **Frameworks** (joined, comma-separated).
     - **Assets** counts (`js`, `css`, `img`, `font`).
     - **Screenshot** path (`screenshot_path`) — note that it is local
       to the bxc daemon's filesystem.
     - **Selector extractions** (if any): up to 20 first matches, one
       per line, truncated to 200 chars each.
   - Persist the full JSON to `./.aphrody/scrapes/<sha1-of-url>.json`
     (create the directory if missing). Print the file path.

## Anti-stub

- Do not fabricate output if either MCP call fails. Surface the error
  verbatim and stop.
- Do not invent CDN/framework names that the tool did not return.
- Do not skip the persistence step — every scrape is an audit artifact.
