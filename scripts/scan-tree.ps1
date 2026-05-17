<#
.SYNOPSIS
    Scan ultra-rapide (NTFS) de vendor/, packages/, crates/ → JSON.

.DESCRIPTION
    Best-practices Microsoft pour I/O massif :
      - System.IO.Directory.EnumerateFiles (streaming, lazy)
      - EnumerationOptions { IgnoreInaccessible; AttributesToSkip=ReparsePoint;
        RecurseSubdirectories=true } pour skip junctions sans Test-Path
      - System.Threading.Tasks.Parallel.ForEach (low-overhead vs PS Parallel)
      - System.Text.Json.JsonSerializer (rapide vs ConvertTo-Json)
      - ConcurrentBag<T> pour agrégation thread-safe sans lock

    Sortie :
      Tableau JSON [{ group, name, files, bytes, mb, gb, topExt }] +
      résumé par groupe (vendor / packages / crates) + total racine projet.

.PARAMETER Root
    Racine du projet (par défaut = parent du script).

.PARAMETER OutputPath
    Chemin du fichier JSON de sortie.

.PARAMETER TopExtCount
    Nombre d'extensions à inclure dans le top par dossier (défaut 5).

.EXAMPLE
    pwsh scripts/scan-tree.ps1
    pwsh scripts/scan-tree.ps1 -OutputPath C:\aphrody-backup\tree-scan.json
#>
[CmdletBinding()]
param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot),
    [string]$OutputPath = (Join-Path -Path (Split-Path -Parent $PSScriptRoot) -ChildPath 'tree-scan.json'),
    [int]$TopExtCount = 5
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

Add-Type -AssemblyName 'System.Text.Json'

# Skip junctions/symlinks (ReparsePoint) ET fichiers inaccessibles
$enumOpts = [System.IO.EnumerationOptions]::new()
$enumOpts.IgnoreInaccessible    = $true
$enumOpts.AttributesToSkip      = [System.IO.FileAttributes]::ReparsePoint
$enumOpts.RecurseSubdirectories = $true
$enumOpts.BufferSize            = 65536

$Throttle = [Environment]::ProcessorCount * 2

Write-Host ""
Write-Host "===============================================================" -ForegroundColor Cyan
Write-Host "  SCAN-TREE  (NTFS native, parallel)" -ForegroundColor Cyan
Write-Host "  Root        : $Root" -ForegroundColor Cyan
Write-Host "  Parallelism : $Throttle" -ForegroundColor Cyan
Write-Host "===============================================================" -ForegroundColor Cyan

$globalStart = [System.Diagnostics.Stopwatch]::StartNew()

# Énumère les sous-dossiers directs des 3 groupes
$tasks = New-Object System.Collections.Generic.List[PSObject]
foreach ($group in @('vendor', 'packages', 'crates')) {
    $base = Join-Path $Root $group
    if (-not [System.IO.Directory]::Exists($base)) { continue }
    foreach ($d in [System.IO.Directory]::EnumerateDirectories($base, '*', [System.IO.SearchOption]::TopDirectoryOnly)) {
        $tasks.Add([PSCustomObject]@{ DirPath = $d; Group = $group; Name = [System.IO.Path]::GetFileName($d) })
    }
}

$results = $tasks | ForEach-Object -ThrottleLimit $Throttle -Parallel {
    $task = $_
    $topN = $using:TopExtCount

    $opts = [System.IO.EnumerationOptions]::new()
    $opts.IgnoreInaccessible    = $true
    $opts.AttributesToSkip      = [System.IO.FileAttributes]::ReparsePoint
    $opts.RecurseSubdirectories = $true
    $opts.BufferSize            = 65536

    if (-not [System.IO.Directory]::Exists($task.DirPath)) { return }

    $bytes    = [int64]0
    $files    = [int]0
    $extCount = @{}

    try {
        foreach ($file in [System.IO.Directory]::EnumerateFiles($task.DirPath, '*', $opts)) {
            try {
                $fi     = [System.IO.FileInfo]::new($file)
                $bytes += $fi.Length
                $files++
                $ext    = if ([string]::IsNullOrEmpty($fi.Extension)) { '<noext>' } else { $fi.Extension.ToLowerInvariant() }
                if ($extCount.ContainsKey($ext)) { $extCount[$ext]++ } else { $extCount[$ext] = 1 }
            } catch { continue }
        }
    } catch { return }

    $topExts = $extCount.GetEnumerator() |
        Sort-Object Value -Descending |
        Select-Object -First $topN |
        ForEach-Object { @{ ext = $_.Key; count = $_.Value } }

    [PSCustomObject]@{
        group  = $task.Group
        name   = $task.Name
        files  = $files
        bytes  = $bytes
        mb     = [math]::Round($bytes / 1MB, 2)
        gb     = [math]::Round($bytes / 1GB, 3)
        topExt = @($topExts)
    }
}

$globalStart.Stop()

# Totaux par groupe
$byGroup = $results | Group-Object group | ForEach-Object {
    $sumBytes = ($_.Group | Measure-Object bytes -Sum).Sum
    $sumFiles = ($_.Group | Measure-Object files -Sum).Sum
    [PSCustomObject]@{
        group   = $_.Name
        dirs    = $_.Count
        files   = $sumFiles
        bytes   = $sumBytes
        mb      = [math]::Round($sumBytes / 1MB, 2)
        gb      = [math]::Round($sumBytes / 1GB, 3)
    }
}

# Total racine projet (un seul scan, mais le plus gros)
$rootBytes = [int64]0
$rootFiles = [int]0
foreach ($f in [System.IO.Directory]::EnumerateFiles($Root, '*', $enumOpts)) {
    try {
        $rootBytes += [System.IO.FileInfo]::new($f).Length
        $rootFiles++
    } catch { continue }
}

$report = [PSCustomObject]@{
    generatedAt  = (Get-Date).ToString('o')
    root         = $Root
    elapsedMs    = $globalStart.ElapsedMilliseconds
    rootTotal    = @{ files = $rootFiles; bytes = $rootBytes; gb = [math]::Round($rootBytes / 1GB, 3) }
    byGroup      = @($byGroup)
    directories  = @($results | Sort-Object bytes -Descending)
}

# System.Text.Json (plus rapide que ConvertTo-Json pour gros payloads)
$jsonOpts = [System.Text.Json.JsonSerializerOptions]::new()
$jsonOpts.WriteIndented = $true
# Cast en JsonElement via temp ConvertTo-Json (PSCustomObject n'est pas serializable direct)
$json = $report | ConvertTo-Json -Depth 8
[System.IO.File]::WriteAllText($OutputPath, $json, [System.Text.UTF8Encoding]::new($false))

# Console summary
Write-Host ""
Write-Host "=== PER-GROUP TOTALS ===" -ForegroundColor Yellow
$byGroup | Format-Table group,dirs,files,gb -AutoSize

Write-Host "=== TOP 20 DIRECTORIES BY SIZE ===" -ForegroundColor Yellow
$results | Sort-Object bytes -Descending | Select-Object -First 20 |
    Format-Table @{N='Group';E={$_.group}},@{N='Name';E={$_.name}},@{N='Files';E={$_.files}},@{N='MB';E={$_.mb}} -AutoSize

Write-Host "=== ROOT TOTAL ===" -ForegroundColor Yellow
Write-Host ("  {0:N0} files / {1:N2} GB" -f $rootFiles, ($rootBytes / 1GB))

Write-Host ""
Write-Host ("Done in {0:N1} s — report: $OutputPath" -f ($globalStart.ElapsedMilliseconds / 1000)) -ForegroundColor Green
