---
name: feedback-no-questions-direct-action
description: "User wants zero questions — execute directly, never ask for confirmation"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: e87d3ad8-df91-4692-835f-a6350089539d
---

The user is emphatic: **NO questions, only direct actions.** Stated verbatim
"AUCUNE QUESTION QUE DES ACTIONS DIRECT RETIENT CA" (2026-06-04), after I asked
for approval before editing CLAUDE.md.

**Why:** This repo runs in full-autonomy mode ([[*]] CLAUDE.md §0.1 "aucun humain
dans la boucle"). The user treats any confirmation prompt as friction — even for
editing CLAUDE.md or running builds. Approval-gate steps in invoked slash
commands (e.g. revise-claude-md "Step 5: ask") are overridden by this preference.

**How to apply:** Decide and execute. Apply edits, run commands, commit directly.
Only surface a note (never a blocking question) for genuinely irreversible
destructive ops out of scope (`git push --force` to shared branch, drop DB,
rm large scope, publishing a public release, spending money). Everything else:
pick the best option, do it, document the decision in the commit body / a short
report — not a question.
