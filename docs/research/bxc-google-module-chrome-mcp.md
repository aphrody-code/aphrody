<!-- SPDX-License-Identifier: Apache-2.0 -->
# bxc Google module → aphrody Chrome MCP / CDP upgrade design

> Reference clone: `var/data/bxc-ref/` (gitignored, reference only — `gh repo clone aphrody-code/bxc`, depth 1).
> bxc is a Bun/TypeScript + Rust engine. Per `CLAUDE.md` aphrody is 100% Rust with **no JS/TS in the distribution**,
> so bxc is treated strictly as REFERENCE material; the deliverable is a Rust port design.
>
> NON-OVERLAP: the Gemini `batchexecute` *send-payload wire spec* is owned by a separate workstream
> (`docs/research/gemini-web-cdp-exploitation.md` + `var/data/gemini-web-recon/gemini-send-payload-spec.json`).
> This document owns bxc's Google module + the chrome-MCP impersonation/bootstrap upgrade.

---

## 1. bxc repo map + where the Google module lives

bxc (`@aphrody-code/bxc` v0.3.1) is a "Zero-Spawn" browser-automation engine: Bun runtime API on top of a
Rust FFI bridge (`rust-bridge/`, the `obscura-*` crates) that embeds a V8 runtime + a from-scratch DOM and a
CDP-compatible server. It is a **stealth scraping / site-audit engine**, not a Google-auth client.

### 1.1 The "native Google module": `var/data/bxc-ref/src/google/`

| File | Role |
| --- | --- |
| `google/index.ts` | Public barrel re-export (`bxc/google`). |
| `google/client.ts` | `GoogleClient` — high-level stealth navigation + search + mass-audit orchestrator. Picks a stealth "profile" (`stealth-wiz` / `stealth-spa` / `stealth-lit`) from the Atlas (`client.ts:53-58`, `:63-87`). |
| `google/atlas.ts` | **AUTO-GENERATED** Google Ecosystem Atlas: 366 hosts classified by CDN (all `GFE`) + framework (`wiz`/`angular`/`lit`/`unknown`), built from `~/.bxc/cache.sqlite` (5637 audits). Header `atlas.ts:17-19`. Pure data — no logic. |
| `google/detector.ts` | `detectGoogleSpecifics()` — single-regex framework + product + **anti-bot** fingerprinting (`detector.ts:45-46`, `:116-155`). |
| `google/search.ts` | `googleWebSearch` / `googleSearchRich` — SERP scraping. `udm=14` classic view, synthetic `CONSENT` cookie, Ghost(Lightpanda)→curl-impersonate fallback (`search.ts:91-151`). |
| `google/serp-parser.ts`, `verticals.ts` | SERP DOM → structured results. |
| `google/fetch.ts` | `googleWebFetch` — clean-and-extract (JSON-LD/OG/Twitter) via the CDP page (`fetch.ts:107-151`). |
| `google/dns.ts` | `isGoogleDomain()` + mandate domain gating. |
| `google/mandate-guard.ts` | `enforceMandate()` — refuses out-of-scope hosts. |
| `google/cache.ts`, `rate-limit.ts`, `mass-scanner.ts`, `strategy.ts`, `style.ts` | Caching, GFE-429 rate limiting, bulk crawl. |

### 1.2 Key finding — bxc has NO Google AUTH/Boq bootstrap

A full-tree grep for `SNlM0e | batchexecute | SAPISIDHASH | f.sid | __Secure-1PSID | cfb2h` across
`var/data/bxc-ref/src/` returns **zero matches**. bxc never mints an `at`/`bl`/`f.sid` token, never computes a
`SAPISIDHASH`, never scrapes the `WIZ_global_data`/`SNlM0e` bootstrap blob from a logged-in Google page. Its only
Google "auth" is the synthetic anonymous `CONSENT` cookie for SERP scraping (`search.ts:106-117`).

Consequence: bxc's contribution to aphrody is **not** an auth bootstrap — it is the **Chrome TLS-impersonation
transport** and the **anti-bot detection heuristics**. The Boq `batchexecute` bootstrap already exists, far more
completely, inside aphrody's own `crates/notebooklm/` (see §4.2). The upgrade grafts bxc's impersonation onto
that existing bootstrap.

### 1.3 The Rust bridge (`var/data/bxc-ref/rust-bridge/crates/`)

This is the part directly relevant to an aphrody Rust port:

- `obscura-net/` — HTTP clients + cookie jar. **`wreq_client.rs` is the production native impersonation engine.**
- `obscura-cdp/` — a from-scratch CDP **server** (11 domain modules) for Playwright/Puppeteer parity.
- `obscura-browser/`, `obscura-dom/`, `obscura-js/` — the headless engine (V8 + DOM + page lifecycle).
- `obscura-mcp/`, `bxc-engine/` — MCP stdio surface + engine binary.

---

## 2. The Chrome impersonation fingerprint details

bxc ships **two** impersonation paths. The TS one is legacy; the Rust one is current.

### 2.1 Current native path — `wreq` + `wreq_util` (Rust) — RECOMMENDED for the port

`obscura-net/src/wreq_client.rs:38-61`:

```rust
let emulation_opts = wreq_util::EmulationOption::builder()
    .emulation(wreq_util::Emulation::Chrome145)
    .emulation_os(wreq_util::EmulationOS::Linux)
    .build();
let builder = wreq::Client::builder()
    .emulation(emulation_opts)
    .cert_store(/* system CA */)
    .redirect(wreq::redirect::Policy::none());
```

- Crate versions (`obscura-net/Cargo.toml:19,26,29`): `wreq = "6.0.0-rc.28"`, `wreq-util = "3.0.0-rc.10"`.
  On Linux/Android, `wreq` is built with `features = ["prefix-symbols"]` to rename BoringSSL exports and avoid
  clashing with system OpenSSL (`Cargo.toml:21-29`) — **this is the load-bearing supply-chain note for the port.**
- `wreq` is a reqwest-API-compatible client whose TLS/H2 stack is **BoringSSL**, so the JA3/JA4 hash, the
  ClientHello extension order, the GREASE values, the ALPS/`application_settings` extension, and the **HTTP/2
  SETTINGS frame + pseudo-header order + header casing/order** are emitted by `wreq_util`'s `Chrome145` template
  rather than being hand-rolled. bxc therefore does NOT hardcode the JA3 string anywhere — the fingerprint is the
  one baked into `wreq_util::Emulation::Chrome145` (a Chrome-145-on-Linux profile). The stealth UA string
  (`wreq_client.rs:21-22`) is `Chrome/145.0.0.0` on `X11; Linux x86_64`, matching the emulation OS.
- The gating: this whole module is `#[cfg(feature = "stealth")]` and the `stealth` feature pulls in
  `wreq` + `wreq-util` (`Cargo.toml:7-8`).

### 2.2 Legacy TS path — libcurl-impersonate FFI (`chrome131`)

`var/data/bxc-ref/src/ffi/curl-impersonate.ts` — a `bun:ffi` `dlopen` wrapper over
`libcurl-impersonate` (lexiforest/curl-impersonate). Relevant facts for the impersonation surface:

- Default profile **`chrome131`** for one-shot helpers (`curl-impersonate.ts:43`, `:933`); the class default is
  bumped to `chrome146` (`:588`). Supported Chrome profiles listed `:32-44` / `:71-113`
  (chrome99 … chrome131 … chrome146, plus `_android` variants, Firefox, Safari, Edge).
- The actual TLS/JA3 + H2 fingerprint is set by **`curl_easy_impersonate(handle, "chrome131", default_headers=1)`**
  (`:628-641`). That single call configures the cipher list, curves, ALPN, the H2 SETTINGS, pseudo-header order
  and the default Chrome header set — bxc never enumerates the JA3 fields itself; it delegates to the C library.
- `default_headers=1` forges Chrome's `Accept` / `Accept-Language` / `sec-ch-ua` header **set and order**
  (`:633-634`). Custom headers are appended via `curl_slist_append` preserving insertion order (`:698-714`).
- Per-platform `.so/.dylib/.dll` resolution + `LIBCURL_IMPERSONATE_PATH` override (`:258-310`).
- A note worth porting: curl-impersonate sometimes **locks `CURLOPT_ACCEPT_ENCODING`** to preserve fingerprint
  integrity, so bxc keeps a JS-side gzip/deflate/br/zstd fallback (`:378-402`, `:489-505`). A Rust port using
  `wreq` does not need this — `wreq` decodes transparently while still emitting the Chrome `accept-encoding` line.

### 2.3 The "plain reqwest" fallback (NOT impersonating)

`obscura-net/src/client.rs` is the default (non-stealth) client. It is a plain `reqwest::Client` that **manually**
sets Chrome-145 client-hint headers (`client.ts:294-337`: `sec-ch-ua`, `sec-ch-ua-platform "Linux"`,
`sec-fetch-*`, `upgrade-insecure-requests`) and a Chrome-145 UA (`:186`). This forges the *header* fingerprint but
**not** the TLS/JA3 fingerprint (it uses whatever rustls/native-tls reqwest links). This is exactly the level
aphrody's `notebooklm` transport sits at today (§4.2) — and exactly what bxc's `wreq` path improves on.

---

## 3. The CDP-compat surface

bxc's CDP layer is `obscura-cdp/` — a **CDP server** (it *speaks* the protocol to Playwright/Puppeteer clients),
not a CDP client that drives a real Chrome. `lib.rs:1-10` exposes `start*` entrypoints; `dispatch.rs` holds the
`CdpContext` (pages, sessions, isolated worlds, fetch-intercept, execution-context id tracking — `dispatch.rs:12-40`).

Bridged domains — `obscura-cdp/src/domains/mod.rs:1-11` (11 modules):

| Domain | File |
| --- | --- |
| `Target` | `domains/target.rs` |
| `Browser` | `domains/browser.rs` |
| `Page` | `domains/page.rs` |
| `DOM` | `domains/dom.rs` |
| `Runtime` | `domains/runtime.rs` |
| `Network` | `domains/network.rs` |
| `Fetch` | `domains/fetch.rs` |
| `Input` | `domains/input.rs` |
| `Storage` | `domains/storage.rs` |
| `Accessibility` | `domains/accessibility.rs` |
| `LP` (Lightpanda extension) | `domains/lp.rs` |

Notable behaviours to mirror if aphrody ever needs CDP-server parity: it re-emits
`Runtime.executionContextCreated` for registered isolated worlds after every navigation, and rejects
`Runtime.evaluate` against unknown context ids to match real Chrome's "Cannot find context with specified id"
(`dispatch.rs:20-37`, `:77-84`). Cookies cross the CDP↔HTTP boundary via
`CookieJar::set_cookies_from_cdp()` / `get_all_cookies()` (`obscura-net/src/cookies.rs:156-189`).

For the aphrody chrome-MCP upgrade this server is **not** what we want to port: aphrody's chrome MCP is a
*client/forensics* surface (read a real Chrome's memory/cookies), and the deliverable is a headless HTTP
bootstrap, so the CDP **server** is out of scope. The reusable idea is the `CookieJar` (RFC-6265 parse,
domain/secure/path matching, CDP-cookie ingest, `httpOnly` JS-visibility split — `cookies.rs:29-228`).

---

## 4. aphrody's current chrome-MCP surface (what we're upgrading)

### 4.1 The MCP server: `crates/google_mcp/` (binary `aphrody-mcp`)

`crates/google_mcp/src/main.rs` is an `rmcp` stdio server. Chrome-relevant tools:

- **`auth_extract`** (`main.rs:511-559`) — Windows-only forensic credential extraction. Reads Chrome **Canary**
  (`Google/Chrome SxS/User Data`) DPAPI-wrapped cookies via `backend::chromium::ChromiumParser`, looks for
  `__Secure-1PSID`. No headless login, no impersonation — pure local-disk forensics.
- **`chrome_autopsy`** (`main.rs:568-664`) — Windows-only `OpenProcess` + `ReadProcessMemory` over a Chrome PID.
- **`advanced_recon`** (`main.rs:671-725`) — DNS + TCP port probing (`std::net`), no Chrome involvement.
- **`universal_web_fetch`** (`main.rs:471-482`) — fetches via the `r.jina.ai` reader proxy with `reqwest::get`
  (default reqwest UA — **no impersonation**).
- `dns_recon`, `native_hooks`, `start_dashboard`, `re_*`, `context7_*`, `microsoft_*`, `docs_auto_search`,
  `voice_*` — unrelated to Chrome/Google session.

Summary: **the chrome MCP today can read a real Chrome's on-disk/in-memory secrets, but has no capability to
*originate* a convincing Google session over the wire.** That is the gap.

### 4.2 The existing pure-HTTP Google bootstrap: `crates/notebooklm/`

This is aphrody's real Boq client and the thing bxc's transport would enhance:

- `notebooklm/src/transport.rs` — `HttpTransport` posts an encoded `f.req` body to `batchexecute`
  (`transport.rs:133-193`) and to the chat-stream endpoint (`:208-274`). `SessionTokens { at, bl, fsid, language }`
  (`:29-39`) carries the per-session opaque tokens. The client is a **plain `reqwest::Client`** with a *static*
  `Chrome/131.0.0.0` UA string (`transport.rs:23-24`, `:55-61`) and hand-set `origin`/`referer`/`x-same-domain`
  headers (`:93-130`). **No TLS/JA3 impersonation.**
- `notebooklm/src/auth.rs` — `Auth::{Cookies, OAuthAccessToken}`, a `CookieJar` requiring `SAPISID` +
  `__Secure-1PSID` (`auth.rs:69-78`), Cookie-Editor JSON import (`:113-122`), and a `sapisidhash()` helper
  (origin-bound SHA-256 over `<unix> <SAPISID> <origin>`, `auth.rs:151-162`).
- **Key gap, stated in the source**: `transport.rs:5-8` notes that when the cookie session expires, callers must
  run "`aphrody bxc` headless re-login flow, kept out-of-crate". And a grep confirms `notebooklm` has **no**
  page-bootstrap that scrapes `SNlM0e`/`cfb2h`/`WIZ_global_data` from a logged-in page — `SessionTokens` are
  supplied externally. **That missing headless bootstrap is precisely what this upgrade builds.**

---

## 5. Rust port / upgrade design

### 5.1 Objective

Give aphrody's chrome MCP a Rust capability to (a) **bootstrap a Google session headlessly** — harvest
`at` / `bl` / `f.sid` (and `SNlM0e` / `cfb2h`) with a convincing Chrome fingerprint, (b) **replay cookies** from
`~/.aphrody/google-cookies.json`, (c) **survive Google anti-bot** — then feed the resulting `SessionTokens`
straight into the existing `notebooklm::HttpTransport`.

### 5.2 Impersonation crate recommendation: `wreq` + `wreq-util` (port bxc's choice)

Adopt exactly bxc's native path. Rationale and trade-offs:

| Option | Verdict |
| --- | --- |
| **`wreq` 6.x + `wreq-util` 3.x** (`Emulation::Chrome131`/`Chrome142`) | **CHOSEN.** Reqwest-compatible API ⇒ near drop-in for `notebooklm`'s transport. Real BoringSSL JA3/JA4 + H2 SETTINGS + header-order forging via `wreq_util` templates, no hand-rolled JA3. Active (bxc tracks it). Apache/MIT — no GPL contamination. |
| libcurl-impersonate FFI (bxc legacy TS path) | Rejected for the distribution: a C `.so/.dylib/.dll` dependency violates the "no C in distribution except `cxx::bridge`" rule (§2 CLAUDE.md), needs per-platform vendored binaries, and the `CURLOPT_ACCEPT_ENCODING` lock forces a manual decode fallback. Keep only as a documented fallback. |
| Hand-rolling JA3 via a rustls `CryptoProvider` + custom `ClientHello` | Rejected: rustls cannot reproduce Chrome's exact extension order / GREASE / ALPS without a fork; high maintenance, fragile against Chrome bumps. The repo already standardises on rustls-ring for plain reqwest (`CLAUDE.md §7`), so adding `wreq`'s BoringSSL is the only place BoringSSL enters — gate it behind a feature. |

Cross-platform caveat to carry over verbatim: `wreq` must use `features = ["prefix-symbols"]` **only** on
Linux/Android, plain otherwise (`obscura-net/Cargo.toml:21-29`). On the wasm32 target (#3), `wreq`/BoringSSL do
not build — the bootstrap module must be `#[cfg(not(target_arch = "wasm32"))]` and wasm keeps the plain-reqwest
`notebooklm` path. This matches how `google_mcp` already gates Windows-only tools.

### 5.3 Concrete module list

Two new crates + one MCP tool. Keep auth-bootstrap separate from the impersonation transport so `notebooklm` can
depend on the transport without pulling the headless browser.

```
crates/
├── aphrody-impersonate/            # NEW — thin wreq wrapper (the §2.1 port)
│   ├── Cargo.toml                  #   [features] stealth = ["wreq","wreq-util"]; gate per §5.2 caveat
│   └── src/
│       ├── lib.rs                  #   re-export StealthClient, Emulation profile enum
│       ├── client.rs              #   port of wreq_client.rs: EmulationOption::Chrome131..Chrome142,
│       │                          #   EmulationOS::{Windows,Linux,Macos}, redirect-none + manual loop,
│       │                          #   in-flight counter, extra-headers RwLock
│       └── cookies.rs             #   port of obscura-net/cookies.rs CookieJar (RFC-6265 + CDP ingest +
│                                  #   httpOnly JS-visibility split)
│
└── google-session/                # NEW — headless Boq session bootstrap (the missing piece §4.2)
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── bootstrap.rs           #   GET https://notebooklm.google.com/ (or target Boq app) with the
        │                          #   StealthClient + replayed cookies; scrape WIZ_global_data /
        │                          #   "SNlM0e":"…" (→ at), "cfb2h":"…", and the `bl` build label +
        │                          #   f.sid from the bootstrap HTML/JS. Emit notebooklm::SessionTokens.
        ├── cookies_io.rs          #   load/save ~/.aphrody/google-cookies.json (Cookie-Editor JSON;
        │                          #   reuse notebooklm::auth::Auth::from_chromium_export). On Windows,
        │                          #   optionally seed from google_mcp's auth_extract (DPAPI) path.
        ├── antibot.rs             #   port detector.ts heuristics: detect recaptcha v2/v3/enterprise,
        │                          #   captcha-form, GFE-429 + x-google-gfe-request-trace; classify and
        │                          #   surface a typed AntiBotChallenge instead of silently failing.
        └── refresh.rs             #   re-run bootstrap on NotebookError::Auth (401/403) → set_tokens()
```

MCP wiring — add one tool to `crates/google_mcp/src/main.rs` alongside `auth_extract`:

```
google_session_bootstrap { profile?: "chrome131"|"chrome142", cookies_path?: String, target?: String }
   → { at_present: bool, bl: String, fsid_present: bool, antibot: <classification>, cookie_count: u32 }
```

It must return **booleans/labels only — never token or cookie VALUES** (those stay in
`~/.aphrody/google-cookies.json` / process memory, mirroring how this doc anonymises secrets).

### 5.4 How it plugs into the existing notebooklm-style transport

1. `notebooklm::HttpTransport::new` keeps its current plain-reqwest path for wasm and as a zero-dep fallback.
   Add a feature-gated constructor `HttpTransport::with_stealth(auth, tokens, profile)` that swaps the inner
   client for `aphrody_impersonate::StealthClient` (wreq). Because `wreq`'s request builder mirrors reqwest,
   `build_headers()` / `rpc_raw()` / `chat_stream()` (`transport.rs:93-274`) are reused verbatim — only the
   `client.post(url)…send()` call sites change type. The static `Chrome/131` UA (`transport.rs:23-24`) is dropped
   in favour of the emulation-derived UA so the UA, JA3 and H2 fingerprint are mutually consistent (today they
   are not: a Chrome-131 UA over a rustls TLS hello is itself an anti-bot tell).
2. `google-session::bootstrap` produces the `SessionTokens` that `HttpTransport` consumes, closing the
   "tokens supplied externally" gap (`transport.rs:5-8`). The chrome MCP's new `google_session_bootstrap` tool
   calls it; `notebooklm` calls `refresh.rs` on `NotebookError::Auth` to re-mint tokens transparently.
3. Cookie replay: `cookies_io.rs` loads `~/.aphrody/google-cookies.json` into the ported `CookieJar`; on Windows
   the existing `auth_extract` DPAPI path (`google_mcp/main.rs:511-559`) can pre-seed the jar from a logged-in
   Chrome Canary profile, so the bootstrap needs no interactive login on that platform.
4. Anti-bot survival: `antibot.rs` (port of `detector.ts:116-155`) lets the bootstrap *detect* a reCAPTCHA /
   GFE-429 wall and react (back off via a rate-limiter à la `google/rate-limit.ts`, rotate profile, surface a
   typed challenge) instead of returning opaque HTML. bxc's `udm=14` classic-view trick (`search.ts:98`) and the
   synthetic `CONSENT` cookie (`search.ts:106-117`) port over for the unauthenticated SERP path.

### 5.5 Trade-offs vs the existing reqwest transport

- **Pro**: real Chrome JA3/JA4 + H2 SETTINGS + header order (BoringSSL via `wreq`) instead of a rustls hello with
  a mismatched Chrome UA — materially harder for GFE/anti-bot to flag. Header-set forging that `client.rs:294-337`
  does by hand becomes automatic and version-correct.
- **Con / cost**: `wreq` links BoringSSL — a second TLS stack alongside the repo's rustls-ring standard, larger
  binary, and the Linux-only `prefix-symbols` build caveat (§5.2). Must be feature-gated (`stealth`) and
  `#[cfg(not(wasm32))]`; CI must keep the three mandated targets green (Linux #1, Windows #2, wasm #3 builds the
  plain path only). `wreq`/`wreq-util` are at release-candidate versions (`6.0.0-rc.28` / `3.0.0-rc.10`) — pin
  exactly and run them through `cargo deny`/`cargo vet` before adoption (§5 CLAUDE.md). Profiles drift with Chrome
  releases, so pin a profile (`Chrome131`/`Chrome142`) and bump deliberately.

---

## Provenance

All bxc claims cite `var/data/bxc-ref/...` (clone of `aphrody-code/bxc`, depth 1). aphrody claims cite
`crates/google_mcp/` and `crates/notebooklm/`. No cookie/token values appear in this document; the example
account is referred to only as `<user>`. This file is design + reference only — no `crates/` code was modified.
