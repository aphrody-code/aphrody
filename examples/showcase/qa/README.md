<!-- SPDX-License-Identifier: Apache-2.0 -->

# Gemini AI Mode — visual QA

Real-browser visual-QA harness for the showcase "Gemini AI Mode" demo
(`src/components/GeminiAiMode.tsx`, the live transposition of the Google Search
/ AI Mode grammar from `docs/design/google/ANALYSE.md` onto real M3 components
and `--md-sys-color-*` tokens).

## What it does

`src/qa-gemini.ts` drives **real system Chromium** through
[`@aphrody-code/bxc`](https://github.com/aphrody-code) and asserts the rendered
surface. It:

1. Serves a dedicated entry — `src/qa-gemini.html` -> `src/qa-gemini-entry.tsx`
   — that mounts the **real** `<GeminiAiMode>` (real `md-*` web components, real
   tokens) via `Bun.serve`, on an ephemeral port. The 3D WebGL backdrop
   (`<ThreeWorld>`) is omitted because its `@react-three/fiber` build is
   incompatible with this React version (it throws on the removed
   `ReactCurrentOwner` internal before paint) — unrelated to this demo.
2. Opens **real Chrome** via `Browser.newPage({ profile: "stealth" })`. That
   profile makes bxc's `WebSocketTransport` launch the binary in
   `BXC_CHROME_BIN` / `CHROME_BIN` / `CHROME_PATH` with
   `--remote-debugging-port` and connect over CDP. Lightpanda is **not** used —
   it lacks `ElementInternals`, which every `md-*` component depends on.
3. Asserts: the `.gemini*` selectors exist; the `md-assist-chip` **upgraded**
   (its `shadowRoot` is non-null — this is the proof we are on real Chrome, not
   Lightpanda); `--gemini-sparkle` is non-empty and carries the Gemini brand
   stops `#4285f4 -> #9b72cb -> #d96570`; representative `--md-sys-color-*` roles
   are applied; and there are **zero page console errors** during load.
4. Captures the section in **light** and **dark** (`<html data-theme>`):
   `gemini-light.png` + `gemini-dark.png` (clean section clip via CDP).
5. Uses the bxc **`/google`** module: `checkGoogleStyle` +
   `GOOGLE_TS_STYLE_RULES` (a TypeScript _style_ linter — it correctly flags
   this repo's tab indentation; informational only), and `parseSerp` to
   structurally parse the rendered SERP markup against the Google SERP grammar
   (the demo uses M3 class names rather than Google's obfuscated classes, so
   `organic=0` is expected). `parseSerp` needs the bxc rust-bridge cdylib; when
   it is not built on the host the step is skipped gracefully.
6. Prints `GEMINI QA: PASS` / `FAIL …` and exits accordingly.

## Run

`@aphrody-code/bxc` is an **optional dependency** (published on GitHub Packages,
which requires npm auth to install). It is deliberately NOT a hard dependency so
a tokenless `bun install` of the monorepo never fails, and `bun run typecheck`
(the gate) stays green without it — `qa-gemini.ts` is typechecked separately via
`tsconfig.qa.json`. Install bxc first, then run:

```bash
cd examples/showcase
bun add -O @aphrody-code/bxc            # needs GitHub Packages auth (~/.npmrc)
bunx tsc -p tsconfig.qa.json --noEmit   # optional: typecheck the harness
CHROME_BIN=/usr/local/bin/google-chrome bun run qa
# also honoured: BXC_CHROME_BIN, CHROME_PATH
# optional: BXC_RUST_BRIDGE_LIB=/path/to/libbxc_rust_bridge.so (enables parseSerp)
```

## Screenshots

`gemini-light.png` and `gemini-dark.png` are **committed** as small (~90 KB
each, 1310x523) visual baselines so the rendered surface can be reviewed in PRs
without running Chrome. They are overwritten on every `bun run qa`.
