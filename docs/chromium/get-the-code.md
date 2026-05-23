<!-- SPDX-License-Identifier: Apache-2.0 -->
# Getting the code — depot_tools + `fetch v8`

Source: <https://v8.dev/docs/source-code> · <https://chromium.googlesource.com/chromium/src/+/main/docs/get_the_code.md>
(fetched 2026-05-22, distilled)

## 1. depot_tools

depot_tools bundles `gclient`, `fetch`, `gn`, `ninja`, `autoninja`, and a
managed Git. Clone it (no official GitHub repo exists — third-party mirrors are
not authoritative):

```bat
cd C:\src
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git
```

Put `C:\src\depot_tools` at the **front** of `PATH` so its bundled tools win.
Then run `gclient` once with no args to self-bootstrap (downloads Python, Git,
ninja, etc.).

## 2. Fetch V8

Never `git clone` the V8 repo directly — `fetch` also pulls the `DEPS`-managed
dependencies. The checkout lands in a detached HEAD.

```bat
mkdir C:\src\v8src & cd C:\src\v8src
fetch v8
cd v8
```

aphrody's checkout lives at `C:\src\v8` (the `fetch` was run from `C:\src`).

## 3. Fast fetch / sync accelerators (used here)

The full checkout is ~22 GB+. Levers that actually cut wall-time:

| Lever | Effect |
|-------|--------|
| `fetch --no-history v8` | shallow-ish fetch, ~2× less transfer |
| `gclient sync --no-history --jobs N` | parallel dependency sync (N = ~2× cores) |
| `GIT_CACHE_PATH=C:\src\.git_cache` | shared object cache / reference clones across re-syncs |
| `git config --global protocol.version 2` | smarter ref negotiation |
| `git config --global fetch.parallel 0` | parallel sub-fetches (0 = auto) |
| `git config --global core.fscache true` + `core.preloadindex true` | Windows FS speedups |
| `git config --global core.longpaths true` | avoids `MAX_PATH` failures in deep deps |

## 4. Keep updated

```bat
git pull
gclient sync
```

`gclient sync` reconciles `DEPS` after any pull; `-D` additionally prunes
removed dependencies.
