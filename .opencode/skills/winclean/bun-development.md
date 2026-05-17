# Bun Development Skill

> Fast, modern JavaScript/TypeScript development with Bun runtime.

## When to Use

- Starting new JS/TS projects with Bun
- Migrating from Node.js to Bun
- Optimizing development speed
- Using Bun's built-in tools (bundler, test runner)

## Installation

```powershell
# Windows
powershell -c "irm bun.sh/install.ps1 | iex"

# Upgrade
bun upgrade
```

## Why Bun?

| Feature | Bun | Node.js |
|---------|-----|---------|
| Startup | ~25ms | ~100ms+ |
| Package install | 10-100x faster | Baseline |
| TypeScript | Native | Requires transpiler |
| Test runner | Built-in | External |
| Bundler | Built-in | External |

## Quick Commands

```bash
bun init                    # Initialize project
bun add <pkg>              # Add dependency
bun run <script>           # Run script
bun test                   # Run tests
bun build ./src --outdir ./dist  # Bundle
bunx <pkg>                 # Execute package (like npx)
```

## WinClean Rules (STRICT)

1. **Toolchain**: NEVER use Node.js, npm, yarn, or pnpm. ALWAYS use `bun`
2. **Linting**: NEVER use ESLint or Prettier. ALWAYS use `oxlint` and `oxfmt`
3. **Data**: ALL volatile JSON/data files to `C:\winclean\var\json\`
4. **.NET**: C# Code MUST be NativeAOT with System.Text.Json source generators
