# aphrody-bin (AUR)

`aphrody-bin` is the pre-built binary variant of [`aphrody`](../arch/) for Arch
Linux users who want a fast install without pulling the Rust nightly toolchain
and the full workspace dependency graph.

## Install

```bash
yay -S aphrody-bin     # pre-built binary (this package, fast)
yay -S aphrody         # source build (../arch/, full rebuild)
```

The `provides=('aphrody')` and `conflicts=('aphrody')` directives ensure the
two cannot coexist: pick one. Anything that depends on `aphrody` will accept
either.

## Maintenance

The maintainer (`aphrody-code`) submits both packages to the AUR after every
release tag by pushing to `ssh://aur@aur.archlinux.org/aphrody-bin.git`. The
`sha256sums_*` fields are switched from `SKIP` to the real release-asset
digests at tag time.
