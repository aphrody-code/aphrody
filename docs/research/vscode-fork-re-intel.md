<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: 2026 aphrody contributors -->

# VSCode-fork reverse-engineering intel

> Facts mined from `microsoft/vscode` (`main`, fetched 2026-05-21) to analyse a
> VSCode fork such as the **Antigravity** workbench (Codeium fork of VSCode).
> **No secrets.** Everything below is public source structure / on-disk layout.

## 1. `product.json` — rebrand surface (`/product.json`)

The OSS `product.json` is the central branding + endpoint manifest. A fork
rebrands by overwriting these keys. Observed in upstream OSS build:

- Identity: `nameShort`, `nameLong`, `applicationName` (`code-oss`),
  `dataFolderName` (`.vscode-oss` → fork picks its own, e.g. `.antigravity`),
  `win32AppUserModelId`, `darwinBundleIdentifier`, `urlProtocol`
  (`code-oss` → fork's custom URI scheme).
- Build/quality/commit: `quality` (`stable`/`insider`) and `commit` are **not**
  in the OSS checked-in `product.json`; they are injected at build time by the
  packaging pipeline (`build/`), as is `version`. A fork's distributed
  `product.json` therefore carries `commit` (the source git SHA),
  `quality`, and `nameShort` rebranded — the fastest fingerprint of a fork.
- Extension marketplace: `extensionsGallery` (`{ serviceUrl, itemUrl, ... }`)
  is **absent** from OSS `product.json` and added by the rebrand to point at
  the fork's own gallery (Open VSX or a private one). Its presence + a
  non-Microsoft `serviceUrl` is the canonical "this is a fork" tell.
- `webviewContentExternalBaseUrlTemplate` embeds the build commit
  (`.../insider/<commit>/out/...`) — another commit-leak vector.
- `builtInExtensions[]`: pinned bundled extensions with `name`, `version`,
  `sha256`, `repo`, and gallery `metadata` (publisher ids). A Codeium fork
  swaps these for its own language-server / agent extension.

## 2. Secret storage model

### `secret://` keying (`src/vs/platform/secrets/common/secrets.ts`)

- All secrets are stored under the prefix `SECRET_STORAGE_PREFIX = "secret://"`;
  `secretStorageKey(key) = "secret://" + key`. The key itself is often a JSON
  blob, e.g. `{"extensionId":"vscode.github-authentication","key":"github.auth"}`
  (see `CROSS_APP_SHARED_SECRET_KEYS`).
- `ISecretStorageProvider.type` is `'persisted' | 'in-memory' | 'unknown'`.
  Persisted secrets are written into the same SQLite storage as everything else
  (see §3) but the *value* is the output of `encrypt()`.

### Encryption (`src/vs/platform/encryption/electron-main/encryptionMainService.ts`)

- Encryption is delegated to **Electron `safeStorage`**:
  `JSON.stringify(safeStorage.encryptString(value))` → stored;
  `safeStorage.decryptString(Buffer.from(JSON.parse(value).data))` → read. So a
  persisted secret on disk is `{"data":[<byte array>]}` JSON, where `data` is the
  OS-encrypted ciphertext.
- Key storage backend (`getKeyStorageProvider`):
  - **Windows** → `dplib` (DPAPI; `CryptProtectData`, per-user/per-machine).
  - **macOS** → `keychainAccess` (Keychain).
  - **Linux** → `safeStorage.getSelectedStorageBackend()` (gnome-libsecret /
    kwallet / `basic` plaintext). `--password-store=basic` opts into plaintext.
- Implication for RE: you **cannot** decrypt VSCode-fork secrets offline without
  the OS key material (DPAPI master key / Keychain / libsecret). aphrody must
  only *detect presence* and surface the storage backend, never the plaintext.

## 3. `state.vscdb` — SQLite ItemTable (`src/vs/base/parts/storage/node/storage.ts`)

- The global/workspace state DB is SQLite with a single table:
  ```sql
  CREATE TABLE IF NOT EXISTS ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
  ```
  Read path: `SELECT * FROM ItemTable` → `Map<key, value>`. Write path uses
  chunked `INSERT ... ON CONFLICT(key) DO UPDATE`.
- Both global state and per-secret entries live in this table; secrets are the
  `secret://...` keys whose `value` is the `{"data":[...]}` ciphertext from §2.
- File name is `state.vscdb` (with a `state.vscdb.backup` sibling).

## 4. Storage layout (`src/vs/platform/userDataProfile/common/userDataProfile.ts`)

- `globalStorageHome` = `<userData>/User/globalStorage` (default profile uses
  the top-level one; named profiles use `<profileLocation>/globalStorage`).
  - `globalStorage/state.vscdb` — the SQLite ItemTable above.
  - `globalStorage/<publisher>.<ext>/` — per-extension `globalState`.
- `workspaceStorage/<workspace-md5>/state.vscdb` — per-workspace state (keyed by
  a hash of the workspace folder URI), with a `workspace.json` describing the
  folder.
- `<userData>` root (`getUserDataPath`, `productService.nameShort`):
  - Windows: `%APPDATA%\<nameShort>` (fork rebrands `nameShort`, e.g. `Antigravity`).
  - macOS: `~/Library/Application Support/<nameShort>`.
  - Linux: `$XDG_CONFIG_HOME/<nameShort>` or `~/.config/<nameShort>`.
- Chromium-layer storage living alongside (because the workbench is an Electron
  app): `Local Storage/leveldb/` (LevelDB; auth/session blobs from web logins
  land here), `Cookies` (SQLite, encrypted per the cookieEncryption fuse),
  `Network/`, `Session Storage/`. The `Local Storage/leveldb` is the place a
  Codeium-fork web auth flow would cache tokens.

## 5. Language-server / extension-host bootstrap (fork hook point)

- VSCode launches an **extension host** child process; extensions register a
  language server via the LSP client (`vscode-languageclient`) which spawns the
  server as a child process and talks JSON-RPC over stdio/socket.
- A Codeium/Antigravity fork hooks this by shipping a built-in extension (see
  `builtInExtensions` in §1) whose `activate()` spawns the proprietary
  `language_server` binary (the Go `language_server.exe`, cf.
  `docs/plans/antigravity-exploitation.md` launch contract) and proxies
  agent/Cascade RPCs. The extension's contributed commands + the LS launch args
  are the RE seam between the open VSCode shell and the proprietary core.
- `webviewContentExternalBaseUrlTemplate` + the fork's contributed webview
  panels are where the agent UI is rendered; the commit embedded there ties the
  workbench build back to a source SHA.

## What aphrody-re should implement

The binary-level Electron detection lives in `electron.rs` (this PR). The
VSCode-fork-specific surface is data-on-disk, not a binary blob, so it belongs
in the `forensics`/`re` data-decode path (WS6/WS7 of the exploitation plan),
not in `electron.rs`. Concretely:

1. `re asar app.asar` (sibling `asar.rs`): list entries + surface
   `integrity.hash` / `unpacked` flags from the pickle header.
2. `re leveldb <Local Storage/leveldb>` (sibling `leveldb.rs`): read-only enum
   of keys, never printing decrypted values.
3. SQLite `state.vscdb` decode (`forensics`, `rusqlite`): enumerate `ItemTable`
   keys, flag `secret://` keys and report the encryption backend (DPAPI /
   Keychain / libsecret) **without** attempting decryption.
4. `product.json` parse (`forensics`): surface `nameShort`, `quality`, `commit`,
   `extensionsGallery.serviceUrl`, and rebranded `applicationName`/`urlProtocol`
   as the fork fingerprint.
5. Cross-reference the `builtInExtensions[]` against the LS launch contract to
   confirm the Codeium fork seam. All read-only, no-secret-print, cross-platform.
