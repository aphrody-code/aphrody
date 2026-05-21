<!-- SPDX-License-Identifier: Apache-2.0 -->
# Adobe creative integration for aphrody

Recon + decision record for wiring Adobe's creative developer surface into
aphrody, triggered by three references:

- `developer.adobe.com/adobe-for-creativity/` — the **Adobe for creativity**
  *connector* (an end-user Claude integration, not a developer API).
- `github.com/alisaitteke/photoshop-mcp` — a community **Photoshop MCP server**.
- `developer.adobe.com/photoshop/` — Adobe's **Photoshop developer platform**.

## What each thing actually is

| Source | Kind | Transport | Relevant to aphrody? |
|---|---|---|---|
| Adobe for creativity | End-user connector inside Claude chat | — | Inspiration only; no dev API documented. |
| `photoshop-mcp` | MCP server (TypeScript / Node) | stdio; drives a **locally installed** Photoshop via ExtendScript (COM on Windows, AppleScript on macOS) | Capability target — but **out of policy** (JS/Node banned, §2; needs the app open; Win/macOS only). |
| Photoshop developer platform | (a) UXP plugins, (b) **cloud Photoshop API** (REST, part of Firefly Services) | HTTPS REST + async jobs | The cloud API is the in-policy path. |
| Firefly Services / Firefly API | Cloud image generation + edit | HTTPS REST + async jobs, **IMS OAuth S2S** | Direct fit — second image backend next to Nano Banana. |

## Decision

**Build a native Rust client (`aphrody-firefly`) on the cloud APIs; do not port
`photoshop-mcp`'s local-automation model.**

Rationale:

- **Policy (§2)**: aphrody is 100% Rust, JS/Node banned. A Node MCP server is a
  non-starter; ExtendScript-over-COM ties us to a running desktop Photoshop.
- **Cross-platform (§0)**: the cloud Photoshop/Firefly REST APIs are headless
  and OS-independent (Linux #1). Local COM automation is Windows/macOS only.
- **Shared auth core**: the Firefly image API *and* the cloud Photoshop /
  Lightroom APIs all authenticate with the **same IMS server-to-server token**.
  One auth module (`aphrody_firefly::auth`) backs the whole family.
- **Latency (project objective)**: token fetched once, cached until ~60 s before
  expiry, reused; HTTP client reused; outputs downloaded concurrently
  (`JoinSet`).

## Verified protocol (2026-05, from Adobe docs)

**IMS token** — `POST https://ims-na1.adobelogin.com/ims/token/v3`,
`Content-Type: application/x-www-form-urlencoded`:

```
grant_type=client_credentials
client_id=<id>&client_secret=<secret>
scope=openid,AdobeID,session,additional_info,read_organizations,firefly_api,ff_apis
```

Returns `{ access_token, token_type, expires_in }`. Note the Adobe quirk:
`expires_in` is reported in **milliseconds** (~`86_399_999` for a 24 h token) —
`auth::interpret_expires_in` compensates.

**Firefly v3 async generate** — `POST https://firefly-api.adobe.io/v3/images/generate-async`,
headers `x-api-key: <client_id>` + `Authorization: Bearer <token>`, JSON body
`{ prompt, numVariations, size{width,height}, contentClass, negativePrompt,
visualIntensity, promptBiasingLocaleCode, seeds }`. Submission returns
`{ jobId, statusUrl, cancelUrl }`. Poll `statusUrl` until
`status ∈ {succeeded, failed, cancelled}`; on success `result.outputs[].image.url`
holds pre-signed download links.

## What landed

- **`crates/aphrody-firefly`** — pure-Rust client. `auth` (IMS S2S token, cached,
  secret-redacted `Debug`), `models` (typed request/response, camelCase wire,
  `JobStatus` with `Unknown` fallback), `client` (`FireflyClient`: submit → poll
  → concurrent download → atomic save). `#![forbid(unsafe_code)]`,
  clippy::pedantic, 23 offline tests (token-expiry math, serialization, status
  parsing, save). Live calls need real Developer Console credentials.
- **CLI** — `aphrody firefly generate "<prompt>" --out <dir> [--variations N
  --size WxH --content-class photo|art --negative … --locale … --json]`
  (feature `firefly`, host-only). Credentials from `FIREFLY_CLIENT_ID` /
  `FIREFLY_CLIENT_SECRET` (never CLI args — keeps secrets out of shell history).
- **aphrody-mcp** — tool `firefly_generate` (`crates/google_mcp/src/firefly_tools.rs`):
  cached client, optional `save_dir`, returns `{ count, outputs:[{ seed,
  content_type, bytes, saved_path? }] }`.

## Event-driven completion — Adobe I/O Events journaling

Polling each job's `statusUrl` adds latency and request volume. Adobe I/O
**Events journaling** is the pull-based, at-least-once event log: one stream
delivers every event the registration subscribes to (e.g. async-job
completion).

Verified protocol (Adobe docs, 2026-05): `GET <journal_url>[?latest=true |
since=<pos>&limit=<n>]` with headers `Authorization: Bearer <ims_token>`,
`x-api-key: <client_id>`, `x-ims-org-id: <org>@AdobeOrg`. `200` →
`{ events:[{ position, event }], _page:{ last, count } }` plus an HTTP
`Link: <…?since=…>; rel="next"` header for paging. `204 No Content` → caught
up; `retry-after` (seconds) gives the back-off and `Link` rel="next" the
resume position.

Landed in `aphrody_firefly::events`: `JournalClient` (shared `TokenCache`),
`Position` (Oldest / Latest / Since / NextLink), `read()` (one batch, parses
the `Link` header, handles 204 + retry-after), `drain()` (follow `next` until
caught up, returns events + resume position). Pure-logic `Link`-header parser
and percent-encoder are unit-tested. CLI: `aphrody firefly events [--latest |
--since <pos>] [--max-batches N] [--json]`.

Config (a journal URL + org id + api key) is **local-only** — kept under
`var/` (gitignored), sourced into `FIREFLY_JOURNAL_URL` / `FIREFLY_IMS_ORG_ID`
/ `FIREFLY_CLIENT_ID` / `FIREFLY_CLIENT_SECRET`. Never committed or logged.

## Next extension (same crate, same auth core)

The **cloud Photoshop API** (`photoshopv2-api.json`, base
`https://image.adobe.io/...`) is the in-policy answer to `photoshop-mcp`'s tool
surface — document operations, smart-object replace, text-layer edits,
`renditionCreate`, action playback — all async REST jobs that reuse
`aphrody_firefly::auth`. Adding a `photoshop` module + `photoshop_*` MCP tools
gives aphrody headless PSD editing with no Photoshop install and no JS runtime.
Tracked as the follow-up; the async submit/poll plumbing is already shared.

## Security / privacy

- Credentials read from the environment only; the secret is never logged,
  never serialized, redacted from `Debug`.
- Generated bytes are downloaded to memory and saved only where the caller
  asks (`--out` / `save_dir`); the MCP tool returns sizes + optional paths, not
  raw bytes by default.
- Prompts and outputs go to Adobe (the user's own Firefly entitlement) — the
  same trust boundary as any cloud generation call.
