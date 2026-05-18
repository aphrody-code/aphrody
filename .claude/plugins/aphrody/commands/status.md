---
description: One-screen status report of the aphrody project (binary, branch, PLAN ⏳ items, A2A peer, bxc daemon health, plugin version).
allowed-tools: Bash, Read
argument-hint: (no arguments)
model: sonnet
---

# /status — aphrody project status snapshot

Aggregate the canonical project state in one shot and render a tight
report. No mutations — pure read-only diagnostics.

## Steps

1. **Binary + plugin metadata**
   - `aphrody --version` → version + commit + target.
   - Read `.claude/plugins/aphrody/.claude-plugin/plugin.json` → plugin
     version + agent count + MCP server count.

2. **Git state**
   - `git -C "$(git rev-parse --show-toplevel)" status --short | head -20`
     → uncommitted file count.
   - `git -C "$(git rev-parse --show-toplevel)" log --oneline -5` → last
     5 commits.
   - Current branch via `git -C "$(git rev-parse --show-toplevel)" branch --show-current`.

3. **PLAN.md ⏳ items**
   - `grep -c '⏳' docs/PLAN.md` → count.
   - `grep -n '⏳' docs/PLAN.md | head -10` → first 10 with line numbers.

4. **A2A peer (winclean)**
   - `aphrody doctor --json | jq -r '.peer_a2a.heartbeat_detail'`.
   - Surface stale-flag if `is_stale: true`.

5. **bxc daemon**
   - `curl -fsS --max-time 2 http://localhost:8765/healthz 2>/dev/null`
     → 200 = healthy, else "offline (auto-starts on next aphrody scrape)".

6. **Hooks status** (informational)
   - `aphrody self bootstrap --check | head -20` → toolchain readiness.

## Output format

Single screen, no fluff. Render as :

```
aphrody status — <iso-date>

binary    : <version> @ <commit> (target=<triple>)
plugin    : <version> · <N> agents · <M> MCP servers
branch    : <name> (<N> uncommitted)
last      : <sha> <subject>
PLAN ⏳   : <N> total (first 5 line numbers shown)
A2A peer  : <heartbeat detail> [stale|fresh]
bxc       : <up | down>
toolchain : <N>/N required tools OK
```

End with a single bullet list of action items if any state is degraded
(stale peer > 600 s, bxc down for >1 h, unblocked ⏳ count > 10).

## Anti-stub

- Never fabricate a state if a command fails — surface stderr verbatim.
- Never invent ⏳ items not in PLAN.md.
- The `/status` command is informational only — no `git add`, no
  `aphrody bxc daemon`, no writes.
