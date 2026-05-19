# SPDX-License-Identifier: Apache-2.0
# Purge residual material-design-icons subdirs to C:\aphrody-purge\.
# Resolves the repo root via `git rev-parse --show-toplevel` so the script is
# portable across clone locations (previously hard-coded to `C:\src\aphrody`).
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

# Find the repo root from the script's own location (works even when invoked
# from a different cwd) and locate the mdi vendored package inside it.
$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot  = (git -C $scriptDir rev-parse --show-toplevel).Trim()
if (-not $repoRoot) {
    throw "Cannot resolve repo root from $scriptDir (not in a git repo?)"
}
$src   = Join-Path $repoRoot 'packages\material-design-icons'

$stamp = Get-Date -Format 'yyyyMMddHHmmss'
$purge = "C:\aphrody-purge\mdi-residual-$stamp"
New-Item -ItemType Directory -Path $purge -Force | Out-Null

foreach ($sub in @('ios','png','src','symbols','android','font','LICENSE','README.md')) {
    $s = Join-Path $src $sub
    if (Test-Path -LiteralPath $s) {
        Write-Host "Moving $sub"
        try {
            Move-Item -LiteralPath $s -Destination (Join-Path $purge $sub) -Force
            Write-Host "  ok $sub"
        } catch {
            Write-Host "  FAIL $sub : $_"
        }
    }
}

Write-Host '---REMAINING---'
Get-ChildItem -LiteralPath $src -Force | Select-Object Name | Format-Table -AutoSize
