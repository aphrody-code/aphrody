# `.claude/skills/` — Central Skill Registry

Single source of truth for **agent skills** consumed by Claude Code, Gemini CLI,
and the `skill` runtime (docs.rs/skill). Each skill is a directory whose
`SKILL.md` declares triggers, scope, and operating instructions.

Last updated: 2026-05-17.

## Skill format (SKILL.md)

Every skill **MUST** start with a YAML frontmatter block:

```yaml
---
name: my-skill                # kebab-case, matches directory name
description: One-line summary of what the skill does and why it exists.
when_to_use: |                # natural-language trigger conditions
  User says "X", types "/Y", or the conversation reaches state Z.
---
```

Optional siblings:

| File / dir         | Purpose                                                      |
|--------------------|--------------------------------------------------------------|
| `references/`      | Supporting docs (playbooks, catalogs, links).                |
| `scripts/`         | Helper automation (`.py`, `.ts`, `.sh`).                     |
| `evals/`           | Evaluation harness (deterministic test cases).               |

Specs:
- [`docs.rs/skill`](https://docs.rs/skill) — Rust runtime (`SkillManager`).
- [`docs.rs/agent-skills`](https://docs.rs/agent-skills) — spec validator library.
- [Vercel agent-skills](https://github.com/vercel-labs/agent-skills) — canonical catalog.

## Project skills

| Name                                         | Trigger                                  | Body                                          |
|----------------------------------------------|------------------------------------------|-----------------------------------------------|
| [`start/`](./start/SKILL.md)                 | `/start`, "lance", "go", "exécute"       | Continuous autonomous execution mode          |
| [`vps-commander/`](./vps-commander/SKILL.md) | "start the tunnel", "connect to the vps" | OVH VPS SSH-tunnel operator                   |

## Project agents (`.claude/agents/`)

Agents are **invokable sub-agents** with their own toolset and system prompt.
The Trinity Architecture decides which agent fires per task class:

| Agent              | Domain                                                  | Tools                  |
|--------------------|---------------------------------------------------------|------------------------|
| `cargo-auditor`    | Workspace audit: licensing, CVE, code quality           | Read, Grep, Glob, Bash |
| `cpp-engineer`     | C/C++ development (Google Style)                        | Read, Edit, Write, Bash, Glob, Grep |
| `ffi-architect`    | C++↔Bun FFI zero-allocation architecture                | Read, Edit, Write, Bash, Glob, Grep |
| `rust-architect`   | Cargo workspaces, FFI boundaries (Fuchsia/Windows-rs)   | Read, Edit, Write, Bash, Glob, Grep |
| `rust-engineer`    | Rust implementation (Chromium/Google Style)             | Read, Edit, Write, Bash, Glob, Grep |

## Global (user-scope) skills

Live in `~/.claude/skills/` — apply to every repo on this machine:

- `repo-hygiene-audit/` — portability + hygiene sweep, invoked via `/repo-hygiene-audit`.

## Authoring a new skill

```bash
mkdir -p .claude/skills/<name>/{references,scripts}
$EDITOR  .claude/skills/<name>/SKILL.md        # paste frontmatter template

bun run skills:discover                         # detect all SKILL.md files
bun run skills:validate                         # spec-validate every SKILL.md
```

## Pulling in upstream skill catalogs

| Catalog                                                                                          | Command                                    |
|--------------------------------------------------------------------------------------------------|--------------------------------------------|
| [`vercel-labs/agent-skills`](https://github.com/vercel-labs/agent-skills) — Vercel React/Next.js | `bun run skills:sync:vercel`               |
| `anthropics/skills` — official Claude Code skills                                                | `bun run skills:sync:claude-official`      |
| Arbitrary GitHub source                                                                          | `bun run scripts/skills-sync.ts <org>/<repo>` |

The runtime binaries (`skill`, `agent-skills`) are installed via
`cargo install --locked skill-cli agent-skills-cli` — no Node dependency.
See [`docs/cargo/SKILLS.md`](../../docs/cargo/SKILLS.md) for the full ecosystem doc.
