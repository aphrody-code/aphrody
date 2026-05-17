<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody contributors
-->

# IEVR — save format hypotheses

**Status: pre-P2, almost entirely HYPOTHESIS.** No save file has been
located or hashed by aphrody yet. Claims below are predictions from prior
Level-5 titles and 2018+ PC port conventions. Cross-reference
[`eac-considerations.md`](eac-considerations.md) before dynamic capture and
`legal-checklist.md` (pending) before publishing tooling.

## 1. Where saves live on Windows — HYPOTHESIS

Candidates, in decreasing likelihood:

- `%LOCALAPPDATA%\Level-5\IEVR\` or `%LOCALAPPDATA%\Level5\InazumaElevenVR\`
- `%APPDATA%\Level-5\IEVR\` (roaming variant; unlikely for a binary blob)
- `%USERPROFILE%\Documents\My Games\IEVR\`
- `<Steam install>\saves\` (legacy convention, uncommon today)
- `steamapps\userdata\<steamid>\<appid>\remote\` — Steam Cloud mirror

## 2. Prior Level-5 conventions — PARTIALLY CONFIRMED via community RE

From third-party reversing (verify before trusting):

- **3DS IE Go / Go 2** — `.dat` with rotating XOR mask plus CRC32 footer.
- **3DS IE 3 Sekai e** — similar XOR; header magic varied per region build.
- **PC era (post-2018)** — predicted migration toward structured
  serialisation (JSON, MessagePack, FlatBuffer, or proprietary TLV) under
  a thin obfuscation wrapper. Unverified for IEVR.

## 3. Detection workflow — PROCEDURE

1. Snapshot `%LOCALAPPDATA%`, `%APPDATA%`, Documents, and `userdata\`
   before launch.
2. Boot via Steam, play roughly ten minutes, trigger a manual save, exit.
3. Diff the snapshots. Mutated files under §1 paths are candidates.
4. Run Procmon with a path filter during save to confirm the active file.
5. Hex view the candidate; look for plaintext player name, team name, or
   gold amount. Plaintext fields argue against a strong cipher.

## 4. Common Level-5 obfuscation patterns — HYPOTHESIS

- XOR with a title-derived key (often `MD5(title_string)` as 16 bytes
  cycled across the payload).
- Header CRC32 over the body. Bit-flip a non-meaningful byte; if the title
  refuses to load, integrity checking is confirmed.
- Optional `IsDirty` or `LastWriteTimestamp` field near the header.
- Footer length field for variable-length section tables.

## 5. Tools to use — PROCEDURE

- **HxD** or **010 Editor** for hex inspection and template authoring.
- Short Python script for XOR brute force across a curated key list
  (title strings, build IDs, Level-5 internal codenames).
- **Cheat Engine** in-memory only for live-value scans (gold `1000`,
  `0x03E8`, `0x000003E8`). Cheat Engine trips EAC — see §6 and
  `eac-considerations.md`.

## 6. Integrity considerations — CONFIRMED policy

- Always copy the candidate save to `<path>.bak` before any byte edit.
- Edited saves may fail an internal CRC or HMAC check and be flagged by
  EAC as tampering. **Never run a modified save with EAC attached.**
- Steam Cloud overwrites local edits on next sync. Disable cloud sync for
  the title before testing, or work entirely offline.

## 7. Cross-platform — HYPOTHESIS

Switch and PS5 builds will not share the Windows layout: Switch uses NEX
or platform-SDK serialisation; PS5 uses its own trophy and save sandbox.
Each requires its own RE pass; future dumps should not assume byte
compatibility, even for free-form score and roster fields.

## 8. Open questions — to resolve post-P2

- [ ] What is the save file extension and final on-disk path?
- [ ] Which obfuscation scheme (XOR, AES, Salsa20, or none)?
- [ ] What header magic bytes identify the format?
- [ ] What field-level layout (protobuf, FlatBuffer, MessagePack, TLV)?
- [ ] What integrity check (CRC32, HMAC, signature)?
- [ ] Does Steam Cloud add an extra envelope or signature?
- [ ] Are multiple slots stored in one file or as separate files?

## 9. Mission alignment

Save analysis is **singleplayer only**; multiplayer rank syncing is out of
scope per `legal-checklist.md` (pending). The deliverable is a format spec
for archival and interop research — no save-editing or cheat tooling will
be published.

See also: [`eac-considerations.md`](eac-considerations.md),
[`PLAN.md`](PLAN.md), `legal-checklist.md` (in flight).
