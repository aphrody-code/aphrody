<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody-x — headless X / Twitter framework

> **Not xAI:** This directory is **Twitter/X** (x.com). For **xAI Grok** (API `api.x.ai`, Grok CLI), see [`../grok/README.md`](../grok/README.md). For **bxc** integration see [`bxc-integration.md`](bxc-integration.md).

`aphrody-x` (crate [`crates/aphrody-x-client`](../../crates/aphrody-x-client))
is a complete, headless X/Twitter control framework: it drives an entire
account from the command line using only the browser session cookies
(`auth_token` + `ct0`) — **no browser, no API key, no developer portal**. It
is a single cross-platform Rust binary (lib + `aphrody-x` bin), built for
agents and pipelines (JSON-first output) with an optional `--plain` human mode.

It reaches and surpasses two reference tools:

- **[`@steipete/bird`](https://www.npmjs.com/package/@steipete/bird)** — a fast
  X CLI over the private web GraphQL API.
- **[`steipete/birdclaw`](https://github.com/steipete/birdclaw)** — a
  local-first SQLite archive of tweets/DMs/likes "claw-able for agents".

## Documentation map

| Doc | Contents |
|-----|----------|
| [commands.md](commands.md) | Every command and flag, with examples. |
| [architecture.md](architecture.md) | Internals: auth, queryId auto-refresh, POST-hybrid recovery, error model, stealth headers. |
| [store.md](store.md) | The local-first SQLite store: sync, FTS5 search, follow graph, archive import, digest, scheduler. |
| [bxc-integration.md](bxc-integration.md) | `bxc`, `@aphrody-code/x`, MCP `bxc_x_client`. |
| [bxc-cookies.md](bxc-cookies.md) | bxc cookie jar JSON format, shortcuts, global paths. |
| [grok-com-scan.md](grok-com-scan.md) | bxc detect/recon on grok.com (Cloudflare notes). |
| [env-and-auth.md](env-and-auth.md) | Cookie vs developer API env vars. |

## Why it is "better"

| Dimension | aphrody-x | bird | birdclaw |
|-----------|-----------|------|----------|
| Runtime | single Rust binary | Node ≥20 | Node ≥25 |
| Platforms | Linux (priority #1), Windows, macOS | macOS-first | macOS-first |
| Windows cookie extraction | headless ABE v20 (IElevator/VSS) | browser session only | — |
| queryId refresh | **all 158 ops**, runtime cache + POST-hybrid 404 recovery | ~30 ops | via bird |
| AI features | Gemini (no OpenAI dependency) | — | OpenAI |
| Schedulers | cross-OS (schtasks / launchd / systemd) | — | launchd only |

## Quickstart

```bash
# Build (from the crate dir — it is a self-rooted workspace)
cd crates/aphrody-x-client && cargo build --release

# Who am I?
aphrody-x whoami

# Read / search / timeline (JSON by default, --plain for humans)
aphrody-x read https://x.com/user/status/1234567890123456789
aphrody-x search "gemini" -n 10 --plain
aphrody-x home --following -n 20

# Post (optionally with media + alt text)
aphrody-x post "hello from aphrody" --media pic.png --alt "a description"

# Refresh the live GraphQL queryId cache (survives X rotations, no recompile)
aphrody-x query-ids --refresh

# Local-first archive for agents
aphrody-x sync authored -n 500
aphrody-x db search "rust OR gemini" -n 20
aphrody-x db digest
```

## Credentials

Resolution order (highest first):

1. `--cookie-string "auth_token=<v>; ct0=<v>"` (or env `X_COOKIE_STRING`)
2. `~/.aphrody/x-session.json` (`{ "auth_token": "...", "ct0": "..." }`)
3. env `X_AUTH_TOKEN` + `X_CT0`

`aphrody-x check` reports which sources are available. On Windows, the session
file is bootstrapped headlessly from Chrome's App-Bound-Encrypted cookies (see
the `chrome-abe-cookie-extraction` reference); on Linux/macOS, supply the
cookies via flag, env, or file.

## Honesty on rate limits

X enforces **server-side, per-account** limits (e.g. error `344` "daily tweet
cap"). No client can bypass these. aphrody-x captures `x-rate-limit-*` headers,
offers an opt-in waiting invoker for soft window limits, and surfaces hard caps
cleanly as `XError::Api { code, message }`.

## Scope

This crate is the cross-platform **CLI + library**. birdclaw's React web UI and
an LLM-backed digest are intentionally out of scope here (a deterministic
`db digest` is provided instead); everything else from both reference tools is
covered. See [store.md](store.md) for the local-archive surface.
