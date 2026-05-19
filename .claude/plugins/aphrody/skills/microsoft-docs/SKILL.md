---
name: microsoft-docs
description: Understand Microsoft technologies by querying official documentation. Use whenever the user asks how something works, wants tutorials, needs configuration options, limits, quotas, or best practices for any Microsoft technology (Azure, .NET, M365, Windows, Power Platform, etc.) — even if they don't mention "docs." If the question is about understanding a concept rather than writing code, this is the right skill. Source: upstream `microsoftdocs/mcp` (MIT), imported 2026-05-19.
license: MIT
upstream: https://github.com/microsoftdocs/mcp/tree/main/skills/microsoft-docs
---

# Microsoft Docs

Live lookup pipeline for the **official Microsoft Learn documentation
corpus**, backed by three native Rust tools fused into the unified
`aphrody-mcp` binary. The tools proxy `https://learn.microsoft.com/api/mcp`
in pure Rust (`reqwest` + SSE unwrap) — no separate MCP server entry,
no JS runtime.

## Tools

| Tool                              | Use For                                                       |
| --------------------------------- | ------------------------------------------------------------- |
| `mcp__aphrody__microsoft_docs_search` | Find documentation — concepts, guides, tutorials, configuration |
| `mcp__aphrody__microsoft_docs_fetch`  | Get full page content (when search excerpts aren't enough)    |

## When to Use

- **Understanding concepts** — "How does Cosmos DB partitioning work?"
- **Learning a service** — "Azure Functions overview", "Container Apps architecture"
- **Finding tutorials** — "quickstart", "getting started", "step-by-step"
- **Configuration options** — "App Service configuration settings"
- **Limits & quotas** — "Azure OpenAI rate limits", "Service Bus quotas"
- **Best practices** — "Azure security best practices"
- **Windows internals** — Win32 API behavior, NTDLL semantics, WinRT class
  references when porting peer `winclean` Windows-only logic into the
  aphrody Rust workspace.

## Query Effectiveness

Good queries are specific:

```
# Too broad
"Azure Functions"

# Specific
"Azure Functions Python v2 programming model"
"Cosmos DB partition key design best practices"
"Container Apps scaling rules KEDA"
```

Include context:

- **Version** when relevant (`.NET 8`, `EF Core 8`, `Windows 11 24H2`)
- **Task intent** (`quickstart`, `tutorial`, `overview`, `limits`)
- **Platform** for multi-platform docs (`Linux`, `Windows`)

## When to Fetch Full Page

Fetch after search when:

- **Tutorials** — need complete step-by-step instructions
- **Configuration guides** — need all options listed
- **Deep dives** — user wants comprehensive coverage
- **Search excerpt is cut off** — full context needed

## Why Use This

- **Accuracy** — live docs, not training data that may be outdated
- **Completeness** — tutorials have all steps, not fragments
- **Authority** — official Microsoft documentation
- **No subprocess relay** — pure HTTP, zero JS / Bun / Node runtime
  required (consistent with the aphrody `Rust-only` policy)
