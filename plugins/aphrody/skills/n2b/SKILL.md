---
name: n2b
description: Scan and migrate a Node.js codebase to Bun using n2b, including API mappings, Bun Shell ($), shebang updates, and package config.
when_to_use: User asks to migrate a Node project to Bun, mentions "n2b", "Node to Bun", or wants to analyze node compatibility of scripts.
version: "1.1.0"
---

<!-- SPDX-License-Identifier: Apache-2.0 -->

# n2b — Node to Bun Migration & Scaffolding Tool

The `n2b` skill provides comprehensive rules, syntax mappings, and automation commands to migrate JavaScript/TypeScript codebases from Node.js to Bun. It also details the usage of the native `n2b` tool.

---

## 1. Mappings: Node.js to Bun Native APIs

When refactoring code to run on Bun, prefer Bun-native APIs over Node.js polyfills for maximum performance and efficiency:

### File I/O
* **Read file as text (Async):**
  - *Node:* `const data = await fs.promises.readFile(path, 'utf8')`
  - *Bun:* `const data = await Bun.file(path).text()`
* **Write file (Async):**
  - *Node:* `await fs.promises.writeFile(path, content)`
  - *Bun:* `await Bun.write(path, content)`
* **Check if file exists:**
  - *Node:* `await fs.promises.access(path)`
  - *Bun:* `await Bun.file(path).exists()`

### Subprocess Execution
* **Spawn a process (Async):**
  - *Node:* `const proc = child_process.spawn('cmd', ['arg1'])`
  - *Bun:* `const proc = Bun.spawn(['cmd', 'arg1'])`
* **Run command and capture stdout (Shell):**
  - *Node:* `const stdout = child_process.execSync('ls -la').toString()`
  - *Bun:* `const stdout = await Bun.$`ls -la` .text()` (Bun Shell is fully cross-platform and secure).

### HTTP Server
* **Serve HTTP requests:**
  - *Node:* `http.createServer((req, res) => { ... }).listen(3000)`
  - *Bun:* 
    ```ts
    Bun.serve({
      port: 3000,
      fetch(request) {
        return new Response("Hello World");
      },
    });
    ```

### Environmental Variables
* **Dotenv loading:**
  - *Node:* Requires importing `dotenv` and calling `dotenv.config()`.
  - *Bun:* Automatically loads `.env` files on startup. Accessible via `process.env` or `Bun.env`.

---

## 2. Codebase Conventions & Quality Rules

To maintain the project's strict `feedback_bun_only` standard:
1. **Require vs Import:** Do not use CommonJS `require()` of node built-ins. Always use ES module `import` syntax.
2. **Built-in imports:** Use `node:` prefixes for Node compatibility imports (e.g., `import fs from "node:fs"` instead of `import fs from "fs"`).
3. **Shebangs:** Replace `#!/usr/bin/env node` with `#!/usr/bin/env bun` in executable scripts.
4. **Current Directory:** Replace `__dirname` and `__filename` with `import.meta.dirname` (Bun >= 1.1) or use the `n2b-shims` helper:
   ```ts
   import { dirOf } from "n2b-shims";
   const __dirname = dirOf(import.meta);
   ```
5. **Engines declaration:** Set `"engines": { "bun": ">=1.3.0" }` in `package.json` to enforce Bun at install-time.

---

## 3. n2b CLI Usage Reference

The `n2b` tool can be invoked through `aphrody n2b [args...]` or directly using the local `n2b` binary.

### Scan and Audit
* **Scan for Node-specific patterns:**
  ```bash
  aphrody n2b .
  ```
* **List all active migration rules:**
  ```bash
  aphrody n2b rules
  ```
* **Scan GitHub issues/PRs for Node-to-Bun transition issues:**
  ```bash
  aphrody n2b audit
  ```

### Migration & Auto-Fixing
* **Apply safe automatic refactorings (no side-effects):**
  ```bash
  aphrody n2b . --fix
  ```
* **Apply aggressive corrections (more invasive code changes):**
  ```bash
  aphrody n2b . --aggressive
  ```
* **Full migration (autofix + locks deletion + workspace config + bun install):**
  ```bash
  aphrody n2b . --migrate
  ```

### Project Scaffolding
* **Scaffold a standard Bun app (CLI, TUI, GUI, standalone):**
  ```bash
  aphrody n2b app
  ```
* **Scaffold a Bun + Win32 integration project (with Rust FFI or inline C):**
  ```bash
  aphrody n2b win32
  ```
* **Scaffold a Bun + Linux systems project:**
  ```bash
  aphrody n2b linux
  ```
* **Scaffold a Rust -> WASM -> Bun pipeline:**
  ```bash
  aphrody n2b wasm
  ```

### Auxiliary Commands
* **Create/apply patches to node_modules:**
  ```bash
  aphrody n2b patch <package_name>
  ```
* **Generate `llms.txt` and `llms-full.txt` from a site/URL:**
  ```bash
  aphrody n2b llmstxt <url>
  ```
