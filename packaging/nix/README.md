[//]: # (SPDX-License-Identifier: Apache-2.0)

# aphrody — Nix flake

Try aphrody instantly without installing it:

```sh
nix run github:aphrody-code/aphrody?dir=packaging/nix
```

Enter a dev shell with Rust nightly, Bun, cargo-nextest, cargo-deny, and
cargo-zigbuild already available:

```sh
nix develop github:aphrody-code/aphrody?dir=packaging/nix
```

> **Note:** The flake will move to the repository root in a future release,
> allowing the shorter `nix run github:aphrody-code/aphrody` form.
> The `packaging/nix/` path will remain as a redirect alias.
