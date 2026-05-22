<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody-x — command reference

47 subcommands. Output is JSON by default; pass the global `--plain` for stable
human text. All commands accept the global credential flag.

## Global flags

| Flag | Env | Meaning |
|------|-----|---------|
| `--cookie-string <s>` | `X_COOKIE_STRING` | `auth_token=<v>; ct0=<v>` (overrides file + env) |
| `--plain` | | Stable text output (no JSON, no color, no emoji) |

Configuration file (JSON5, best-effort): `<config>/aphrody/x/config.json5`
(global) and `./.aphrodyxrc.json5` (project). Keys: `timeoutMs`, `quoteDepth`,
`defaultCount`, `output` (`"json"`/`"plain"`). Env shortcuts:
`APHRODY_X_TIMEOUT_MS`, `APHRODY_X_QUOTE_DEPTH`.

Exit codes: `0` success, `1` runtime error, `2` usage error.

## Pagination flags (reading commands)

`thread`, `replies`, `search`, `user-tweets`, `home`, `likes`, `bookmarks`,
`mentions`, `following`, `followers`, `list-timeline` share:

| Flag | Default | Meaning |
|------|---------|---------|
| `-n, --count <N>` | 20 | Items per page |
| `--all` | off | Fetch all pages (bounded by `--max-pages`) |
| `--max-pages <N>` | 5 | Page cap when `--all` |
| `--cursor <s>` | | Start from an explicit cursor |
| `--quote-depth <N>` | 1 | Max quoted-tweet nesting in output (0 disables) |

## Writing

| Command | Args / flags | Notes |
|---------|--------------|-------|
| `post <text>` | `--media <path>…` `--alt <text>…` | Up to 4 images/GIFs or 1 video |
| `reply <id\|url> <text>` | `--media` `--alt` | Reply to a tweet |
| `delete <id>` | | Delete your tweet |
| `note <body>` | `--preview <text>` | Long-form note tweet (Premium) |
| `upload-media <path>` | `--alt <text>` | Returns `media_id` for scripted posting |

`post`/`reply` auto-fall back to the legacy `statuses/update.json` endpoint if
GraphQL returns error `226` ("automated request").

## Engagement & relations

| Command | Arg |
|---------|-----|
| `like` / `unlike` | `<tweet-id>` |
| `retweet` / `unretweet` | `<tweet-id>` |
| `bookmark` / `unbookmark` | `<tweet-id>` |
| `pin` / `unpin` | `<tweet-id>` |
| `follow` / `unfollow` | `<user-id>` |
| `block` / `unblock` | `<user-id>` |
| `mute` / `unmute` | `<user-id>` |
| `dm <user-id> <text>` | | direct message |

## Reading

| Command | Args / flags |
|---------|--------------|
| `read <id\|url>` | single tweet (full Note/Article text) |
| `thread <id\|url>` | full conversation (+ pagination) |
| `replies <id\|url>` | replies to a tweet (+ pagination) |
| `search <query>` | `--top` (Top tab vs Latest) (+ pagination) |
| `user-tweets <handle>` | profile timeline (+ pagination) |
| `home` | `--following` (chronological feed) (+ pagination) |
| `likes` | your likes (+ pagination) |
| `bookmarks` | your bookmarks (+ pagination) |
| `mentions` | `--user <handle>` (default: you) (+ pagination) |
| `following` / `followers` | `--user <handle>` (default: you) (+ pagination) |
| `list-timeline <id\|url>` | tweets from a list (+ pagination) |
| `lists` | `--member-of` `--user <handle>` `-n <N>` |
| `news` (alias `trending`) | `-n` `--ai-only` `--for-you` `--news-only` `--sports` `--entertainment` `--trending-only` |
| `user <handle>` | quick profile lookup |
| `whoami` | the account your cookies belong to |
| `check` | which credential sources are available |

## GraphQL / catalog / diagnostics

| Command | Args / flags | Notes |
|---------|--------------|-------|
| `graphql <Op>` | `--var k=v` `--vars-json <json>` `--wait` `--max-wait-secs <s>` | Generic invoker over all 158 catalog ops |
| `catalog` | `--mutations` `--queries` `--filter <substr>` | Browse the embedded catalog |
| `rate-limit` | `--handle <h>` | Warm-up lookup + print `x-rate-limit-*` |
| `query-ids` | `--refresh` | Inspect / force-refresh the live queryId cache |

## Local-first store

See [store.md](store.md) for details.

| Command | Args / flags |
|---------|--------------|
| `sync <kind>` | `authored \| likes \| bookmarks \| timeline \| mentions \| graph` (each `-n <limit>`) |
| `db stats` | store counts by kind |
| `db search <query>` | `-n <N>` — FTS5 full-text search |
| `db export` | `--format json\|jsonl\|md` |
| `db digest` | `-n <top>` — deterministic "what happened" |
| `graph mutuals` | accounts that follow you back |
| `graph non-mutual-following` | accounts you follow that don't follow back |
| `import archive <path>` | `--handle <h>` — ingest a Twitter data export |
| `jobs` | `--what <kind>` `--every-minutes <n>` — print a cross-OS scheduler snippet |

## Examples

```bash
# Generic invoker with rate-limit waiting
aphrody-x graphql HomeTimeline --var count=5 --wait --max-wait-secs 300

# Browse mutation queryIds
aphrody-x catalog --mutations --filter Tweet

# Build a local archive, then query it offline
aphrody-x sync authored -n 800
aphrody-x sync graph -n 2000
aphrody-x db search "from-the-archive term" -n 50
aphrody-x graph non-mutual-following
aphrody-x db export --format md > my-tweets.md

# Import an official Twitter export
aphrody-x import archive ~/Downloads/twitter-archive --handle aphrody_code

# Periodic sync (prints the scheduler config for your OS)
aphrody-x jobs --what timeline --every-minutes 30
```
