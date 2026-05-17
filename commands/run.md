---
description: Execute a complex task autonomously using the Bun-powered agent
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, Agent
argument-hint: "[task description]"
---

Launch the `bun-agent:bun-runner` agent to execute this task autonomously.

## Task
$ARGUMENTS

## Instructions
1. Analyze the task — identify inputs, outputs, and potential failures
2. Plan the steps — determine which can run in parallel
3. Execute each step, verify success before proceeding
4. If a step fails, analyze the error and fix it
5. Report a concise summary of results

Use Bun native APIs when Claude Code tools aren't sufficient:
```bash
bun -e "console.log(await Bun.file('path').text())"
bun -e "for await (const f of new Bun.Glob('**/*.ts').scan('.')) console.log(f)"
bun -e "console.log(await (await fetch('url')).json())"
```
