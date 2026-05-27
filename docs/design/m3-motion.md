<!-- SPDX-License-Identifier: Apache-2.0 -->
# Material 3 motion

A recap of the Material Design 3 **motion** system — easing and duration
tokens — mapped from <https://m3.material.io/styles/motion/overview> and the
canonical spec page
<https://m3.material.io/styles/motion/easing-and-duration/tokens-specs>. All
prose is **paraphrased**; the concrete bézier/ms values below are factual token
data quoted as-is from the spec tables. The closing section cross-references
[`crates/m3-tokens/src/motion.rs`](../../crates/m3-tokens/src/motion.rs) and
lists which tokens our crate has versus which are **missing**. See also the
[design-tokens foundation](m3-design-tokens.md).

## How motion works

M3 motion makes UI feel responsive and expressive by pairing an **easing**
curve (the shape of acceleration/deceleration over time) with a **duration**
(how long the transition runs). Larger / more expressive transitions use the
*emphasized* easing set with longer durations; small utility transitions use the
*standard* set with short durations. All values are published as
`md.sys.motion.*` system tokens.

## Easing tokens

Two sets, three curves each. Values are the official CSS `cubic-bezier(...)`
forms (control points). Note the **emphasized (full)** curve is a *non-cubic*
path on Android/Flutter; CSS/iOS have **no single-bézier equivalent** and the
spec says to fall back to *standard* for that one token.

| Token | Curve | CSS cubic-bezier |
|---|---|---|
| `md.sys.motion.easing.emphasized` | Emphasized | N/A (path interpolator; **fall back to standard** in CSS/iOS) |
| `md.sys.motion.easing.emphasized.decelerate` | Emphasized decelerate | `cubic-bezier(0.05, 0.7, 0.1, 1.0)` |
| `md.sys.motion.easing.emphasized.accelerate` | Emphasized accelerate | `cubic-bezier(0.3, 0.0, 0.8, 0.15)` |
| `md.sys.motion.easing.standard` | Standard | `cubic-bezier(0.2, 0.0, 0, 1.0)` |
| `md.sys.motion.easing.standard.decelerate` | Standard decelerate | `cubic-bezier(0, 0, 0, 1)` |
| `md.sys.motion.easing.standard.accelerate` | Standard accelerate | `cubic-bezier(0.3, 0, 1, 1)` |
| `md.sys.motion.easing.linear` | Linear | `cubic-bezier(0, 0, 1, 1)` (from the official DSP repo) |

> The full **emphasized** curve on Android is the two-segment path
> `M 0,0 C 0.05,0 0.133333,0.06 0.166666,0.4 C 0.208333,0.82 0.25,1 1,1`. There
> is no exact CSS single-`cubic-bezier` for it — a common approximation in CSS
> is `cubic-bezier(0.2, 0, 0, 1)` (i.e. reuse standard), which is what the spec
> recommends as the fallback.

## Duration tokens

The spec defines **16** duration tokens in four families. (Our crate currently
defines only 8 and uses wrong values for the long family — see gaps below.)

| Family | Token | Value |
|---|---|---|
| **Short** (small utility transitions) | `md.sys.motion.duration.short1` | 50 ms |
| | `md.sys.motion.duration.short2` | 100 ms |
| | `md.sys.motion.duration.short3` | 150 ms |
| | `md.sys.motion.duration.short4` | 200 ms |
| **Medium** (mid-screen transitions) | `md.sys.motion.duration.medium1` | 250 ms |
| | `md.sys.motion.duration.medium2` | 300 ms |
| | `md.sys.motion.duration.medium3` | 350 ms |
| | `md.sys.motion.duration.medium4` | 400 ms |
| **Long** (large expressive, usually w/ emphasized) | `md.sys.motion.duration.long1` | 450 ms |
| | `md.sys.motion.duration.long2` | 500 ms |
| | `md.sys.motion.duration.long3` | 550 ms |
| | `md.sys.motion.duration.long4` | 600 ms |
| **Extra-long** (ambient, no user input) | `md.sys.motion.duration.extra-long1` | 700 ms |
| | `md.sys.motion.duration.extra-long2` | 800 ms |
| | `md.sys.motion.duration.extra-long3` | 900 ms |
| | `md.sys.motion.duration.extra-long4` | 1000 ms |

Usage hints from the spec: selection controls 200 ms + standard; a FAB
expanding into a sheet 400 ms + emphasized; a card expanding full-screen 500 ms
+ emphasized; an ambient carousel auto-advance 1000 ms + emphasized.

## Springs

The token-specs page exposes only **easing + duration** tokens (the M3 web spec
does not publish numeric spring stiffness/damping token values; springs are a
platform-implementation concern in Compose/SwiftUI motion guidance). No concrete
spring token values are available to mirror in the crate at this time.

---

## → aphrody m3-tokens (motion.rs gap analysis)

Our crate ([`motion.rs`](../../crates/m3-tokens/src/motion.rs)) exposes `Easing`
(name + cubic-bezier string) and `Duration` (name + ms) with `ALL_EASINGS` /
`ALL_DURATIONS` arrays.

### Easing — status

| Spec token | Crate const | Status |
|---|---|---|
| emphasized | `EASING_EMPHASIZED` = `cubic-bezier(0.2, 0.0, 0, 1.0)` | ✅ present (matches the spec's recommended *standard* fallback; spec lists CSS as N/A) |
| emphasized.decelerate | `EASING_EMPHASIZED_DECELERATE` = `(0.05, 0.7, 0.1, 1.0)` | ✅ correct |
| emphasized.accelerate | `EASING_EMPHASIZED_ACCELERATE` = `(0.3, 0.0, 0.8, 0.15)` | ✅ correct |
| standard | `EASING_STANDARD` = `(0.2, 0.0, 0, 1.0)` | ✅ correct |
| standard.decelerate | `EASING_STANDARD_DECELERATE` = `(0, 0, 0, 1)` | ✅ correct |
| standard.accelerate | `EASING_STANDARD_ACCELERATE` = `(0.3, 0, 1, 1)` | ✅ correct |
| **linear** | — | ❌ **MISSING** — `cubic-bezier(0, 0, 1, 1)` (present in the official DSP repo) |

All 6 of the M3 named easings (emphasized ×3, standard ×3) are present and
correct. Only the auxiliary `linear` curve is absent.

### Duration — status (5 missing tokens + 2 wrong values)

The crate defines 8 durations; the spec defines 16. Crate `long1`/`long2` also
use the **wrong values** (350/400) — those are actually the spec's
`medium3`/`medium4`. Correct mapping:

| Spec token | Spec value | Crate const | Status |
|---|---|---|---|
| short1 | 50 ms | `DURATION_SHORT1` = 50 | ✅ |
| short2 | 100 ms | `DURATION_SHORT2` = 100 | ✅ |
| short3 | 150 ms | `DURATION_SHORT3` = 150 | ✅ |
| short4 | 200 ms | `DURATION_SHORT4` = 200 | ✅ |
| medium1 | 250 ms | `DURATION_MEDIUM1` = 250 | ✅ |
| medium2 | 300 ms | `DURATION_MEDIUM2` = 300 | ✅ |
| medium3 | 350 ms | *(crate calls 350 `LONG1`)* | ⚠️ **mislabeled** — should be `medium3` |
| medium4 | 400 ms | *(crate calls 400 `LONG2`)* | ⚠️ **mislabeled** — should be `medium4` |
| long1 | **450 ms** | `DURATION_LONG1` = **350** | ❌ **WRONG VALUE** (should be 450) |
| long2 | **500 ms** | `DURATION_LONG2` = **400** | ❌ **WRONG VALUE** (should be 500) |
| long3 | 550 ms | — | ❌ **MISSING** |
| long4 | 600 ms | — | ❌ **MISSING** |
| extra-long1 | 700 ms | — | ❌ **MISSING** |
| extra-long2 | 800 ms | — | ❌ **MISSING** |
| extra-long3 | 900 ms | — | ❌ **MISSING** |
| extra-long4 | 1000 ms | — | ❌ **MISSING** |

The crate's `PRIMARY_DURATIONS_MS = [50, 150, 200, 250, 300, 400]` and the
`long2_is_400ms` test are based on the old/incorrect long mapping and will need
updating alongside any fix.

## Source provenance

- Motion overview: <https://m3.material.io/styles/motion/overview> — fetched
  2026-05-21 (SPA nav only; tokens come from the specs page below).
- Easing + duration token values:
  <https://m3.material.io/styles/motion/easing-and-duration/tokens-specs> —
  fetched 2026-05-21.
- `linear` easing + control-point CSS form:
  <https://github.com/material-foundation/material-tokens> `css/motion.css`.
- Prose paraphrased; numeric token values quoted from the spec tables.
