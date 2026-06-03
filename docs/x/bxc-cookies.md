<!-- SPDX-License-Identifier: Apache-2.0 -->
# bxc cookie jar format (Playwright / CDP)

bxc normalises browser exports into **`~/.bxc/cookies/<shortcut>.json`**. Shortcut names are alphanumeric/kebab (e.g. `google`, `grok`, `gemini`) and resolve via `resolveCookiePath()` in the bxc repo.

## JSON shape (required fields)

Each array element:

| Field | Type | Notes |
| --- | --- | --- |
| `name` | string | Cookie name |
| `value` | string | Secret — never log or commit |
| `domain` | string | Leading `.` for subdomain cookies (e.g. `.grok.com`) |
| `path` | string | Usually `/` |
| `expires` | number | UNIX **seconds** (not ms). `-1` or omit → session |
| `httpOnly` | boolean | |
| `secure` | boolean | |
| `sameSite` | `"Strict"` \| `"Lax"` \| `"None"` | DevTools `no_restriction` → `None` |

Also accepted: `expirationDate` (Chrome DevTools), `hostOnly`, `session: true`.

Supported import formats: **Playwright/CDP JSON array**, Chrome DevTools export, **Netscape** `cookies.txt`.

## CLI

```bash
bxc cookies load grok          # validate ~/.bxc/cookies/grok.json
bxc cookies show grok          # metadata (masked values)
bxc cookies list
bxc cookies save grok /path/to/export.json
```

## Programmatic use (detect / recon / scrape)

Pass the shortcut or path into `Browser.newPage({ cookies: "grok", profile: "stealth" })`. The stock `bxc recon` / `bxc detect` CLIs do **not** expose `--cookies` yet; use a small Bun script or MCP browser tools with the jar loaded separately.

## Global paths on this VPS

| Location | Purpose |
| --- | --- |
| `~/.bxc/cookies/grok.json` | bxc shortcut **`grok`** (mode `600`) |
| `~/.aphrody/cookies/grok.json` | aphrody mirror (same content, mode `600`) |
| `~/.bxc/cookies/google.json` | Google Search / SSO (existing) |

Regenerate jars from a fresh browser export; do not commit cookie files. Optional builder (local only, no secrets in git): `python3 ~/aphrody/scripts/build-grok-cookie-jar.py` after updating `RAW` in that script from DevTools.

## grok.com-specific cookies

| Name | Domain | Role |
| --- | --- | --- |
| `cf_clearance` | `.grok.com` | Cloudflare clearance (IP/session bound) |
| `__cf_bm` | `.grok.com` | Cloudflare bot management (short TTL) |
| `__stripe_*` | `.grok.com` | Stripe billing UI |
| `__Secure-*PSID*` | `.google.com` | Google SSO when signing in with Google |

Cloudflare may still return **403 / “Just a moment…”** from the VPS even with a valid export if the clearance was issued for another IP or has expired. Prefer **Grok CLI** (`~/.grok/auth.json`) or **xAI API** for automation; use bxc cookies only for browser-shaped recon on grok.com.