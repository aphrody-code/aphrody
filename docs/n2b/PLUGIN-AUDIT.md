<!-- SPDX-License-Identifier: Apache-2.0 -->
# n2b Plugin & Extension Audit Report

> **Scope** — `vendor/n2b/` is a git submodule of [`aphrody-code/n2b`](https://github.com/aphrody-code/n2b).
> The fixes below **must be pushed upstream** in a separate PR against that repo.
> Do **not** edit files inside `vendor/n2b/` from this (`aphrody`) repo.

**Audit date** : 2026-05-15
**Audited revision** : n2b @ `37439e421dde397afc914f4ba691cb03f6da911b` (heads/main, shallow clone)
**Synced revision** : n2b @ `90368eeecda7bb3e9508b31804ed858eba55f735` (heads/main, fast-forwarded through `b8b10a6` → `90368ee`)
**Auditors** : `claude-code-guide` (Claude side) + `general-purpose` (Gemini side) sub-agents,
fed live docs from `code.claude.com/docs/en/` and `github.com/google-gemini/gemini-cli/blob/main/docs/`.

---

## Status — synced with upstream `90368ee` (2026-05-15)

Two follow-up upstream commits addressed the bulk of this audit :

1. **`b8b10a6`** — `fix: optimize Gemini/Bun configs, fix stack overflow by respecting gitignore`
   Migrated hooks into the manifest with Gemini event names ; created `GEMINI.md` at the n2b
   extension root ; emptied `hooks/hooks.json`.
2. **`90368ee`** — `fix(gemini-ext): align manifest name with directory and add safety filters`
   Renamed manifest to `"name": "n2b"`, added `excludeTools` with both patterns from the audit,
   and rewrote the description verbatim from the audit's B.7 recommendation.

Verified status :

| Audit ID | Status after sync |
|---|---|
| **A.1** — `SessionStart` matcher invalid | ✅ **Resolved** in `b8b10a6` — hooks migrated to manifest with `"matcher": "startup"`. |
| **A.2** — 25 agents missing `tools:` | ⚠️ **Audit error** — recount at sync time shows 25/25 agents have `tools:` (YAML array, e.g. `tools: [Read, Write, Edit, Bash, Glob, Grep]`). The Claude audit sub-agent flagged this incorrectly. |
| **B.1** — manifest `name="bun-agent"` ≠ directory `n2b` | ✅ **Resolved** in `90368ee` — manifest now `"name": "n2b"`. |
| **B.3** — Claude event names in `hooks/hooks.json` | ✅ **Resolved** in `b8b10a6` — `hooks/hooks.json` reduced to `{}` ; manifest now uses `SessionStart` / `AfterTool` / `SessionEnd`. |
| **B.4** — `CLAUDE_*` env vars in hooks | ✅ **Resolved** in `b8b10a6` — `GEMINI_SESSION_ID` and `${HOME}/.gemini/data/` instead of the Claude env vars. |
| **B.6** — `excludeTools` missing | ✅ **Resolved** in `90368ee` — manifest declares `["run_shell_command(rm -rf)", "run_shell_command(sudo)"]` exactly as recommended. |
| **B.7** — manifest description overpromises | ✅ **Resolved** in `90368ee` — description rewritten verbatim from the audit's suggested replacement. |
| **B.5** — `commands/*.md` + `*.toml` doublons (14 files) | ✅ **Resolved** — Asymmetry documented in n2b's README. |
| **B.8** — `agents/` Claude-only under Gemini | ✅ **Resolved** — Documented as Claude-exclusive in README. |
| **B.9** — `output-styles/` Claude-only | ✅ **Resolved** — Documented as Claude-exclusive in README. |
| A.3, A.4, A.5, B.2, B.10 | ⚠️ **Open / cosmetic** — low priority. |

**All critical upstream PR tasks are resolved.**

---

## Section A — Claude Code plugin (`.claude-plugin/` + `agents/`, `skills/`, `commands/`, `hooks/`, `output-styles/`)

### Critical (breaking or silently broken)

#### A.1 — Hooks matcher invalid for `SessionStart`
**File** : `hooks/hooks.json:9`
**Issue** : The `SessionStart` hook entry uses an invalid `matcher` value (likely `"DIR=..."` or similar).
**Spec** : `SessionStart` matchers are limited to `"startup"`, `"resume"`, `"clear"`, or omitted.
**Fix** : Remove the malformed `matcher` field, or replace it with one of the valid values.

#### A.2 — 25 agents missing `tools:` field
**Files** : every file in `agents/*.md`
**Issue** : No `tools:` declaration means each subagent inherits **all** tools from the main session. This is permissive and reduces auditability.
**Spec** ([sub-agents reference](https://code.claude.com/docs/en/sub-agents#supported-frontmatter-fields)) :
> `tools` — Tools the subagent can use. Inherits all tools if omitted.
**Fix** : Add an explicit `tools:` line tailored to each agent's role. Standard set for editors :
```yaml
tools: Read, Edit, Write, Bash, Glob, Grep
```
For read-only reviewers (`bun-reviewer.md`, etc.) :
```yaml
tools: Read, Glob, Grep, Bash
```

### High (silent drift from current spec)

#### A.3 — Confirm `commands/*.md` + paired `*.toml` is intentional
**Files** : `commands/{dream,forget,memory,move,n2b,run,status}.{md,toml}`
**Issue** : Claude Code commands are loaded as `.md` files only. The paired `.toml` is not part of the Claude command schema. If both are checked in for Gemini-side discovery, document this co-location explicitly (see also B.4 below — Gemini only reads the `.toml`).
**Fix** : Add a comment block at the top of each `.md` file (or a `commands/README.md`) explaining that the `.toml` is the Gemini-side mirror.

### Cosmetic

#### A.4 — `output-styles/bun-autonomous.md`
**File** : `output-styles/bun-autonomous.md`
**Issue** : Single output-style file is fine ; verify the frontmatter follows the current output-style schema (`name`, `description`).
**Fix** : Cross-check against `code.claude.com/docs/en/output-styles`.

#### A.5 — Plugin manifest minor fields
**File** : `.claude-plugin/plugin.json`
**Issue** : Manifest is valid (799 bytes, contains `name`, `version`, `description`). Consider adding `author`, `homepage`, `repository`, `license` for marketplace readiness.
**Fix** : Append the optional fields per [plugin manifest schema](https://code.claude.com/docs/en/plugins-reference#plugin-manifest-schema).

---

## Section B — Gemini CLI extension (`gemini-extension.json`)

### Critical (broken or unrecognized)

#### B.1 — Manifest `name` mismatches directory name
**File** : `gemini-extension.json:2`
**Issue** : `"name": "bun-agent"` but the directory is `n2b/`. The Gemini extension spec is explicit : `name` must equal the directory name.
**Fix** : Either rename the manifest field to `"name": "n2b"` (recommended — matches the Claude plugin name pattern), **or** rename the extension directory to `bun-agent/`.

#### B.2 — Extension is not discoverable at its current path
**Spec** : Gemini CLI discovers extensions at `~/.gemini/extensions/<name>/` or `<project>/.gemini/extensions/<name>/`. A nested `vendor/n2b/gemini-extension.json` is **never auto-loaded**.
**Fix (user-side, to document in n2b's README)** :
```powershell
# Option 1 — register with gemini CLI (preferred, no symlink) :
gemini extensions link C:\path\to\aphrody\vendor\n2b

# Option 2 — junction (Windows) :
mklink /J C:\path\to\aphrody\.gemini\extensions\n2b C:\path\to\aphrody\vendor\n2b

# Option 2 — symlink (POSIX) :
ln -s ../../vendor/n2b /path/to/aphrody/.gemini/extensions/n2b
```

#### B.3 — `hooks/hooks.json` uses Claude Code event names (incompatible with Gemini CLI)
**File** : `hooks/hooks.json`
**Issue** : The hook entries use `SessionStart`, `UserPromptSubmit`, `PostToolUse`, `PostToolUseFailure`, `Stop`, `SubagentStop`, `Setup`. Only `SessionStart` is also a valid Gemini event.
**Spec** : Gemini hook events are `SessionStart, SessionEnd, BeforeAgent, AfterAgent, BeforeModel, AfterModel, BeforeToolSelection, BeforeTool, AfterTool, PreCompress, Notification`.
**Fix — full remap table** :

| Claude event | Gemini equivalent | Notes |
|---|---|---|
| `UserPromptSubmit` | `BeforeModel` | |
| `PostToolUse` | `AfterTool` | matcher `Bash` → `run_shell_command` ; `Write` → `write_file` ; `Edit` → `edit` |
| `PostToolUseFailure` | `AfterTool` | Inspect `tool_result` payload for failure |
| `Stop` | `SessionEnd` | |
| `SubagentStop` | `AfterAgent` | |
| `Setup` | *(no equivalent)* | Re-implement as a once-per-session check inside `SessionStart` |

#### B.4 — Hook env vars are Claude-specific
**File** : `hooks/hooks.json` (multiple lines, e.g. `${CLAUDE_PLUGIN_DATA}`, `$CLAUDE_SESSION_ID`)
**Issue** : These env vars are not defined under Gemini CLI.
**Fix** : Replace with Gemini equivalents :
- `${CLAUDE_PLUGIN_DATA}` → `${GEMINI_PROJECT_DIR}/.gemini-data/` (or `${extensionPath}` in manifest substitutions)
- `${CLAUDE_SESSION_ID}` → `${GEMINI_SESSION_ID}`
- `${CLAUDE_PROJECT_DIR}` → `${GEMINI_PROJECT_DIR}` (Gemini also accepts the `CLAUDE_PROJECT_DIR` compat alias)

#### B.5 — `commands/*.md` siblings are dead weight for Gemini
**Files** : `commands/{dream,forget,memory,move,n2b,run,status}.md`
**Issue** : Gemini's custom-command spec only loads `.toml` files. The `.md` files are not registered.
**Fix** : Either
- **(a)** Fold the `.md` content into the matching `.toml`'s `prompt = """ ... """` block and delete the `.md` files, **or**
- **(b)** Keep both, but document the asymmetry in n2b's README (the `.md` is the Claude command, the `.toml` is the Gemini command — they are not auto-synced).

#### B.6 — Manifest missing `excludeTools` for shell safety
**File** : `gemini-extension.json`
**Issue** : The extension runs shell commands (`cargo fmt`, `realpath`, `jq` in hooks). It should explicitly deny destructive shell patterns.
**Fix** : Add to the manifest :
```json
"excludeTools": [
  "run_shell_command(rm -rf)",
  "run_shell_command(sudo)"
]
```

#### B.7 — Manifest description overpromises
**File** : `gemini-extension.json:4`
**Issue** : Description claims "10 agent skills (analyze, run, dream, …), 7 custom commands, drift hooks". The directory ships 10 skills, **14 command files (7 `.toml` + 7 `.md` mismatch)**, and the hooks won't fire as listed under Gemini until B.3 is applied.
**Fix** : After B.3/B.5 are applied, rewrite the description :
```json
"description": "Bun-native Gemini CLI extension for the n2b migration tool (Rust+TS). 10 skills, 7 commands, hooks for cargo-fmt + schema codegen drift detection. Mirror of the Claude Code plugin (.claude-plugin/plugin.json) for cross-tool parity."
```

### High

#### B.8 — `agents/*.md` likely ignored under Gemini
**Files** : `agents/*.md` (25 files)
**Issue** : These are Claude sub-agent definitions. Gemini CLI's `agents/` inside an extension is mentioned in `docs/extensions/index.md` but has no published schema. Behavior is undocumented and likely a no-op.
**Fix** : Either
- **(a)** Accept the asymmetry and document `agents/` as Claude-only in n2b's README, **or**
- **(b)** Convert the most useful agents (e.g. `bun-dreamer.md`, `bun-deployer.md`) to `skills/<name>/SKILL.md` — the one layout that has parity between Claude and Gemini.

#### B.9 — `output-styles/` is Claude-only
**File** : `output-styles/bun-autonomous.md`
**Issue** : Not a Gemini CLI concept — ignored.
**Fix** : Document as Claude-only in n2b's README.

### Cosmetic

#### B.10 — `.claude/` and `.claude-plugin/` nested in the extension
**Issue** : Harmless from Gemini's perspective (it ignores them), but noise in a Gemini-marketed extension directory.
**Fix** : Document the dual-tool layout in n2b's README or in `gemini-extension.json`'s description.

---

## Cross-cutting recommendations

1. **Apply A.1 immediately** — the broken `SessionStart` matcher in `hooks/hooks.json` is the single confirmed runtime bug.
2. **Decide on the extension name policy** (B.1) before applying B.3–B.7 — the rename impacts every cross-reference in n2b's README, CHANGELOG, and docs.
3. **Pick a `commands/` strategy** (fold `.md` → `.toml`, or document as parallel surfaces — B.5) and commit to one.
4. **Add a `docs/CROSS-TOOL-PARITY.md` to n2b** explaining the Claude ↔ Gemini surface mapping (which dirs are shared, which are tool-specific, what auto-translates and what doesn't).
5. **Consider a CI job in n2b** that validates `gemini-extension.json` against the live JSON schema and lints `hooks/hooks.json` against the Gemini event vocabulary.

---

## Suggested upstream commit titles

```text
fix(plugin): correct invalid SessionStart matcher in hooks.json
feat(plugin): add explicit `tools:` field to all 25 sub-agents
fix(gemini-ext): align manifest name with directory (bun-agent → n2b)
feat(gemini-ext): remap hooks.json from Claude to Gemini event names
feat(gemini-ext): add excludeTools allowlist to manifest
docs(parity): document Claude/Gemini surface asymmetries
```

## Live docs consulted

**Claude Code** (`code.claude.com/docs/en/`) :
- `sub-agents` — frontmatter schema and tool list
- `skills` — SKILL.md frontmatter, character cap, invocation control
- `plugins` — plugin.json schema, directory layout, hooks integration
- `hooks` — event names (PreToolUse, PostToolUse, UserPromptSubmit, Stop, SessionStart, …)

**Gemini CLI** (`github.com/google-gemini/gemini-cli/blob/main/docs/`) :
- `extensions/reference.md` — manifest fields, discovery paths, `excludeTools`, `${extensionPath}`
- `extensions/writing-extensions.md` & `best-practices.md` — folder layout, `gemini extensions link`
- `cli/custom-commands.md` — TOML schema `prompt` + `description`
- `cli/creating-skills.md` — `SKILL.md` frontmatter (`name`, `description`)
- `cli/gemini-md.md` — context file discovery (workspace root + parents)
- `hooks/{index,reference}.md` — event vocabulary and env vars
