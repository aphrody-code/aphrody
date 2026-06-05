# Memory index — aphrody monorepo (Claude Code)

Snapshot: **2026-06-04**. Project path: `/home/ubuntu/aphrody`.

## Deploy & VPS (canonical)

- [Deploy aphrody](../../../../aphrody/DEPLOY.md) — Rust CLI, `aphrody-mcp`, A2A, `vps-deploy-bxc-aphrody.sh`, Linux `config.linux-vps.toml`
- [Deploy bxc](../../../../bxc/DEPLOY.md) — `bxc-control.sh`, systemd `:9222` + crawler
- [Agent-stack fast path](../../../../aphrody/docs/agent-stack/DEPLOY.md) — stop/clean/smoke
- [VPS AI unify](../../../../awesome-grok-build/docs/VPS_AI_UNIFY.md) — Grok + shared MCP memory

## Agent home (`~/.aphrody`)

- [Layout & MEMORY curatée](../../../../.aphrody/workspace/MEMORY.md) — faits structurels VPS
- [AGENTS policy](../../../../.aphrody/workspace/AGENTS.md) — autonomie Grok/Claude
- Doc repo: `aphrody/docs/dot-aphrody/README.md`

## Build rules (Linux)

- `export RUSTC_WRAPPER=` (sccache absent → build fail)
- `CARGO_CONFIG=~/aphrody/.cargo/config.linux-vps.toml`
- Release bins: `target/x86_64-unknown-linux-gnu/x86_64-unknown-linux-gnu/release/{aphrody,aphrody-mcp}`
- **One** `cargo build --release` at a time (LTO)
- Install: `scripts/deploy.sh` or `install -m 755` (not `cp` while binary running)
- [Sibling builds & versions](sibling-repo-build-and-versions.md) — `CARGO_TARGET_DIR` sends n2b/bxc builds into aphrody's target; bxc version is compile-baked; n2b 0.6.1 / bxc 0.6.2

## MCP

- Shared: `~/.config/aphrody/mcp.json` — `aphrody-mcp`, `bxc-mcp`; secrets in `~/aphrody/.env`, not JSON
- Claude uses aphrody plugin + same binaries

## Tooling / Bun

- [n2b --fix + agent stack Bun](n2b-fix-and-agent-stack-bun.md) — n2b only auto-applies the safe class (not fs/sleep); agent stack is Node-free
- [Bun test runner pattern](bun-test-runner-pattern.md) — `scripts/test-all.ts` in rpbey+shenron covers EVERY scope (turbo skips script-less ones); per-file isolation, .next double-run, vendored-fork classification, `bun run test:ci`
- VPS Bun is **1.4.0-canary.1** (manifests pin 1.3.14, ignore that)

## Cross-repo contracts (aphrody = hub)

- `aphrody/docs/api-unified-pattern.md` — REST/GraphQL/`Bun.serve`/cron unification (rpbey+shenron); A2A stays JSON-RPC, exempt
- `aphrody/docs/rag-unified-pattern.md` — shared e5-small sidecar + RRF retrieval
- [bxc test pkgs: bxc-test + next-playwright](bxc-test-and-next-playwright.md) — Playwright-compat `@aphrody-code/bxc-test` + Next `instant()` port over native CDP (shipped, bxc `11a923d`); `Network.deleteCookies` fix

## rpbey (Beyblade community app)

- [Ranking calc + Challonge import](rpbey-ranking-and-import.md) — BTS/global + Stardust ranking formulas/constants, import-bts-tournaments.ts, triggers, the B_TS4-dup root cause + participant unique constraint added 2026-06-04

## A2A (Rust)

- `aphrody a2a serve|tick|invoke` — commit **b105cbd**+ ; tests `cargo test -p a2a-coord --test http_e2e`
- Manifest: `~/aphrody/ai.json` · coord: `.coord/*.jsonl`

## Services — all **active + enabled** (verified 2026-06-04; may be stopped to save RAM)

| Unit | Port | Note |
| --- | --- | --- |
| `aphrody.service` | 8082 | Python `/opt/aphrody` — **not** Rust CLI |
| `bxc.service` | 9222 | CDP (execs bxc standalone — restart after a `bxc` rebuild) |
| `bxc-crawler.service` | — | 24/7 worker |

Re-enable: `sudo systemctl enable --now bxc.service bxc-crawler.service aphrody.service`
Stop to free RAM: `sudo systemctl disable --now bxc-crawler.service bxc.service aphrody.service`

## Credentials

- Audit: `bash ~/awesome-grok-build/scripts/aphrody-env-audit.sh`
- Never commit `~/aphrody/.env`, `~/.aphrody/x-session.json`, cookie jars
## Feedback (working style)

- [No questions, direct action](feedback-no-questions-direct-action.md) — execute, never ask for confirmation (user emphatic 2026-06-04)
- [No reflexive loops](feedback-no-reflexive-loops.md) — don't mechanically repeat bxc/workflow calls; think if the action is even needed (user said "infinite loop, pause et réfléchi" 2026-06-04)
