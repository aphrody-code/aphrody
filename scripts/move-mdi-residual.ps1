Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$src   = 'C:\src\aphrody\packages\material-design-icons'
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
