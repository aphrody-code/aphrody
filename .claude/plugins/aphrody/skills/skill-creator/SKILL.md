---
name: skill-creator
description: >-
  Create agent skills for any technology (Microsoft / Rust crate / Azure / .NET / browser API / framework) by investigating it live through the MCP servers wired into this plugin, then generating a hybrid SKILL.md with essential knowledge stored locally and dynamic lookups for depth. Use whenever the user wants to build, generate, scaffold, or improve a skill — even phrased casually as "make a skill for tokio" or "I need a skill that covers DirectX 12 RTV creation". Source: upstream `microsoftdocs/mcp` `microsoft-skill-creator` (MIT), generalised for the aphrody cross-platform scope on 2026-05-19.
license: MIT
upstream: https://github.com/microsoftdocs/mcp/tree/main/skills/microsoft-skill-creator
---

# Skill Creator

Generate hybrid skills that store essential knowledge locally while
enabling dynamic MCP lookups for deeper details. Generalises the
upstream Microsoft skill-creator to the **single unified MCP server**
this plugin wires (24 tools) :

- **Library / framework docs** — `mcp__aphrody__context7_resolve_library_id`
  → `mcp__aphrody__context7_query_docs` (Rust port of Context7,
  `mcp.context7.com/api/v2/{libs/search,context}`).
- **Microsoft / Azure / .NET / Windows / M365 / Power Platform** —
  `mcp__aphrody__microsoft_docs_search`,
  `mcp__aphrody__microsoft_docs_fetch`,
  `mcp__aphrody__microsoft_code_sample_search` (Rust HTTP-MCP proxy
  onto `learn.microsoft.com/api/mcp`).
- **Single-shot fanout** — `mcp__aphrody__docs_auto_search` fires
  Context7 + Microsoft Learn + code samples + Google in parallel
  (`tokio::join!`) and returns a fused markdown report. Default entry
  point when the topic is unfamiliar.
- **Generic web** — `mcp__aphrody__universal_web_fetch` (any URL,
  Jina reader-proxy) and `mcp__aphrody__agent_browser_scrape` (browser-based CSS
  extraction for JS-rendered docs).

## About Skills

Skills are modular packages that extend agent capabilities with
specialized knowledge and workflows. A skill transforms a
general-purpose agent into a specialized one for a specific domain.

### Skill Structure

```
skill-name/
├── SKILL.md (required)     # Frontmatter (name, description) + instructions
├── references/             # Documentation loaded into context as needed
├── sample_codes/           # Working code examples
└── assets/                 # Files used in output (templates, etc.)
```

### Key Principles

- **Frontmatter is critical**: `name` and `description` determine when
  the skill triggers — be clear and comprehensive.
- **Concise is key**: Only include what agents don't already know;
  context window is shared.
- **No duplication**: Information lives in SKILL.md OR reference files,
  not both.

## Discovery Tools

| Tool                                                   | Purpose                  | When to Use                          |
| ------------------------------------------------------ | ------------------------ | ------------------------------------ |
| `mcp__aphrody__microsoft_docs_search`          | Search official MS docs  | Microsoft / Azure / .NET / Windows   |
| `mcp__aphrody__microsoft_docs_fetch`           | Get full MS page content | Deep dive into important MS pages    |
| `mcp__aphrody__microsoft_code_sample_search`   | Find official MS samples | Need implementation patterns         |
| `mcp__aphrody__universal_web_fetch`                    | Generic URL → Markdown   | Non-Microsoft docs (docs.rs, MDN, …) |
| `mcp__aphrody__agent_browser_scrape`                   | Browser CSS extraction   | JS-rendered docs, dynamic content    |
| `mcp__aphrody__google_search`                          | Web search (stealth)     | Locating a doc page from concept     |

If `context7` MCP is installed alongside, prefer it for first-pass
library API surface lookups — it caches and indexes versioned docs.

## Creation Process

### Step 1 — Investigate the Topic

Build deep understanding in three phases:

**Phase 1 — Scope Discovery:**

```
# Microsoft tech
microsoft_docs_search(query="{technology} overview what is")
microsoft_docs_search(query="{technology} concepts architecture")
microsoft_docs_search(query="{technology} getting started tutorial")

# Generic tech
google_search(query="{technology} architecture concepts")
universal_web_fetch(url="https://docs.rs/{crate}/latest/{crate}/")    # Rust crate
```

**Phase 2 — Core Content:**

```
microsoft_docs_fetch(url="…")                # high-signal pages from Phase 1
microsoft_code_sample_search(query="{technology}", language="{lang}")
universal_web_fetch(url="…")                 # generic pages
agent_browser_scrape(url="…", selector="article") # JS-rendered docs
```

**Phase 3 — Depth:**

```
microsoft_docs_search(query="{technology} best practices")
microsoft_docs_search(query="{technology} troubleshooting errors")
google_search(query="{technology} pitfalls common mistakes")
```

#### Investigation Checklist

After investigating, verify:

- [ ] Can explain what the technology does in one paragraph
- [ ] Identified 3–5 key concepts
- [ ] Have working code for basic usage
- [ ] Know the most common API patterns
- [ ] Have search queries for deeper topics

### Step 2 — Clarify with User

Present findings and ask:

1. "I found these key areas: [list]. Which are most important?"
2. "What tasks will agents primarily perform with this skill?"
3. "Which programming language should code samples prioritize?"

### Step 3 — Generate the Skill

Use the appropriate template from
[skill-templates.md](references/skill-templates.md):

| Technology Type                    | Template           |
| ---------------------------------- | ------------------ |
| Rust crate (crates.io / docs.rs)   | **Rust Crate**     |
| Client library, NuGet / npm / pip  | SDK / Library      |
| Azure resource                     | Azure Service      |
| App development framework          | Framework/Platform |
| REST API, protocol, specification  | API / Protocol     |

#### Generated Skill Structure

```
{skill-name}/
├── SKILL.md                    # Core knowledge + dynamic-lookup guidance
├── references/                 # Detailed local documentation (if needed)
└── sample_codes/               # Working code examples
    ├── getting-started/
    └── common-patterns/
```

### Step 4 — Balance Local vs Dynamic Content

**Store locally when:**

- Foundational (needed for any task)
- Frequently accessed
- Stable (won't change)
- Hard to find via search

**Keep dynamic when:**

- Exhaustive reference (too large)
- Version-specific
- Situational (specific tasks only)
- Well-indexed (easy to search)

#### Content Guidelines

| Content Type          | Local               | Dynamic             |
| --------------------- | ------------------- | ------------------- |
| Core concepts (3–5)   | Full                |                     |
| Hello world code      | Full                |                     |
| Common patterns (3–5) | Full                |                     |
| Top API methods       | Signature + example | Full docs via fetch |
| Best practices        | Top 5 bullets       | Search for more     |
| Troubleshooting       |                     | Search queries      |
| Full API reference    |                     | Doc links           |

### Step 5 — Validate

1. **Review**: Is local content sufficient for common tasks?
2. **Test**: Do suggested search queries return useful results?
3. **Verify**: Do code samples compile / run without errors?
   - For Rust : `cargo build` in `sample_codes/getting-started/`.
   - For C# / Python / JS : copy into a scratch project and run.

## Common Investigation Patterns

### For Rust crates

```
"crate {name} overview"                          → docs.rs landing
"crate {name} examples"                          → docs.rs examples/
universal_web_fetch("https://docs.rs/{name}/latest/{name}/")
universal_web_fetch("https://crates.io/crates/{name}")
google_search("{name} crate gotchas")
```

### For SDKs / Libraries

```
"{name} overview" → purpose, architecture
"{name} getting started quickstart" → setup steps
"{name} API reference" → core classes/methods
"{name} samples examples" → code patterns
"{name} best practices performance" → optimization
```

### For Azure Services

```
"{service} overview features" → capabilities
"{service} quickstart {language}" → setup code
"{service} REST API reference" → endpoints
"{service} SDK {language}" → client library
"{service} pricing limits quotas" → constraints
```

### For Frameworks / Platforms

```
"{framework} architecture concepts" → mental model
"{framework} project structure" → conventions
"{framework} tutorial walkthrough" → end-to-end flow
"{framework} configuration options" → customization
```

## Example: Creating a "tokio" Skill

### Investigation

```
universal_web_fetch(url="https://docs.rs/tokio/latest/tokio/")
universal_web_fetch(url="https://tokio.rs/tokio/tutorial")
google_search(query="tokio runtime flavor multi_thread current_thread")
google_search(query="tokio gotchas pitfalls blocking")
```

### Generated Skill (sketch)

```
tokio/
├── SKILL.md
└── sample_codes/
    ├── getting-started/
    │   └── hello_async.rs
    └── common-patterns/
        ├── select_loop.rs
        └── join_set.rs
```

### Generated SKILL.md

```markdown
---
name: tokio
description: Async runtime for Rust. Use for any task spawning async tasks, awaiting futures, building TCP servers, scheduling timeouts, or coordinating channels in Rust code.
---

# tokio

The de-facto async runtime for Rust — work-stealing multi-threaded
scheduler, I/O reactor, time wheel, and channel primitives.

## Key Concepts

- **Runtime**: `current_thread` (single-thread) vs `multi_thread` (work-stealing).
- **`tokio::spawn`**: spawn an `async` task on the runtime.
- **`select!`**: race multiple futures, branch on first to complete.
- **`JoinSet`**: dynamic set of spawned tasks with structured concurrency.
- **`tokio::sync`**: `Mutex`, `RwLock`, `mpsc`, `oneshot`, `broadcast`,
  `Notify`, `Semaphore`.

## Quick Start

See [getting-started/hello_async.rs](sample_codes/getting-started/hello_async.rs).

## Learn More

| Topic                | How to Find                                                                  |
| -------------------- | ---------------------------------------------------------------------------- |
| Runtime flavors      | `universal_web_fetch("https://docs.rs/tokio/latest/tokio/runtime/index.html")` |
| Channels             | `universal_web_fetch("https://docs.rs/tokio/latest/tokio/sync/index.html")`  |
| Tutorial             | `universal_web_fetch("https://tokio.rs/tokio/tutorial")`                     |
```

## Example: Creating a "Semantic Kernel" Skill

### Investigation

```
microsoft_docs_search(query="semantic kernel overview")
microsoft_docs_search(query="semantic kernel plugins functions")
microsoft_code_sample_search(query="semantic kernel", language="csharp")
microsoft_docs_fetch(url="https://learn.microsoft.com/semantic-kernel/overview/")
```

### Generated Skill

```
semantic-kernel/
├── SKILL.md
└── sample_codes/
    ├── getting-started/
    │   └── hello-kernel.cs
    └── common-patterns/
        ├── chat-completion.cs
        └── function-calling.cs
```
