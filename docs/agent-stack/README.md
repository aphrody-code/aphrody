<!-- SPDX-License-Identifier: Apache-2.0 -->
# Unified agent stack (VPS snapshot)

Snapshot date: **2026-06-03**. CLI `--help` captures and `llms.txt` mirrors live under this directory.

## CLIs on this host

| Binary | Role | Help snapshot |
| --- | --- | --- |
| **agy** | Antigravity / Gemini CLI bridge | [`agy-help.txt`](agy-help.txt) |
| **grok** | Grok Build TUI (xAI) | [`grok-help.txt`](grok-help.txt) |
| **claude** | Claude Code | [`claude-help.txt`](claude-help.txt) |
| **bxc** | Bun-native browser engine + domain CLIs | [`bxc-help.txt`](bxc-help.txt) |
| **aphrody** | Cross-platform Rust agent (`1.0.0-canary`) | [`aphrody-help.txt`](aphrody-help.txt) |

Install paths (typical): `~/.local/bin/{agy,grok,claude,bxc,aphrody,aphrody-mcp,bxc-mcp}`.

## MCP configuration

### Shared config: `~/.config/aphrody/mcp.json`

Used by **aphrody**, **Claude**, and compatible agents. Structure:

| Server | Command | Purpose |
| --- | --- | --- |
| `aphrody` | `~/.local/bin/aphrody-mcp` | Docs, RE, Context7, Gemini bridge, Postgres/Redis-backed tools |
| `bxc` | `~/.local/bin/bxc-mcp` | Browser automation, scrape/crawl, X client, Grok API tools (~40 tools) |

**Env vars injected at MCP spawn** (names only — values live in `~/aphrody/.env`, `~/.bash_secrets`, or service env):

| Variable | Used by | Purpose |
| --- | --- | --- |
| `BXC_DB_PATH` | aphrody, bxc | Primary bxc SQLite (`/home/ubuntu/bxc/data/bxc.sqlite`) |
| `BXC_MEMORY_DB` | aphrody, bxc | Agent memory SQLite |
| `REDIS_URL` | aphrody, bxc | Local Redis |
| `DATABASE_URL` | aphrody | Postgres (rpb_neon) |
| `DATABASE_URL_AZALEE` | aphrody | Postgres (rose_griffon) |

Do **not** commit tokens into `mcp.json`; prefer env at spawn time.

### Grok Build: `~/.grok/config.toml`

| Section | Notes |
| --- | --- |
| `[models]` | Default `grok-build`; web search `grok-4.20-multi-agent` |
| `[permission]` | VPS: `rules = [{ action = "allow", tool = "any" }]` |
| `[subagents]` | Roles: `code-researcher`, `parallel-implementer`, `verifier` |
| `[compat.claude]` / `[compat.cursor]` | Skills, rules, agents, MCPs, hooks enabled |
| `[plugins]` | `aphrody`, `material-design`, `awesome-grok-unlocked` |
| `[mcp_servers.aphrody]` | Same binary as aphrody MCP; 15s startup / 120s tool timeout |

Auth for Grok Build (not in committed docs): `~/.grok/auth.json` after `grok login`. Optional metered API: `XAI_API_KEY` in `~/.bash_secrets`.

### Headless `grok -p` (verified)

```bash
grok -p "task" --always-approve --permission-mode bypassPermissions --max-turns 80
```

Models: `grok-build` (default), `grok-composer-2.5-fast` (`-m`). **Do not** pass `--effort` or `--reasoning-effort` with `grok-build` (HTTP 400). Full notes: [`~/awesome-grok-build/docs/grok-headless.md`](file:///home/ubuntu/awesome-grok-build/docs/grok-headless.md).

### Other MCP entrypoints

| Consumer | Config location |
| --- | --- |
| Gemini / agy | `~/.gemini/antigravity-cli/mcp_config.json` (may also reference `bxc-mcp`) |
| Build bxc-mcp | `cd ~/bxc && bun run build:mcp` → `dist/standalone/bxc-mcp` |

## Shared session & data paths

| Path | Domain | Contents |
| --- | --- | --- |
| `~/.aphrody/x-session.json` | **Twitter/X** | `auth_token` + `ct0` for `bxc x` / `@aphrody-code/x` (mode `0600`) |
| `~/.bxc/cookies/xcom.json` | X.com | Full cookie jar (bxc shortcut `xcom`) |
| `~/.bxc/cookies/grok.json` | xAI web | grok.com cookies (separate from X session) |
| `~/.grok/auth.json` | xAI API | OIDC JWT for Grok Build / `bxc grok` (field `key`) |
| `BXC_DB_PATH` | bxc engine | `/home/ubuntu/bxc/data/bxc.sqlite` |
| `BXC_MEMORY_DB` | MCP memory | `/home/ubuntu/bxc/bxc-memory.sqlite` |

Twitter/X env vars: `X_AUTH_TOKEN`, `X_CT0`, `X_HANDLE` → `~/.bash_secrets`. See [`../x/env-and-auth.md`](../x/env-and-auth.md).

xAI env: `XAI_API_KEY` (metered fallback). See [`../grok/env-and-auth.md`](../grok/env-and-auth.md).

## Documentation mirrors (`llms.txt`)

Refresh: `bash ~/aphrody/scripts/fetch-ai-llms.sh`

| File | Source |
| --- | --- |
| [`x-ai-llms.txt`](x-ai-llms.txt) | https://docs.x.ai/llms.txt |
| [`bun-llms.txt`](bun-llms.txt) | https://bun.com/docs/llms.txt |
| [`anthropic-llms.txt`](anthropic-llms.txt) | https://docs.anthropic.com/llms.txt |
| [`claude-code-llms.txt`](claude-code-llms.txt) | https://code.claude.com/docs/llms.txt |

## Agent state (`~/.aphrody`)

Layout, workspaces, secrets policy: [`../dot-aphrody/README.md`](../dot-aphrody/README.md).  
Curated memory: `~/.aphrody/workspace/MEMORY.md`.

## Deploy (VPS)

| Doc | Contents |
| --- | --- |
| [`DEPLOY.md`](DEPLOY.md) | Fast stop/start, clean, A2A smoke |
| [`../../DEPLOY.md`](../../DEPLOY.md) | Full aphrody deploy |
| [`../../../bxc/DEPLOY.md`](../../../bxc/DEPLOY.md) | Full bxc deploy |
| [`../../../awesome-grok-build/docs/VPS_AI_UNIFY.md`](../../../awesome-grok-build/docs/VPS_AI_UNIFY.md) | Grok global memory |

## Related trees

| Topic | Path |
| --- | --- |
| Twitter/X + bxc | [`../x/README.md`](../x/README.md) |
| xAI Grok | [`../grok/README.md`](../grok/README.md) |
| X Pro (Gryphon decks) | [`x-pro-integration.md`](x-pro-integration.md) → canonical [`~/bxc/packages/x/docs/X_PRO.md`](file:///home/ubuntu/bxc/packages/x/docs/X_PRO.md) |
| agy ↔ aphrody map | [`../agy-cli/aphrody-agy-map.md`](../agy-cli/aphrody-agy-map.md) |

## Audit

```bash
bash ~/awesome-grok-build/scripts/aphrody-env-audit.sh
aphrody doctor
grok inspect   # when available
```

Never paste secrets into chat or committed markdown.