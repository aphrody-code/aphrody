<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: 2026 aphrody contributors -->

# Electron reverse-engineering intel

> RE-useful facts mined from `electron/electron`, `electron/asar`, and
> `electron/fuses` (upstream, `main` branch, fetched 2026-05-21). Used to drive
> `crates/aphrody-re/src/electron.rs`. **No secrets.** Everything below is the
> public on-disk / on-binary wire format of an Electron application.

## 1. ASAR archive format (`electron/asar`)

ASAR is a concatenation archive ("like `tar` without compression") with a JSON
header and random-access support. Layout, from `src/disk.ts`
(`readArchiveHeaderSync`) and `src/filesystem.ts`:

```
[ 8-byte Pickle "size" header ][ Pickle-wrapped UTF-8 JSON header ][ file payloads ... ]
```

- The first 8 bytes are a Chromium `Pickle` blob (`Pickle.createFromBuffer`)
  whose first `UInt32` is the byte length of the following header pickle. (The
  4 leading bytes encode the pickle's own payload length; the next 4 bytes are
  the JSON header size — this is why the size prefix is read as 8 bytes, not 4.)
- The header pickle then holds a single `readString()` value: the JSON
  directory tree. Each leaf is a `FilesystemFileEntry`:
  `{ offset: string, size: number, executable?: bool, unpacked?: bool, integrity?: {...} }`.
  `offset` is a *decimal string* (validated by `/^\d+$/` in `disk.ts`
  `validateFileEntry`) because file offsets can exceed `2^53`.
- Directory entries: `{ files: { <name>: <entry> } }`. Link entries: `{ link }`.

### app.asar vs app.asar.unpacked

Files flagged `"unpacked": true` in the header are **not** stored inside
`app.asar`; they live next to it under `app.asar.unpacked/<relpath>` (native
`.node` addons, executables, anything that must exist as a real file on disk).
`asar pack --unpack-dir "{x1,x2}"` controls which paths are externalised
(README "Excluding multiple resources from being packed").

### ASAR integrity (`src/integrity.ts` + `docs/tutorial/asar-integrity.md`)

- Algorithm fixed at **SHA256**, default `blockSize` = 4 MiB (`4 * 1024 * 1024`).
- Per-file integrity: `{ algorithm: "SHA256", hash, blockSize, blocks: [...] }`
  where `hash` is over the whole file and `blocks[i]` is over each `blockSize`
  chunk. Empty files still emit one block hash of the empty input.
- **Header hash**: separately, a hex SHA256 of the *entire JSON header* is
  embedded out-of-band at package time and checked at runtime. A mismatch (or
  absence) **forcibly terminates** the app when the integrity fuse is on.
- Out-of-band header-hash storage is platform-specific:
  - **macOS** `Info.plist` `ElectronAsarIntegrity` dict, keyed by
    `Resources/app.asar` → `{ algorithm: "SHA256", hash: "<hex>" }`.
  - **Windows** (electron ≥ 30) embeds it in the binary / resource. Integrity
    checking is supported macOS ≥ 16.0.0, Windows ≥ 30.0.0.

## 2. Electron Fuses (`electron/fuses` + `docs/tutorial/fuses.md`)

Fuses are "magic bits" flipped at package time, *before* code signing, so the
OS code-signature (Gatekeeper / AppLocker) prevents them being flipped back.

### Sentinel + wire layout (authoritative, `src/constants.ts` + `src/index.ts`)

```
SENTINEL = "dL7pKGdnNz796PbbjQWNKmHXBZaB9tsX"   (32 ASCII bytes)
... immediately followed by ...
[ version: 1 byte ][ wire_length: 1 byte ][ fuse byte 0 ][ fuse byte 1 ] ... [ fuse byte N-1 ]
```

`readFuseWire` reads `version = buf[0]`, `length = buf[1]` (single byte, so
`wire_length <= 255`), then `length` fuse-state bytes. Each fuse-state byte is
**not** `0/1/2** — the brief's assumption is wrong. From `src/constants.ts`:

```
FuseState.DISABLE = 0x30   ('0')
FuseState.ENABLE  = 0x31   ('1')
FuseState.REMOVED = 0x72   ('r')
FuseState.INHERIT = 0x90   (no ASCII glyph)
```

`@electron/fuses` scans for the sentinel with `Buffer.indexOf` over 4 MiB
chunks (the canonical RE technique: locate sentinel, read header, decode wire).
Two sentinels = universal macOS binary (one per arch slice); >2 = corrupt.

### Fuse order (V1, `src/config.ts` `FuseV1Options`)

| Index | Name | Default | Effect when disabled |
|---|---|---|---|
| 0 | `RunAsNode` | enabled | `ELECTRON_RUN_AS_NODE` ignored (kills a class of LotL attacks) |
| 1 | `EnableCookieEncryption` | disabled | cookie SQLite DB stays plaintext |
| 2 | `EnableNodeOptionsEnvironmentVariable` | enabled | `NODE_OPTIONS` / `NODE_EXTRA_CA_CERTS` ignored |
| 3 | `EnableNodeCliInspectArguments` | enabled | `--inspect`/`--inspect-brk` + `SIGUSR1` debugger disabled |
| 4 | `EnableEmbeddedAsarIntegrityValidation` | disabled | no runtime app.asar header-hash check |
| 5 | `OnlyLoadAppFromAsar` | disabled | search order limited to `app.asar` only |
| 6 | `LoadBrowserProcessSpecificV8Snapshot` | disabled | main process loads `browser_v8_context_snapshot.bin` |
| 7 | `GrantFileProtocolExtraPrivileges` | enabled | `file://` pages keep extra fetch/service-worker/universal-access privileges |
| 8 | `WasmTrapHandlers` | enabled | V8 signal handlers trap OOB Wasm memory access |

Security-relevant RE reading: `RunAsNode=ENABLE` + `EnableNodeCliInspect=ENABLE`
+ `NodeOptions=ENABLE` on a shipped app means the renderer/main can be coerced
into running arbitrary Node — a "living off the land" surface. `OnlyLoadAppFromAsar=DISABLE`
+ `AsarIntegrity=DISABLE` means app code can be swapped on disk without detection.

## 3. V8 snapshot / code cache (`ELECTRON_RUN_AS_NODE`, snapshots)

- `ELECTRON_RUN_AS_NODE`: env var that makes the Electron binary behave as a
  plain Node. Gated by the `RunAsNode` fuse (index 0). Its presence as a string
  literal in a binary is a strong "this is Electron, RunAsNode-capable" marker.
- V8 snapshot files shipped beside the binary: `v8_context_snapshot.bin`,
  `snapshot_blob.bin`, and (with fuse 6) `browser_v8_context_snapshot.bin`.
  These are mmaped V8 startup heaps; the `LoadBrowserProcessSpecificV8Snapshot`
  fuse selects the browser-specific one.
- V8 code cache (`bytenode`, `v8-compile-cache`): compiled-JS blobs prefixed by
  a V8 cached-data header. The header begins with a magic/version word derived
  from the V8 version + flags hash; a mismatch invalidates the cache. RE tools
  treat the presence of `cachedData` / `.jsc` files + the header word as the
  marker rather than fully parsing the (V8-version-coupled) format.

## 4. Canonical Electron RE tooling (GitHub survey)

| Tool | What it parses |
|---|---|
| `@electron/asar` (electron/asar) | ASAR list/extract; `readArchiveHeaderSync` exposes the pickle header + integrity dict |
| `asar` npm CLI (`asar l`, `asar ef`, `asar e`) | list / extract-file / extract |
| `@electron/fuses` (electron/fuses) | `read`/`write` fuse wire via the sentinel scan documented above |
| `electron-fuses` CLI (`npx @electron/fuses read --app`) | dumps decoded fuse states of a packaged app |
| `bytenode` / `v8-compile-cache` | produce/consume V8 code-cache (`.jsc`) blobs |
| `asar-extractor`, `npx asar`, `7zip asar plugin` | community ASAR extractors (same pickle format) |
| `electron-asar-hot-patcher`, `nukeasar` | patch app code inside app.asar (relevant to OnlyLoadAppFromAsar / integrity bypass research) |

(`electron-fiddle` is a dev sandbox, **not** an RE tool, and is excluded.)

## What aphrody-re should implement

Implemented in `crates/aphrody-re/src/electron.rs` over a raw `&[u8]`:

1. **Fuse sentinel scan** via `memchr::memmem::find` for
   `dL7pKGdnNz796PbbjQWNKmHXBZaB9tsX`; on hit, read `version`/`length` header
   then decode `length` fuse bytes using the **real** state alphabet
   (`0x30/0x31/0x72/0x90`) into named `FuseState{name,state}` using the V1 order
   above. Never index past the slice (return what is readable).
2. **Security verdicts** derived from decoded fuses: `run_as_node`,
   `asar_integrity` (fuse 4 enabled), `only_load_app_from_asar` (fuse 5).
3. **Version extraction**: `Electron/<v>` / `electron@<v>` (electron),
   `node/<v>` / `Node.js v<v>` (node), reuse the `Chrome/M.m.b.p` pattern
   (chromium) — surfaced independently of family.
4. **V8 markers**: `v8_context_snapshot`, `snapshot_blob`,
   `browser_v8_context_snapshot` (snapshot) and V8 code-cache hints.
5. **`ELECTRON_RUN_AS_NODE`** literal detection as an extra RunAsNode signal.
6. Pure-Rust, no GPL, `#![forbid(unsafe_code)]` (inherited), never panics on
   arbitrary input. ASAR header pickle parsing stays in the sibling `asar.rs`
   module (owned by a concurrent agent); this module is binary-only.
