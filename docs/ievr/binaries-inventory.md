<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody-code
-->

# IEVR binaries inventory — 2026-05-17

Initial read-only inventory of the *Inazuma Eleven: Victory Road* Steam install,
captured for reverse-engineering / interop work on a legally purchased copy.
No file in the install dir was modified.

## Install location

| Field                | Value                                                                                       |
|----------------------|---------------------------------------------------------------------------------------------|
| Steam library        | `C:\Program Files (x86)\Steam\` (only library declared in `libraryfolders.vdf`)             |
| Install dir          | `C:\Program Files (x86)\Steam\steamapps\common\INAZUMA ELEVEN Victory Road\`                |
| App ID               | `2799860`                                                                                   |
| Depot ID             | `2799861` (manifest `1147708054852059036`)                                                  |
| Build ID             | `22792000`                                                                                  |
| Reported size on disk| 60 355 047 097 bytes (~56.21 GiB / ~57 559.06 MiB)                                          |
| Bytes downloaded     | 60 824 548 368 (matches BytesToDownload — 100 % complete)                                   |
| Bytes staged         | 60 857 694 225                                                                              |
| File count           | **975**                                                                                     |
| Last updated         | 2026-05-17 15:32:30 UTC (epoch `1779031950`)                                                |
| User language        | French (`UserConfig`/`MountedConfig` = `french`)                                            |
| User data path       | `<USERHOME>\AppData\LocalLow\LEVEL5 Inc_\<install dir name>\` (per `remove_local_files.ps1`, not yet created — game not launched) |

All Steamworks redistributables come from the shared depots `228980` / `228989`
/ `228990` / `229000` / `229007` (referenced under `SharedDepots` in
`appmanifest_2799860.acf`).

## Engine + framework signals

Identified directly by string-scanning `nie.exe` (30.01 MiB, PE64, 9 sections,
linker timestamp `2026-04-15 03:31:56 UTC`):

| Component                   | Evidence                                                                       |
|-----------------------------|--------------------------------------------------------------------------------|
| Custom Level-5 engine "nie" | Main executable is `nie.exe`; namespaces `game::BASARA_BUILD_INFO::lives`; user data path published as `LEVEL5 Inc_` |
| Renderer                    | Direct3D 11 only (`D3D11CreateDevice` import; no D3D12 / Vulkan / OpenGL / Metal / WebGPU strings) |
| Audio middleware            | **CRIWARE / CRI ADX2** — hundreds of strings prefixed `CRIWARE/` (AmplitudeAnalyzer, BusBufferPool, Bandpass, Biquad, Delay, …); confirms `.cpk` (CRIWARE Pack) and `.usm` (CRIWARE USM movie) file types |
| Audio (additional)          | `fmodf` / `<fmod` — uncertain whether full FMOD Studio is linked or just libm `fmod()`; no `FMOD Studio` / `fmodstudio` symbols found |
| Physics                     | **NVIDIA PhysX** (`PxScene::setFilterShaderData`, `FilterShaderDataSize`) |
| Scripting                   | **Lua 5.2** embedded (`LUA_PATH_5_2`, `LUA_NOENV`, `luaopen_%s`, `lua/?.lua;lua/?/init.lua;…`) |
| Compression                 | `lz4` referenced; `libcurl` for HTTP transport (also shipped as `libcurl.dll`) |
| ICU                         | Some `icu` symbols (incidental — could be Unicode collation only)              |
| Shader pipeline             | Internal compiled-effects format `*.cfxo` (paths like `#/shader/<SHADER_VERSION>/cs_cubemap_copy_texture2.cfxo`) |
| Network                     | `NIE_Socket_0.9.0` — proprietary socket wrapper                                |
| Online services             | Epic Online Services SDK 1.x (`EOSSDK-Win64-Shipping.dll`, `EOS_Platform_GetAntiCheatClientInterface`, `EOS_Platform_GetP2PInterface`, `EOS_ProductUserId_IsValid`); EOS product id `da518e53730f4be6acbac5ebf75745e0` |
| Steamworks                  | `SteamAPI_Init/Shutdown/RegisterCallback/RegisterCallResult/GetHSteamUser`; `steam_api64.dll`, `sdkencryptedappticket64.dll` |

**No Unreal Engine, Unity, Cocos, Godot, CryEngine or Sony Phyre markers were
found.** The build is a proprietary Level-5 in-house engine (referred to here
as the "nie" engine after the executable name).

### Anti-tamper / DRM stack

| Layer                                    | Files                                                                  |
|------------------------------------------|------------------------------------------------------------------------|
| Bootstrapper                             | `GameBootstrapper.exe` → launches `EACLauncher.exe` (per `GameBootstrapper.ini`) |
| Easy Anti-Cheat (EAC, EOS variant)       | `EasyAntiCheat\` dir (5.00.24.00), `EACLauncher.exe`, `base.bin`, `base.cer`, `runtime.conf`, `EasyAntiCheat_EOS_Setup.exe` installer |
| Epic Online Services SDK + installer     | `EpicOnlineServices\EpicOnlineServicesInstaller.exe` (126.06 MiB) |
| Asset envelope encryption                | All 921 `.cpk` files have unique random first-4 bytes (no `CPK ` magic preserved). Catalog likely in `data\cpk_list.cfg.bin` (12.18 MiB, no plaintext header). |
| Steam encrypted app ticket               | `sdkencryptedappticket64.dll`                                          |

No Denuvo, no BattlEye, no Arxan / nProtect markers observed in the
executable strings.

### Platform residues

`steam_input_*.vdf` controller manifests are shipped for: Steam Deck
(`neptune`), PS4, PS5, Steam Controller (`gordon`), Switch Joy-Con pair,
Switch Pro, Xbox 360, Xbox One. No console-specific binaries
(no `.nso` / `.nca` / `.pkg` / `.elf`).

## Binaries by extension

Full numeric summary (recursive walk of the install dir):

| Count | Extension | Total MiB    |
|-------|-----------|--------------|
| 921   | `.cpk`    | 57 354.95    |
| 20    | `.cfg`    | 0.16         |
| 10    | `.vdf`    | 0.07         |
| 5     | `.exe`    | 163.28       |
| 4     | `.dll`    | 19.96        |
| 3     | `.txt`    | 0.02         |
| 3     | `.bin`    | 12.91        |
| 2     | `.usm`    | 7.26         |
| 1     | `.ps1`    | < 0.01       |
| 1     | `.bat`    | < 0.01       |
| 1     | `.conf`   | < 0.01       |
| 1     | `.json`   | < 0.01       |
| 1     | `.ini`    | < 0.01       |
| 1     | `.cer`    | < 0.01       |
| 1     | `.png`    | 0.44         |

(No files without an extension; no hidden/system files.)

### `.exe` (5 files)

| Path                                                | Size (MiB) | SHA-256 (first 16 hex) |
|-----------------------------------------------------|-----------:|------------------------|
| `EACLauncher.exe`                                   |      3.792 | `903e7aba292448d6`     |
| `EasyAntiCheat\EasyAntiCheat_EOS_Setup.exe`         |      0.915 | `f6a52adbb75c0155`     |
| `EpicOnlineServices\EpicOnlineServicesInstaller.exe`|    126.056 | `eca924456817b149`     |
| `GameBootstrapper.exe`                              |      2.505 | `e804a912cede25c0`     |
| **`nie.exe`** (main game binary, PE64)              |     30.010 | `4c53ea758f7235a5`     |

### `.dll` (4 files)

| Path                              | Size (MiB) | SHA-256 (first 16 hex) |
|-----------------------------------|-----------:|------------------------|
| `EOSSDK-Win64-Shipping.dll`       |     18.154 | `0eec2f7f47c4a991`     |
| `libcurl.dll`                     |      0.519 | `14fb7a87ed1d5d62`     |
| `sdkencryptedappticket64.dll`     |      0.978 | `e300ec5111d6d7c8`     |
| `steam_api64.dll`                 |      0.305 | `e082bf5c9f881c82`     |

### `.bin` (3 files)

| Path                                                  | Size (MiB) | SHA-256 (first 16 hex) |
|-------------------------------------------------------|-----------:|------------------------|
| `data\common\system\app_config_5.00.24.00.cfg.bin`    |      0.022 | `1e7eeca649b0548a`     |
| `data\cpk_list.cfg.bin`                               |     12.182 | `e1819efa6b4741f1`     |
| `EasyAntiCheat\Certificates\base.bin`                 |      0.708 | `a6ee06e7dc0cd378`     |

The two `.cfg.bin` files have no recognisable plaintext header (random first
bytes) → likely the same scrambling envelope as the `.cpk` archives.
`base.bin` is the EAC runtime module (binary), pinned by `base.cer`.

### `.usm` (2 files — CRIWARE movies, encrypted/scrambled)

| Path                                  | Size (MiB) | SHA-256 (first 16 hex) |
|---------------------------------------|-----------:|------------------------|
| `data\dx11\movie\IE_15th.usm`         |      4.138 | `ec12758a55835bdb`     |
| `data\dx11\movie\L5logo.usm`          |      3.126 | `7c6eb9c6010ae093`     |

A standard CRIWARE USM begins with `CRID` (`0x43 52 49 44`). These two start
with random bytes instead, matching the `.cpk` scrambling pattern.

### `.cpk` (921 files, 57 354.95 MiB ≈ 56.011 GiB)

All asset packs live in `data\packs\<32-hex>.cpk`. File names are 32-char
lowercase hex (likely the result of an internal path hash, e.g. MD5 of the
logical asset path). **No plaintext CRIWARE `CPK ` magic on any of the 921
files** — each first 4 bytes are unique (921 distinct headers for 921 files),
which indicates an envelope obfuscation: either AES/XOR with a per-entry IV /
key derived from the asset hash, or whole-file encryption with deterministic
nonces. The mapping `hash → real path` is almost certainly in
`data\cpk_list.cfg.bin`.

CPK size distribution:

| Bucket          | File count |
|-----------------|-----------:|
| < 10 KiB        |          6 |
| 10 KiB – 100 KiB|        136 |
| 100 KiB – 1 MiB |        143 |
| 1 – 10 MiB      |        247 |
| 10 – 100 MiB    |        277 |
| 100 MiB – 1 GiB |        106 |
| ≥ 1 GiB         |          6 |

Min: 8.14 KiB · Max: 4 366.31 MiB · Avg: 62.27 MiB.

Top 20 largest `.cpk` (these dominate the install footprint — 5 alone
account for ~15.2 GiB):

| File (basename, `.cpk` omitted)                | Size (MiB) |
|------------------------------------------------|-----------:|
| `764a613d3a9601ddf1da2e9bd742d0a9`              |   4 366.31 |
| `7332f90b479b3b2a50add663c7926a5c`              |   3 827.93 |
| `9f44bbcbf647e12ee4c3b84e62dd8209`              |   2 679.48 |
| `1f726268b7dc51d01af16d449959523a`              |   2 647.80 |
| `fc62c6effe5a8c4748ab9cb3721c1017`              |   1 678.22 |
| `4c5e1ac99f3aac0ad0701ec49cda3ae3`              |   1 285.82 |
| `854911f974c7349307766d958cae5899`              |     938.74 |
| `8270b06812644c530829ca93d8660352`              |     829.80 |
| `75ce1f1145cb6c1824ea12ecc84d7090`              |     796.29 |
| `8387d7bd3fefe8e1c46b9c224b7b5d1e`              |     780.53 |
| `aa77ba6f22a1000e26377a4d1747f43e`              |     683.16 |
| `aac838542711da9b74fd7d1d78386bbd`              |     620.70 |
| `73b2510139afd8edc8cac3380cf24b75`              |     591.20 |
| `17fc7699d1899ff3818347130df932bc`              |     588.29 |
| `96fcd9b6c0f42d321719abbf113effaa`              |     587.98 |
| `f8c7155778a351a2ef9d948941fb55f3`              |     563.78 |
| `a731848118da7944b5cd93aa4bc0d222`              |     543.82 |
| `ada59827d6c3760e82e63fa0291c3043`              |     522.23 |
| `592a47f8dc7f141c9e75a1458a96a7d3`              |     464.34 |
| `44e84be947a454b70947d764e191a04a`              |     450.17 |

Per-file SHA-256 of the 921 `.cpk` was **[not captured]** — would cost ~60 GiB
of disk read, deferred until needed for a specific deduplication / integrity
pass.

### `.cer` / `.conf` / `.json` / `.png` (EAC + bootstrap configs)

| Path                                            | Size (MiB) | SHA-256 (first 16 hex) |
|-------------------------------------------------|-----------:|------------------------|
| `EasyAntiCheat\Certificates\base.cer`           |      0.001 | `2675b52d00a2f5d4`     |
| `EasyAntiCheat\Certificates\runtime.conf`       |      0.001 | — (text, binary garbled, looks encrypted) |
| `EasyAntiCheat\Settings.json`                   |    < 0.001 | — (plain JSON, transcribed below) |
| `EasyAntiCheat\SplashScreen.png`                |      0.440 | — (PNG splash screen)  |

`EasyAntiCheat\Settings.json` (plain text):

```json
{
    "title":            "INAZUMA ELEVEN: Victory Road",
    "executable":       "nie.exe",
    "productid":        "da518e53730f4be6acbac5ebf75745e0",
    "sandboxid":        "6eded9b52bc74c84858eb0a82c4d41e7",
    "deploymentid":     "39c5c5a2bf8144d38bbfc1805d99b49d",
    "requested_splash": "EasyAntiCheat/SplashScreen.png",
    "wait_for_game_process_exit": "true",
    "hide_bootstrapper": "false",
    "hide_gui":          "false"
}
```

### `.vdf` (10 Steam configs, plaintext KeyValues, no hashes captured)

`install_script_2799860_rel.vdf`, `steam_action_manifest.vdf`,
`steam_input_for_{neptune,ps4,ps5,steamcontroller_gordon,switch_joycon_pair,switch_pro,xbox360,xboxone}.vdf`.
All sizes ≤ 10 KiB.

### `.cfg` (20 files, all EAC localisation, ~8 KiB each)

`EasyAntiCheat\Localization\<locale>.cfg` for: ar_sa, cs_cz, de_de, en_us,
es_ar, es_es, fr_fr, id_id, it_it, ja_ja, ko_kr, nl_nl, pl_pl, pt_br, ru_ru,
th_th, tr_tr, vi_vn, zh_cn, zh_tw.

### `.ini` / `.txt` / `.ps1` / `.bat` (bootstrap + EAC licenses)

| Path                                            | Size      |
|-------------------------------------------------|-----------|
| `GameBootstrapper.ini`                          | 82 B      |
| `EasyAntiCheat\Licenses\Apache-2.0.txt`         | ~11 KiB   |
| `EasyAntiCheat\Licenses\Licenses.txt`           | ~9 KiB    |
| `EasyAntiCheat\Licenses\MIT.txt`                | ~1 KiB    |
| `uninstall\remove_local_files.ps1`              | 761 B     |
| `uninstall\remove_local_files.bat`              | 181 B     |

`GameBootstrapper.ini`:

```ini
ApplicationPath=EACLauncher.exe
WorkingDirectory=
WaitForExit=0
NoOperation=0
```

`remove_local_files.ps1` confirms the user-data layout:
`<USERHOME>\AppData\LocalLow\LEVEL5 Inc_\<install-dir-name>\` (preserves the
`users\` subfolder).

## Notable files

- **Largest single binary**: `data\packs\764a613d3a9601ddf1da2e9bd742d0a9.cpk`
  (4 366.31 MiB / ~4.27 GiB). Likely the global character/animation/textures
  master pack.
- **Likely main executable**: `nie.exe` (30.01 MiB, x86-64 PE,
  linker timestamp 2026-04-15). Pulls in CRI ADX2, NVIDIA PhysX, Lua 5.2,
  libcurl, Steamworks, EOS SDK, EAC.
- **Likely asset catalog**: `data\cpk_list.cfg.bin` (12.18 MiB) — only
  plausible index for the 921 hashed `.cpk` files.
- **Movies**: only two `.usm` (15th-anniversary intro + Level-5 logo).
- **Largest dependency**: `EOSSDK-Win64-Shipping.dll` (18.15 MiB), then the
  Epic installer (126 MiB but only a one-shot installer, not loaded at runtime).
- **No PDB / no debug symbols** present.
- **No data outside `data\`** for game content — everything packed.

## Next steps for reverse-engineering

1. **Decrypt the CPK envelope.** Implement an offline tool that XORs / decrypts
   the first 32 bytes of a few `.cpk` files against the standard CRIWARE
   `CPK \x00\x00\x00\x00FF FF FF FF…` prefix to recover the per-file key (or
   the constant pattern, if the envelope is e.g. AES-CTR with a deterministic
   nonce derived from the filename hash). The 32-char hex basename is the
   prime candidate for the nonce input.
2. **Parse `cpk_list.cfg.bin`.** Once a CPK can be decrypted, the matching
   plaintext catalog format should reveal logical path strings and per-asset
   metadata (size, offset, content type, possibly a per-asset XOR key).
3. **Hook the CRIWARE loader inside `nie.exe`.** Static analysis (Ghidra /
   IDA) on `nie.exe` around CRI symbol references (`criatomex`, `sof_dec`,
   `crimv`, …) to identify the wrapper that opens a `.cpk` and feeds the
   decryption layer. Cross-reference with `cpk_list.cfg.bin` parsing code.
4. **Catalogue EAC kernel-driver surface.** `EasyAntiCheat\Certificates\base.bin`
   + `runtime.conf` are the bootstrapper-loaded EAC user-mode module + its
   pinning material. Useful only for understanding launch-time control flow;
   do **not** attempt to bypass EAC — it would be both ToS-breaking and
   pointless for an offline interop / asset-extraction effort.
5. **Lua script inventory.** Once at least one `.cpk` is decrypted, search for
   `.lua` / `.luac` payloads — Lua 5.2 is embedded, so gameplay logic is
   likely scripted. These scripts will be the highest-leverage RE target for
   understanding game systems and writing companion tools.
