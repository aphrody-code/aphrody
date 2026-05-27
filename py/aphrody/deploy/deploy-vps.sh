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

# Calculate dynamic memory limits based on available physical memory (for large servers like 40 GB RAM)
MEM_HIGH="2G"
MEM_MAX="4G"
if [[ -f /proc/meminfo ]]; then
  TOTAL_RAM_KB=$(grep MemTotal /proc/meminfo | awk '{print $2}')
  TOTAL_RAM_GB=$((TOTAL_RAM_KB / 1024 / 1024))
  if [[ $TOTAL_RAM_GB -gt 8 ]]; then
    # Scale: High = 75% of total RAM, Max = 90% of total RAM
    MEM_HIGH="$((TOTAL_RAM_GB * 75 / 100))G"
    MEM_MAX="$((TOTAL_RAM_GB * 90 / 100))G"
  fi
fi

echo "==> Configuring systemd memory limits: High=${MEM_HIGH}, Max=${MEM_MAX}"

# Locate uv for high-performance Python environment management
UV_BIN=""
if command -v uv &>/dev/null; then
  UV_BIN=$(command -v uv)
elif [[ -x "/home/ubuntu/.local/bin/uv" ]]; then
  UV_BIN="/home/ubuntu/.local/bin/uv"
elif [[ -x "/root/.local/bin/uv" ]]; then
  UV_BIN="/root/.local/bin/uv"
fi

[[ -n "$WHEEL" ]] || { echo "provide --wheel <path-to-aphrody-*.whl>" >&2; exit 1; }

if [[ -n "$UV_BIN" ]]; then
  echo "==> High-performance Python venv + install via uv ($UV_BIN)"
  # Setup uv cache directory to leverage host caching
  export UV_CACHE_DIR="${UV_CACHE_DIR:-/opt/aphrody/.uv-cache}"
  mkdir -p "$UV_CACHE_DIR"
  chown -R aphrody:aphrody "$UV_CACHE_DIR" || true
  
  $UV_BIN venv "$APP/venv" --allow-existing
  $UV_BIN pip install --python "$APP/venv/bin/python" --quiet "$WHEEL"
else
  echo "==> Python venv + install (uv not found, falling back to standard pip)"
  python3 -m venv "$APP/venv"
  "$APP/venv/bin/pip" install --quiet --upgrade pip
  "$APP/venv/bin/pip" install --quiet "$WHEEL"
fi

case "$MODE" in
  react)
    echo "==> React static/SPA mode (root=$SITE host=$HOST port=$PORT)"
    cat > "$ETC/serve.env" <<EOF
APHRODY_SITE_ROOT=$SITE
APHRODY_HOST=$HOST
APHRODY_PORT=$PORT
EOF
    # Inject dynamic memory limits into systemd service unit
    sed -e "s/MemoryHigh=2G/MemoryHigh=${MEM_HIGH}/" \
        -e "s/MemoryMax=4G/MemoryMax=${MEM_MAX}/" \
        "$HERE/aphrody.service" > /tmp/aphrody.service
    install -m 0644 /tmp/aphrody.service /etc/systemd/system/aphrody.service
    rm -f /tmp/aphrody.service
    UNIT=aphrody.service
    ;;
  rust)
    [[ -n "$BINARY" ]] || { echo "rust mode needs --binary <path>" >&2; exit 1; }
    echo "==> Rust app mode (binary=$BINARY)"
    echo "APHRODY_RUST_BIN=$BINARY" > "$ETC/rust.env"
    # Inject dynamic memory limits into systemd service unit
    sed -e "s/MemoryHigh=2G/MemoryHigh=${MEM_HIGH}/" \
        -e "s/MemoryMax=4G/MemoryMax=${MEM_MAX}/" \
        "$HERE/aphrody-rust.service" > /tmp/aphrody-rust.service
    install -m 0644 /tmp/aphrody-rust.service /etc/systemd/system/aphrody-rust.service
    rm -f /tmp/aphrody-rust.service
    UNIT=aphrody-rust.service
    ;;
  *) echo "bad --mode: $MODE (react|rust)" >&2; exit 2 ;;
esac

chown -R aphrody:aphrody "$APP" "$LOG"
systemctl daemon-reload
systemctl enable --now "$UNIT"
echo "==> Deployed $UNIT"
systemctl --no-pager --lines=0 status "$UNIT" || true
