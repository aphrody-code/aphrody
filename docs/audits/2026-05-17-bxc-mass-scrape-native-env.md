<!-- SPDX-License-Identifier: Apache-2.0 -->

# Audit — bxc mass-scrape native Windows env

**Date (UTC):** 2026-05-17
**Tick:** YOLO grind tick 36+ — user-directed pivot ("complex bxc en mode windows pour scrape massif")
**Initiator:** aphrody-side orchestrator (this repo)
**Counterpart:** local clone `C:\worktree\bxc` (aphrody-code/bxc@aphrody branch)
**Status:** shipped — `scripts/bxc-mass-scrape.{ts,ps1,urls.json}` + `var/data/bxc-cache/`

## 1. Why this exists

The pre-existing `scripts/scrape-m3-tokens.ts` only walks 6 M3 pages with a
single-page-at-a-time loop. To seed the new `crates/m3-tokens`,
`crates/shadcn-bridge`, `crates/a2a-ui`, and the `aphrody-translate`
corpus, we need a mass-scrape pass against ~130 URLs covering:

| Category         | Count | Purpose                                          |
|---|---|---|
| `m3`             |   10  | M3 design tokens (color/typography/shape/motion) |
| `m3-components`  |   29  | M3 component baselines (button/card/dialog/...)  |
| `shadcn`         |   45  | shadcn primitives reference                      |
| `material-web`   |   14  | MWC3 component reference (Lit-based)             |
| `agntcy`         |    6  | AGNTCY a2a/v0.4 + slim + identity + dir specs    |
| `wasm`           |    5  | wasm-bindgen / web-sys / js-sys API refs         |
| `wgpu` / `webgpu`|    4  | WebGPU spec + wgpu Rust crate                    |
| `lit`            |    2  | Lit element framework (MWC3 substrate)           |
| `mdn`            |    5  | Web Components, custom elements, shadow DOM, CSS |
| `turbopack`/`swc`/`next` | 6 | per `project_aphrody_ultimate_goals` memory |

Total: 126 URLs in the v1 list. Easy to extend.

## 2. Native Windows env requirements

Per `CLAUDE.md` (root) + `memory/feedback_bun_only` + `memory/feedback_latest_toolchain`:

| Component     | Required | Install                            |
|---|---|---|
| **bun**       | >= 1.3   | `winget install Oven-sh.Bun`       |
| **gh**        | latest   | `winget install GitHub.cli`        |
| **bxc clone** | `C:\worktree\bxc` on branch `aphrody` | `gh repo clone aphrody-code/bxc -- --branch aphrody C:\worktree\bxc` |
| **node**      | NEVER    | `node` forbidden per `feedback_bun_only` |

The `bxc-mass-scrape.ps1` launcher auto-clones `bxc` on first run if absent,
errors out cleanly otherwise.

## 3. Run shapes

### 3.1 Smoke test (1 page, no cache)

```pwsh
.\scripts\bxc-mass-scrape.ps1 -Force `
  -Urls (New-TemporaryFile | Tee-Object -Variable f).FullName
# Write a one-URL test list to $f.FullName first.
```

### 3.2 Full mass scrape (126 URLs, 6 lanes, fast profile, static mode)

```pwsh
.\scripts\bxc-mass-scrape.ps1
```

Estimated runtime: 90-180 s (depends on network + per-URL HTML size).

### 3.3 JS-heavy SPA mode (shadcn + material-web pages)

```pwsh
.\scripts\bxc-mass-scrape.ps1 -Mode full -Profile stealth -TimeoutMs 90000
```

`mode=full` triggers bxc to spawn Lightpanda via SocketPairTransport instead
of using the in-process CDP path. Slower but correctly renders SPAs.

### 3.4 High-concurrency aggressive sweep

```pwsh
.\scripts\bxc-mass-scrape.ps1 -Concurrency 12 -Profile max -Retry 4
```

`profile=max` enables every anti-fingerprint signal bxc exposes
(`curl-impersonate` ja3, header reordering, TLS profile rotation).

### 3.5 Refresh only failed URLs from last run

```pwsh
# Default: skips URLs whose <sha256>.html already exists + size > 0.
# Use -Force to invalidate the cache.
.\scripts\bxc-mass-scrape.ps1
```

## 4. Output schema

`var/data/bxc-cache/<sha256-of-url>.html` — raw DOM serialization
(`page.content()` or `document.documentElement.outerHTML` fallback).

`var/data/bxc-cache/manifest.json` (schema
`https://aphrody-code.dev/schemas/bxc-mass-scrape/v1`):

```json
{
  "$schema": "https://aphrody-code.dev/schemas/bxc-mass-scrape/v1",
  "generatedAt": "2026-05-17T20:42:00.000Z",
  "args": { "bxcRoot": "...", "urlsFile": "...", "cacheDir": "...",
            "concurrency": 6, "profile": "fast", "mode": "static",
            "timeoutMs": 60000, "retry": 2, "force": false },
  "stats": { "total": 126, "ok": 124, "cached": 0, "failed": 2,
             "totalBytes": 12345678, "totalMs": 142000,
             "failureRate": 0.0159 },
  "results": [
    { "url": "...", "category": "m3", "sha256": "...",
      "status": "ok", "bytes": 12345, "ms": 832, "attempt": 1,
      "error": null, "outputPath": "..." },
    ...
  ]
}
```

## 5. Failure budget + safety rails

- **Per-URL retry**: exponential backoff `250 * 2^(attempt-1)` ms.
  Default `--retry=2` → up to 3 attempts per URL.
- **Global failure threshold**: `FAIL_THRESHOLD = 0.5` in
  `scripts/bxc-mass-scrape.ts`. If > 50 % of URLs fail, the script writes
  the manifest then exits non-zero. PowerShell launcher propagates `$rc`.
- **Cache invalidation**: only `-Force` re-fetches an already-cached URL.
  Default behaviour is incremental.
- **Concurrency cap**: hard 1..32 range (parser rejects out-of-band).
- **Robots / ToS**: this is a developer-tool corpus seed, not a production
  crawler — keep concurrency moderate (default 6) and avoid hitting same
  origin > 4 parallel.

## 6. Integration with A2A coord channel

`scripts/bxc-mass-scrape.ts` is **aphrody-local** — it does not require the
peer winclean Claude to be online. The peer's bxc is owned by the peer; we
use *our own clone* at `C:\worktree\bxc`.

A separate `apx-ask-bxc-mass-scrape-1` envelope is appended to
`C:\winclean\.coord\inbox-from-aphrody.jsonl` so the peer can opportunistically
take half the URL list if it spins back up. The peer's response (if any) lands
in `C:\winclean\.coord\inbox-from-winclean.jsonl` with topic
`apx-ans-bxc-mass-scrape-1` and SHOULD reference the same URL hashes.

## 7. Verify

```pwsh
# Validate URL list parses
bun run -e "console.log(JSON.parse(require('fs').readFileSync('scripts/bxc-mass-scrape.urls.json','utf8')).urls.length)"
# Expected: 126

# Dry-run the orchestrator (will error on first URL if bxc is missing — that's the gate)
.\scripts\bxc-mass-scrape.ps1 -TimeoutMs 5000 -Retry 0
```

## 8. Next steps

- Wire the cache manifest into `crates/m3-tokens` build script so M3 token
  refresh becomes deterministic + offline-capable.
- Add `aphrody scrape` subcommand in `crates/cli` that shells out to
  `scripts/bxc-mass-scrape.ps1` for cross-platform users (Linux fallback
  via `bun run scripts/bxc-mass-scrape.ts` directly).
- Schedule a weekly cron via `.github/workflows/bxc-scrape-refresh.yml`
  to keep the corpus fresh.
