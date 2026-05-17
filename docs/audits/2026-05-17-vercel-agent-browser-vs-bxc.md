# SPDX-License-Identifier: Apache-2.0

# Compatibility Audit: vercel-labs/agent-browser vs aphrody's bxc

**Date:** 2026-05-17  
**Scope:** Can agent-browser integrate with bxc-driven orchestrators?  
**Verdict:** Side-by-side coexistence.

---

## Executive Summary

**agent-browser** (Vercel Labs) and **bxc** (aphrody) are fundamentally different:

- **agent-browser**: Rust CLI + daemon. Multi-engine (Chrome, Lightpanda). 25+ commands.
- **bxc**: TypeScript/Bun library. In-process, zero-spawn. High-concurrency static DOM.

**Direct adapter not practical** — agent-browser orchestrates its own daemon; bxc is a library. Rewriting bxc-mass-scrape to subprocess agent-browser CLI loses bxc's in-process concurrency.

**Recommendation:** Keep both. Use agent-browser for interactive + React introspection; bxc for high-concurrency static DOM.

---

## Capability Matrix

| Capability | agent-browser | bxc | Notes |
|---|---|---|---|
| Runtime | Rust (native) | Bun (1.3.14+) | agent-browser: daemon+CLI. bxc: library. |
| Spawn Mechanism | CDP + Lightpanda | In-process StaticDomTransport | bxc zero-spawn by design. |
| Anti-Bot | Chrome TLS + UA. iOS Safari Appium. | curl-impersonate ja3 (HTTP profile) | Different layers. |
| Cookie Injection | Playwright JSON, CDP, Chrome profile | Playwright JSON, CDP, Netscape .txt, HTTP header | Both multi-format. |
| Navigation API | goto(url, waitUntil, timeoutMs) | goto(url, waitUntil, timeoutMs) | Identical. |
| Content Extraction | page.content, outerHTML, $$, DOM | page.content, outerHTML, $$, Zig DOM FFI | bxc's Zig faster for static. |
| Streaming / SSE | Yes (WebSocket dashboard) | Unknown | agent-browser has dashboard. |
| Frame / Iframe | Yes (Frame API) | Minimal (mainFrame only) | agent-browser full support. |
| **React Introspection** | **YES (tree/renders/Web Vitals)** | **NO** | **EXCLUSIVE** |
| **AI Chat** | **YES (natural language)** | **NO** | **EXCLUSIVE** |
| **Annotated Screenshots** | **YES (--annotate)** | **NO** | **EXCLUSIVE** |
| Multi-Tab | Yes (tab new/close/labels) | Single Page | agent-browser multi-tab. |
| Auth Vault | Yes (encrypted) | No | **EXCLUSIVE** |
| Windows Binary | No (Chrome for Testing) | Lightpanda NOT on Windows | agent-browser friendly. |
| Cloud Providers | YES (6+ providers + iOS) | NO | **EXCLUSIVE** |
| Dashboard | YES (port 4848, live viewport) | NO | **EXCLUSIVE** |

---

## Public API

**agent-browser:** CLI-only. Commands: open, snapshot, click, fill, screenshot, get, find, wait, network route, cookies, react tree/renders, vitals, auth, tab, state, batch, chat, stream.

**bxc:** TypeScript library. Browser.newPage({ profile, mode, viewport, cookies }). Page: goto, title, content, screenshot, pdf, $, $$, evaluate, addCookies, route, unroute, blockResources, click, type, waitForSelector, locator.

---

## Verdict: Side-by-Side Coexistence

### Why NOT direct adapter:

1. Process overhead: Each agent-browser CLI call is subprocess. bxc in-process concurrency drops 50-80%.
2. JSON parse tax: agent-browser --json output requires parsing. bxc TypeScript API is zero-copy.
3. No equivalent factory: agent-browser has no Browser.newPage() without CLI subprocess.
4. Daemon state: agent-browser per-session. bxc per-page in-process.

**Effort:** 2-3 weeks functional; 8+ weeks production-ready.

### Use Cases:

**agent-browser:** React introspection, semantic/accessibility interaction, AI chat, multi-tab, cloud browsers, Windows without Chromium.

**bxc:** High-concurrency static DOM (6-32+ concurrent), sub-millisecond latency, Windows native, HTTP spoofing (ja3), Cloudflare/anti-bot, resource-constrained.

---

## Integration Steps

1. **Decision tree in PLAN.md:** Static DOM scale → bxc. React → agent-browser-react-audit. Interactive → agent-browser. Windows SPA → fallback.

2. **Create scripts/agent-browser-react-audit.ts:** CLI wrapper invoking agent-browser --enable react-devtools --json. Parse tree/renders/vitals. Write to var/data/react-audit/.

3. **Extend edge-mass-scrape.ts:** Fallback to agent-browser when bxc Lightpanda unavailable on Windows.

4. **Document in README.md:** Add "Mass-Scrape Strategies" section.

5. **CI/CD test both:** bxc concurrency, agent-browser React audit, edge fallback.

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Process overhead | High | Throughput -50-80% | Accept: use agent-browser for interactive only. |
| Daemon state leak | Medium | Processes interfere | Use --session isolation. Test cleanup. |
| Windows Lightpanda | Medium | bxc stuck in static mode | Monitor Lightpanda releases. |
| Cloud provider drift | Low | Adapters unmaintained | Accept upstream burden. |
| License | Low | Permissive Apache-2.0 | Keep both LICENSEs. |

---

## Top 3 Unknowns

1. **Lightpanda Windows Support:** When does bxc spawn Lightpanda on Windows?
2. **Mass-Scrape Scale:** Target concurrency? (<10/sec simpler; >50/sec bxc only.)
3. **React Introspection Priority:** Production need or dev-debug only?

---

## Top 3 Exclusive Capabilities

1. **React Introspection + Web Vitals:** Component tree, renders, suspense, LCP/CLS/TTFB/FCP/INP, hydration phases.
2. **AI Chat / Natural Language Automation:** English to agent-browser commands via Vercel AI Gateway.
3. **Real-Time Observability Dashboard:** Live viewport on port 4848, pair browsing via WebSocket.

---

## Conclusion

**Verdict: Side-by-side coexistence.**

bxc and agent-browser solve orthogonal problems:
- **bxc** = engine room: high-concurrency, latency-critical, static DOM.
- **agent-browser** = cockpit: interactive, React-aware, multi-cloud, AI-driven.

Maintain both, use boundary conditions to route work, and document trade-offs.
