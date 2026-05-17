#Requires -Version 7
<#
.SYNOPSIS
    Build vendor/terminal (microsoft/terminal) avec la toolchain VS 2026 Insiders
    et le Windows SDK 10.0.26100.0 disponibles sur la machine de dev.

.DESCRIPTION
    L'upstream pin :
        PlatformToolset              = v143  (MSVC VS 2022)
        WindowsTargetPlatformVersion = 10.0.22621.0 (Windows 11 21H2)
    Ce wrapper surcharge les deux pour utiliser :
        v145  (MSVC 14.51, VS 2026 Insiders)
        10.0.26100.0 (Windows 11 24H2)
    Voir docs/terminal/BUILD.md pour les détails et le troubleshooting.

    Patches locaux appliqués sous vendor/terminal/ (sous-module reste dirty) :
      - dep/vcpkg-overlay-triplets/{,fuzzing/}{x64,x86,arm64}-windows-static.cmake
        v143 -> v145
      - src/common.build.pre.props : ajout 4875 à DisableSpecificWarnings
        (ms-gsl 3.1.0 utilise l'ancienne syntaxe [[gsl::suppress(token)]] que
        MSVC v145 émet en C4875, traité comme erreur par TreatWarningAsError).

.PARAMETER Project
    Cible MSBuild /t: optionnelle. Par défaut vide = toute la solution.
    Exemple : 'Conhost\Host_EXE', 'Terminal\CascadiaPackage'.

.PARAMETER Configuration
    Debug | Release | AuditMode | Fuzzing. Défaut : Release.

.PARAMETER Platform
    x64 | x86 | ARM64. Défaut : x64.

.EXAMPLE
    # Build smoke test conhost seul
    ./scripts/terminal/build.ps1 -Project Conhost\Host_EXE

.EXAMPLE
    # Build complet x64 Release
    ./scripts/terminal/build.ps1 -Configuration Release
#>
[CmdletBinding()]
param(
    [string]$Project = '',
    [ValidateSet('Debug', 'Release', 'AuditMode', 'Fuzzing')]
    [string]$Configuration = 'Release',
    [ValidateSet('x64', 'x86', 'ARM64')]
    [string]$Platform = 'x64'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# scripts/terminal/build.ps1 -> repo root est deux niveaux au-dessus.
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..' '..')
$terminalRoot = Join-Path $repoRoot 'vendor' 'terminal'

if (-not (Test-Path (Join-Path $terminalRoot 'OpenConsole.slnx'))) {
    throw "vendor/terminal not initialized. Run: git submodule update --init -- vendor/terminal"
}

Push-Location $terminalRoot
try {
    Write-Host "==> Importing OpenConsole.psm1" -ForegroundColor Cyan
    Import-Module (Join-Path $terminalRoot 'tools' 'OpenConsole.psm1') -Force

    Write-Host "==> Set-MsbuildDevEnvironment -Prerelease (picks VS 2026 Insiders)" -ForegroundColor Cyan
    Set-MsbuildDevEnvironment -Prerelease

    # Force toolset/SDK les plus récents.
    $env:PlatformToolset = 'v145'
    $env:WindowsTargetPlatformVersion = '10.0.26100.0'

    # NuGet 4.1 embarqué ne parse pas .slnx (issue NuGet/Home #14034).
    # Le NuGet récent + -MSBuildPath vers MSBuild 18.7 délègue le parse à MSBuild.
    $nuget = Join-Path $terminalRoot 'dep' 'nuget' 'nuget-latest.exe'
    if (-not (Test-Path $nuget)) {
        Write-Host "==> Downloading latest nuget.exe" -ForegroundColor Cyan
        Invoke-WebRequest -UseBasicParsing `
            -Uri 'https://dist.nuget.org/win-x86-commandline/latest/nuget.exe' `
            -OutFile $nuget
    }

    $msbuildBin = Join-Path $env:VSINSTALLDIR 'MSBuild' 'Current' 'Bin'

    Write-Host "==> nuget restore OpenConsole.slnx" -ForegroundColor Cyan
    & $nuget restore (Join-Path $terminalRoot 'OpenConsole.slnx') -MSBuildPath $msbuildBin
    if ($LASTEXITCODE -ne 0) { throw "nuget restore OpenConsole.slnx failed ($LASTEXITCODE)" }

    Write-Host "==> nuget restore dep\nuget\packages.config" -ForegroundColor Cyan
    & $nuget restore (Join-Path $terminalRoot 'dep' 'nuget' 'packages.config') `
        -SolutionDirectory $terminalRoot
    if ($LASTEXITCODE -ne 0) { throw "nuget restore packages.config failed ($LASTEXITCODE)" }

    $msbuildArgs = @(
        (Join-Path $terminalRoot 'OpenConsole.slnx')
        "/p:Configuration=$Configuration"
        "/p:Platform=$Platform"
        "/p:PlatformToolset=v145"
        "/p:WindowsTargetPlatformVersion=10.0.26100.0"
        "/p:TargetPlatformVersion=10.0.26100.0"      # surcharge wapproj UAP
        "/p:AppxSymbolPackageEnabled=false"
        '/m'
        '/nologo'
        '/v:minimal'
    )
    if ($Project) {
        $msbuildArgs += "/t:$Project"
    }

    Write-Host "==> msbuild $($msbuildArgs -join ' ')" -ForegroundColor Cyan
    & msbuild.exe @msbuildArgs
    if ($LASTEXITCODE -ne 0) { throw "msbuild failed ($LASTEXITCODE)" }

    Write-Host "==> Build OK" -ForegroundColor Green
}
finally {
    Pop-Location
}
