---
name: dream
description: "Run memory consolidation (dream). TRIGGER when: user asks to consolidate memories, clean up learnings, organize memory files, run a dream, or says 'dream'. Also proactively triggered after long productive sessions."
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, Agent
model: inherit
user-invocable: true
version: "2.0"
---

# Dream — Memory Consolidation

Mode `/goal` permanent : décider seul, ne pas demander confirmation, finir la consolidation.

Consolidate memories from recent sessions directly in the current context. Paths use `${CLAUDE_PLUGIN_DATA}` so the skill is portable; on Windows the inline `ls`/`wc`/`cat` previews degrade gracefully (use the Glob/Read tools if a shell builtin is missing).

## Current State

Memory directory: `${CLAUDE_PLUGIN_DATA}/memory/`
Sessions directory: `${CLAUDE_PLUGIN_DATA}/sessions/`

Memory files: !`ls ${CLAUDE_PLUGIN_DATA}/memory/ 2>/dev/null || echo "(empty — first dream)"`

Session count: !`ls ${CLAUDE_PLUGIN_DATA}/sessions/*.jsonl 2>/dev/null | wc -l 2>/dev/null || echo "0"`

Current MEMORY.md: !`cat ${CLAUDE_PLUGIN_DATA}/memory/MEMORY.md 2>/dev/null || echo "(no index yet)"`

## Instructions

1. Ensure the memory directory exists: `mkdir -p ${CLAUDE_PLUGIN_DATA}/memory/`
2. Read the session transcripts under the sessions directory above
3. Extract durable learnings and merge them into the memory directory
4. If this is the first dream (no MEMORY.md), create the initial index

For the memory file format specification, see `${CLAUDE_SKILL_DIR}/references/memory-format.md`.
