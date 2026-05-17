# aphrody — RPM packaging (Fedora / RHEL / Rocky / AlmaLinux)

Target platforms: **Fedora 40+**, **RHEL 9 / CentOS Stream 9**, **Rocky Linux 9**,
**AlmaLinux 9**. Architectures: `x86_64`, `aarch64`.

## Install a pre-built RPM

```sh
# Fedora
sudo dnf install ./aphrody-1.0.0~canary-1.fc40.x86_64.rpm

# RHEL / CentOS Stream 9 / Rocky 9 / Alma 9
sudo yum install ./aphrody-1.0.0~canary-1.el9.x86_64.rpm
```

RPM's automatic dependency generator resolves `glibc`, `libssl` and friends
from the ELF binary; no explicit `Requires:` is needed.

## Build locally

```sh
# From a clean source checkout (or unpacked tarball matching Source0):
rpmbuild -ba packaging/rpm/aphrody.spec
```

Output lands under `~/rpmbuild/RPMS/<arch>/` and the SRPM under
`~/rpmbuild/SRPMS/`.

## Hosted builds via Fedora COPR

```sh
copr-cli build aphrody-code/aphrody packaging/rpm/aphrody.spec
```

COPR will build the SRPM for every enabled chroot (Fedora N, Fedora N-1,
EPEL 9, EPEL 10) and publish a yum repository at
`https://copr.fedorainfracloud.org/coprs/aphrody-code/aphrody/`.

## Notes

- Version uses `~canary` (RPM pre-release tilde, equivalent to Debian's `~`),
  so any future stable `1.0.0-1` will correctly outrank this build.
- `%check` runs `aphrody --version` to smoke-test the binary inside the
  buildroot before the RPM is sealed.
- Debuginfo subpackage is disabled (`%global debug_package %{nil}`) because
  release builds are stripped via `RUSTFLAGS=-C strip=symbols`.
