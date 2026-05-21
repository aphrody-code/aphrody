<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: 2026 aphrody contributors -->
<!--
  Full public-web recon of https://antigravity.google/ (marketing/product site,
  Google Antigravity). Public assets only: fetch / crawl / parse. No auth bypass,
  no infra attack, no rate hammering. All raw artifacts saved (gitignored) under
  var/data/antigravity-site/ ; only this report is committed.
  Recon date: 2026-05-21. Tooling: curl (--compressed), Jina reader
  (mcp__aphrody__universal_web_fetch), gh CLI.
-->

# Recon : `antigravity.google` (site public + code findings)

## Overview

`antigravity.google` is the public marketing / product / docs site for **Google
Antigravity** (the agentic IDE). It is a **single-page Angular app** (standalone
components, esbuild bundle, `<app-root>`) served from **Google App Engine** behind
**Google Frontend (GFE)**. The site is a thin shell: all 26 sitemap routes return
the identical SPA bootstrap HTML, and all page content / docs / pricing copy lives
inside the `main-ULJOXPIW.js` bundle and is hydrated client-side. No JSON-LD, no
backend feature flags, no secrets are exposed on the public surface.

The recon's highest-value payload is the **download / install architecture** for
the Antigravity IDE and the new **`agy` CLI**, extracted from the JS bundle and the
public `install.{sh,ps1,cmd}` scripts: a SHA512-verified, self-updating manifest
flow backed by Cloud Run updater services + public GCS buckets + `pkg.dev`
apt/yum repos, all under GCP project `974169037036`.

- Recon date: **2026-05-21**.
- Connect IP: **216.239.32.61** (Google LLC netblock 216.239.32.0/19).
- Server: `Google Frontend`; HTTP/1.1 + HTTP/3 (`Alt-Svc h3`).
- Raw artifacts: `var/data/antigravity-site/` (gitignored — see Manifest §7).

## 1. Pages map

`robots.txt` = `User-agent: * / Allow: /` + sitemap pointer. `sitemap.xml` lists
**26 URLs**; all return the same 4008-byte (gzip) / 22769-byte (decompressed) SPA
shell — there is no server-side prerendered per-route content.

| Section | Routes |
|---|---|
| Root | `/` |
| Product / use-cases | `/product`, `/use-cases`, `/use-cases/{frontend, frontend-developer, fullstack, full-stack-developer, professional, enterprise-developer}` |
| Docs | `/docs`, `/docs/{get-started, features, agent-features, editor-features, faq, rest-api}` |
| Download | `/download`, `/download/linux` |
| Commerce / community | `/pricing`, `/plugin`, `/support`, `/interest-form`, `/terms` |
| Content | `/blog`, `/blog/introducing-google-antigravity`, `/changelog` |

Docs content (verified via Jina reader against `/docs/get-started`): Antigravity
2.0 IDE; platform mins **macOS >= 12 (arm only, no x86), Windows 10 64-bit, Linux
glibc >= 2.28 / glibcxx >= 3.4.25**; agent **Projects** (folder/repo scoping),
**Local Mode** vs **New Worktree Mode**; slash commands `/goal`, `/grill-me`,
`/schedule`, `/browser` (the last drives a Chrome debugging session with user
permission). Note: `/docs/rest-api` currently renders the get-started content
(client-side route content not yet distinct in the served bundle).

## 2. Assets inventory

Counts (full tree with sizes in `var/data/antigravity-site/MANIFEST.txt`):

- **HTML**: 26 page shells (`site/html/`), all identical SPA bootstrap.
- **JS**: `main-ULJOXPIW.js` (1.77 MB decompressed), `chunk-E6TGZIGP.js` (esbuild
  interop helpers). **No sourcemaps** (`*.js.map` → 404).
- **CSS**: `styles-7KLEMMT6.css` (20.6 KB).
- **Install scripts**: `install.sh` (7.4 KB), `install.ps1` (7.2 KB),
  `install.cmd` (6.0 KB) under `/cli/`.
- **Images**: 34 (brand lockups/icons PNG, use-case SVGs, 7 og "sitecard" PNGs,
  3 large landing thumbnails ~2 MB JPG each, 11 THREE.js texture PNGs).
- **Video**: 8 referenced; 7 downloaded (459 KB – 9.7 MB), 1 over the 50 MB cap
  recorded URL-only: `assets/video/use-cases/pmm/hero-bg-pmm.mp4` (54,103,792 B).
- **Fonts**: Google Sans Flex, Google Sans Code, Google Symbols (served from
  `fonts.gstatic.com`, not re-hosted — URLs catalogued in `index.html`).
- **Meta**: `robots.txt`, `sitemap.xml`, `favicon.ico` exist. **404**:
  `manifest.json`, `site.webmanifest`, `ads.txt`, `/.well-known/security.txt`,
  `/.well-known/assetlinks.json`, `/.well-known/apple-app-site-association`
  (404 bodies saved for completeness; they are not real artifacts).

Notable files: the three `/cli/install.*` scripts and the live release JSONs
(`site/data/*.json`).

## 3. JS / code findings

### 3.1 Framework & libs
Angular standalone SPA (version string `20.3.11` in bundle), **three.js**
(`window.__THREE__`), **GSAP**, **marked** (markdown). Analytics = **GTM container
`GTM-M4N2ZKXQ`**, loaded only after the gstatic **Glue cookie-notification-bar**
consent (category `2A`). GA4 dataLayer events seen: `page_view`, `download`,
`download_cta`, `install_extension`, `interest_form_submit`. **No raw GA
measurement ID, no OAuth client_id, no API key** is present in the public bundle.
The only `window.__*` globals are `__THREE__` and `__debugMouse`; "feature flag"
greps returned only THREE.js/Angular internals (not product flags).

### 3.2 Download / install architecture (the headline finding)
GCP project number **`974169037036`**. Three Cloud Run updater services
(`*.us-central1.run.app`):

- **CLI updater** `antigravity-cli-auto-updater-974169037036.us-central1.run.app`
- **IDE updater** `antigravity-ide-auto-updater-974169037036.us-central1.run.app/releases`
- **Hub updater** `antigravity-auto-updater-974169037036.us-central1.run.app/releases`

**The `agy` CLI** (new — `pypi.org/project/google-antigravity`, repo
`google-antigravity/antigravity-cli`). Bootstrap flow from `install.sh` /
`install.ps1` (both saved):
1. Detect OS/arch (+ Linux musl detection via `/lib/libc.musl-*.so.1` / `ldd`).
2. `GET {cli-updater}/manifests/{platform}.json` → `{version,url,sha512}`.
3. Download payload, **verify SHA512 (hard-fail on mismatch)**, install binary.
4. Target: Unix `~/.local/bin/agy`; Windows `%LOCALAPPDATA%\agy\bin\agy.exe`.
5. Hand off to `agy install` for shell-PATH setup; **self-updates in background**.

Live manifests pulled during recon (`site/data/cli-manifest-*.json`): CLI
**v1.0.0**, payloads in GCS bucket
`storage.googleapis.com/antigravity-public/antigravity-cli/1.0.0-5288553236791296/{platform}/...`.

**IDE / Hub** downloads use two CDNs: Google's `edgedl.me.gvt1.com/edgedl/release2/j0qc3/antigravity/stable/`
and `storage.googleapis.com/antigravity-public/antigravity-hub/`. Marketing JS
hardcodes IDE **2.0.1**; the live `/releases` feed shows latest **2.0.2** (IDE) and
a long history back to 1.15.x (full lists in `site/data/{ide,hub}-releases.json`).
**Linux packages** via Artifact Registry: apt
`us-central1-apt.pkg.dev/projects/antigravity-auto-updater-dev/`, yum
`us-central1-yum.pkg.dev/.../antigravity-rpm`, GPG key at
`us-central1-apt.pkg.dev/doc/repo-signing-key.gpg`.

### 3.3 External references in bundle
GitHub org **`google-antigravity`** (id 242056456, created 2025-11-04, 2 public
repos): **`antigravity-sdk-python`** (651 stars, Python, Apache-2.0) and
**`antigravity-cli`** (331 stars). Also `google-deepmind/science-skills`,
`markedjs/marked`, `anthropic.com/legal/commercial-terms` (Anthropic models
surface in the product), `one.google.com/ai/{credits,activity}`,
`discuss.ai.google.dev/c/antigravity/64` (community), `x.com/antigravity`,
`linkedin.com/company/google-antigravity`. Models named in copy:
**gemini-3-1-pro, gemini-3-5-flash, gemini-3-flash** ("in-google-antigravity").

## 4. Embedded data

No JSON-LD on the page. Open Graph / Twitter (from `index.html`): site name
"Google Antigravity", description "Build the new way", card
`summary_large_image`, `twitter:site @antigravity`, og image
`/assets/image/sitecards/sitecard-default.png` (7 sitecards exist, one per
section). Full structured extract: `var/data/antigravity-site/site/data/embedded.json`.

## 5. Infra fingerprint (CDN / headers / DNS)

- **Hosting**: Google App Engine. The CSP `script-src`/`worker-src` allowlists the
  backend origins explicitly: `gweb-jetski.appspot.com`,
  `gweb-jetski.uc.r.appspot.com`, and the versioned prod deploy
  **`prod-20260520t145130-dot-gweb-jetski.appspot.com`** (App Engine version
  timestamp `20260520t145130`). CSP also allows `*.google.com`,
  `*.google-analytics.com`, `*.googletagmanager.com`, `*.gstatic.com`,
  `*.youtube.com`, `*.ytimg.com` + one inline-script hash.
- **Edge**: `server: Google Frontend`, `x-cloud-trace-context` present,
  `content-encoding: gzip`, `Alt-Svc h3` (HTTP/3). Static caching
  `Cache-Control: public, max-age=600`, ETag `"EITV_A"`.
- **Security headers**: `Strict-Transport-Security max-age=2592000;
  includeSubdomains`, `X-Content-Type-Options nosniff`, `X-Frame-Options DENY`,
  `X-XSS-Protection 1; mode=block`, `object-src none`. No public security.txt.
- **Cookies**: none set on the HTML responses (consent-gated GTM only).
- **DNS**: the recon network has a transparent ISP DNS interceptor
  (`ns1.numericable.net` / SDV France) that hijacks all `.google` lookups and
  returns `nc-ass-vip.sdv.fr` / `212.95.74.75` — **not authoritative**, even via
  `8.8.8.8`. Authoritative signal therefore taken at the HTTP layer: curl's TLS
  connection resolved to **216.239.32.61** (Google). Raw output: `site/meta/dns.txt`.

## 6. What this tells us vs the desktop client RE

Cross-reference: [`docs/research/antigravity-sdk-analysis.md`](antigravity-sdk-analysis.md)
§0.0 (forensic RE of the installed Antigravity 2.0.1 desktop client).

- **Two complementary surfaces, cleanly separated.** The desktop RE established
  the *runtime / auth / cloud wire* (Codeium/Windsurf `language_server.exe` Go
  engine, OAuth2 token in Windows Credential Manager `gemini:antigravity`,
  client_id `1071006060591-…apps.googleusercontent.com`, redirect
  `localhost:9109`, endpoints `cloudcode-pa.googleapis.com :loadCodeAssist /
  :fetchAvailableModels / :onboardUser`, cert pin). The site recon establishes the
  *distribution / install / update* surface. **They do not overlap** — the public
  site exposes zero auth material, confirming the RE finding that auth lives only
  in the client/OS keystore.
- **New CLI (`agy`) not covered by the SDK analysis.** The site reveals a separate
  `agy` CLI (PyPI `google-antigravity`, repo `antigravity-cli`, v1.0.0) with its
  own Cloud Run manifest+SHA512 self-update flow — distinct from the IDE/Hub
  channels and from the Python SDK (`crates/antigravity-sdk` ports the SDK's Cloud
  path #1, not this CLI). The CLI's `--standalone --subclient_type hub` LS spawn in
  §0.0 path #2 is the likely runtime the `agy` binary fronts.
- **Version reconciliation.** SDK analysis pinned the desktop client at 2.0.1; the
  site's live `/releases` feed now lists IDE **2.0.2** as latest (marketing JS still
  says 2.0.1). The Hub feed history (1.15.x → 2.0.1) corroborates the same release
  train the desktop updater pulls from.
- **Provisioning / supply-chain map for any future interop.** GCP project
  `974169037036`, GCS `antigravity-public/{antigravity-hub,antigravity-cli}`,
  `edgedl.me.gvt1.com/.../j0qc3/antigravity/stable/`, and apt/yum `pkg.dev` repos
  (project `antigravity-auto-updater-dev`) are now documented — useful for a Rust
  port to mirror install/verify behavior (SHA512 manifest) without the proprietary
  binary, consistent with the SDK analysis "Cloud direct (path #1)" recommendation.
- **No new endpoints to harvest.** The public site does not expose any
  `*.googleapis.com` API beyond what §0.0 already documented; it only adds the
  distribution Cloud Run + GCS + pkg.dev infrastructure above.

## 7. Artifact manifest (saved under `var/data/antigravity-site/`)

Gitignored (proven: `git check-ignore var/data/antigravity-site/` → match).
**91 files, ~25.7 MB total.** Tree summary (full per-file sizes in
`var/data/antigravity-site/MANIFEST.txt`):

```
var/data/antigravity-site/
  MANIFEST.txt
  site/
    html/        26 SPA page shells (index + 25 routes), each ~22.8 KB decompressed
    meta/        robots.txt, sitemap.xml, favicon.ico, headers.txt, dns.txt,
                 + 404 bodies (manifest.json, site.webmanifest, ads.txt,
                   security.txt, assetlinks.json, apple-app-site-association)
    assets/
      js/        main-ULJOXPIW.js (1.77 MB), chunk-E6TGZIGP.js
      css/       styles-7KLEMMT6.css
      install.sh / install.ps1 / install.cmd   (the /cli bootstrappers)
      media/
        image/   brand lockups + icons, use-case SVGs, 7 sitecards,
                 3 landing thumbnails (~2 MB JPG each)
        textures/icons/  11 THREE.js texture PNGs
        video/   7 mp4 (459 KB–9.7 MB); pmm hero (54 MB) URL-only, see video-sizes.txt
    data/        embedded.json, cli-manifest-{linux_amd64,windows_amd64}.json,
                 ide-releases.json, hub-releases.json, video-sizes.txt
```

Largest items: 3 landing JPGs (~2 MB each), `hero-bg-science.mp4` (9.7 MB),
`hero_video.mp4` (3.4 MB), `main-ULJOXPIW.js` (1.77 MB).
