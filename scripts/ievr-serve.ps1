#requires -Version 7
<#
.SYNOPSIS
    Boot the IEVR (Inazuma Eleven Victory Road) browser runtime served by winclean's iecode-web.

.DESCRIPTION
    Bootstrap wrapper around `bun --cwd <winclean>/apps/iecode-web run dev`.
    Aphrody owns the cross-platform CLI; the actual server lives in the winclean repo
    (peer instance per ai.json a2a coord). This script lets you launch /ievr from any
    directory on this machine without remembering the cwd.

.PARAMETER WincleanRoot
    Path to the winclean repo root. Default: C:\winclean.

.PARAMETER OpenBrowser
    If set, opens http://localhost:8787/ievr in the default browser (Edge recommended).

.PARAMETER Port
    Override the port (default 8787, set via Bun env PORT).

.EXAMPLE
    pwsh scripts/ievr-serve.ps1 -OpenBrowser

.EXAMPLE
    pwsh scripts/ievr-serve.ps1 -WincleanRoot D:/winclean -Port 9000
#>
param(
    [string]$WincleanRoot = 'C:\winclean',
    [int]$Port = 8787,
    [switch]$OpenBrowser
)

$ErrorActionPreference = 'Stop'

$iecodeWeb = Join-Path $WincleanRoot 'apps\iecode-web'
if (-not (Test-Path $iecodeWeb)) {
    Write-Error "iecode-web not found at $iecodeWeb. Pass -WincleanRoot <path>."
    exit 1
}

$bun = (Get-Command bun -ErrorAction SilentlyContinue).Source
if (-not $bun) {
    Write-Error "bun not on PATH. Install via 'irm bun.sh/install.ps1 | iex' and retry."
    exit 1
}

# Don't shadow node — Bun reads env to know its own port.
$env:PORT = "$Port"

$url = "http://localhost:$Port/ievr"
Write-Host "Serving IEVR at $url (cwd: $iecodeWeb)" -ForegroundColor Cyan
Write-Host "Stop with Ctrl+C." -ForegroundColor DarkGray

if ($OpenBrowser) {
    Start-Process $url
}

Push-Location $iecodeWeb
try {
    & $bun run dev
} finally {
    Pop-Location
}
