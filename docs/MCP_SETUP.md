<!-- SPDX-License-Identifier: Apache-2.0 -->
# MCP servers — team setup

aphrody ships **one native Rust MCP server** plus an optional remote GitHub
MCP. There is no Bun/Node MCP relay anymore — the entire surface is a single
`aphrody-mcp` binary.

## The `aphrody` MCP server (native Rust, plugin-loaded)

The Claude Code plugin (`.claude/plugins/aphrody/`) declares a single stdio
MCP server:

```jsonc
// .claude/plugins/aphrody/.claude-plugin/plugin.json
"mcpServers": {
  "aphrody": { "type": "stdio", "command": "aphrody-mcp", "args": [], "env": {} }
}
```

`aphrody-mcp` is a single ~7 MB native binary (sub-millisecond cold start, zero
JS runtime) exposing **15 tools** across 5 capability groups:

- forensics / recon (8): `coding_style_guide`, `universal_web_fetch`,
  `dns_recon`, `auth_extract`, `chrome_autopsy`, `advanced_recon`,
  `native_hooks`, `start_dashboard`;
- voice (2): `voice_synthesize`, `voice_transcribe`;
- Context7 docs (2): `context7_resolve_library_id`, `context7_query_docs`
  (native Rust port — no Bun bridge);
- Microsoft Learn docs (3): `microsoft_docs_search`, `microsoft_docs_fetch`,
  `microsoft_code_sample_search`;
- reverse engineering (1): `re_triage` (PE/ELF triage via `aphrody-re`).

### Build & install

The `aphrody-mcp` binary is produced by the `google_mcp` crate. There is **no
automatic install hook** — rebuild it manually after touching MCP code:

```bash
# Build the binary (package google_mcp, binary aphrody-mcp)
cargo build --release --bin aphrody-mcp        # equivalently: -p google_mcp

# Copy it onto PATH so the plugin can spawn it
cp target/release/aphrody-mcp ~/.local/bin/    # Linux/macOS
# Windows: copy target\release\aphrody-mcp.exe into a PATH directory
```

Restart your Claude Code session, then verify:

```
/mcp list
→ aphrody    (stdio, native Rust)
```

## Optional — remote GitHub MCP

The official GitHub MCP server (remote `streamable-http`, no Docker) lets you
manage issues / PRs / Actions on `aphrody-code/aphrody` via natural language.

```bash
# Reuse the gh CLI token (recommended; same scopes as gh CLI)
export GITHUB_PERSONAL_ACCESS_TOKEN="$(gh auth token)"
```

Persist the export in your shell rc (`~/.bashrc`, `~/.zshrc`, or PowerShell
`$PROFILE`) and restart Claude Code.

## Plugin surface

The aphrody plugin exposes:

- **1 MCP server**: `aphrody` (15 tools, see above);
- **slash commands**: `/status`, `/docs`;
- a catalogue of agents and skills (see `docs/cargo/SKILLS.md` and
  `.claude/skills/README.md`).

## Security notes

- **Never commit `GITHUB_PERSONAL_ACCESS_TOKEN`**. Reference it via `${VAR}`
  so Claude Code resolves it at runtime from your shell env.
- `.env` files are gitignored.
- If you accidentally commit a token: rotate it immediately at
  <https://github.com/settings/tokens>.
- `aphrody-mcp` itself needs no token (forensics/recon/docs tools are
  local or use their own provider keys via env).
