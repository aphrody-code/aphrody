<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody-x-client — complete documentation

Headless X (Twitter) account-control framework. Drives the **full** account from
the command line or as a Rust library, using only the browser session cookies —
**no browser at runtime, no developer API key, no OAuth app**.

- Binary: `aphrody-x`
- Library: `aphrody_x_client`
- Workspace: self-rooted (`crates/aphrody-x-client/`, excluded from the root
  workspace). Build from inside the crate dir.

> Companion docs: [`../RECON.md`](../RECON.md) (reverse-engineering of X's API
> surface) and [`data/x-graphql-catalog.json`](../data/x-graphql-catalog.json)
> (the 158-operation catalog).

---

## Table of contents

1. [Architecture](#architecture)
2. [Credential bootstrap (no browser)](#credential-bootstrap-no-browser)
3. [Authentication model](#authentication-model)
4. [Install & build](#install--build)
5. [CLI reference](#cli-reference)
6. [The GraphQL operation catalog](#the-graphql-operation-catalog)
7. [Rate limiting — honest behavior](#rate-limiting--honest-behavior)
8. [Library (Rust) API](#library-rust-api)
9. [Error model](#error-model)
10. [Examples](#examples)
11. [Security & scope](#security--scope)

---

## Architecture

```
                aphrody-x (CLI)            your Rust code
                      |                          |
                      v                          v
        +-------------------------------------------------+
        |                XClient (client.rs)              |
        |  reqwest (rustls) + cookie jar + auth headers   |
        |  generic graphql() / graphql_waiting()          |
        +----------------------+--------------------------+
                 |             |                  |
        catalog.rs       features.rs          session.rs
     158 operations   148 feature flags   ~/.aphrody/x-session.json
     (queryId/type)   default + per-op    auth_token + ct0
                 |
                 v
   https://x.com/i/api/graphql/{queryId}/{Operation}   (GraphQL)
   https://x.com/i/api/1.1/...                          (REST v1.1)
```

| Module | Responsibility |
|---|---|
| `session.rs` | Load credentials (`~/.aphrody/x-session.json`, env, or cookie string). |
| `client.rs` | `XClient`, header construction, generic GraphQL invoker, rate-limit capture. |
| `catalog.rs` | Embeds + parses the 158-operation catalog; lookup by name. |
| `features.rs` | Default feature-flag object + per-operation merge. |
| `api.rs` | Typed convenience methods + `TweetResult`/`UserInfo`/`TimelineTweet`. |
| `main.rs` | clap CLI over the library (25 subcommands). |

X's frontend is a React + Redux + Webpack SPA; the backend is a GraphQL gateway
(`/i/api/graphql/{queryId}/{Operation}`) plus legacy REST v1.1 (`/i/api/1.1/…`).

---

## Credential bootstrap (no browser)

The client needs `auth_token` (httpOnly) and `ct0` (CSRF) from a logged-in
browser session, persisted to `~/.aphrody/x-session.json`:

```json
{ "auth_token": "…40 hex…", "ct0": "…160 chars…", "handle": "your_handle" }
```

On Windows with Chrome ≥ 127 the cookies are protected by **App-Bound Encryption
(ABE, v20 flag-3)**. The documented one-time extraction (admin) is:

1. **VSS snapshot** of the locked `…\User Data\<Profile>\Network\Cookies` DB.
2. Decrypt the ABE cookie key via Chrome's **`IElevator` COM** service
   (CLSID `{708860E0-…}`), called from a helper located **inside the Chrome
   install directory** (caller-path validation). Fix the stale `HKCR\TypeLib`
   entry if Chrome was updated (`TYPE_E_LIBNOTREGISTERED`).
3. AES-256-GCM-decrypt each `v20` cookie (strip the 32-byte domain prefix).

Full procedure is recorded in the project memory `chrome-abe-cookie-extraction`.
On Linux/macOS, or if you already have the values, just write the JSON file (or
use `X_AUTH_TOKEN` / `X_CT0`, or `--cookie-string`).

---

## Authentication model

Every request carries:

| Header | Value |
|---|---|
| `authorization` | `Bearer <WEB_BEARER>` — the static public client-web bearer |
| `cookie` | `auth_token=…; ct0=…` |
| `x-csrf-token` | the `ct0` value |
| `x-twitter-auth-type` | `OAuth2Session` |
| `x-twitter-active-user` | `yes` |
| `x-twitter-client-language` | `en` |
| `user-agent` | a current Windows Chrome UA (`CHROME_UA`) |

`x-client-transaction-id` is **not** required for the operations exercised here
(live `CreateTweet` returns `344`, never `353`). It is a tracked best-effort
follow-up; the framework works without it today.

---

## Install & build

```bash
cd crates/aphrody-x-client
cargo build --release          # target/<triple>/release/aphrody-x
cargo test                     # 43 tests (offline, no network)
```

Credential precedence (highest first): `--cookie-string` / `X_COOKIE_STRING`
→ `X_AUTH_TOKEN` + `X_CT0` → `~/.aphrody/x-session.json`.

---

## CLI reference

Global flag: `--cookie-string "auth_token=…; ct0=…"` (or env `X_COOKIE_STRING`).

### Write actions (tweets)
| Command | Description |
|---|---|
| `post <text>` | Post a tweet (≤ 280 chars unless Premium). |
| `reply <tweet_id> <text>` | Reply to a tweet. |
| `delete <id>` | Delete a tweet. |
| `note <body> [--preview <text>]` | Long-form note tweet (Premium). |

### Engagement
| Command | Description |
|---|---|
| `like <id>` / `unlike <id>` | Favorite / unfavorite. |
| `retweet <id>` / `unretweet <id>` | Repost / remove repost. |
| `bookmark <id>` / `unbookmark <id>` | Bookmark / remove. |
| `pin <id>` / `unpin <id>` | Pin / unpin to profile. |

### Relationships (by numeric user ID — resolve via `user <handle>`)
| Command | Description |
|---|---|
| `follow <user_id>` / `unfollow <user_id>` | REST v1.1 friendships. |
| `block <user_id>` / `unblock <user_id>` | REST v1.1 blocks. |
| `mute <user_id>` / `unmute <user_id>` | REST v1.1 mutes. |

### Read & messaging
| Command | Description |
|---|---|
| `user <handle>` | Resolve a handle → id, name, follower/following counts. |
| `timeline [--count N]` | Authenticated home timeline (default 20). |
| `dm <recipient_id> <text>` | Direct message (REST v1.1 `dm/new2.json`). |

### Framework
| Command | Description |
|---|---|
| `graphql <Op> [--var k=v]… [--vars-json <JSON>] [--wait]` | Invoke ANY of the 158 catalog operations; prints raw JSON. `--wait` queues on soft rate limits. |
| `catalog [--mutations\|--queries] [--filter <substr>]` | List operations (name, queryId, type). |
| `rate-limit` | Show the last captured `x-rate-limit-*` window. |

`--var` values are parsed as JSON when possible (`--var count=5`,
`--var dark_request=false`), otherwise treated as strings.

---

## The GraphQL operation catalog

`data/x-graphql-catalog.json` holds **158 operations** (94 queries, 64
mutations), each `{ queryId, operationType, featureSwitches[] }`, extracted live
from X's `main.js`. It is embedded at compile time (`include_str!`) and parsed
once. Because queryIds rotate with X deployments, a refresh only needs the JSON
updated — no code change.

Live queryIds (2026-05-22) for the common actions:

| Operation | queryId |
|---|---|
| CreateTweet | `H-t2v_HvFR07ZBP9aOeKoA` |
| CreateNoteTweet | `yeInFtqpUoABoBE_YWPYgA` |
| DeleteTweet | `nxpZCY2K-I6QoFHAHeojFQ` |
| FavoriteTweet / UnfavoriteTweet | `lI07N6Otwv1PhnEgXILM7A` / `ZYKSe-w7KEslx3JhSIk5LA` |
| CreateRetweet / DeleteRetweet | `mbRO74GrOvSfRcJnlMapnQ` / `ZyZigVsNiFO6v1dEks1eWg` |
| CreateBookmark / DeleteBookmark | `aoDbu3RHznuiSkQ9aNM67Q` / `Wlmlj2-xzyS1GN3a6cj-mQ` |
| PinTweet / UnpinTweet | `VIHsNu89pK-kW35JpHq7Xw` / `BhKei844ypCyLYCg0nwigw` |
| UserByScreenName | `IGgvgiOx4QZndDHuD3x9TQ` |
| HomeTimeline | `Ly0idwoXvMotg0ArhGnnow` |

Refresh procedure: fetch the authenticated `x.com/home`, pull
`abs.twimg.com/responsive-web/client-web/main.<hash>.js`, regex
`queryId:"…",operationName:"…",operationType:"…"`, rewrite the JSON.

---

## Rate limiting — honest behavior

X enforces **server-side, per-account** limits. They **cannot** be bypassed by
any client. The framework instead manages them:

- Every response's `x-rate-limit-limit` / `-remaining` / `-reset` is captured
  into `XClient::last_rate_limit()`.
- `graphql_waiting(op, vars, feats, max_wait)` transparently sleeps until the
  window `reset` when `remaining == 0` (bounded by `max_wait`; returns
  `XError::RateLimited { reset_epoch }` if the wait would exceed it). CLI: the
  `--wait` flag on `graphql`.
- Hard per-account caps (e.g. **error 344**, the daily tweet/message limit)
  surface cleanly via `XError::Api { code, message }`. There is no client-side
  workaround — only respect / queue / retry later.

"No rate limit" therefore means *graceful handling and maximum throughput*, not
removal of X's server-side ceilings.

---

## Library (Rust) API

```rust
use aphrody_x_client::{XSession, XClient, XError, catalog};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), XError> {
    let session = XSession::load_or_env()?;        // ~/.aphrody/x-session.json or env
    let x = XClient::new(session)?;

    // typed convenience methods
    let me = x.user_by_screen_name("aphrody_code").await?;
    let posted = x.create_tweet("hello from rust", None).await?;   // TweetResult { id, text }
    x.like(&posted.id).await?;
    let tl = x.home_timeline(10).await?;            // Vec<TimelineTweet>

    // generic invoker over ANY of the 158 operations
    let raw = x.graphql("UserByScreenName",
                        json!({ "screen_name": "aphrody_code" }),
                        None).await?;               // serde_json::Value

    // catalog introspection
    let op = catalog::operation("CreateTweet").unwrap();
    println!("{} {:?} {}", op.name, op.op_type, op.query_id);
    println!("{} mutations", catalog::mutations().len());

    if let Some(rl) = x.last_rate_limit() {
        println!("remaining {}/{}, reset {}", rl.remaining, rl.limit, rl.reset_epoch);
    }
    Ok(())
}
```

### Surface

**`XSession`** — `new(auth_token, ct0)`, `load()`, `from_env()`,
`load_or_env()`, `from_cookie_string(&str)`, `cookie_header()`.

**`XClient`** — `new(session)`, `inner()`, `session()`, `last_rate_limit()`,
`graphql(op, vars, extra_features)`, `graphql_waiting(op, vars, feats,
max_wait)`, plus typed methods: `create_tweet`, `delete_tweet`, `like`/`unlike`,
`retweet`/`unretweet`, `bookmark`/`unbookmark`, `pin_tweet`/`unpin_tweet`,
`note_tweet`, `follow`/`unfollow`, `block`/`unblock`, `mute`/`unmute`,
`user_by_screen_name`, `home_timeline`, `send_dm`.

**`catalog`** — `operation(name) -> Option<&'static Operation>`, `all()`,
`mutations()`, `queries()`. `Operation { name, query_id, op_type,
feature_switches }`, `OpType::{Query, Mutation, Subscription}`.

**`features`** — `default_features() -> Value`, `features_for(&Operation) -> Value`.

**Result types** — `TweetResult { id, text }`, `UserInfo { id, name,
screen_name, followers_count, friends_count }`, `TimelineTweet { id, text }`.

---

## Error model

```rust
pub enum XError {
    Http(reqwest::Error),                       // transport
    Api { code: i64, message: String },         // X returned an errors[] array
    Auth(String),                               // missing/invalid credentials
    Json(serde_json::Error),
    Io(std::io::Error),
    UnknownOperation(String),                   // op not in catalog
    RateLimited { reset_epoch: i64 },           // graphql_waiting exceeded max_wait
}
```

Notable X API codes surfaced via `Api`: `32` (bad auth — usually wrong/missing
`ct0`), `344` (daily tweet/message cap), `353` (missing transaction id).

---

## Examples

```bash
# resolve a handle, then follow by id
aphrody-x user nasa
aphrody-x follow 11348282

# post, reply, like, delete
aphrody-x post "shipping aphrody-x"
aphrody-x reply 1880000000000000000 "nice"
aphrody-x like 1880000000000000000

# generic invoker — any catalog op
aphrody-x graphql HomeTimeline --var count=5
aphrody-x graphql CreateRetweet --vars-json '{"tweet_id":"1880…","dark_request":false}'

# browse the catalog
aphrody-x catalog --mutations --filter Bookmark

# with an explicit cookie string (no session file)
aphrody-x --cookie-string "auth_token=…; ct0=…" user aphrody_code
```

> On MSYS/Git-Bash, operation names and flags are fine, but prefer PowerShell if
> an argument begins with `/` (MSYS path-mangles it).

---

## Security & scope

- Intended for controlling **your own** account. Credentials live only in
  `~/.aphrody/x-session.json` (outside the repo) or env vars; none are committed.
- The bearer token is the public, static client-web bearer — not a secret.
- Respect X's Terms of Service and applicable rate limits; this tool does not and
  cannot circumvent server-side enforcement.
