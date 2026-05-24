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
#   pwsh scripts/tauri.ps1                 # release build
#   pwsh scripts/tauri.ps1 -Action run     # release build + launch
#   pwsh scripts/tauri.ps1 -Action dev      # debug build + launch (console kept)
#   pwsh scripts/tauri.ps1 -Frontend desktop-ui   # use the vanilla shell instead
[CmdletBinding()]
param(
  [ValidateSet('build', 'run', 'dev')] [string]$Action = 'build',
  [ValidateSet('desktop-react', 'desktop-ui')] [string]$Frontend = 'desktop-react'
)
$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$tsRepo = Join-Path (Split-Path -Parent $repo) 'aphrody-ts'
$appDir = Join-Path $repo 'crates/aphrody-app'
$feDir = Join-Path $tsRepo "apps/$Frontend"

if (-not (Test-Path $feDir)) {
  throw "Frontend not found: $feDir (is the sibling aphrody-ts repo checked out?)"
}

Write-Host "==> Building frontend ($Frontend) with Bun (production)" -ForegroundColor Cyan
Push-Location $feDir
try {
  bun install
  if ($LASTEXITCODE -ne 0) { throw "bun install failed" }
  bun run build
  if ($LASTEXITCODE -ne 0) { throw "bun run build failed" }
}
finally { Pop-Location }

Write-Host "==> Syncing dist -> crates/aphrody-app/dist" -ForegroundColor Cyan
$dist = Join-Path $appDir 'dist'
if (Test-Path $dist) { Remove-Item -Recurse -Force $dist }
Copy-Item -Recurse -Force (Join-Path $feDir 'dist') $dist

Write-Host "==> cargo $Action (aphrody-app, shared target dir)" -ForegroundColor Cyan
$env:CARGO_TARGET_DIR = Join-Path $repo 'target'
Push-Location $appDir
try {
  switch ($Action) {
    'build' { cargo build --release }
    'run' { cargo run --release }
    'dev' { cargo run }
  }
  if ($LASTEXITCODE -ne 0) { throw "cargo $Action failed" }
}
finally { Pop-Location }

Write-Host "==> Done." -ForegroundColor Green
