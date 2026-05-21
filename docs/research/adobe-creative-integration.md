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

## Cloud Photoshop API — landed (`aphrody_firefly::photoshop`)

The in-policy answer to `photoshop-mcp`'s tool surface: headless PSD editing
over REST, no Photoshop install, no JS, on the **same IMS token**
(`TokenCache`).

Verified protocol (Adobe Photoshop API SDK, 2026-05): base
`https://image.adobe.io/pie/psdService`; ops `documentManifest`,
`documentOperations`, `smartObject`, `renditionCreate`. Auth headers
`Authorization: Bearer` + `x-api-key`. Inputs/outputs are `{ href, storage }`
(+`type`,`overwrite` for outputs); `storage ∈ {aio, adobe, external, azure,
dropbox}`; output `type ∈ {image/jpeg, image/png, image/vnd.adobe.photoshop,
image/tiff, image/x-adobe-dng}`. POST returns `{ _links:{ self:{ href }}}`;
poll until every `outputs[].status` is terminal (`succeeded`/`failed`;
transient `pending`/`running`/`uploading`).

`PhotoshopClient`: `document_manifest`, `create_rendition`,
`document_operations` (typed inputs/outputs + passthrough `options` layer
tree), `smart_object`. Typed `Storage`/`OutputType`/`PsJobStatus` (with
`Unknown` fallback), `PhotoshopJob::{all_terminal, all_succeeded}`. 42 offline
tests across the crate.

### MCP endpoint (`google_mcp/src/photoshop_tools.rs`)

Tools on `aphrody-mcp`: `photoshop_manifest`, `photoshop_rendition`,
`photoshop_document_operations`, and the **`firefly_to_photoshop`** bridge —
generate with Firefly (whose outputs are presigned, Adobe-readable URLs), then
feed that URL straight into a Photoshop op (e.g. convert a generated image into
an editable PSD, or return its layer manifest). `photoshop_manifest` and the
manifest branch of the bridge need only a readable input URL — no writable
storage — so they run end-to-end today; rendition/edit ops additionally need a
writable `output_url` (the user's presigned PUT / SAS / CC destination).

## The "Gemini plugin" and the JS boundary

A literal in-app Photoshop *plugin panel* must be **UXP** (HTML/JS) or a legacy
C++ `.8bf` — and JS/TS is banned from the aphrody repo (CLAUDE.md §2). So the
Gemini↔Photoshop capability is delivered **in-policy** as: (1) the Rust bridge
+ `firefly_to_photoshop` MCP tool above, and (2) the `gemini_*` MCP tools. A
UXP panel that simply calls `aphrody-mcp` is the out-of-policy, **user-owned**
artifact — its manifest + a thin `fetch()` bridge are documented in
`docs/integrations/photoshop-uxp-panel.md`; it is intentionally *not* committed
as source to keep the repo Rust-only. Override only on explicit instruction.

## Maximal Photoshop automation — Lightroom + Sensei ops (landed)

To match the official connector's editing surface (not just generate + manifest),
`aphrody_firefly::photoshop` now drives the full headless edit family on the
same IMS token, all submit→poll→terminal jobs:

| Capability | REST op (verified 2026-05) | SDK method | MCP tool |
|---|---|---|---|
| Auto-tone (AI exposure/contrast/highlights/shadows/vibrance) | `POST image.adobe.io/lrService/autoTone` | `lr_auto_tone` | `photoshop_auto_tone` |
| Auto-straighten (Upright) | `POST .../lrService/autoStraighten` | `lr_auto_straighten` | `photoshop_auto_straighten` |
| Camera-Raw edit (exposure…sharpness) | `POST .../lrService/edit` | `lr_edit` (`LrEdit`) | `photoshop_edit` |
| Apply `.xmp` preset | `POST .../lrService/presets` | `lr_apply_preset` | — |
| Remove background | `POST .../sensei/cutout` | `remove_background` | `photoshop_remove_background` |
| Create subject/bg mask | `POST .../sensei/mask` | `create_mask` | `photoshop_create_mask` |
| Product crop | `POST .../psdService/productCrop` | `product_crop` | `photoshop_product_crop` |
| Depth blur (Neural Filter) | `POST .../psdService/depthBlur` | `depth_blur` | `photoshop_depth_blur` |
| Play actionJSON | `.../psdService/documentOperations` `options.actionJSON` | `action_json` | `photoshop_action_json` |

Wire-shape notes (verified): **Lightroom** takes `inputs` as a single object
(not an array) + `outputs[]` + optional `options`; status polls at
`lrService/status/<id>`. **Sensei** (`cutout`/`mask`) takes singular
`input`/`output` objects. **psdService** ops use `inputs[]`/`outputs[]`.
`LrEdit` serializes to the canonical Camera-Raw Process-2012 XMP keys
(`Exposure2012`, `Contrast2012`, `Highlights2012`, …). `PhotoshopJob` now also
tolerates a top-level `status` (Lightroom/Sensei single-status shape) in
addition to the per-output array. `productCrop`/`depthBlur` are Adobe
"coming-soon" surfaces — wired and ready, return the API's status verbatim.

## RE: the official "Adobe for creativity" connector (Claude Desktop)

Reverse-engineered from the locally-installed connector plugin (read-only):
`…\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude\local-agent-mode-sessions\…\rpm\plugin_017FSfZwAM3GF7xTpsbDUHoA\`.

- **`.mcp.json`**: `{ "Adobe for creativity": { "type": "http", "url":
  "https://adobe-creativity.adobe.io/mcp" } }` — a **remote, Adobe-hosted MCP**.
  No local binary, no `env`/secrets on disk. Auth = interactive Adobe IMS OAuth
  handled by the connector; skills retry on 401 by re-authenticating.
- **`plugin.json`**: `adobe-for-creativity` v1.0.2, author Adobe, Apache-2.0,
  repo `github.com/adobe/skills`.
- **6 skills** (`SKILL.md`) orchestrate the remote tools: `adobe-batch-edit-photos`,
  `adobe-create-social-variations`, `adobe-design-from-template` (Express),
  `adobe-edit-quick-cut`, `adobe-resize-photos-and-videos`, `adobe-retouch-portraits`.
- **Hosted tool surface** referenced by the skills: `image_apply_auto_tone`
  (`type: cameraRawFilter`), `image_auto_straighten` (`uprightMode`,
  `constrainCrop`), `image_adjust_exposure|highlights|dark_portions|light_portions|`
  `brightness_and_contrast|vibrance_and_saturation|color_temperature` (a/b/luminance),
  `image_apply_preset` (`presetName`), `image_select_subject` (`bodyParts`),
  `image_apply_gaussian_blur` / `image_apply_lens_blur` (`blurRadius`,
  `blurTarget`), `image_crop_and_resize` (`output`, `fit: reframe|pad|extract`,
  `focus: subject|face|upper_body|{prompt}|{x,y}`), `video_resize`
  (`mode: letterbox|crop|stretch`), `video_create_quick_cut`, plus asset/board
  helpers (`asset_add_file`, `asset_preview_file`, `asset_*_file_upload`,
  `create_firefly_board`) and an `adobe_mandatory_init` handshake.

**Mapping to aphrody (verified equivalences):** the connector's hosted
`image_apply_auto_tone` ⇔ our `lrService/autoTone`; `image_auto_straighten` ⇔
`lrService/autoStraighten`; the `image_adjust_*` family ⇔ `lrService/edit` with
the Camera-Raw keys above; `image_crop_and_resize` ⇔ `psdService/productCrop` +
rendition; background removal ⇔ `sensei/cutout`; `image_select_subject`/mask ⇔
`sensei/mask`. aphrody reaches the **same underlying Firefly Services REST APIs**
through its own IMS server-to-server token — independent of the hosted connector.

## Security / privacy

- Credentials read from the environment only; the secret is never logged,
  never serialized, redacted from `Debug`.
- Generated bytes are downloaded to memory and saved only where the caller
  asks (`--out` / `save_dir`); the MCP tool returns sizes + optional paths, not
  raw bytes by default.
- Prompts and outputs go to Adobe (the user's own Firefly entitlement) — the
  same trust boundary as any cloud generation call.
