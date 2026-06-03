# VPS deploy memory (agent stack)

**Canonical full guides:**

- [`../../DEPLOY.md`](../../DEPLOY.md) — aphrody Rust CLI, MCP, A2A, Python systemd
- [`../../../bxc/DEPLOY.md`](../../../bxc/DEPLOY.md) — bxc CLI, MCP, systemd crawler
- [`../../../awesome-grok-build/docs/VPS_AI_UNIFY.md`](../../../awesome-grok-build/docs/VPS_AI_UNIFY.md) — Grok + one-page sync

## Fast path (after git pull)

```bash
bash ~/aphrody/scripts/vps-deploy-bxc-aphrody.sh   # build + ~/.local/bin + MCP sync
cd ~/bxc && ./scripts/bxc-control.sh deploy        # systemd bxc + crawler (optional)
bash ~/awesome-grok-build/scripts/aphrody-env-audit.sh
```

## Stop everything (free CPU/RAM)

```bash
sudo systemctl stop aphrody.service bxc.service bxc-crawler.service
sudo systemctl disable aphrody.service bxc.service bxc-crawler.service  # optional
killall -TERM aphrody-mcp bxc-mcp 2>/dev/null || true
fuser -k 8082/tcp 9222/tcp 8788/tcp 8790/tcp 2>/dev/null || true
```

## Clean monorepos

```bash
cd ~/bxc && bun run clean
cd ~/aphrody && rm -rf target node_modules .turbo && cargo clean 2>/dev/null || true
```

## MCP + Grok config (do not duplicate secrets)

| File | Contents |
| --- | --- |
| `~/.config/aphrody/mcp.json` | `aphrody-mcp`, `bxc-mcp` commands + env **names** |
| `~/.grok/config.toml` | `[mcp_servers.aphrody]`, `[mcp_servers.bxc]` |
| `~/aphrody/.env` | Values — sourced from `~/.bashrc` |

Refresh help snapshots: `bash ~/aphrody/scripts/fetch-ai-llms.sh`

## A2A smoke (Rust CLI)

```bash
aphrody a2a --help
cargo test -p a2a-coord --test http_e2e --target x86_64-unknown-linux-gnu
aphrody a2a serve --bind 127.0.0.1:8788   # foreground test
```

See [`../a2a/README.md`](../a2a/README.md).