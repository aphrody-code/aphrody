<!-- SPDX-License-Identifier: Apache-2.0 -->

# aphrody-terminal — LLM-first terminal specification

> **One-line positioning**: the terminal designed for sub-agents, skills,
> hooks, MCP servers and Ink/React TUIs — JSON output everywhere, markdown
> rendered inline, JSON-config full, WASM-native + M3-themed.

## What this terminal is NOT

- **Not a Windows Terminal clone.** Tabs/panes/profiles exist only because
  LLM tooling needs them (one pane per sub-agent stream, one pane per MCP
  server status, etc.) — not as a generic productivity feature.
- **Not a wterm port.** `vercel-labs/wterm` is the WASM-emulator API
  reference; we replace it with pure Rust to add the LLM-first surface.
- **Not a Warp-style "AI in your terminal".** The terminal is *for* LLMs
  running underneath (Claude Code, Gemini CLI, codex, etc.) and *for*
  humans collaborating with them — not a vertical AI chatbox.

## What this terminal IS

A WASM-native, M3-themed terminal whose every design decision answers one
question: **does this make life easier for the LLM running inside it?**

### Five pillars

1. **JSON output on every channel.** Every command exposes `--json`. The
   terminal frames non-JSON output into JSON envelopes too (stdout/stderr
   chunks, exit codes, timing, environment). Sub-agents can consume the
   terminal's session log without re-parsing ANSI.
2. **Markdown rendered inline.** When the underlying program emits
   markdown (`# Heading`, ` ```rust `, `- list`), the WASM renderer
   detects and renders it natively: headings, fenced code blocks with
   syntax highlight via `syntect`, lists, links, images. Toggleable via
   `aphrody-md` ANSI extension OSC sequence.
3. **JSON config full.** No YAML, no TOML for terminal config. One
   `~/.aphrody/terminal.json` with strict schema. Compatible with the
   patterns of `claude.json`, `settings.json`, `mcp.json`, `.gemini/`,
   so an LLM that knows one knows them all.
4. **Sub-agent + MCP + hooks + skills as first-class concepts.** The
   terminal exposes panes/regions for:
   - Live sub-agent task tree (one row per task, status + last log)
   - MCP server status bus (one row per server, last RPC + state)
   - Hook firing log (one row per hook event)
   - Active skill surface (one row per loaded skill, last invocation)
5. **Ink/React TUI compatibility.** Claude Code and Gemini CLI both
   render via Ink (React TUI). The VT must support: alternate screen
   buffer (`\e[?1049h`), cursor save/restore (`\e[s/u`), bracketed paste
   (`\e[?2004h`), mouse SGR (`\e[?1000h..1006h`), focus in/out
   (`\e[?1004h`), 24-bit true color SGR (`\e[38;2;r;g;b`,
   `\e[48;2;r;g;b`), 256-color (`\e[38;5;n`), DECSTBM scroll regions
   (`\e[1;24r`), insert/delete line (`\e[L`/`\e[M`), erase character
   (`\e[X`). Without these, Ink renders garbled.

## Crate stack

```
aphrody-terminal-vt          (no_std, pure Rust)
  └─ vte parser + ScreenBuffer + ALL Ink-essential CSI/SGR/DCS
aphrody-terminal-wasm        (wasm32-unknown-unknown)
  └─ DOM renderer + M3 colors + keyboard + mouse + bracketed paste
     + markdown overlay layer + JSON inspect panel
aphrody-terminal-backend     (native)
  └─ portable-pty (ConPTY/openpty) + tokio-tungstenite WS server
     + JSON resize/data protocol
aphrody-terminal-llm         (native + wasm)
  └─ Sub-agent stream multiplexer
  └─ MCP server status event bus (poll mcp.json servers, surface state)
  └─ Hook event surface (subscribe to hook firings, render)
  └─ Skill activation slot (loaded skill registry, last invocation)
aphrody-terminal-markdown    (no_std capable)
  └─ comrak CommonMark + syntect highlighter
  └─ OSC sequence detector: `\e]aphrody-md;...\a` enters markdown mode
aphrody-terminal-json-out    (no_std)
  └─ Frame stdout/stderr chunks into JSONL envelopes
  └─ Detect application-emitted JSON and pass through unmodified
aphrody-terminal-config      (native)
  └─ ~/.aphrody/terminal.json strict schema (serde + jsonschema)
  └─ Compat shims: import from settings.json, claude.json, mcp.json
aphrody-terminal-browser     (native + wasm)
  └─ Bridge: terminal LLM event bus <-> bxc (in-process) / agent-browser (RPC)
  └─ Native LLM <-> DOM automation: nav, eval JS, query selectors, screenshot,
     extract structured data, intercept requests, replay sessions
  └─ Surfaces a browser pane in the terminal (mini-viewport + DOM tree + console)
```

## JSON config schema (v1, normative)

```jsonc
{
  "$schema": "https://aphrody.dev/schemas/terminal/v1.json",
  "version": 1,
  "appearance": {
    "theme": "m3-dark-tonal",          // m3-{dark,light}-{tonal,vibrant,expressive}
    "scheme_seed": "#1A73E8",          // generates full M3 palette
    "font_family": "google-sans-flex",
    "font_size_px": 14,
    "line_height": 1.4,
    "cursor": "block-blink"            // block|underline|bar × blink|steady
  },
  "shell": {
    "default": "$SHELL",               // resolved at runtime
    "argv": ["-l"],
    "env": { "TERM": "aphrody-256color" }
  },
  "llm": {
    "sub_agent_pane": true,
    "mcp_status_pane": true,
    "hook_event_pane": true,
    "skill_pane": true,
    "json_output_default": true,
    "markdown_inline": true,
    "markdown_code_theme": "github-dark"
  },
  "integrations": {
    "claude_code": { "settings_path": "~/.claude/settings.json" },
    "gemini_cli":  { "config_path": "~/.gemini/" },
    "mcp":         { "config_path": "~/.aphrody/mcp.json" }
  },
  "keybindings": [
    { "id": "command-palette",   "binding": "ctrl+shift+p" },
    { "id": "toggle-sub-agents", "binding": "ctrl+shift+a" },
    { "id": "toggle-mcp",        "binding": "ctrl+shift+m" },
    { "id": "toggle-hooks",      "binding": "ctrl+shift+h" },
    { "id": "toggle-skills",     "binding": "ctrl+shift+s" },
    { "id": "toggle-markdown",   "binding": "ctrl+shift+d" },
    { "id": "json-export-session", "binding": "ctrl+shift+j" }
  ]
}
```

## Ink/React TUI compatibility checklist

These must work for Claude Code + Gemini CLI to render correctly. They
form the `aphrody-terminal-vt` acceptance criteria.

| Sequence | Name | Mandatory |
|---|---|---|
| `\e[?1049h/l`            | Alternate screen buffer enter/leave              | yes |
| `\e[?25h/l`              | Show/hide cursor                                  | yes |
| `\e[?2004h/l`            | Bracketed paste mode                              | yes |
| `\e[?1000;1002;1003;1006h/l` | Mouse reporting (any-event + SGR)            | yes |
| `\e[?1004h/l`            | Focus in/out events                               | yes |
| `\e[s` / `\e[u`          | Cursor save / restore (SCO)                       | yes |
| `\e7` / `\e8`            | Cursor save / restore (DEC)                       | yes |
| `\e[<top>;<bot>r`        | DECSTBM scroll region                             | yes |
| `\e[<n>S` / `\e[<n>T`    | Scroll up / down                                  | yes |
| `\e[<n>L` / `\e[<n>M`    | Insert / delete line                              | yes |
| `\e[<n>@` / `\e[<n>P`    | Insert / delete character                         | yes |
| `\e[<n>X`                | Erase character                                   | yes |
| `\e[<r>;<c>H`            | Cursor position (CUP)                             | yes |
| `\e[<n>A..D`             | Cursor up/down/right/left                         | yes |
| `\e[<n>G` / `\e[<n>d`    | Horizontal / vertical position                    | yes |
| `\e[<n>m` SGR full       | Bold/italic/underline/inverse/strike/dim          | yes |
| `\e[38;2;r;g;b m`        | 24-bit RGB foreground                             | yes |
| `\e[48;2;r;g;b m`        | 24-bit RGB background                             | yes |
| `\e[38;5;n m`            | 256-color indexed foreground                      | yes |
| `\e[48;5;n m`            | 256-color indexed background                      | yes |
| `\e[<n>J` / `\e[<n>K`    | Erase display / line                              | yes |
| `\e]0;TITLE\a`           | OSC 0 set title                                   | yes |
| `\e]52;c;BASE64\a`       | OSC 52 clipboard read/write                       | yes |
| `\eP` ... `\e\`          | DCS string passthrough (sixel/kitty optional)     | optional |

## LLM-extension ANSI sequences (aphrody-specific)

We reserve a single OSC namespace prefix `aphrody-*` for LLM-aware
extensions. All optional, all detected, all gracefully ignored by
non-aphrody terminals.

| Sequence | Meaning |
|---|---|
| `\e]aphrody-md;<base64-markdown>\a`                  | Render markdown block inline |
| `\e]aphrody-json;<base64-json>\a`                    | Surface JSON in inspect panel |
| `\e]aphrody-sub-agent;<id>;<status>;<text>\a`        | Sub-agent status update |
| `\e]aphrody-mcp;<server>;<state>;<rpc>\a`            | MCP server activity |
| `\e]aphrody-hook;<event>;<payload>\a`                | Hook firing log entry |
| `\e]aphrody-skill;<name>;<phase>;<payload>\a`        | Skill invocation log |
| `\e]aphrody-task;<id>;<status>;<subject>\a`          | Task tree update |
| `\e]aphrody-browser-nav;<url>\a`                     | Navigate active browser to URL |
| `\e]aphrody-browser-eval;<base64-js>\a`              | Eval JS in browser, response via JSON pane |
| `\e]aphrody-browser-dom;<base64-selector>\a`         | Query DOM (CSS selector), surface result tree |
| `\e]aphrody-browser-screenshot;<area>\a`             | Capture viewport / element / full-page, render inline |
| `\e]aphrody-browser-intercept;<base64-rule>\a`       | Install request interception rule |
| `\e]aphrody-browser-extract;<base64-schema>\a`       | Structured extraction (schema-driven, returns JSON) |
| `\e]aphrody-browser-record;<id>;<state>\a`           | Start/stop session recording for replay |

## Architectural invariants

1. **No JS in the core path.** TS only allowed in `packages/` for non-core
   helpers; the core renderer pipeline is pure Rust + WASM.
2. **No `unsafe` outside FFI boundaries.** `#![deny(unsafe_code)]` on
   every crate except where `wasm-bindgen` requires it.
3. **JSON config is the only config.** No YAML, no TOML, no INI for the
   terminal user-facing config.
4. **Apache-2.0 SPDX header line 1** of every file.
5. **Linux is target #1.** If a feature can't ship on Linux, it doesn't
   ship.
6. **No emoji in source or docs.** (CLAUDE.md §6 invariant.)

## Browser automation extensions (LLM ↔ DOM, native)

The terminal exposes a **browser pane** driven by two pluggable backends:

| Backend | Transport | Mode | Use case |
|---|---|---|---|
| `bxc` (aphrody-code/bxc @ aphrody) | In-process via `crates/bxc-runtime` | Lightpanda (Linux/Mac) or curl-impersonate (HTTP) | Fast scrape, static + light JS, no GPU needed |
| `agent-browser` (vercel-labs) | RPC (stdio JSON-RPC) | Full Chromium via CDP | Real SPA, WebGPU, video, complex auth flows |
| `edge` (built-in Win fallback) | spawn msedge `--headless=new --dump-dom` | DOM snapshot only | When neither above is installed |

**Selection policy** — `terminal.json` `llm.browser.preferred` chooses
default. The terminal probes availability at startup and surfaces the
chosen backend in the browser pane header. Sub-agents emit
`\e]aphrody-browser-*\a` sequences; the LLM bridge dispatches to the
active backend.

**Native LLM-DOM round-trip** (sub-second on bxc, < 3 s on agent-browser):

```
LLM sub-agent          aphrody-terminal-llm        aphrody-terminal-browser
     │                          │                              │
     │ "extract pricing table"  │                              │
     ├─────────────────────────►│                              │
     │                          │ \e]aphrody-browser-extract;  │
     │                          │  <schema-base64>\a           │
     │                          ├──────────────────────────────►│
     │                          │                              │ bxc.fetch(url)
     │                          │                              │ + schema-driven
     │                          │                              │   extraction
     │                          │     {rows: [...], meta:{...}}│
     │                          │◄──────────────────────────────│
     │       JSON envelope      │                              │
     │◄─────────────────────────│                              │
```

## Reference upstreams (read-only)

- `C:/worktree/wterm/` — Apache-2.0, TS+Zig WASM. API surface reference.
- `C:/worktree/terminal/` — MIT, C++. Buffer/Renderer/AtlasEngine/
  ConPTY/profiles.schema.json algorithmic reference.
- `C:/worktree/gemini-cli/` — Ink + React TUI. Compatibility test target.
- `C:/worktree/bxc/` — bxc in-process browser. Primary LLM-DOM backend.
- `C:/worktree/agent-browser/` — vercel-labs full Chromium. Heavy-SPA backend.
- (Anticipated, not yet cloned) — Anthropic Claude Code Ink TUI. Same.

## Roadmap (tick-sized)

| Tick | Deliverable | Status |
|---|---|---|
| T-1  | Worktrees added, foundation 3 crates scaffolded | in-flight |
| T-2  | VT extended with full Ink/React essentials (table above) | queued |
| T-3  | `aphrody-terminal-llm` — sub-agent + MCP + hooks + skills surfaces | queued |
| T-4  | `aphrody-terminal-markdown` — comrak + syntect inline renderer | queued |
| T-5  | `aphrody-terminal-json-out` — JSONL session framing | queued |
| T-6  | `aphrody-terminal-config` — JSON schema + claude.json/mcp.json compat | queued |
| T-6b | `aphrody-terminal-browser` — bxc + agent-browser + edge fallback, OSC `aphrody-browser-*` | queued |
| T-7  | `aphrody term` CLI subcommand + WASM demo HTML | queued |
| T-8  | Demo gif: Claude Code running inside aphrody-terminal w/ live sub-agent pane + browser pane scraping a real site | queued |
