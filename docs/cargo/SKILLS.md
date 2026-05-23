<!-- SPDX-License-Identifier: Apache-2.0 -->
# Agent Skills — Ecosystem & Centralization

> Last updated: 2026-05-22.
> Single source of truth for **agent skills** consumed by Claude Code,
> Gemini CLI, and the `skill` Rust runtime.

## 1. What is a "skill"

A **skill** is a versioned, frontmatter-tagged Markdown file (`SKILL.md`) plus
optional siblings (`references/`, `scripts/`, `evals/`) that tells an LLM agent
*when* and *how* to perform a specific task. Skills are the unit of reusable
instruction across agent frameworks (Claude Code, Cursor, Gemini, OpenCode…)
and are standardized by:

| Spec / Tooling                                                | Role             |
|---------------------------------------------------------------|------------------|
| [Vercel Agent Skills](https://github.com/vercel-labs/agent-skills) — `agentskills.io` | Canonical catalog format (Vercel-curated) |
| [`docs.rs/skill`](https://docs.rs/skill)                      | Rust library (`SkillManager` runtime)     |
| [`docs.rs/agent-skills`](https://docs.rs/agent-skills)        | Rust library (spec validator)             |
| [`crates.io/skill-cli`](https://crates.io/crates/skill-cli)   | Rust binary runtime (install/run/list)    |
| [`crates.io/agent-skills-cli`](https://crates.io/crates/agent-skills-cli) | Rust binary spec validator      |

## 2. SKILL.md format

Every `SKILL.md` in this workspace MUST begin with this YAML frontmatter:

```yaml
---
name: my-skill                # kebab-case, must match the directory name
description: One-line summary of what the skill does and why it exists.
when_to_use: |                # natural-language activation conditions
  User types "/X", says "Y", or the conversation reaches state Z.
---
```

Allowed siblings (all optional):

```
.claude/skills/<name>/
├── SKILL.md          ← required: frontmatter + instruction body
├── references/       ← supporting docs (playbooks, link catalogs)
├── scripts/          ← bun, python, ps1, sh — never node
└── evals/            ← deterministic test cases (graded by an LLM judge)
```

## 3. Centralized inventory

### 3.1 Project skills (`/.claude/skills/`)

| Skill                         | Trigger                                           | Description                                              |
|-------------------------------|---------------------------------------------------|----------------------------------------------------------|
| [`start`](../../.claude/plugins/aphrody/skills/start/SKILL.md)                 | `/start`, "lance", "go"            | Continuous autonomous execution mode (drives PLAN.md)    |
| [`vps-commander`](../../.claude/plugins/aphrody/skills/vps-commander/SKILL.md) | "start the tunnel"                 | OVH VPS SSH-tunnel operator (chrome/postgres/bun/SOCKS5) |
| [`google-design`](../../.claude/plugins/aphrody/skills/google-design/SKILL.md) | `/google-design`, any M3 / Gemini / Google Sans / token / color / shape / motion / adaptive question | Canonical Google/Material 3 authority — grounds answers in `docs/design/` + `crates/m3-tokens` + `mui-rs`; reading list = [`notebook-google-design-corpus.md`](../design/notebook-google-design-corpus.md) |

### 3.2 Project agents (`/.claude/agents/`)

Agents are spawnable sub-agents (separate context, tool whitelist, model).
The Trinity Architecture routes per task class:

| Agent              | Domain                                                  | Tools                                    |
|--------------------|---------------------------------------------------------|------------------------------------------|
| `cargo-auditor`    | Workspace audit: licensing, CVE, code quality           | `Read, Grep, Glob, Bash`                 |
| `cpp-engineer`     | C/C++ development (Google Style)                        | `Read, Edit, Write, Bash, Glob, Grep`    |
| `ffi-architect`    | C++↔Bun FFI zero-allocation architecture                | `Read, Edit, Write, Bash, Glob, Grep`    |
| `rust-architect`   | Cargo workspaces, FFI boundaries (Fuchsia/Windows-rs)   | `Read, Edit, Write, Bash, Glob, Grep`    |
| `rust-engineer`    | Rust implementation (Chromium/Google Style)             | `Read, Edit, Write, Bash, Glob, Grep`    |
| `google-design-researcher` | Google-sources-only design reader (m3.material.io / design.google / developer.android.com / fonts.google.com) — distilled, attributed facts; refuses non-Google systems as authority | `Read, Grep, Glob, universal_web_fetch, docs_auto_search, WebFetch` |

### 3.3 Global user-scope skills (`~/.claude/skills/`)

| Skill                  | Trigger                  | Scope                                                  |
|------------------------|--------------------------|--------------------------------------------------------|
| `repo-hygiene-audit`   | `/repo-hygiene-audit`    | Portability + hygiene sweep, all repos on this machine |

### 3.4 Installed Claude Code plugins (active skill surface)

Plugins each contribute a skill bundle. The active session sees the union of
project + global plugins. Below: live inventory captured 2026-05-17.

#### `feature-dev@claude-code-plugins`
- `feature-dev` — guided feature development with codebase understanding.

#### `code-review@claude-plugins-official`
- `code-review` — code-review a pull request.

#### `frontend-design@claude-plugins-official`
- `frontend-design` — production-grade frontend interfaces, anti-AI-aesthetic.

#### `rust-analyzer-lsp` / `typescript-lsp` / `csharp-lsp` (LSP-only)
No skills — these wire LSP servers into the harness.

#### `winclean@winclean-local` (25 skills — large catalog)
| Skill                                | Purpose                                           |
|--------------------------------------|---------------------------------------------------|
| `build`                              | Build the WinClean monorepo (Turborepo)           |
| `debloat`                            | Purge telemetry + bloatware via payload scripts   |
| `profile`                            | Generate dev environment profile                  |
| `scan`                               | Ultra-fast streaming scan of Windows install      |
| `cartographer`                       | Map codebases via parallel subagents              |
| `cpp-pro`                            | Modern C++ idioms (RAII, smart ptrs, STL)         |
| `csharp-aot`                         | .NET 10 NativeAOT zero-alloc best practices       |
| `csharp-async`                       | C# async best practices                           |
| `deploy-to-vercel`                   | Deploy apps to Vercel                             |
| `docs`                               | Unified intel (MS Learn + Context7)               |
| `dpapi-timing-audit`                 | Side-channel audit on DPAPI/CryptUnprotectData    |
| `pe-symbol-recovery`                 | Recover symbols in stripped PE/.NET binaries      |
| `platform-design`                    | 300+ HIG / MD3 / WCAG rules                       |
| `shadcn-ui`                          | Build shadcn/ui components                        |
| `python-performance-optimization`    | cProfile + memory profilers + perf patterns       |
| `skill-vetter`                       | Security-first vet of OpenClaw skills             |
| `uv-package-manager`                 | Modern Python deps via `uv`                       |
| `vercel-cli-with-tokens`             | Vercel CLI w/ token auth                          |
| `vercel-composition-patterns`        | React composition patterns at scale               |
| `vercel-react-best-practices`        | React/Next.js perf rules (Vercel Eng)             |
| `vercel-react-native-skills`         | React Native / Expo best practices                |
| `vercel-react-view-transitions`      | React View Transitions API                        |
| `ui-skills`                          | Cross-UI coherence constraints                    |
| `web-design-guidelines`              | Web Interface Guidelines audit                    |
| `windows-payload-yara`               | YARA rules for Windows payloads/persistence       |

#### Harness built-ins (always available)
`update-config`, `keybindings-help`, `simplify`, `fewer-permission-prompts`,
`loop`, `schedule`, `claude-api`, `implementing-jsc-classes-cpp`,
`implementing-jsc-classes-zig`, `javascriptcore-garbage-collector`,
`slowest-tests`, `writing-bundler-tests`, `writing-dev-server-tests`,
`zig-system-calls`, `init`, `review`, `security-review`.

> Total active skill surface in a typical session: **50+ skills**. Each is
> activated by name (`/skill-name`) or auto-triggered by its `when_to_use`
> frontmatter when the conversation matches.

## 4. Workflow

### 4.1 Authoring a new skill

```bash
mkdir -p .claude/skills/<name>/{references,scripts}
$EDITOR  .claude/skills/<name>/SKILL.md     # paste frontmatter template (§2)

bun run skills:discover                      # detect all SKILL.md files
bun run skills:validate                      # spec-validate every SKILL.md
```

### 4.2 Pulling in an upstream catalog

```bash
bun run skills:sync:vercel             # vercel-labs/agent-skills
bun run skills:sync:claude-official    # anthropics/skills
bun run scripts/skills-sync.ts <org>/<repo>[#<ref>]   # arbitrary repo
```

The sync script (`scripts/skills-sync.ts`):
1. Shallow-clones the source repo into a tmp dir.
2. Walks for every `SKILL.md`.
3. Reads its `name:` frontmatter (fallback: directory basename).
4. Copies the containing folder into `.claude/skills/<name>/`.
5. Skips skills that already exist locally (pass `--force` to overwrite).

### 4.3 Runtime use

The `skill` library is wired into `[workspace.dependencies]`:

```toml
skill = { version = "0.8", default-features = false, features = ["network"] }
```

Crates that need runtime skill discovery can depend on it:

```rust
use skill::manager::SkillManager;

let mgr = SkillManager::builder().build();
let installed = mgr.list_installed(&Default::default()).await?;
```

The CLI binaries are installed once via cargo:

```bash
cargo install --locked skill-cli agent-skills-cli
```

After install:
- `skill list` — list installed skills (per-scope).
- `skill discover <dir>` — list `SKILL.md` found under `<dir>`.
- `skill install <org>/<repo>` — fetch + install an external skill.
- `agent-skills validate <dir>` — spec-check every `SKILL.md` under `<dir>`.

## 5. Conventions

1. **Kebab-case** skill names (matches `SKILL.md` frontmatter `name:`).
2. **One H1** per `SKILL.md` body — the title becomes the skill's public name.
3. **No node** — helper scripts use bun, python, or pwsh; never plain `node`.
4. **References under `references/`** — keep raw playbooks/catalogs there, not
   in the main `SKILL.md` body, to keep the activation prompt short.
5. **No machine-specific paths** — use `${HOME}`, `$LOCALAPPDATA`, env vars,
   or document the variable to define.
6. **Apache-2.0 compatible** — any text that's copied into a skill is treated
   as workspace-licensed (see `LICENSE` at repo root).

## 6. Validation gate

`skills` are part of the Google Mode Golden Gate (cf. `docs/cargo/GOOGLE_MODE.md`):

```bash
bun run skills:validate         # must exit 0 — frontmatter + spec compliance
```

CI integration is tracked in `docs/PLAN.md` (P13 — Skill ecosystem hardening).
