<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody-x — architecture & internals

## Module map (`crates/aphrody-x-client/src`)

| Module | Responsibility |
|--------|----------------|
| `session.rs` | Credential resolution (`XSession`): file / env / cookie-string. |
| `client.rs` | `XClient` — reqwest client, auth headers, generic GraphQL invoker, rate-limit capture, 404 recovery. |
| `catalog.rs` | Embedded 158-op GraphQL catalog (`data/x-graphql-catalog.json`). |
| `runtime_query_ids.rs` | Live queryId discovery + on-disk cache. |
| `features.rs` | GraphQL feature-flag blobs. |
| `parse.rs` | Typed tweet/user extraction from timeline trees. |
| `api.rs` | Typed methods (create_tweet, search, timelines, lists, whoami, …). |
| `media.rs` | Chunked media upload. |
| `news.rs` | Explore-tab news/trending. |
| `output.rs` | JSON / plain rendering. |
| `config.rs` | JSON5 config layering. |
| `store.rs` | Local-first SQLite store (see [store.md](store.md)). |
| `archive.rs` | Twitter data-export import. |

## Authentication

X's private web API authenticates with three cooperating signals on every
request:

1. `Cookie: auth_token=<v>; ct0=<v>` — session cookies.
2. `X-Csrf-Token: <ct0>` — the `ct0` value re-sent as a header (double-submit
   CSRF); the two must match exactly.
3. `Authorization: Bearer <web_bearer>` — the static **public** web bearer
   embedded in X's JS bundle (identical for all browsers; not personal).

Plus browser-like markers that make traffic look human:
`x-twitter-auth-type: OAuth2Session`, `x-twitter-active-user: yes`,
`x-twitter-client-language: en`, a stable per-session `x-client-uuid` /
`x-twitter-client-deviceid` (UUIDv4), `origin`/`referer`, a Chrome UA, and a
fresh random 32-hex `x-client-transaction-id` per request.

> The real `x-client-transaction-id` is derived from an animation SVG + a page
> key. Empirically X accepts an opaque random value (the reference client does
> the same), so the full algorithm is **not** required.

## queryId auto-refresh (the resilience core)

X rotates each operation's `queryId` whenever it ships a new web bundle, so the
embedded catalog goes stale. `runtime_query_ids.rs` keeps the client working:

1. Fetch public discovery pages (`x.com/?lang=en`, `/explore`, …) with an
   **unauthenticated** browser-like client — the `<script>` bundle URLs only
   appear in the logged-out HTML shell, so the authenticated client's
   bearer/cookie/json headers would yield a different document.
2. Regex out every `abs.twimg.com/responsive-web/client-web/*.js` URL, fetch
   the bundles concurrently, and run four `{operationName, queryId}` patterns.
3. Cache the resolved ids to `<config>/aphrody/x/query-ids-cache.json` with a
   24h freshness TTL (a stale snapshot is still used; only a successful refresh
   overwrites it).

Resolution at call time: **runtime cache → embedded catalog**. A rotation needs
only `aphrody-x query-ids --refresh` (or happens automatically on a 404), never
a recompile. Override the cache path with `APHRODY_X_QUERY_IDS_CACHE`.

## Request dispatch & 404 recovery

`XClient::graphql(op, variables, extra_features)`:

- **Queries** → `GET /i/api/graphql/{queryId}/{op}?variables=…&features=…`.
- **Mutations** → `POST` with body `{variables, features, queryId}`.

On HTTP **404** (invalid queryId *or* an op X only serves over POST):

- **Queries**: retry with the **POST-hybrid** form (variables in the URL,
  `{features, queryId}` in the body) using the same queryId first — this is how
  X serves `SearchTimeline` and friends. Only if that *also* 404s do we pay for
  a live queryId refresh and retry.
- **Mutations**: refresh queryIds once and retry.

This two-tier recovery is why search works without a queryId refresh on every
call, and why the client survives rotations transparently.

## Error model (`XError`)

| Variant | Meaning |
|---------|---------|
| `Http(reqwest::Error)` | transport/TLS |
| `Api { code, message }` | structured X error (e.g. `32` auth, `226` automated, `344` daily cap, `353` missing txn-id) |
| `Auth(String)` | credential/format problem |
| `Json` / `Io` / `Db` | (de)serialization / file / SQLite |
| `UnknownOperation(String)` | op not in catalog |
| `RateLimited { reset_epoch }` | soft window limit, wait exceeds `max_wait` |

## Cross-platform notes

- Self-rooted workspace (own `Cargo.lock`); build from inside the crate dir.
- SQLite is bundled (no system dependency); FTS5 enabled.
- The CLI runs on a 32 MiB worker thread with a manual multi-thread tokio
  runtime: in debug builds, clap's non-inlined command-tree builders for the
  large (47-subcommand) surface overflow the default 1 MiB main stack
  (release is unaffected) — the worker thread makes debug and release identical.
