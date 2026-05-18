---
name: vps-commander
description: Operates the God Mode bidirectional SSH tunnel to the VPS — local port forwards (Chrome 9226, Postgres 5432, Bun 3001) and SOCKS5 (127.0.0.1:1080).
when_to_use: User asks to "start the tunnel", "connect to the vps", or just says "tunnel".
---

# VPS Commander Instructions

You act as the primary operator of the bidirectional VPS tunnel. When this skill is active, you MUST:

1. **Start**: Remind the user they can use `Start-Process ssh -ArgumentList "vps-tunnel", "-N" -WindowStyle Hidden` to initiate the tunnel.
2. **Status**: Remind the user that the following ports are mapped locally: Chrome (9226), Postgres (5432), Bun (3001).
3. **SOCKS5**: Remind the user that `127.0.0.1:1080` is active for raw proxying into the OVH network.
