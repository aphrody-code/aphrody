<#
.SYNOPSIS
    Wipe agressif des artifacts régénérables — safe via git clean -fdX.

.DESCRIPTION
    Utilise `git clean -fdX` (uniquement les fichiers ignorés par .gitignore)
    pour ne JAMAIS supprimer un fichier tracké ou un fichier non-ignoré.

    Phase 2 : wipe ciblé des caches sous vendor/ qui ne sont pas couverts
    par git clean (car vendor/ est lui-même tracked).
#>
[CmdletBinding()]
param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot),
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

Push-Location $Root
try {
    $globalStart = Get-Date

    Write-Host ""
    Write-Host "==============================================================" -ForegroundColor Cyan
    Write-Host "  WIPE ARTIFACTS  ($(if ($DryRun) {'DRY-RUN'} else {'APPLY'}))" -ForegroundColor Cyan
    Write-Host "  Root : $Root" -ForegroundColor Cyan
    Write-Host "==============================================================" -ForegroundColor Cyan

    # ---- Snapshot AVANT ----
    Write-Host "`n[snapshot] mesure taille AVANT..." -ForegroundColor Gray
    $before = Get-ChildItem -Recurse -File -Force -EA SilentlyContinue | Measure-Object Length -Sum

    # ---- Phase 1 : git clean -fdX ----
    Write-Host "`n[1/2] git clean -fdX (ne supprime QUE les fichiers ignorés)" -ForegroundColor Yellow
    if ($DryRun) {
        $log = 'C:\aphrody-backup\git-clean-dryrun.log'
        & git clean -fdX --dry-run 2>&1 | Tee-Object -FilePath $log | Select-Object -First 50
        $totalLines = (Get-Content $log).Count
        Write-Host "...(full log: $log, $totalLines lignes)" -ForegroundColor DarkGray
    } else {
        & git clean -fdX --quiet
        Write-Host "  done" -ForegroundColor Green
    }

    # ---- Phase 2 : caches sous vendor/ (non couverts par git clean) ----
    Write-Host "`n[2/2] vendor caches (hors scope git clean)" -ForegroundColor Yellow
    $vendorCaches = @(
        'vendor\bun\build',
        'vendor\bun\node_modules',
        'vendor\bun\.cache',
        'vendor\bun\packages\bun-types\dist',
        'vendor\depot_tools\.cipd_bin',
        'vendor\depot_tools\.versions',
        'vendor\depot_tools\bootstrap-3.8.0.chromium.8_bin',
        'vendor\a2a-rs\target',
        'crates\coreutils\target',
        'crates\util-linux\target'
    )
    foreach ($vc in $vendorCaches) {
        $p = Join-Path $Root $vc
        if (-not (Test-Path $p)) { continue }
        $sz = (Get-ChildItem $p -Recurse -File -Force -EA SilentlyContinue | Measure-Object Length -Sum).Sum
        if ($DryRun) {
            Write-Host ("  [DRY] {0,-50} {1,8:N1} MB" -f $vc, ($sz / 1MB)) -ForegroundColor Gray
        } else {
            cmd.exe /c "rd /s /q `"$p`"" 2>$null
            Write-Host ("  [RM ] {0,-50} {1,8:N1} MB" -f $vc, ($sz / 1MB)) -ForegroundColor Green
        }
    }

    # ---- Snapshot APRÈS ----
    Write-Host "`n[snapshot] mesure taille APRÈS..." -ForegroundColor Gray
    $after = Get-ChildItem -Recurse -File -Force -EA SilentlyContinue | Measure-Object Length -Sum

    $elapsed = ((Get-Date) - $globalStart).TotalSeconds

    Write-Host "`n==============================================================" -ForegroundColor Cyan
    Write-Host ("  AVANT : {0,10:N0} files / {1,7:N2} GB" -f $before.Count, ($before.Sum / 1GB)) -ForegroundColor White
    Write-Host ("  APRÈS : {0,10:N0} files / {1,7:N2} GB" -f $after.Count,  ($after.Sum  / 1GB)) -ForegroundColor White
    Write-Host ("  FREED : {0,10:N0} files / {1,7:N2} GB" -f ($before.Count - $after.Count), (($before.Sum - $after.Sum) / 1GB)) -ForegroundColor Green
    Write-Host ("  TIME  : {0,10:N1} s" -f $elapsed) -ForegroundColor Gray
    Write-Host "==============================================================" -ForegroundColor Cyan
} finally {
    Pop-Location
}
