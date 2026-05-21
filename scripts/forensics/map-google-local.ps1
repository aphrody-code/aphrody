# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
#
# map-google-local.ps1 — reproducible, non-interactive forensic map of the
# local Google desktop-app install directory (WebView2 host + profiles).
#
# Produces a JSON tree (path, size, ext, sha256-of-small-files, last-modified)
# under an output directory the caller keeps gitignored (var/data/...).
#
# SECURITY CONTRACT (inviolable):
#   - File CONTENTS of secret-looking artefacts (cookies, credentials, token
#     stores, leveldb/sqlite under user-data) are NEVER opened. Only metadata
#     (path/size/ext/mtime) is recorded for those.
#   - SHA-256 is computed ONLY for small (<= $MaxHashBytes) files whose path
#     does NOT match the secret denylist below. Hashing a file does read its
#     bytes, so the denylist is what enforces "no secret bytes touched".
#   - Nothing is ever written outside -OutDir. No network access.
#
# Usage (non-interactive):
#   pwsh -NoProfile -File scripts/forensics/map-google-local.ps1 `
#       -Target "$env:LOCALAPPDATA\Google\Google" `
#       -OutDir  var/data/google-local-map
[CmdletBinding()]
param(
    [string]$Target = (Join-Path $env:LOCALAPPDATA 'Google\Google'),
    [string]$OutDir = 'var/data/google-local-map',
    [int]$MaxHashBytes = 1048576  # 1 MiB: hash only small files
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $Target)) {
    Write-Error "target not found: $Target"
    exit 2
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# Secret denylist — substrings matched case-insensitively against the FULL
# path. Files whose path contains any of these are recorded metadata-only and
# are NEVER hashed/opened. Mirrors Chromium/Electron secret stores.
$SecretPatterns = @(
    'cookies', 'login data', 'web data', 'credential', 'token',
    'local state', 'leveldb', '\.ldb', 'manifest-', 'session storage',
    'local storage', 'indexeddb', 'network\\', 'autofill', 'password',
    'user_history', 'preferences.txtpb', 'global_preferences'
)

function Test-Secret([string]$p) {
    $lower = $p.ToLowerInvariant()
    foreach ($pat in $SecretPatterns) {
        if ($lower -match [regex]::Escape($pat) -or $lower -like "*$($pat.Replace('\',''))*") {
            return $true
        }
    }
    return $false
}

$entries = New-Object System.Collections.Generic.List[object]
$secretCount = 0
$hashedCount = 0

Get-ChildItem -LiteralPath $Target -Recurse -Force -File -ErrorAction SilentlyContinue | ForEach-Object {
    $f = $_
    $isSecret = Test-Secret $f.FullName
    $ext = if ($f.Extension) { $f.Extension.TrimStart('.').ToLowerInvariant() } else { $null }
    $sha = $null
    if (-not $isSecret -and $f.Length -le $MaxHashBytes) {
        try {
            $sha = (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
            $hashedCount++
        } catch { $sha = $null }
    }
    if ($isSecret) { $secretCount++ }
    $entries.Add([ordered]@{
        path        = $f.FullName
        rel         = $f.FullName.Substring($Target.Length).TrimStart('\','/')
        size        = $f.Length
        ext         = $ext
        sha256      = $sha
        modified    = $f.LastWriteTimeUtc.ToString('o')
        mtime_unix  = [int64]([DateTimeOffset]$f.LastWriteTimeUtc).ToUnixTimeSeconds()
        secret_meta_only = $isSecret
    })
}

$report = [ordered]@{
    target          = $Target
    generated_at    = (Get-Date).ToUniversalTime().ToString('o')
    file_count      = $entries.Count
    hashed_count    = $hashedCount
    secret_meta_only_count = $secretCount
    max_hash_bytes  = $MaxHashBytes
    files           = $entries
}

$outFile = Join-Path $OutDir 'tree.json'
$report | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $outFile -Encoding UTF8
Write-Host "wrote $outFile : $($entries.Count) files, $hashedCount hashed, $secretCount secret-meta-only"
