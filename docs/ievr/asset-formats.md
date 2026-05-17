<!-- SPDX-License-Identifier: Apache-2.0 -->

# Asset Formats — Quick Reference

Post-CPK-extraction (see `cpk-extraction-workflow.md`), thousands of files have
opaque extensions. This is the lookup card: magic → format → tool.

## 1. Magic byte lookup table

| Magic (hex)        | ASCII          | Format        | Category     | Tool                          |
|--------------------|----------------|---------------|--------------|-------------------------------|
| `43 50 4B 20`      | `CPK `         | CPK           | archive      | QuickBMS + `cpk.bms`          |
| `43 52 49 44`      | `CRID`         | USM           | video        | vgmstream / CRI Demux         |
| `48 43 41 00`      | `HCA\0`        | HCA           | audio        | vgmstream                     |
| `41 46 53 32`      | `AFS2`         | AWB / AFS2    | archive      | vgmstream / QuickBMS          |
| `80 00 ?? ??`      | (binary)       | ADX           | audio        | vgmstream                     |
| `44 44 53 20`      | `DDS `         | DDS texture   | image        | nvtt / Compressonator         |
| `89 50 4E 47`      | `\x89PNG`      | PNG           | image        | standard image tools          |
| `4F 67 67 53`      | `OggS`         | OGG container | audio        | vorbis-tools                  |
| `52 49 46 46`      | `RIFF`         | WAV/AVI/WebP  | various      | ffmpeg                        |
| `52 41 52 21`      | `RAR!`         | RAR archive   | archive      | 7zip / unrar                  |
| `50 4B 03 04`      | `PK\x03\x04`   | ZIP           | archive      | unzip / 7zip                  |
| `78 9C` / `78 DA`  | (binary)       | zlib stream   | compression  | `python -c "zlib.decompress"` |
| `04 22 4D 18`      | (LZ4 frame)    | LZ4 frame     | compression  | `lz4 -d`                      |
| `28 B5 2F FD`      | (zstd)         | zstd          | compression  | `zstd -d`                     |
| `1B 4C 75 61`      | `\x1BLua`      | Lua bytecode  | script       | luadec / unluac               |
| `FD 37 7A 58 5A`   | `\xFD7zXZ`     | XZ stream     | compression  | `xz -d`                       |
| `42 4D`            | `BM`           | BMP image     | image        | standard image tools          |

## 2. By category — extensions

- **Archives**: `.cpk` (CRI), `.pak` (Unreal), `.bundle` / `.assets` (Unity),
  `.afs` / `.awb` (CRI audio bank).
- **Video**: `.usm` (CRI Sofdec2), `.bik` / `.bk2` (Bink), `.webm`, `.mp4`.
- **Audio**: `.hca` / `.adx` / `.aax` (CRI), `.wem` (Wwise), `.ogg`, `.wav`,
  `.fsb` (FMOD bank).
- **Texture**: `.dds`, `.ktx2`, `.png`, `.tga`, `.bntx` (Switch).
- **Model**: `.fbx`, `.glb`, `.obj`, `.usk` (Unreal skeletal).
- **Animation**: `.anm`, `.bvh`, `.usa` (Unreal anim seq).
- **Script**: `.lua` / `.lub` (Lua), `.cs`, `.bp` (Blueprint compiled).
- **Localization**: `.locres` (Unreal), `.po` / `.mo` (gettext), `.csv` /
  `.json` (custom).
- **Save**: game-specific — `.sav`, `.dat`, `.bin`.

## 3. CRI Middleware family (priority for IEVR)

- CPK → AWB → HCA / ADX (audio chain).
- CPK → ADX (direct embedding, less common).
- CPK → USM (video chain, demux yields HCA audio + Sofdec2 video).
- CPK → custom Level-5 `.bin` / `.dat` (game-specific inner payloads).

## 4. Likely IEVR custom formats

Per `level5-engine-notes.md`, after CPK extraction expect:

- `.bin` / `.dat` — generic Level-5 binary assets (often `@UTF`-table
  look-alikes or proprietary).
- Possibly `.lub` — Lua bytecode if scripting is Lua (common in recent
  Level-5 titles).
- Possibly `.zib` — Level-5 historical archive (3DS-era regression).

## 5. Detection workflow

```bash
file <unknown.ext>                 # OS-level type guess via libmagic
xxd <unknown.ext> | head -2        # first 32 bytes — match against table above
ent <unknown.ext>                  # entropy: ~8.0 bits/byte ⇒ encrypted/compressed
strings -n 8 <unknown.ext> | head  # ASCII fragments leak format hints
```

## 6. When the magic does not match

- **Encrypted** — try common XOR / AES keys from `cri-known-keys.md`.
- **Level-5 proprietary** — log signature + sample path in
  `static-analysis-log.md`, schedule RE.
- **Raw numeric data** — mesh vertices, anim curves. Hex view for periodic
  4-byte (`float32`) or 12-byte (`vec3`) patterns.
- **Wrapper layer** — `@UTF` prefix is common across CRI artefacts; inner
  payload still needs detection.

## 7. Tools quick-list (per category)

- **General**: `xxd`, `hexdump`, `ent`, `strings`, `file`, `binwalk`.
- **Archive**: 7zip, QuickBMS, Rust crate `ievr-fmt` (planned).
- **Image**: GIMP, ImageMagick, `nvtt_export`, Compressonator CLI.
- **Audio**: vgmstream (CLI + foobar2000 plugin), Audacity.
- **Video**: ffmpeg, vgmstream (USM audio track), MPV (Bink).
- **Mesh**: Blender with format addons, Noesis (Windows-only).

## 8. Related docs

- `cpk-format.md` / `usm-format.md` / `adx-hca-format.md` — container detail.
- `cri-toolchain.md` — upstream tool inventory.
- `cpk-extraction-workflow.md` — end-to-end pipeline.
- `static-analysis-log.md` — RE notes log.
