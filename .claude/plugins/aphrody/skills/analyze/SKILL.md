---
name: analyze
description: "Analyze project health, dependencies, code quality, and structure. TRIGGER when: user asks about project status, health, dependency audit, dead code, bundle size, outdated packages, architecture overview, or says 'analyze', 'audit', 'health check'."
allowed-tools: Read, Bash, Glob, Grep
model: inherit
user-invocable: true
version: "1.0"
---

# Project Analysis & Health Check

Mode `/goal` permanent : décider seul, ne pas s'arrêter avant rapport complet.

Run a comprehensive analysis of the current project. Adapt the checks based on what's present (the `cargo`-specific checks apply only to Rust repos).

## Checks to Run

### 1. Git Status
```bash
git status --short
git log --oneline -5
```

### 2. Dependencies
```bash
# Check for outdated crates
cargo outdated --workspace 2>/dev/null || echo "cargo-outdated not installed"
# Detect unused dependencies
cargo machete 2>/dev/null || echo "cargo-machete not installed"
```

### 3. Compilation & Lints
```bash
cargo check --workspace --locked 2>&1 | tail -20
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -20
```

### 4. Code Quality
- Search for TODOs/FIXMEs: `rg -n "TODO|FIXME|HACK|XXX" -g "*.rs" crates/ | head -20`
- Look for `dbg!` / `println!` left in production code
- Look for `unwrap()` / `expect()` on fallible paths

### 5. Security Quick Scan
- Check for hardcoded secrets patterns
- Verify `.env` files are gitignored
- Check dependency vulnerabilities + licenses: `cargo deny check 2>/dev/null && cargo audit 2>/dev/null`

### 6. Bundle / Build
- Verify build succeeds
- Check for large files that shouldn't be committed

## Output
Report findings grouped by category with severity indicators:
- OK: no issues found
- WARN: potential issues
- ERROR: action needed

$ARGUMENTS
