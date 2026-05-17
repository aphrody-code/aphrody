# SPDX-License-Identifier: Apache-2.0
# bxc-supervise.ps1 — watchdog for the `aphrody bxc daemon` process.

[CmdletBinding()]
param(
    [int]$Port = 8765,
    [int]$HealthInterval = 5,
    [int]$RestartCooldown = 10,
    [int]$MaxRestarts = -1,
    [switch]$NoSpawn,
    [switch]$Help
)

if ($Help) {
@'
Usage: bxc-supervise.ps1 [options]

Options:
  -Port <N>                Daemon port (default 8765).
  -HealthInterval <secs>   Poll interval (default 5).
  -RestartCooldown <secs>  Sleep after a respawn (default 10).
  -MaxRestarts <N>         Hard cap on restarts (default -1 = unlimited).
  -NoSpawn                 Do not auto-spawn; just report.
  -Help                    This help.

Exits 0 on clean shutdown, 1 on max-restarts exceeded, 2 on misuse, 130 on SIGINT.
'@
    exit 0
}

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot

foreach ($p in @($Port, $HealthInterval, $RestartCooldown)) {
    if ($p -lt 0) { [Console]::Error.WriteLine("invalid numeric arg"); exit 2 }
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
    try {
        $r = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/ping" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
        return ($r.StatusCode -eq 200)
    } catch { return $false }
}

function Read-Pid {
    $f = Join-Path $repoRoot "var/run/bxc.pid"
    if (Test-Path -LiteralPath $f) {
        $raw = (Get-Content -LiteralPath $f -Raw).Trim()
        if ($raw -match '^\d+$') { return [int]$raw }
    }
    return 0
}

function Emit-Event {
    param([string]$Status, [int]$Pid, [int]$Uptime, [int]$Restarts, [bool]$ToStderr = $false)
    $obj = [ordered]@{
        ts            = (Get-Date -AsUTC -Format "yyyy-MM-ddTHH:mm:ssZ")
        status        = $Status
        pid           = $Pid
        uptime_s      = $Uptime
        restart_count = $Restarts
        port          = $Port
    }
    $line = $obj | ConvertTo-Json -Compress
    if ($ToStderr) { [Console]::Error.WriteLine($line) } else { [Console]::Out.WriteLine($line) }
}

$script:restartCount = 0
$script:startedAt = [int][double]::Parse((Get-Date -UFormat %s))
$script:lastUpTs = 0

$sigintHandler = [System.ConsoleCancelEventHandler]{
    param($sender, $e)
    $e.Cancel = $true
    $now = [int][double]::Parse((Get-Date -UFormat %s))
    $uptime = $now - $script:startedAt
    Emit-Event -Status 'shutdown' -Pid (Read-Pid) -Uptime $uptime -Restarts $script:restartCount -ToStderr $true
    [Environment]::Exit(130)
}
[Console]::add_CancelKeyPress($sigintHandler)

while ($true) {
    if (Test-Daemon) {
        if ($script:lastUpTs -eq 0) {
            $script:lastUpTs = [int][double]::Parse((Get-Date -UFormat %s))
        }
        $up = [int][double]::Parse((Get-Date -UFormat %s)) - $script:lastUpTs
        Emit-Event -Status 'up' -Pid (Read-Pid) -Uptime $up -Restarts $script:restartCount
    } else {
        $script:lastUpTs = 0
        Emit-Event -Status 'down' -Pid (Read-Pid) -Uptime 0 -Restarts $script:restartCount
        if ($NoSpawn) {
            Start-Sleep -Seconds $HealthInterval
            continue
        }
        if ($MaxRestarts -ge 0 -and $script:restartCount -ge $MaxRestarts) {
            Emit-Event -Status 'max_restarts_exceeded' -Pid 0 -Uptime 0 -Restarts $script:restartCount -ToStderr $true
            exit 1
        }
        Emit-Event -Status 'restarting' -Pid 0 -Uptime 0 -Restarts $script:restartCount
        Start-Process -FilePath $aphrody -ArgumentList @('bxc','daemon','--port',"$Port") -WindowStyle Hidden | Out-Null
        $script:restartCount++
        Start-Sleep -Seconds $RestartCooldown
    }
    Start-Sleep -Seconds $HealthInterval
}
