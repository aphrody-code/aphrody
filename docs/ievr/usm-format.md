<!-- SPDX-License-Identifier: Apache-2.0 -->

# USM (CRI Sofdec2) — Video Container Format Spec

Reference notes for **USM** under `data/dx11/movie/*.usm` in IEVR.
Sibling docs: [`cpk-format.md`](cpk-format.md),
[`cri-toolchain.md`](cri-toolchain.md).

## 1. Origin

USM (**U**niversal **S**tream **M**ux) is CRI Middleware's video
container, produced by **CRI Sofdec** (PlayStation-era MPEG-1) and
**CRI Sofdec2** (~2010 onward: H.264 / VP9 video, ADX / HCA audio).
De-facto cutscene container for any title already using CPK.

## 2. Container structure

USM is **chunk-based**: back-to-back fixed-format chunks, each carrying
one slice of one elementary stream.

- **Header chunk** `CRID` at offset `0x00` carries a `@UTF` table (same
  binary descriptor as CPK) listing streams and codecs.
- **Chunk header**: 4-byte ASCII tag, big-endian 32-bit size, two
  reserved bytes, 1-byte header offset, 1-byte footer offset, payload.
- **Chunk types**: `CRID` (file header); `@SFV` (video access units);
  `@SFA` (audio frames); `@ALP` (alpha plane); `@SBT` (subtitles,
  rare); `#CONTENTS END   ` (16-byte end sentinel).
- Streams interleave in playback order to bound demuxer buffers.

## 3. Codecs supported

- **H.264 / AVC** — modern default. Annex-B NALs in `@SFV`; SPS / PPS
  in the `CRID` table.
- **VP9** — Sofdec2 since ~2018; royalty-free encodes.
- **Sofdec.Prime** — legacy MPEG-4-Visual derivative; logo stings.
- **MPEG-1** — original Sofdec codec; retired on PC.

## 4. Audio tracks

Audio rides in `@SFA` chunks; codec recorded in the `CRID` table.

- **ADX** — CRI's legacy ADPCM. Magic `0x80 0x00`.
- **HCA** — CRI's modern psychoacoustic codec. Magic `HCA\0`; per-title
  key variants exist.
- **AIX** — multi-channel ADX wrapper; rare in cutscenes.
- Multi-track is common: EN plus JP audio interleaved in one USM.

## 5. Encryption

A subset of titles XOR-masks `@SFV` chunk bodies — never the headers —
with a per-title key supplied to CRI's encoder. Without the key:
playable audio plus glitched video. Recover by scanning the main
executable for the key constant near `criMana_*` imports, or by hooking
`criMana_Player_SetKey()` with Frida. Try plain demux first; many
titles ship unencrypted.

## 6. Magic bytes

- File header: `CRID` at offset `0x00` (`0x43 0x52 0x49 0x44`).
- Per-chunk tags: `@SFV`, `@SFA`, `@ALP`, `@SBT`, `CRID`,
  `#CONTENTS END   `.
- `ievr-fmt::usm` detection: leading `CRID` plus a valid big-endian
  chunk length plus an `@UTF` signature at the expected offset.

## 7. Tools

- **vgmstream** — USM audio (ADX / HCA to WAV).
- **CRI Demultiplexer** — CRIWare's first-party splitter; closed
  source, free for non-commercial use. Correctness oracle.
- **ffmpeg with USM patches** — community patches; fallback.
- **WannaCRI** (Python) — open-source demuxer / remuxer; cross-check
  oracle for Rust bring-up.

## 8. IEVR-specific notes

IEVR ships USM cutscenes under `data/dx11/movie/`. Confirmed in
inventory: **`IE_15th.usm`** (4.3 MB, Inazuma Eleven 15th anniversary
sting) and **`L5logo.usm`** (3.3 MB, Level-5 corporate logo intro).
Both load before the title screen and fall in the "logo sting" class
(< 10 MB), suggesting larger story cutscenes are packed inside CPK
archives under the same `movie/` taxonomy (see
[`cpk-format.md`](cpk-format.md) §9 step 4). Next: demux loose USMs
with vgmstream plus WannaCRI to confirm codecs; scan `nie.exe` for
`criMana_*` to pin the Sofdec2 runtime; promote the demuxer into
`ievr-fmt::usm` once parsing is stable across loose and CPK-embedded
samples.
