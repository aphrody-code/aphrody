<!-- SPDX-License-Identifier: Apache-2.0 -->

# Migrating to aphrody

A pragmatic, tool-by-tool guide for engineers who already have a working
setup and want a concrete path to `aphrody`. For feature parity at a glance,
see [`COMPARISON.md`](COMPARISON.md). For conceptual questions, see
[`FAQ.md`](FAQ.md). To contribute a recipe, read
[`CONTRIBUTING.md`](../CONTRIBUTING.md).

The honest stance: aphrody replaces some tools cleanly, complements others,
and is the wrong tool for a few. Each section below states which.

## 1. From `just` (justfile → aphrody)

Map every recipe to an `aphrody auto <name>` invocation. aphrody's `auto`
subcommand embeds shell execution, so a one-shot migration is feasible.

Migration script idea: parse the `justfile`, emit `_aphrody_recipes.sh`
with one shell function per recipe, and let `aphrody auto` source it.

```text
# justfile                          # aphrody equivalent
build:                              aphrody auto build
    cargo build --release           # → runs `cargo build --release`

test:                               aphrody auto test
    cargo nextest run               # → runs `cargo nextest run`

deploy host:                        aphrody auto deploy --arg host=prod
    ssh {{host}} 'systemctl reload' # → ssh $host 'systemctl reload'
```

What aphrody adds over `just`: A2A cross-agent coordination, embedded
AGNTCY a2a/v0.4 manifest, parallel YOLO grind, and WASM-target awareness.
What `just` does better: zero-magic execution. Keep `just` if that
suffices.

## 2. From `taskfile` (Taskfile.yml → aphrody)

Map each `tasks:` entry to the same `aphrody auto` shim. The YAML-to-shell
translation is mechanical for plain task definitions.

Known gap: aphrody does not (yet) implement taskfile's `vars:` templating,
`includes:` for splitting a Taskfile, or `internal: true` markers. If your
Taskfile leans heavily on those, **DO NOT migrate** the whole thing —
keep your Taskfile and invoke specific tasks from aphrody recipes
(`aphrody auto release: task release-prod`).

When migration is worth it: flat task lists with shell bodies and no
templating. When it is not: matrix builds with computed variables, deep
`includes:` trees, or `precondition:`/`status:` checks driving the run.

## 3. From `gh` (GitHub CLI → aphrody)

**DO NOT migrate.** aphrody is not a GitHub CLI replacement, and trying to
make it one would be wasted work. The official `gh` is the right tool for
pull requests, issues, releases, gists, workflow runs, and project boards.

Combine the two: `gh` for everything GitHub-specific, `aphrody` for
everything else. Concretely, aphrody complements `gh` with:

- `aphrody doctor` — environment diagnostic (toolchain, lockfile, A2A
  manifest, target triples).
- `aphrody a2a` — cross-agent coordination over file-based mailbox channels
  (`ai.json` + `.coord/inbox-*.jsonl`).
- `aphrody chromium sync` — Chromium profile + forensics surface that `gh`
  intentionally does not touch.

A typical flow: `gh pr create` opens the PR, `aphrody auto ci` runs the
local gates, an A2A peer is notified via `aphrody a2a send`.

## 4. From `devcontainer.json` (Codespaces → aphrody-only)

**DO NOT migrate.** devcontainers and aphrody solve unrelated problems:
the former gives a reproducible containerised IDE, the latter is a
host-native cross-platform CLI. Use both.

aphrody ships its own `.devcontainer/devcontainer.json` (commit b228509)
so contributors who prefer Codespaces get a working setup. Inside that
container, the `aphrody` CLI is the incremental value: A2A coord, MRX
scanning, `aphrody doctor`, and parallel grind on top of the base image.

If you rely on devcontainer features for tool installation, keep doing
that — add `aphrody` to your `postCreateCommand`.

## 5. From `asdf` (runtime version manager → aphrody)

**DO NOT migrate.** aphrody does not manage external runtime versions and
never will. `asdf` (and its successors `mise` / `proto`) is the correct
tool for pinning Node, Python, Ruby, Java, or polyglot toolchains.

aphrody pins exactly one thing: its own nightly Rust toolchain via
`rust-toolchain.toml` at repo root, consumed by `rustup` directly. If you
need Bun pinned, use `bun --version` checks in `aphrody auto bootstrap`;
for Python or Node, keep `asdf`/`mise`.

## 6. From a custom shell script collection

The easiest migration. Move scripts into a dedicated directory, then
wrap each with an `aphrody auto` invocation in your README or
`Taskfile`-replacement.

```bash
mkdir -p .aphrody-scripts
for f in scripts/*.sh; do mv "$f" .aphrody-scripts/; done
# Then in your docs:
#   aphrody auto deploy   → runs .aphrody-scripts/deploy.sh
```

Benefit: a single discoverable entry point (`aphrody auto --list`), no
PATH manipulation, shell-completion-aware. Cost: one extra indirection
when debugging. Drop the wrapper if a recipe is invoked only once a year.

## 7. From `make` / `Makefile`

Map targets to `aphrody auto <name>` for the common case, to shell
wrappers for legacy build steps, or to `cargo make` if the workflow is
Rust-specific and benefits from `cargo`-integrated tasks.

Important: aphrody does **not** implement Makefile's dependency graph or
incremental rebuild semantics. If your Makefile drives a real build graph
(C/C++ sources with header dependencies, code generation with timestamp
checks), **DO NOT migrate** the build portion — keep `make` for builds and
call `make` from `aphrody auto build`. Migrate only the convenience
targets (`make lint`, `make fmt`, `make docs`).

## 8. Common migration pitfalls

- Do not migrate everything in one pull request. Pick the three most-used
  recipes, ship them, gather feedback, then expand. Big-bang migrations
  burn goodwill.
- `aphrody auto` invokes a shell. Escape special characters (`$`, backticks,
  glob metacharacters) the same way you would in any shell script. The
  rules of your local shell apply.
- If you depend on `just`-specific features (env file loading via
  `set dotenv-load`, attribute decorators like `[no-cd]`, recipe
  parameters with defaults), keep `just` and run aphrody alongside.
- A2A coordination assumes the peer has a `.well-known/ai.json` discoverable
  endpoint. If your team has no peer agent, the A2A surface is dormant —
  not broken, just unused.
- aphrody's nightly Rust pin can clash with stable-only environments.
  Read `rust-toolchain.toml` before building in CI.

## 9. Migration assistance

Stuck on a recipe? Open a GitHub issue using the `question.yml` template
at `.github/ISSUE_TEMPLATE/question.yml`. Tag it `migration` so the
triage rotation can route it correctly. Include the source tool, the
recipe you are trying to convert, and what you tried.

Migration recipe pull requests are explicitly welcome. The
[`CONTRIBUTING.md`](../CONTRIBUTING.md) guide describes the workflow:
fork, add your recipe under `docs/MIGRATION.md` (or a sibling cookbook
file if substantial), open the PR, and link the upstream tool's
documentation so reviewers can verify the mapping. Honest disclaimers
("aphrody does not cover X — keep tool Y") are encouraged, not penalised.
