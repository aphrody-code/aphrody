#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Build (or run) the aphrody Tauri desktop app WITHOUT touching the core build.
#
#   1. Builds the React frontend in the sibling aphrody-ts repo (Bun, prod).
#   2. Copies its dist/ into crates/aphrody-app/dist (embedded by Tauri's
#      generate_context! at compile time).
#   3. Runs cargo on the build-EXCLUDED aphrody-app crate, sharing the core
#      target dir so the already-compiled aphrody CLI tree is reused.
#
# The core workspace (`cargo ci-offline`, the `aphrody` binary) is never touched:
# aphrody-app is excluded and carries its own Cargo.lock.
#
# Usage:
#   scripts/tauri.sh                       # release build
#   scripts/tauri.sh run                   # release build + launch
#   scripts/tauri.sh dev                   # debug build + launch
#   FRONTEND=desktop-ui scripts/tauri.sh   # use the vanilla shell instead
#
# Linux #1 needs the webkit2gtk-4.1 dev packages installed:
#   sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev
set -euo pipefail

action="${1:-build}"
frontend="${FRONTEND:-desktop-react}"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$script_dir/.." && pwd)"
ts_repo="$(cd "$repo/.." && pwd)/aphrody-ts"
app_dir="$repo/crates/aphrody-app"
fe_dir="$ts_repo/apps/$frontend"

if [[ ! -d "$fe_dir" ]]; then
  echo "Frontend not found: $fe_dir (is the sibling aphrody-ts repo checked out?)" >&2
  exit 1
fi

echo "==> Building frontend ($frontend) with Bun (production)"
(cd "$fe_dir" && bun install && bun run build)

echo "==> Syncing dist -> crates/aphrody-app/dist"
rm -rf "$app_dir/dist"
cp -r "$fe_dir/dist" "$app_dir/dist"

echo "==> cargo $action (aphrody-app, shared target dir)"
export CARGO_TARGET_DIR="$repo/target"
cd "$app_dir"
case "$action" in
  build) cargo build --release ;;
  run) cargo run --release ;;
  dev) cargo run ;;
  *) echo "unknown action: $action (use build|run|dev)" >&2; exit 1 ;;
esac

echo "==> Done."
