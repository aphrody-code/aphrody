<#
.SYNOPSIS
    Scan ultra-rapide des manifestes du projet (Cargo / Bun / Turbo / TS / Python).

.DESCRIPTION
    Énumère et parse tous les fichiers manifestes du projet, extrait les méta-
    données importantes (name, version, deps count) et émet un JSON consolidé.

    Manifestes ciblés :
      Cargo.toml, package.json, turbo.json, tsconfig.json, bunfig.toml,
      pyproject.toml, uv.toml, deno.json, bun.lock, requirements.txt

    Best-practices Microsoft :
      - System.IO.Directory.EnumerateFiles avec EnumerationOptions
        (RecurseSubdirectories, IgnoreInaccessible, skip ReparsePoint)
      - Lecture via [System.IO.File]::ReadAllText (UTF8, sans BOM detection PS)
      - Parallel.ForEach pour traitement parallèle CPU-bound (parse)
      - ConcurrentBag<T> pour collecte thread-safe
      - Skip vendor/<deep>/node_modules et target/ (faux positifs)

.PARAMETER Root
    Racine projet (défaut = parent du script).

.PARAMETER OutputPath
    Chemin JSON de sortie.

.PARAMETER ExcludeDirs
    Sous-chemins à exclure pendant l'énumération (matching simple sur substring).
#>
[CmdletBinding()]
param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot),
    [string]$OutputPath = (Join-Path -Path (Split-Path -Parent $PSScriptRoot) -ChildPath 'manifest-scan.json'),
    [string[]]$ExcludeDirs = @(
        '\node_modules\', '\target\', '\.git\', '\dist\', '\build\',
        '\.cargo\registry\', '\.cache\', '\.next\', '\.turbo\',
        '\__pycache__\', '\.venv\', '\.mypy_cache\', '\.ruff_cache\'
    )
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

$ManifestPatterns = @(
    'Cargo.toml',
    'package.json',
    'turbo.json',
    'tsconfig.json',
    'tsconfig.*.json',
    'bunfig.toml',
    'pyproject.toml',
    'uv.toml',
    'deno.json',
    'deno.jsonc',
    'bun.lock',
    'requirements.txt'
)

$enumOpts = [System.IO.EnumerationOptions]::new()
$enumOpts.IgnoreInaccessible    = $true
$enumOpts.AttributesToSkip      = [System.IO.FileAttributes]::ReparsePoint
$enumOpts.RecurseSubdirectories = $true
$enumOpts.BufferSize            = 65536
$enumOpts.MatchCasing           = [System.IO.MatchCasing]::CaseInsensitive

$Throttle = [Environment]::ProcessorCount * 2

# --- TOML mini-parseur (name / version / dep counts) ---
function Read-TomlMeta {
    param([string]$Text)
    $meta = @{ name = $null; version = $null; type = 'toml' }
    if ($Text -match '(?m)^\s*name\s*=\s*"([^"]+)"')    { $meta.name    = $matches[1] }
    if ($Text -match '(?m)^\s*version\s*=\s*"([^"]+)"') { $meta.version = $matches[1] }
    # Dépendances : compter les sections [dependencies], [dev-dependencies], [build-dependencies]
    $depsCount = 0
    foreach ($section in @('dependencies', 'dev-dependencies', 'build-dependencies')) {
        $pattern = "(?ms)^\[$section\](.*?)(?=^\[|\z)"
        $m = [regex]::Match($Text, $pattern)
        if ($m.Success) {
            $depsCount += ([regex]::Matches($m.Groups[1].Value, '(?m)^\s*[\w-]+\s*=')).Count
        }
    }
    $meta.dependencyCount = $depsCount
    # Workspace members
    if ($Text -match '(?ms)\[workspace\].*?members\s*=\s*\[(.*?)\]') {
        $members = ([regex]::Matches($matches[1], '"([^"]+)"')).Count
        $meta.workspaceMembers = $members
    }
    return $meta
}

function Read-JsonMeta {
    param([string]$Text)
    $meta = @{ name = $null; version = $null; type = 'json' }
    try {
        $obj = $Text | ConvertFrom-Json -Depth 10 -EA Stop
        if ($obj.PSObject.Properties['name'])    { $meta.name    = $obj.name }
        if ($obj.PSObject.Properties['version']) { $meta.version = $obj.version }
        if ($obj.PSObject.Properties['dependencies'])    { $meta.dependencyCount    = ($obj.dependencies.PSObject.Properties).Count }
        if ($obj.PSObject.Properties['devDependencies']) { $meta.devDependencyCount = ($obj.devDependencies.PSObject.Properties).Count }
        if ($obj.PSObject.Properties['workspaces']) {
            $meta.workspaceMembers = if ($obj.workspaces -is [array]) { $obj.workspaces.Count } else { 1 }
        }
        if ($obj.PSObject.Properties['scripts'])  { $meta.scriptCount = ($obj.scripts.PSObject.Properties).Count }
        if ($obj.PSObject.Properties['pipeline']) { $meta.pipelineTasks = ($obj.pipeline.PSObject.Properties).Count }   # turbo
    } catch {
        $meta.parseError = $_.Exception.Message.Substring(0, [math]::Min(120, $_.Exception.Message.Length))
    }
    return $meta
}

Write-Host ""
Write-Host "===============================================================" -ForegroundColor Cyan
Write-Host "  SCAN-MANIFESTS  (NTFS native)" -ForegroundColor Cyan
Write-Host "  Root        : $Root" -ForegroundColor Cyan
Write-Host "  Patterns    : $($ManifestPatterns -join ', ')" -ForegroundColor Cyan
Write-Host "  Parallelism : $Throttle" -ForegroundColor Cyan
Write-Host "===============================================================" -ForegroundColor Cyan

$sw = [System.Diagnostics.Stopwatch]::StartNew()

# Énumère via EnumerateFiles + filtre pattern (un pass par pattern, accumule)
$allCandidates = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
foreach ($pattern in $ManifestPatterns) {
    try {
        foreach ($f in [System.IO.Directory]::EnumerateFiles($Root, $pattern, $enumOpts)) {
            # Filter excluded dirs (substring match on lowercased path)
            $skip = $false
            $lf = $f.ToLowerInvariant()
            foreach ($ex in $ExcludeDirs) { if ($lf.Contains($ex.ToLowerInvariant())) { $skip = $true; break } }
            if (-not $skip) { [void]$allCandidates.Add($f) }
        }
    } catch { Write-Warning "Pattern $pattern : $_" }
}

Write-Host ("  Manifests found: {0}" -f $allCandidates.Count) -ForegroundColor Gray

# Parse parallèle
$results = $allCandidates | ForEach-Object -ThrottleLimit $Throttle -Parallel {
    $path    = $_
    $rootRef = $using:Root

    try {
        $text     = [System.IO.File]::ReadAllText($path, [System.Text.Encoding]::UTF8)
        $fileName = [System.IO.Path]::GetFileName($path)
        $relPath  = $path.Substring($rootRef.Length).TrimStart('\','/')
        $sizeKB   = [math]::Round((Get-Item $path).Length / 1KB, 2)

        $meta = @{}
        if ($fileName -eq 'bun.lock') {
            $meta = @{ type = 'lockfile' }
        }
        elseif ($fileName -match '\.json[c]?$') {
            $meta = @{ type = 'json' }
            try {
                $obj = $text | ConvertFrom-Json -Depth 10 -EA Stop
                if ($obj.PSObject.Properties['name'])             { $meta.name              = $obj.name }
                if ($obj.PSObject.Properties['version'])          { $meta.version           = $obj.version }
                if ($obj.PSObject.Properties['dependencies'])     { $meta.dependencyCount   = ($obj.dependencies.PSObject.Properties).Count }
                if ($obj.PSObject.Properties['devDependencies'])  { $meta.devDependencyCount= ($obj.devDependencies.PSObject.Properties).Count }
                if ($obj.PSObject.Properties['workspaces'])       { $meta.workspaceMembers  = if ($obj.workspaces -is [array]) { $obj.workspaces.Count } else { 1 } }
                if ($obj.PSObject.Properties['scripts'])          { $meta.scriptCount       = ($obj.scripts.PSObject.Properties).Count }
                if ($obj.PSObject.Properties['pipeline'])         { $meta.pipelineTasks     = ($obj.pipeline.PSObject.Properties).Count }
            } catch {
                $meta.parseError = $_.Exception.Message.Substring(0, [math]::Min(120, $_.Exception.Message.Length))
            }
        }
        elseif ($fileName -match '\.toml$') {
            $meta = @{ type = 'toml' }
            if ($text -match '(?m)^\s*name\s*=\s*"([^"]+)"')    { $meta.name    = $matches[1] }
            if ($text -match '(?m)^\s*version\s*=\s*"([^"]+)"') { $meta.version = $matches[1] }
            $depsCount = 0
            foreach ($section in @('dependencies', 'dev-dependencies', 'build-dependencies')) {
                $pattern = "(?ms)^\[$section\](.*?)(?=^\[|\z)"
                $m = [regex]::Match($text, $pattern)
                if ($m.Success) { $depsCount += ([regex]::Matches($m.Groups[1].Value, '(?m)^\s*[\w-]+\s*=')).Count }
            }
            $meta.dependencyCount = $depsCount
            if ($text -match '(?ms)\[workspace\].*?members\s*=\s*\[(.*?)\]') {
                $meta.workspaceMembers = ([regex]::Matches($matches[1], '"([^"]+)"')).Count
            }
        }
        elseif ($fileName -eq 'requirements.txt') {
            $lines = ($text -split "`n" | Where-Object { $_.Trim() -and -not $_.StartsWith('#') }).Count
            $meta = @{ type = 'pip-requirements'; dependencyCount = $lines }
        }
        else {
            $meta = @{ type = 'unknown' }
        }

        [PSCustomObject]@{
            path     = $relPath
            fileName = $fileName
            sizeKB   = $sizeKB
            meta     = $meta
        }
    } catch { return }
}

$sw.Stop()

# Stats par type
$byType = $results | Group-Object fileName | Sort-Object Count -Descending | ForEach-Object {
    [PSCustomObject]@{ pattern = $_.Name; count = $_.Count }
}

$report = [PSCustomObject]@{
    generatedAt  = (Get-Date).ToString('o')
    root         = $Root
    elapsedMs    = $sw.ElapsedMilliseconds
    totalCount   = $results.Count
    byType       = @($byType)
    manifests    = @($results | Sort-Object path)
}

$json = $report | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText($OutputPath, $json, [System.Text.UTF8Encoding]::new($false))

Write-Host ""
Write-Host "=== MANIFEST COUNTS BY TYPE ===" -ForegroundColor Yellow
$byType | Format-Table -AutoSize

# Workspaces et root crates
Write-Host "=== WORKSPACE ROOTS ===" -ForegroundColor Yellow
$results | Where-Object { $_.meta.PSObject.Properties['workspaceMembers'] } |
    Select-Object path, @{N='members';E={$_.meta.workspaceMembers}} |
    Format-Table -AutoSize

Write-Host ""
Write-Host ("Done in {0:N1} s — report: $OutputPath" -f ($sw.ElapsedMilliseconds / 1000)) -ForegroundColor Green
