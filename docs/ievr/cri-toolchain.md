<!-- SPDX-License-Identifier: Apache-2.0 -->

# IEVR — CRIWare / CPK Toolchain

CRIWare-specific RE tooling for IEVR. The client carries 921 CPK archives
plus USM video and (very likely) ADX/HCA audio, all built on CRI Middleware.
This doc covers **only** CRI-format tools — generic disassembly and ML
tooling live in the sibling `re-toolchain.md` and `ml-env-audit.md`. Format
internals for the CPK container live in [`cpk-format.md`](cpk-format.md).

## 1. CPK extraction

- **CriPakTools** (Brolijah fork) — C# extractor + repacker. Reliable on
  un-encrypted CPKs; failures are loud.
- **QuickBMS + `cpk.bms`** — Luigi Auriemma's engine plus the community
  script. Lowest setup cost, scriptable from CI, handles most non-encrypted
  variants and many keyed ones.
- **FModel** — CPK support exists but is secondary to its Unreal focus.
- **Custom Rust parser** — for batch work over all 921 archives, a walker
  on `goblin` + `binrw` outperforms shell-spawning external tools. Justified
  only after format internals are pinned.

## 2. USM (Sofdec2 video) decoding

- **vgmstream** — strong USM **audio** support. Use first to pull audio
  out of mixed video containers.
- **CRI Demultiplexer** — CRIWare's first-party tool. Closed source, free
  for non-commercial use. Authoritative reference for split correctness.
- **ffmpeg with USM patches** — community patches add USM demux; quality
  varies by patch generation, treat as a fallback.
- **GongPlayer** — Japanese fan-built USM viewer for visual confirmation.

## 3. ADX / HCA (CRI audio) decoding

- **vgmstream** — primary tool. Covers ADX, ADX2, HCA, AAX, CRI ACB/AWB.
  Outputs WAV directly; trivial to script.
- **VGAudio** (Thealexbarney) — C# library + CLI. Pure-managed HCA decoder,
  handy on key derivation edge cases vgmstream stumbles on.
- **decodeAdx / adx2wav** — legacy CLI tools, useful only on archaic ADX
  revisions modern vgmstream skips.

## 4. AFS2 (CRI archive variant)

AFS2 is a paginated container shipped alongside CPK as an audio bank
(`.awb`). Simpler than CPK: fixed header, offset table, back-to-back blobs.
QuickBMS handles AFS2 with the matching script; for IEVR the AFS2 surface
is expected to be audio-only.

## 5. Key extraction (encrypted CPKs)

- **Frida hook** on `criFsBinder_BindCpk()` or `criFsIo_Open()` inside
  `nie.exe`. Capture the key argument at runtime; CRIWare's DLL ABI is
  stable enough that public Frida snippets transfer across titles.
- **Static analysis** — search `nie.exe` for AES-key-shaped byte runs via
  Ghidra string + entropy analysis, cross-referenced against CRIWare init
  call sites.
- **Heuristic seeds** — some titles derive the key from the English game
  name, the publisher code, or a fixed CRIWare default. Try those first.

## 6. Validation / smoke tests

- After extract, confirm file count equals the ToC entry count from the
  tool's verbose mode.
- Sample-decode 3 to 5 USM files to confirm key + demux; GongPlayer
  suffices for visual confirmation.
- Inspect `nie.exe`'s import table for `cri_*` exports (`dumpbin /imports`
  or `llvm-readobj --coff-imports`); presence + version pin the toolchain
  branch.

## 7. Recommendations for IEVR

- **Start** with QuickBMS + `cpk.bms` on one sample CPK — lowest setup
  cost, fail-fast signal on encryption.
- **If encrypted**: try CriPakTools next (built-in key heuristics), then
  fall back to Frida on `nie.exe`.
- **For USM**: vgmstream for audio, CRI Demultiplexer for video split,
  ffmpeg for transcode downstream.
- **For ADX / HCA**: vgmstream straight to WAV.

## 8. Open questions to resolve in P1 setup

- Are the IEVR CPKs encrypted? Test: open one with QuickBMS, check exit
  code + extracted file headers.
- Which CRIWare version ships in `nie.exe`? Grep `.rdata` for `cri 2.xx`
  or `criware_le`.
- Does the title ship AFS2 alongside CPK, or are audio banks embedded?
  `Get-ChildItem -Recurse -Include *.awb` over the install root answers it.
- Are USM streams demuxed per-language, or multi-track inside one
  container? Affects the audio extraction pipeline shape.
