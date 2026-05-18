---
description: Recon + targeted scrape on a URL via the aphrody CLI (which auto-starts the bxc Bun daemon on demand). Returns CDN, framework detection, headers, css selectors, status, bytes, and (optionally) selector-scoped text extraction.
argument-hint: <url> [css-selector]
allowed-tools: Bash, Read, Write
---

# /scrape — aphrody recon + scrape (auto-start bxc)

Run a two-stage scrape against `$ARGUMENTS` via the native `aphrody` CLI.
The CLI auto-spawns the bxc Bun daemon on `localhost:8765` if it is not
already running, polls `/healthz` until ready, then hits `/api/recon` and
`/api/scrape`.

`$ARGUMENTS` is parsed as `<url> [selector]`:

- The first token is the URL (must start with `http://` or `https://`).
- The remainder (if any) is treated as the CSS selector for the scrape
  stage. If absent, only the recon stage runs.

## Steps

1. **Recon**
   - Run `aphrody bxc recon <url>` and capture stdout (JSON envelope
     `bxc-recon-v1` : `{url, finalUrl, httpStatus, bytes, cssSelectors,
     frameworks, headers:{cdnVendor, server, ...}, gotoMs}`).
   - If the CLI exits non-zero, print stderr verbatim and stop. Common
     causes: bun missing on PATH (install from https://bun.sh) or
     `packages/bxc/` absent (re-clone with `gh repo clone aphrody-code/bxc`).

2. **Detect** (always — fast and yields the canonical CDN/framework view)
   - Run `aphrody bxc detect <url>` for the rich CDN/framework/DNS/CMS
     breakdown (`{cdn:[{name,evidence,source,confidence}], dns, frontend,
     backend, …}`). Merge into the recon report.

3. **Selector scrape** (only if a selector was provided)
   - Run `aphrody scrape --selector "<selector>" <url>` and capture stdout
     (`{url, selector, matches:[{index, text}]}`).

4. **Report**
   - Print a one-screen summary:
     - **URL** (+ finalUrl after redirects)
     - **httpStatus** + **bytes** + **gotoMs**
     - **CDN** (`cdn[].name` joined) + **frameworks** (`frameworks[]`)
     - **CSS selectors discovered** (`cssSelectors[]`, first 10)
     - **Selector extractions** (if any): up to 20 first matches, one per
       line, truncated to 200 chars each.
   - Persist the merged JSON `{recon, detect, scrape?}` to
     `./.aphrody/scrapes/<sha1-of-url>.json` (create the directory if
     missing). Print the file path.

## Anti-stub

- Do not fabricate output if `aphrody` exits non-zero. Surface stderr
  verbatim and stop.
- Do not invent CDN/framework names that the CLI did not return.
- Do not skip the persistence step — every scrape is an audit artifact.
