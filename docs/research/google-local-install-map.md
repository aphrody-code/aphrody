<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody contributors
-->

# Google local install map — `%LOCALAPPDATA%\Google\Google`

Authorized reverse-engineering / interop map of the **Google desktop app for
Windows** install on the operator's own machine. Account anonymised to
`<user>` throughout. No secret values are recorded — secret-bearing artefacts
are listed by path/size/structure only.

- **Target root**: `C:\Users\<user>\AppData\Local\Google\Google`
- **Machine map artefact** (gitignored): `var/data/google-local-map/tree.json`
  (676 files, 585 hashed, 71 secret-meta-only)
- **Reproduce**: `pwsh -NoProfile -File scripts/forensics/map-google-local.ps1`
  or `bash scripts/forensics/map-google-local.sh` (see §5)

## 1. What this install actually is

This is **not** the Antigravity IDE (that is a separate Electron app under
`%LOCALAPPDATA%\Programs\Antigravity`). The `Google\Google` tree is the
**Google desktop companion app** for Windows — package
`com.google.windows.app` (per `latest\manifest.xml`) — a Microsoft Edge
**WebView2-hosted** launcher that renders a Google Search / Lens / account UI
as local web content.

Family classification (matches the `magika-webview2-re` memory):

| Binary | Family | Evidence |
| --- | --- | --- |
| `latest\google.exe` (22.27 MB) | `web_view2` | strings `msedgewebview2`, `WebView2Loader`, `CoreWebView2`, UA `Chrome/135.0.7049.37`, signer `Google LLC` |
| `latest\WebView2Loader.dll` (161 KB) | WebView2 runtime loader | Evergreen WebView2 bootstrap |
| `updater.exe` (5.26 MB) | Omaha updater | strings `omaha`, signer `Google LLC` |
| `context_menu_handler.dll` | Shell extension | Explorer context-menu integration |
| `latest\crashpad_handler.exe` | Crashpad | crash reporting |
| `local_files.db` (852 MB) | SQLite FTS5 | local filesystem search index (see §3) |

`google.exe` is the WebView2 **host** (it hosts web UI via the system Edge
runtime) — distinct from a self-contained Chromium browser. The actual
Chromium engine lives in the user-installed Edge WebView2 Evergreen runtime,
not in this tree; only the per-app user-data profile is stored here.

## 2. Directory layout (notable artefacts, cited paths)

```
Google\Google\
├─ context_menu_handler.dll      Explorer shell ext (right-click → Google)
├─ updater.exe                   Google Omaha updater
├─ GoogleIdentity.msix           MSIX identity / packaging artefact
├─ resources.pri                 Windows package resource index (PRI)
├─ global_preferences.txtpb      protobuf prefs (SECRET-META-ONLY)
├─ local_files.db                852 MB SQLite FTS5 local-file index (§3)
├─ Crashpad\                     crash dumps + attachments
├─ profiles\default\
│  ├─ preferences.txtpb          protobuf prefs (SECRET-META-ONLY)
│  └─ user_history.pb            protobuf history (SECRET-META-ONLY)
└─ latest\
   ├─ google.exe                 WebView2 host (com.google.windows.app)
   ├─ crashpad_handler.exe
   ├─ WebView2Loader.dll
   ├─ manifest.xml               <PackageName>com.google.windows.app</PackageName>
   ├─ THIRD_PARTY_NOTICES.txt
   ├─ Assets\                    Square*/StoreLogo tiles, toast hero PNGs
   ├─ html\                      rendered WebView2 UI (see §2.1)
   └─ default\WebView2\EBWebView\  per-app WebView2 user-data (§2.2)
```

### 2.1 `latest\html\` — the WebView2 UI surface

The hosted UI is a set of self-contained HTML pages + bundled JS + localized
message catalogs (`*_messages.xmb`) + Material-style SVG icons + the
`GoogleSans-v12.ttf` / `Roboto.ttf` fonts. The page set names the app's
feature surface precisely:

- `main.html` / `main_js.js` (193 KB) — primary launcher surface
- `lens_overlay.html` / `lens_overlay_app_js.js` (156 KB) — **Google Lens** overlay
- `login_page.html` / `login_page_js.js` — account sign-in
- `onboarding_v2.html` / `onboarding_v2_js.js` (206 KB) — first-run onboarding
- `settings.html`, `feedback.html`, `whats_new.html`,
  `screen_share_notice.html`, `content_permission_page.html`,
  `error_page` / `offline_error_page` / `generic_error_page`
- `web_suggestion.svg` / `history_web_suggestion.svg` — search-suggestion chrome
- `variables.css` + `lens_styles.css` (+ `.map` sourcemaps) — theming tokens

These are public, non-secret UI assets and are safe to study for interop /
design reference.

### 2.2 `latest\default\WebView2\EBWebView\` — WebView2 user-data profile

Standard Chromium/Edge profile layout under the app-private `EBWebView`
user-data folder. Component sub-trees present (versioned):
`AutoLaunchProtocolsComponent`, `CertificateRevocation`, `OriginTrials`,
`PKIMetadata`, `Subresource Filter`, `TrustTokenKeyCommitments`,
`Trust Protection Lists`, `hyphen-data`, `MEIPreload`, `Speech Recognition`
(ships `Microsoft.CognitiveServices.Speech.core.dll`), `WidevineCdm` (DRM),
`GraphiteDawnCache` / `GrShaderCache` / `ShaderCache` (WebGPU/GL caches),
`component_crx_cache` / `extensions_crx_cache`.

The `Default\` profile holds the Chromium stores enumerated in §4 as
secret-meta-only (cookies, login data, web data, network state, leveldb
key-value stores). **Their contents are never read by the mapping tooling.**

## 3. `local_files.db` — local filesystem search index (key finding)

`local_files.db` is an **852 MB SQLite database** (header `SQLite format 3`)
that the Google app maintains as a **full-text index of the user's local
files**. Schema (read-only, schema-only dump — no row values were selected):

```sql
CREATE TABLE local_files (
  file_id            INTEGER PRIMARY KEY AUTOINCREMENT,
  file_path          TEXT UNIQUE COLLATE NOCASE,
  parent_path        TEXT,
  filename           TEXT,
  extension          TEXT,
  is_directory       INTEGER,
  is_users_directory INTEGER,
  last_modified      INTEGER
);
CREATE INDEX idx_parent_path ON local_files(parent_path);
-- FTS5 virtual table + shadow tables:
--   local_files_fts, local_files_fts_config, local_files_fts_data,
--   local_files_fts_docsize, local_files_fts_idx
-- Sync triggers: trg_local_files_ai / _au / _ad (after insert/update/delete)
```

Aggregate (count only, no values): **1,590,051 rows**. This is a complete,
incrementally-maintained, FTS5-backed index of filenames + paths across the
user's home directory. It is the single most interesting artefact for aphrody:
it is a reference design for the local-file search capability proposed in
[`aphrody-search-upgrades.md`](aphrody-search-upgrades.md).

The index stores **path metadata only** (path, filename, extension,
mtime, directory flag) — not file contents — which keeps it cheap and
privacy-bounded. aphrody can replicate this exact shape natively.

## 4. Hardening notes (authorized, own machine — defensive framing)

Insecure-by-default observations from mapping this tree. Framed defensively;
no exploitation steps.

1. **Plaintext-at-rest secret stores (DPAPI-wrapped, not file-encrypted).**
   The WebView2 `Default\Network\Cookies`, `Default\Login Data`,
   `Default\Login Data For Account`, `Default\Web Data`, and
   `Default\Vpn Tokens` SQLite DBs follow standard Chromium layout: row values
   are encrypted with the per-profile key in `EBWebView\Local State`, which is
   itself wrapped with **DPAPI (user scope)**. Anything running **as the same
   Windows user** can transparently unwrap them (the classic Chromium cookie /
   credential extraction class). Newer Chromium adds **App-Bound Encryption
   (ABE)** keyed to the app identity; verifying whether this WebView2 profile
   uses ABE for the `Local State` `app_bound_encrypted_key` is the right
   hardening check. *Mitigation:* OS-level full-disk encryption + treating any
   same-user process as inside the trust boundary; do not rely on these stores
   for confidentiality against local malware.

2. **852 MB local-file index readable by the user.** `local_files.db` is a
   complete map of the user's filesystem (1.59M paths) readable by any
   same-user process. It is metadata only (no contents) but still a rich
   reconnaissance surface (project names, paths, file types). *Mitigation:*
   acceptable for a single-user box; on shared machines confirm ACLs restrict
   it to the owning user.

3. **`global_preferences.txtpb` / `profiles\default\preferences.txtpb` /
   `user_history.pb`** are protobuf stores held metadata-only here; treat as
   potentially identity-linked and keep out of any committed artefact.

4. **Shell extension DLL (`context_menu_handler.dll`).** A signed
   (`Google LLC`) in-process Explorer extension loaded into `explorer.exe`.
   Standard, but worth noting as an auto-loaded native module; verify the
   Authenticode signature chain is current.

5. **Stale cert / trust state.** `CertificateRevocation` (`6498.2025.9.4`),
   `PKIMetadata` (`46.0.0.0`), `TrustTokenKeyCommitments` (`2026.3.23.1`),
   `Trust Protection Lists` components are version-pinned; if the app stops
   updating, these go stale. Confirm `updater.exe` (Omaha) is scheduled and
   healthy so revocation/trust data stays fresh.

All secret values remained on-disk and untouched. The mapping tooling never
opened the contents of any artefact on the denylist (§5).

## 5. Reproduction (non-interactive)

Both scripts produce `var/data/google-local-map/tree.json` deterministically
and never read secret-file contents (denylist enforced in code):

- Windows: `pwsh -NoProfile -File scripts/forensics/map-google-local.ps1`
- POSIX (for a copied tree on Linux): `bash scripts/forensics/map-google-local.sh`

The native Rust path is `aphrody forensics map --target <dir> --out <dir>`
(built with `--features forensics`), enhanced in this work to emit
`sha256` (small files), `modified` (RFC 3339), and `mtime_unix` alongside the
existing `path/size/ext`. The `aphrody re google <binary>` and
`aphrody re classify <file>` (magika, `--features magika`) commands classify
individual binaries (family detection + OAuth/endpoint extraction) and content
types respectively.
