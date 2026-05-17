#!/usr/bin/env bash
# Install the exact wasm-bindgen-cli version pinned by the workspace.
# Reads the locked version from Cargo.lock (the source of truth) and forces
# the install so the schema matches at runtime.
#
# Why this exists :
#   wasm-bindgen's schema changes on every publish. If the CLI version differs
#   from the crate version, the generated bindings produce silent breakage or
#   a runtime mismatch error at WASM instantiation.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ ! -f Cargo.lock ]; then
    echo "ERROR: Cargo.lock not found. Run 'cargo generate-lockfile' first." >&2
    exit 1
fi

VERSION=$(
    awk '
        /^name = "wasm-bindgen"/ { found = 1; next }
        found && /^version = / {
            gsub(/[ "]/, "", $0); sub(/^version=/, "", $0); print; exit
        }
    ' Cargo.lock
)

if [ -z "$VERSION" ]; then
    echo "ERROR: could not find wasm-bindgen version in Cargo.lock" >&2
    exit 1
fi

echo "Installing wasm-bindgen-cli =$VERSION (matching Cargo.lock)..."
cargo install wasm-bindgen-cli --version "=$VERSION" --force --locked

INSTALLED=$(wasm-bindgen --version | awk '{print $2}')
if [ "$INSTALLED" = "$VERSION" ]; then
    echo "OK : wasm-bindgen-cli $INSTALLED installed and matches workspace pin."
else
    echo "WARN : installed $INSTALLED but expected $VERSION" >&2
    exit 1
fi
