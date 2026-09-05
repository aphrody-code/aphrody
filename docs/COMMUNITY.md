<!-- SPDX-License-Identifier: Apache-2.0 -->

# Community

This document describes where the aphrody community lives, how to engage with
it, and what is expected from participants. It is the entry point for anyone
arriving from a launch post, a release announcement, or a search engine result.

If you only have time to read three things, read [`../README.md`](../README.md),
[`../CONTRIBUTING.md`](../CONTRIBUTING.md), and [`../CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md).

## 1. Channels

We deliberately keep the surface small. Pick the channel that matches what you
want to do.

- **GitHub Discussions** — primary channel for open-ended questions, design
  proposals, and project mentions. URL:
  `https://github.com/aphrody-code/aphrody/discussions`.
- **GitHub Issues** — bug reports and feature requests only. Templates live in
  [`../.github/ISSUE_TEMPLATE/`](../.github/ISSUE_TEMPLATE/).
- **GitHub Security Advisories** — vulnerability reports, handled privately.
  See [`../SECURITY.md`](../SECURITY.md) for the disclosure process and SLAs.
- **Discord** — _pending — a public Discord invite will land in the v1.0.0
  release notes._
- **Matrix** — _pending — a matrix.org space will land in the v1.0.0 release
  notes._
- **Email** — `community@aphrody.dev` for non-actionable feedback
  (testimonials, project mentions, conference talk submissions). Not for
  support, not for vulnerabilities.
- **Twitter / Mastodon** — _pending — handles will land after the v1.0.0
  launch._

## 2. Communication norms

- Be technical. Show code. Cite line numbers and commit hashes.
- Be honest. If aphrody is the wrong tool for your use case, we will say so
  and point you elsewhere. See [`MIGRATION.md`](MIGRATION.md) for a comparison
  with adjacent tools and migration paths in both directions.
- Be patient. Maintainers are volunteers; response time is best-effort within
  one week. Vulnerability response follows the SLAs in
  [`../SECURITY.md`](../SECURITY.md).
- Be safe. Every interaction is bound by
  [`../CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md).

## 3. How to contribute

- Read [`../CONTRIBUTING.md`](../CONTRIBUTING.md) first. It covers the local
  toolchain, the commit conventions, and the DCO sign-off requirement.
- For non-trivial changes, open a Discussion before writing the PR. This
  avoids wasted effort and surfaces alignment concerns early.
- For docs and typo fixes, send a PR direct; no discussion needed.
- All PRs must pass the gates: `cargo check`, `cargo clippy -D warnings`,
  `cargo deny check`, `cargo nextest run`. Linux is the primary target; see
  [`../CLAUDE.md`](../CLAUDE.md) for the cross-platform priority order.

## 4. How to ask for help

In order:

1. Check [`FAQ.md`](FAQ.md).
2. Check [`TROUBLESHOOTING.md`](TROUBLESHOOTING.md).
3. Search existing issues and Discussions.
4. Open a Discussion (not an issue) using the question template.

## 5. How to report a bug

- Verify the symptom is not already covered in
  [`TROUBLESHOOTING.md`](TROUBLESHOOTING.md).
- Open an issue using
  [`../.github/ISSUE_TEMPLATE/bug_report.yml`](../.github/ISSUE_TEMPLATE/bug_report.yml).
- Include `aphrody doctor --json` output (auto-redacted) for environment
  context.
- Include reproduction steps that fit in a single shell snippet.

## 6. Contributor recognition

- Every contributor is cited in [`../CHANGELOG.md`](../CHANGELOG.md) entries
  via Conventional Commits trailers.
- Notable contributions land in `CONTRIBUTORS.md` (_pending — auto-generated
  quarterly once v1.0.0 ships_).
- Security researchers who responsibly disclose are credited in the release
  notes and [`../SECURITY.md`](../SECURITY.md).

## 7. Decision process

- Small changes (single-file fixes, tests, docs): maintainer judgment.
- Medium changes (new helper, refactor inside one crate): one maintainer
  approval after a Discussion.
- Large changes (architecture shifts, breaking API, dependency additions,
  platform support): a written design rationale is required in the PR
  description, cross-referenced from [`SOURCE_OF_TRUTH.md`](SOURCE_OF_TRUTH.md).

## 8. Mission alignment

The project mission is stated in [`../README.md`](../README.md) and
elaborated in [`ROADMAP.md`](ROADMAP.md) and
[`SOURCE_OF_TRUTH.md`](SOURCE_OF_TRUTH.md). Contributions that move the
project away from the mission, for example adding telemetry, embedding a GTK
GUI inside the CLI binary, or npm-publishing non-WASM modules, will likely be
declined. If you are unsure whether your change fits, open a Discussion to
explore alignment before investing in the implementation.

## 9. License

All contributions are licensed under Apache-2.0 (see [`../LICENSE`](../LICENSE)).
By submitting a PR, you affirm DCO sign-off as described in
[`../CONTRIBUTING.md`](../CONTRIBUTING.md). No CLA is required.
