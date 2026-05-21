---
name: autopilot
description: "Launch the aphrody autopilot loop — Claude Code + Gemini CLI pilot the repo in parallel, infinitely, zero human-in-loop. Use whenever the user says 'lance l'autopilot', 'autopilot', 'duel infini', 'go forever', 'pilote claude et gemini', 'tu pilotes en boucle', 'fais tourner', 'mode auto', 'autonome', or any variant that means 'keep working on aphrody by yourself without me'. Spawns `scripts/autopilot.sh` in the background ; the loop picks the next ⏳ item from `docs/PLAN.md`, dispatches it to Claude (`claude -p`) and Gemini (`aphrody gemini` / resolved `gemini` binary) in parallel each tick, logs NDJSON to `var/log/autopilot.jsonl`, bumps `ai/heartbeat.txt`. Stop with `kill $(cat var/run/autopilot.pid)`."
license: Apache-2.0
metadata:
  version: 1.0.0
  source: aphrody scripts/autopilot.sh + feedback_aphrody_full_autonomy
---

# autopilot — duel Claude + Gemini infini, zéro humain

The aphrody autopilot is a background bash loop that pilots the repository
end-to-end. Two LLM lanes run in parallel every tick :

- **Claude lane** : `claude -p` with a prompt to pick the highest-leverage
  `⏳` item from `docs/PLAN.md` and ship it (implement → `cargo check` →
  Conventional Commit, no AI co-author trailer per `AGENTS.md`).
- **Gemini lane** : `aphrody gemini --prompt` (resolves the `gemini` binary
  per CLAUDE.md §0.4, no Bun/npx) — independent audit of the most recent
  commit, fact-checks
  against the `best-stack-2026` skill (canonical Rust 2026 crates, no GPL
  leak, cross-platform). Outputs JSON verdict, never modifies files.

The loop keeps running until you `kill $(cat var/run/autopilot.pid)` or
send SIGINT. Each tick takes ~60s default + LLM latency.

## How to launch

The wrapper has bash↔pwsh parity (cf. CLAUDE.md §4.1). Pick the one that
matches the host shell.

```powershell
# Windows native (PowerShell 7+) — recommended on this repo
pwsh scripts/autopilot.ps1                              # loop infini, 60s tick
pwsh scripts/autopilot.ps1 -Once                        # single tick (debug)
pwsh scripts/autopilot.ps1 -Interval 30 -MaxTicks 100   # bounded
```

```bash
# Linux / macOS / git-bash — same flags, --kebab-case
bash scripts/autopilot.sh                               # loop infini
bash scripts/autopilot.sh --once
APHRODY_AUTOPILOT_MAX_TICKS=10 bash scripts/autopilot.sh
```

When invoking this skill from **Claude Code on Windows** (current host), the
canonical action is :

```powershell
Start-Process pwsh -ArgumentList '-NoProfile','-File','scripts/autopilot.ps1' -WindowStyle Hidden
"autopilot started, pid=$(Get-Content var/run/autopilot.pid)"
Get-Content var/log/autopilot.jsonl -Wait    # tail in another shell
```

Or, equivalent from git-bash on Windows :

```bash
nohup pwsh -NoProfile -File scripts/autopilot.ps1 > /dev/null 2>&1 &
echo "autopilot started, pid=$(cat var/run/autopilot.pid)"
tail -f var/log/autopilot.jsonl
```

Return to the user immediately with the PID and the tail command — do
**not** wait for the loop. Per `feedback_aphrody_full_autonomy`, the user
never has to confirm or check in ; they can ignore the loop indefinitely.

## Environment variables

| Var | Default | Effect |
|---|---|---|
| `APHRODY_AUTOPILOT_INTERVAL` | `60` | seconds between ticks |
| `APHRODY_AUTOPILOT_MAX_TICKS` | `0` (infini) | hard stop after N ticks |
| `APHRODY_CLAUDE_TIMEOUT` | `300` | seconds before killing the Claude subprocess |
| `APHRODY_GEMINI_TIMEOUT` | `300` | seconds before killing the Gemini subprocess |

## Outputs

| Path | Content |
|---|---|
| `var/log/autopilot.jsonl` | NDJSON — one line per tick, contains `ts`, `tick`, `task`, `claude` (head 800 chars), `gemini` (head 800 chars). Append-only. |
| `var/run/autopilot.pid` | PID of the bash loop. Use to `kill` the daemon. |
| `ai/heartbeat.txt` | ISO-8601 timestamp + current task — overwritten each tick. Read by A2A peers (cf. `CLAUDE.md` §6.1). |

## Stop conditions

- **Windows** : `Stop-Process -Id (Get-Content var/run/autopilot.pid)`
- **Unix/git-bash** : `kill $(cat var/run/autopilot.pid)` (SIGTERM, clean)
- `Ctrl-C` if running in foreground (`ConsoleCancelEventHandler` / SIGINT trap → clean shutdown)
- Set `APHRODY_AUTOPILOT_MAX_TICKS=N` (env) or `-MaxTicks N` (pwsh) / `--max-ticks N` (bash) to stop after N iterations
- The loop **does not stop on individual tick failures** — Claude or
  Gemini timeouts are logged as `{"err": "..."}` and the loop continues.
  This is intentional : transient provider errors should not kill the
  daemon.

## Safety / blast radius

- Each Claude tick runs with `--dangerously-skip-permissions` so it can
  edit, commit, and run arbitrary commands without prompts. This is
  acceptable per `feedback_aphrody_full_autonomy` because the loop runs
  in a single-user repo with `git revert` always available.
- Gemini lane is **read-only** by prompt design (audit only, must output
  JSON, must not write files). Even if a future Gemini decided to write,
  it does not have `--dangerously-skip-permissions` equivalent here.
- The loop **does not** `git push` automatically. Commits stay local until
  the operator pushes manually (or until `release-please` / `dependabot`
  bot account does so). This is the single guardrail kept on purpose.

## Relation to existing aphrody loops

- `a2a-duel-loop` skill = ONE tick per invocation (paired with external
  `/loop 60s`). Autopilot subsumes it by running the loop *inside* a
  single skill invocation, no external `/loop` needed.
- `aphrody-yolo-grind` skill = 4 sub-agents per tick on the same model.
  Autopilot is broader : two **different models** (Claude + Gemini) on
  the same tick, with different roles (ship vs audit).
- Both legacy skills remain valid for narrower workflows.

## Verify observable

```powershell
pwsh scripts/autopilot.ps1 -Once
Get-Content var/log/autopilot.jsonl | Select-Object -Last 2 | ConvertFrom-Json |
    ForEach-Object { [pscustomobject]@{ tick = $_.tick; task = $_.task; has_claude = ($_.claude.Length -gt 0); has_gemini = ($_.gemini.Length -gt 0) } }
```

Should print one row with `tick: 1`, both `has_claude` and `has_gemini`
`True` (or the line records `err` keys if a provider was unreachable —
loop continues regardless, that's the design).
