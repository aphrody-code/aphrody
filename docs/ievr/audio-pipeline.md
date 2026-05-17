<!-- SPDX-License-Identifier: Apache-2.0 -->

# IEVR — Audio Extraction Pipeline (CPK to WAV / OGG)

End-to-end chain that turns MD5-named CPK shards into per-stream PCM
WAV (or transcoded OGG). Companion to
[`cpk-extraction-workflow.md`](cpk-extraction-workflow.md) (walk),
[`adx-hca-format.md`](adx-hca-format.md) (codec internals),
[`cri-known-keys.md`](cri-known-keys.md) (keys).

## 1. The chain

```
CPK  ->  AWB / AFS2  ->  HCA / ADX  ->  WAV / OGG
```

CPK is the Steam-delivered pack. AWB (`AFS2` magic) is a CRI wave-bank
holding many HCA or ADX streams, paired with an optional `.acb`
cuesheet that maps stream IDs to cue names. HCA is the modern MDCT
codec; ADX is the legacy ADPCM codec still used for some BGM. WAV is
for analysis, OGG Vorbis for archival.

## 2. Per-stage tool

| Stage | Tool | Role |
|---|---|---|
| CPK -> inner files | QuickBMS + `cpk.bms` | Unwrap container. |
| AWB -> streams | vgmstream-cli | Split `AFS2` blocks. |
| HCA/ADX -> WAV | vgmstream-cli | Decode, honour loops. |
| WAV -> OGG | ffmpeg / libvorbis | Optional archival. |

Install matrix lives in [`cri-toolchain.md`](cri-toolchain.md).

## 3. End-to-end commands

```bash
# 1. CPK extract.
quickbms cpk.bms data/packs/<hash>.cpk extracted/

# 2. AWB -> per-stream WAV.
vgmstream-cli extracted/audio.awb -o "stream_%02s.wav"

# 3. WAV -> OGG (optional).
ffmpeg -i stream_01.wav -codec:a libvorbis -q:a 4 stream_01.ogg

# 4. Loop-aware BGM decode.
vgmstream-cli --loops 2 --fade-time 5 bgm.hca -o "bgm.wav"
```

## 4. Encryption handling

Try unencrypted first. On `key required`, supply via `--key <hex>`
(HCA: 8-byte `high32 | low32`; ADX: XOR key). Candidates live in
[`cri-known-keys.md`](cri-known-keys.md). Batch-try documented Level-5
keys before community brute-force tables.

## 5. Classification (BGM / SFX / Voice)

Classify by duration heuristic from the WAV header:

- **BGM**: > 30 s, stereo, loop markers present.
- **SFX**: < 5 s, mono, no loop.
- **Voice**: 1 to 15 s, mono, no loop, grouped in language-named AWB.

Refine with the paired `.acb` cuesheet: cue category bits explicitly
mark `BGM / SE / VOICE`.

## 6. Looping

HCA and ADX preserve sample-accurate loop start and end markers. For
natural BGM rendering, never truncate at the first loop - request
`--loops 2 --fade-time 5` so vgmstream plays through twice then fades.
Skip looping for SFX and Voice.

## 7. Localization

IEVR likely ships separate per-language voice CPKs
(`JP / EN / FR / ES / DE / IT / KR / CN`). Identify by AWB inner
filename patterns in the CPK ToC: substrings like `_jp`, `_en`,
`voice_fr`, `loc_de` from
[`cpk-extraction-workflow.md`](cpk-extraction-workflow.md). BGM is
language-neutral, usually in a dedicated `bgm.cpk`.

## 8. Bulk pipeline (sketch)

```bash
# Decode every AWB to a flat hash-prefixed output dir.
out=decoded/
mkdir -p "$out"
find extracted/ -type f -name "*.awb" | while read -r awb; do
  cpk_hash="$(basename "$(dirname "$awb")")"
  awb_name="$(basename "$awb" .awb)"
  vgmstream-cli "$awb" -o "${out}/${cpk_hash}__${awb_name}__%02s.wav"
done
```

Layout: `decoded/<cpk_hash>__<awb>__<stream_idx>.wav`. Provenance
(CPK + AWB + stream) embedded in the filename so the classifier in
[`asset-classification-pipeline.md`](asset-classification-pipeline.md)
ingests the flat tree without extra metadata.
