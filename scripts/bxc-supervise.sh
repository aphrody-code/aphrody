#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# bxc-supervise.sh — watchdog for the `aphrody bxc daemon` process.
#
# Polls GET /ping on --port every --health-interval seconds; on failure spawns
# the daemon after --restart-cooldown seconds and bumps a restart counter.
# Emits one NDJSON heartbeat per tick on stdout.
set -u
set -o pipefail

usage() {
    cat <<'USAGE'
Usage: bxc-supervise.sh [options]

Options:
  --port <N>                 Daemon port (default 8765).
  --health-interval <secs>   Poll interval (default 5).
  --restart-cooldown <secs>  Sleep after a respawn (default 10).
  --max-restarts <N>         Hard cap on restarts (default unlimited).
  --no-spawn                 Do not auto-spawn on first failure; just report.
  -h, --help                 This help.

Exits 0 on clean shutdown, 1 on max-restarts exceeded, 2 on misuse, 130 on SIGINT.
USAGE
}

port=8765
health_interval=5
restart_cooldown=10
max_restarts=-1
no_spawn=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help) usage; exit 0 ;;
        --port) port="${2:-}"; shift 2 ;;
        --health-interval) health_interval="${2:-}"; shift 2 ;;
        --restart-cooldown) restart_cooldown="${2:-}"; shift 2 ;;
        --max-restarts) max_restarts="${2:-}"; shift 2 ;;
        --no-spawn) no_spawn=1; shift ;;
        *) echo "unknown flag: $1" >&2; usage >&2; exit 2 ;;
    esac
done

for v in "$port" "$health_interval" "$restart_cooldown"; do
    [[ "$v" =~ ^[0-9]+$ ]] || { echo "invalid numeric arg: $v" >&2; exit 2; }
done
[[ "$max_restarts" =~ ^-?[0-9]+$ ]] || { echo "invalid --max-restarts" >&2; exit 2; }

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

resolve_aphrody() {
    if command -v aphrody >/dev/null 2>&1; then command -v aphrody; return; fi
    for cand in "$repo_root/target/release/aphrody.exe" "$repo_root/target/release/aphrody" \
                "$repo_root/target/debug/aphrody.exe"   "$repo_root/target/debug/aphrody"; do
        [[ -x "$cand" ]] && { echo "$cand"; return; }
    done
    return 1
}
APHRODY_BIN="$(resolve_aphrody)" || {
    echo "aphrody binary not found in PATH or target/{release,debug}" >&2
    exit 2
}

iso_now() { date -u +%Y-%m-%dT%H:%M:%SZ; }

ping_daemon() {
    curl -fsS --max-time 2 "http://127.0.0.1:${port}/ping" >/dev/null 2>&1
}

pid_file_candidates=(
    "$repo_root/var/run/bxc.pid"
)
read_pid() {
    for f in "${pid_file_candidates[@]}"; do
        if [[ -f "$f" ]]; then
            local p
            p="$(tr -d '[:space:]' < "$f")"
            [[ "$p" =~ ^[0-9]+$ ]] && { echo "$p"; return; }
        fi
    done
    echo "0"
}

emit_event() {
    local status="$1" pid="$2" uptime="$3" restarts="$4"
    printf '{"ts":"%s","status":"%s","pid":%s,"uptime_s":%s,"restart_count":%s,"port":%d}\n' \
        "$(iso_now)" "$status" "$pid" "$uptime" "$restarts" "$port"
}

shutdown() {
    emit_event "shutdown" "$(read_pid)" "$(( EPOCHSECONDS - started_at ))" "$restart_count" >&2
    exit 130
}
trap shutdown INT TERM

restart_count=0
started_at="$EPOCHSECONDS"
# EPOCHSECONDS is bash 5+; fall back to date.
if [[ -z "${started_at:-}" || "$started_at" == "" ]]; then
    started_at="$(date +%s)"
fi

last_up_ts=0
while true; do
    if ping_daemon; then
        if [[ $last_up_ts -eq 0 ]]; then last_up_ts="$(date +%s)"; fi
        uptime=$(( $(date +%s) - last_up_ts ))
        emit_event "up" "$(read_pid)" "$uptime" "$restart_count"
    else
        last_up_ts=0
        emit_event "down" "$(read_pid)" "0" "$restart_count"
        if [[ $no_spawn -eq 1 ]]; then
            sleep "$health_interval"
            continue
        fi
        if [[ $max_restarts -ge 0 && $restart_count -ge $max_restarts ]]; then
            emit_event "max_restarts_exceeded" "0" "0" "$restart_count" >&2
            exit 1
        fi
        emit_event "restarting" "0" "0" "$restart_count"
        "$APHRODY_BIN" bxc daemon --port "$port" >/dev/null 2>&1 &
        restart_count=$(( restart_count + 1 ))
        sleep "$restart_cooldown"
    fi
    sleep "$health_interval"
done
