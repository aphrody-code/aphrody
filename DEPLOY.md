# Aphrody — deployment guide (VPS & agents)

Canonical deploy for the **Rust CLI**, **aphrody-mcp** (crate `google_mcp`), **Bun/TS workspace**, optional **Python systemd site**, and **A2A coordination**. Pair with [`../bxc/DEPLOY.md`](../bxc/DEPLOY.md).

**Snapshot:** 2026-06-03 · repo `aphrody-code/aphrody`

---

## Deploy surfaces (do not confuse)

| Surface | Binary / runtime | Default port | systemd unit |
| --- | --- | --- | --- |
| **Rust CLI** | `~/.local/bin/aphrody` | — (CLI) | — |
| **MCP server** | `~/.local/bin/aphrody-mcp` (`google_mcp` package) | stdio | — |
| **A2A coord** | `aphrody a2a serve` | `127.0.0.1:8788` | — |
| **Python site** | `/opt/aphrody/venv/.../aphrody serve` | `0.0.0.0:8082` | `aphrody.service` (user `aphrody`) |
| **Bun apps** | `turbo` / `apps/web` | dev ports | — |

The Rust CLI (`aphrody a2a`, `aphrody doctor`, …) is **independent** of the Python `aphrody.service` on `:8082`.

---

## Prerequisites (Linux VPS)

```bash
source ~/.cargo/env
# Recommended: awesome-grok-build env (nightly ≥ 1.97, no broken sccache)
source ~/awesome-grok-build/scripts/rust-nightly-env.sh 2>/dev/null || true

export RUSTC_WRAPPER=
export CARGO_CONFIG="${APHRODY_REPO:-$HOME/aphrody}/.cargo/config.linux-vps.toml"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/aphrody/target/x86_64-unknown-linux-gnu}"

rustup toolchain install nightly-2026-05-17 2>/dev/null || true
command -v bun && bun --version
```

**Note:** Default repo `.cargo/config.toml` targets Windows MSVC. On Linux always set `CARGO_CONFIG` to [`.cargo/config.linux-vps.toml`](.cargo/config.linux-vps.toml) or pass `--config-path`.

Release artifacts land in:

`$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release/` (when using `--target x86_64-unknown-linux-gnu`).

---

## 1. Rust CLI + MCP (agents)

### Build + install

```bash
cd ~/aphrody
bun install                    # TS workspace (packages/apps)

cargo build --release --target x86_64-unknown-linux-gnu -p aphrody -p google_mcp

RELEASE="$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release"
install -m 755 "$RELEASE/aphrody" ~/.local/bin/aphrody
install -m 755 "$RELEASE/aphrody-mcp" ~/.local/bin/aphrody-mcp
# google_mcp crate may also emit binary name google_mcp — prefer aphrody-mcp if present
[[ -f "$RELEASE/google_mcp" && ! -f "$RELEASE/aphrody-mcp" ]] && \
  install -m 755 "$RELEASE/google_mcp" ~/.local/bin/aphrody-mcp

aphrody version
aphrody a2a --help
```

### Portable install script

[`scripts/deploy.sh`](scripts/deploy.sh) discovers `target/*/release` and copies prefixed binaries:

```bash
./scripts/deploy.sh --target x86_64-unknown-linux-gnu --prefixes aphrody,mrx
```

Aliases from `rust-nightly-env.sh` (if sourced in `~/.bashrc`):

- `aphrody-build` → `cargo build -p aphrody --release --target x86_64-unknown-linux-gnu`
- `aphrody-install` → copy to `~/.local/bin`

---

## 2. Unified VPS script (bxc + aphrody + MCP sync)

```bash
bash ~/aphrody/scripts/vps-deploy-bxc-aphrody.sh
```

Steps: bxc `bun install` + `build:mcp`, optional `x-cli`, aphrody `cargo build`, install to `~/.local/bin`, [`vps-sync-agent-stack.sh`](scripts/vps-sync-agent-stack.sh), optional yoyo hub `:8790`.

Agent config only (no full Rust rebuild):

```bash
bash ~/aphrody/scripts/vps-sync-agent-stack.sh
```

Writes/refreshes `~/.config/aphrody/mcp.json`, appends `[mcp_servers.bxc]` to `~/.grok/config.toml` if missing, help snapshots under `docs/agent-stack/`.

---

## 3. Python static site (systemd, optional)

Bootstrap **once** as root (wheel + site root):

```bash
cd ~/aphrody/py/aphrody/deploy
sudo ./deploy-vps.sh --mode react --wheel /path/to/aphrody-*.whl \
  --site /path/to/dist --host 0.0.0.0 --port 8082
```

- Unit: [`py/aphrody/deploy/aphrody.service`](py/aphrody/deploy/aphrody.service)
- Env: `/etc/aphrody/serve.env`
- User: `aphrody` · data under `/opt/aphrody`

```bash
sudo systemctl status aphrody.service
curl -sS -o /dev/null -w "%{http_code}\n" http://127.0.0.1:8082/
```

---

## 4. A2A coordination

Spec: [A2A Protocol 1.0](https://a2a-protocol.org/latest/specification/). In-repo: [`docs/a2a/README.md`](docs/a2a/README.md), manifest [`ai.json`](ai.json).

```bash
# HTTP: agent card + JSON-RPC + file /msg
aphrody a2a serve --bind 127.0.0.1:8788

# JSONL duel / coord
aphrody a2a tick --iteration 1 --side aphrody --peer winclean

# Native peers (dry-run first)
aphrody a2a invoke "summarize PLAN.md" --peer grok --dry-run
```

Tests:

```bash
cargo test -p a2a-coord --test http_e2e --target x86_64-unknown-linux-gnu
```

Native peers: `grok`, `agy`, `claude`, `bxc` — see `docs/a2a/README.md` for CLI flags (no `--effort` with `grok-build`).

---

## 5. Agent stack memory (Claude · Grok · Gemini)

| Doc | Role |
| --- | --- |
| [`docs/agent-stack/README.md`](docs/agent-stack/README.md) | MCP matrix, paths, versions |
| [`docs/grok/README.md`](docs/grok/README.md) | xAI Grok Build on VPS |
| [`docs/agy-cli/`](docs/agy-cli/) | agy OAuth (`~/.gemini/antigravity-cli/...`) |
| [`~/awesome-grok-build/docs/aphrody-grok-setup.md`](../awesome-grok-build/docs/aphrody-grok-setup.md) | Grok plugin + credentials |
| [`~/awesome-grok-build/docs/VPS_AI_UNIFY.md`](../awesome-grok-build/docs/VPS_AI_UNIFY.md) | One-page VPS memory |

Env: `~/aphrody/.env` on PATH via `~/.bashrc` — never commit. Audit: `bash ~/awesome-grok-build/scripts/aphrody-env-audit.sh`.

---

## systemd / process hygiene

| Service | Purpose | Stop all daemons |
| --- | --- | --- |
| `aphrody.service` | Python SPA `:8082` | `sudo systemctl stop aphrody.service` |
| `bxc.service` | CDP `:9222` | see bxc DEPLOY.md |
| `bxc-crawler.service` | 24/7 crawler | see bxc DEPLOY.md |

Avoid duplicate **cargo** release builds (one job; LTO is heavy). Kill orphans before clean:

```bash
killall -TERM aphrody-mcp 2>/dev/null || true
# cargo: kill by PID from pgrep -x cargo — avoid pkill -f patterns in scripts that embed "cargo"
```

---

## Clean rebuild

```bash
cd ~/aphrody
sudo systemctl stop aphrody.service 2>/dev/null || true
killall -TERM aphrody-mcp 2>/dev/null || true

export RUSTC_WRAPPER= CARGO_CONFIG="$PWD/.cargo/config.linux-vps.toml"
export CARGO_TARGET_DIR="$PWD/target/x86_64-unknown-linux-gnu"
cargo clean --target x86_64-unknown-linux-gnu
rm -rf target node_modules .turbo apps/*/.next packages/*/dist

bun install
cargo build --release --target x86_64-unknown-linux-gnu -p aphrody -p google_mcp
# install bins (see §1)
bash scripts/vps-sync-agent-stack.sh
```

No root `bun clean` script at repo root — use `rm` above or package-level cleans.

---

## Health checks

```bash
aphrody version
aphrody doctor
command -v aphrody-mcp && echo ok
test -f ~/.config/aphrody/mcp.json && echo mcp.json ok
cargo test -p a2a-coord --test http_e2e --target x86_64-unknown-linux-gnu
```

---

## See also

- [`README.md`](README.md) — product overview
- [`CLAUDE.md`](CLAUDE.md) — Claude Code rules
- [`docs/automation_and_deployment.md`](docs/automation_and_deployment.md) — Python autopilot / packaging
- [`scripts/deploy.sh`](scripts/deploy.sh) — cross-platform binary install