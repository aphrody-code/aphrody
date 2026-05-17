---
description: Run memory consolidation (dream) — organize learnings from recent sessions
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, Agent
---

Launch the `bun-agent:bun-dreamer` agent to consolidate memories from recent sessions.

## Directories
- Memory: `${CLAUDE_PLUGIN_DATA}/memory/`
- Sessions: `${CLAUDE_PLUGIN_DATA}/sessions/`

## Instructions
1. Ensure the memory directory exists: `mkdir -p ${CLAUDE_PLUGIN_DATA}/memory/`
2. Delegate to the `bun-agent:bun-dreamer` agent with the paths above
3. If this is the first dream (no MEMORY.md), create the initial index
