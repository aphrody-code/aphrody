# Intégration Depot_Tools & ChromeOS GN

## Depot Tools
`depot_tools` est l'arsenal de développement de Google (Gclient, GN, Ninja, Vpython). Il télécharge automatiquement les bibliothèques C++ de Google (V8, WebRTC, Skia) et un toolchain (Clang/LLVM) sur-mesure.

Le fichier `.gclient` devra cibler ChromeOS explicitement :
```python
solutions = [
  {
    "url": "https://chromium.googlesource.com/chromium/src.git",
    "name": "src",
  },
]
target_os = ['chromeos']
```

## GN (Generate Ninja) Configuration
L'objectif est d'utiliser le fork privé `aphrody-code/depot_tools` (invoqué via `google-os.dll`).
La commande `gn gen out/ChromeOS --args='target_os="chromeos" is_chromeos_device=false use_remoteexec=false'` indique au build engine :
1. `target_os="chromeos"` : Oblige GN à compiler avec le SDK ChromeOS (Unix Only).
2. `is_chromeos_device=false` : Permet l'exécution du binaire sur l'environnement hôte (qui sera notre terminal google-os).
3. `use_remoteexec=false` : Désactive Reclient, forçant la compilation en local sur la machine Windows via Ninja.

## Ozone & VcXsrv (Serveur X11 Local)
Parce que Windows 11 ne supporte pas l'affichage direct d'un serveur Wayland ou X11 interne de base, nous devons compiler avec le *backend Ozone* de Google ou invoquer le binaire en transmettant l'affichage via la variable d'environnement `DISPLAY=:0`. Un serveur Windows X (tel que `VcXsrv`) affichera l'UI Linux native.
