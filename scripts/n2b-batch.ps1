# SPDX-License-Identifier: Apache-2.0
# n2b-batch.ps1 — parallel `aphrody n2b` runner over a target list.
#
# Emits NDJSON on stdout (one line per completed target). Per-target logs go to
# --LogDir. Summary line emitted on stderr.

[CmdletBinding()]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Targets,

    [string]$FromFile,
    [int]$Jobs = 0,
    [string]$LogDir,
    [string]$ExtraArgs = "",
    [switch]$Help
)

if ($Help) {
@'
Usage: n2b-batch.ps1 [-Targets <list>] [options]

Options:
  -FromFile <path>     Read targets (one per line, # comments allowed).
  -Jobs <N>            Parallel workers (default = CPU count).
  -LogDir <dir>        Per-target log directory (default: var/log/n2b-batch-<ts>).
  -ExtraArgs <str>     Extra args appended to each `aphrody n2b` invocation.
  -Help                This help.

Exits 0 on full success, 1 if >=1 target failed, 2 on misuse, 130 on SIGINT.
'@
    exit 0
}

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot

$collected = @()
if ($Targets) { $collected += $Targets }
if ($FromFile) {
    if (-not (Test-Path -LiteralPath $FromFile)) {
        [Console]::Error.WriteLine("from-file not found: $FromFile"); exit 2
    }
    Get-Content -LiteralPath $FromFile | ForEach-Object {
        $line = $_.Trim()
        if ($line -and -not $line.StartsWith('#')) { $collected += $line }
    }
}
if ($collected.Count -eq 0) {
    [Console]::Error.WriteLine("no targets provided (use -Targets or -FromFile)")
    exit 2
}

if ($Jobs -le 0) {
    $Jobs = [Environment]::ProcessorCount
    if ($Jobs -lt 1) { $Jobs = 4 }
}

if (-not $LogDir) {
    $ts = (Get-Date -AsUTC -Format "yyyyMMddTHHmmssZ")
    $LogDir = Join-Path $repoRoot "var/log/n2b-batch-$ts"
}
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

function Resolve-Aphrody {
    $cmd = Get-Command aphrody -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $cands = @(
        (Join-Path $repoRoot "target/release/aphrody.exe"),
        (Join-Path $repoRoot "target/release/aphrody"),
        (Join-Path $repoRoot "target/debug/aphrody.exe"),
        (Join-Path $repoRoot "target/debug/aphrody")
    )
    foreach ($c in $cands) { if (Test-Path -LiteralPath $c) { return $c } }
    return $null
}
$aphrody = Resolve-Aphrody
if (-not $aphrody) {
    [Console]::Error.WriteLine("aphrody binary not found in PATH or target/{release,debug}")
    exit 2
}

# SIGINT trap: emit event and exit 130.
$sigintHandler = [System.ConsoleCancelEventHandler]{
    param($sender, $e)
    $e.Cancel = $true
    [Console]::Out.WriteLine('{"event":"sigint"}')
    [Environment]::Exit(130)
}
[Console]::add_CancelKeyPress($sigintHandler)

$resultsBag = [System.Collections.Concurrent.ConcurrentBag[string]]::new()

$collected | ForEach-Object -ThrottleLimit $Jobs -Parallel {
    $target    = $_
    $aphrody   = $using:aphrody
    $logDir    = $using:LogDir
    $extraArgs = $using:ExtraArgs
    $bag       = $using:resultsBag

    $sanitized = ($target -replace '[^A-Za-z0-9._-]', '_')
    if ($sanitized.Length -gt 200) { $sanitized = $sanitized.Substring(0, 200) }
    $log  = Join-Path $logDir "$sanitized.log"
    $code = Join-Path $logDir "$sanitized.code"

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $argList = @('n2b') + ($target -split '\s+')
    if ($extraArgs) { $argList += ($extraArgs -split '\s+') }
    $argList = $argList | Where-Object { $_ -ne "" }

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $aphrody
    foreach ($a in $argList) { $psi.ArgumentList.Add($a) }
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError  = $true
    $psi.UseShellExecute = $false
    $p = [System.Diagnostics.Process]::Start($psi)
    $stdout = $p.StandardOutput.ReadToEnd()
    $stderr = $p.StandardError.ReadToEnd()
    $p.WaitForExit()
    $sw.Stop()

    Set-Content -LiteralPath $log -Value ($stdout + $stderr) -NoNewline
    Set-Content -LiteralPath $code -Value $p.ExitCode

    $obj = [ordered]@{
        target      = $target
        exit        = $p.ExitCode
        duration_ms = [int]$sw.ElapsedMilliseconds
        log         = $log
    }
    $json = $obj | ConvertTo-Json -Compress
    [Console]::Out.WriteLine($json)
    $bag.Add($json) | Out-Null
}

$total = 0; $ok = 0; $fail = 0
$durations = New-Object System.Collections.Generic.List[int]
foreach ($line in $resultsBag) {
    $total++
    try {
        $o = $line | ConvertFrom-Json
        if ($o.exit -eq 0) { $ok++ } else { $fail++ }
        $durations.Add([int]$o.duration_ms)
    } catch { $fail++ }
}
$p50 = "null"; $p95 = "null"
if ($durations.Count -gt 0) {
    $sorted = $durations | Sort-Object
    $n = $sorted.Count
    $i50 = [Math]::Min($n - 1, [int]([Math]::Floor($n * 0.50)))
    $i95 = [Math]::Min($n - 1, [int]([Math]::Floor($n * 0.95)))
    $p50 = $sorted[$i50]
    $p95 = $sorted[$i95]
}
$summary = [ordered]@{
    event   = "summary"
    total   = $total
    ok      = $ok
    fail    = $fail
    p50_ms  = $p50
    p95_ms  = $p95
    log_dir = $LogDir
}
[Console]::Error.WriteLine(($summary | ConvertTo-Json -Compress))

if ($fail -gt 0) { exit 1 } else { exit 0 }
