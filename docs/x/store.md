<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody-x — local-first store (birdclaw-class archive)

A single cross-platform SQLite database at `~/.aphrody/x-store.sqlite` holds a
canonical, account-scoped, agent-queryable archive of your X activity — the
"claw-able for agents" core of birdclaw, in pure Rust (bundled SQLite, FTS5; no
Node runtime, no external service).

## Schema

| Table | Purpose |
|-------|---------|
| `tweets` | canonical tweet rows (text, counts, ids, raw `json`, `first_seen`) |
| `users` | canonical user rows |
| `edges` | account-scoped membership: `(account, kind, tweet_id)` |
| `follows` | follow graph: `(account, direction, user_id, …)` |
| `tweets_fts` | FTS5 virtual table over tweet text, kept in sync on every upsert |

`edges.kind` ∈ `authored | liked | bookmarked | timeline | mention`. Tweets are
deduplicated by id and updated in place, so re-syncing or merging an archive
with live data never duplicates rows; the FTS index reflects the latest text.

## Sync

`aphrody-x sync <kind> [-n <limit>]` paginates the live client into the store:

| Kind | Source op | Edge |
|------|-----------|------|
| `authored` | UserTweets (your id) | authored |
| `likes` | Likes | liked |
| `bookmarks` | Bookmarks | bookmarked |
| `timeline` | HomeTimeline | timeline |
| `mentions` | SearchTimeline `(@you)` | mention |
| `graph` | Following + Followers | (→ `follows`) |

```bash
aphrody-x sync authored -n 800
aphrody-x sync bookmarks -n 500
aphrody-x sync graph -n 5000
```

## Query

```bash
# Aggregate stats (counts + per-edge-kind breakdown)
aphrody-x db stats

# FTS5 full-text search (supports AND/OR/NEAR/quoted phrases)
aphrody-x db search "rust OR gemini" -n 20
aphrody-x db search '"exact phrase"' -n 10

# Deterministic "what happened": top authors + most-liked stored tweets
aphrody-x db digest -n 10

# Export the whole store
aphrody-x db export --format json     # array of full tweet objects
aphrody-x db export --format jsonl    # one JSON object per line (git-friendly)
aphrody-x db export --format md       # Markdown bullet list with links
```

## Follow graph

After `sync graph`:

```bash
aphrody-x graph mutuals                 # follow you back
aphrody-x graph non-mutual-following    # you follow, they don't follow back
```

These are SQL set operations over `follows`
(`following INTERSECT follower`, `following EXCEPT follower`), so they consume
no live API quota.

## Archive import

Ingest an official Twitter/X data export — the JS-wrapped `data/tweets.js`
(`window.YTD.tweets.part0 = [ … ]`) or a plain JSON array — into the store:

```bash
aphrody-x import archive ~/Downloads/twitter-2026-archive --handle aphrody_code
# or point directly at the file:
aphrody-x import archive ./tweets.js --handle aphrody_code
```

Each legacy tweet maps onto the canonical `Tweet` shape and is upserted with an
`authored` edge — identical to live sync, so archive + live merge and dedup
cleanly. `--handle` defaults to the authenticated user.

## Scheduled sync (cross-OS)

`jobs` prints a platform-appropriate scheduler snippet for a recurring sync —
**it never modifies your system**; install the snippet deliberately:

```bash
aphrody-x jobs --what timeline --every-minutes 30
```

- **Windows** → `schtasks /Create …`
- **macOS** → a launchd `.plist`
- **Linux** → a systemd `--user` `.service` + `.timer`

This surpasses birdclaw's launchd-only schedulers with cross-OS coverage.

## Not included

birdclaw's React web UI and its OpenAI-backed digest/research are out of scope
for this CLI crate. The deterministic `db digest` covers the local
"what happened" need; an LLM-backed digest would route through aphrody's
existing Gemini surface rather than an OpenAI dependency.
