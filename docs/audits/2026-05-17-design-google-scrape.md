<!-- SPDX-License-Identifier: Apache-2.0 -->

# Audit — `design.google` scrape via bxc mass-scrape

**Date (UTC):** 2026-05-17
**Initiator:** user request — `https://design.google/`
**Pipeline:** `scripts/bxc-mass-scrape.ts` + bxc local clone (`C:/worktree/bxc`)
**URL list:** `scripts/bxc-mass-scrape.design-google.urls.json` (10 URLs)
**Cache dir:** `var/data/bxc-cache/` (merged with the 127-URL corpus)
**Run flags:** `--profile=static --concurrency=4 --retry=2 --timeout=25000`

## 1. Run summary

| Metric           | Value          |
|---|---|
| URLs requested   | 10             |
| OK               | 10             |
| Failed           | 0              |
| Total bytes      | 358.2 KB       |
| Total time       | 1132 ms        |
| Throughput       | ~9 URL / s     |

All 10 URLs returned HTTP 200 + non-empty body. Cache files written
with `<sha256-of-url>.html` naming + merged into the main
`var/data/bxc-cache/manifest.json`.

## 2. Per-URL byte sizes — what the numbers say

| URL                                                 | Bytes  | Content classification         |
|---|---|---|
| `https://design.google/`                            | 92 791 | **Real SSR content** (home)   |
| `https://design.google/about`                       | 49 829 | **Real SSR content** (about)  |
| `https://design.google/library`                     | 28 029 | App-shell + minimal SSR       |
| `https://design.google/library/topics`              | 28 028 | **SPA shell** (no SSR)        |
| `https://design.google/events`                      | 28 028 | **SPA shell** (no SSR)        |
| `https://design.google/products`                    | 28 028 | **SPA shell** (no SSR)        |
| `https://design.google/library/material-3-design-tokens` | 28 028 | **SPA shell** (no SSR)   |
| `https://design.google/library/material-design-3`   | 28 028 | **SPA shell** (no SSR)        |
| `https://design.google/library/design-systems`      | 28 028 | **SPA shell** (no SSR)        |
| `https://design.google/library/accessibility`       | 28 028 | **SPA shell** (no SSR)        |

`design.google/` is a Next.js app:

- The **home** + **about** pages render server-side (full HTML in the response).
- Most **subroutes** ship only the empty app shell (`28 028 B` exactly) —
  the article body is hydrated by JavaScript on the client. With
  `profile: "static"` (in-process StaticDomTransport, no JS execution)
  we capture the shell but not the article content.

## 3. What the SSR'd home gave us (useful signal)

From the 92 KB `https://design.google/` payload:

- **Title:** "Google Design - Discover the people and stories behind the
  products"
- **Description:** "Design resources and inspiration from Google —
  including the Material Design system, Google Fonts, and the people
  and processes behind the products."
- **Social tags:** complete `og:` + `twitter:` open-graph block.
- **CSS module fingerprints:** `Navigation_nav__logo__xP0dZ`,
  `Navigation_nav__skip__uEOfT`, etc. — Next.js CSS modules with
  hashed class names. Confirms Next.js + CSS modules tech stack (no
  Tailwind).
- **Inline SVG iconography:** menu glyph `M22 8H2V10H22V8Z…`, close
  glyph `M8.29688 18.59L14.8769 12L8.29688 5.41L9.70687 4L17.7069…` —
  Material symbols at 24×24.

## 4. Limitation + next step for full SPA capture

`profile: "static"` in bxc is the in-process StaticDomTransport
(no Chrome, no Lightpanda, zero binary). For SPA routes that need JS
hydration to materialise their article content, we need a binary-backed
profile:

| Profile     | Binary required          | Use case                  |
|---|---|---|
| `static`    | none (in-process DOM)    | Static HTML / SSR pages   |
| `http`      | none (curl-impersonate)  | Fastest, HTTP-only        |
| `fast`      | bxc-engine OR Chrome     | Light SPAs                |
| `stealth`   | bxc-engine + ja3 spoof   | Anti-bot SPAs             |
| `max`       | bxc-engine full stack    | Hostile anti-bot sites    |

The `bxc-engine` binary is **not currently installed** on this host
(prior smoke test confirmed `"Process exited before emitting ws url.
Output:"`). Install path:

```pwsh
# Option A — install Lightpanda (peer-managed, depends on winclean side)
# See C:\winclean\... or peer Claude's notes.

# Option B — use bun-native fetch with manual JS execution via jsdom
# (slower, but fully aphrody-local, no peer dep).
```

Once `bxc-engine` is present, re-run with:

```pwsh
.\scripts\bxc-mass-scrape.ps1 -Urls scripts/bxc-mass-scrape.design-google.urls.json `
  -Profile fast -Mode full -Force
```

The orchestrator already passes `mode=full` correctly through to
`Browser.newPage({ mode, profile })`.

## 5. Files committed in this pass

- `scripts/bxc-mass-scrape.design-google.urls.json` — 10-URL design.google subset
- `docs/audits/2026-05-17-design-google-scrape.md` — this report

Cache contents are gitignored (`var/`) so the 358 KB of HTML stays
local. Manifest entries are visible to any tool consuming
`var/data/bxc-cache/manifest.json`.
