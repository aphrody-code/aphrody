# aphrody XDG `.desktop` entry

`aphrody.desktop` is the [XDG Desktop Entry](https://specifications.freedesktop.org/desktop-entry-spec/latest/)
file that registers the `aphrody` CLI in Linux application launchers (GNOME, KDE,
Xfce, etc.). The `.deb` / `.rpm` packages install it automatically via their
postinst hook; the steps below are for manual installation.

## Manual install (per-user)

```bash
install -D -m 0644 aphrody.desktop \
    ~/.local/share/applications/aphrody.desktop
install -D -m 0644 ../../assets/aphrody-mark.svg \
    ~/.local/share/icons/hicolor/scalable/apps/aphrody.svg
update-desktop-database ~/.local/share/applications/
```

## Manual install (system-wide, requires root)

```bash
sudo install -D -m 0644 aphrody.desktop \
    /usr/share/applications/aphrody.desktop
sudo install -D -m 0644 ../../assets/aphrody-mark.svg \
    /usr/share/icons/hicolor/scalable/apps/aphrody.svg
sudo update-desktop-database /usr/share/applications/
```

The launcher will then expose `aphrody`, plus the `Doctor` and `Version`
right-click actions defined in the entry.
