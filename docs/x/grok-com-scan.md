<!-- SPDX-License-Identifier: Apache-2.0 -->
# grok.com — bxc detect / recon (2026-06-03)

Target: [https://grok.com/](https://grok.com/)

Cookie jar: `~/.bxc/cookies/grok.json` (shortcut `grok`), mirrored to `~/.aphrody/cookies/grok.json`.

## Commands run

```bash
# Without cookies (baseline)
bxc detect https://grok.com/ --json
bxc recon https://grok.com/ --profile stealth --timeout 60000

# With cookie jar (stealth browser)
cd ~/bxc && bun run scripts/grok-scan-with-cookies.ts
```

## Results summary

| Probe | HTTP | Notes |
| --- | --- | --- |
| `bxc detect` (fetch) | **403** | Cloudflare; CDN = Cloudflare; NS = `*.ns.cloudflare.com` |
| `bxc recon` (stealth, no cookie CLI flag) | **403** | Challenge page; CSP allows `challenges.cloudflare.com` |
| Stealth browser + `cookies: "grok"` | **403** body | Title “Just a moment…” — CF managed challenge; clearance likely IP-bound or stale |

Resolved IPs: `104.18.28.234`, `104.18.29.234` (Cloudflare anycast).

### Detect (CDN / DNS only)

- **CDN:** Cloudflare (`cf-ray` header, IP range)
- **DNS:** Cloudflare nameservers
- **Frameworks:** none (body is challenge HTML, not app bundle)

### Recon assets (challenge shell)

- Script: `/cdn-cgi/challenge-platform/h/g/orchestrate/chl_page/v1`
- Selectors: `#challenge-error-text`, `.main-content`, `body`, `html`

## Artifacts (local, not committed)

Full JSON and HTML snapshots live under:

`docs/x/reports/grok-com/` (gitignored via `docs/x/reports/`)

Files: `detect.json`, `recon.json`, `browser-html-head.txt`, `browser-final-url.txt`, `README.md`.

## Recommendations

1. Refresh `cf_clearance` and `__cf_bm` from the same machine/IP as the VPS, or run `bxc cookies extract grok.com` on a desktop Chrome profile (if available).
2. For agent automation of Grok chat, use **Grok Build CLI** / **xAI API** — not bxc scrape of grok.com.
3. For X/Twitter automation, use `bxc x` / `aphrody-x` with `auth_token` + `ct0` (see [bxc-integration.md](bxc-integration.md)).