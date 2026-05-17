#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# bxc-crawl.sh — parallel bxc crawl over URL list with optional loop + recon cache.
#
# Each URL is run against the requested actions (recon, detect, tokens). When
# `recon` is in --actions, the body-hash of the recon response is memoised; on
# subsequent loop iterations URLs whose hash matched the previous tick skip
# detect+tokens (cache hit). NDJSON streamed to stdout, one event per
# URL×action result plus tick summaries.
set -u
set -o pipefail

usage() {
    cat <<'USAGE'
Usage: bxc-crawl.sh [URL ...] [options]

Options:
  --from-file <path>        URLs (one per line, # comments allowed).
  --jobs <N>                Parallel workers (default min(8, CPU)).
  --actions a,b,c           Subset of recon,scrape,detect,tokens (default: recon,detect,tokens).
  --selector <css>          CSS selector used for `scrape` action (default: body).
  --loop                    Re-crawl forever until SIGINT.
  --interval <secs>         Seconds between loop ticks (default 30).
  --port <N>                bxc daemon port (default 8765).
  --no-spawn-daemon         Do not auto-spawn the daemon if ping fails.
  -h, --help                This help.

Exits 0 on full success, 1 if >=1 action failed, 2 on misuse, 130 on SIGINT.
USAGE
}

urls=()
from_file=""
jobs=""
actions="recon,detect,tokens"
selector="body"
loop_mode=0
interval=30
port=8765
spawn_daemon=1

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help) usage; exit 0 ;;
        --from-file) from_file="${2:-}"; shift 2 ;;
        --jobs) jobs="${2:-}"; shift 2 ;;
        --actions) actions="${2:-}"; shift 2 ;;
        --selector) selector="${2:-}"; shift 2 ;;
        --loop) loop_mode=1; shift ;;
        --interval) interval="${2:-}"; shift 2 ;;
        --port) port="${2:-}"; shift 2 ;;
        --no-spawn-daemon) spawn_daemon=0; shift ;;
        --) shift; while [[ $# -gt 0 ]]; do urls+=("$1"); shift; done ;;
        -*) echo "unknown flag: $1" >&2; usage >&2; exit 2 ;;
        *) urls+=("$1"); shift ;;
    esac
done

if [[ -n "$from_file" ]]; then
    [[ -f "$from_file" ]] || { echo "from-file not found: $from_file" >&2; exit 2; }
    while IFS= read -r line; do
        [[ -z "$line" || "${line:0:1}" == "#" ]] && continue
        urls+=("$line")
    done < "$from_file"
fi
[[ ${#urls[@]} -gt 0 ]] || { echo "no URLs provided" >&2; usage >&2; exit 2; }

if [[ -z "$jobs" ]]; then
    if command -v nproc >/dev/null 2>&1; then j="$(nproc)"
    elif [[ -n "${NUMBER_OF_PROCESSORS:-}" ]]; then j="$NUMBER_OF_PROCESSORS"
    else j=4; fi
    jobs=$(( j < 8 ? j : 8 ))
fi
[[ "$jobs" =~ ^[0-9]+$ && "$jobs" -gt 0 ]] || { echo "invalid --jobs: $jobs" >&2; exit 2; }
[[ "$interval" =~ ^[0-9]+$ && "$interval" -gt 0 ]] || { echo "invalid --interval" >&2; exit 2; }
[[ "$port" =~ ^[0-9]+$ ]] || { echo "invalid --port" >&2; exit 2; }

# Validate actions list.
IFS=',' read -r -a action_list <<< "$actions"
for a in "${action_list[@]}"; do
    case "$a" in
        recon|scrape|detect|tokens) ;;
        *) echo "invalid action: $a" >&2; exit 2 ;;
    esac
done

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

ping_daemon() {
    curl -fsS --max-time 2 "http://127.0.0.1:${port}/ping" >/dev/null 2>&1
}

if ! ping_daemon; then
    if [[ $spawn_daemon -eq 1 ]]; then
        printf '{"event":"daemon_spawn","port":%d}\n' "$port"
        "$APHRODY_BIN" bxc daemon --port "$port" >/dev/null 2>&1 &
        # Wait up to 10s for daemon to come up.
        for _ in 1 2 3 4 5 6 7 8 9 10; do
            sleep 1
            ping_daemon && break
        done
        ping_daemon || { echo "daemon failed to start on port $port" >&2; exit 1; }
    else
        echo "daemon not reachable on port $port (use --spawn-daemon to autostart)" >&2
        exit 1
    fi
fi

# Cache dir for recon hashes (per session).
cache_dir="$(mktemp -d 2>/dev/null || mktemp -d -t bxccrawl)"
trap 'rm -rf "$cache_dir"; echo "{\"event\":\"sigint\"}"; exit 130' INT

export APHRODY_BIN CACHE_DIR="$cache_dir" SELECTOR="$selector"

# Hash helper: prefer sha256sum (Linux) then certutil/powershell on Windows.
hash_stdin() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 | awk '{print $1}'
    else
        # Last-resort fallback (cksum: weaker but always present).
        cksum | awk '{print $1"-"$2}'
    fi
}
export -f hash_stdin

# Per (url, action) worker. Input lines: "url\taction\tactions_csv"
worker() {
    local line="$1"
    local url action actions_csv
    url="$(printf '%s' "$line" | awk -F'\t' '{print $1}')"
    action="$(printf '%s' "$line" | awk -F'\t' '{print $2}')"
    actions_csv="$(printf '%s' "$line" | awk -F'\t' '{print $3}')"

    local url_key
    url_key="$(printf '%s' "$url" | tr -c 'A-Za-z0-9._-' '_')"
    local hash_file="$CACHE_DIR/$url_key.hash"
    local prev_hash="" cur_hash=""
    [[ -f "$hash_file" ]] && prev_hash="$(cat "$hash_file")"

    # Cache logic: skip detect/tokens if previous recon hash matches.
    if [[ "$action" != "recon" && "$action" != "scrape" && -n "$prev_hash" ]]; then
        local sentinel="$CACHE_DIR/$url_key.recon_changed"
        if [[ -f "$sentinel" && "$(cat "$sentinel")" == "unchanged" ]]; then
            local jurl
            jurl="$(printf '%s' "$url" | sed 's/\\/\\\\/g; s/"/\\"/g')"
            printf '{"url":"%s","action":"%s","cached":true,"exit":0,"duration_ms":0}\n' "$jurl" "$action"
            return 0
        fi
    fi

    local start_ns end_ns dur_ms exit_code out_b64
    start_ns="$(date +%s%N 2>/dev/null || python -c 'import time;print(int(time.time()*1e9))')"
    local out
    if [[ "$action" == "scrape" ]]; then
        out="$("$APHRODY_BIN" bxc scrape "$url" --selector "$SELECTOR" 2>&1)"
    else
        out="$("$APHRODY_BIN" bxc "$action" "$url" 2>&1)"
    fi
    exit_code=$?
    end_ns="$(date +%s%N 2>/dev/null || python -c 'import time;print(int(time.time()*1e9))')"
    dur_ms=$(( (end_ns - start_ns) / 1000000 ))

    if [[ "$action" == "recon" && $exit_code -eq 0 ]]; then
        cur_hash="$(printf '%s' "$out" | hash_stdin)"
        local sentinel="$CACHE_DIR/$url_key.recon_changed"
        if [[ -n "$prev_hash" && "$cur_hash" == "$prev_hash" ]]; then
            printf 'unchanged' >"$sentinel"
        else
            printf 'changed' >"$sentinel"
        fi
        printf '%s' "$cur_hash" >"$hash_file"
    fi

    # Output size — emit byte length, full payload kept in log only on first 4KB.
    local body_b64
    if command -v base64 >/dev/null 2>&1; then
        body_b64="$(printf '%s' "$out" | head -c 4096 | base64 | tr -d '\n')"
    else
        body_b64=""
    fi
    local jurl jhash
    jurl="$(printf '%s' "$url" | sed 's/\\/\\\\/g; s/"/\\"/g')"
    jhash="${cur_hash:-${prev_hash:-}}"
    printf '{"url":"%s","action":"%s","exit":%d,"duration_ms":%d,"bytes":%d,"hash":"%s","body_head_b64":"%s"}\n' \
        "$jurl" "$action" "$exit_code" "$dur_ms" "${#out}" "$jhash" "$body_b64"
}
export -f worker

run_tick() {
    local tick_idx="$1"
    local tick_start_ns tick_end_ns tick_dur_ms
    tick_start_ns="$(date +%s%N 2>/dev/null || python -c 'import time;print(int(time.time()*1e9))')"
    local results_file
    results_file="$(mktemp 2>/dev/null || mktemp -t bxctick)"

    # Build job list: for each URL, recon first (drives cache), then others.
    local jobs_input=""
    for url in "${urls[@]}"; do
        # Re-order so recon goes first in the action list per URL.
        local ordered=""
        for a in "${action_list[@]}"; do
            [[ "$a" == "recon" ]] && ordered="recon\n$ordered" || ordered="${ordered}${a}\n"
        done
        # Emit each action line for this URL.
        while IFS= read -r a; do
            [[ -z "$a" ]] && continue
            jobs_input+="${url}\t${a}\t${actions}"$'\n'
        done < <(printf '%b' "$ordered")
    done

    printf '%b' "$jobs_input" \
        | xargs -d '\n' -P "$jobs" -I {} bash -c 'worker "$@"' _ {} \
        | tee "$results_file"

    tick_end_ns="$(date +%s%N 2>/dev/null || python -c 'import time;print(int(time.time()*1e9))')"
    tick_dur_ms=$(( (tick_end_ns - tick_start_ns) / 1000000 ))
    local total ok fail cached
    total=$(wc -l < "$results_file" | tr -d ' ')
    ok=$(grep -c '"exit":0' "$results_file" 2>/dev/null || echo 0)
    fail=$(( total - ok ))
    cached=$(grep -c '"cached":true' "$results_file" 2>/dev/null || echo 0)
    rm -f "$results_file"
    local urls_count=${#urls[@]}
    local urls_per_sec="0"
    if [[ $tick_dur_ms -gt 0 ]]; then
        urls_per_sec="$(awk "BEGIN{printf \"%.2f\", $urls_count * 1000 / $tick_dur_ms}")"
    fi
    local cache_ratio="0.00"
    if [[ $total -gt 0 ]]; then
        cache_ratio="$(awk "BEGIN{printf \"%.2f\", $cached / $total}")"
    fi
    printf '{"event":"tick_summary","tick":%d,"urls":%d,"actions":%d,"ok":%d,"fail":%d,"cached":%d,"cache_ratio":%s,"urls_per_sec":%s,"duration_ms":%d}\n' \
        "$tick_idx" "$urls_count" "$total" "$ok" "$fail" "$cached" "$cache_ratio" "$urls_per_sec" "$tick_dur_ms" >&2

    return $fail
}

overall_fail=0
tick=0
if [[ $loop_mode -eq 1 ]]; then
    while true; do
        tick=$(( tick + 1 ))
        run_tick "$tick" || overall_fail=1
        sleep "$interval"
    done
else
    run_tick 1 || overall_fail=1
fi

rm -rf "$cache_dir"
[[ $overall_fail -eq 0 ]] && exit 0 || exit 1
