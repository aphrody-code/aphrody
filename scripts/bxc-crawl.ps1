# SPDX-License-Identifier: Apache-2.0
# bxc-crawl.ps1 — parallel bxc crawl over URL list with optional loop + recon cache.
#
# Mirrors the bash variant: same flags, same NDJSON event schema, same exit codes.

[CmdletBinding()]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Urls,

    [string]$FromFile,
    [int]$Jobs = 0,
    [string]$Actions = "recon,detect,tokens",
    [string]$Selector = "body",
    [switch]$Loop,
    [int]$Interval = 30,
    [int]$Port = 8765,
    [switch]$NoSpawnDaemon,
    [switch]$Help
)

if ($Help) {
@'
Usage: bxc-crawl.ps1 [-Urls <list>] [options]

Options:
  -FromFile <path>      URLs (one per line, # comments allowed).
  -Jobs <N>             Parallel workers (default min(8, CPU)).
  -Actions a,b,c        Subset of recon,scrape,detect,tokens (default: recon,detect,tokens).
  -Selector <css>       CSS selector for `scrape` action (default: body).
  -Loop                 Re-crawl forever until SIGINT.
  -Interval <secs>      Seconds between loop ticks (default 30).
  -Port <N>             bxc daemon port (default 8765).
  -NoSpawnDaemon        Do not auto-spawn the daemon if ping fails.
  -Help                 This help.

Exits 0 on full success, 1 if >=1 action failed, 2 on misuse, 130 on SIGINT.
'@
    exit 0
}

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot

$collected = @()
if ($Urls) { $collected += $Urls }
if ($FromFile) {
    if (-not (Test-Path -LiteralPath $FromFile)) {
        [Console]::Error.WriteLine("from-file not found: $FromFile"); exit 2
    }
    Get-Content -LiteralPath $FromFile | ForEach-Object {
        $l = $_.Trim()
        if ($l -and -not $l.StartsWith('#')) { $collected += $l }
    }
}
if ($collected.Count -eq 0) {
    [Console]::Error.WriteLine("no URLs provided"); exit 2
}

if ($Jobs -le 0) {
    $cpu = [Environment]::ProcessorCount
    if ($cpu -lt 1) { $cpu = 4 }
    $Jobs = [Math]::Min(8, $cpu)
}

$actionList = $Actions -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ }
foreach ($a in $actionList) {
    if ($a -notin @('recon','scrape','detect','tokens')) {
        [Console]::Error.WriteLine("invalid action: $a"); exit 2
    }
}

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

function Test-Daemon {
    param([int]$Port)
    try {
        $r = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/ping" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
        return ($r.StatusCode -eq 200)
    } catch { return $false }
}

if (-not (Test-Daemon -Port $Port)) {
    if (-not $NoSpawnDaemon) {
        [Console]::Out.WriteLine((@{event='daemon_spawn'; port=$Port} | ConvertTo-Json -Compress))
        Start-Process -FilePath $aphrody -ArgumentList @('bxc','daemon','--port',"$Port") -WindowStyle Hidden | Out-Null
        $ok = $false
        for ($i = 0; $i -lt 10; $i++) {
            Start-Sleep -Seconds 1
            if (Test-Daemon -Port $Port) { $ok = $true; break }
        }
        if (-not $ok) {
            [Console]::Error.WriteLine("daemon failed to start on port $Port"); exit 1
        }
    } else {
        [Console]::Error.WriteLine("daemon not reachable on port $Port"); exit 1
    }
}

# Per-session cache: ConcurrentDictionary so parallel workers see prior hashes.
$cache = [System.Collections.Concurrent.ConcurrentDictionary[string,string]]::new()

$sigintHandler = [System.ConsoleCancelEventHandler]{
    param($sender, $e)
    $e.Cancel = $true
    [Console]::Out.WriteLine('{"event":"sigint"}')
    [Environment]::Exit(130)
}
[Console]::add_CancelKeyPress($sigintHandler)

function Invoke-Tick {
    param([int]$TickIdx)

    $tickSw = [System.Diagnostics.Stopwatch]::StartNew()

    # Order recon first so the cache decision is made before detect/tokens.
    $ordered = @()
    if ('recon' -in $actionList) { $ordered += 'recon' }
    foreach ($a in $actionList) { if ($a -ne 'recon') { $ordered += $a } }

    $jobs = @()
    foreach ($url in $collected) {
        foreach ($a in $ordered) {
            $jobs += [pscustomobject]@{ Url = $url; Action = $a }
        }
    }

    $bag = [System.Collections.Concurrent.ConcurrentBag[string]]::new()

    $jobs | ForEach-Object -ThrottleLimit $Jobs -Parallel {
        $j        = $_
        $aphrody  = $using:aphrody
        $selector = $using:Selector
        $cache    = $using:cache
        $bag      = $using:bag

        $urlKey  = ($j.Url -replace '[^A-Za-z0-9._-]', '_')
        $prevHash = $null; $null = $cache.TryGetValue("hash:$urlKey", [ref]$prevHash)
        $changed  = $null; $null = $cache.TryGetValue("changed:$urlKey", [ref]$changed)

        if ($j.Action -ne 'recon' -and $j.Action -ne 'scrape' -and $changed -eq 'unchanged') {
            $obj = [ordered]@{
                url = $j.Url; action = $j.Action; cached = $true; exit = 0; duration_ms = 0
            }
            $line = $obj | ConvertTo-Json -Compress
            [Console]::Out.WriteLine($line)
            $bag.Add($line) | Out-Null
            return
        }

        $argList = @('bxc', $j.Action, $j.Url)
        if ($j.Action -eq 'scrape') { $argList += @('--selector', $selector) }

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
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
        $out = $stdout + $stderr

        $curHash = ""
        if ($j.Action -eq 'recon' -and $p.ExitCode -eq 0) {
            $sha = [System.Security.Cryptography.SHA256]::Create()
            $bytes = [System.Text.Encoding]::UTF8.GetBytes($out)
            $curHash = -join ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') })
            $sha.Dispose()
            $cache["hash:$urlKey"] = $curHash
            if ($prevHash -and $curHash -eq $prevHash) {
                $cache["changed:$urlKey"] = 'unchanged'
            } else {
                $cache["changed:$urlKey"] = 'changed'
            }
        }

        $bytesOut = [System.Text.Encoding]::UTF8.GetByteCount($out)
        $head = if ($out.Length -gt 4096) { $out.Substring(0, 4096) } else { $out }
        $headB64 = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($head))

        $obj = [ordered]@{
            url          = $j.Url
            action       = $j.Action
            exit         = $p.ExitCode
            duration_ms  = [int]$sw.ElapsedMilliseconds
            bytes        = $bytesOut
            hash         = if ($curHash) { $curHash } else { if ($prevHash) { $prevHash } else { "" } }
            body_head_b64 = $headB64
        }
        $line = $obj | ConvertTo-Json -Compress
        [Console]::Out.WriteLine($line)
        $bag.Add($line) | Out-Null
    }

    $tickSw.Stop()
    $total = $bag.Count
    $ok = 0; $cached = 0
    foreach ($l in $bag) {
        try {
            $o = $l | ConvertFrom-Json
            if ($o.exit -eq 0) { $ok++ }
            if ($o.PSObject.Properties['cached'] -and $o.cached) { $cached++ }
        } catch {}
    }
    $fail = $total - $ok
    $urlsCount = $collected.Count
    $urlsPerSec = if ($tickSw.ElapsedMilliseconds -gt 0) {
        [Math]::Round(($urlsCount * 1000.0 / $tickSw.ElapsedMilliseconds), 2)
    } else { 0 }
    $cacheRatio = if ($total -gt 0) { [Math]::Round(($cached / [double]$total), 2) } else { 0 }
    $summary = [ordered]@{
        event        = 'tick_summary'
        tick         = $TickIdx
        urls         = $urlsCount
        actions      = $total
        ok           = $ok
        fail         = $fail
        cached       = $cached
        cache_ratio  = $cacheRatio
        urls_per_sec = $urlsPerSec
        duration_ms  = [int]$tickSw.ElapsedMilliseconds
    }
    [Console]::Error.WriteLine(($summary | ConvertTo-Json -Compress))
    return $fail
}

$overallFail = 0
$tick = 0
if ($Loop) {
    while ($true) {
        $tick++
        $f = Invoke-Tick -TickIdx $tick
        if ($f -gt 0) { $overallFail = 1 }
        Start-Sleep -Seconds $Interval
    }
} else {
    $f = Invoke-Tick -TickIdx 1
    if ($f -gt 0) { $overallFail = 1 }
}

if ($overallFail -gt 0) { exit 1 } else { exit 0 }
