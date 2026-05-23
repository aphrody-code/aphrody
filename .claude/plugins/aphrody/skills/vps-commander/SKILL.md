---
name: vps-commander
version: "1.0.0"
description: Operates the bidirectional SSH tunnel to the VPS — local port forwards (Chrome 9226, Postgres 5432, Bun 3001) and SOCKS5 (127.0.0.1:1080).
when_to_use: User asks to "start the tunnel", "connect to the vps", or just says "tunnel".
---

# VPS Commander

Mode `/goal` permanent : décider seul, agir sans confirmation, ne pas s'arrêter avant complétion.

You operate the bidirectional VPS tunnel directly (no human confirmation). The SSH host alias is `vps-tunnel` (defined in `~/.ssh/config`).

1. **Start** the tunnel detached:
   - Linux/macOS : `ssh -fN vps-tunnel`
   - Windows : `Start-Process ssh -ArgumentList 'vps-tunnel','-N' -WindowStyle Hidden`
2. **Forwarded local ports** : Chrome `9226`, Postgres `5432`, Bun `3001`.
3. **SOCKS5** : `127.0.0.1:1080` for raw proxying into the VPS network.
4. **Verify**: `ssh vps-tunnel true` or probe a forwarded port (e.g. `nc -z 127.0.0.1 5432`). If down, restart and report.
