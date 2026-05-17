#requires -Version 5.1
<#
.SYNOPSIS
  Rename the local working tree from C:\src\google-cli to C:\src\aphrody and
  migrate the corresponding Claude Code project directory in one shot.

.DESCRIPTION
  Performs a Move-Item of:
    1. C:\src\google-cli           -> C:\src\aphrody
    2. ~\.claude\projects\C--src-google-cli  -> ~\.claude\projects\C--src-aphrody
       (preserves memory/, sessions/, etc., so all auto-memory survives)

  Must be run from a shell that is NOT inside either directory. Will refuse if
  any handle on the tree is detected (rust-analyzer, VS Code, another shell,
  Claude Code itself...).

.NOTES
  Run AFTER closing Claude Code and any IDE / terminal that has google-cli open.

.EXAMPLE
  cd C:\
  powershell -NoProfile -ExecutionPolicy Bypass -File C:\src\google-cli\scripts\rename-to-aphrody.ps1
#>

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$Old        = 'C:\src\google-cli'
$New        = 'C:\src\aphrody'
$OldClaude  = Join-Path $env:USERPROFILE '.claude\projects\C--src-google-cli'
$NewClaude  = Join-Path $env:USERPROFILE '.claude\projects\C--src-aphrody'

function Test-PathLocked([string]$Path) {
    if (-not (Test-Path $Path)) { return $false }
    try {
        $tmp = Join-Path $Path '_lock_probe.tmp'
        [IO.File]::WriteAllText($tmp, 'probe')
        Remove-Item $tmp -Force
        $false
    } catch {
        $true
    }
}

# --- Pre-checks -----------------------------------------------------------
Write-Host '[1/5] Verifying pre-conditions...' -ForegroundColor Cyan

$cwd = (Get-Location).Path
if ($cwd -like "$Old*") {
    Write-Host "  FAIL : current shell is inside $Old. cd out first." -ForegroundColor Red
    exit 1
}
if (-not (Test-Path $Old)) {
    Write-Host "  $Old does not exist." -ForegroundColor Yellow
    if (Test-Path $New) {
        Write-Host "  $New already exists. Already migrated, nothing to do." -ForegroundColor Green
        exit 0
    }
    Write-Host "  Nothing to rename." -ForegroundColor Red
    exit 1
}
if (Test-Path $New) {
    Write-Host "  FAIL : $New already exists. Manual conflict resolution required." -ForegroundColor Red
    exit 1
}

Write-Host '[2/5] Probing for locks on the source tree (this can take a few seconds)...' -ForegroundColor Cyan
if (Test-PathLocked $Old) {
    Write-Host "  FAIL : $Old has a process holding a handle on it." -ForegroundColor Red
    Write-Host "  Common culprits : Claude Code, VS Code (window), rust-analyzer, cargo build, file explorer." -ForegroundColor Yellow
    Write-Host "  Close them and re-run." -ForegroundColor Yellow
    exit 1
}

# --- Optional: claude project dir state ----------------------------------
Write-Host '[3/5] Checking Claude Code project mirror...' -ForegroundColor Cyan
if (Test-Path $OldClaude) {
    if (Test-Path $NewClaude) {
        Write-Host "  FAIL : both $OldClaude AND $NewClaude exist. Resolve manually." -ForegroundColor Red
        exit 1
    }
    Write-Host "  OK : $OldClaude will be moved to $NewClaude (preserves memory + sessions)." -ForegroundColor Green
} else {
    Write-Host "  Note : no Claude Code project mirror found ; skipping that step." -ForegroundColor Yellow
}

# --- Rename --------------------------------------------------------------
Write-Host "[4/5] Renaming working tree..." -ForegroundColor Cyan
Move-Item -Path $Old -Destination $New
Write-Host "  $Old -> $New" -ForegroundColor Green

if (Test-Path $OldClaude) {
    Write-Host "[4b/5] Migrating Claude Code project mirror..." -ForegroundColor Cyan
    Move-Item -Path $OldClaude -Destination $NewClaude
    Write-Host "  $OldClaude -> $NewClaude" -ForegroundColor Green
}

# --- Post-rename verifications ------------------------------------------
Write-Host '[5/5] Post-rename verifications...' -ForegroundColor Cyan

Push-Location $New
try {
    $remote = git remote get-url origin 2>$null
    $branch = git rev-parse --abbrev-ref HEAD 2>$null
    $head   = git rev-parse --short HEAD 2>$null
    Write-Host "  git remote : $remote" -ForegroundColor Green
    Write-Host "  git branch : $branch  (HEAD $head)" -ForegroundColor Green

    Write-Host '  cargo workspace ping (cargo metadata --no-deps -q --format-version 1 | head)...' -ForegroundColor Cyan
    $meta = & cargo metadata --no-deps --format-version 1 2>$null | Out-String
    if ($meta) {
        $obj = $meta | ConvertFrom-Json
        Write-Host ('    workspace_root : ' + $obj.workspace_root) -ForegroundColor Green
        Write-Host ('    packages       : ' + ($obj.packages.Count)) -ForegroundColor Green
    } else {
        Write-Host '    cargo metadata returned nothing — investigate.' -ForegroundColor Yellow
    }
} finally {
    Pop-Location
}

Write-Host ''
Write-Host '====================================================================' -ForegroundColor Green
Write-Host ' Rename complete.' -ForegroundColor Green
Write-Host '' -ForegroundColor Green
Write-Host " Working tree            : $New" -ForegroundColor Green
Write-Host " Claude Code memory dir  : $NewClaude" -ForegroundColor Green
Write-Host '' -ForegroundColor Green
Write-Host ' Next steps :' -ForegroundColor Green
Write-Host "   cd $New" -ForegroundColor Green
Write-Host '   claude       # opens a fresh Claude Code session in the new path' -ForegroundColor Green
Write-Host '====================================================================' -ForegroundColor Green
