---
name: context7-mcp
description: Fetch current library documentation, API references, and code examples for any developer technology (React, Next.js, tokio, wgpu, Prisma, Supabase, Express, Tailwind, Django, Spring Boot, …) — even well-known ones. Use this skill whenever the user asks setup or configuration questions ("How do I configure Next.js middleware?"), requests code involving libraries ("Write a Prisma query for…"), needs API references ("What are the Supabase auth methods?"), or mentions a specific framework or crate by name. Use even when you think you know the answer — training data may be outdated. Source — upstream `upstash/context7` (MIT), `skills/context7-mcp/SKILL.md`, imported 2026-05-19 and rewired onto the native Rust `mcp__aphrody__context7_*` tools.
license: MIT
upstream: https://github.com/upstash/context7/tree/master/skills/context7-mcp
---

# Context7 (via aphrody MCP)

When the user asks about libraries, frameworks, or needs code examples,
use Context7 — wired into this plugin as **two native Rust tools** on
the unified `aphrody-mcp` stdio server — to fetch current documentation
instead of relying on training data.

## Wired Tools

| Tool                                            | Purpose                                                         |
| ----------------------------------------------- | --------------------------------------------------------------- |
| `mcp__aphrody__context7_resolve_library_id`     | Resolve a package / framework name to a Context7 library ID     |
| `mcp__aphrody__context7_query_docs`             | Pull current docs + code snippets for a resolved library ID     |

The two tools call `https://mcp.context7.com/api/v2/{libs/search,context}`
in pure Rust (`reqwest` + `rustls/ring`). They use the `CONTEXT7_API_KEY`
env var as a Bearer token if present — the public tier works
unauthenticated.

## When to Use This Skill

Activate this skill when the user :

- Asks setup or configuration questions ("How do I configure Next.js middleware?")
- Requests code involving libraries ("Write a Prisma query for …")
- Needs API references ("What are the Supabase auth methods ?")
- Mentions specific frameworks (React, Vue, Svelte, Express, Tailwind, …)
- Mentions Rust crates by name (tokio, reqwest, wgpu, serde, …) — Context7
  indexes docs.rs in addition to JS / Python / etc.

## How to Fetch Documentation

### Step 1 — Resolve the Library ID

Call `mcp__aphrody__context7_resolve_library_id` with :

- `libraryName` : The library name extracted from the user's question
  (e.g. `"Next.js"`, `"tokio"`, `"Prisma"`).
- `query` : The user's full question (improves relevance ranking).

### Step 2 — Select the Best Match

From the resolution results, choose based on :

- Exact or closest name match to what the user asked for.
- Higher benchmark scores indicate better documentation quality.
- If the user mentioned a version (e.g. "React 19"), prefer
  version-specific IDs (format `/org/project/version`).

### Step 3 — Fetch the Documentation

Call `mcp__aphrody__context7_query_docs` with :

- `libraryId` : The selected Context7 library ID (e.g. `/vercel/next.js`).
- `query` : The user's specific question.

### Step 4 — Use the Documentation

Incorporate the fetched documentation into your response :

- Answer the user's question using current, accurate information.
- Include relevant code examples from the docs.
- Cite the library version when relevant.

## Guidelines

- **Be specific** : pass the user's full question as the query for
  better results.
- **Version awareness** : when users mention versions ("Next.js 15",
  "React 19"), use version-specific library IDs if available from the
  resolution step.
- **Prefer official sources** : when multiple matches exist, prefer
  official / primary packages over community forks.
- **Library IDs require a `/` prefix** — `/facebook/react`, not
  `facebook/react`.
- **Always resolve first** — calling `context7_query_docs` with a
  non-resolved name will return a structured error envelope
  (`CONTEXT7_BAD_REQUEST`).

## Error Handling

The tools return a structured JSON error envelope on failure :

```json
{
  "error": "CONTEXT7_TIMEOUT" | "CONTEXT7_UNAVAILABLE" | "CONTEXT7_BAD_REQUEST",
  "reason": "<details>",
  "endpoint": "https://mcp.context7.com/api/v2/..."
}
```

If the envelope reports `CONTEXT7_BAD_REQUEST` with a `Monthly quota
reached` snippet, tell the user the public quota is exhausted and
recommend setting `CONTEXT7_API_KEY` (from
[context7.com/dashboard](https://context7.com/dashboard)) in
`~/.bashrc` / Windows env vars. Do not silently fall back to training
data — always tell the user why Context7 was not used.

## Comparison with other tools in this plugin

| Task                           | Preferred tool                                                                                  |
| ------------------------------ | ----------------------------------------------------------------------------------------------- |
| Library docs / API reference   | **`context7_query_docs`** (this skill) — versioned, indexed, code snippets                      |
| Microsoft-specific docs        | `mcp__aphrody__microsoft_docs_search` (Azure / .NET / Windows / M365)                   |
| Generic web page → Markdown    | `mcp__aphrody__universal_web_fetch` (any URL, Jina reader-proxy via Google Style Guide tool)    |
| DOM-aware extraction           | `mcp__aphrody__bxc_scrape` (JS-rendered docs, dynamic content, CSS selector)                    |
| Web search                     | `mcp__aphrody__google_search` (stealth via Lightpanda)                                          |
