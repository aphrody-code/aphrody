#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Publish aphrody crates to crates.io in topological order.
#
# Prerequisites :
#   export CARGO_REGISTRY_TOKEN=<your-crates.io-token>   # from https://crates.io/me
#   OR
#   cargo login                                          # interactive paste
#
# Usage :
#   bash scripts/publish-crates.sh --dry-run    # validate without publishing
#   bash scripts/publish-crates.sh              # apply with 30s sleep between crates
#                                               # (allows index.crates.io to refresh)

set -euo pipefail

DRY_RUN=0
for arg in "$@"; do
    case "$arg" in
        --dry-run|-n) DRY_RUN=1 ;;
    esac
done

if [[ $DRY_RUN -eq 0 ]] && [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
    if [[ ! -f "${CARGO_HOME:-$HOME/.cargo}/credentials.toml" ]]; then
        echo "ERROR : no CARGO_REGISTRY_TOKEN env and no credentials.toml found"
        echo "        get token at https://crates.io/me, then either :"
        echo "          export CARGO_REGISTRY_TOKEN=ci_***"
        echo "        or"
        echo "          cargo login"
        exit 2
    fi
fi

# Topological publish ladder. Each crate must be published before its consumers
# resolve it from index.crates.io. The 30s sleep gives the sparse index time
# to propagate.
LADDER=(
    base
    a2a-pb
    a2a-client-lf
    a2a-server-lf
    a2a-grpc
    backend
    aphrody-translate
    aphrody
)

SUCCESS=0
FAILED=()

for crate in "${LADDER[@]}"; do
    echo ""
    echo "============================================================"
    echo "  $crate"
    echo "============================================================"

    if [[ $DRY_RUN -eq 1 ]]; then
        cmd="cargo package --list -p $crate --offline"
    else
        cmd="cargo publish -p $crate --locked"
    fi

    if env -u RUSTUP_HOME -u CARGO_HOME -u BUN_INSTALL $cmd; then
        SUCCESS=$((SUCCESS + 1))
        if [[ $DRY_RUN -eq 0 ]]; then
            echo "Waiting 30s for index.crates.io to settle..."
            sleep 30
        fi
    else
        echo "FAILED : $crate"
        FAILED+=("$crate")
        # Continue ladder — manual rebuild may be required for the failure,
        # but downstream crates are independent in the publish sense (they
        # only need the dep to be on crates.io eventually).
    fi
done

echo ""
echo "============================================================"
echo "  Summary"
echo "============================================================"
echo "Total : ${#LADDER[@]}"
echo "Pass  : $SUCCESS"
if [[ ${#FAILED[@]} -gt 0 ]]; then
    echo "Fail  : ${#FAILED[@]} (${FAILED[*]})"
    exit 1
fi
echo "All crates published successfully."
