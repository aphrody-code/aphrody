<!-- SPDX-License-Identifier: Apache-2.0 -->

# CPK (CRI Middleware) — File Format Spec

Notes on the **CPK** container used by IEVR (921 archives, ~60 GB). Target: a Rust parser in `ievr-fmt` (see `PLAN.md` §3) and tooling pick.

## 1. Origin

CPK is the asset-pack container from **CRI Middleware** (株式会社CRI・ミドルウェア, Tokyo). CRI's stack — `CRI File System` (CPK), `Sofdec2` (USM video), `ADX2`/`HCA` (audio) — ships widely in Japanese games: Bandai Namco, Sega, Level-5, Capcom, From Software, Square Enix, Konami. CPK is middleware-neutral, like Unreal's `.pak`. It aggregates loose files (textures, audio, models, scripts, localisation, config) under a seekable archive for streaming reads.

## 2. Container header

- **Magic**: ASCII `CPK ` (three letters plus space, four bytes) — confirmed by public RE work.
- **Header table**: `@UTF` (CRI's row/column descriptor, also used for the ToC), preceded by a four-byte length.
- **Fields**: `UpdateDateTime`, `FileSize`, `ContentOffset`/`Size`, `TocOffset`/`Size`, `EtocOffset`/`Size`, `ItocOffset`/`Size`, `Files`, `Version`, `Revision`, `Align`. Varies across CRI File System versions (CpkMaker SDK v1.x circa 2005 to v2.x).
- **Alignment**: payload aligns to `Align` (commonly 2048 — DVD sector, kept for locality).

## 3. Table of Contents (ToC)

Several optional `@UTF` index tables:

- **TOC** — primary; one row per file. Columns: `DirName`, `FileName`, `FileSize`, `ExtractSize`, `FileOffset`, `ID`, `UserString`. `FileOffset` is relative to `ContentOffset` or absolute, per CPK mode (`FILENAME`, `ID`, `FILENAME_AND_ID`, `FILENAMEONLY`).
- **ITOC** — id-only, for CPKs without filenames; common in titles that strip strings.
- **ETOC** — extended metadata (timestamps, attributes). **GTOC** — group table (rare).
- **Per-file hash**: not always embedded; integrity left to container CRC or distribution layer (Steam).

## 4. Compression

- **Uncompressed** — `FileSize == ExtractSize`.
- **CRILAYLA** — CRI's in-house LZ codec. Signature `CRILAYLA` (8 ASCII bytes), then uncompressed and compressed sizes as little-endian `uint32`. Decoded by `CriPakTools` and `QuickBMS`.
- **LZ4 / LZMA / Deflate** — newer CRI builds expose generic codecs per-file; rare in legacy titles, more common in 2020+ releases. Codec flagged in the ToC.

## 5. Encryption

Some titles encrypt ToC and / or payload with an XOR-mask or AES key embedded in the main executable, derived from a constant string or per-title scramble.

- **Static**: scan `nie.exe` for high-entropy 16 / 32-byte constants near `CPK ` or `@UTF` references; trial-decrypt the ToC.
- **Dynamic (Frida)**: hook `criFs_*` or `criFsBinder_*`; dump the key at first ToC read.
- Many CRI titles ship CPK **unencrypted** — try plain parse first.

## 6. Subformats commonly contained

Inner files keep their own magic:

- **USM** — CRI Sofdec2 video (H.264 / VP9 + ADX / HCA audio).
- **ADX** — CRI legacy ADPCM audio; magic `0x80 0x00` plus `(c)CRI` tail.
- **HCA** — CRI modern audio (`HCA\0`); encrypted variants common.
- **ACB / AWB** — CRI audio cuesheet + bank.
- **Engine assets** — `.uasset` / `.uexp` for Unreal titles, Level-5 in-house `.bin` / `.dat` for IEVR.

## 7. Known tools

- **CriPakTools** — C# extractor / repacker (`Brolijah/CriPakTools` on GitHub; forks add CRILAYLA fixes and newer header variants).
- **QuickBMS** — Aluigi's universal extractor; `cpk.bms` handles common ToC and CRILAYLA. First-line triage.
- **criware-modding** community — Discord toolchain (extractors, ADX / HCA decoders, USM demuxers, key catalogue).
- **FModel** — primarily Unreal `.pak`; CPK support limited.
- **VGMStream** — for inner ADX / HCA / USM audio once extracted.

## 8. IEVR-specific notes

- **Volume**: 921 CPKs, ~60 GB = ~65 MB per archive average — fine-grained, not monolithic.
- **Probable axes**: per-region (JP / global), per-language (EN / JP audio), per-content-area (chapter, system, UI), per-character. Confirm by name-prefix clustering.
- **Engine**: Level-5 shipped IEVR on PC in 2026; lineage from prior IE titles, consistent with Level-5's CRI pipeline.
- **Encryption**: medium likelihood. No-key parse first; escalate only if `@UTF` parsing fails.

## 9. Reverse-engineering workflow

1. **Identify** — scan install tree, filter files whose first four bytes are `CPK ` (`0x43 0x50 0x4B 0x20`).
2. **Parse ToC** — try `quickbms cpk.bms` on a sample. If clean, move on; else write a Rust parser using `binrw` plus a custom `@UTF` decoder (`goblin` for PE work on `nie.exe`).
3. **Extract** — to a scratch volume (60 GB needs planning). Preserve relative paths when `DirName` is present.
4. **Classify** — magic-byte scan: `USM` → video, `ADX` / `HCA` → audio, `.uasset` → Unreal, else → engine-specific.
5. **Document** — taxonomy, codecs, encryption status feed back into `PLAN.md` §3.
6. **Crate** — promote into `ievr-fmt::cpk` once stable across all 921 archives.
