---
name: winclean-mcp-skills
description: Wrapper namespace for 3 nested Microsoft-focused skills synced from the WinClean MCP ecosystem — microsoft-docs (Microsoft Learn API docs lookup), microsoft-code-reference (Microsoft sample code retrieval), microsoft-skill-creator (Microsoft skill templates). Each sub-skill ships its own SKILL.md.
version: "1.0.0"
---

# winclean-mcp-skills — Microsoft skill namespace

Mode `/goal` permanent : décider seul, ne pas s'arrêter avant complétion.

This directory is a **container for 3 nested skills** synced from the peer WinClean MCP server (`$WINCLEAN`, default `C:\winclean`). Not a stand-alone skill — each sub-directory has its own `SKILL.md`, independently discoverable by the skill loader.

## Nested skills

| Sub-skill | Purpose | Source SKILL.md |
|---|---|---|
| `microsoft-docs` | Microsoft Learn / Azure / Windows API documentation lookup | `microsoft-docs/SKILL.md` |
| `microsoft-code-reference` | Sample code retrieval from Microsoft official repos | `microsoft-code-reference/SKILL.md` |
| `microsoft-skill-creator` | Microsoft-flavored skill templates (Markdown + references/) | `microsoft-skill-creator/SKILL.md` |

## Origin

Synced from the WinClean plugin (`$WINCLEAN/plugins/winclean/skills/`). Kept under one namespace to preserve provenance and simplify mass re-sync.

## Cross-reference

See [`skills/winclean/SKILL.md`](../winclean/SKILL.md) for the broader
WinClean ecosystem integration (system scan, debloat, NativeAOT C# dev).
