# Flatpak packaging — `com.aphrody.aphrody`

Flatpak distribution for `aphrody`, the cross-platform CLI. Targets all major
Linux desktops (Ubuntu, Fedora, openSUSE, Arch, Debian) with sandboxed runtime.

## Install (after Flathub publication)

```bash
flatpak install flathub com.aphrody.aphrody
flatpak run com.aphrody.aphrody --help
```

## Local build (from this repo root)

```bash
# Install the rust-nightly SDK extension once
flatpak install flathub org.freedesktop.Sdk.Extension.rust-nightly//24.08

# Build and install into the user scope
flatpak-builder --user --install --force-clean build-dir \
  packaging/flatpak/com.aphrody.aphrody.json

flatpak run com.aphrody.aphrody --version
```

## Status

This manifest is publishable-ready. Flathub submission is gated by the Flathub
review queue (typically 1–4 weeks). License declaration lives in the sibling
`LICENSE.SPDX` file (JSON spec forbids comments).
