# bxc — Bun + Lightpanda browser engine (placeholder)

The full source lives at <https://github.com/aphrody-code/bxc> on branch
**`aphrody`** (integration branch isolated from upstream `main`).

## What it is

> Bun + Lightpanda fusionnés — browser engine in-process pour Bun.
> curl-impersonate Chrome 131 via bun:ffi, CDP-compat, anti-bot ready.

## Why not vendored here

bxc is a sizable Bun/TypeScript project (~900 files, multiple sub-packages).
Vendoring it inside `aphrody` would inflate the monorepo and entangle the
upstream sync cycle. Instead, we keep it as a separately-versioned fork.

## How to consume from aphrody

### Option A — git clone (development)

```bash
# Clone the aphrody branch into a sibling directory
gh repo clone aphrody-code/bxc -- --branch aphrody ../bxc

# Then in packages/bxc/package.json (if you need it as a workspace):
# "dependencies": { "bxc": "file:../../../bxc" }
```

### Option B — bun deps (production)

In your `package.json`:

```jsonc
{
  "dependencies": {
    "bxc": "github:aphrody-code/bxc#aphrody"
  }
}
```

Then `bun install`.

## Updating the branch

The `aphrody` branch on `aphrody-code/bxc` diverges from `main`. To rebase
on upstream:

```bash
cd /c/worktree/bxc
git checkout aphrody
git fetch origin
git merge origin/main --no-ff -m "merge upstream main into aphrody"
git push origin aphrody
```

See `.aphrody/INTEGRATION.md` in the bxc repo for branch contract.
