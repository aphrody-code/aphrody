#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Fill __SHA256_*__ placeholders in Formula/aphrody.rb of the homebrew-tap
# from release artifacts published by .github/workflows/release.yml.
#
# Usage :
#   TAP_DIR=/tmp/homebrew-tap bash scripts/fill-homebrew-shas.sh v1.0.0-canary
#   bash scripts/fill-homebrew-shas.sh v1.0.0-canary           # auto-clones tap to /tmp

set -euo pipefail

TAG="${1:?usage: $0 <tag>}"
TAP_DIR="${TAP_DIR:-/tmp/homebrew-tap}"
REPO="aphrody-code/aphrody"
TAP_REPO="aphrody-code/homebrew-tap"

# Clone tap if not present
if [[ ! -d "$TAP_DIR/.git" ]]; then
    echo "Cloning tap to $TAP_DIR"
    git clone "https://github.com/$TAP_REPO.git" "$TAP_DIR"
fi

cd "$TAP_DIR"
git pull --rebase --quiet origin main

# Download per-target sha256 sidecars (release.yml uploads them next to the .tar.gz / .zip)
TMP="$(mktemp -d)"
trap "rm -rf $TMP" EXIT

declare -A SHA_MAP=(
    [DARWIN_ARM64]="aarch64-apple-darwin.tar.gz"
    [DARWIN_X64]="x86_64-apple-darwin.tar.gz"
    [LINUX_ARM64]="aarch64-unknown-linux-gnu.tar.gz"
    [LINUX_X64]="x86_64-unknown-linux-gnu.tar.gz"
)

for placeholder in "${!SHA_MAP[@]}"; do
    asset_name="aphrody-${TAG}-${SHA_MAP[$placeholder]}.sha256"
    echo "Fetching $asset_name"
    if gh release download "$TAG" --repo "$REPO" --pattern "$asset_name" --dir "$TMP" 2>/dev/null; then
        sha=$(awk '{print $1}' "$TMP/$asset_name")
        echo "  $placeholder = $sha"
        sed -i.bak "s|__SHA256_${placeholder}__|$sha|g" Formula/aphrody.rb
        rm -f Formula/aphrody.rb.bak
    else
        echo "  WARN : $asset_name not yet published (skipping)"
    fi
done

# Bump version in formula
sed -i.bak "s|version \".*\"|version \"${TAG#v}\"|" Formula/aphrody.rb
rm -f Formula/aphrody.rb.bak

if git diff --quiet Formula/aphrody.rb; then
    echo "No changes (already up-to-date)."
    exit 0
fi

git add Formula/aphrody.rb
git -c user.email=noreply@aphrody.dev -c user.name='aphrody-code' commit -q -m "bump: $TAG (auto-filled SHAs from release)"
git push origin main
echo "Pushed $TAP_REPO with $TAG SHAs."
