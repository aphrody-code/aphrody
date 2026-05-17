<!-- SPDX-License-Identifier: Apache-2.0 -->

# ADX / HCA / AWB — CRI Audio Formats Spec

Reference for **ADX** (legacy) and **HCA** (modern) CRI audio codecs
plus the **AWB** wave-bank that wraps them. Feeds extraction from IEVR
CPK contents ([`cpk-format.md`](cpk-format.md)) into
[`cri-toolchain.md`](cri-toolchain.md) §3.

## 1. ADX (Adaptive Differential PCM, CRI variant)

- **Origin** — CRI Audio, mid-1990s; de-facto BGM codec for
  Dreamcast / PS2-era titles, still shipped as legacy stream.
- **Codec** — proprietary ADPCM, 18-byte frame header, fixed block.
- **Magic** — `0x80 0x00` at offset 0; `(c)CRI` ASCII tail.
- **Variants** — ADX1, ADX2 (extended loop metadata), AHX (MPEG-2
  AAC for cinematic dialogue).
- **Encryption** — optional XOR with title-specific key hardcoded
  in the CRI encoder at build time.
- **Looping** — embedded loop start / end sample points; canonical
  seamless-BGM path.
- **Bitrate** — typically 32 to 256 kbps.

## 2. HCA (High Compression Audio, CRI modern)

- **Origin** — CRI Audio v9+ (~2014), ADX's successor for
  storage-constrained titles.
- **Codec** — MDCT; AAC-comparable at ~1/3 the bitrate.
- **Magic** — `HCA\0` at offset 0.
- **Encryption** — optional XOR with a title-specific 8-byte key
  (`high32 | low32`); cipher table derived at decoder init.
- **Containers** — usually inside AWB (§3), nested inside CPK.
- **Looping** — sample-accurate loop markers in the HCA header.

## 3. AWB containers (CRI audio wave bank)

- **Role** — paginated archive for ADX / HCA; one AWB per asset
  category (BGM, SE, voice).
- **Pairing** — shipped with an `.acb` cuesheet referencing each
  stream by name / id.
- **Placement** — typically one level inside a CPK.
- **Magic** — `AFS2` (CRI FS v2); fixed header, offset table,
  back-to-back blobs.

## 4. Tools

- **vgmstream** — primary. Covers ADX, HCA, AWB, ACB, AAX; decodes
  to WAV; ships `vgmstream-cli` plus Foobar2000 plugin.
- **VGAudio** (`Thealexbarney/VGAudio`) — pure C# library + CLI;
  debuggable for key-derivation edge cases.
- **adx2wav / decodeAdx** — legacy CLI for archaic ADX revisions.
- **Foobar2000 + vgmstream plugin** — interactive playback for
  sample verification before batch work.

## 5. Key extraction (encrypted streams)

- **Static** — keys often hardcoded in `nie.exe` as 8-byte patterns
  near `criAtomEx_*` sites; Ghidra entropy search locates them.
- **Dynamic** — Frida hook on `criAtomEx_*` captures the key at
  decoder init; consult
  [`eac-considerations.md`](eac-considerations.md) first.
- **Catalogue** — `vgmstream` ships hundreds of known title keys;
  Level-5 has reused keys across IE titles, so IEVR's may already
  be public.

## 6. IEVR strategy

- Expect AWBs for BGM, SE, voice after CPK extraction.
- Run `vgmstream-cli` per AWB first; many CRI titles decode clean.
- On `unknown key`, scan `nie.exe` statically before Frida.
- Runtime hooks only against offline launch — EAC is governed by
  `eac-considerations.md`.

## 7. Sample workflow

```bash
# Probe streams in an AWB.
vgmstream-cli -L bgm.awb

# Extract stream 5 to WAV.
vgmstream-cli -i 5 bgm.awb -o bgm_05.wav

# Batch decode every stream under a directory.
for awb in *.awb; do
  vgmstream-cli -S 0 "$awb" -o "${awb%.awb}_%02s.wav"
done
```

## 8. References

- **vgmstream** — `vgmstream/vgmstream` on GitHub; key catalogue
  under `src/meta/`.
- **VGAudio** — `Thealexbarney/VGAudio` on GitHub.
- **criware-modding** community — Discord toolchain, key
  submissions, format addenda; primary public source.
