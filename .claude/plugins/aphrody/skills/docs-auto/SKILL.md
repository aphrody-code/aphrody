---
name: docs-auto
description: Single-shot documentation lookup that fans out across Context7 (community docs catalog with version pinning), Microsoft Learn (official Azure / .NET / Windows / M365 / Power Platform docs + code samples), and Google web search in PARALLEL, then returns a fused markdown report. Use FIRST whenever the user asks about ANY library, framework, SDK, CLI tool, cloud service or API — Rust crates (tokio, wgpu, reqwest…), JS frameworks (React, Next.js, Vue…), Python libs (FastAPI, Django, polars…), Azure services, .NET APIs, Windows internals, mobile SDKs, anything. Use even when you think you know the answer — training data may be outdated. Saves wall-clock time (one tool call vs three) and saves context-window tool descriptions (one summary vs four).
license: Apache-2.0
---

# docs-auto — fanout documentation lookup

A single `mcp__aphrody__docs_auto_search` call replaces a sequence of
manual Context7 / Microsoft Learn / Google queries. Internally the
`aphrody-mcp` binary runs all four backends in parallel via
`tokio::join!` and stitches the responses into one markdown report.

## When to Use This Skill

Trigger this skill (and call `docs_auto_search` immediately) whenever
the user :

- Asks an API / config / setup question that mentions a library or
  service by name (React, tokio, Cosmos DB, EF Core, Tailwind, …).
- Says "how do I", "what's the", "where can I find" anything related
  to a developer tool, SDK, or cloud service.
- Needs a code sample / example for a known framework or platform.
- Wants the latest version of an API they haven't used in a while.

Use this **before** falling back to the single-source skills
(`microsoft-docs`, `microsoft-code-reference`, `context7-mcp`) — those
remain available for **drill-down** after the fanout report identifies
a single canonical source.

## Call Signature

```
mcp__aphrody__docs_auto_search({
  query:        "Free-form natural-language question",
  library_name: "Optional library / framework / SDK name",  // unlocks Context7 deep-fetch
  language:     "Optional language filter for MS code samples",
})
```

## What You Get Back

A single markdown document with four sections :

1. **Context7** — community docs catalog. If `library_name` is set,
   includes the resolved library ID and a full `query-docs` fetch.
   Otherwise, just the resolution candidates.
2. **Microsoft Learn — concept / API search** — up to 10 high-quality
   doc chunks with title, URL and excerpt.
3. **Microsoft Learn — code samples** — official Microsoft / Azure code
   snippets matching the query (filtered by `language` if provided).
4. **Google — web search** — stealth Lightpanda profile, automatically
   appends "documentation API reference" to the query.

## Example Invocations

```js
// Generic — Context7 only resolves (no deep fetch)
docs_auto_search({
  query: "How do I structure a tokio task tree with structured concurrency?"
})

// Library-pinned — Context7 does resolve + deep fetch
docs_auto_search({
  query: "JoinSet vs spawn",
  library_name: "tokio",
  language: "rust"
})

// Azure / .NET centric
docs_auto_search({
  query: "Cosmos DB partition key design best practices",
  language: "csharp"
})

// Web framework
docs_auto_search({
  query: "App Router authentication middleware",
  library_name: "Next.js",
  language: "typescript"
})
```

## When to Drill Down

After the fused report, if the user wants the **full reference page**
for a specific URL surfaced in section 2 or 3, follow up with :

- `mcp__aphrody__microsoft_docs_fetch({ url })` — full Microsoft Learn page
- `mcp__aphrody__context7_query_docs({ library_id, query })` — targeted Context7 slice
- `mcp__aphrody__universal_web_fetch({ url })` — any other URL

## Why Use This Instead of Manual Multi-Call

| Cost              | Manual (3 tool calls)            | `docs_auto_search` (1 call)        |
| ----------------- | -------------------------------- | ---------------------------------- |
| Wall-clock        | sum of latencies (serial)        | max of latencies (parallel)        |
| Tool descriptors  | 3 in the model's working set     | 1                                  |
| Decision overhead | model picks the right backend   | server fans out automatically      |
| Error handling    | propagates one error per call    | all four reported in a single body |

The fanout is **free** server-side : same `reqwest` client pool, same
TLS session pool, same tokio runtime. The cost is purely the upstream
quota burn (which you'd pay either way if you eventually called all
three).
