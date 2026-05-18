# WinClean Integration Skill

> WinClean ecosystem integration for Windows 11 optimization and development.

## Overview

This skill provides access to WinClean tools for Windows 11 (24H2+) optimization:
- System scanning and profiling
- Debloating recommendations
- Native AOT development (C#)
- C++20 development
- Bun/TypeScript development
- Python performance optimization

## Key Commands

### System Operations
- `pwsh -File C:\winclean\winclean.ps1 scan` - Quick system scan
- `pwsh -File C:\winclean\winclean.ps1 profile` - Full hardware + apps snapshot
- `pwsh -File C:\winclean\winclean.ps1 debloat` - Apply debloat (requires confirmation)

### Development
- Use bun (NOT npm/yarn/pnpm)
- Use oxlint/oxfmt (NOT eslint/prettier)
- C# must be NativeAOT with System.Text.Json source generators
- Data files go to C:\winclean\var\json\

## Important Rules

1. **Toolchain**: Always use bun, not Node.js
2. **Linting**: Always use oxlint/oxfmt
3. **Reversibility**: Create System Restore point before destructive changes
4. **PPL**: Protected services may fail silently - log and continue
5. **WSL**: NEVER use WSL on this machine

## MCP Integration

When MCP server `winclean` is connected, you can call:
- `get_system_profile` - Cross-check system info
- Run without explicitly invoking winclean.ps1
