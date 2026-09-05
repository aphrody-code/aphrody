<!--
  SPDX-License-Identifier: Apache-2.0
  Pull Request template — aphrody
  PR title MUST follow Conventional Commits: <type>(<scope>): <subject>
  Breaking change? Use `!` notation: <type>(<scope>)!: <subject>
-->

## Summary

<!--
  1-3 bullets: what changed and WHY. Reference `docs/PLAN.md` phase
  if applicable. Avoid restating the diff — explain intent.
-->

-
-

## Type of change

- [ ] `fix` — bug fix (non-breaking)
- [ ] `feat` — new feature (non-breaking)
- [ ] `feat!` / `fix!` — breaking change (SemVer major)
- [ ] `docs` — documentation only
- [ ] `refactor` — code change, no feature/bug
- [ ] `test` — tests added or fixed
- [ ] `chore` / `build` / `ci` — tooling, deps, infra

## Honest-delivery classification

<!--
  Per docs/extensions/honest-delivery-v1.md, classify EACH deliverable
  in this PR as FAIT (done + verifiable artifact), INCOMPLET (partial
  + named missing piece), or NON_FAIT (blocked + named blocker).

  Mark INCOMPLET / NON_FAIT items explicitly — do not silently elide them.
-->

- [ ] All deliverables are `FAIT` (no partial work in this PR)
- [ ] This PR contains `INCOMPLET` items (listed below with the named missing piece)
- [ ] This PR contains `NON_FAIT` items (listed below with the named blocker)

**Per-deliverable status** (one bullet per claim):

- `FAIT` — <claim> — artifact: <commit SHA / file path / passing test name / 200 URL>
- `INCOMPLET` — <claim> — missing: <named piece, e.g. `parse_header` still returns `unimplemented!()` at `crates/foo/src/parser.rs:42`>
- `NON_FAIT` — <claim> — blocker: <named blocker, e.g. waiting on upstream PR agntcy/slim#123>

## Cross-platform impact

- [ ] Compiles on `x86_64-unknown-linux-gnu` (cible #1, mandatory)
- [ ] Compiles on `x86_64-pc-windows-msvc` (cible #2)
- [ ] Compiles on `wasm32-unknown-unknown` (cible #3)
- [ ] Uses `crates/base` / platform abstractions (no raw `LOCALAPPDATA` / `HOME` reads in cross-platform code)
- [ ] Windows-specific code is gated behind `#[cfg(target_os = "windows")]`
- [ ] N/A — touches platform-specific crate only (`gui`, etc.)

## Test plan

<!--
  Checklist a reviewer can follow locally to verify the change.
  Shell commands preferred.
-->

- [ ] `cargo ci-offline` (clippy --workspace --all-targets --locked --offline -- -D warnings)
- [ ] `cargo xt-offline` (nextest run --workspace --locked --offline)
- [ ] `cargo deny check` (CVE + licences + bans + sources)
- [ ] Manual reproduction of the user-visible behavior:
  ```bash
  # commands here
  ```

## Checklist

- [ ] PR title follows Conventional Commits
- [ ] `cargo ci-offline` passes locally (zero warnings)
- [ ] `cargo deny check` passes (no new CVEs, license rejects, banned crates)
- [ ] `cargo nextest run -p <crate>` passes for every crate I touched
- [ ] New deps reviewed: `cargo vet suggest` or justified in `supply-chain/audits.toml` / `deny.toml`
- [ ] New source files carry `// SPDX-License-Identifier: Apache-2.0`
- [ ] `CHANGELOG.md` updated if the change is user-visible
- [ ] `docs/SUMMARY.md` regenerated if I added doc files (`cargo run -p aphrody-summary`)
- [ ] FFI surface (if touched): `# Safety` doc + tested with `cargo miri test` or sanitizer
- [ ] **No AI co-author trailers** (no `Co-Authored-By: Claude / Copilot / GPT-*` lines)

## Related issues

<!-- `Closes #N`, `Refs #N`, links to upstream issues, design docs, etc. -->

-
