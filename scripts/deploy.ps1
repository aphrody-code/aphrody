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
#    ./scripts/deploy.ps1 -IncludeGui           # + copie aphrody-gui (app desktop)
#  Parité : scripts/deploy.sh (Linux/macOS).
#
#  Note GUI : l'app desktop (apps/desktop) est un workspace SELF-ROOTED exclu du
#  workspace core, donc `cargo build --release` ci-dessous ne la construit PAS.
#  -IncludeGui copie le binaire `aphrody-gui` déjà bâti (copy-only) depuis
#  apps/desktop/src-tauri/target/[<triple>/]{release,debug} à côté d'aphrody,
#  pour que le resolver sibling de `aphrody gui` le trouve une fois déployé.
#  Construire la GUI en PRODUCTION d'abord (charge les assets bundlés, sinon le
#  binaire cherche le serveur dev http://localhost:1420) :
#    cd apps/desktop ; bun install ; bun run tauri build
#  (ou, binaire seul sans Node : cargo build --release --features custom-protocol)
# ============================================================================
[CmdletBinding()]
param(
    [switch]$NoBuild,
    [string]$Prefixes = "aphrody,mrx,notebooklm",
    [string]$Dest,
    [string]$Target,
    [switch]$IncludeGui,
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

# --- Répertoire(s) des artefacts -------------------------------------------
# Sans -Target explicite, `.cargo/config.toml` peut forcer un `build.target`
# (ici x86_64-pc-windows-msvc) : les binaires atterrissent alors dans
# target/<triple>/release et NON target/release. On cherche donc dans tous les
# candidats plausibles.
$targetRoot = Join-Path $RepoRoot "target"
$candidateDirs = if ($Target) {
    @(Join-Path (Join-Path $targetRoot $Target) "release")
} else {
    $dirs = @(Join-Path $targetRoot "release")
    # Ajoute chaque target/<triple>/release existant (triple = sous-dossier
    # contenant un dossier release), trié pour un ordre déterministe.
    Get-ChildItem -Path $targetRoot -Directory -ErrorAction SilentlyContinue |
        Where-Object { Test-Path (Join-Path $_.FullName "release") } |
        Sort-Object Name |
        ForEach-Object { $dirs += (Join-Path $_.FullName "release") }
    $dirs
}

# --- Découverte des binaires (glob top-level *.exe matchant un préfixe) -----
$bins = @()
$releaseDir = $null
foreach ($dir in $candidateDirs) {
    if (-not (Test-Path $dir)) { continue }
    $found = Get-ChildItem -Path $dir -Filter "*.exe" -File | Where-Object {
        $name = $_.BaseName
        $prefixList | Where-Object { $name.StartsWith($_) }
    }
    if ($found) { $bins = $found; $releaseDir = $dir; break }
}
if (-not $bins) {
    throw "Aucun binaire *.exe matchant $($prefixList -join ',') dans : $($candidateDirs -join ', ') (build manqué ?)"
}
Write-Host "[deploy] artefacts depuis $releaseDir" -ForegroundColor Cyan

# --- GUI desktop (opt-in) : binaire hors-workspace, build séparé ------------
# `aphrody-gui` est produit par le cargo self-rooted de apps/desktop/src-tauri
# (target propre), jamais par le build core ci-dessus. On l'ajoute au lot de
# copie pour qu'il atterrisse à côté d'aphrody (le resolver sibling de
# `aphrody gui` le trouve alors en prod). Copy-only : aucun build déclenché ici.
if ($IncludeGui) {
    $guiTargetRoot = Join-Path $RepoRoot "apps\desktop\src-tauri\target"
    # Candidate <profile> dirs in priority order (release before debug), honouring
    # both the plain layout (target/<profile>) and a forced build.target layout
    # (target/<triple>/<profile> -- e.g. .cargo/config.toml pins the MSVC triple,
    # inherited by the self-rooted desktop build via config walk-up).
    $guiDirs = @()
    if ($Target) {
        foreach ($prof in @("release", "debug")) {
            $guiDirs += (Join-Path (Join-Path $guiTargetRoot $Target) $prof)
        }
    } else {
        $triples = @()
        if (Test-Path $guiTargetRoot) {
            $triples = Get-ChildItem -Path $guiTargetRoot -Directory -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -notin @("release", "debug") } |
                Sort-Object Name | ForEach-Object { $_.FullName }
        }
        foreach ($prof in @("release", "debug")) {
            $guiDirs += (Join-Path $guiTargetRoot $prof)
            foreach ($t in $triples) { $guiDirs += (Join-Path $t $prof) }
        }
    }
    $guiBin = $null
    foreach ($gd in $guiDirs) {
        $cand = Join-Path $gd "aphrody-gui.exe"
        if (Test-Path $cand) { $guiBin = Get-Item $cand; break }
    }
    if ($guiBin) {
        $bins = @($bins) + $guiBin
        Write-Host "[deploy] +GUI : $($guiBin.FullName)" -ForegroundColor Cyan
    } else {
        Write-Host "[warn] -IncludeGui demandé mais aphrody-gui.exe introuvable dans $($guiDirs -join ', ')." -ForegroundColor Yellow
        Write-Host "       Build production : cd apps/desktop; bun install; bun run tauri build" -ForegroundColor Yellow
    }
}

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
