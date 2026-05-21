#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
#
# map-google-local.sh — POSIX counterpart of map-google-local.ps1.
#
# Reproducible, non-interactive forensic map of a Google desktop-app install
# tree (e.g. a copy of %LOCALAPPDATA%\Google\Google examined on Linux #1).
# Emits the same JSON shape as the PowerShell script + the Rust
# `aphrody forensics map` path: per file { path, rel, size, ext, sha256,
# modified, mtime_unix, secret_meta_only }.
#
# SECURITY CONTRACT (inviolable):
#   - Secret-looking artefacts (cookies, credentials, token stores,
#     leveldb/sqlite under user-data) are recorded metadata-only — their
#     contents are NEVER opened, so they are never hashed.
#   - SHA-256 is computed ONLY for non-secret files at or below MAX_HASH_BYTES.
#   - Nothing is written outside OUT_DIR. No network access.
#
# Usage:
#   scripts/forensics/map-google-local.sh <target-dir> [out-dir] [max-hash-bytes]
set -euo pipefail

TARGET="${1:?usage: map-google-local.sh <target-dir> [out-dir] [max-hash-bytes]}"
OUT_DIR="${2:-var/data/google-local-map}"
MAX_HASH_BYTES="${3:-1048576}"

[ -d "$TARGET" ] || { echo "target not found: $TARGET" >&2; exit 2; }
mkdir -p "$OUT_DIR"

# Secret denylist (lowercased substrings matched against the full path).
SECRET_RE='cookies|login data|web data|credential|token|local state|leveldb|\.ldb|session storage|local storage|indexeddb|autofill|password|user_history|preferences.txtpb|global_preferences'

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

json_escape() { printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'; }

OUT_FILE="$OUT_DIR/tree.json"
tmp="$(mktemp)"
count=0; hashed=0; secret=0
printf '  "files": [\n' >"$tmp"
first=1

while IFS= read -r -d '' f; do
  count=$((count+1))
  size=$(stat -c '%s' "$f" 2>/dev/null || stat -f '%z' "$f")
  mtime=$(stat -c '%Y' "$f" 2>/dev/null || stat -f '%m' "$f")
  modified=$(date -u -d "@$mtime" '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || date -u -r "$mtime" '+%Y-%m-%dT%H:%M:%SZ')
  base="${f##*/}"; ext=""
  case "$base" in *.*) ext="${base##*.}"; ext="$(printf '%s' "$ext" | tr 'A-Z' 'a-z')";; esac
  rel="${f#"$TARGET"}"; rel="${rel#/}"
  lower="$(printf '%s' "$f" | tr 'A-Z' 'a-z')"
  is_secret=false
  printf '%s' "$lower" | grep -Eq "$SECRET_RE" && is_secret=true
  sha="null"
  if [ "$is_secret" = false ] && [ "$size" -le "$MAX_HASH_BYTES" ]; then
    h="$(sha256_of "$f" 2>/dev/null || true)"; [ -n "$h" ] && { sha="\"$h\""; hashed=$((hashed+1)); }
  fi
  [ "$is_secret" = true ] && secret=$((secret+1))
  ext_json="null"; [ -n "$ext" ] && ext_json="\"$(json_escape "$ext")\""
  [ "$first" = 1 ] && first=0 || printf ',\n' >>"$tmp"
  printf '    {"path": "%s", "rel": "%s", "size": %s, "ext": %s, "sha256": %s, "modified": "%s", "mtime_unix": %s, "secret_meta_only": %s}' \
    "$(json_escape "$f")" "$(json_escape "$rel")" "$size" "$ext_json" "$sha" "$modified" "$mtime" "$is_secret" >>"$tmp"
done < <(find "$TARGET" -type f -print0)

printf '\n  ]\n' >>"$tmp"

{
  printf '{\n'
  printf '  "target": "%s",\n' "$(json_escape "$TARGET")"
  printf '  "generated_at": "%s",\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  printf '  "file_count": %s,\n' "$count"
  printf '  "hashed_count": %s,\n' "$hashed"
  printf '  "secret_meta_only_count": %s,\n' "$secret"
  printf '  "max_hash_bytes": %s,\n' "$MAX_HASH_BYTES"
  cat "$tmp"
  printf '}\n'
} >"$OUT_FILE"
rm -f "$tmp"

echo "wrote $OUT_FILE : $count files, $hashed hashed, $secret secret-meta-only"
