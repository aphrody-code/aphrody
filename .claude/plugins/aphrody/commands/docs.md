---
description: Look up current documentation for any library, framework, SDK or cloud service via the unified docs_auto_search aggregator (Context7 + Microsoft Learn + Microsoft code samples + Google in parallel). Source — upstream `upstash/context7` plugin (MIT), generalised on 2026-05-19 and rewired onto the native Rust `mcp__aphrody__docs_auto_search` + drill-down tools.
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
  `prisma`) **or** a Context7 ID starting with `/` (e.g.
  `/sveltejs/svelte`, `/tokio-rs/tokio/v1.45.0`).
- **query** : What you're looking for (optional but **strongly
  recommended** — affects relevance ranking).

## Examples

```
/docs react hooks cleanup
/docs svelte layout load server side load
/docs tokio spawn current_thread runtime
/docs prisma one-to-many relations cascade delete
/docs /sveltejs/svelte/v5.0.0 runes state
/docs /supabase/supabase row level security
```

## How It Works

1. **Default path** — `$ARGUMENTS` is split into `library` (first token)
   and `query` (rest). Both are passed to
   `mcp__aphrody__docs_auto_search` which **fans out in parallel** to
   Context7 (resolve + deep fetch since `library_name` is set),
   Microsoft Learn search, Microsoft code-sample search, and Google web
   search. Returns one fused markdown report (~4 sections).
2. **Explicit Context7 ID path** — if the library argument **starts with
   `/`** (e.g. `/sveltejs/svelte/v5.0.0`), call
   `mcp__aphrody__context7_query_docs` directly with that ID — no
   resolution, no fanout (the user already knows what they want).
3. **Drill-down** — if the fused report surfaces a single canonical URL
   the user wants in full, follow up with
   `mcp__aphrody__microsoft_docs_fetch` or
   `mcp__aphrody__universal_web_fetch`.

## Version-Specific Lookups

Include the version in the library ID for pinned documentation :

```
/docs /sveltejs/svelte/v5.0.0 runes
/docs /facebook/react/v19.0.0 use hook
/docs /tokio-rs/tokio/v1.45.0 JoinSet
```

This is useful when working with a specific version and the docs need
to match exactly.

## Authentication

Works without authentication on the public tier. To unlock higher rate
limits, set `CONTEXT7_API_KEY` in your environment (or in
`.claude/aphrody.local.md` frontmatter). The aphrody MCP server picks
it up automatically on each call.

## Common Mistakes

- Library IDs **require a `/` prefix** — `/facebook/react`, not
  `facebook/react`.
- Always pass a query — `/docs react` will return generic React docs;
  `/docs react useEffect cleanup async` will return targeted snippets.
- Do **not** include sensitive information (API keys, passwords,
  credentials, proprietary code) in queries — they are sent to the
  Context7 SaaS.
