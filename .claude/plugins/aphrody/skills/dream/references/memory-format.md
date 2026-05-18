# Memory File Format Specification

## MEMORY.md (Index)
- No frontmatter
- Each entry is one line under ~150 characters
- Format: `- [Title](file.md) — one-line hook`
- Maximum 200 lines / 25KB
- Lines after 200 are truncated when loaded

## Topic Files
YAML frontmatter with 3 required fields:

```markdown
---
name: {{memory name}}
description: {{one-line — used for relevance matching, be specific}}
type: {{user|feedback|project|reference}}
---

{{content}}
```

## Memory Types

### user
User's role, goals, preferences, knowledge.
```markdown
---
name: user-role
description: User is a senior fullstack dev focused on Beyblade X community platform
type: user
---
Senior fullstack TypeScript developer. Runs the RPB (République Populaire du Beyblade) community.
Prefers concise responses, autonomous execution, French for UI text.
```

### feedback
Corrections AND confirmed approaches from the user.
```markdown
---
name: feedback-testing
description: Integration tests must hit real DB, not mocks — prior incident with mock/prod divergence
type: feedback
---
Integration tests must use a real database, never mocks.
**Why:** Prior incident where mocked tests passed but prod migration failed.
**How to apply:** When writing tests for DB operations, always use the test database.
```

### project
Ongoing work context not derivable from code or git.
```markdown
---
name: project-freeze
description: Merge freeze starting 2026-04-15 for mobile release cut
type: project
---
Merge freeze begins 2026-04-15 for mobile release branch cut.
**Why:** Mobile team needs a stable branch for release.
**How to apply:** Flag any non-critical PR work scheduled after that date.
```

### reference
Pointers to external systems.
```markdown
---
name: reference-bugs
description: Pipeline bugs tracked in Linear project INGEST
type: reference
---
Pipeline bugs are tracked in Linear project "INGEST".
API monitoring dashboard: grafana.internal/d/api-latency
```

## What NOT to Save
- Code patterns, architecture, file paths (derivable from code)
- Git history (use git log/blame)
- Debugging solutions (the fix is in the code)
- Anything already in CLAUDE.md
- Ephemeral task details
