# SPDX-License-Identifier: Apache-2.0
#
# aphrody — one-line installer for Windows (PowerShell 7+).
#
# Usage:
#   irm https://raw.githubusercontent.com/aphrody-code/aphrody/main/packaging/install.ps1 | iex
#   $env:APHRODY_VERSION = "v0.2.0"; irm ... | iex
#
# What it does:
#   1. Detects architecture (x86_64 / aarch64).
#   2. Resolves the latest tag via the GitHub Releases API (or honours
#      $env:APHRODY_VERSION if set).
#   3. Downloads the matching .zip + .sha256 from the GitHub Release.
#   4. Verifies the SHA-256, aborts on mismatch.
#   5. Extracts to $env:APHRODY_BIN (default: %LOCALAPPDATA%\aphrody\bin),
#      adds the directory to the user PATH if absent.

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$ProgressPreference    = 'SilentlyContinue'  # speeds up Invoke-WebRequest

$Owner   = 'aphrody-code'
$Repo    = 'aphrody'
$Bin     = 'aphrody.exe'
$BinDir  = if ($env:APHRODY_BIN) { $env:APHRODY_BIN }
           else { Join-Path $env:LOCALAPPDATA 'aphrody\bin' }

function Die($msg) {
    Write-Host "error: $msg" -ForegroundColor Red
    exit 1
}

# ---- Architecture detection ----------------------------------------------
$arch = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
$archPart = switch ($arch) {
    'X64'   { 'x86_64' }
    'Arm64' { 'aarch64' }
    default { Die "unsupported architecture: $arch" }
}
$target = "$archPart-pc-windows-msvc"

# ---- Resolve version -----------------------------------------------------
if ($env:APHRODY_VERSION) {
    $tag = $env:APHRODY_VERSION
} else {
    Write-Host 'Resolving latest aphrody release...'
    try {
        $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/$Owner/$Repo/releases/latest"
        $tag = $rel.tag_name
    } catch {
        Die "could not query GitHub Releases API: $($_.Exception.Message)"
    }
}
if (-not $tag) { Die 'could not resolve latest release tag' }
$version = $tag.TrimStart('v')
Write-Host "Installing aphrody $version ($target)"

# ---- Download + verify ---------------------------------------------------
$archive = "aphrody-$tag-$target.zip"
$url     = "https://github.com/$Owner/$Repo/releases/download/$tag/$archive"
$tmp     = Join-Path ([System.IO.Path]::GetTempPath()) ([guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
try {
    Invoke-WebRequest -Uri $url           -OutFile (Join-Path $tmp $archive)
    Invoke-WebRequest -Uri "$url.sha256"  -OutFile (Join-Path $tmp "$archive.sha256")

    $expected = (Get-Content (Join-Path $tmp "$archive.sha256") | Select-Object -First 1).Split(' ')[0]
    $actual   = (Get-FileHash -Algorithm SHA256 (Join-Path $tmp $archive)).Hash.ToLowerInvariant()
    if ($expected.ToLowerInvariant() -ne $actual) {
        Die "SHA-256 mismatch (expected $expected, got $actual)"
    }
    Write-Host 'SHA-256 verified.'

    # ---- Install ---------------------------------------------------------
    Expand-Archive -Path (Join-Path $tmp $archive) -DestinationPath $tmp -Force
    $stage = Join-Path $tmp "aphrody-$tag-$target"
    $src   = Join-Path $stage $Bin
    if (-not (Test-Path $src)) { Die "binary not found in archive: $src" }

    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }
    Copy-Item -Path $src -Destination (Join-Path $BinDir $Bin) -Force
    Write-Host "Installed: $(Join-Path $BinDir $Bin)"

    # ---- PATH update -----------------------------------------------------
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (($userPath -split ';') -notcontains $BinDir) {
        [Environment]::SetEnvironmentVariable('Path', "$userPath;$BinDir", 'User')
        Write-Host "Added $BinDir to user PATH. Open a new terminal to pick it up."
    } else {
        Write-Host 'Run: aphrody --version'
    }
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
