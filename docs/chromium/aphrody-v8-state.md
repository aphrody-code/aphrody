<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody V8 build state (this machine)

Snapshot 2026-05-22. Records the exact, reproducible state of the Windows V8
build so it can be resumed without re-deriving the toolchain dance.

## Layout

| Path | What |
|------|------|
| `C:\src\depot_tools` | depot_tools clone (gclient/fetch/gn/ninja/autoninja) |
| `C:\src\v8` | V8 checkout (`fetch --no-history v8` from `C:\src`) |
| `C:\src\v8\out\x64.release` | release monolith build dir |
| `C:\src\.git_cache` | `GIT_CACHE_PATH` shared object cache |
| `C:\src\v8-gngen.log` / `C:\src\v8-ninja.log` | last gn/ninja logs |

> depot_tools and the V8 checkout live **outside** the aphrody repo (in
> `C:\src\` root), so nothing here is committed into aphrody.

## Environment (must be set for every gn/ninja invocation)

```powershell
$env:Path = "C:\src\depot_tools;" + $env:Path
$env:DEPOT_TOOLS_WIN_TOOLCHAIN = "0"
$env:vs2026_install          = "C:\Program Files\Microsoft Visual Studio\18\Insiders"
$env:GYP_MSVS_OVERRIDE_PATH  = "C:\Program Files\Microsoft Visual Studio\18\Insiders"
$env:GYP_MSVS_VERSION        = "2026"
```

Toolchain: **VS 2026 Insiders** (`...\18\Insiders`), MSVC 19.5x, Windows SDK
10.0.26100. `DEPOT_TOOLS_WIN_TOOLCHAIN=0` forces the locally-installed VS
(external/non-Googler build).

## `args.gn` (out\x64.release)

```gn
is_debug = false
target_cpu = "x64"
v8_monolithic = true
v8_use_external_startup_data = false
is_component_build = false
v8_enable_sandbox = false
use_custom_libcxx = false
```

## Gotchas hit & fixed

1. **`vs2022_install` ignored by VS 2026** → `gn gen` exited without writing
   `build.ninja`. Fix: use `vs2026_install` + `GYP_MSVS_OVERRIDE_PATH` (path is
   the non-standard Insiders `...\18\Insiders`).
2. **`cmd /c "gn.bat gen out\\x64.release 2>&1"` from bash** → quoting broke, cmd
   only printed its banner. Fix: drive gn/ninja from **PowerShell**
   (`& cmd /c gn.bat gen out\x64.release`), not nested bash→cmd quoting.
3. **`v8_enable_sandbox = true` + `use_custom_libcxx = false`** → GN assertion
   `BUILD.gn:814` "sandbox requires libc++ hardening". Fix: sandbox **off** for
   the MSVC-STL embed build.
4. **Clang `-Werror,-Wctad-maybe-unsupported`** on `std::atomic_ref` in
   `src/heap/cppgc/heap-object-header.h:357` (V8's bundled clang + newer libc++)
   → 3 compile units failed at ~922/2410. Fix: add
   `treat_warnings_as_errors = false` to `args.gn` (warnings stay, build
   proceeds).

## Status

- depot_tools cloned ✅
- `fetch v8` + `gclient sync` complete ✅
- `gn gen out\x64.release` → 825 targets ✅
- `autoninja v8_monolith` → **blocked** at ~930/2410 on a Torque ABI assertion
  `static_assert(kSize == sizeof(ExtendedMap))` in
  `gen/torque-generated/src/objects/map-tq.cc:102`. Root cause: turning the
  **sandbox off while leaving pointer-compression at its default** desyncs the
  Torque-generated object layout from the runtime `Map` size.

### Decision (do not chase the hand-build)

The Rust↔V8 surface uses the **prebuilt `v8` crate** (see
[`rust-bindings.md`](rust-bindings.md)) — `rusty_v8` ships a known-good
Windows-MSVC monolith whose sandbox/pointer-compression flags are internally
consistent. So this from-source monolith is **only** an offline/patch fallback;
it is not on the critical path and is left blocked rather than ABI-tuned.

If the hand-build is ever needed, the fix is to use a self-consistent flag set
(either keep `v8_enable_sandbox = true` with `use_custom_libcxx = true`, or also
set `v8_enable_pointer_compression = false`) — i.e. mirror the exact `GN_ARGS`
rusty_v8 uses for its release archive, rather than mixing sandbox-off with
default pointer compression.

## Resume command

```powershell
# (set env block above first)
Set-Location C:\src\v8
cmd /c autoninja.bat -C out\x64.release v8_monolith
# output: out\x64.release\obj\v8_monolith.lib
```
