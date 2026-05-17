<#
.SYNOPSIS
    Rename complet et sûr d'un projet polyglotte (Rust + Bun + C/C++ + Python).

.DESCRIPTION
    Renomme le projet sous TOUTES ses formes typographiques tout en
    préservant les mentions légitimes de la racine `google` (Google APIs,
    Chromium upstream, Google LLC copyright, packages npm officiels).

    Formes traitées (par défaut aphrody → aphrody) :
      kebab-case          aphrody       → aphrody
      snake_case          aphrody       → aphrody
      flat                aphrody        → aphrody
      PascalCase          Aphrody        → Aphrody
      Display name        "Aphrody"     → "Aphrody"
      UPPER ENV           APHRODY       → APHRODY
      Path segment        \aphrody\     → \aphrody\
      Path segment        /aphrody/     → /aphrody/

    NB : crates `google_os`, `google_mcp`, `google_kv` ne sont PAS touchés
    (pas de match exact sur "aphrody").

    Exclusions HARD :
      .git, vendor, opt, target, node_modules, build, dist, .cargo/registry
      *.lock, *.tar*, *.zip, *.exe, *.dll, *.pdb, *.so, *.dylib, *.a, *.o,
      *.rlib, *.rmeta, *.wasm, *.png, *.jpg, *.ico, *.woff*, LICENSE*

.PARAMETER OldName
    Nom kebab-case actuel (défaut "aphrody").

.PARAMETER NewName
    Nom kebab-case cible (défaut "aphrody").

.PARAMETER Root
    Racine du projet (défaut = répertoire parent du script).

.PARAMETER DryRun
    Switch — ne modifie rien, génère uniquement le rapport.

.PARAMETER Apply
    Switch — applique les modifications (mutuellement exclusif avec DryRun).

.PARAMETER ReportPath
    Chemin du rapport JSON (défaut "rename-report.json" à la racine).

.PARAMETER ThrottleLimit
    Niveau de parallélisme (défaut = 2 * NumberOfLogicalProcessors).

.EXAMPLE
    pwsh scripts/rename-project.ps1 -DryRun
    pwsh scripts/rename-project.ps1 -Apply -ReportPath C:\aphrody-backup\rename.json

.NOTES
    Best practices appliquées :
      - PS 7+ ForEach-Object -Parallel pour scan + rewrite
      - Pré-filtrage Select-String -List (early-exit dès match)
      - Détection binaire heuristique (NULL byte dans les premiers 8 KB)
      - Préserve encoding UTF-8 sans BOM (Set-Content -NoNewline -Encoding utf8NoBOM)
      - Transactions atomiques (write to .tmp puis Move-Item -Force)
      - Pattern bornés (word-boundary) pour éviter les faux positifs
#>
[CmdletBinding(DefaultParameterSetName = 'DryRun')]
param(
    [string]$OldName = 'aphrody',
    [string]$NewName = 'aphrody',
    [string]$Root = (Split-Path -Parent $PSScriptRoot),

    [Parameter(ParameterSetName = 'DryRun')]
    [switch]$DryRun,

    [Parameter(ParameterSetName = 'Apply')]
    [switch]$Apply,

    [string]$ReportPath = (Join-Path -Path (Split-Path -Parent $PSScriptRoot) -ChildPath 'rename-report.json'),

    [int]$ThrottleLimit = ([Environment]::ProcessorCount * 2)
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

if (-not $DryRun -and -not $Apply) {
    Write-Host "Default mode: DryRun (pass -Apply to actually rewrite)" -ForegroundColor Yellow
    $DryRun = $true
}

# ============================================================================
# CONFIGURATION
# ============================================================================

# Translittération multi-casse : produit la liste exhaustive des substitutions
function Get-RenameRules {
    param([string]$Old, [string]$New)

    $oldKebab = $Old.ToLowerInvariant()
    $newKebab = $New.ToLowerInvariant()

    $oldSnake = $oldKebab -replace '-', '_'
    $newSnake = $newKebab -replace '-', '_'

    $oldFlat = $oldKebab -replace '-', ''
    $newFlat = $newKebab -replace '-', ''

    # PascalCase : "aphrody" -> "Aphrody", "aphrody-os" -> "AphrodyOs"
    $oldPascal = ($oldKebab -split '-' | ForEach-Object {
        if ($_.Length -gt 0) { $_.Substring(0, 1).ToUpperInvariant() + $_.Substring(1).ToLowerInvariant() }
    }) -join ''
    $newPascal = ($newKebab -split '-' | ForEach-Object {
        if ($_.Length -gt 0) { $_.Substring(0, 1).ToUpperInvariant() + $_.Substring(1).ToLowerInvariant() }
    }) -join ''

    # Display name : "Aphrody" / "Aphrody OS"
    $oldDisplay = ($oldKebab -split '-' | ForEach-Object {
        if ($_.Length -gt 0) {
            $up = $_.ToUpperInvariant()
            # CLI / OS / API / GUI conventionnellement tout-majuscules
            if ($_.Length -le 3 -and $_ -match '^[a-z]+$') { $up }
            else { $_.Substring(0, 1).ToUpperInvariant() + $_.Substring(1).ToLowerInvariant() }
        }
    }) -join ' '
    $newDisplay = ($newKebab -split '-' | ForEach-Object {
        if ($_.Length -gt 0) {
            $up = $_.ToUpperInvariant()
            if ($_.Length -le 3 -and $_ -match '^[a-z]+$') { $up }
            else { $_.Substring(0, 1).ToUpperInvariant() + $_.Substring(1).ToLowerInvariant() }
        }
    }) -join ' '

    $oldUpper = $oldSnake.ToUpperInvariant()
    $newUpper = $newSnake.ToUpperInvariant()

    # Liste ordonnée : plus spécifique d'abord (sinon flat avale kebab)
    @(
        [PSCustomObject]@{ Form = 'kebab';   Pattern = "(?<![\w-])$([regex]::Escape($oldKebab))(?![\w-])";   Replacement = $newKebab }
        [PSCustomObject]@{ Form = 'snake';   Pattern = "(?<!\w)$([regex]::Escape($oldSnake))(?!\w)";         Replacement = $newSnake }
        [PSCustomObject]@{ Form = 'pascal';  Pattern = "(?<!\w)$([regex]::Escape($oldPascal))(?!\w)";        Replacement = $newPascal }
        [PSCustomObject]@{ Form = 'display'; Pattern = [regex]::Escape($oldDisplay);                          Replacement = $newDisplay }
        [PSCustomObject]@{ Form = 'upper';   Pattern = "(?<!\w)$([regex]::Escape($oldUpper))(?!\w)";          Replacement = $newUpper }
        [PSCustomObject]@{ Form = 'flat';    Pattern = "(?<![\w-])$([regex]::Escape($oldFlat))(?![\w-])";    Replacement = $newFlat }
        [PSCustomObject]@{ Form = 'path_bs'; Pattern = "\\$([regex]::Escape($oldKebab))\\";                   Replacement = "\$newKebab\" }
        [PSCustomObject]@{ Form = 'path_fs'; Pattern = "/$([regex]::Escape($oldKebab))/";                     Replacement = "/$newKebab/" }
    )
}

$ExcludeDirNames = @(
    '.git', 'vendor', 'opt', 'target', 'node_modules', 'build', 'dist',
    '.cargo', '.venv', '__pycache__', '.cache', '.next', '.turbo',
    'msys64', 'glibc', '.idea', '.vscode',
    'material-design-icons',  # 2.56M upstream files — gitignored placeholder
    'electron-prebuilt'       # binaries
)

$ExcludeExtensions = @(
    '.lock', '.tar', '.tar.gz', '.tar.xz', '.tar.zst', '.tgz', '.zip',
    '.7z', '.rar', '.gz', '.xz', '.zst', '.bz2',
    '.exe', '.dll', '.pdb', '.so', '.dylib', '.a', '.o', '.obj',
    '.rlib', '.rmeta', '.wasm', '.bc',
    '.png', '.jpg', '.jpeg', '.gif', '.ico', '.bmp', '.webp', '.svg',
    '.mp3', '.mp4', '.mov', '.webm', '.ogg', '.wav',
    '.woff', '.woff2', '.ttf', '.otf', '.eot',
    '.db', '.sqlite', '.sqlite3', '.profraw', '.profdata',
    '.bin', '.dat', '.idx', '.pack'
)

$ExcludeFileNames = @(
    'LICENSE', 'LICENSE.md', 'LICENSE.txt', 'LICENSE-MIT', 'LICENSE-APACHE',
    'COPYING', 'COPYING.txt', 'NOTICE', 'NOTICE.txt',
    'AUTHORS', 'CONTRIBUTORS', 'MAINTAINERS', 'CODEOWNERS',
    'Cargo.lock', 'bun.lock', 'package-lock.json', 'pnpm-lock.yaml', 'yarn.lock',
    'uv.lock', 'poetry.lock'
)

# ============================================================================
# SCAN
# ============================================================================

Write-Host ""
Write-Host "===============================================" -ForegroundColor Cyan
Write-Host "  PROJECT RENAME: $OldName -> $NewName" -ForegroundColor Cyan
Write-Host "  Root           : $Root" -ForegroundColor Cyan
Write-Host "  Mode           : $(if ($Apply) {'APPLY (writes)'} else {'DRY-RUN (no changes)'})" -ForegroundColor Cyan
Write-Host "  Throttle       : $ThrottleLimit" -ForegroundColor Cyan
Write-Host "===============================================" -ForegroundColor Cyan

$rules = Get-RenameRules -Old $OldName -New $NewName

Write-Host "`nRENAME RULES:" -ForegroundColor Yellow
$rules | ForEach-Object {
    Write-Host ("  [{0,-8}] {1}  =>  {2}" -f $_.Form, $_.Pattern, $_.Replacement)
}

Write-Host "`n[1/3] Enumerating candidate files..." -ForegroundColor Green

$excludeDirRegex = '\\(' + (($ExcludeDirNames | ForEach-Object { [regex]::Escape($_) }) -join '|') + ')\\'

$allFiles = Get-ChildItem -Path $Root -Recurse -File -Force -ErrorAction SilentlyContinue |
    Where-Object {
        $rel = $_.FullName.Substring($Root.Length)
        # Exclude excluded dirs
        if ($rel -match $excludeDirRegex) { return $false }
        # Exclude extension blacklist
        $ext = $_.Extension.ToLowerInvariant()
        if ($ExcludeExtensions -contains $ext) { return $false }
        # Composite ext (.tar.gz etc.)
        if ($_.Name -match '\.tar\.(gz|xz|zst|bz2)$') { return $false }
        # Exclude filename blacklist
        if ($ExcludeFileNames -contains $_.Name) { return $false }
        # Skip > 5 MB (probably generated)
        if ($_.Length -gt 5MB) { return $false }
        return $true
    }

Write-Host ("  Candidates: {0} files" -f $allFiles.Count) -ForegroundColor Gray

# ============================================================================
# DETECT (parallel)
# ============================================================================

Write-Host "`n[2/3] Detecting matches (parallel x$ThrottleLimit)..." -ForegroundColor Green

# Pré-compile patterns
$compiledRules = $rules | ForEach-Object {
    [PSCustomObject]@{
        Form        = $_.Form
        Regex       = [regex]::new($_.Pattern, [System.Text.RegularExpressions.RegexOptions]::Compiled)
        Replacement = $_.Replacement
    }
}

$findings = $allFiles | ForEach-Object -Parallel {
    $file  = $_
    $rules = $using:compiledRules

    # Fast binary detection : NULL byte in first 8 KB
    try {
        $bytes = [System.IO.File]::ReadAllBytes($file.FullName)
    } catch { return }
    if ($bytes.Length -eq 0) { return }
    $sniff = if ($bytes.Length -gt 8192) { $bytes[0..8191] } else { $bytes }
    if ($sniff -contains 0) { return }  # binary

    $text = [System.Text.Encoding]::UTF8.GetString($bytes)
    $hits = @()
    foreach ($r in $rules) {
        $m = $r.Regex.Matches($text)
        if ($m.Count -gt 0) {
            $hits += [PSCustomObject]@{
                Form    = $r.Form
                Count   = $m.Count
                Samples = @($m | Select-Object -First 3 | ForEach-Object { $_.Value })
            }
        }
    }
    if ($hits.Count -gt 0) {
        [PSCustomObject]@{
            Path      = $file.FullName.Substring($using:Root.Length).TrimStart('\','/')
            SizeBytes = $file.Length
            TotalHits = ($hits | Measure-Object Count -Sum).Sum
            Hits      = $hits
        }
    }
} -ThrottleLimit $ThrottleLimit

$findings = @($findings | Where-Object { $_ })

Write-Host ("  Files with hits: {0}" -f $findings.Count) -ForegroundColor Gray
$totalHits = ($findings | Measure-Object TotalHits -Sum).Sum
Write-Host ("  Total textual hits: {0}" -f $totalHits) -ForegroundColor Gray

# Renamed files/dirs (path containing OldName)
$pathRenames = @(Get-ChildItem -Path $Root -Recurse -Force -ErrorAction SilentlyContinue |
    Where-Object {
        $rel = $_.FullName.Substring($Root.Length)
        if ($rel -match $excludeDirRegex) { return $false }
        $_.Name -match "(?<![\w-])$([regex]::Escape($OldName))(?![\w-])"
    } |
    ForEach-Object {
        [PSCustomObject]@{
            OldFull = $_.FullName
            NewFull = Join-Path -Path $_.Directory.FullName -ChildPath ($_.Name -replace "(?<![\w-])$([regex]::Escape($OldName))(?![\w-])", $NewName)
            Type    = if ($_.PSIsContainer) { 'dir' } else { 'file' }
        }
    })

Write-Host ("  Path renames pending: {0}" -f $pathRenames.Count) -ForegroundColor Gray

# ============================================================================
# REPORT
# ============================================================================

$report = [PSCustomObject]@{
    GeneratedAt    = (Get-Date).ToString('o')
    Mode           = if ($Apply) { 'apply' } else { 'dry-run' }
    Root           = $Root
    OldName        = $OldName
    NewName        = $NewName
    Rules          = $rules
    Excluded       = [PSCustomObject]@{
        Directories = $ExcludeDirNames
        Extensions  = $ExcludeExtensions
        FileNames   = $ExcludeFileNames
    }
    CandidatesScanned = $allFiles.Count
    FilesWithHits     = $findings.Count
    TotalHits         = $totalHits
    PathRenames       = $pathRenames
    Findings          = $findings | Sort-Object TotalHits -Descending
}

$report | ConvertTo-Json -Depth 8 | Set-Content -Path $ReportPath -Encoding utf8NoBOM
Write-Host "`nReport: $ReportPath" -ForegroundColor Cyan

# Top-10 summary to console
Write-Host "`nTOP 10 FILES BY HIT COUNT:" -ForegroundColor Yellow
$findings | Sort-Object TotalHits -Descending | Select-Object -First 10 | ForEach-Object {
    Write-Host ("  {0,4} hits  {1}" -f $_.TotalHits, $_.Path) -ForegroundColor Gray
}

# ============================================================================
# APPLY
# ============================================================================

if (-not $Apply) {
    Write-Host "`nDRY-RUN complete. Re-run with -Apply to rewrite." -ForegroundColor Yellow
    return
}

Write-Host "`n[3/3] Applying rewrites..." -ForegroundColor Green

$applied = 0
$findings | ForEach-Object -Parallel {
    $finding = $_
    $rules   = $using:compiledRules
    $root    = $using:Root
    $full    = Join-Path -Path $root -ChildPath $finding.Path

    try {
        $bytes = [System.IO.File]::ReadAllBytes($full)
        $text  = [System.Text.Encoding]::UTF8.GetString($bytes)
        foreach ($r in $rules) {
            $text = $r.Regex.Replace($text, $r.Replacement)
        }
        # Atomic write : .tmp puis move
        $tmp = "$full.rename.tmp"
        [System.IO.File]::WriteAllText($tmp, $text, [System.Text.UTF8Encoding]::new($false))
        Move-Item -LiteralPath $tmp -Destination $full -Force
        return 1
    } catch {
        Write-Warning "Failed: $($finding.Path) -- $_"
        return 0
    }
} -ThrottleLimit $ThrottleLimit | ForEach-Object { $applied += $_ }

Write-Host ("  Content rewrites: {0} / {1}" -f $applied, $findings.Count) -ForegroundColor Gray

# Path renames (files first, then dirs from deepest to shallowest)
$pathRenames |
    Sort-Object { ($_.OldFull -split '[\\/]').Count } -Descending |
    ForEach-Object {
        try {
            Move-Item -LiteralPath $_.OldFull -Destination $_.NewFull -Force
            Write-Host ("  Renamed {0}: {1} -> {2}" -f $_.Type, $_.OldFull, $_.NewFull) -ForegroundColor DarkGray
        } catch {
            Write-Warning "Failed rename: $($_.OldFull) -- $_"
        }
    }

Write-Host "`nAPPLY complete." -ForegroundColor Green
Write-Host "Don't forget: rename the root folder manually -- e.g. " -ForegroundColor Yellow
Write-Host "  Rename-Item '$Root' '$NewName'  (run this from a parent shell)" -ForegroundColor Yellow
