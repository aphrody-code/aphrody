<!--
  Pull Request template â€” Aphrody
  Conventional Commits required: <type>(<scope>): <subject>
-->

## Summary

<!-- A clear, concise description of WHAT changed and WHY. Reference the PLAN.md phase if applicable. -->

## Type

- [ ] `feat` â€” new feature
- [ ] `fix` â€” bug fix
- [ ] `refactor` â€” code change, no feature/bug
- [ ] `perf` â€” performance
- [ ] `docs` â€” documentation only
- [ ] `test` â€” tests added/fixed
- [ ] `build` â€” build system, deps, Cargo.toml
- [ ] `chore` â€” auxiliary tools

Breaking change? â†’ tick `!` notation: `<type>(<scope>)!: <subject>`

## Issue / Phase reference

<!-- Closes #N, related to docs/PLAN.md Â§P10 / P11 / P12 -->

## Cross-platform impact

- [ ] Compiles on `x86_64-pc-windows-msvc`
- [ ] Compiles on `x86_64-unknown-linux-gnu`
- [ ] Compiles on `aarch64-apple-darwin`
- [ ] Uses `crates/cli/src/platform.rs` abstractions (no direct `LOCALAPPDATA`, `HOME` etc.)
- [ ] N/A â€” Windows-only crate (`google_os`, `base` DPAPI)

## Checklist (mandatory)

- [ ] `cargo ci-offline` passes locally (zero warnings, `--locked --offline`)
- [ ] `cargo deny check` passes (CVE + licences + bans + sources)
- [ ] `cargo nextest run -p <crate>` passes if I touched the crate
- [ ] New deps reviewed: `cargo vet suggest` + justified in `supply-chain/audits.toml` or `deny.toml` ignore
- [ ] `docs/SUMMARY.md` regenerated if I added doc files (`bun run docs:summary`)
- [ ] Conventional Commit message in PR title
- [ ] FFI surface (if touched): `# Safety` doc + tested with `cargo miri test` or `cargo asan`

## Screenshots / output (if UX-relevant)

<!-- Paste terminal output, screenshots, performance numbers, etc. -->
