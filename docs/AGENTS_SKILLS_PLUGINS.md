# Antigravity, Gemini & Agy CLI: Skills, Plugins & Hooks Reference

This document serves as the canonical reference for the **Antigravity 2.0**, **Agy CLI**, and legacy **Gemini CLI** ecosystems. It documents their configuration structures, directory layouts, custom commands, hooks execution protocol, and skill discovery rules.

---

## 1. Directory Layouts & Path Specifications

### 1.1 Config & Runtime Directories

| Component | Path (Unix / macOS) | Path (Windows) | Description |
|---|---|---|---|
| **Global Config** | `~/.config/antigravity/` | `%USERPROFILE%\.config\antigravity\` | Contains main config file `config.toml` |
| **Global CLI Home** | `~/.gemini/antigravity-cli/` | `%USERPROFILE%\.gemini\antigravity-cli\` | Active Antigravity 2.0 CLI config and states |
| **Legacy CLI Home** | `~/.gemini/` | `%USERPROFILE%\.gemini\` | Active legacy Gemini CLI config and states |
| **Windows AppData** | N/A | `%LOCALAPPDATA%\Antigravity\` | Windows installation directory for the `agy` binary |

### 1.2 Skills Resolution Directories

For both project-specific and global contexts, skills are discovered in the following locations (searched in order):

1. **Project Skills (Local)**:
   - `.agents/skills/*.md`
   - `.antigravity/skills/*.md` (Natively supported by aphrody skills)
2. **Global Skills (Home)**:
   - `~/.gemini/antigravity-cli/skills/` (Antigravity 2.0 CLI global scope)
   - `~/.gemini/antigravity/skills/` (Antigravity App desktop global scope)
   - `~/.gemini/skills/` (Legacy Gemini CLI global scope)

---

## 2. Command Line Interface (`agy`)

The `agy` binary (located in `%LOCALAPPDATA%\Antigravity\bin\agy.exe` on Windows or `~/.local/bin/agy` on Unix) replaces the deprecated `gemini` CLI. The transition deadline is **June 18, 2026**.

### 2.1 CLI Flags

| Flag | Argument | Description |
|---|---|---|
| `-p` | `"prompt"` | Headless completion mode. Completes the prompt one-shot and exits. |
| `--output-format`| `json` | Structured output format for stdout redirection or scripting. |
| `-m` | `<model_id>` | Selects a specific model (e.g. `gemini-3.5-flash`, `gemini-3.1-pro`, `claude-sonnet`, `claude-opus`). |
| `inspect` | N/A | Prints active project context, loaded skills, plugins, hooks, and active MCP servers. |
| `plugin import` | `gemini` | Command to migrate legacy extensions (`gemini-extension.json`) to modern plugins. |

### 2.2 TUI Slash Commands

When running in interactive (TUI) mode, the following slash-commands are available:

- `/help`: Show command assistance.
- `/context`: Prints the current project context parsed by the agent.
- `/usage`: Shows token count, model costs, and billing tier usage (e.g. AI Ultra tier).
- `/export`: Exports the current terminal session to the Antigravity 2.0 Desktop GUI.
- `/model <model_id>`: Dynamically switch the model for the active session.
- `/agent <name> "<task>"`: Spawns an asynchronous sub-agent to run in the background.
- `/logout`: Clears authentication tokens.

---

## 3. Plugins & Extensions

Plugins are self-contained extensions configured via a manifest file at their root directory.

### 3.1 Manifest: `gemini-extension.json` / `plugin.json`

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "Custom commands and MCP integrations",
  "mcpServers": {
    "my-server": {
      "command": "node",
      "args": ["${extensionPath}/bin/server.js"],
      "cwd": "${extensionPath}"
    }
  },
  "contextFileName": "AGENTS.md",
  "excludeTools": ["run_shell_command(rm -rf)"],
  "settings": [
    {
      "name": "API_KEY",
      "description": "Service authentication key",
      "envVar": "MY_SERVICE_API_KEY",
      "sensitive": true
    }
  ]
}
```

#### Variables of Substitution:
- `${extensionPath}`: Absolute path to the plugin's folder.
- `${workspacePath}`: Absolute path to the active user project.
- `${/}`: OS-specific path separator (`/` on Unix, `\` on Windows).

### 3.2 Custom Commands (`commands/`)

Plugins expose slash-commands by defining TOML files inside their `commands/` subdirectory.
For example, a file `commands/deploy.toml` registers the command `/deploy`. Nested subdirectories map to namespaces (e.g. `commands/gcs/sync.toml` registers `/gcs:sync`).

---

## 4. Hooks Protocol (Execution Lifecycle)

Hooks are lifecycle interceptors defined in the plugin's `hooks/hooks.json` file. They allow running scripts or binaries during specific transitions of the agent session.

### 4.1 Supported Lifecycle Events

- `SessionStart`: Fires when the CLI session initializes.
- `SessionEnd`: Fires before the CLI exits.
- `BeforeAgent`: Fires before dispatching a request to the agent engine.
- `AfterAgent`: Fires immediately after the agent engine returns a response (pre-output rendering).
- `BeforeModel`: Fires before sending a payload to the LLM model.
- `AfterModel`: Fires after receiving raw text from the LLM model.
- `BeforeToolSelection`: Fires before the agent selects a tool to call.
- `BeforeTool`: Fires immediately before tool execution.
- `AfterTool`: Fires immediately after tool execution.
- `Notification`: Fires on background notifications.
- `PreCompress`: Fires before context compaction/compression occurs.

### 4.2 Hooks Protocol Interface

Each hook communicates via standard I/O streams (**stdin** and **stdout**).

#### Input Payload (stdin JSON):
```json
{
  "session_id": "5e3cca80-36c1-4b18-9d3f-59dc4c41ea06",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/workspace",
  "hook_event_name": "AfterAgent",
  "timestamp": "2026-05-23T10:53:32Z",
  "prompt": "Current user prompt",
  "prompt_response": "Response returned by agent",
  "stop_hook_active": false
}
```

#### Output Payload (stdout JSON):
```json
{
  "systemMessage": "Optional text to inject into system prompt",
  "suppressOutput": false,
  "continue": true,
  "stopReason": "Goal reached",
  "decision": "deny",
  "reason": "Force loop execution because tests are failing",
  "hookSpecificOutput": {}
}
```

#### Decision Control:
- `decision: "allow"`: Standard execution, proceeds to output rendering or next phase.
- `decision: "deny"`: Rejects the current state. When returned by `AfterAgent`, **it forces the agent engine to run another iteration loop**, injecting the `reason` field as a system directive prompt. This is the primary driver of autonomous coding loops (e.g. `aphrody agy-loop`).

---

## 5. Skills Subsystem & Compatibility Matrix

Unlike Claude Code and Open-Design, which auto-trigger based on natural language triggers, **Antigravity and Gemini CLI use tool-based skill activation**.

### 5.1 Invocation Pattern
1. The agent's system prompt list includes all discovered skills (names and descriptions).
2. When the model determines it needs a skill, it invokes the tool `activate_skill(name: string)`.
3. The Loader fetches the skill's `SKILL.md` body, wraps it in `<activated_skill>` tags, and returns it as the tool output.
4. The model continues reasoning with the newly-loaded instructions in its context.

### 5.2 Ecosystem Feature Comparison

| Feature | Gemini CLI / Antigravity | Open-Design | Claude Code | Vercel Agent Skills |
|---|---|---|---|---|
| **Frontmatter keys** | `name`, `description` | `name`, `description`, `triggers`, `od` | `name`, `description`, `when_to_use` | `name`, `description`, `metadata`, `license` |
| **Triggers** | None | Natural language triggers | Natural language triggers | None |
| **Invocation** | Tool call (`activate_skill`) | Direct narrative injection | Direct narrative injection | Direct narrative injection |
| **Config formats** | `hooks/hooks.json` | `agent.json` | `CLAUDE.md` | `plugin.json` |
| **Local Path** | `.agents/skills/` | `.agents/skills/` | `.claude/skills/` | `.agents/skills/` |
| **Global Path** | `~/.gemini/` | `~/.openclaw/` | `~/.claude/` | `~/.agents/` |
