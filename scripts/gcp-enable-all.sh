#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Active TOUTES les APIs Google first-party (*.googleapis.com) sur le projet.
# Lots de 20 ; si un lot échoue (un service non activable), repli service par
# service pour ne pas bloquer les 19 autres. Idempotent. Non-interactif.
set -uo pipefail
PROJECT="${PROJECT:-aphrody}"
LOG="${LOG:-/tmp/gcp-enable-all.log}"
: > "$LOG"

mapfile -t APIS < <(gcloud services list --available --project "$PROJECT" \
    --format='value(config.name)' 2>/dev/null | grep -E '\.googleapis\.com$' | sort -u)
total=${#APIS[@]}
echo "TOTAL=$total" | tee -a "$LOG"

ok=0; failed=0
declare -a FAILED=()

enable_one() {
    if gcloud services enable "$1" --project "$PROJECT" >>"$LOG" 2>&1; then
        ok=$((ok+1))
    else
        failed=$((failed+1)); FAILED+=("$1")
        echo "FAIL $1" | tee -a "$LOG"
    fi
}

for ((i = 0; i < total; i += 20)); do
    batch=("${APIS[@]:i:20}")
    if gcloud services enable "${batch[@]}" --project "$PROJECT" >>"$LOG" 2>&1; then
        ok=$((ok + ${#batch[@]}))
    else
        # repli individuel sur le lot en échec
        for svc in "${batch[@]}"; do enable_one "$svc"; done
    fi
    echo "PROGRESS $((i + ${#batch[@]}))/$total ok=$ok failed=$failed" | tee -a "$LOG"
done

echo "DONE ok=$ok failed=$failed" | tee -a "$LOG"
if ((failed > 0)); then printf 'FAILED_SVC %s\n' "${FAILED[@]}" | tee -a "$LOG"; fi
final=$(gcloud services list --enabled --project "$PROJECT" --format='value(config.name)' 2>/dev/null | wc -l)
echo "ENABLED_NOW=$final" | tee -a "$LOG"
