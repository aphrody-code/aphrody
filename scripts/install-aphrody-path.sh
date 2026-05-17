#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# install-aphrody-path.sh — Symlink the aphrody release binary into $HOME/.local/bin
# and print the PATH export hint when needed.
#
# Usage:
#   bash scripts/install-aphrody-path.sh           # requires pre-built binary
#   bash scripts/install-aphrody-path.sh --build   # builds first with cargo
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$REPO_ROOT/target/release/aphrody"
LINK_DIR="$HOME/.local/bin"
LINK="$LINK_DIR/aphrody"

if [ ! -x "$BIN" ]; then
    if [ "${1:-}" = "--build" ]; then
        echo "[install] Building release binary…"
        cargo build -p aphrody --release --locked
    else
        echo "Binary missing at $BIN." >&2
        echo "Run with --build to compile first: bash scripts/install-aphrody-path.sh --build" >&2
        exit 1
    fi
fi

mkdir -p "$LINK_DIR"
ln -sf "$BIN" "$LINK"
echo "[install] Symlinked $BIN -> $LINK"

case ":${PATH}:" in
    *":$LINK_DIR:"*)
        echo "[install] $LINK_DIR is already on PATH."
        ;;
    *)
        echo ""
        echo "[install] $LINK_DIR is NOT on PATH."
        echo "Add the following to your shell rc file (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
        echo ""
        echo "Then reload: source ~/.bashrc   (or open a new terminal)"
        ;;
esac

echo ""
echo "Verify:"
echo "  aphrody --version"
echo "  aphrody n2b --help"
echo "  aphrody bxc --help"
