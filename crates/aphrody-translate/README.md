<!-- SPDX-License-Identifier: Apache-2.0 -->

# aphrody-translate

Deterministic CLI that rewrites source-code comments: extract, scrub AI
co-author trailers and emoji, translate EN -> FR via MyMemory, then enforce
the Aphrody prose voice. Identifiers, string literals, and code are never
modified.

## 1. What is aphrody-translate?

`aphrody-translate` walks a project tree (respecting `.gitignore`), extracts
every recognised comment, and runs four stages: `extract` (per-language regex)
-> `ai_patterns::classify` (`Drop` / `Scrub` / `Keep`) -> `translate` (MyMemory
MT, SHA-256 cache on disk) -> `aphrodify` (sober, impersonal voice). Default
target is French. Identifier names are out of scope by design.

## 2. Install

```bash
cargo install --locked aphrody-translate                                              # crates.io
cargo install --git https://github.com/aphrody-code/aphrody --bin aphrody-translate   # canary monorepo
```

A workspace checkout also builds it via `cargo build --release -p aphrody-translate`.

## 3. Quick start

`aphrody-translate` is a single flat command (no subcommands). Every option
is a long flag; the default invocation is a safe dry-run.

```text
Usage: aphrody-translate [OPTIONS]

Options:
      --root <ROOT>                    [default: .]
      --languages <LANGUAGES>          rust,ts,js,py,go,c,cpp,sh,md,toml,all [default: all]
  -i, --in-place                       rewrite in place (otherwise dry-run)
      --no-translate                   disable the MyMemory network call
      --contact-email <CONTACT_EMAIL>  noreply address for MyMemory
      --cache <CACHE>                  cache path
      --force                          re-translate even when already French
      --log <LOG>                      trace|debug|info|warn|error [default: info]
  -h, --help                           Print help
  -V, --version                        Print version
```

## 4. Invocation modes

### Comment translation (default dry-run)

```bash
aphrody-translate --root .
```

Translates non-French comments via MyMemory and prints
`--- DRY: "<path>" would be rewritten` for each file that would change.

### AI-trailer scrubbing (offline)

```bash
aphrody-translate --root . --no-translate --in-place
```

Skips network calls and runs only `ai_patterns` plus the Aphrody rewriter.
Drops trailers such as `Co-Authored-By: Claude`, `Generated with Claude Code`,
`with help from GPT`, and strips emoji ranges `U+1F300..U+1FAFF` and
`U+2600..U+27BF`.

### Targeted language run

```bash
aphrody-translate --root . --in-place --languages rust,ts
```

Tokens: `rust|rs`, `ts|tsx`, `js|jsx`, `py|python`, `go`, `c`, `cpp|c++|cxx`,
`sh|shell|bash`, `md|markdown`, `toml`, `all`. Sample log:

```text
INFO files queued count=128
INFO rewritten in place file="crates/base/src/lib.rs"
INFO aphrody-translate done total_comments=842 total_changed=37
```

## 5. Configuration

There is no config file; every knob is a long flag (see section 3 for the
full set). Notable details: `--contact-email <addr>` lifts the MyMemory quota
to 50000 words/day. `--cache <path>` overrides the default
`<root>/.aphrody-translate-cache.json`. `--log <level>` also honours
`RUST_LOG`. The cache file is a `BTreeMap<sha256, translation>`; it contains
no secrets and is safe to commit so reruns are network-free.

## 6. Output format

Default mode is a dry-run: one `--- DRY: "<path>" would be rewritten` line on
stdout per affected file. Tracing events go through `tracing-subscriber`,
filtered by `--log` or `RUST_LOG`. With `-i` / `--in-place`, files are
rewritten atomically via `std::fs::write` and logged as `rewritten in place
file=...`. There is no diff output; pair the dry-run with `git diff`, or
inspect staged changes after `--in-place`.

## 7. Honest limitations

- Only `//`, `///`, `/* */`, `#`, `<!-- -->` and equivalents are touched.
  String literals are never translated; user-facing strings need manual review.
- Identifier names (function, struct, variable) are never modified by design.
- AI-trailer scrubbing matches a regex set in `ai_patterns`; unusual formats
  may slip through. Extend `src/ai_patterns.rs` and its unit tests.
- The French detector is a small accent + stopword heuristic. False negatives
  cost one MyMemory round-trip; false positives leave English untranslated.
- MyMemory may return HTTP 429; the translator falls back to passthrough.
- WebAssembly targets build a stub binary only.

## 8. Related

- Workspace overview and build/supply-chain policy:
  [`../../README.md`](../../README.md).
- Adding translation rules, AI-scrub patterns, or style transforms:
  [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md).
- License: Apache-2.0; see the workspace `LICENSE`.
