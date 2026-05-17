# SPDX-License-Identifier: Apache-2.0
#!/usr/bin/env bash
# =============================================================================
#  aphrody -- changelog preview helper
#
#  Prints Conventional Commits since the last `v*` tag (or since a given ref
#  / date), grouped by type. Pure preview tool: never edits the changelog
#  file (that file is owned by release-please).
#
#  Usage:
#    ./scripts/changelog-since.sh                          # since last v* tag
#    ./scripts/changelog-since.sh --since v1.2.3           # since explicit ref
#    ./scripts/changelog-since.sh --since-date 2026-05-01  # since date
#    ./scripts/changelog-since.sh --out preview.md         # write to file
#    ./scripts/changelog-since.sh --text                   # plain (no markdown)
#
#  Output groups (Conventional Commits 1.0.0):
#    ## Added (feat)
#    ## Fixed (fix)
#    ## Docs (docs)
#    ## Perf (perf)
#    ## Reverts (revert)
#    ## Internal      (chore, build, ci, refactor, test, style)
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# ----------------------------------------------------------------------------
# Arg parsing
# ----------------------------------------------------------------------------
SINCE=""
SINCE_DATE=""
OUT=""
FORMAT="markdown"   # markdown | text
USE_LAST_TAG=1

usage() {
    sed -n '3,24p' "$0"
    exit 0
}

while [ $# -gt 0 ]; do
    case "$1" in
        --since)        SINCE="${2:-}"; USE_LAST_TAG=0; shift 2 ;;
        --since-tag)    USE_LAST_TAG=1; shift ;;
        --since-date)   SINCE_DATE="${2:-}"; USE_LAST_TAG=0; shift 2 ;;
        --out)          OUT="${2:-}"; shift 2 ;;
        --markdown)     FORMAT="markdown"; shift ;;
        --text)         FORMAT="text"; shift ;;
        -h|--help)      usage ;;
        *)              echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

# ----------------------------------------------------------------------------
# Resolve <since> ref
# ----------------------------------------------------------------------------
RANGE_LABEL=""
if [ -n "$SINCE_DATE" ]; then
    # Date-based selection — git log accepts --since="YYYY-MM-DD".
    RANGE_LABEL="since $SINCE_DATE"
    COMMITS_RAW="$(git log --oneline --no-merges --since="$SINCE_DATE" HEAD)"
elif [ -n "$SINCE" ]; then
    RANGE_LABEL="$SINCE..HEAD"
    COMMITS_RAW="$(git log --oneline --no-merges "$SINCE..HEAD")"
elif [ "$USE_LAST_TAG" = "1" ]; then
    LAST_TAG="$(git tag --list 'v*' --sort=-version:refname | head -n 1 || true)"
    if [ -n "$LAST_TAG" ]; then
        SINCE="$LAST_TAG"
        RANGE_LABEL="$LAST_TAG..HEAD"
        COMMITS_RAW="$(git log --oneline --no-merges "$LAST_TAG..HEAD")"
    else
        SINCE="HEAD~50"
        RANGE_LABEL="HEAD~50..HEAD (no v* tag found)"
        COMMITS_RAW="$(git log --oneline --no-merges -n 50 HEAD)"
    fi
fi

# ----------------------------------------------------------------------------
# Filter to Conventional Commits and bucket by type
# ----------------------------------------------------------------------------
CC_RE='^[a-f0-9]+ (feat|fix|docs|refactor|test|chore|build|perf|style|revert|ci)(\([^)]*\))?(!)?:'

FILTERED="$(printf '%s\n' "$COMMITS_RAW" | grep -E "$CC_RE" || true)"

bucket_for() {
    case "$1" in
        feat)     echo "Added (feat)" ;;
        fix)      echo "Fixed (fix)" ;;
        docs)     echo "Docs (docs)" ;;
        perf)     echo "Perf (perf)" ;;
        revert)   echo "Reverts (revert)" ;;
        chore|build|ci|refactor|test|style) echo "Internal" ;;
        *)        echo "Other" ;;
    esac
}

# Bucket order (insertion order preserved).
BUCKET_ORDER="Added (feat)|Fixed (fix)|Docs (docs)|Perf (perf)|Reverts (revert)|Internal"

# tmpdir for per-bucket files (sort/group without jq/python).
TMPDIR_RUN="$(mktemp -d 2>/dev/null || mktemp -d -t changelog-since)"
trap 'rm -rf "$TMPDIR_RUN"' EXIT

TOTAL=0
FEAT_COUNT=0     # "FAIT" = feat + fix + perf + revert (user-facing)
HYGIENE_COUNT=0  # chore + build + ci + refactor + test + style + docs

if [ -n "$FILTERED" ]; then
    while IFS= read -r line; do
        [ -z "$line" ] && continue
        TOTAL=$((TOTAL + 1))
        # Extract `type` (strip optional scope + `!`).
        type_only="$(printf '%s' "$line" | sed -E 's/^[a-f0-9]+ ([a-z]+)(\([^)]*\))?(!)?:.*$/\1/')"
        bucket="$(bucket_for "$type_only")"
        # File-safe bucket key.
        bucket_key="$(printf '%s' "$bucket" | tr ' ()' '___')"
        printf '%s\n' "$line" >> "$TMPDIR_RUN/$bucket_key.lst"

        case "$type_only" in
            feat|fix|perf|revert)
                FEAT_COUNT=$((FEAT_COUNT + 1)) ;;
            chore|build|ci|refactor|test|style|docs)
                HYGIENE_COUNT=$((HYGIENE_COUNT + 1)) ;;
        esac
    done <<EOF
$FILTERED
EOF
fi

# ----------------------------------------------------------------------------
# Render output (markdown or plain text)
# ----------------------------------------------------------------------------
render() {
    if [ "$FORMAT" = "markdown" ]; then
        echo "# Changelog preview ($RANGE_LABEL)"
        echo ""
    else
        echo "Changelog preview ($RANGE_LABEL)"
        echo "================================"
        echo ""
    fi

    IFS='|' read -r -a buckets <<< "$BUCKET_ORDER"
    for bucket in "${buckets[@]}"; do
        bucket_key="$(printf '%s' "$bucket" | tr ' ()' '___')"
        bucket_file="$TMPDIR_RUN/$bucket_key.lst"
        [ -f "$bucket_file" ] || continue

        if [ "$FORMAT" = "markdown" ]; then
            echo "## $bucket"
        else
            echo "$bucket"
            printf '%*s\n' "${#bucket}" '' | tr ' ' '-'
        fi

        # Sort by SHA for deterministic preview.
        while IFS= read -r entry; do
            sha="${entry%% *}"
            subject="${entry#* }"
            if [ "$FORMAT" = "markdown" ]; then
                printf -- '- %s %s\n' "$sha" "$subject"
            else
                printf -- '  - %s %s\n' "$sha" "$subject"
            fi
        done < <(sort -u "$bucket_file")

        echo ""
    done

    # Summary line (always last).
    echo "----"
    echo "$TOTAL commits found since $RANGE_LABEL, $FEAT_COUNT FAIT, $HYGIENE_COUNT hygiene"
}

if [ -n "$OUT" ]; then
    render > "$OUT"
    echo "wrote $OUT ($TOTAL commits, $FEAT_COUNT FAIT, $HYGIENE_COUNT hygiene)" >&2
else
    render
fi
