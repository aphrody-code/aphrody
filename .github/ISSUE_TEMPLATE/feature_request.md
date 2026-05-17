---
name: Feature request
about: Propose a new capability for the cross-platform `cli` binary
title: "feat: <short title>"
labels: ["enhancement"]
---

## Motivation

<!-- Why is this needed? What user / contributor problem does it solve? -->

## Proposal

<!-- High-level design. -->

## Cross-platform considerations

- [ ] Must work on Windows
- [ ] Must work on Linux
- [ ] Must work on macOS
- [ ] Must work on Android (cargo-ndk)
- [ ] Must work on wasm (`wasm32-wasip1`)
- [ ] Windows-only (gated `#![cfg(windows)]`) — justify below
- [ ] Other: ___________

## Alignement Google patterns

Does this feature have a corresponding pattern in AOSP / Chromium / Fuchsia?
If so, link to the relevant doc and explain how we follow it.

## Alternatives considered

<!-- What other designs did you consider? Why is this proposal preferred? -->

## Acceptance criteria

- [ ] `cargo ci-offline` passes
- [ ] Tests added (specify which crate / file)
- [ ] `docs/cargo/` updated if architectural
- [ ] No new untracked dep (or audit added)
