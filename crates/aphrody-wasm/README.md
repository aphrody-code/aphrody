[//]: # (SPDX-License-Identifier: Apache-2.0)

# @aphrody-code/aphrody-wasm

WebAssembly bindings for [aphrody](https://github.com/aphrody-code/aphrody) — the
cross-platform CLI toolkit.  Compiled from Rust via `wasm-bindgen` and distributed
as an ES module for browser consumers.

## Installation

```sh
npm install @aphrody-code/aphrody-wasm
```

## Exported API

| Export | Signature | Description |
|---|---|---|
| `start` | `() => void` | Installs the browser panic hook and wires `log` macros to `console.*`. Called automatically on module load via `#[wasm_bindgen(start)]`. |
| `version` | `() => string` | Returns the crate version declared in `Cargo.toml` (e.g. `"1.0.0-canary"`). |
| `platform_short_name` | `() => string` | Returns the compile-time target triple short name (`"wasm32-unknown-unknown"` in browsers, `"wasm32-wasip1"` under WASI). |
| `decrypt_aes_gcm` | `(ciphertext: Uint8Array, key: Uint8Array) => Uint8Array` | Decrypts AES-256-GCM ciphertext. `ciphertext` must be at least 15 bytes (3-byte version prefix + 12-byte nonce + payload). `key` must be exactly 32 bytes. Throws a JS `Error` on failure. |

## Browser usage

```html
<script type="module">
import init, { decrypt_aes_gcm, version } from '@aphrody-code/aphrody-wasm';

await init(); // fetches and instantiates aphrody_wasm_bg.wasm

console.log('aphrody-wasm', version()); // e.g. "1.0.0-canary"

const key = crypto.getRandomValues(new Uint8Array(32));
// ciphertext must be produced by the aphrody base crate's Crypto::encrypt_aes_gcm
const plaintext = decrypt_aes_gcm(ciphertext, key);
</script>
```

## Examples

The [`examples/`](examples/) directory contains a self-contained browser playground
that exercises all three exported functions without any external dependencies:

- **[`examples/browser-playground.html`](examples/browser-playground.html)** — displays
  `version()` and `platform_short_name()` at page load, then provides an interactive
  AES-256-GCM encrypt/decrypt panel powered by `decrypt_aes_gcm()`.

Build and serve:

```sh
cd crates/aphrody-wasm
wasm-pack build --target web --release
bun x serve examples/
# or: python -m http.server -d examples 8000
```

Open <http://localhost:8000/browser-playground.html>.

## License

Apache-2.0 — see [LICENSE](../../LICENSE).
