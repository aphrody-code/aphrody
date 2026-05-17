# MSYS2 Makepkg & Package Spoofing

## Architecture de Compilation MSYS2
MSYS2 utilise l'utilitaire `makepkg-mingw` (issu d'Arch Linux) pour compiler ses packages depuis le code source.

1. **PKGBUILD** : Le fichier manifeste qui définit comment compiler la DLL racine (`msys2-runtime`).
2. **Sources** : MSYS2 clone le code source de Cygwin avec leurs patchs MSYS.

## Le Fork Google-OS
Au lieu de simplement patcher la DLL binaire avec LIEF, nous pouvons aussi builder notre propre version de la DLL directement depuis les sources.

```bash
# Workflow de spoofing
git clone https://github.com/msys2/msys2-runtime.git
cd msys2-runtime
# ... Modification manuelle du fichier source uname.cc pour forcer "Linux" ...
# ... Modification des headers C++ pour renommer la DLL en google-os.dll ...
makepkg-mingw -sLf
```

## L'Écosystème Multi-Gestionnaires
Le système final `google-os` utilise un PATH dynamique qui lie :
- `/usr/bin/pacman` (natif)
- `/usr/bin/bun` (wrapper pointant vers bun.exe)
- `/usr/bin/vcpkg` (wrapper pointant vers vcpkg.exe)
- `/usr/bin/winget` (wrapper pointant vers winget.exe)
- `/opt/depot_tools/` (git cloné)

Chacun de ces utilitaires pourra être invoqué depuis le shell bash avec les privilèges NT absolus accordés par l'entrée de `google-os.dll`.
