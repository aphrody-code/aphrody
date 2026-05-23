<!-- SPDX-License-Identifier: Apache-2.0 -->
# Building V8 — GN + ninja

Source: <https://v8.dev/docs/build> · <https://v8.dev/docs/build-gn>
(fetched 2026-05-22, distilled)

## Option A — `gm.py` (one-shot helper)

`tools/dev/gm.py` chains `gn gen` + `ninja` (+ optional tests) and **prints
every command it runs**, so you can copy them for manual re-use.

```bat
python tools\dev\gm.py x64.release            REM compile only
python tools\dev\gm.py x64.release.check       REM compile + run the test suite
python tools\dev\gm.py x64.debug mjsunit/foo   REM debug + targeted tests
```

## Option B — manual GN (what aphrody uses)

Manual control is preferred here because the VS 2026 Insiders toolchain needs
explicit env vars (see [`windows-build.md`](windows-build.md)) and we want a
**monolithic, MSVC-STL, sandbox-off** embed library.

1. Write `out\x64.release\args.gn`:

   ```gn
   is_debug = false
   target_cpu = "x64"
   v8_monolithic = true
   v8_use_external_startup_data = false
   is_component_build = false
   v8_enable_sandbox = false      # required when use_custom_libcxx = false
   use_custom_libcxx = false      # link MSVC STL → embeddable in non-Chromium apps
   ```

2. Generate ninja files (with the VS 2026 env set):

   ```bat
   gn gen out\x64.release
   gn args out\x64.release --list    REM inspect every available arg
   ```

3. Build the target:

   ```bat
   ninja -C out\x64.release v8_monolith    REM the static embed lib
   ninja -C out\x64.release d8             REM the standalone JS shell
   autoninja -C out\x64.release v8_monolith REM autoninja = ninja + auto -j
   ```

## Key GN args

| Arg | Meaning |
|-----|---------|
| `is_debug` | `false` = release (optimised, no DCHECKs) |
| `target_cpu` / `v8_target_cpu` | host arch / cross-compile target (`"x64"`, `"arm64"`) |
| `v8_monolithic` | bundle everything into a single `v8_monolith` static lib |
| `v8_use_external_startup_data` | `false` = embed the snapshot in the binary (no `.bin` files) |
| `is_component_build` | `false` = static (one lib), `true` = many DLLs (faster incremental) |
| `v8_enable_sandbox` | pointer-compression sandbox; **requires `use_safe_libcxx`** |
| `use_custom_libcxx` | `true` = Chromium hardened libc++; `false` = platform STL (MSVC) |

> **Constraint proven on this machine:** `v8_enable_sandbox = true` asserts
> `use_safe_libcxx`, which conflicts with `use_custom_libcxx = false`. For an
> MSVC-STL embed build, set the sandbox **off**.

## Targets

- `v8_monolith` — static lib for embedders (pairs with `v8_libbase`,
  `v8_libplatform` when not monolithic).
- `d8` — the V8 developer shell / REPL.
- Tests via `tools\run-tests.py --gn` or the `.check` gm suffix.
