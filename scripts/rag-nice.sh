#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# rag-nice.sh — run a heavy RAG/embedding/training job de-prioritized and
# single-flight, so it exploits spare CPU/IO without starving interactive
# agents on this shared VPS. Canonical wrapper for docs/rag-unified-pattern.md §5.
#
#   usage: scripts/rag-nice.sh <lockname> <cmd> [args...]
#   e.g.:  scripts/rag-nice.sh shenron-ragbuild bun --filter @shenron/bot run rag:build
#          scripts/rag-nice.sh rpbey-vectors   bun apps/web/scripts/build-search-vectors.ts
#
# Behaviour:
#   - single-flight per <lockname> via flock (a second run no-ops, exit 0)
#   - caps ONNX/OpenMP/libuv thread pools to ~1/3 of cores (leaves room for others)
#   - nice -n 15 (CPU) + ionice best-effort prio 7 (IO) so it always yields
set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <lockname> <cmd> [args...]" >&2
  exit 2
fi

LOCKNAME="$1"; shift
LOCKFILE="${TMPDIR:-/tmp}/rag-${LOCKNAME}.lock"

# single-flight: never two heavy RAG jobs of the same name at once.
exec 9>"$LOCKFILE"
if ! flock -n 9; then
  echo "rag-nice: job '${LOCKNAME}' already running (${LOCKFILE}); skipping." >&2
  exit 0
fi

# leave ~1/3 of cores for interactive work; cap the native thread pools.
CORES="$(nproc 2>/dev/null || echo 4)"
THREADS=$(( CORES / 3 ))
[ "$THREADS" -lt 1 ] && THREADS=1
export OMP_NUM_THREADS="$THREADS"
export ORT_NUM_THREADS="$THREADS"   # onnxruntime
export UV_THREADPOOL_SIZE="$THREADS"

echo "rag-nice: '${LOCKNAME}' -> nice 15 / ionice c2n7 / ${THREADS} threads (of ${CORES} cores): $*" >&2

# ionice may be unavailable (containers); degrade to nice-only.
if command -v ionice >/dev/null 2>&1; then
  exec nice -n 15 ionice -c2 -n7 "$@"
else
  exec nice -n 15 "$@"
fi
