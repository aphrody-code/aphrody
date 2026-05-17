---
name: bun-dream
description: "Run memory consolidation (dream). TRIGGER when: user asks to consolidate memories, clean up learnings, organize memory files, run a dream, or says 'dream'. Also proactively triggered after long productive sessions."
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, Agent
model: inherit
user-invocable: true
version: "2.0"
---

# Dream — Memory Consolidation

Launch the `bun-dreamer` agent to consolidate memories from recent sessions.

## Current State

Memory directory: `${CLAUDE_PLUGIN_DATA}/memory/`
Sessions directory: `${CLAUDE_PLUGIN_DATA}/sessions/`

Memory files: !`ls ${CLAUDE_PLUGIN_DATA}/memory/ 2>/dev/null || echo "(empty — first dream)"`

Session count: !`ls ${CLAUDE_PLUGIN_DATA}/sessions/*.jsonl 2>/dev/null | wc -l 2>/dev/null || echo "0"`

Current MEMORY.md: !`cat ${CLAUDE_PLUGIN_DATA}/memory/MEMORY.md 2>/dev/null || echo "(no index yet)"`

## Instructions

1. Ensure the memory directory exists: `mkdir -p ${CLAUDE_PLUGIN_DATA}/memory/`
2. Launch the `bun-agent:bun-dreamer` agent
3. Pass it the memory directory and session directory paths above
4. If this is the first dream (no MEMORY.md), create the initial index

For the memory file format specification, see `${CLAUDE_SKILL_DIR}/references/memory-format.md`.
