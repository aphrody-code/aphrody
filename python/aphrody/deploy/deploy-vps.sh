#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Deploy aphrody on a Linux VPS under systemd. One-shot bootstrap (run as root).
#
#   React (serve a built dist):
#     sudo ./deploy-vps.sh --mode react --wheel aphrody-0.1.0-*.whl \
#          --site /srv/site --host 0.0.0.0 --port 8080
#   Rust app (run a binary under systemd):
#     sudo ./deploy-vps.sh --mode rust  --wheel aphrody-0.1.0-*.whl \
#          --binary /opt/myapp/bin/myapp
set -euo pipefail

MODE=react SITE=/srv/site BINARY="" HOST=0.0.0.0 PORT=8080 WHEEL=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)   MODE="$2";   shift 2 ;;
    --site)   SITE="$2";   shift 2 ;;
    --binary) BINARY="$2"; shift 2 ;;
    --host)   HOST="$2";   shift 2 ;;
    --port)   PORT="$2";   shift 2 ;;
    --wheel)  WHEEL="$2";  shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

[[ $EUID -eq 0 ]] || { echo "run as root (sudo)" >&2; exit 1; }
HERE="$(cd "$(dirname "$0")" && pwd)"
APP=/opt/aphrody ETC=/etc/aphrody LOG=/var/log/aphrody

id aphrody &>/dev/null || useradd --system --home-dir "$APP" --shell /usr/sbin/nologin aphrody
mkdir -p "$APP" "$ETC" "$LOG"

echo "==> Python venv + install"
python3 -m venv "$APP/venv"
"$APP/venv/bin/pip" install --quiet --upgrade pip
[[ -n "$WHEEL" ]] || { echo "provide --wheel <path-to-aphrody-*.whl>" >&2; exit 1; }
"$APP/venv/bin/pip" install --quiet "$WHEEL"

case "$MODE" in
  react)
    echo "==> React static/SPA mode (root=$SITE host=$HOST port=$PORT)"
    cat > "$ETC/serve.env" <<EOF
APHRODY_SITE_ROOT=$SITE
APHRODY_HOST=$HOST
APHRODY_PORT=$PORT
EOF
    install -m 0644 "$HERE/aphrody.service" /etc/systemd/system/aphrody.service
    UNIT=aphrody.service
    ;;
  rust)
    [[ -n "$BINARY" ]] || { echo "rust mode needs --binary <path>" >&2; exit 1; }
    echo "==> Rust app mode (binary=$BINARY)"
    echo "APHRODY_RUST_BIN=$BINARY" > "$ETC/rust.env"
    install -m 0644 "$HERE/aphrody-rust.service" /etc/systemd/system/aphrody-rust.service
    UNIT=aphrody-rust.service
    ;;
  *) echo "bad --mode: $MODE (react|rust)" >&2; exit 2 ;;
esac

chown -R aphrody:aphrody "$APP" "$LOG"
systemctl daemon-reload
systemctl enable --now "$UNIT"
echo "==> Deployed $UNIT"
systemctl --no-pager --lines=0 status "$UNIT" || true
