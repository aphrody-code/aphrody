#!/usr/bin/env pwsh
# ============================================================================
#  aphrody — auto-deploy (PowerShell)
#  Remplace l'ancien `cargo xtask deploy` (crate aphrody-xtask supprimée le
#  2026-05-21). Build release de tous les binaires du workspace puis copie
#  ceux qui matchent les préfixes vers ~/.local/bin (convention d'install).
#
#  Usage :
#    ./scripts/deploy.ps1                       # build + install (aphrody,mrx,notebooklm)
#    ./scripts/deploy.ps1 -NoBuild              # copie seulement (build déjà fait)
#    ./scripts/deploy.ps1 -Prefixes aphrody     # restreindre aux bins aphrody*
#    ./scripts/deploy.ps1 -Target x86_64-pc-windows-msvc
#    ./scripts/deploy.ps1 -Dest D:\bin -DryRun  # simulation, dest custom
#  Parité : scripts/deploy.sh (Linux/macOS).
# ============================================================================
[CmdletBinding()]
param(
    [switch]$NoBuild,
    [string]$Prefixes = "aphrody,mrx,notebooklm",
    [string]$Dest,
    [string]$Target,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# --- Résolution racine repo (le script vit dans scripts/) -------------------
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $RepoRoot

$prefixList = $Prefixes.Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ }
if (-not $prefixList) { throw "-Prefixes doit contenir au moins une entrée non vide." }

# --- Destination ------------------------------------------------------------
if (-not $Dest) {
    $home0 = if ($env:USERPROFILE) { $env:USERPROFILE } else { $HOME }
    $Dest = Join-Path (Join-Path $home0 ".local") "bin"
}

# --- Build ------------------------------------------------------------------
if (-not $NoBuild) {
    $buildArgs = @("build", "--release", "--locked")
    if ($Target) { $buildArgs += @("--target", $Target) }
    Write-Host "[deploy] cargo $($buildArgs -join ' ')" -ForegroundColor Cyan
    if (-not $DryRun) {
        & cargo @buildArgs
        if ($LASTEXITCODE -ne 0) { throw "cargo build a échoué (exit $LASTEXITCODE)" }
    }
}

# --- Répertoire des artefacts ----------------------------------------------
$releaseDir = if ($Target) {
    Join-Path (Join-Path (Join-Path $RepoRoot "target") $Target) "release"
} else {
    Join-Path (Join-Path $RepoRoot "target") "release"
}
if (-not (Test-Path $releaseDir)) { throw "Répertoire release introuvable : $releaseDir (build manqué ?)" }

# --- Découverte des binaires (glob top-level *.exe matchant un préfixe) -----
$bins = Get-ChildItem -Path $releaseDir -Filter "*.exe" -File | Where-Object {
    $name = $_.BaseName
    $prefixList | Where-Object { $name.StartsWith($_) }
}
if (-not $bins) { throw "Aucun binaire *.exe matchant $($prefixList -join ',') dans $releaseDir" }

if (-not $DryRun) { New-Item -ItemType Directory -Force -Path $Dest | Out-Null }

# --- Copie avec gestion des verrous (process actif) -------------------------
$deployed = @(); $locked = @()
foreach ($bin in $bins) {
    $dst = Join-Path $Dest $bin.Name
    if ($DryRun) {
        Write-Host "[dry-run] $($bin.FullName) -> $dst" -ForegroundColor DarkGray
        continue
    }
    try {
        Copy-Item -Path $bin.FullName -Destination $dst -Force
        $deployed += [pscustomobject]@{ Name = $bin.Name; Size = (Get-Item $dst).Length }
    } catch {
        # Fichier verrouillé (binaire en cours d'exécution) : temp + rename.
        $tmp = "$dst.deploy-new"
        try {
            Copy-Item -Path $bin.FullName -Destination $tmp -Force
            Move-Item -Path $tmp -Destination $dst -Force
            $deployed += [pscustomobject]@{ Name = $bin.Name; Size = (Get-Item $dst).Length }
        } catch {
            if (Test-Path $tmp) { Remove-Item $tmp -Force -ErrorAction SilentlyContinue }
            $locked += $bin.Name
        }
    }
}

# --- Rapport ----------------------------------------------------------------
if ($deployed) {
    Write-Host "=== aphrody deploy : $($deployed.Count) binaire(s) installé(s) -> $Dest ===" -ForegroundColor Green
    foreach ($d in $deployed) { Write-Host ("  [ok] {0,-28} ({1} bytes)" -f $d.Name, $d.Size) }
}
if ($locked) {
    Write-Host "=== $($locked.Count) binaire(s) verrouillé(s) (process actif — kill puis re-deploy) ===" -ForegroundColor Yellow
    foreach ($l in $locked) { Write-Host "  [locked] $l" }
}

# --- Check PATH -------------------------------------------------------------
$pathDirs = $env:PATH.Split([IO.Path]::PathSeparator)
if ($pathDirs -notcontains $Dest) {
    Write-Host "[warn] $Dest n'est pas dans le PATH. Ajoute-le :" -ForegroundColor Yellow
    Write-Host "  [Environment]::SetEnvironmentVariable('Path', `"`$env:Path;$Dest`", 'User')"
}

if ($locked) { exit 1 }
