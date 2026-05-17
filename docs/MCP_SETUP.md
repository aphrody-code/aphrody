<!-- SPDX-License-Identifier: Apache-2.0 -->
# MCP servers — team setup

aphrody ships **3 MCP servers** for the team. Two are project-scoped
(auto-loaded with the repo), one is user-installed.

## Project-scoped MCPs (auto-loaded)

### `github` (via `.mcp.json` at repo root)

Official GitHub MCP server (remote `streamable-http`, no Docker). Lets
you ask Claude to manage issues / PRs / Actions on
`aphrody-code/{aphrody, n2b, bxc, ui, next.js}` using natural language.

**Setup** (one time, per developer):

```bash
# Option A — reuse the gh CLI token (recommended; same scopes as gh CLI)
export GITHUB_PERSONAL_ACCESS_TOKEN="$(gh auth token)"

# Option B — generate a fresh fine-scoped PAT for MCP use
#   GitHub → Settings → Developer settings → Personal access tokens (classic)
#   Scopes needed: repo, workflow, read:org
#   Then:
export GITHUB_PERSONAL_ACCESS_TOKEN="ghp_…"
```

Persist the export in your shell rc (`~/.bashrc`, `~/.zshrc`, or for
PowerShell `$PROFILE`) so every Claude Code session picks it up.

Restart your Claude Code session after setting the env var. Verify the
MCP loaded:

```
/mcp list
```

You should see `github` with the 50+ tools (create_issue, get_pr, etc.).

### `bxc-scrapper` (via `.claude/plugins/aphrody/`)

Auto-loaded with the aphrody plugin. Local stdio Bun server. Tools:
`bxc_scrape`, `bxc_recon`, `bxc_detect`, `google_search`,
`google_atlas_route`, `extract_structured`, `vision_analyze`.

**Setup**:

```bash
cd .claude/plugins/aphrody/mcp/bxc-scrapper
bun install
```

Restart Claude Code. The MCP server boots on demand.

Optional — if the bxc daemon is running at `http://127.0.0.1:8765`,
the MCP routes there for performance; otherwise it spawns `bxc-engine`
per call.

## User-installed MCPs (recommended)

These are Claude Code plugins, not in `.mcp.json`. Install via the
plugin marketplace.

### `context7`

Live documentation fetching for any library/framework/SDK. Critical for
fact-checking dep versions and API signatures before adding to
`Cargo.toml`. See `feedback_context7_for_docs` memory.

```
/plugin install context7
```

### `playwright`

Real browser automation. Used by the `m3-spec-auditor` agent to
screenshot M3 components and diff against the live spec page on
`m3.material.io`.

```
/plugin install playwright
```

Then once installed:

```bash
bunx playwright install chromium
```

## Verification

After installing all three, you should have:

```
/mcp list
→ github               (remote streamable-http)
→ bxc-scrapper         (stdio bun)
→ context7             (plugin)
→ playwright           (plugin)
```

And the aphrody plugin should expose:

- 3 skills: `pixel-perfect`, `rust-target-check`, `m3-component`
- 3 agents: `n2b-ultra`, `cross-platform-validator`, `m3-spec-auditor`
- 2 commands: `/scrape`, `/tokens`
- 3 hooks (PostToolUse): `oxlint`, `cargo-check`, `cargo-toml-validate`

## Security notes

- **Never commit `GITHUB_PERSONAL_ACCESS_TOKEN`**. The `.mcp.json` file
  uses `${VAR}` references which Claude Code resolves at runtime from
  your shell env — the token never touches the repo.
- `.env` files are gitignored (`.gitignore` section 7).
- If you accidentally commit a token: rotate it immediately at
  <https://github.com/settings/tokens>.
- The `bxc-scrapper` MCP does NOT need any token (local-only).
