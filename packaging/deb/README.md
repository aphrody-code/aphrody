# aphrody — Debian / Ubuntu packaging

Target platform: **Ubuntu 26.04 LTS** (amd64 primary, arm64 secondary).

## Files in this directory

| File | Purpose |
|---|---|
| `control.template` | Debian control file template; CI substitutes `__VERSION__` |
| `aphrody.postinst` | Post-install hook: prints usage hint to stdout |
| `aphrody.prerm` | Pre-remove hook: exits 0 (no services to stop) |
| `cargo-deb-snippet.toml` | Paste into `crates/cli/Cargo.toml` to drive `cargo deb` |

## Install cargo-deb

```sh
cargo install cargo-deb
```

## Produce the .deb

From the repository root, after a successful `cargo build --release -p aphrody`:

```sh
cargo deb -p aphrody
```

For arm64 (requires a cross-linker such as `gcc-aarch64-linux-gnu`):

```sh
cargo deb -p aphrody --target aarch64-unknown-linux-gnu
```

The output lands at:

```
target/debian/aphrody_<VERSION>_amd64.deb   # or arm64
```

## Install on the target machine

```sh
sudo dpkg -i aphrody_*.deb
sudo apt -f install        # resolves any missing deps (libc6, libssl3)
```

Verify:

```sh
aphrody --help
```

## Static musl variant

If a fully-static binary is built against `x86_64-unknown-linux-musl` with
OpenSSL vendored (`OPENSSL_STATIC=1`), produce a separate package named
`aphrody-musl` with `Depends: libc6` only (libssl3 is statically linked).
Add a `[package.metadata.deb.variants.musl]` section in Cargo.toml — see
`cargo-deb-snippet.toml` for the hook point.

## PPA / APT repository

Uploading to a Launchpad PPA or a self-hosted APT repository (e.g. via
`reprepro`) is **out of scope for this commit**. The CI pipeline will handle
signing and publishing to `deb.aphrody-code.dev` in a follow-up. The
`control.template` and hook scripts here are the inputs that pipeline will
consume.
