# SPDX-License-Identifier: Apache-2.0
#
# autopilot.ps1 — pilote Claude Code + Gemini CLI en duel parallèle, infini.
# Mirror PowerShell de scripts/autopilot.sh (cf. CLAUDE.md §4.1 bash↔pwsh parity).
#
#   pwsh scripts/autopilot.ps1                       # loop infini, 60s tick
#   pwsh scripts/autopilot.ps1 -Once                 # 1 tick (debug)
#   pwsh scripts/autopilot.ps1 -Interval 30 -MaxTicks 100
#
# Sorties identiques au .sh :
#   var/log/autopilot.jsonl  — NDJSON 1 ligne / tick
#   var/run/autopilot.pid    — PID du loop pwsh
#   ai/heartbeat.txt         — bump ISO-8601 (A2A)
#
# Ctrl-C ou Stop-Process -Id (Get-Content var/run/autopilot.pid) → clean stop.
# Conforme `feedback_aphrody_full_autonomy` (zéro humain) et CLAUDE.md §4.1
# gotcha (CancelKeyPress via delegate, jamais .Add({...})).

[CmdletBinding()]
param(
    [int]$Interval       = [int]($env:APHRODY_AUTOPILOT_INTERVAL  ?? 60),
    [int]$MaxTicks       = [int]($env:APHRODY_AUTOPILOT_MAX_TICKS ?? 0),
    [int]$ClaudeTimeout  = [int]($env:APHRODY_CLAUDE_TIMEOUT      ?? 300),
    [int]$GeminiTimeout  = [int]($env:APHRODY_GEMINI_TIMEOUT      ?? 300),
    [switch]$Once
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Continue'   # don't kill the loop on transient errors

$RepoRoot  = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$LogPath   = Join-Path $RepoRoot 'var/log/autopilot.jsonl'
$PidPath   = Join-Path $RepoRoot 'var/run/autopilot.pid'
$Heartbeat = Join-Path $RepoRoot 'ai/heartbeat.txt'
$PlanPath  = Join-Path $RepoRoot 'docs/PLAN.md'

foreach ($dir in @('var/log', 'var/run', 'ai')) {
    New-Item -ItemType Directory -Force -Path (Join-Path $RepoRoot $dir) | Out-Null
}

# Refuse double-start.
if (Test-Path $PidPath) {
    $existing = Get-Content $PidPath -ErrorAction SilentlyContinue
    if ($existing -and (Get-Process -Id $existing -ErrorAction SilentlyContinue)) {
        Write-Error "autopilot already running (pid=$existing). Stop-Process -Id $existing pour arrêter."
        exit 1
    }
}
$PID | Out-File -FilePath $PidPath -Encoding ascii -NoNewline

# Clean shutdown — CLAUDE.md §4.1 gotcha : utiliser le delegate, jamais .Add({})
$stopRequested = $false
$cancelHandler = [System.ConsoleCancelEventHandler] {
    param($sender, $e)
    $e.Cancel = $true
    $script:stopRequested = $true
}
[Console]::add_CancelKeyPress($cancelHandler)

function Write-JsonLine {
    param([hashtable]$Obj)
    # ConvertTo-Json compresse en une ligne avec -Compress, parfait pour NDJSON.
    $line = $Obj | ConvertTo-Json -Compress -Depth 4
    Add-Content -Path $LogPath -Value $line -Encoding utf8
}

function Get-NextTask {
    if (-not (Test-Path $PlanPath)) { return 'audit-curiosity' }
    $line = Select-String -Path $PlanPath -Pattern '⏳' -SimpleMatch |
            Select-Object -First 1 -ExpandProperty Line
    if (-not $line) { return 'audit-curiosity' }
    # Strip pipes + collapse + clamp ; Substring length must reference the
    # *post-transform* string (the trim/replace can shrink it under 160).
    $clean = (($line -replace '\|', ' ') -replace '\s+', ' ').Trim()
    if ($clean.Length -gt 160) { $clean.Substring(0, 160) } else { $clean }
}

function Get-ClaudePrompt {
    param([string]$Task, [int]$TickN)
@"
You are running inside the aphrody autopilot loop (tick $TickN). Read
CLAUDE.md §0.1 (zero human-in-loop). Pick the highest-leverage ⏳ item
from docs/PLAN.md and ship it end-to-end : implement, cargo check, commit
(Conventional Commit, no AI co-author trailer per AGENTS.md).

Selected hint (may be stale, you choose): $Task

Constraints: stay reversible, no force-push, no destructive rm outside
target/ + var/. If blocked, surface blocker in commit footer
'NON_FAIT: <reason>' and pick another ⏳ item.
"@
}

function Get-GeminiPrompt {
    param([string]$Task, [int]$TickN)
@"
You are the second lane of the aphrody autopilot duel (tick $TickN).
Read AGENTS.md + CLAUDE.md §0.1. Independently audit the most recent
commit on origin/main : check against best-stack-2026 skill recos
(canonical Rust 2026 crates), license safety (no GPL leak), cross-platform
build (Linux/Windows/wasm32).

Output JSON only (single line): {"verdict":"ack|nack","reasons":["..."],
"suggested_followup":"..."}. Do NOT modify files. If main is clean,
suggest the next ⏳ item from docs/PLAN.md.

Hint: $Task
"@
}

# Spawn a process non-blocking ; returns the Process + async read tasks.
function Start-LaneProcess {
    param([string]$FilePath, [string[]]$ArgList)
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $FilePath
    foreach ($a in $ArgList) { $psi.ArgumentList.Add($a) }
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError  = $true
    $psi.UseShellExecute        = $false
    $psi.CreateNoWindow         = $true
    try {
        $proc = [System.Diagnostics.Process]::Start($psi)
    } catch {
        return $null
    }
    [pscustomobject]@{
        Proc       = $proc
        StdoutTask = $proc.StandardOutput.ReadToEndAsync()
        StderrTask = $proc.StandardError.ReadToEndAsync()
    }
}

# Wait up to TimeoutSec, force-kill if exceeded ; collect stdout/stderr.
function Wait-LaneProcess {
    param($Lane, [int]$TimeoutSec)
    if (-not $Lane) { return @{ ok = $false; stdout = ''; err = 'spawn failed' } }
    if (-not $Lane.Proc.WaitForExit($TimeoutSec * 1000)) {
        try { $Lane.Proc.Kill($true) } catch { }
        return @{ ok = $false; stdout = ''; err = "timeout after ${TimeoutSec}s" }
    }
    $out = $Lane.StdoutTask.GetAwaiter().GetResult()
    $err = $Lane.StderrTask.GetAwaiter().GetResult()
    return @{ ok = ($Lane.Proc.ExitCode -eq 0); stdout = $out; err = $err }
}

function Resolve-ClaudeLane {
    param([string]$Task, [int]$TickN)
    $cmd = Get-Command claude -ErrorAction SilentlyContinue
    if (-not $cmd) { return @{ proc = $null; missing = 'claude CLI not in PATH' } }
    $prompt = Get-ClaudePrompt -Task $Task -TickN $TickN
    @{ proc = (Start-LaneProcess -FilePath $cmd.Source -ArgList @('-p', $prompt, '--dangerously-skip-permissions')); missing = $null }
}

function Resolve-GeminiLane {
    param([string]$Task, [int]$TickN)
    $prompt = Get-GeminiPrompt -Task $Task -TickN $TickN

    # Préférer le wrapper natif aphrody (a2a) — il route vers le fork in-tree
    # packages/gemini-cli/ via APHRODY_GEMINI_BIN, ou vers le binaire installé,
    # avec fallback message d'aide structuré (CLAUDE.md §0.4). C'est l'outil
    # maison (cf. memory project_aphrody_owned_tools).
    $aphrody = Get-Command aphrody -ErrorAction SilentlyContinue
    if ($aphrody) {
        return @{ proc = (Start-LaneProcess -FilePath $aphrody.Source -ArgList @('a2a', $prompt)); missing = $null }
    }
    # Fallback ultime : binaire Gemini upstream s'il est dans PATH.
    $cmd = Get-Command gemini -ErrorAction SilentlyContinue
    if ($cmd) {
        return @{ proc = (Start-LaneProcess -FilePath $cmd.Source -ArgList @('--prompt', $prompt)); missing = $null }
    }
    @{ proc = $null; missing = 'aphrody binary not in PATH (and no upstream gemini either)' }
}

# Truncate long output to keep NDJSON readable.
function ClipText { param([string]$Text, [int]$Max = 800) if (-not $Text) { return '' }; if ($Text.Length -le $Max) { $Text } else { $Text.Substring(0, $Max) } }

Write-JsonLine @{
    ts = (Get-Date -AsUTC -Format 'o')
    event = 'autopilot_start'
    pid = $PID
    interval = $Interval
    max_ticks = $MaxTicks
}

$tickN = 0
try {
    while (-not $stopRequested) {
        $tickN++
        $ts = Get-Date -AsUTC -Format 'o'
        $task = Get-NextTask

        # Parallel fan-out — kick both processes asynchronously, then wait both.
        # No Job / ThreadJob (scope isolation pain). Native Process API does it
        # cleanly via async ReadToEndAsync + WaitForExit(timeoutMs).
        $claudeLane = Resolve-ClaudeLane -Task $task -TickN $tickN
        $geminiLane = Resolve-GeminiLane -Task $task -TickN $tickN

        $claudeRes = if ($claudeLane.missing) {
            @{ ok = $false; stdout = ''; err = $claudeLane.missing }
        } else {
            Wait-LaneProcess -Lane $claudeLane.proc -TimeoutSec $ClaudeTimeout
        }
        $geminiRes = if ($geminiLane.missing) {
            @{ ok = $false; stdout = ''; err = $geminiLane.missing }
        } else {
            Wait-LaneProcess -Lane $geminiLane.proc -TimeoutSec $GeminiTimeout
        }

        "$ts autopilot tick #$tickN task=`"$task`"" | Set-Content -Path $Heartbeat -Encoding utf8

        Write-JsonLine @{
            ts     = $ts
            tick   = $tickN
            task   = $task
            claude = ClipText ($claudeRes.stdout ?? $claudeRes.err ?? '')
            gemini = ClipText ($geminiRes.stdout ?? $geminiRes.err ?? '')
        }

        if ($Once) { break }
        if ($MaxTicks -gt 0 -and $tickN -ge $MaxTicks) { break }

        # Sleep that wakes up on Ctrl-C.
        $waited = 0
        while ($waited -lt $Interval -and -not $stopRequested) {
            Start-Sleep -Seconds 1
            $waited++
        }
    }
} finally {
    Write-JsonLine @{ ts = (Get-Date -AsUTC -Format 'o'); event = 'autopilot_stop'; pid = $PID; ticks = $tickN }
    Remove-Item $PidPath -ErrorAction SilentlyContinue
    [Console]::remove_CancelKeyPress($cancelHandler)
}
