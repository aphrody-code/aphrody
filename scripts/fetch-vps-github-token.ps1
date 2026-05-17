#requires -Version 5.1
$ErrorActionPreference = 'Stop'

function Test-HasProp([object]$Obj, [string]$Name) {
    if ($null -eq $Obj) { return $false }
    return ($Obj.PSObject.Properties | Where-Object { $_.Name -eq $Name }) -ne $null
}

# Pulls GITHUB_PERSONAL_ACCESS_TOKEN from the VPS (via `gh auth token` over SSH)
# and persists it locally in:
#   1. User-scope environment variable (inherited by all future shells)
#   2. %USERPROFILE%\.claude\settings.json under env.GITHUB_PERSONAL_ACCESS_TOKEN
#
# The token never touches disk on the local side except inside settings.json.
# It is never echoed to stdout.

$VpsHost      = 'vps-tunnel'
$SettingsPath = Join-Path $env:USERPROFILE '.claude\settings.json'
$Stamp        = Get-Date -Format 'yyyyMMdd-HHmmss'
$BackupPath   = "$SettingsPath.bak-$Stamp"

Write-Host "[1/5] Verifying SSH + gh auth on $VpsHost..." -ForegroundColor Cyan
$probe = ssh -q -o BatchMode=yes -o ConnectTimeout=8 $VpsHost 'gh auth status >/dev/null 2>&1 && echo OK || echo KO' 2>$null
if ($LASTEXITCODE -ne 0 -or $probe -ne 'OK') {
    Write-Host "ERROR: cannot reach $VpsHost or gh is not authenticated there." -ForegroundColor Red
    Write-Host "       Try:  ssh $VpsHost 'gh auth status'" -ForegroundColor Yellow
    exit 1
}

Write-Host "[2/5] Pulling token (in-memory only, never echoed)..." -ForegroundColor Cyan
$token = (ssh -q -o BatchMode=yes -o ConnectTimeout=8 $VpsHost 'gh auth token' 2>$null) -replace "`r|`n",''
if (-not $token -or $token.Length -lt 20) {
    Write-Host "ERROR: empty or invalid token received from VPS." -ForegroundColor Red
    exit 1
}
$len    = $token.Length
$prefix = $token.Substring(0, [Math]::Min(4, $len))

Write-Host "[3/5] Persisting GITHUB_PERSONAL_ACCESS_TOKEN at User scope..." -ForegroundColor Cyan
[Environment]::SetEnvironmentVariable('GITHUB_PERSONAL_ACCESS_TOKEN', $token, 'User')

Write-Host "[4/5] Merging into $SettingsPath..." -ForegroundColor Cyan
if (Test-Path $SettingsPath) {
    Copy-Item $SettingsPath $BackupPath -Force
    $json = Get-Content $SettingsPath -Raw | ConvertFrom-Json
} else {
    New-Item -ItemType Directory -Force -Path (Split-Path $SettingsPath) | Out-Null
    $json = [pscustomobject]@{}
}
if (-not (Test-HasProp $json 'env')) {
    $json | Add-Member -NotePropertyName 'env' -NotePropertyValue ([pscustomobject]@{})
}
if (Test-HasProp $json.env 'GITHUB_PERSONAL_ACCESS_TOKEN') {
    $json.env.GITHUB_PERSONAL_ACCESS_TOKEN = $token
} else {
    $json.env | Add-Member -NotePropertyName 'GITHUB_PERSONAL_ACCESS_TOKEN' -NotePropertyValue $token
}
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($SettingsPath, ($json | ConvertTo-Json -Depth 32), $utf8NoBom)

Write-Host "[5/5] Wiping in-memory token..." -ForegroundColor Cyan
Remove-Variable token -ErrorAction SilentlyContinue
[System.GC]::Collect()

Write-Host ''
Write-Host "OK: $len chars, starts with $prefix****" -ForegroundColor Green
Write-Host "  - User scope set (new shell required for inheritance)"
if (Test-Path $BackupPath) {
    Write-Host "  - settings.json updated (backup: $BackupPath)"
} else {
    Write-Host "  - settings.json created"
}
Write-Host "  - Restart Claude Code in a fresh PowerShell so .mcp.json picks up the token."
