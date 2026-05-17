Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$src   = 'C:\src\aphrody\crates\google_os'
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$root  = 'C:\google-os-archive'
$dst   = Join-Path $root $stamp

New-Item -ItemType Directory -Path $root -Force | Out-Null

if (Test-Path -LiteralPath $src) {
    Move-Item -LiteralPath $src -Destination $dst -Force
    Write-Host "ARCHIVED: $src -> $dst"
} else {
    Write-Host "SOURCE MISSING: $src (already archived?)"
}

Write-Host "Source still exists: $((Test-Path -LiteralPath $src))"
Write-Host "Archive contents:"
Get-ChildItem -LiteralPath $dst -Force | Select-Object Name, Length | Format-Table -AutoSize
