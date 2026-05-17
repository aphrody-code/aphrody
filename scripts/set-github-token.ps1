Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$token = & gh auth token 2>$null
if (-not $token) {
    Write-Host 'ERROR: gh auth token returned empty. Run `gh auth login` first.' -ForegroundColor Red
    exit 1
}

[Environment]::SetEnvironmentVariable('GITHUB_PERSONAL_ACCESS_TOKEN', $token, 'User')
$len    = $token.Length
$prefix = $token.Substring(0, [Math]::Min(4, $len))

Write-Host "OK: GITHUB_PERSONAL_ACCESS_TOKEN set at User scope ($len chars, starts with $prefix****)." -ForegroundColor Green
Write-Host 'Restart Claude Code (new shell session) for the variable to be picked up by the github MCP server.' -ForegroundColor Yellow
