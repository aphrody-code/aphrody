Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$root  = 'C:\aphrody-purge'
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
New-Item -ItemType Directory -Path $root -Force | Out-Null

$paths = @(
    'C:\src\google-cli\packages\a2ui',
    'C:\src\google-cli\packages\gemini-cli',
    'C:\src\google-cli\packages\material-web',
    'C:\src\google-cli\packages\next.js',
    'C:\src\google-cli\packages\material-design-icons',
    'C:\src\google-cli\vendor\bun',
    'C:\src\google-cli\vendor\uv',
    'C:\src\google-cli\vendor\depot_tools',
    'C:\src\google-cli\vendor\a2a-rs',
    'C:\src\google-cli\crates\coreutils',
    'C:\src\google-cli\crates\util-linux'
)

foreach ($p in $paths) {
    if (Test-Path -LiteralPath $p) {
        $name = Split-Path $p -Leaf
        $dst  = Join-Path $root "dropped-$name-$stamp"
        try {
            Move-Item -LiteralPath $p -Destination $dst -Force
            Write-Host "moved $p"
        } catch {
            Write-Host "FAIL $p : $_"
        }
    }
}

Write-Host '---REMAINING in packages---'
Get-ChildItem -LiteralPath 'C:\src\google-cli\packages' -Directory | Select-Object Name | Format-Table -AutoSize
Write-Host '---REMAINING in vendor---'
Get-ChildItem -LiteralPath 'C:\src\google-cli\vendor' -Directory -ErrorAction SilentlyContinue | Select-Object Name | Format-Table -AutoSize
