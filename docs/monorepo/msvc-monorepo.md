<!-- SPDX-License-Identifier: Apache-2.0 -->
# MSVC / CMake Monorepo (C++)

Dans l'écosystème C++ et Windows (MSVC), l'approche moderne (2026) pour un monorepo repose sur **CMake** agissant comme le chef d'orchestre global, générant les solutions Visual Studio (`.sln`) ou s'interfaçant avec `Ninja`.

## Architecture CMake

Le principe est d'avoir un `CMakeLists.txt` à la racine qui agit en tant que hub, et d'inclure les sous-projets via `add_subdirectory()`.

```cmake
cmake_minimum_required(VERSION 3.30)
project(GoogleCliNative VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Inclusion des sous-projets
add_subdirectory(lib/forensics)
add_subdirectory(lib/injector)
add_subdirectory(src/daemon)
```

## Gestion des Dépendances Internes

Les bibliothèques (statiques ou partagées) créées dans un `add_subdirectory` sont directement accessibles aux autres répertoires via `target_link_libraries`.

```cmake
# Dans src/daemon/CMakeLists.txt
add_executable(google-daemon main.cpp)
target_link_libraries(google-daemon PRIVATE injector_lib)
```

## Avantages et MSBuild

L'utilisation de CMake permet :
1.  De compiler nativement avec **Ninja** (pour la vitesse) ou **MSBuild** (pour le débogage natif Windows).
2.  D'utiliser **vcpkg** en mode "Manifest" (`vcpkg.json` à la racine) pour gérer les dépendances C++ externes comme un workspace moderne.
3.  Une isolation parfaite des cibles (targets) évitant les conflits d'espaces de noms.
