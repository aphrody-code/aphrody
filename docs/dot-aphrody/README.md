# `~/.aphrody` — agent state directory

Canonical layout for **aphrody-agent-home**, MCP sessions and X/Twitter auth mirrors on a developer or VPS host.

**Do not commit** this tree — permissions `0600`/`0700` on secrets.

Snapshot: **2026-06-03**

---

## Root layout

| Path | Mode | Purpose |
| --- | --- | --- |
| `aphrody.json` | `644` | Bootstrap metadata (`version`, `workspace`, `bootstrapped_at`) |
| `.env` | `600` | Env overrides (often symlinked or mirrored from `~/aphrody/.env`) |
| `personas.json` | `644` | Persona registry for multi-agent routing |
| `settings.json` | `644` | Global policy: full-auto, bypass, proactive, concise |
| `x-session.json` | `600` | X.com `auth_token` + `ct0` for `bxc x` / `@aphrody-code/x` |
| `x-account.json` | `600` | Account metadata (no secrets in docs) |
| `cookies/xcom.json` | `600` | Mirror of x.com cookie jar |
| `cookies/grok.json` | `600` | Mirror of grok.com cookies |

Optional stores (may exist on VPS):

| Path | Purpose |
| --- | --- |
| `x-store.sqlite` | X RAG corpus (tweets FTS + embeddings) |
| `antigravity-token.json` | agy OAuth copy from CLI login |
| `terminal.json` | Imported terminal/MCP config schema |

---

## Workspaces (`workspace*`)

Resolution order ([`aphrody-agent-home`](../../crates/aphrody-agent-home)):

1. `$APHRODY_WORKSPACE` — explicit override
2. `$APHRODY_PROFILE` → `~/.aphrody/workspace-<profile>`
3. Default → `~/.aphrody/workspace`

| Directory | Profile |
| --- | --- |
| `workspace/` | default |
| `workspace-azalee/` | Azalée (+ `.coord/` A2A JSONL) |
| `workspace-shenron/` | Shenron |
| `workspace-debug/` | debug |

### Per-workspace files

| File | Required | Role |
| --- | --- | --- |
| `AGENTS.md` | yes | Operating rules, autonomy, skill routing |
| `TOOLS.md` | yes | VPS tool conventions (YAML frontmatter) |
| `SOUL.md` | opt | Persona |
| `IDENTITY.md` | opt | Agent name / vibe |
| `USER.md` | opt | User preferences |
| `MEMORY.md` | opt | Curated long-term facts |
| `HEARTBEAT.md` | opt | Periodic checklist |
| `BOOT.md` | opt | Session boot checklist |
| `memory/YYYY-MM-DD.md` | opt | Daily log |
| `.aphrody/workspace-state.json` | auto | Schema v1 state |

Coordination (A2A file channel): `workspace-*/.coord/inbox-from-<peer>.jsonl`

---

## Relation to repo `~/aphrody`

| Repo | Dot-dir |
| --- | --- |
| `~/aphrody/.env` | Often **source of truth** for API keys; sourced in `~/.bashrc` |
| `~/aphrody/ai.json` | A2A manifest (repo, not dot-dir) |
| `~/aphrody/.coord/` | Repo-local coord when working in tree |

MCP config is **not** under `~/.aphrody`:

| Agent | Config |
| --- | --- |
| Claude / aphrody | `~/.config/aphrody/mcp.json` |
| agy | `~/.gemini/antigravity-cli/mcp_config.json` (+ [`MCP.md`](file:///home/ubuntu/.gemini/antigravity-cli/MCP.md)) |
| Grok | `~/.grok/config.toml` |

---

## Refresh / audit

```bash
# Curated memory lives in ~/.aphrody/workspace/MEMORY.md (edit by agents)
aphrody doctor
```

## See also

- [`../agent-stack/README.md`](../agent-stack/README.md)
- [`../agent-stack/DEPLOY.md`](../agent-stack/DEPLOY.md)
- [`../../DEPLOY.md`](../../DEPLOY.md)
