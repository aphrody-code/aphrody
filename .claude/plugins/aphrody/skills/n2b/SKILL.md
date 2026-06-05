---
name: n2b
description: Scan and migrate a Node.js codebase to Bun using n2b.
when_to_use: User asks to migrate a Node project to Bun, mentions "n2b", "Node to Bun", or wants to analyze node compatibility of scripts.
version: "1.0.0"
---

# n2b — Node to Bun Migration & Scaffolding Tool

The `n2b` tool simplifies migrating projects from Node.js to Bun and scaffolding low-level systems projects.

## Key Subcommands

1. **Migration / Scanning:**
   - Run `aphrody n2b [path]` to scan the workspace (default: `.`) for Node-specific patterns.
   - Use `--fix` to apply safe automatic corrections (e.g., rewriting `require` to `import` where safe, or prefixing node built-in imports with `node:`).
   - Use `--migrate` for full migration (removes lockfiles, sets up workspace configuration, runs `bun install`).

2. **Scaffolding:**
   - `aphrody n2b app`: Scaffold CLI, TUI, GUI, or standalone executables.
   - `aphrody n2b win32`: Scaffold low-level Win32 projects with Bun FFI and Rust integration.
   - `aphrody n2b linux`: Scaffold low-level Linux systems projects.
   - `aphrody n2b wasm`: Rust -> WASM -> Bun workflow orchestration.

3. **Analysis:**
   - `aphrody n2b rules`: List all active lint/migration rules.
   - `aphrody n2b audit`: Scan GitHub issues/PRs for Node-to-Bun transition issues.
   - `aphrody n2b llmstxt`: Crawl any site/URL and generate `llms.txt` and `llms-full.txt`.
