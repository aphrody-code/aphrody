# Deploying aphrody on a Linux VPS

`aphrody` runs cross-platform (the Windows-only Blender paths are
existence-checked with Linux fallbacks; binaries/dirs are env-overridable). On a
VPS it serves a built front-end (e.g. a TypeScript React `dist/`) or supervises a
Rust app, under **systemd**, tuned to use available **RAM**.

## Build the wheel (on your dev box)
```bash
uv build --package aphrody --wheel        # -> dist/aphrody-0.1.0-py3-none-any.whl
scp dist/aphrody-0.1.0-*.whl  aphrody/deploy/*  vps:/tmp/aphrody-deploy/
```

## Serve a React (or any static/SPA) site
```bash
# build the front-end, ship its dist to the VPS (e.g. /srv/site), then:
sudo ./deploy-vps.sh --mode react --wheel /tmp/aphrody-deploy/aphrody-0.1.0-*.whl \
     --site /srv/site --host 0.0.0.0 --port 8080
```
This creates a system `aphrody` user, a venv in `/opt/aphrody`, installs the
wheel, writes `/etc/aphrody/serve.env`, installs `aphrody.service`, and
`enable --now`. The server has **SPA fallback** (unknown routes → `index.html`)
and **`--cache`** (the whole site preloaded into RAM → zero disk hits).

Manual equivalent: `aphrody serve /srv/site --host 0.0.0.0 --port 8080 --cache`.

## Supervise a Rust app instead
```bash
sudo ./deploy-vps.sh --mode rust --wheel /tmp/aphrody-deploy/aphrody-0.1.0-*.whl \
     --binary /opt/myapp/bin/myapp
```
Installs `aphrody-rust.service` running the binary from `/etc/aphrody/rust.env`
(`APHRODY_RUST_BIN=`), with the same RAM caps + hardening + `Restart=always`.

A common topology: run the Rust API under `aphrody-rust.service` on `:3000`, and
`aphrody serve` the React build on `:8080`, with nginx/Caddy terminating TLS and
routing `/api` → 3000, `/` → 8080.

## One process: static + reverse proxy to a backend
Skip the front proxy entirely — `aphrody serve` can serve the React build **and**
forward an API prefix to a backend (e.g. the Rust app) from a single process:
```bash
aphrody serve /srv/site --proxy http://127.0.0.1:3000 --proxy-prefix /api
# /api/* → the Rust backend; everything else → static/SPA
```
(Plain HTTP forwarding of GET/HEAD/POST/PUT/DELETE/PATCH with `X-Forwarded-*`;
WebSocket upgrades are not proxied — use nginx/Caddy for those.)

## Docker
```bash
docker build -f aphrody/deploy/Dockerfile -t aphrody-serve aphrody
docker run --rm -p 8080:8080 -v /path/to/dist:/srv/site:ro aphrody-serve
# front a Rust backend: append the CMD with --proxy http://backend:3000
```

## RAM / resource tuning
The units set `MemoryHigh=2G` / `MemoryMax=4G` (raise for bigger sites/apps),
`LimitNOFILE=65536`, `Restart=always`. The static server is threaded
(`ThreadingHTTPServer`) and `--cache` trades RAM for latency — exactly what a
VPS with spare memory should do.

## Manage
```bash
systemctl status aphrody        # or aphrody-rust
journalctl -u aphrody -f
systemctl restart aphrody       # after editing /etc/aphrody/serve.env
```

## Cross-platform / paths
- Secrets: `$APHRODY_SECRETS_DIR` → in-repo `var/secrets` → `~/.aphrody` (mode 0600).
- Blender (optional, for the 3D features): `$APHRODY_BLENDER_BIN` → known installs
  (incl. `/usr/bin/blender`) → `blender` on `PATH`.
- No absolute paths are baked into the package; everything is a flag or env var.
