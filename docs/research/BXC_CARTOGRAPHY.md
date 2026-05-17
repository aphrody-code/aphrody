<!-- SPDX-License-Identifier: Apache-2.0 -->
# bxc (Bun+Lightpanda) — Cartographie pour intégration aphrody

> Document de recherche pour OBJECTIF #2 + #3 — utiliser bxc@aphrody comme
> scraper Google Design/M3 + MCP `bxc-scrapper` du plugin Claude Code.
>
> Source : agent Explore (2026-05-17).

---

## 1. Architecture top-level

**bxc** = moteur de navigation "Zero-Spawn" fusionnant :
- **Bun** (runtime JS)
- **Lightpanda** (DOM Zig)
- **Rust V8 bindings**
- via une FFI bridge sans processus externes

**Trois niveaux de profils** :
- `static` (in-process, no JS)
- `fast`/`stealth`/`max` (CDP WebSocket spawned)
- `http` (curl-impersonate, TLS-fingerprinted)

**Rust-bridge** : 7 crates `obscura-*` + `bxc-engine` binary.
**Vendored** : MCP SDK TypeScript + Gemma 4 (llama.cpp) pour extraction IA + memory SQLite.

---

## 2. Modules Google natifs (`src/google/`, 13 modules)

| Module | Fonction |
|---|---|
| `atlas.ts` | **5637 audits** de domaines Google (366 uniques), auto-généré ; `CDN_COUNTS` (GFE=5637), `FRAMEWORK_COUNTS` (Wiz=20, Angular=1, Lit=1), ALL_HOSTS list |
| `client.ts` | `GoogleClient` : `.open(url)` avec mandate enforcement + Atlas-aware routing auto-detect (stealth-wiz, stealth-spa, stealth-lit) + network audit |
| `search.ts` | `googleWebSearch()`, `googleSearchRich()` : SERP parsing via ZigQuery, cache TTL, ghost profile (Lightpanda + stealth), local LLM via Gemma |
| `serp-parser.ts` | `parseSerp()` : organic, featured snippet, knowledge panel, PAA, rich elements |
| `verticals.ts` | Verticals (News, Images, Videos, Scholar) + ZigQuery element-scoped |
| `detector.ts` | Google tech detection (Wiz, Angular, Lit, GFE markers) + wappalyzergo |
| `mandate-guard.ts` | `enforceMandate(url)` : whitelist + géo-restriction (Google-only testing) |
| `mass-scanner.ts` | `GoogleMassScanner` : batch 5600+ domains, perf telemetry |
| `cache.ts` | `sharedCache` Redis-compatible (SERP + domain audits, TTL) |
| `fetch.ts` | Google-safe fetch (header normalization, UA rotation, proxy) |
| `dns.ts` | `isGoogleDomain()`, CNAME chain analysis |
| `rate-limit.ts` | Rate limiter per domain (GFE backpressure, expo backoff) |
| `strategy.ts` | `suggestGoogleStrategy()` : decision tree Atlas + tech |

**Detection plus large** :
- `src/detect.ts` : framework/CMS via wappalyzergo Go binary → `DetectedTech[]`
- `src/detect-deep.ts` : multi-signal (headers, DNS, IP ranges, HTML, CSP) → buckets `{frontend, backend, cdn, dns, hosting, analytics, cms}`

---

## 3. API surface scraping

### Core Browser API (`src/api/browser.ts`)

```typescript
const page = await Browser.newPage({
  profile: "static" | "fast" | "stealth" | "max" | "http",
  viewport?, userAgent?, cookies?,
});
await page.goto(url);
const html = await page.content();
const els = await page.$$(selector);
await page.screenshot({ path });
const result = await page.evaluate((x) => x * 2, 5);
```

### CLI entrypoints

| Cmd | Fonction |
|---|---|
| `bxc scrape <url> <selector>` | Extract textContent (JSON) |
| `bxc recon <url>` | Headers, CDN, frameworks, assets, selectors, screenshot → MD/JSON |
| `bxc detect <url>` | Framework detection via wappalyzer |
| `bxc har record/replay` | HTTP Archive |
| `bxc mirror <url> <out>` | Full site mirror w/ rewritten links |
| `bxc cookies load <jar.json>` | Cookie injector |

### Structured extraction (`packages/llm-extract`)

```typescript
import { extractStructured } from "@aphrody-code/llm-extract";
import { z } from "zod";

const schema = z.object({ title: z.string(), price: z.number().optional() });
const data = await extractStructured(html, { schema, url });
// 1ère call : LLM → CSS selectors (cached)
// suivantes : <1ms via ZigQuery
```

### Google-specific

```typescript
import { google } from "bxc/google";
const { page, audit } = await google.open("https://m3.material.io");
const results = await googleWebSearch("material design 3");
const rich = await googleSearchRich("dynamic color", { hl: "en" });
```

---

## 4. CDP / Lightpanda

**In-process CDP stack** (`src/cdp/`) : 18 domains
- Accessibility, Audits, Browser, DOM, Emulation, Fetch, Input, IO, Log,
- Network, Page, Performance, Runtime, Security, Storage, Target, Tracing, WebMCP

**Lightpanda FFI** : `ZigQuery` wrapper (`querySelector{,All}`, `textContent`, `innerHTML`, `getAttribute`). Path natif Zig sans JS execution (html5ever + cssparser).

**JS execution** (profile `fast|stealth|max`) : V8 isolate per page, `page.evaluate`, listeners, hydration awaits.

---

## 5. Rust-bridge crates (7)

| Crate | Rôle | Exports clés |
|---|---|---|
| `obscura-dom` | HTML5 parsing (html5ever) + DOM tree + CSS (cssparser) | `parse_html()`, `DomTree`, `NodeId` |
| `obscura-net` | HTTP client (reqwest-tokio), cookie jar, robots, tracker blocklist, stealth (JA3/JA4) | `ObscuraHttpClient`, `StealthHttpClient`, `CookieJar` |
| `obscura-browser` | Page lifecycle, navigation, render tree | `Page`, `Context` |
| `obscura-cdp` | CDP protocol dispatch, 18 domains | `start()`, `start_with_options()` |
| `obscura-js` | V8 runtime + Markdown + ops (read/write/eval) | `V8Runtime`, `MarkdownConverter` |
| `obscura-mcp` | MCP server wiring (HTTP transport) | MCP stdio handler |
| `bxc-engine` | Binary entrypoint : CDP server + worker pool, 24 subagents | `bxc serve/detect/recon` |

**FFI exports C** (`rust-bridge/src/lib.rs`) :
```rust
extern "C" fn bxc_parse_html(html: *const c_char) -> *mut DomTree;
extern "C" fn bxc_query_selector(tree: *mut DomTree, sel: *const c_char) -> *mut c_char;
extern "C" fn bxc_query_selector_all(tree: *mut DomTree, sel: *const c_char) -> *mut c_char;
extern "C" fn bxc_extract_title(html: *const c_char) -> *mut c_char;
extern "C" fn bxc_strip_tags(html: *const c_char) -> *mut c_char;
```

---

## 6. Pooling & concurrency (`src/pool/`)

- `PagePool` : high-throughput, LRU eviction, backpressure
- `AutoscaledPool` : workers auto-scaling + jitter
- `SessionPool` : cookie + auth reuse
- `ProxyPool` : sticky per domain, fallback 407
- `RequestQueue` : FIFO + rate limit per domain

```typescript
const pool = new PagePool({ profile: "fast", concurrency: 50 });
const titles = await pool.run(urls, async (page, url) => {
  await page.goto(url);
  return page.title();
});
```

---

## 7. Packages secondaires

| Package | Rôle |
|---|---|
| `api` | Elysia GraphQL + REST (`/api/v1/scrape`, `/graphql`) |
| `bxc-extension` | **MCP server natif `bxc-gemini`** : 7 tools (tune_memory_sqlite, vision_analyze, start_scraping_subagent, auto_detect_skills, ...) |
| `llm-extract` | Gemma 4 + llama.cpp, single-stream, selector caching, <1ms/page après LLM |

---

## 8. Benchmarks (vs upstream)

| Scenario | bxc-static | Playwright |
|---|---|---|
| Cold start | 85 ms | 850 ms |
| Memory | 38 MB | 240 MB |
| DOM latency | 50 µs | 5 ms |

---

## 9. Plan d'intégration aphrody

### MCP `bxc-scrapper` (plugin Claude Code)

Tools à exposer (depuis bxc-extension existant) :
- `bxc_scrape` : url + selector → JSON
- `bxc_recon` : url → {headers, cdn, frameworks, assets, css, screenshot}
- `bxc_detect` : url → DetectedTech[]
- `google_search` : query → OrganicResult[]
- `google_atlas_route` : url → recommended profile + stealth hints
- `extract_structured` : html + zod schema → typed fields
- `vision_analyze` : screenshot → {elements, text, colors, fonts, hierarchy}

### Skill `pixel-perfect`

- Lever sur `vision_analyze` (MCP tool natif bxc)
- Canvas-less screenshot analysis via Rust vision pipeline
- Design token extraction (colors, fonts, spacing) via CDP + LLM
- Pipeline scraping M3 :
  1. `google.open("https://m3.material.io/components")` → component list
  2. Pour chaque composant : `page.evaluate(getCSSVars)` → tokens
  3. `vision_analyze(screenshot)` → confirmation visuelle
  4. Output : `packages/ui/tokens/m3.json` + `packages/ui/components/<name>/spec.json`

### Crates Rust à intégrer en workspace.dependencies de aphrody

Via git+branch="aphrody" :
```toml
obscura-dom     = { git = "https://github.com/aphrody-code/bxc.git", branch = "aphrody", package = "obscura-dom" }
obscura-net     = { git = "https://github.com/aphrody-code/bxc.git", branch = "aphrody", package = "obscura-net" }
obscura-browser = { git = "https://github.com/aphrody-code/bxc.git", branch = "aphrody", package = "obscura-browser" }
obscura-cdp     = { git = "https://github.com/aphrody-code/bxc.git", branch = "aphrody", package = "obscura-cdp" }
obscura-js      = { git = "https://github.com/aphrody-code/bxc.git", branch = "aphrody", package = "obscura-js" }
obscura-mcp     = { git = "https://github.com/aphrody-code/bxc.git", branch = "aphrody", package = "obscura-mcp" }
```

### Priorisation

1. **Core stack** (immédiat) : obscura-dom + obscura-net + bxc-engine binary → CLI
2. **Google ecosystem** (haute) : atlas.ts + client.ts + search.ts + detect-deep.ts + mandate-guard.ts
3. **Stealth & anti-bot** (moyenne) : profiles/ghost + curl-impersonate JA3/JA4
4. **Extraction IA** (moyenne) : llm-extract + skill `pixel-perfect` MCP
5. **Pooling** (basse, post-alpha) : PagePool + AutoscaledPool + api GraphQL

---

## 10. Fichiers-clés

- Architecture : `C:\worktree\bxc\{README,MEGA-PLAN,CLAUDE,GEMINI}.md`
- Google : `src/google/*`, `src/detect{,-deep}.ts`
- API : `src/api/browser.ts`, `src/api/types.ts`
- Rust FFI : `rust-bridge/src/lib.rs`, `crates/obscura-*/src/lib.rs`
- CLI : `src/cli/{scrape,recon,detect,serve}.ts`
- Extraction : `packages/llm-extract/src/*`
- MCP : `packages/bxc-extension/server.ts`
