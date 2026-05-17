Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$crateNames = $args
if (-not $crateNames -or $crateNames.Count -eq 0) {
    Write-Error "Usage: archive-crates.ps1 <crate1> [crate2] [crate3] ..."
    exit 2
}

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$root  = 'C:\aphrody-archive'
New-Item -ItemType Directory -Path $root -Force | Out-Null

foreach ($crate in $crateNames) {
    $src = "C:\src\google-cli\crates\$crate"
    $dst = Join-Path $root "${crate}-$stamp"

    if (Test-Path -LiteralPath $src) {
        Move-Item -LiteralPath $src -Destination $dst -Force
        Write-Host "ARCHIVED: $src -> $dst"
    } else {
        Write-Host "SKIP (missing): $src"
    }
}

Write-Host '---'
Write-Host 'Archive root contents:'
Get-ChildItem -LiteralPath $root -Directory -Force |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 8 Name, LastWriteTime |
    Format-Table -AutoSize
