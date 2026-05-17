<!-- SPDX-License-Identifier: Apache-2.0 -->

# IEVR — CRI encryption key reference (community-documented)

> **Disclaimer.** Keys here are PUBLICLY KNOWN from community RE work
> (vgmstream source, public mod tools). They unlock the user's OWN
> purchased game content for personal use. Do NOT use them to access
> content you do not own. Per `legal-checklist.md` _(pending)_, this is
> interoperability research on licit copies. Cross-refs:
> [`cpk-format.md`](cpk-format.md), `adx-hca-format.md` _(pending)_,
> [`eac-considerations.md`](eac-considerations.md).

## 1. How CRI keys work

CPK + HCA + ADX use XOR or AES with per-title keys:

- **8-byte XOR mask (HCA)** — two 32-bit values (`key1` / `key2`).
- **64-byte or 128-byte AES key** — newer CPK variants (post ~2018).
- **Hardcoded in the game binary** — for IEVR, `nie.exe`.

## 2. Where keys live in shipped games

- **Hardcoded constants** — 8-byte patterns near refs to `criAtomEx*`,
  `criWare*`, `cri_fs_*`.
- **Runtime API** — `criAtomEx_SetEncryptionKey()` at startup; Frida hook
  only if [`eac-considerations.md`](eac-considerations.md) §4 permits.
- **Derived** — some Level-5 titles compute keys from a CRC of the title
  string (look for `nameOfTitle` + CRC32 imports).

## 3. Known Level-5 title keys (community-documented)

> Values from `vgmstream`'s `hca_keys.h` and public mod-tool repos.
> Starting hypotheses for IEVR; this repo does not fabricate hex.

| Title                            | Year      | Key (HCA, hex)        | Source              |
| -------------------------------- | --------- | --------------------- | ------------------- |
| Yokai Watch 4 (Switch)           | 2019      | `<placeholder>`       | vgmstream community |
| Yokai Watch Jam: Yokai Academy Y | 2020      | `<placeholder>`       | vgmstream community |
| Snack World                      | 2017      | `<placeholder>`       | vgmstream community |
| Inazuma Eleven (DS / 3DS era)    | 2008-2017 | `<placeholder — AES>` | various mod tools   |

## 4. IEVR — starting hypothesis

IEVR most likely uses HCA with a title-specific key. Try:

1. Run `vgmstream-cli` on `.awb` with **no key** — if it plays, the
   title shipped unencrypted.
2. Try keys from prior Level-5 titles (§3).
3. If none match, static-analyse `nie.exe` for 8-byte constants near
   `criAtomEx_*` call sites.
4. If static fails, Frida hook — only if
   [`eac-considerations.md`](eac-considerations.md) §4 clears it.

## 5. CPK encryption

Some recent CPKs use AES (not XOR). Detection: open with QuickBMS — if
it fails with "decryption error", it is AES-encrypted. AES keys are
hardcoded or loaded via `cri_fs_BindEncryptionFunction()`; §4 Frida
caveats apply.

## 6. Tool reference for key handling

- **vgmstream** — `vgmstream-cli --hca-key <hex>`.
- **VGAudio** (Thealexbarney) — C# API, programmatic key setting.
- **CriPakTools** — GUI field for AES key entry.
- **QuickBMS** — `encryption aes` / `encryption xor` directives.

## 7. Per-bit ethics

- Use these keys ONLY on content already licensed to the user.
- DO NOT publish IEVR's key here unless it has landed in vgmstream
  upstream (then it is community knowledge, not fresh disclosure).
- If a NEW key is discovered, contribute back to vgmstream / CriPakTools
  rather than publishing in isolation.

## 8. References

- vgmstream HCA keys:
  <https://github.com/vgmstream/vgmstream/blob/master/src/meta/hca_keys.h>
- CriPakTools (Brolijah fork) README and source.
- CRI Middleware public docs (limited; most internals are NDA).
- Cross-doc: [`cpk-format.md`](cpk-format.md) §4,
  `adx-hca-format.md` _(pending)_ §3,
  [`eac-considerations.md`](eac-considerations.md) §3-4.
