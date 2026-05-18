---
name: bun-run
description: "Execute a complex task autonomously using Bun agent capabilities. TRIGGER when: user asks to run a multi-step workflow, automate something, process files in bulk, execute a build pipeline, do a DevOps task, or explicitly says 'run with bun'. Also triggers when the task clearly needs more than 3-4 tool calls to complete."
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, Agent
model: inherit
user-invocable: true
argument-hint: "[task description]"
version: "2.0"
---

# Bun Agent — Autonomous Task Runner

Execute the following task with full autonomy. Do NOT ask for confirmation at each step — analyze, plan, execute, verify.

## Bun Native APIs Reference
When Claude Code tools aren't sufficient, use Bun directly via shell:

```bash
# File I/O (lazy-loaded, fast)
bun -e "console.log(await Bun.file('path').text())"
bun -e "await Bun.write('out.txt', 'content')"
bun -e "console.log(await Bun.file('f').exists())"

# Glob (recursive, fast)
bun -e "for await (const f of new Bun.Glob('**/*.ts').scan('.')) console.log(f)"

# HTTP (native fetch)
bun -e "console.log(await (await fetch('https://api.example.com')).json())"

# Shell (auto-escaped, cross-platform)
bun -e "import {\$} from 'bun'; console.log(await \$\`ls -la\`.text())"

# Subprocess (streaming stdout/stderr)
bun -e "const p=Bun.spawn(['cmd'],{stdout:'pipe'}); console.log(await new Response(p.stdout).text())"
```

For the complete Bun API reference, read `${CLAUDE_SKILL_DIR}/references/bun-apis.md`.

## Execution Strategy
1. **Analyze** the task — identify inputs, outputs, steps, and potential failures
2. **Plan** — determine which tools to use, which steps can run in parallel
3. **Execute** — run each step, verify success before proceeding
4. **Handle failures** — if a step fails, analyze the error and retry with a fix
5. **Report** — concise summary of what was done and the result

## Task
$ARGUMENTS
