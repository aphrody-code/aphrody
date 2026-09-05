---
description: Look up current documentation for any library, framework, SDK or cloud service via the unified docs_auto_search aggregator (Context7 + Microsoft Learn + Microsoft code samples in parallel). Source — upstream `upstash/context7` plugin (MIT), generalised on 2026-05-19 and rewired onto the native Rust `mcp__aphrody__docs_auto_search` + drill-down tools.
argument-hint: <library> [query]
allowed-tools: mcp__aphrody__docs_auto_search, mcp__aphrody__context7_resolve_library_id, mcp__aphrody__context7_query_docs, mcp__aphrody__microsoft_docs_search, mcp__aphrody__microsoft_docs_fetch, mcp__aphrody__microsoft_code_sample_search
model: sonnet
---

# /docs — library documentation lookup

Fetches up-to-date documentation and code examples for `$ARGUMENTS`
through the **native Rust port** of the Context7 client embedded in the
`aphrody-mcp` binary.

## Usage

```
/docs <library> [query]
```

- **library** : The library name (e.g. `react`, `tokio`, `svelte`,
  `prisma`).
- **query** : What you're looking for (optional but **strongly
  recommended** — affects relevance ranking).

## Examples

```
/docs react hooks cleanup
/docs svelte layout load server side load
/docs tokio spawn current_thread runtime
/docs prisma one-to-many relations cascade delete
/docs svelte 5.0 runes state
/docs supabase row level security
```

## How It Works

1. **Default path** — `$ARGUMENTS` is split into `library` (first token)
   and `query` (rest). Both are passed to
   `mcp__aphrody__docs_auto_search` which **fans out in parallel** to
   Context7 (resolve + deep fetch since `library_name` is set),
   Microsoft Learn search and Microsoft code-sample search. Returns one
   fused markdown report (~3 sections).
2. **Context7 drill-down** — when a targeted follow-up is required, call
   `mcp__aphrody__context7_resolve_library_id`, select the matching result,
   then call `mcp__aphrody__context7_query_docs`. Never invent or reuse a
   stale ID without this resolution step.
3. **Microsoft drill-down** — if the fused report surfaces a Microsoft Learn
   canonical URL the user wants in full, follow up with
   `mcp__aphrody__microsoft_docs_fetch`.

## Version-Specific Lookups

Include the version in the query so Context7 can resolve a matching versioned
ID:

```
/docs svelte 5.0 runes
/docs react 19.0 use hook
/docs tokio 1.45 JoinSet
```

This is useful when working with a specific version and the docs need
to match exactly.

## Authentication

Works without authentication on the public tier. To unlock higher rate
limits, set `CONTEXT7_API_KEY` in your environment (or in
the repository's ignored local `.env`). Never store the value in tracked
Claude/Codex configuration or Markdown. The local aphrody MCP server loads
the nearest `.env` before serving requests.

## Common Mistakes

- Resolve the library on every Context7 drill-down; do not guess an ID from
  the package name or copy a previously resolved version blindly.
- Always pass a query — `/docs react` will return generic React docs;
  `/docs react useEffect cleanup async` will return targeted snippets.
- Do **not** include sensitive information (API keys, passwords,
  credentials, proprietary code) in queries — they are sent to the
  Context7 SaaS.
