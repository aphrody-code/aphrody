# SPDX-License-Identifier: Apache-2.0
# Install-AphrodyToPath.ps1 — Add the aphrody binary to the current user PATH.
#
# Adds `target\release\` to HKCU\Environment\Path so that `aphrody`, `aphrody n2b`,
# and `aphrody bxc` are available from any shell without a full path.
#
# Usage:
#   .\scripts\Install-AphrodyToPath.ps1
#   .\scripts\Install-AphrodyToPath.ps1 -BuildIfMissing
#   .\scripts\Install-AphrodyToPath.ps1 -BinPath "C:\custom\aphrody.exe"

[CmdletBinding()]
param(
    # Absolute path to the aphrody.exe binary.
    # Defaults to target\release\aphrody.exe relative to the repo root.
    [string]$BinPath,

    # When set, runs `cargo build -p aphrody --release --locked` if the binary
    # is not found at $BinPath before aborting.
    [switch]$BuildIfMissing
)

$ErrorActionPreference = 'Stop'

# Resolve repo root as the directory that contains this script's parent.
$RepoRoot = Split-Path -Parent $PSScriptRoot
if (-not $BinPath) {
    $BinPath = Join-Path $RepoRoot 'target' 'release' 'aphrody.exe'
}

if (-not (Test-Path $BinPath)) {
    if ($BuildIfMissing) {
        Write-Host "Binary not found at $BinPath — building release…"
        Push-Location $RepoRoot
        try {
            & cargo build -p aphrody --release --locked
            if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
        } finally {
            Pop-Location
        }
    } else {
        throw "Binary not found at $BinPath. Pass -BuildIfMissing to compile it first."
    }
}

$BinDir = Split-Path -Parent (Resolve-Path $BinPath)

$CurrentPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$Entries = $CurrentPath -split ';' | Where-Object { $_ -ne '' }

if ($Entries -contains $BinDir) {
    Write-Host "$BinDir is already in the user PATH." -ForegroundColor Yellow
} else {
    $NewPath = ($Entries + $BinDir) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $NewPath, 'User')
    Write-Host "Added $BinDir to user PATH." -ForegroundColor Green
    Write-Host "Re-open your terminal (or run: `$env:Path = [Environment]::GetEnvironmentVariable('Path','User')`) to apply." -ForegroundColor Cyan
}

Write-Host ""
Write-Host "Verify installation:"
Write-Host "  aphrody --version"
Write-Host "  aphrody n2b --help"
Write-Host "  aphrody bxc --help"
