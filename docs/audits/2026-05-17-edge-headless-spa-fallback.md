<!-- SPDX-License-Identifier: Apache-2.0 -->

# Audit — Edge headless SPA-rendering fallback (Windows-native "max power")

**Date (UTC):** 2026-05-17
**Initiator:** user — "tu n'utilises pas tout le potentiel de bxc,
utilise son meilleur profile puissance max"
**Outcome:** bxc max profiles unavailable on Windows. Shipped
`scripts/edge-mass-scrape.ts` as the Windows-native JS-rendering
fallback. A/B verified: bxc-static 35 964 B vs Edge headless
~200 KB / page on Angular Material SPAs.

## 1. Why bxc max is unreachable on Windows

bxc's profile matrix:

| profile     | backend                              | Windows-native? |
|---|---|---|
| `static`    | in-process StaticDomTransport        | ✅ works         |
| `http`      | curl-impersonate (libcurl-impersonate.dll) | ❌ DLL absent |
| `fast`      | WebSocketTransport → bxc-engine     | ❌ Linux/macOS only |
| `stealth`   | WebSocketTransport → bxc-engine     | ❌ Linux/macOS only |
| `max`       | WebSocketTransport → bxc-engine     | ❌ Linux/macOS only |

`bun run C:/worktree/bxc/src/cli/install.ts` produced:

> `[bxc install] WARNING : Unsupported platform win32/x64. Lightpanda
> supports linux-x64, linux-arm64, darwin-x64, darwin-arm64.`

`profile=http` returned: `libcurl-impersonate not found. Expected in
vendor/curl-impersonate/`. The Lightpanda installer did not stage the
companion DLL.

**Net:** on Windows, the maximum bxc can deliver natively is
`profile=static` (in-process DOM, no JS execution).

## 2. The fallback: `scripts/edge-mass-scrape.ts`

Microsoft Edge ships with every Windows 11 install at
`C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`.
Edge's Chromium backend supports `--headless=new --dump-dom <url>`,
which prints the post-hydration DOM serialisation to stdout after JS
execution.

The new orchestrator:

- Same URL JSON format as `bxc-mass-scrape.ts` (no churn for consumers)
- Same manifest schema (v1) + adds `"engine": "edge"` discriminator
- Same concurrency pool / retry / cache-by-sha256 pipeline
- Isolated per-spawn `--user-data-dir` (no cross-contamination)
- `--virtual-time-budget=N ms` for deterministic SPA capture
- 11 Edge flags applied for clean headless run (no GPU, no first-run,
  no translate prompt, no extensions, etc.)

### CLI

```pwsh
# Default cache: var/data/edge-cache/
bun run scripts/edge-mass-scrape.ts `
  --urls=scripts/edge-mass-scrape.angular-spa.urls.json `
  --concurrency=2 --timeout=45000 --virtual-time=12000 --retry=1
```

### Flags

| Flag            | Default | Purpose                                |
|---|---|---|
| `--edge=<path>` | C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe | binary  |
| `--urls=<file>` | scripts/bxc-mass-scrape.urls.json | URL JSON list   |
| `--cache=<dir>` | var/data/edge-cache | per-URL HTML + manifest     |
| `--concurrency` | 4       | parallel Edge spawns (1..16)           |
| `--timeout`     | 45000   | kill timer per URL (ms)                |
| `--retry`       | 1       | retry count (0..4), exponential backoff|
| `--virtual-time`| 8000    | freeze JS timers after N ms (deterministic) |
| `--force`       | off     | invalidate cache, re-fetch all         |

## 3. A/B benchmark — bxc-static vs Edge headless

Same 5 Angular Material SPA URLs. bxc returned 35 964 B identical
(empty shell). Edge headless `--virtual-time-budget=12000`:

| URL                                                    | bxc-static | Edge headless | factor |
|---|---|---|---|
| `material.angular.dev/components/button/overview`      | 35 964 B   | 207 559 B     | **5.8×** |
| `material.angular.dev/components/card/overview`        | 35 964 B   | 177 056 B     | 4.9×    |
| `material.angular.dev/components/dialog/overview`      | 35 964 B   | 234 104 B     | **6.5×** |
| `material.angular.dev/components/tabs/overview`        | 35 964 B   | 229 398 B     | 6.4×    |
| `material.angular.dev/components/slider/overview`      | 35 964 B   | 199 885 B     | 5.6×    |

Total: 1 023.4 KB in 16 079 ms (5 URLs, concurrency=2).

Content quality verified on the button page — Edge captured the full
API table including `MAT_BUTTON_CONFIG`, `MatAnchor`, `MatButton`,
`MatButtonToggle`, `MatIcon`, plus directive attributes
`matButton="outlined"`, `matFab`, `matIconButton`, `matMiniFab`, and
all `aria-*` / `disabledInteractive` / `iconPositionEnd` API fields.
None of these strings appear in the bxc-static shell.

## 4. When to use each engine

| Site type                         | Engine             | Profile / flags       |
|---|---|---|
| Static HTML, MDN, GitHub repo     | bxc                | `--profile=static`    |
| SSR'd page (home of design.google)| bxc                | `--profile=static`    |
| TLS-fingerprinted scrape (when DLL present) | bxc      | `--profile=http`      |
| **SPA hydration required (Angular / Next.js article pages)** | **edge** | **`--virtual-time=12000`** |
| Anti-bot, full Chrome (Linux/Mac) | bxc                | `--profile=max`       |

## 5. Files committed in this pass

- `scripts/edge-mass-scrape.ts` — Edge-headless orchestrator (~265 l.)
- `scripts/edge-mass-scrape.angular-spa.urls.json` — A/B test 5-URL set
- `docs/audits/2026-05-17-edge-headless-spa-fallback.md` — this report

## 6. Verification

```pwsh
# Smoke (3 stable static URLs, ~8 s)
bun run scripts/edge-mass-scrape.ts `
  --urls=scripts/bxc-mass-scrape.smoke.urls.json `
  --cache=var/data/edge-cache-smoke --force

# Angular Material A/B (5 SPA URLs, ~16 s)
bun run scripts/edge-mass-scrape.ts `
  --urls=scripts/edge-mass-scrape.angular-spa.urls.json `
  --cache=var/data/edge-cache --virtual-time=12000 --force
```

Both verified in this session: 3/3 OK 9.0 KB 8.1 s; 5/5 OK 1 023 KB 16 s.

## 7. Roadmap

- Wire an `--engine={bxc,edge}` flag into a single shared orchestrator,
  so callers can A/B compare without two script names.
- Add `--engine=auto` mode that picks bxc-static first, then re-fetches
  with Edge if response byte size matches a known SPA-shell fingerprint
  (e.g. exactly the same byte count across 3+ URLs from the same host).
- When `bxc-engine` ships a Windows binary (or libcurl-impersonate.dll
  becomes available locally), bxc max profiles become viable on
  Windows too — switch the SPA path back to bxc for the in-process
  performance win.
