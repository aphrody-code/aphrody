#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# aphrody — one-line installer for Linux + macOS.
#
# Canonical (recommended, once a release is published):
#   curl -fsSL https://github.com/aphrody-code/aphrody/releases/latest/download/install.sh | bash
#
# Pre-release / main-branch fallback (always works, no release required):
#   curl -fsSL https://raw.githubusercontent.com/aphrody-code/aphrody/main/packaging/install.sh | bash
#
# Pin a specific version (skips the GitHub Releases API roundtrip):
#   curl -fsSL https://github.com/aphrody-code/aphrody/releases/latest/download/install.sh | APHRODY_VERSION=v0.2.0 bash
#
# What it does:
#   1. Detects OS (linux/darwin) + arch (x86_64/aarch64).
#   2. Resolves the latest tag via the GitHub Releases API (or honours
#      $APHRODY_VERSION if set).
#   3. Downloads the matching tar.gz + .sha256 from the GitHub Release.
#   4. Verifies the SHA-256 (shasum -a 256), aborts on mismatch.
#   5. Extracts to a temp dir and installs `aphrody` into $APHRODY_BIN
#      (default: $HOME/.local/bin).
#   6. Hints the user to add $APHRODY_BIN to PATH if it is not already there.
#      Never modifies the user's shell rc files automatically (idempotent).
#
# The script NEVER invokes the installed binary; it only stages it.
# Requires bash (for `set -o pipefail`). Tested with bash 4 and 5.

set -euo pipefail

OWNER="aphrody-code"
REPO="aphrody"
BIN="aphrody"
BIN_DIR="${APHRODY_BIN:-$HOME/.local/bin}"

die() {
    printf '\033[31merror:\033[0m %s\n' "$1" >&2
    exit 1
}

need() {
    command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

need curl
need tar
# shasum is POSIX-ish; sha256sum is Linux-only; openssl ships everywhere.
if command -v shasum >/dev/null 2>&1; then
    SHA="shasum -a 256"
elif command -v sha256sum >/dev/null 2>&1; then
    SHA="sha256sum"
elif command -v openssl >/dev/null 2>&1; then
    SHA="openssl dgst -sha256 -r"
else
    die "no SHA-256 tool found (shasum / sha256sum / openssl)"
fi

# ---- OS + arch detection --------------------------------------------------
UNAME_S=$(uname -s)
UNAME_M=$(uname -m)
case "$UNAME_S" in
    Linux)  OS_PART="unknown-linux-gnu" ;;
    Darwin) OS_PART="apple-darwin" ;;
    *)      die "unsupported OS: $UNAME_S (try install.ps1 on Windows)" ;;
esac
case "$UNAME_M" in
    x86_64|amd64)  ARCH_PART="x86_64" ;;
    aarch64|arm64) ARCH_PART="aarch64" ;;
    *)             die "unsupported architecture: $UNAME_M" ;;
esac
TARGET="${ARCH_PART}-${OS_PART}"

# ---- Resolve version ------------------------------------------------------
if [ -n "${APHRODY_VERSION:-}" ]; then
    TAG="$APHRODY_VERSION"
else
    printf 'Resolving latest aphrody release...\n'
    TAG=$(curl -sSfL "https://api.github.com/repos/$OWNER/$REPO/releases/latest" \
        | grep '"tag_name":' | head -n1 | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')
    [ -n "$TAG" ] || die "could not resolve latest release tag"
fi
VERSION="${TAG#v}"
printf 'Installing aphrody %s (%s)\n' "$VERSION" "$TARGET"

# ---- Download + verify ----------------------------------------------------
ARCHIVE="aphrody-${TAG}-${TARGET}.tar.gz"
URL="https://github.com/$OWNER/$REPO/releases/download/$TAG/$ARCHIVE"
TMP=$(mktemp -d 2>/dev/null || mktemp -d -t aphrody)
# shellcheck disable=SC2064
trap "rm -rf '$TMP'" EXIT

curl -sSfL "$URL" -o "$TMP/$ARCHIVE" \
    || die "download failed: $URL"
curl -sSfL "$URL.sha256" -o "$TMP/$ARCHIVE.sha256" \
    || die "download failed: $URL.sha256"

EXPECTED=$(awk '{print $1}' < "$TMP/$ARCHIVE.sha256")
ACTUAL=$($SHA "$TMP/$ARCHIVE" | awk '{print $1}')
[ "$EXPECTED" = "$ACTUAL" ] || die "SHA-256 mismatch (expected $EXPECTED, got $ACTUAL)"
printf 'SHA-256 verified.\n'

# ---- Install --------------------------------------------------------------
tar -xzf "$TMP/$ARCHIVE" -C "$TMP" \
    || die "tar extraction failed"
STAGE_DIR="$TMP/aphrody-${TAG}-${TARGET}"
[ -f "$STAGE_DIR/$BIN" ] || die "binary not found in archive (expected $STAGE_DIR/$BIN)"

mkdir -p "$BIN_DIR"
install -m 0755 "$STAGE_DIR/$BIN" "$BIN_DIR/$BIN"
printf 'Installed: %s/%s\n' "$BIN_DIR" "$BIN"

# ---- PATH hint ------------------------------------------------------------
case ":$PATH:" in
    *":$BIN_DIR:"*)
        printf 'Run: aphrody --version\n'
        ;;
    *)
        printf '\nAdd %s to your PATH:\n' "$BIN_DIR"
        printf '  echo '\''export PATH="%s:$PATH"'\'' >> ~/.profile\n' "$BIN_DIR"
        ;;
esac
