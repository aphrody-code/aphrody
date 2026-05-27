#!/usr/bin/env bash
# Bash Autopilot Loop for Aphrody
# Saves PID to var/run/autopilot.pid, heartbeats to ai/heartbeat.txt, logs to var/log/autopilot.jsonl

INTERVAL=60
MAX_TICKS=0
ONCE=0

# Parse arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --interval) INTERVAL="$2"; shift ;;
        --max-ticks) MAX_TICKS="$2"; shift ;;
        --once) ONCE=1 ;;
        *) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done

# Resolve repo root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT" || exit 1

# Setup run, log, and ai dirs
mkdir -p var/run var/log ai

# Save PID
MY_PID=$$
echo "$MY_PID" > var/run/autopilot.pid

LOG_FILE="var/log/autopilot.jsonl"
HEARTBEAT_FILE="ai/heartbeat.txt"
PLAN_FILE="docs/PLAN.md"

echo "=== Aphrody Autopilot Started (PID: $MY_PID) ==="
echo "Logging to: $LOG_FILE"

if [ "$ONCE" -eq 1 ]; then
    MAX_TICKS=1
fi

tick=0

while true; do
    tick=$((tick + 1))
    if [ "$MAX_TICKS" -gt 0 ] && [ "$tick" -gt "$MAX_TICKS" ]; then
        echo "Reached max ticks ($MAX_TICKS). Stopping."
        break
    fi

    echo -e "\n--- Tick $tick (Interval: ${INTERVAL}s) ---"

    # Find first pending task in PLAN.md
    task="Idle"
    if [ -f "$PLAN_FILE" ]; then
        while IFS= read -r line; do
            trimmed=$(echo "$line" | xargs)
            if [[ "$trimmed" == -* ]] && [[ "$trimmed" == *"⏳"* ]]; then
                task="${line#*⏳}"
                task=$(echo "$task" | tr -d '`' | tr -d '\r' | xargs)
                if [[ "$task" == \]* ]]; then
                    task="${task#]}"
                    task=$(echo "$task" | xargs)
                fi
                break
            fi
        done < "$PLAN_FILE"
    fi

    timestamp=$(date -Iseconds)
    echo "Active Task: $task"

    # Write Heartbeat
    echo "$timestamp - Tick $tick - $task" > "$HEARTBEAT_FILE"

    # If no task, sleep and continue
    if [ "$task" = "Idle" ]; then
        echo "No pending tasks in PLAN.md. Resting."
        # Write log entry
        echo "{\"ts\":\"$timestamp\",\"tick\":$tick,\"task\":\"$task\",\"claude\":\"\",\"gemini\":\"No tasks found\"}" >> "$LOG_FILE"
        
        if [ "$ONCE" -eq 1 ]; then break; fi
        sleep "$INTERVAL"
        continue
    fi

    # Claude Lane (Execution)
    echo "Running Claude Lane..."
    claude_output=""
    prompt="You are an autonomous developer agent. Implement this task: '$task'. Modify files as needed. Verify that 'uv run ruff check' and 'uv run pytest aphrody/tests' pass cleanly. Commit the changes using a Conventional Commit message. Do not add co-author trailers."
    
    # Run in background with timeout
    (claude -p "$prompt" --dangerously-skip-permissions) > /tmp/claude_out.log 2>&1 &
    claude_pid=$!
    
    # Timeout logic (5 mins)
    timeout_counter=0
    while kill -0 "$claude_pid" 2>/dev/null; do
        sleep 1
        timeout_counter=$((timeout_counter + 1))
        if [ "$timeout_counter" -ge 300 ]; then
            kill "$claude_pid" 2>/dev/null
            echo "Claude lane timed out."
            claude_output="Err: Timeout after 300s"
            break
        fi
    done

    if [ -z "$claude_output" ]; then
        claude_output=$(cat /tmp/claude_out.log | tr -d '"\r\n' | head -c 800)
    fi

    # Gemini Lane (Audit)
    echo "Running Gemini Lane..."
    gemini_output=""
    audit_prompt="Audit the most recent commit in this repository. Verify against the best-stack-2026 guidelines (no GPL licenses, optimal Python modules, fully cross-platform path handling). Output a strict JSON report summarizing your findings."
    
    (gemini --prompt "$audit_prompt") > /tmp/gemini_out.log 2>&1 &
    gemini_pid=$!
    
    timeout_counter=0
    while kill -0 "$gemini_pid" 2>/dev/null; do
        sleep 1
        timeout_counter=$((timeout_counter + 1))
        if [ "$timeout_counter" -ge 300 ]; then
            kill "$gemini_pid" 2>/dev/null
            echo "Gemini lane timed out."
            gemini_output="Err: Timeout after 300s"
            break
        fi
    done

    if [ -z "$gemini_output" ]; then
        gemini_output=$(cat /tmp/gemini_out.log | tr -d '"\r\n' | head -c 800)
    fi

    # Log entry
    echo "{\"ts\":\"$timestamp\",\"tick\":$tick,\"task\":\"$task\",\"claude\":\"$claude_output\",\"gemini\":\"$gemini_output\"}" >> "$LOG_FILE"

    # Mark the task completed in PLAN.md
    if [ -f "$PLAN_FILE" ]; then
        # Replace first occurrence of ⏳ task with ✅ task
        # Use python inline or sed (python is more cross-platform)
        python -c "
import sys
content = open('$PLAN_FILE', 'r', encoding='utf-8').read()
target = '⏳'
replacement = '✅'
if target in content:
    # replace first occurrence only
    parts = content.split(target, 1)
    # verify if this is the task
    if '$task' in parts[1]:
        new_content = parts[0] + replacement + parts[1]
        open('$PLAN_FILE', 'w', encoding='utf-8').write(new_content)
        print('Marked task as completed.')
"
    fi

    if [ "$ONCE" -eq 1 ]; then break; fi
    sleep "$INTERVAL"
done
