#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# n2b-batch.sh — parallel `aphrody n2b` runner over a target list.
#
# Streams one NDJSON line per completed target on stdout; per-target stdout/stderr
# are captured under --log-dir. Final summary printed on stderr.
set -u
set -o pipefail

usage() {
    cat <<'USAGE'
Usage: n2b-batch.sh [TARGET ...] [options]

Options:
  --from-file <path>     Read targets (one per line, # comments allowed).
  --jobs <N>             Parallel workers (default = nproc, or 4).
  --log-dir <dir>        Per-target log directory (default: var/log/n2b-batch-<ts>).
  --extra-args <str>     Extra args appended to each `aphrody n2b` invocation.
  -h, --help             This help.

Exits 0 on full success, 1 if >=1 target failed, 2 on misuse, 130 on SIGINT.
USAGE
}

targets=()
from_file=""
jobs=""
log_dir=""
extra_args=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help) usage; exit 0 ;;
        --from-file) from_file="${2:-}"; shift 2 ;;
        --jobs) jobs="${2:-}"; shift 2 ;;
        --log-dir) log_dir="${2:-}"; shift 2 ;;
        --extra-args) extra_args="${2:-}"; shift 2 ;;
        --) shift; while [[ $# -gt 0 ]]; do targets+=("$1"); shift; done ;;
        -*) echo "unknown flag: $1" >&2; usage >&2; exit 2 ;;
        *) targets+=("$1"); shift ;;
    esac
done

if [[ -n "$from_file" ]]; then
    [[ -f "$from_file" ]] || { echo "from-file not found: $from_file" >&2; exit 2; }
    while IFS= read -r line; do
        [[ -z "$line" || "${line:0:1}" == "#" ]] && continue
        targets+=("$line")
    done < "$from_file"
fi

if [[ ${#targets[@]} -eq 0 ]]; then
    echo "no targets provided" >&2; usage >&2; exit 2
fi

# Resolve nproc portably (linux nproc / git-bash NUMBER_OF_PROCESSORS / fallback 4).
if [[ -z "$jobs" ]]; then
    if command -v nproc >/dev/null 2>&1; then
        jobs="$(nproc)"
    elif [[ -n "${NUMBER_OF_PROCESSORS:-}" ]]; then
        jobs="$NUMBER_OF_PROCESSORS"
    else
        jobs=4
    fi
fi
[[ "$jobs" =~ ^[0-9]+$ && "$jobs" -gt 0 ]] || { echo "invalid --jobs: $jobs" >&2; exit 2; }

# Resolve repo root = parent of this script's directory.
script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

if [[ -z "$log_dir" ]]; then
    ts="$(date -u +%Y%m%dT%H%M%SZ)"
    log_dir="$repo_root/var/log/n2b-batch-$ts"
fi
mkdir -p "$log_dir"

# Resolve aphrody binary: PATH > release > debug.
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
export APHRODY_BIN LOG_DIR="$log_dir" EXTRA_ARGS="$extra_args"

# Per-target worker: emits one NDJSON line on stdout.
worker() {
    local target="$1"
    local sanitized
    sanitized="$(printf '%s' "$target" | tr -c 'A-Za-z0-9._-' '_' | cut -c1-200)"
    local log="$LOG_DIR/$sanitized.log"
    local code_file="$LOG_DIR/$sanitized.code"
    local start_ns end_ns dur_ms exit_code
    start_ns="$(date +%s%N 2>/dev/null || python -c 'import time;print(int(time.time()*1e9))')"
    # shellcheck disable=SC2086
    "$APHRODY_BIN" n2b $target $EXTRA_ARGS >"$log" 2>&1
    exit_code=$?
    end_ns="$(date +%s%N 2>/dev/null || python -c 'import time;print(int(time.time()*1e9))')"
    dur_ms=$(( (end_ns - start_ns) / 1000000 ))
    printf '%s\n' "$exit_code" >"$code_file"
    # Escape target for JSON (backslash + quote).
    local jtarget
    jtarget="$(printf '%s' "$target" | sed 's/\\/\\\\/g; s/"/\\"/g')"
    local jlog
    jlog="$(printf '%s' "$log" | sed 's/\\/\\\\/g; s/"/\\"/g')"
    printf '{"target":"%s","exit":%d,"duration_ms":%d,"log":"%s"}\n' \
        "$jtarget" "$exit_code" "$dur_ms" "$jlog"
}
export -f worker

trap 'echo "{\"event\":\"sigint\"}"; exit 130' INT

# Drive xargs -P. Use NUL delimiters to survive targets with spaces.
results_file="$(mktemp 2>/dev/null || mktemp -t n2bbatch)"
printf '%s\0' "${targets[@]}" \
    | xargs -0 -P "$jobs" -I {} bash -c 'worker "$@"' _ {} \
    | tee "$results_file"

# Summary on stderr — count + p50/p95 from captured NDJSON lines.
total=0; ok=0; fail=0
durations=()
while IFS= read -r line; do
    total=$((total + 1))
    case "$line" in
        *'"exit":0,'*) ok=$((ok + 1)) ;;
        *) fail=$((fail + 1)) ;;
    esac
    d="${line##*\"duration_ms\":}"; d="${d%%,*}"; d="${d%%\}*}"
    [[ "$d" =~ ^[0-9]+$ ]] && durations+=("$d")
done < "$results_file"
rm -f "$results_file"

# Sort durations numerically for percentile picking.
p50="null"; p95="null"
if [[ ${#durations[@]} -gt 0 ]]; then
    sorted=()
    while IFS= read -r d; do sorted+=("$d"); done < <(printf '%s\n' "${durations[@]}" | sort -n)
    n=${#sorted[@]}
    idx50=$(( (n * 50) / 100 ));   [[ $idx50 -ge $n ]] && idx50=$(( n - 1 ))
    idx95=$(( (n * 95) / 100 ));   [[ $idx95 -ge $n ]] && idx95=$(( n - 1 ))
    p50="${sorted[$idx50]}"
    p95="${sorted[$idx95]}"
fi

printf '{"event":"summary","total":%d,"ok":%d,"fail":%d,"p50_ms":%s,"p95_ms":%s,"log_dir":"%s"}\n' \
    "$total" "$ok" "$fail" "$p50" "$p95" \
    "$(printf '%s' "$log_dir" | sed 's/\\/\\\\/g; s/"/\\"/g')" >&2

[[ $fail -eq 0 ]] || exit 1
exit 0
