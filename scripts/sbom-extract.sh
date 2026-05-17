# SPDX-License-Identifier: Apache-2.0
#!/usr/bin/env bash
# =============================================================================
#  aphrody -- SBOM extractor
#
#  Extracts the Software Bill of Materials (SBOM) embedded into a Rust binary
#  by `cargo-auditable`. Wraps `auditable info <binary>` (raw JSON) and emits
#  one of four formats consumable by downstream supply-chain tooling:
#
#    - json       Raw cargo-auditable JSON (schema: rustsec.org/schemas/cargo-auditable.json)
#    - table      Human-readable columns: name | version | kind | source
#    - spdx       SPDX 2.3 JSON-LD (https://spdx.dev/use/specifications/)
#    - cyclonedx  CycloneDX 1.5 JSON (https://cyclonedx.org/specification/)
#
#  Usage:
#    ./scripts/sbom-extract.sh                                    # table from target/release/aphrody
#    ./scripts/sbom-extract.sh target/release/aphrody             # explicit binary
#    ./scripts/sbom-extract.sh --format json                      # raw JSON
#    ./scripts/sbom-extract.sh --format spdx --out aphrody.spdx.json
#    ./scripts/sbom-extract.sh --format cyclonedx --out aphrody.cdx.json
#
#  Exit codes:
#    0  success
#    2  bad args
#    3  `auditable` binary missing
#    4  target binary missing
#    5  binary has no embedded SBOM (built without `cargo auditable build`)
#
#  Requirements (runtime):
#    - bash >= 4
#    - jq   >= 1.6
#    - auditable (https://crates.io/crates/auditable)  -- install: cargo install auditable
# =============================================================================
set -euo pipefail

# ---------------------------------------------------------------------------
# Arg parsing
# ---------------------------------------------------------------------------
BINARY=""
FORMAT="table"
OUT=""

usage() {
    sed -n '3,32p' "$0"
    exit 0
}

while [ $# -gt 0 ]; do
    case "$1" in
        --format)   FORMAT="${2:-}"; shift 2 ;;
        --out)      OUT="${2:-}"; shift 2 ;;
        -h|--help)  usage ;;
        --) shift; break ;;
        -*) echo "unknown flag: $1" >&2; exit 2 ;;
        *)  BINARY="$1"; shift ;;
    esac
done

BINARY="${BINARY:-target/release/aphrody}"

case "$FORMAT" in
    json|table|spdx|cyclonedx) ;;
    *) echo "invalid --format: $FORMAT (want json|table|spdx|cyclonedx)" >&2; exit 2 ;;
esac

# ---------------------------------------------------------------------------
# Tool checks
# ---------------------------------------------------------------------------
if ! command -v auditable >/dev/null 2>&1; then
    cat >&2 <<EOF
error: \`auditable\` is not installed.

Install it once with:

    cargo install auditable

Then re-run:

    $0 ${BINARY} --format ${FORMAT}
EOF
    exit 3
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "error: \`jq\` is required (https://jqlang.github.io/jq/)" >&2
    exit 3
fi

if [ ! -f "$BINARY" ]; then
    echo "error: binary not found: $BINARY" >&2
    echo "hint: build first with \`cargo auditable build --release -p aphrody\`" >&2
    exit 4
fi

# ---------------------------------------------------------------------------
# Extract raw SBOM (cargo-auditable JSON)
# ---------------------------------------------------------------------------
RAW_JSON="$(auditable info "$BINARY" 2>/dev/null || true)"

if [ -z "$RAW_JSON" ] || ! printf '%s' "$RAW_JSON" | jq empty >/dev/null 2>&1; then
    echo "error: no embedded SBOM in $BINARY" >&2
    echo "hint: rebuild with \`cargo auditable build --release\`" >&2
    exit 5
fi

# ---------------------------------------------------------------------------
# Summary stats (stderr, always)
# ---------------------------------------------------------------------------
TOTAL=$(printf '%s' "$RAW_JSON"     | jq '[.packages[]] | length')
RUNTIME=$(printf '%s' "$RAW_JSON"   | jq '[.packages[] | select(.kind == "runtime" or .kind == null)] | length')
BUILD_ONLY=$(printf '%s' "$RAW_JSON" | jq '[.packages[] | select(.kind == "build")] | length')
DEV_ONLY=$(printf '%s' "$RAW_JSON"  | jq '[.packages[] | select(.kind == "dev")]   | length')

emit_summary() {
    {
        echo "----"
        echo "SBOM summary for $BINARY"
        echo "  total deps : $TOTAL"
        echo "  runtime    : $RUNTIME"
        echo "  build-only : $BUILD_ONLY"
        echo "  dev-only   : $DEV_ONLY"
    } >&2
}

# ---------------------------------------------------------------------------
# Format renderers
# ---------------------------------------------------------------------------

# Normalise the `source` field (which is either a string enum or {"Other": "..."}).
SOURCE_NORM='
def src_str:
  if type == "string" then .
  elif type == "object" then (.Other // (to_entries[0].value | tostring))
  else "unknown" end;
'

render_json() {
    printf '%s' "$RAW_JSON" | jq '.'
}

render_table() {
    # Pretty columns via jq -> column-friendly TSV -> column(1).
    # Fall back to plain tab-separated if column(1) is missing (POSIX Windows).
    local tsv
    tsv="$(printf '%s' "$RAW_JSON" | jq -r "
        $SOURCE_NORM
        [\"NAME\",\"VERSION\",\"KIND\",\"SOURCE\"],
        ( .packages
          | sort_by(.name)
          | .[]
          | [ .name,
              .version,
              (.kind // \"runtime\"),
              (.source | src_str) ] )
        | @tsv
    ")"
    if command -v column >/dev/null 2>&1; then
        printf '%s\n' "$tsv" | column -t -s $'\t'
    else
        printf '%s\n' "$tsv"
    fi
}

render_spdx() {
    # SPDX 2.3 JSON-LD. Minimal-but-valid: spdxVersion + dataLicense +
    # SPDXID + name + documentNamespace + creationInfo + packages[].
    # We synthesize SPDXIDs as SPDXRef-Package-<idx> and PURLs as
    # `pkg:cargo/<name>@<version>` when source is CratesIo, else `pkg:generic/...`.
    local now
    now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    local bname
    bname="$(basename "$BINARY")"
    printf '%s' "$RAW_JSON" | jq --arg now "$now" --arg bname "$bname" --arg bpath "$BINARY" "
        $SOURCE_NORM
        def purl:
          if (.source | src_str) == \"CratesIo\"
            then \"pkg:cargo/\" + .name + \"@\" + .version
          elif (.source | src_str) == \"Git\"
            then \"pkg:generic/\" + .name + \"@\" + .version + \"?vcs_url=git\"
          elif (.source | src_str) == \"Local\"
            then \"pkg:generic/\" + .name + \"@\" + .version + \"?source=local\"
          else  \"pkg:generic/\" + .name + \"@\" + .version
          end;
        {
          spdxVersion: \"SPDX-2.3\",
          dataLicense: \"CC0-1.0\",
          SPDXID: \"SPDXRef-DOCUMENT\",
          name: (\"aphrody-sbom-\" + \$bname),
          documentNamespace: (\"https://spdx.org/spdxdocs/aphrody/\" + \$bname + \"-\" + \$now),
          creationInfo: {
            created: \$now,
            creators: [ \"Tool: cargo-auditable\", \"Tool: aphrody-sbom-extract\" ],
            licenseListVersion: \"3.23\"
          },
          packages: ( .packages
            | to_entries
            | map({
                SPDXID: (\"SPDXRef-Package-\" + (.key | tostring)),
                name: .value.name,
                versionInfo: .value.version,
                downloadLocation: \"NOASSERTION\",
                filesAnalyzed: false,
                licenseConcluded: \"NOASSERTION\",
                licenseDeclared: \"NOASSERTION\",
                copyrightText: \"NOASSERTION\",
                supplier: \"NOASSERTION\",
                primaryPackagePurpose: ( if (.value.kind // \"runtime\") == \"build\"
                                           then \"OPERATING-SYSTEM\"
                                           else \"LIBRARY\" end ),
                externalRefs: [{
                  referenceCategory: \"PACKAGE-MANAGER\",
                  referenceType: \"purl\",
                  referenceLocator: (.value | purl)
                }]
              })
          ),
          documentDescribes: [ \"SPDXRef-Package-0\" ]
        }
    "
}

render_cyclonedx() {
    # CycloneDX 1.5 JSON. bomFormat + specVersion are mandatory; we add
    # serialNumber, version, metadata.timestamp, and components[] for full
    # auditor-grade output. bom-ref = `pkg:cargo/<name>@<version>` (matches PURL).
    local now uuid
    now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    if command -v uuidgen >/dev/null 2>&1; then
        uuid="urn:uuid:$(uuidgen | tr 'A-Z' 'a-z')"
    else
        # Fallback: deterministic-ish UUIDv4 from /dev/urandom + jq.
        uuid="urn:uuid:$(printf '%s' "$RAW_JSON$now" \
            | jq -Rsr '@base64' \
            | tr -dc 'a-f0-9' \
            | head -c 32 \
            | sed -E 's/(.{8})(.{4})(.{4})(.{4})(.{12}).*/\1-\2-\3-\4-\5/')"
    fi
    local bname
    bname="$(basename "$BINARY")"
    printf '%s' "$RAW_JSON" | jq --arg now "$now" --arg uuid "$uuid" --arg bname "$bname" "
        $SOURCE_NORM
        def purl:
          if (.source | src_str) == \"CratesIo\"
            then \"pkg:cargo/\" + .name + \"@\" + .version
          else  \"pkg:generic/\" + .name + \"@\" + .version
          end;
        {
          bomFormat: \"CycloneDX\",
          specVersion: \"1.5\",
          serialNumber: \$uuid,
          version: 1,
          metadata: {
            timestamp: \$now,
            tools: [{
              vendor: \"aphrody\",
              name: \"sbom-extract\",
              version: \"1.0.0\"
            }, {
              vendor: \"rust-secure-code\",
              name: \"cargo-auditable\"
            }],
            component: {
              type: \"application\",
              name: \$bname,
              \"bom-ref\": (\"binary:\" + \$bname)
            }
          },
          components: ( .packages
            | map(select((.root // false) | not))
            | map({
                type: \"library\",
                \"bom-ref\": purl,
                name: .name,
                version: .version,
                purl: purl,
                scope: ( if (.kind // \"runtime\") == \"build\"
                           then \"excluded\"
                         elif (.kind // \"runtime\") == \"runtime\"
                           then \"required\"
                         else  \"optional\" end ),
                properties: [
                  { name: \"cargo:source\", value: (.source | src_str) },
                  { name: \"cargo:kind\",   value: (.kind // \"runtime\") }
                ]
              })
          )
        }
    "
}

# ---------------------------------------------------------------------------
# Dispatch
# ---------------------------------------------------------------------------
render() {
    case "$FORMAT" in
        json)      render_json ;;
        table)     render_table ;;
        spdx)      render_spdx ;;
        cyclonedx) render_cyclonedx ;;
    esac
}

if [ -n "$OUT" ]; then
    # Sanity-check JSON formats parse before writing.
    case "$FORMAT" in
        json|spdx|cyclonedx)
            render | jq empty >/dev/null
            render > "$OUT"
            ;;
        *)
            render > "$OUT"
            ;;
    esac
    echo "wrote $OUT ($FORMAT, $TOTAL packages)" >&2
else
    render
fi

emit_summary
