---
name: n2b-fix-and-agent-stack-bun
description: n2b --fix only applies the safe class; signature-changing autofixes need manual work. Agent stack (claude/grok/agy/aphrody/bxc) is fully Node-free.
metadata: 
  node_type: memory
  type: reference
  originSessionId: e87d3ad8-df91-4692-835f-a6350089539d
---

n2b binary lives at `/home/ubuntu/.local/bin/n2b` (compiled Rust). JSON report
shape: top-level `{schema_version, files_scanned, findings_total, files:[{path, findings:[{rule_id, severity, autofix, line, message}]}]}` — findings are nested under `files[]`, NOT a flat array. Run with `--report json --quiet --ignore "**/vendor/**" ...`; exit code 1 means findings exist.

**Gotcha — `n2b --fix` is conservative beyond the `autofix` flag.** Many findings
carry `autofix:true` in the JSON (e.g. `api/sleep-promise`, `api/fs-readFileSync`,
`api/fs-existsSync`, `api/fs-writeFileSync`) yet `--fix` reports "fichiers modifiés : 0"
and changes nothing. Reason: `--fix` only applies the genuinely-safe class
(`node:` prefix, shebang, `cli`, `ci`); it refuses sync→async fs rewrites and
sleep-promise rewrites because they change function signatures / can't auto-rewrite
callers. To apply those: `--aggressive` (invasive, repo-wide) or convert by hand.
- `await new Promise(r => setTimeout(r, ms))` → `await Bun.sleep(ms)` (global, no import) — always safe in an existing async context.
- `fs.readFileSync/existsSync` → `await Bun.file(p).text()/.exists()` — ONLY if the enclosing fn is already async; in a sync public method, leave `node:fs` (it is fully Bun-native, the warn is stylistic).
- `api/child-process-spawn` is a naive token match: it flags `Bun.spawn(` too. Most bxc hits are false positives (already `Bun.spawn`). Verify before "fixing".

**Agent stack is Node-free (verified 2026-06-04).** `claude` (native ELF, ~/.local/share/claude/versions/*), `grok` (native ELF, ~/.grok), `agy`=gemini bridge (native ELF), `aphrody` (native Rust ELF) — none are node scripts. `bxc` is a bash wrapper that execs the bun-compiled `dist/standalone/$BINARY` or falls back to `bun run src/cli/index.ts`. No `#!/usr/bin/env node`, no `npx`, no `node`-as-command in first-party scripts of either aphrody or bxc.
