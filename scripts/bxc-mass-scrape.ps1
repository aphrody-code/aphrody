# SPDX-License-Identifier: Apache-2.0
#
# bxc-mass-scrape.ps1
# ===================
#
# Native Windows launcher for scripts/bxc-mass-scrape.ts.
# Ensures bxc is cloned at C:\worktree\bxc, the cache dir exists,
# and bun is on PATH, then invokes the orchestrator with sane defaults.
#
# Examples:
#   .\scripts\bxc-mass-scrape.ps1
#   .\scripts\bxc-mass-scrape.ps1 -Concurrency 12 -Profile stealth -Mode full
#   .\scripts\bxc-mass-scrape.ps1 -Urls scripts/bxc-mass-scrape.urls.json -Force
#
# Requirements (per CLAUDE.md §2):
#   - bun on PATH                       (winget install Oven-sh.Bun)
#   - gh on PATH                        (winget install GitHub.cli)
#   - C:\worktree\bxc (cloned on first run)
#
# Exit codes:
#   0 = OK (>= 50 % URLs succeeded or cached)
#   1 = orchestrator FAILED (failure rate > 50 %, bxc missing, etc.)

[CmdletBinding()]
param(
	[string]$BxcRoot = "C:\worktree\bxc",
	[string]$Urls    = "scripts/bxc-mass-scrape.urls.json",
	[string]$Cache   = "var/data/bxc-cache",
	[int]$Concurrency = 6,
	[ValidateSet("static", "fast", "stealth", "max")]
	[string]$Profile  = "fast",
	[ValidateSet("static", "full")]
	[string]$Mode     = "static",
	[int]$TimeoutMs   = 60000,
	[int]$Retry       = 2,
	[switch]$Force
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# --------------------------------------------------------------------------
# Pre-flight checks
# --------------------------------------------------------------------------

if (-not (Get-Command bun -ErrorAction SilentlyContinue)) {
	throw "bun not found on PATH. Install via: winget install Oven-sh.Bun"
}

if (-not (Test-Path -LiteralPath $BxcRoot)) {
	Write-Host "[bxc-mass-scrape] bxc not found at $BxcRoot — cloning..." -ForegroundColor Yellow
	if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
		throw "gh not found on PATH. Install via: winget install GitHub.cli"
	}
	$parent = Split-Path -Parent $BxcRoot
	if (-not (Test-Path -LiteralPath $parent)) {
		New-Item -ItemType Directory -Path $parent -Force | Out-Null
	}
	& gh repo clone aphrody-code/bxc -- --branch aphrody $BxcRoot
	if ($LASTEXITCODE -ne 0) {
		throw "gh repo clone aphrody-code/bxc failed (exit $LASTEXITCODE)"
	}
}

$browserApi = Join-Path $BxcRoot "src\api\browser.ts"
if (-not (Test-Path -LiteralPath $browserApi)) {
	throw "bxc clone at $BxcRoot is missing src/api/browser.ts — corrupt clone."
}

if (-not (Test-Path -LiteralPath $Cache)) {
	New-Item -ItemType Directory -Path $Cache -Force | Out-Null
}

if (-not (Test-Path -LiteralPath $Urls)) {
	throw "URL list not found: $Urls"
}

# --------------------------------------------------------------------------
# Run
# --------------------------------------------------------------------------

$env:BXC_ROOT = $BxcRoot

$bunArgs = @(
	"run", "scripts/bxc-mass-scrape.ts",
	"--bxc=$BxcRoot",
	"--urls=$Urls",
	"--cache=$Cache",
	"--concurrency=$Concurrency",
	"--profile=$Profile",
	"--mode=$Mode",
	"--timeout=$TimeoutMs",
	"--retry=$Retry"
)
if ($Force) { $bunArgs += "--force" }

Write-Host "[bxc-mass-scrape] launching: bun $($bunArgs -join ' ')" -ForegroundColor Cyan
& bun @bunArgs
$rc = $LASTEXITCODE

# --------------------------------------------------------------------------
# Summary
# --------------------------------------------------------------------------

$manifest = Join-Path $Cache "manifest.json"
if (Test-Path -LiteralPath $manifest) {
	$m = Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
	Write-Host ""
	Write-Host "[bxc-mass-scrape] === Summary ===" -ForegroundColor Green
	Write-Host ("  Total      : {0}" -f $m.stats.total)
	Write-Host ("  OK         : {0}" -f $m.stats.ok)
	Write-Host ("  Cached     : {0}" -f $m.stats.cached)
	Write-Host ("  Failed     : {0}" -f $m.stats.failed)
	Write-Host ("  Bytes      : {0:N0}" -f $m.stats.totalBytes)
	Write-Host ("  Elapsed    : {0:N0} ms" -f $m.stats.totalMs)
	Write-Host ("  FailureRate: {0:P1}" -f $m.stats.failureRate)
	Write-Host ("  Manifest   : {0}" -f $manifest)
}

exit $rc
