<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody contributors
-->

# IEVR — save format hypotheses

**Status: pre-P2, almost entirely HYPOTHESIS.** No save file has been
located or hashed by aphrody yet. Predictions derive from prior Level-5
titles. Cross-reference [`eac-considerations.md`](eac-considerations.md)
before dynamic capture and `legal-checklist.md` (pending) before tooling
release.

## 1. Where saves live on Windows — HYPOTHESIS

Candidates, in decreasing likelihood:

- `%LOCALAPPDATA%\Level-5\IEVR\` or `Level5\InazumaElevenVR\`
- `%APPDATA%\Level-5\IEVR\` (roaming variant; unlikely for a binary blob)
- `%USERPROFILE%\Documents\My Games\IEVR\`
- `<Steam install>\saves\` (legacy convention, uncommon today)
- `steamapps\userdata\<steamid>\<appid>\remote\` — Steam Cloud mirror

## 2. Prior Level-5 conventions — PARTIALLY CONFIRMED via community RE

- **3DS IE Go / Go 2** — `.dat`, rotating XOR mask, CRC32 footer.
- **3DS IE 3 Sekai e** — similar XOR; header magic varied per region.
- **PC era (post-2018)** — predicted structured serialisation (JSON,
  MessagePack, FlatBuffer, TLV) under a thin obfuscation wrapper.
  Unverified for IEVR.

## 3. Detection workflow — PROCEDURE

1. Snapshot `%LOCALAPPDATA%`, `%APPDATA%`, Documents, `userdata\`.
2. Boot via Steam, play ten minutes, manual save, exit.
3. Diff snapshots; mutated files under §1 are candidates.
4. Run Procmon with a path filter during save to confirm the file.
5. Hex view; look for plaintext name, team, or gold. Plaintext argues
   against a strong cipher.

## 4. Obfuscation patterns — HYPOTHESIS

- XOR with a title-derived key (often `MD5(title_string)` cycled).
- Header CRC32 over the body. Bit-flip a non-meaningful byte; refusal to
  load confirms integrity checking.
- Optional `IsDirty` or `LastWriteTimestamp` field near the header.
- Footer length field for variable-length section tables.

## 5. Tools — PROCEDURE

- **HxD** or **010 Editor** for hex inspection and template authoring.
- Python XOR brute-force script across a curated key list (titles,
  build IDs, internal codenames).
- **Cheat Engine** in-memory only for live-value scans (gold `1000`,
  `0x03E8`). Cheat Engine trips EAC — see §6.

## 6. Integrity considerations — CONFIRMED policy

- Always copy candidates to `<path>.bak` before any byte edit.
- Edited saves may fail an internal CRC or HMAC check and be flagged by
  EAC as tampering. **Never run a modified save with EAC attached.**
- Steam Cloud overwrites local edits on next sync. Disable cloud sync for
  the title before testing, or work offline.

## 7. Cross-platform — HYPOTHESIS

Switch and PS5 builds will not share the Windows layout: Switch uses
NEX or platform-SDK serialisation; PS5 uses its own save sandbox. Each
requires its own RE pass.

## 8. Open questions — to resolve post-P2

- [ ] Save file extension and final on-disk path?
- [ ] Obfuscation scheme (XOR, AES, Salsa20, none)?
- [ ] Header magic bytes?
- [ ] Field-level layout (protobuf, FlatBuffer, MessagePack, TLV)?
- [ ] Integrity check (CRC32, HMAC, signature)?
- [ ] Does Steam Cloud add an extra envelope?
- [ ] Multiple slots in one file or separate files?

## 9. Mission alignment

Save analysis is **singleplayer only**; multiplayer rank syncing is out
of scope per `legal-checklist.md` (pending). Deliverable is a format
spec for archival and interop research — no cheat tooling published.

See also: [`eac-considerations.md`](eac-considerations.md),
[`PLAN.md`](PLAN.md), `legal-checklist.md` (in flight).
