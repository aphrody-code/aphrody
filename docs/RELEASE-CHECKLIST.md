<!-- SPDX-License-Identifier: Apache-2.0 -->

# Release Checklist

Per-release maintainer checklist for `aphrody`. Run through every section
before invoking [`scripts/release.sh`](../scripts/release.sh) with `--push`.
Items here are the human-judgement steps that automation cannot
substitute for.

Copy this checklist into the release tracking issue (or PR description)
and tick boxes as you go.

## 1. Pre-flight (T-24h)

- [ ] All open security advisories triaged
      (`gh api repos/aphrody-code/aphrody/security-advisories` or GitHub UI).
- [ ] All Dependabot PRs reviewed; deny-list (per
      [`.github/dependabot.yml`](../.github/dependabot.yml)) intact.
- [ ] All in-flight Discussions answered or moved to issues.
- [ ] [`docs/PLAN.md`](PLAN.md) reviewed; all `Pending` items
      either shipped or explicitly punted.
- [ ] [`docs/ROADMAP.md`](ROADMAP.md) reflects current quarter targets.
- [ ] [`docs/FAQ.md`](FAQ.md) covers new features in this release.

## 2. Test gates (run locally)

- [ ] `cargo check --workspace --all-targets --locked --offline` exit 0
- [ ] `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` exit 0
- [ ] `cargo nextest run --workspace --locked --offline` 100 percent pass
- [ ] `cargo deny check` advisories, bans, licenses, sources all ok
- [ ] `cargo machete --with-metadata` no unused deps
- [ ] `cargo vet --locked` no formatting errors (unvetted backlog acceptable)
- [ ] `cargo bench --workspace --no-run` exit 0
- [ ] `cargo doc --workspace --no-deps --locked` exit 0
- [ ] Cross-target: `cargo check -p aphrody --target wasm32-unknown-unknown --locked` exit 0
- [ ] WASM: `wasm-pack build crates/aphrody-wasm --target web --release` produces `pkg/`

## 3. Docs sync

- [ ] [`CHANGELOG.md`](../CHANGELOG.md) Unreleased section reflects all
      merged work since last tag (run
      [`scripts/changelog-since.sh`](../scripts/changelog-since.sh) for preview).
- [ ] [`docs/SUMMARY.md`](SUMMARY.md) regenerated
      (`bun run scripts/gen_summary.ts`).
- [ ] All cross-links resolve (manual spot-check on
      [`README.md`](../README.md), [`PLAN.md`](PLAN.md), recent
      [`docs/posts/`](posts/) entries).
- [ ] No `_pending_` markers for features being released in this version.

## 4. Version bump

- [ ] `Cargo.toml [workspace.package].version` matches the planned release version.
- [ ] `.release-please-manifest.json` matches `Cargo.toml` version.
- [ ] Per-crate `version.workspace = true` everywhere (no per-crate overrides).
- [ ] All `publish = true` crates have a `description`, `license`,
      `homepage`, `repository`, `keywords`, and `categories` set.

## 5. Tag and push

- [ ] `./scripts/release.sh <version>` to verify gates and create local
      commit and tag.
- [ ] Inspect the local diff (`git show v<version>`) before pushing.
- [ ] `./scripts/release.sh <version> --push` to push (or manual
      `git push origin main && git push origin v<version>`).

## 6. Post-tag

- [ ] Watch `.github/workflows/release.yml` run for the tag (binary builds).
- [ ] Watch `.github/workflows/release-please.yml` open the next-version
      bump PR.
- [ ] Verify the GitHub Release page lists all expected artefacts
      (binaries per OS/arch, SBOM, checksums, source tarball).
- [ ] If publishing to crates.io: follow
      [`docs/cargo/PUBLISH-LADDER.md`](cargo/PUBLISH-LADDER.md)
      (8 rungs in order).
- [ ] Update Homebrew tap, Scoop bucket, AUR, and any other downstream
      package manifests with the new SHA-256 sums.

## 7. Announce (Show HN, Lobste.rs, etc.)

Per [`docs/POST-LAUNCH.md`](POST-LAUNCH.md) Show HN playbook (for
`v1.0.0` stable; canary and minor releases typically skip the full
launch flow and post only to the project Discussions board).

## 8. Rollback plan

If a release breaks production for users:

- Yank from crates.io: `cargo yank --vers <version> -p <name>`
  (per [`docs/cargo/PUBLISH-LADDER.md`](cargo/PUBLISH-LADDER.md) section 5).
- Revert the offending commit on `main`; tag `v<version>.1` patch with
  the fix.
- DO NOT delete the tag or release on GitHub (history immutability).
- Open a post-mortem issue tagged `release-rollback` documenting the
  failure mode and the gate that should have caught it; feed the
  finding back into section 2 of this checklist.

## 9. Sign-off

Maintainer initials and ISO-8601 date:

```
[ ] aphrody-code  2026-??-??
```

Archive the completed checklist (with ticked boxes) as a comment on the
release tracking issue so future audits can reconstruct which gates
were green at tag time.
