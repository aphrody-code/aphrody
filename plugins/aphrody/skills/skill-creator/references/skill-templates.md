# Skill Templates

Ready-to-use templates for different technology types. Five templates
cover the common shapes encountered in the aphrody scope.

All templates below reference the two MCP servers wired into the
aphrody plugin:

- `mcp__aphrody__*` — Microsoft Learn HTTP MCP (Azure / .NET /
  Windows / M365 / Power Platform).
- `mcp__aphrody__*` — local stdio MCP (universal_web_fetch, agent_browser_scrape,
  google_search, …) for non-Microsoft documentation.

If the user has `context7` MCP installed alongside, prefer it for
language-agnostic library docs (Rust crates, Node packages, Python libs).

---

## Template 1: Rust Crate Skill

For crates published on crates.io / docs.rs. **Default template for any
new aphrody workspace crate or third-party Rust dependency.**

````markdown
---
name: {crate-name}
description: {What it does in one sentence}. Use when agents need to {primary task} with {technology context} in Rust.
---

# {Crate Name}

{One paragraph: what the crate is, why it exists, when to use it,
what its competitors are and why this one was chosen.}

## Installation

```toml
# Cargo.toml
[dependencies]
{crate-name} = { version = "{X.Y}", features = ["{feature}"] }
```

Workspace-wide install (preferred per aphrody policy):

```toml
# Cargo.toml [workspace.dependencies]
{crate-name} = { workspace = true }
```

## Key Concepts

### {Concept 1: e.g. Runtime}

{Brief explanation, 2-3 lines max.}

### {Concept 2: e.g. Builder pattern}

{Brief explanation.}

### {Concept 3: e.g. Error model}

{Brief explanation.}

## Quick Start

See [getting-started/hello.rs](sample_codes/getting-started/hello.rs).

Inline (if < 25 lines):

```rust
use {crate_name}::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // …
    Ok(())
}
```

## Common Patterns

### {Pattern 1}

```rust
{code}
```

### {Pattern 2}

```rust
{code}
```

## API Quick Reference

| Type / Function | Purpose         | Example         |
| --------------- | --------------- | --------------- |
| `{Foo}`         | {what it does}  | `Foo::new(…)`   |
| `{bar()}`       | {what it does}  | `bar(&input)?`  |

For the full API surface :

- `universal_web_fetch(url="https://docs.rs/{crate-name}/latest/{crate_name}/")`
- `universal_web_fetch(url="https://docs.rs/{crate-name}/latest/{crate_name}/{module}/")`

## Best Practices

- **Do**: {recommendation}.
- **Do**: {recommendation}.
- **Avoid**: {anti-pattern}.

## Gotchas

- {Specific footgun #1 — version pin, feature flag, MSRV, …}
- {Specific footgun #2}

## Learn More

| Topic              | How to Find                                                                                |
| ------------------ | ------------------------------------------------------------------------------------------ |
| {Advanced topic 1} | `universal_web_fetch(url="https://docs.rs/{crate}/latest/{crate}/{topic}/")`               |
| Source             | `universal_web_fetch(url="{github-url}")`                                                  |
| Crate metadata     | `universal_web_fetch(url="https://crates.io/crates/{crate}")`                              |
| Examples           | `agent_browser_scrape(url="https://docs.rs/{crate}/latest/{crate}/", selector=".examples")`          |
````

---

## Template 2: SDK / Library Skill

For client libraries, SDKs, and programming frameworks (NuGet / npm /
pip / gem). For Rust crates, prefer Template 1.

````markdown
---
name: {sdk-name}
description: {What it does}. Use when agents need to {primary task} with {technology context}. Supports {languages/platforms}.
---

# {SDK Name}

{One paragraph: what it is, why it exists, when to use it.}

## Installation

{Package manager commands for supported languages}

## Key Concepts

{3-5 essential concepts, one paragraph each max.}

### {Concept 1}

{Brief explanation.}

### {Concept 2}

{Brief explanation.}

## Quick Start

{Minimal working example — inline if < 30 lines, otherwise reference
sample_codes/}

## Common Patterns

### {Pattern 1: e.g. Basic CRUD}

```{language}
{code}
```

### {Pattern 2: e.g. Error Handling}

```{language}
{code}
```

## API Quick Reference

| Class / Method | Purpose        | Example   |
| -------------- | -------------- | --------- |
| {name}         | {what it does} | `{usage}` |

For full API documentation:

- `microsoft_docs_search(query="{sdk} {class} API reference")`
- `microsoft_docs_fetch(url="{url}")`

## Best Practices

- **Do**: {recommendation}
- **Do**: {recommendation}
- **Avoid**: {anti-pattern}

## Learn More

| Topic              | How to Find                                                                 |
| ------------------ | --------------------------------------------------------------------------- |
| {Advanced topic 1} | `microsoft_docs_search(query="{sdk} {topic}")`                              |
| {Advanced topic 2} | `microsoft_docs_fetch(url="{url}")`                                         |
| {Code examples}    | `microsoft_code_sample_search(query="{sdk} {scenario}", language="{lang}")` |
````

---

## Template 3: Azure Service Skill

For Azure services and cloud resources.

````markdown
---
name: {service-name}
description: Work with {Azure Service}. Use when agents need to {primary capabilities}. Covers provisioning, configuration, and SDK usage.
---

# {Azure Service Name}

{One paragraph: what the service does, primary use cases.}

## Overview

- **Category**: {Compute / Storage / AI / Networking / …}
- **Key capability**: {main value proposition}
- **When to use**: {scenarios}

## Getting Started

### Prerequisites

- Azure subscription
- {Other requirements}

### Provisioning

{CLI / Portal / Bicep snippet for creating the resource}

## SDK Usage ({Language})

### Installation

```
{package install command}
```

### Authentication

```{language}
{auth code pattern}
```

### Basic Operations

```{language}
{CRUD or primary operations}
```

## Key Configurations

| Setting   | Purpose            | Default |
| --------- | ------------------ | ------- |
| {setting} | {what it controls} | {value} |

## Pricing & Limits

- **Pricing model**: {consumption / tier-based / …}
- **Key limits**: {important quotas}

For current pricing: `microsoft_docs_search(query="{service} pricing")`

## Common Patterns

### {Pattern 1}

{Code or configuration.}

### {Pattern 2}

{Code or configuration.}

## Troubleshooting

| Issue          | Solution |
| -------------- | -------- |
| {Common error} | {Fix}    |

For more issues: `microsoft_docs_search(query="{service} troubleshoot {symptom}")`

## Learn More

| Topic     | How to Find                                                        |
| --------- | ------------------------------------------------------------------ |
| REST API  | `microsoft_docs_fetch(url="{url}")`                                |
| ARM/Bicep | `microsoft_docs_search(query="{service} bicep template")`          |
| Security  | `microsoft_docs_search(query="{service} security best practices")` |
````

---

## Template 4: Framework / Platform Skill

For development frameworks and platforms (ASP.NET, MAUI, Blazor,
Next.js, Tauri, …).

````markdown
---
name: {framework-name}
description: Build {type of apps} with {Framework}. Use when agents need to create, modify, or debug {framework} applications.
---

# {Framework Name}

{One paragraph: what it is, what you build with it, why choose it.}

## Project Structure

```
{typical-project}/
├── {folder}/        # {purpose}
├── {file}           # {purpose}
└── {file}           # {purpose}
```

## Getting Started

### Create New Project

```bash
{CLI command to scaffold}
```

### Project Configuration

{Key files to configure and what they control.}

## Core Concepts

### {Concept 1: e.g. Components}

{Explanation with minimal code example.}

### {Concept 2: e.g. Routing}

{Explanation with minimal code example.}

### {Concept 3: e.g. State Management}

{Explanation with minimal code example.}

## Common Patterns

### {Pattern 1}

```{language}
{code}
```

### {Pattern 2}

```{language}
{code}
```

## Configuration Options

| Setting   | File   | Purpose        |
| --------- | ------ | -------------- |
| {setting} | {file} | {what it does} |

## Deployment

{Brief deployment guidance or reference.}

For detailed deployment: `microsoft_docs_search(query="{framework} deploy {target}")`

## Learn More

| Topic              | How to Find                                                    |
| ------------------ | -------------------------------------------------------------- |
| {Advanced feature} | `microsoft_docs_search(query="{framework} {feature}")`         |
| {Integration}      | `microsoft_docs_fetch(url="{url}")`                            |
| {Samples}          | `microsoft_code_sample_search(query="{framework} {scenario}")` |
````

---

## Template 5: API / Protocol Skill

For APIs, protocols, and specifications (Microsoft Graph, OOXML, gRPC,
MCP itself, …).

````markdown
---
name: {api-name}
description: Interact with {API/Protocol}. Use when agents need to {primary operations}. Covers authentication, endpoints, and common operations.
---

# {API/Protocol Name}

{One paragraph: what it provides access to, primary use cases.}

## Authentication

{Auth method and code pattern.}

## Base Configuration

- **Base URL**: `{url}`
- **Version**: `{version}`
- **Format**: {JSON / XML / protobuf / …}

## Common Endpoints / Operations

### {Operation 1: e.g. List Items}

```
{HTTP method} {endpoint}
```

```{language}
{SDK code}
```

### {Operation 2: e.g. Create Item}

```
{HTTP method} {endpoint}
```

```{language}
{SDK code}
```

## Request / Response Patterns

### Pagination

{How to handle pagination.}

### Error Handling

{Error format and common codes.}

## Quick Reference

| Operation | Endpoint / Method | Notes  |
| --------- | ----------------- | ------ |
| {op}      | `{endpoint}`      | {note} |

## Permissions / Scopes

| Operation | Required Permission |
| --------- | ------------------- |
| {op}      | `{permission}`      |

## Learn More

| Topic                   | How to Find                                                   |
| ----------------------- | ------------------------------------------------------------- |
| Full endpoint reference | `microsoft_docs_fetch(url="{url}")`                           |
| Permissions             | `microsoft_docs_search(query="{api} permissions {resource}")` |
| SDKs                    | `microsoft_docs_search(query="{api} SDK {language}")`         |
````

---

## Choosing a Template

| Rust crate (crates.io)                   | **Rust Crate**         | `tokio`, `reqwest`, `wgpu`, `axum`             |
| Client library, NuGet / npm / pip / gem  | SDK / Library          | Semantic Kernel, Azure SDK, MSAL, OpenAI SDK   |
| Azure resource                           | Azure Service          | Cosmos DB, Azure Functions, App Service        |
| App development framework                | Framework / Platform   | ASP.NET Core, Blazor, MAUI, Tauri              |
| REST API, protocol, specification        | API / Protocol         | Microsoft Graph, OOXML, FHIR, MCP, JSON-RPC    |

## Customization Guidelines

Templates are starting points. Customize by:

1. **Adding sections** for unique aspects of the technology.
2. **Removing sections** that don't apply.
3. **Adjusting depth** based on complexity (more concepts for complex tech).
4. **Adding reference files** for detailed content that doesn't fit in SKILL.md.
5. **Adding `sample_codes/`** for working examples beyond inline snippets.
