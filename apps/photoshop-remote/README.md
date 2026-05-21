<!-- SPDX-License-Identifier: Apache-2.0 -->
# @aphrody/photoshop-remote

Bun-native TypeScript client for Adobe Photoshop's **Remote Connections**
feature (Edit ▸ Remote Connections). It opens a TCP socket to a live Photoshop
on port `49494` and runs **ExtendScript** on it over the network — the third
Photoshop control surface in aphrody, alongside the headless cloud API
(`aphrody-firefly::photoshop`) and the local UXP bridge (`apps/photoshop-uxp`).

> Bun/TypeScript is used here by explicit authorization (CLAUDE.md §2 otherwise
> bans JS). It leans on **Bun-native APIs**: `Bun.connect` for TCP and
> WebCrypto (`crypto.subtle`) for PBKDF2; `node:crypto` (Bun's native
> implementation) supplies Triple-DES, which WebCrypto lacks.

## Protocol

3DES-CBC (`des-ede3-cbc`); key = PBKDF2-HMAC-**SHA1**(password, salt
`"Adobe Photoshop"`, 1000 iterations, 24 bytes); IV = 8 zero bytes; PKCS#7;
**no HMAC**. Wire frame: `[u32 BE len = 4 + ciphertext][u32 BE status=0][ciphertext]`;
decrypted cleartext: `[u32 BE version=1][u32 BE transaction][u32 BE contentType][body]`.
Content type `2` = ExtendScript, `1` = error string. (Sources: Adobe
`generator-core/lib/ps_crypto.js` + `photoshop.js`; `kyamagu/photoshop-connection`.)

## Setup

```bash
cd apps/photoshop-remote
bun install        # @types/bun + typescript (dev only; the client is dep-free)
bun test           # pure-protocol round-trip tests (no network)
```

## Usage

Credentials come from the environment (preferred) — never written to disk:

```bash
PS_HOST=<host> PS_PASSWORD=*** bun src/cli.ts 'app.activeDocument.name'
# → {"transaction":1,"contentType":2,"text":"Untitled-1.psd","isError":false}
```

Library:

```ts
import { PhotoshopRemote } from "@aphrody/photoshop-remote";

const ps = new PhotoshopRemote({ host: process.env.PS_HOST!, password: process.env.PS_PASSWORD! });
await ps.connect();
console.log((await ps.info()).text);                 // "PS 27.7.0 | docs=1 | active=…"
const r = await ps.exec('app.documents.length');     // any ExtendScript
ps.close();
```

`exec()` multiplexes over one socket and correlates each reply by transaction
id, so concurrent calls are safe. `{ shared: true }` uses Photoshop's shared
scripting engine (content type 10).

## Enabling on the Photoshop side

In Photoshop: **Edit ▸ Remote Connections** → enable, set a Service Name and
password, port `49494`. The host must be reachable on the network (LAN).

## Security

The password and host are runtime inputs (env/argv); nothing here is committed.
Triple-DES is a legacy cipher — it is dictated by Photoshop's protocol, not a
choice. Treat the connection as you would any LAN admin channel.
