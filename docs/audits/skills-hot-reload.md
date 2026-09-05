<!-- SPDX-License-Identifier: Apache-2.0 -->
# `skills-hot-reload` — integration notes

`scripts/skills-hot-reload.ts` watches `.claude/skills/` and
`.claude/agents/`, re-parses YAML frontmatter on every change, writes
a flat JSON manifest, and touches a single signal file. Downstream
runtimes can `fs.watch` the signal file as a cheap zero-payload
reload trigger.

Generated: 2026-05-17T21:24:52.085Z.

## Outputs

- **Manifest** `C:\src\aphrody\var\data\skills-manifest.json` — `{generated_at, count, entries[]}`. Each entry is `{kind, id, name, description, when_to_use, triggers[], source, relSource, dir, mtimeMs, valid}`.
- **Signal**   `C:\src\aphrody\var\data\skills-reload.signal` — one line per touch: `<iso8601> <reason>`. Watchers can ignore the content and react on `mtime`.

## Run modes

```bash
bun run scripts/skills-hot-reload.ts            # watch (poll 1500 ms)
bun run scripts/skills-hot-reload.ts --once     # CI: one scan, exit
bun run scripts/skills-hot-reload.ts \
  --reload-signal=var/data/custom.signal \
  --manifest=var/data/custom-manifest.json
```

`--once` exits with code `1` when any skill fails validation — wire
it into pre-commit or the CI lint pass to catch broken frontmatter
before it lands.

## Integration patterns

### Claude Code (this repo)

Claude Code already re-scans `.claude/skills/` opportunistically
between turns. Run the watcher in the background of a long session
to surface validation failures the moment a SKILL.md is edited — the
signal file is a passive observation channel; the harness needs no
changes.

### openclaw daemon

The daemon's `apps/daemon/src/skills.ts` re-scans on every
`GET /api/skills`. Replace the per-request scan with a single
`fs.watch` against the signal file:

```ts
import { watch } from 'node:fs';
let cached = await listSkills(roots);
watch('C:/src/aphrody/var/data/skills-reload.signal', () => {
  listSkills(roots).then((next) => { cached = next; });
});
app.get('/api/skills', (_, res) => res.json(cached));
```

### Hypothetical `aphrody-runtime` crate

In Rust, use `notify`'s `RecommendedWatcher` against the signal file
and reload an `ArcSwap<Vec<Skill>>` on each event. The signal file
is short enough to fit a single `read_to_string`; the manifest is
the source of truth for the parse output.

```rust
let (tx, rx) = std::sync::mpsc::channel();
let mut w = notify::recommended_watcher(tx)?;
w.watch(Path::new("C:/src/aphrody/var/data/skills-reload.signal"), RecursiveMode::NonRecursive)?;
for _evt in rx { reload_manifest(&manifest_path)?; }
```

## Validation rules

- File must begin with `---` (YAML frontmatter delimiter).
- `name` must match `/^[a-z0-9][a-z0-9-]*$/`.
- `description` must be a non-empty string.
- Either `when_to_use` (aphrody schema) **or** non-empty `triggers:`
  (open-design schema) must be present.

A single skill failing validation never crashes the watcher — it is
recorded as `{valid: false, error}` in the manifest and printed to
stdout once. The watcher keeps running.
