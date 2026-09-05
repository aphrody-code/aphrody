# `skills/` — aphrody plugin skill registry

Single source of truth for the **agent skills** bundled with the aphrody
Claude Code plugin. Each skill is a directory whose `SKILL.md` declares
its triggers, scope, and operating instructions.

Last updated: 2026-05-21 (post-purge: 34 skills, 21 agents).

## Skill format (`SKILL.md`)

Every skill **MUST** start with a YAML frontmatter block:

```yaml
---
name: my-skill                # kebab-case, matches directory name
description: One-line summary of what the skill does and when it fires.
---
```

Optional siblings:

| File / dir    | Purpose                                          |
|---------------|--------------------------------------------------|
| `references/` | Supporting docs (playbooks, catalogs, links).    |
| `scripts/`    | Helper automation (Rust / `cargo run` wrappers). |
| `evals/`      | Evaluation harness (deterministic test cases).   |

> **100 % Rust policy** (CLAUDE.md §2): no `bun` / `node` / `npm` / `tsc`
> in skills or scripts. Helper automation is Rust (`cargo run -p …`) or a
> one-shot shell/pwsh wrapper.

## Highlights

| Skill                  | Trigger                                   |
|------------------------|-------------------------------------------|
| `start`                | `/start`, "lance", "go", "exécute"        |
| `aphrody-yolo-grind`   | "yolo", "grind", parallel multi-agent     |
| `aphrody-perfect-grind`| "code en boucle", drive repo to ship-ready|
| `a2a-duel-loop`        | sustained A2A coordination duel           |
| `rust-target-check`    | parallel 3-target `cargo check`           |
| `vps-commander`        | "start the tunnel", "connect to the vps"  |
| `agent-browser`        | preferred browser automation entrypoint   |
| `docs-auto` / `context7-mcp` | library / framework / SDK docs lookup |

The full catalogue is auto-discovered from `skills/<name>/SKILL.md`; the
plugin manifest (`.claude-plugin/plugin.json`) and the plugin
[`README.md`](../README.md) carry the authoritative counts.

## Agents (`../agents/`)

Invokable sub-agents with their own toolset and system prompt
(`rust-architect`, `rust-engineer`, `cargo-auditor`, `cpp-engineer`,
`ffi-architect`, `zig-engineer`, `cross-platform-validator`,
`aphrody-cli`, `explore`, `build`, `code-review`, `yolo-prod-ready`,
`design-google-curator`, plus the infra/quality specialists). See the
plugin [`README.md`](../README.md) for the full table.
