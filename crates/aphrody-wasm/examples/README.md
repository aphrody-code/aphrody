[//]: # (SPDX-License-Identifier: Apache-2.0)

# aphrody-wasm examples

## browser-playground.html

Build the WASM package first, then serve the `examples/` directory:

```sh
cd crates/aphrody-wasm
wasm-pack build --target web --release
bun x serve examples/
# or: python -m http.server -d examples 8000
```

Open <http://localhost:8000/browser-playground.html>.

The page demonstrates `version()`, `platform_short_name()`, and `decrypt_aes_gcm()` with a live AES-256-GCM encrypt/decrypt round-trip — no external network requests required.
