# PowerShell Autopilot Loop for Aphrody
# Saves PID to var/run/autopilot.pid, heartbeats to ai/heartbeat.txt, logs to var/log/autopilot.jsonl

param(
    [int]$Interval = 60,
    [int]$MaxTicks = 0,
    [switch]$Once
)

# 1. Setup paths and directories
$scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Get-Item (Join-Path $scriptPath "..\..")).FullName
Set-Location $repoRoot

$runDir = Join-Path $repoRoot "var/run"
$logDir = Join-Path $repoRoot "var/log"
$aiDir = Join-Path $repoRoot "ai"

New-Item -ItemType Directory -Force -Path $runDir | Out-Null
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
New-Item -ItemType Directory -Force -Path $aiDir | Out-Null

# Write current PID
$pidFile = Join-Path $runDir "autopilot.pid"
$MyProcessId = $PID
$MyProcessId | Out-File -FilePath $pidFile -Encoding utf8 -Force

$logFile = Join-Path $logDir "autopilot.jsonl"
$heartbeatFile = Join-Path $aiDir "heartbeat.txt"
$planFile = Join-Path $repoRoot "docs/PLAN.md"

Write-Host "=== Aphrody Autopilot Started (PID: $MyProcessId) ==="
Write-Host "Logging to: $logFile"

if ($Once) {
    $MaxTicks = 1
}

$tick = 0

while ($true) {
    $tick++
    if ($MaxTicks -gt 0 -and $tick -gt $MaxTicks) {
        Write-Host "Reached max ticks ($MaxTicks). Stopping."
        break
    }

    Write-Host "`n--- Tick $tick (Interval: $Interval s) ---"

    # Read PLAN.md and find first pending item
    $task = "Idle"
    if (Test-Path $planFile) {
        $lines = Get-Content $planFile
        foreach ($line in $lines) {
            $trimmed = $line.Trim()
            if ($trimmed.StartsWith("-") -and $trimmed.Contains("⏳")) {
                $idx = $line.IndexOf("⏳")
                $rawTask = $line.Substring($idx + 1)
                $task = $rawTask.replace('`', '').trim()
                if ($task.StartsWith("]")) { $task = $task.Substring(1).Trim() }
                break
            }
        }
    }

    $timestamp = (Get-Date).ToString("yyyy-MM-ddTHH:mm:sszzz")
    Write-Host "Active Task: $task"

    # Write Heartbeat
    "$timestamp - Tick $tick - $task" | Out-File -FilePath $heartbeatFile -Encoding utf8 -Force

    # If no task, sleep and continue
    if ($task -eq "Idle") {
        Write-Host "No pending tasks in PLAN.md. Resting."
        $logEntry = @{
            ts = $timestamp
            tick = $tick
            task = $task
            claude = ""
            gemini = "No tasks found"
        } | ConvertTo-Json -Compress
        $logEntry | Out-File -FilePath $logFile -Encoding utf8 -Append
        
        if ($Once) { break }
        Start-Sleep -Seconds $Interval
        continue
    }

    # 2. Claude Lane (Ship modification)
    Write-Host "Running Claude Lane..."
    $claudeOutput = ""
    try {
        # Formulate instruction prompt
        $prompt = "You are an autonomous developer agent. Implement this task: '$task'. Modify files as needed. Verify that 'uv run ruff check' and 'uv run pytest aphrody/tests' pass cleanly. Commit the changes using a Conventional Commit message. Do not add co-author trailers."
        
        # Execute Claude Code command
        $claudeRes = Start-Job -ScriptBlock {
            param($p, $root)
            Set-Location $root
            # Run Claude Code CLI (or alternative CLI if available)
            claude -p $p --dangerously-skip-permissions
        } -ArgumentList $prompt, $repoRoot
        
        # Wait with timeout
        $waitRes = Wait-Job $claudeRes -Timeout 300
        $claudeOutput = Receive-Job $claudeRes
        Remove-Job $claudeRes
    }
    catch {
        $claudeOutput = "Err: $_"
    }

    # 3. Gemini Lane (Audit)
    Write-Host "Running Gemini Lane..."
    $geminiOutput = ""
    try {
        $auditPrompt = "Audit the most recent commit in this repository. Verify against the best-stack-2026 guidelines (no GPL licenses, optimal Python modules, fully cross-platform path handling). Output a strict JSON report summarizing your findings."
        
        $geminiRes = Start-Job -ScriptBlock {
            param($ap, $root)
            Set-Location $root
            # Call Gemini CLI
            gemini --prompt $ap
        } -ArgumentList $auditPrompt, $repoRoot
        
        $waitGemini = Wait-Job $geminiRes -Timeout 300
        $geminiOutput = Receive-Job $geminiRes
        Remove-Job $geminiRes
    }
    catch {
        $geminiOutput = "Err: $_"
    }

    # Log entry
    $logEntry = @{
        ts = $timestamp
        tick = $tick
        task = $task
        claude = $claudeOutput
        gemini = $geminiOutput
    } | ConvertTo-Json -Compress
    $logEntry | Out-File -FilePath $logFile -Encoding utf8 -Append

    # Mark the task completed in PLAN.md
    if (Test-Path $planFile) {
        $content = Get-Content $planFile
        $newContent = @()
        $marked = $false
        foreach ($line in $content) {
            if ($line -like "*⏳*$task*" -and -not $marked) {
                $newContent += $line -replace "⏳", "✅"
                $marked = $true
                Write-Host "Marked task as completed in PLAN.md."
            } else {
                $newContent += $line
            }
        }
        $newContent | Out-File -FilePath $planFile -Encoding utf8 -Force
    }

    if ($Once) { break }
    Start-Sleep -Seconds $Interval
}
