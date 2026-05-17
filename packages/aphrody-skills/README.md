<!-- SPDX-License-Identifier: Apache-2.0 -->

# @aphrody/skills

Unified skill-aggregator CLI for `SKILL.md` files. One binary, six upstream
catalogs, zero npm dependencies beyond Bun's standard library.

```bash
bunx @aphrody/skills           # alias for `list`
bunx @aphrody/skills list      # flat table: NAME | SOURCE | MODE | LOC | DESCRIPTION
bunx @aphrody/skills sources   # print the source registry
bunx @aphrody/skills sync      # mirror everything into ~/.aphrody/skills/
```

## Why

Agent skills live in many places — every team picks its own directory layout
(`.claude/skills/`, `.gemini/skills/`, `.agents/skills/`, `skills/`). This
package gives you ONE CLI surface that:

1. **Discovers** SKILL.md files in every known location at runtime.
2. **Normalises** the four schema variants (open-design, vercel, gemini,
   aphrody) into a flat table.
3. **Synchronises** a per-user mirror under `$APHRODY_SKILLS_HOME` so you can
   load skills the same way regardless of which upstream they came from.

The package does NOT vendor any SKILL.md content. It reads from your local
checkouts of the upstream repos — keeping the published package tiny
(< 50 KB) and always in sync with `git pull` upstream.

## Sources

| Slug          | Schema       | Default path                                                 | Upstream                                              |
|---------------|--------------|--------------------------------------------------------------|-------------------------------------------------------|
| open-design   | open-design  | `C:/worktree/open-design/skills`                             | https://github.com/nexu-io/open-design                |
| openclaw      | open-design  | `C:/worktree/openclaw/.agents/skills`                        | https://github.com/openclaw/openclaw                  |
| claude-code   | aphrody      | `<project>/.claude/skills`                                   | https://github.com/anthropics/skills                  |
| gemini-cli    | gemini       | `C:/worktree/gemini-cli/.gemini/skills`                      | https://github.com/google-gemini/gemini-cli           |
| vercel-labs   | vercel       | `C:/worktree/vercel-agent-skills/skills`                     | https://github.com/vercel-labs/agent-skills           |
| vercel-skills | vercel       | `C:/worktree/vercel-skills/skills`                           | https://github.com/vercel-labs/skills                 |
| vercel-open-agents | vercel  | `C:/worktree/open-agents/.agents/skills`                     | https://github.com/vercel-labs/open-agents            |
| google-labs   | open-design  | `C:/worktree/google-labs-skills/skills`                      | https://github.com/google/labs-skills                 |

Any source can be overridden with an env variable (see `.env.example`).
Missing sources are silently skipped — `skills sources` reports their status.

## CLI reference

```
skills list [--source=<slug>] [--json]
skills info <name> [--source=<slug>]
skills sync [--source=<slug>] [--dry-run]
skills install <name> [--source=<slug>]
skills where <name> [--source=<slug>]
skills sources [--json]
skills --help
skills --version
```

Every subcommand accepts `--help` for inline usage.

### Manifest

`skills sync` and `skills install` write to:

```
$APHRODY_SKILLS_HOME/<source>/<name>/SKILL.md
$APHRODY_SKILLS_HOME/manifest.json
```

The manifest tracks `{ name, source, source_path, copied_at, size_bytes }`
for every mirrored skill. Default `$APHRODY_SKILLS_HOME` is
`~/.aphrody/skills/` (or `%USERPROFILE%\.aphrody\skills\` on Windows).

## Installation

The package auto-resolves through Bun's workspace pattern under
`packages/*`. Run from the repo root:

```bash
bun install
bun run packages/aphrody-skills/src/cli.ts sources
```

For ad-hoc use outside this repo, publish to npm and call via `bunx`:

```bash
bunx @aphrody/skills list
```

## Programmatic API

```ts
import {
  SOURCES,
  discoverSkills,
  parseFrontMatter,
  skillsHome,
  upsertEntry,
} from "@aphrody/skills";

for (const spec of SOURCES) {
  for (const skill of discoverSkills(spec)) {
    console.log(spec.slug, skill.name, skill.skillMdPath);
  }
}
```

## License

Apache-2.0. Each upstream catalog keeps its own license — this package only
indexes and copies SKILL.md files; it does not relicense them.
