<!-- SPDX-License-Identifier: Apache-2.0 -->
# Windows build prerequisites

Source: <https://chromium.googlesource.com/chromium/src/+/main/docs/windows_build_instructions.md>
(fetched 2026-05-22, distilled)

## System requirements

- x86-64, **≥ 8 GB RAM** (16 GB+ recommended), **≥ 100 GB** free on an **NTFS**
  drive (FAT32 unsupported).
- Windows 10 or newer.
- **Visual Studio 2026** (build ≥ 17.0.0 per the doc; this machine runs the
  **2026 Insiders** channel = v18.x, MSVC 19.5x).
- **Windows SDK 10.0.26100.7705**, with **SDK Debugging Tools 10.0.26100.3323+**.

## Visual Studio components

Install the Native Desktop workload + ATL/MFC:

```bat
vs_installer.exe ^
  --add Microsoft.VisualStudio.Workload.NativeDesktop ^
  --add Microsoft.VisualStudio.Component.VC.ATLMFC ^
  --includeRecommended
```

ARM64 cross builds additionally need `VC.Tools.ARM64` + `VC.MFC.ARM64`.
The **Debugging Tools for Windows** component is required (gn looks for it).

## Git config (Windows)

```bat
git config --global core.autocrlf false
git config --global core.filemode false
git config --global core.preloadindex true
git config --global core.fscache true
git config --global core.longpaths true
```

## The two critical environment variables

| Var | Value | Why |
|-----|-------|-----|
| `DEPOT_TOOLS_WIN_TOOLCHAIN` | `0` | **External (non-Googler) builds**: use the locally-installed VS instead of Google's internal hermetic toolchain package. |
| `vs2026_install` | `C:\Program Files\Microsoft Visual Studio\2026\<Edition>` | Tells `build/vs_toolchain.py` where VS lives. **Year-specific** — `vs2022_install` is silently ignored by a 2026 VS, which makes `gn gen` fail to find the toolchain. |

For the **Insiders** channel the path is non-standard
(`...\Visual Studio\18\Insiders`), so also set the path-based override which is
version-agnostic:

```bat
set GYP_MSVS_OVERRIDE_PATH=C:\Program Files\Microsoft Visual Studio\18\Insiders
set GYP_MSVS_VERSION=2026
```

> Gotcha proven on this machine: setting only `vs2022_install` made `gn gen`
> exit without writing `build.ninja`. Switching to `vs2026_install` +
> `GYP_MSVS_OVERRIDE_PATH` fixed toolchain detection. See
> [`aphrody-v8-state.md`](aphrody-v8-state.md).

## Build (Chromium proper, for reference)

```bat
gn gen out\Default
autoninja -C out\Default chrome
```

For V8 specifically, see [`v8-build.md`](v8-build.md).
