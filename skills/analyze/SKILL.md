---
name: bun-analyze
description: "Analyze project health, dependencies, code quality, and structure. TRIGGER when: user asks about project status, health, dependency audit, dead code, bundle size, outdated packages, architecture overview, or says 'analyze', 'audit', 'health check'."
allowed-tools: Read, Bash, Glob, Grep
model: inherit
user-invocable: true
version: "1.0"
---

# Project Analysis & Health Check

Run a comprehensive analysis of the current project. Adapt the checks based on what's present in the project.

## Checks to Run

### 1. Git Status
```bash
git status --short
git log --oneline -5
```

### 2. Dependencies
```bash
# Check for outdated packages
bun outdated 2>/dev/null || npm outdated 2>/dev/null || echo "No package manager detected"
```

### 3. Type Safety
```bash
bunx tsc --noEmit 2>&1 | tail -20
```

### 4. Code Quality
- Search for TODOs/FIXMEs: `grep -rn "TODO\|FIXME\|HACK\|XXX" --include="*.ts" --include="*.tsx" src/ | head -20`
- Check for console.log in production code
- Look for unused exports

### 5. Security Quick Scan
- Check for hardcoded secrets patterns
- Verify .env files are gitignored
- Check dependency vulnerabilities: `bun audit 2>/dev/null || npm audit 2>/dev/null`

### 6. Bundle / Build
- Verify build succeeds
- Check for large files that shouldn't be committed

## Output
Report findings grouped by category with severity indicators:
- OK: no issues found
- WARN: potential issues
- ERROR: action needed

$ARGUMENTS
