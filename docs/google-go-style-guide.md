<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: 2026 aphrody contributors -->

# Google Go Style Guide (distilled)

## Where it lives in-repo

Imported by commit `976b998f2` ("feat: set up Go and import Google Go Style
Guide", 2026-05-21). That commit also installed Go 1.26.3 on the host
(`go version go1.26.3 windows/amd64`) and registered it in `google.json`
(`tools.go = "go"`, `GoLang.Go` 1.26.3 providing `go`/`gofmt`) and `url.json`
(four `go_style_*` URLs).

Files (verbatim Markdown conversions of <https://google.github.io/styleguide/go>):

| File | Upstream page | Status |
|---|---|---|
| [`index.md`](index.md) | <https://google.github.io/styleguide/go/index> | Overview / definitions |
| [`guide.md`](guide.md) | <https://google.github.io/styleguide/go/guide> | **Normative & canonical** core style |
| [`decisions.md`](decisions.md) | <https://google.github.io/styleguide/go/decisions> | Normative style decisions |
| [`best-practices.md`](best-practices.md) | <https://google.github.io/styleguide/go/best-practices> | Non-normative patterns |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | (in-repo) | Go module design notes for aphrody |

The guide is a **readability reference**, not an aphrody build policy — aphrody
itself ships no Go (see [`CLAUDE.md`](../../CLAUDE.md) §2).

## The four documents (from `index.md`)

| Document | Audience | Normative | Canonical |
|---|---|---|---|
| Style Guide (`guide.md`) | Everyone | Yes | Yes |
| Style Decisions (`decisions.md`) | Readability mentors | Yes | No |
| Best Practices (`best-practices.md`) | Anyone interested | No | No |

- **Canonical** = prescriptive, enduring rules all code (old and new) should
  follow; not expected to change.
- **Normative** = agreed-upon style for reviewers; may evolve over time.
- **Idiomatic** = common, familiar Go patterns; prefer idiomatic over
  unidiomatic when both serve the same purpose.

Baseline assumed: [Effective Go](https://go.dev/doc/effective_go).

## Core style principles (`guide.md`)

Attributes of readable code, **in order of importance**:

1. **Clarity** — purpose and rationale clear to the reader (favoured over ease
   of writing). Comments should explain *why*, not *what*; let self-describing
   names speak. Avoid redundant comments that add maintenance burden.
2. **Simplicity** — accomplish the goal the simplest way; no unnecessary
   abstraction; add complexity only deliberately and with documentation. "Least
   mechanism": prefer a core language construct (channel, slice, map, loop,
   struct) → then the standard library → then a core Google library → only then
   a new dependency. (E.g. `map[string]bool` for a set check before pulling a
   set library.)
3. **Concision** — high signal-to-noise ratio; cut repetition, extraneous
   syntax, opaque names, unnecessary abstraction. Table-driven tests factor out
   repetition.
4. **Maintainability** — code is edited far more than written; make it easy to
   change correctly; minimise dependencies; comprehensive tests with actionable
   diagnostics; don't hide critical logic where a `=` vs `:=` or a stray `!` is
   easy to miss.
5. **Consistency** — look/feel/behave like surrounding code. Consistency breaks
   ties but never overrides the principles above; package-level consistency
   matters most.

### Core guidelines (canonical)

- **Formatting**: all source must match `gofmt` output (enforced by presubmit).
  Generated code formatted too (`go/format.Source`).
- **MixedCaps**: `MixedCaps` / `mixedCaps`, never underscores. `MaxLength` (not
  `MAX_LENGTH`) if exported, `maxLength` if unexported. Locals count as
  unexported for initial capitalisation.
- **Line length**: no fixed limit. Don't split before an indentation change, and
  don't split a long string (e.g. a URL) to fit; refactor instead, or leave it
  long.
- **Naming**: shorter than many languages; don't be repetitive, take context
  into account, don't repeat already-clear concepts.
- **Local consistency**: where the guide is silent, follow nearby code; but a
  change must not worsen an existing deviation or spread it to more API surface.

## Key decisions (`decisions.md`) — selected rules

- **Naming**: no `Underscores`; short lowercase **package names** (no
  under_scores or mixedCaps); concise, consistent **receiver names** (1-2 chars,
  same name across methods); `MixedCaps` **constants**; **initialisms** keep one
  case (`URL`, `userID`, not `Url`/`UserId`); **no `Get` prefix** on getters
  (`Count()` not `GetCount()`); avoid **repetition** between package and symbol
  (`bytes.Buffer`, not `bytes.BytesBuffer`).
- **Comments**: doc comments are full sentences starting with the symbol name;
  package comments precede `package`; comment line length is reader-driven.
- **Imports**: group standard vs others with a blank line; rename on collision;
  avoid blank (`import _`) and dot (`import .`) imports except for narrow cases.
- **Errors**: return errors (don't panic); error strings lowercase, no trailing
  punctuation; handle errors (don't discard with `_` casually); avoid in-band
  error sentinels; **indent the error flow**, keep the happy path
  left-aligned; use `%w` to wrap.
- **Don't panic**: use `error` for ordinary failures; `Must…` helpers may panic
  only at init/program-setup time.
- **Goroutines**: lifetimes must be clear — know when/whether each one exits.
- **Interfaces**: accept interfaces, return concrete types; define interfaces
  where they're used (consumer side), not pre-emptively; avoid unnecessary
  interfaces. Use generics only when they genuinely simplify.
- **Receivers / values**: pick value vs pointer receiver consistently per type;
  pass values when small and copy-safe.
- **Contexts**: `context.Context` is the first parameter (`ctx`), never stored
  in a struct.
- **`crypto/rand`** for security-sensitive randomness (never `math/rand`).
- **Testing**: prefer the standard `testing` package over assertion libraries;
  "got before want" in failure messages; identify the function and input in
  failures; **table-driven tests** with named subtests (`t.Run`); use full
  structure comparison / diffs (`cmp.Diff`); test error semantics, not exact
  strings; `t.Error` to continue vs `t.Fatal` to stop; never call `t.Fatal`
  from a separate goroutine.
- **Formatting verbs**: `%q` for quoted strings, `%v`/`%s` per local style,
  `any` (not `interface{}`).

## Key best practices (`best-practices.md`) — selected patterns

- **Naming**: function/method names avoid stutter with the package; dedicated
  test-double / helper packages.
- **Errors**: structure errors (sentinel vars, custom types) deliberately; add
  information when wrapping; place `%w` carefully; log errors at one level only.
- **Initialization & panics**: program init / checks; "when to panic" =
  programmer error only.
- **Strings**: `+` for simple concatenation, `fmt.Sprintf` for formatting,
  `strings.Builder` for piecewise construction; mark constant strings.
- **Documentation**: godoc formatting, preview, "signal boosting" (call out
  subtle code with a comment).
- **APIs**: option structures and variadic options for extensible constructors;
  declare variables with zero values; field names in struct literals; size
  hints for slices/maps; channel-direction types; designing extensible
  validation APIs; prefer real transports in tests over mocks; keep package
  state minimal (litmus tests, default-instance pattern); avoid unnecessary
  interfaces; mind interface ownership/visibility.

## Sources

- In-repo: [`docs/go/index.md`](index.md), [`docs/go/guide.md`](guide.md),
  [`docs/go/decisions.md`](decisions.md),
  [`docs/go/best-practices.md`](best-practices.md), commit `976b998f2`,
  `google.json`, `url.json`.
- External: <https://google.github.io/styleguide/go>,
  <https://google.github.io/styleguide/go/guide>,
  <https://google.github.io/styleguide/go/decisions>,
  <https://google.github.io/styleguide/go/best-practices>,
  <https://go.dev/doc/effective_go>.
