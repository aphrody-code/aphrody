# WinGet — Package Management pour Aphrody

> Documentation complète de l'intégration WinGet dans le projet `aphrody`.
> Dernière mise à jour : 2026-05-15.

---

## Table des matières

1. [Vue d'ensemble](#1-vue-densemble)
2. [Architecture WinGet](#2-architecture-winget)
3. [Quickstart — Bootstrapper un poste](#3-quickstart--bootstrapper-un-poste)
4. [Catalogue Google complet](#4-catalogue-google-complet)
5. [Fichier de configuration DSC](#5-fichier-de-configuration-dsc)
6. [Commandes de référence](#6-commandes-de-référence)
7. [Inventaire local](#7-inventaire-local)
8. [Schémas JSON / YAML](#8-schémas-json--yaml)
9. [Intégration CI/CD](#9-intégration-cicd)
10. [Dépannage](#10-dépannage)

---

## 1. Vue d'ensemble

WinGet (Windows Package Manager) est utilisé dans `aphrody` comme **gestionnaire
de packages système** pour trois usages :

| Usage | Outil | Fichier |
|---|---|---|
| **Environnement déclaratif** | `winget configure` | [`.config/configuration.winget`](../../.config/configuration.winget) |
| **Export / Import d'env** | `winget export` / `winget import` | JSON ad-hoc |
| **Inventaire centralisé** | `google.json` | [`google.json`](../../google.json) |

### Pourquoi WinGet et pas Chocolatey / Scoop ?

- **Natif Windows** — préinstallé depuis Windows 11 22H2, pas de bootstrap.
- **Source officielle** — les packages Google sont publiés par Google eux-mêmes
  dans le [winget-pkgs](https://github.com/microsoft/winget-pkgs) community repo.
- **DSC (Desired State Configuration)** — le fichier `.winget` est déclaratif,
  idempotent, et intégrable dans une CI.
- **Aucune dépendance externe** — pas de PowerShell Gallery, pas de binaire tiers.

---

## 2. Architecture WinGet

```
aphrody/
├── .config/
│   └── configuration.winget    ← Fichier DSC (24 packages déclarés)
├── google.json                 ← Inventaire + config projet (schema 2.0)
└── docs/winget/
    ├── README.md               ← Ce fichier
    ├── CATALOG.md              ← Catalogue exhaustif Google × WinGet
    └── CHEATSHEET.md           ← Aide-mémoire commandes
```

### Flux de données

```
┌──────────────────────┐     winget configure      ┌──────────────────────┐
│  .config/            │ ──────────────────────────►│  Machine locale      │
│  configuration.winget│     (idempotent)           │  (packages installés)│
└──────────────────────┘                            └──────────┬───────────┘
                                                               │
                                                    winget export -o
                                                               │
                                                               ▼
┌──────────────────────┐     lecture manuelle       ┌──────────────────────┐
│  google.json         │ ◄─────────────────────────│  export.json         │
│  (inventaire projet) │                            │  (snapshot machine)  │
└──────────────────────┘                            └──────────────────────┘
```

---

## 3. Quickstart — Bootstrapper un poste

### Méthode 1 : Configuration DSC (recommandée)

```powershell
# Cloner le repo
git clone https://github.com/nicmusic/aphrody.git
cd aphrody

# Appliquer la configuration déclarative (installe les 24 packages)
winget configure .config/configuration.winget
```

> **Note :** La première exécution peut prendre 10-15 minutes (téléchargement
> de Dart SDK ~200 MB, Cloud SDK, etc.). Les exécutions suivantes sont
> quasi-instantanées (idempotent).

### Méthode 2 : Installation manuelle sélective

```powershell
# Google Core
winget install Google.Chrome.EXE Google.CloudSDK Google.WorkspaceCLI Google.PlatformTools

# Google Dev Libraries
winget install Google.DartSDK Google.FirebaseCLI Google.Protobuf Google.flatbuffers
winget install Google.Libwebp Google.Brotli Google.OSVScanner Google.Perfetto

# Toolchain (non-Google)
winget install Rustlang.Rustup Oven-sh.Bun Kitware.CMake LLVM.LLVM zig.zig
winget install astral-sh.uv Microsoft.Git Mozilla.sccache Ninja-build.Ninja
```

### Méthode 3 : Import JSON

```powershell
# Exporter l'état actuel d'une machine de référence
winget export -o google-cli-env.json --source winget

# Importer sur une nouvelle machine
winget import -i google-cli-env.json --accept-package-agreements
```

---

## 4. Catalogue Google complet

### 4.1 Packages installés — Core

| ID WinGet | Nom | Version | CLIs fournis |
|---|---|---|---|
| `Google.Chrome.EXE` | Google Chrome | 148.0.7778.168 | `chrome` |
| `Google.CloudSDK` | Google Cloud SDK | 568.0.0 | `gcloud`, `gsutil`, `bq` |
| `Google.WorkspaceCLI` | Workspace CLI | 0.22.5 | `workspace` |
| `Google.PlatformTools` | Android Platform-Tools | 37.0.0 | `adb`, `fastboot` |
| `Google.IAPDesktop` | IAP Desktop | 2.49.1797 | GUI |
| `Google.Antigravity` | Antigravity | 1.23.2 | — |

### 4.2 Packages installés — Dev Libraries

| ID WinGet | Nom | Version | CLIs fournis |
|---|---|---|---|
| `Google.DartSDK` | Dart SDK | 3.11.6 | `dart`, `dartaotruntime` |
| `Google.FirebaseCLI` | Firebase CLI | 20.18.2 | `firebase` |
| `Google.Protobuf` | Protocol Buffers | 34.1 | `protoc` |
| `Google.flatbuffers` | FlatBuffers | 25.12.19 | `flatc` |
| `Google.Libwebp` | libwebp | 1.6.0 | `cwebp`, `dwebp`, `gif2webp`, `img2webp`, `vwebp`, `webpmux` |
| `Google.Brotli` | Brotli | 1.2.0 | `brotli` |
| `Google.OSVScanner` | OSV Scanner | 2.3.8 | `osv-scanner` |
| `Google.Perfetto` | Perfetto | 55.1 | `perfetto` |
| `Google.Magika` | Magika | 1.1.0 | `magika` (via pip) |

### 4.3 Packages optionnels (non installés)

| ID WinGet | Nom | Dernière version | Intérêt pour le projet |
|---|---|---|---|
| `Google.AndroidStudio` | Android Studio | 2025.3.4.7 | IDE Android (installé hors winget) |
| `Google.AndroidStudio.Canary` | AS Canary | 2026.1.1.5 | Preview features |
| `Google.AndroidCLI` | Android CLI | 0.7.15411012 | Automatisation Android |
| `Google.AndroidGPUInspector` | GPU Inspector | 3.3.3 | Profilage GPU Android |
| `Google.Chrome.Canary` | Chrome Canary | 150.0.7839.0 | Test navigateur cutting-edge |
| `Google.Chrome.Dev` | Chrome Dev | 150.0.7838.0 | Test APIs expérimentales |
| `Google.ChromeRemoteDesktopHost` | Chrome Remote Desktop | 148.0.7778.23 | Accès distant |
| `Google.EarthPro` | Earth Pro | 7.3.7.1155 | Visualisation géospatiale |
| `Google.GoogleDrive` | Google Drive | 125.0.0.0 | Sync fichiers |
| `Google.GoogleWebDesigner` | Web Designer | 14.2.4.0 | HTML5 / bannières |
| `Google.ContainerTools.Skaffold` | Skaffold | 2.18.2 | Dev Kubernetes |
| `Google.UIforETW` | UIforETW | 1.59 | Profilage ETW Windows |
| `Google.GoogleUpdater` | Google Update | 149.0.7814.0 | Auto-update daemon |

### 4.4 Tout le catalogue `Google.*` sur winget

```powershell
# Lister tous les packages Google disponibles
winget search --source winget --id Google

# Lister avec détails
winget show Google.CloudSDK --source winget
```

---

## 5. Fichier de configuration DSC

### Emplacement

```
.config/configuration.winget
```

### Schéma

```yaml
# yaml-language-server: $schema=https://aka.ms/configuration-dsc-schema/0.2
```

### Structure

Le fichier DSC est divisé en 3 sections :

1. **`assertions`** — Préconditions (OS minimum Windows 11 22H2)
2. **`resources` — Google Core** — 6 packages Google essentiels
3. **`resources` — Google Dev** — 8 bibliothèques de développement
4. **`resources` — Toolchain** — 9 outils non-Google (Rust, Bun, CMake, etc.)

### Syntaxe d'un package

```yaml
- resource: Microsoft.WinGet.DSC/WinGetPackage
  id: protobuf                    # ID unique dans le fichier (pour dependsOn)
  dependsOn:                      # Optionnel : dépendances séquentielles
    - cmake
  directives:
    description: Protocol Buffers compiler (protoc)
  settings:
    id: Google.Protobuf            # ID winget officiel
    source: winget                 # Source (winget | msstore)
    version: "34.1"                # Optionnel : version pinée
```

### Ajouter un package

1. Trouver l'ID : `winget search <nom> --source winget`
2. Ajouter le bloc YAML dans `.config/configuration.winget`
3. Ajouter l'entrée dans `google.json` → `winget_packages`
4. Tester : `winget configure .config/configuration.winget`

---

## 6. Commandes de référence

### Installation

```powershell
# Installer un package
winget install Google.Protobuf

# Installer silencieusement (CI)
winget install Google.Protobuf --silent --accept-source-agreements --accept-package-agreements

# Installer une version spécifique
winget install Google.DartSDK --version 3.11.6

# Installer plusieurs packages
winget install Google.DartSDK Google.FirebaseCLI Google.Protobuf
```

### Mise à jour

```powershell
# Vérifier les mises à jour disponibles
winget upgrade --source winget

# Mettre à jour un package
winget upgrade Google.CloudSDK

# Mettre à jour tous les packages Google
winget upgrade --all --source winget --include-unknown
```

### Gestion

```powershell
# Lister les packages installés
winget list --source winget

# Lister uniquement les packages Google
winget list --source winget --name Google

# Informations détaillées
winget show Google.CloudSDK --source winget

# Désinstaller
winget uninstall Google.IAPDesktop
```

### Export / Import

```powershell
# Exporter l'état complet de la machine
winget export -o env-snapshot.json --source winget

# Importer sur une autre machine
winget import -i env-snapshot.json --accept-package-agreements

# Appliquer la configuration DSC du projet
winget configure .config/configuration.winget
```

### Recherche

```powershell
# Rechercher par nom
winget search protobuf --source winget

# Rechercher par ID (préfixe)
winget search --id Google --source winget

# Rechercher avec filtre de tag
winget search --tag cli --source winget
```

---

## 7. Inventaire local

### Emplacements Google sur cette machine

| Composant | Chemin | Type |
|---|---|---|
| **Google Chrome** | `C:\Program Files\Google\Chrome` | Navigateur |
| **Chrome for Testing** | `%LOCALAPPDATA%\Google\Chrome for Testing` | Test automation |
| **Google Cloud SDK** | `C:\Program Files (x86)\Google\Cloud SDK` | CLI cloud |
| **Android Studio** | `C:\Program Files\Android\Android Studio` | IDE |
| **Android SDK** | `%LOCALAPPDATA%\Android\Sdk` | SDK complet |
| **IAP Desktop** | `%APPDATA%\Google\IAP Desktop` | Config GCP |
| **Google VSCode Extension** | `%LOCALAPPDATA%\google-vscode-extension` | Extension éditeur |

### Composants Android SDK

```
%LOCALAPPDATA%\Android\Sdk\
├── build-tools/        # Outils de compilation (aapt2, d8, etc.)
├── cmake/              # CMake embarqué pour NDK
├── cmdline-tools/      # sdkmanager, avdmanager
│   └── latest/bin/
├── emulator/           # Émulateur Android
├── extras/             # Bibliothèques supplémentaires
├── licenses/           # Acceptation de licences
├── ndk/                # Native Development Kit (C/C++ Android)
├── platform-tools/     # adb, fastboot
├── platforms/          # android.jar par API level
├── sources/            # Sources Java du framework
└── system-images/      # Images système pour émulateur
```

### Entrées PATH Google / Android

```
C:\Program Files\Google\Chrome\Application
C:\Program Files (x86)\Google\Cloud SDK\google-cloud-sdk\bin
%LOCALAPPDATA%\Android\Sdk\platform-tools
%LOCALAPPDATA%\Android\Sdk\cmdline-tools\latest\bin
%LOCALAPPDATA%\Android\Sdk\emulator
%LOCALAPPDATA%\Android\Sdk\tools
%LOCALAPPDATA%\Android\Sdk\tools\bin
```

### CLIs disponibles après installation

| Commande | Source | Usage |
|---|---|---|
| `gcloud` | Cloud SDK | Google Cloud Platform |
| `gsutil` | Cloud SDK | Google Cloud Storage |
| `bq` | Cloud SDK | BigQuery |
| `firebase` | Firebase CLI | Firebase / Firestore |
| `dart` | Dart SDK | Langage Dart |
| `adb` | Platform-Tools | Android Debug Bridge |
| `fastboot` | Platform-Tools | Flash Android |
| `emulator` | Android SDK | Émulateur Android |
| `sdkmanager` | cmdline-tools | Gestion composants SDK |
| `avdmanager` | cmdline-tools | Gestion AVDs |
| `protoc` | Protobuf | Compilation .proto |
| `flatc` | FlatBuffers | Compilation .fbs |
| `cwebp` | libwebp | Encodage WebP |
| `dwebp` | libwebp | Décodage WebP |
| `brotli` | Brotli | Compression Brotli |
| `osv-scanner` | OSV Scanner | Audit vulnérabilités |
| `perfetto` | Perfetto | Tracing système |
| `magika` | Magika (pip) | Détection type fichier |

---

## 8. Schémas JSON / YAML

### 8.1 Schéma `winget export` (JSON)

Utilisé par `winget export -o file.json` et `winget import -i file.json`.

```
$schema: https://aka.ms/winget-packages.schema.2.0.json
```

Structure :

```json
{
  "$schema": "https://aka.ms/winget-packages.schema.2.0.json",
  "CreationDate": "2026-05-15T...",
  "Sources": [
    {
      "Packages": [
        { "PackageIdentifier": "Google.Chrome.EXE" },
        { "PackageIdentifier": "Google.CloudSDK" }
      ],
      "SourceDetails": {
        "Argument": "https://cdn.winget.microsoft.com/cache",
        "Identifier": "Microsoft.Winget.Source_8wekyb3d8bbwe",
        "Name": "winget",
        "Type": "Microsoft.PreIndexed.Package"
      }
    }
  ],
  "WinGetVersion": "1.29.140-preview"
}
```

### 8.2 Schéma WinGet Configuration DSC (YAML)

Utilisé par `winget configure file.winget`.

```
$schema: https://aka.ms/configuration-dsc-schema/0.2
```

Structure :

```yaml
properties:
  configurationVersion: 0.2.0

  assertions:
    - resource: Microsoft.Windows.Developer/OsVersion
      id: osCheck
      settings:
        MinVersion: "10.0.22621"

  resources:
    - resource: Microsoft.WinGet.DSC/WinGetPackage
      id: monPackage
      dependsOn: [autrePackage]      # séquencement
      directives:
        description: Description humaine
      settings:
        id: Vendor.PackageName       # ID winget officiel
        source: winget               # winget | msstore
        version: "1.0.0"             # optionnel
```

### 8.3 Schéma Manifest WinGet (YAML — pour publier un package)

Utilisé pour soumettre un package au [winget-pkgs](https://github.com/microsoft/winget-pkgs).

```
$schema: https://aka.ms/winget-manifest.version.1.9.0.schema.json
$schema: https://aka.ms/winget-manifest.installer.1.9.0.schema.json
$schema: https://aka.ms/winget-manifest.defaultLocale.1.9.0.schema.json
```

Structure multi-fichiers :

```
manifests/g/Google/Protobuf/34.1/
├── Google.Protobuf.yaml                    # version manifest
├── Google.Protobuf.installer.yaml          # installer manifest
└── Google.Protobuf.locale.en-US.yaml       # locale manifest
```

---

## 9. Intégration CI/CD

### GitHub Actions

```yaml
# .github/workflows/setup.yml
name: Setup Dev Environment
on: workflow_dispatch

jobs:
  setup:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Apply WinGet Configuration
        run: winget configure .config/configuration.winget --accept-configuration-agreements

      - name: Verify Google Tools
        run: |
          gcloud --version
          dart --version
          protoc --version
          firebase --version
```

### Script PowerShell autonome

```powershell
# scripts/setup-env.ps1
#Requires -RunAsAdministrator

Write-Host "🔧 Configuration de l'environnement aphrody..." -ForegroundColor Cyan

# Méthode 1 : DSC (idéale)
if (Get-Command winget -ErrorAction SilentlyContinue) {
    winget configure "$PSScriptRoot/../.config/configuration.winget" `
        --accept-configuration-agreements
}

# Méthode 2 : Fallback package par package
else {
    $packages = @(
        'Google.Chrome.EXE', 'Google.CloudSDK', 'Google.DartSDK',
        'Google.FirebaseCLI', 'Google.Protobuf', 'Google.flatbuffers',
        'Google.Libwebp', 'Google.Brotli', 'Google.OSVScanner',
        'Google.Perfetto', 'Rustlang.Rustup', 'Oven-sh.Bun',
        'Kitware.CMake', 'LLVM.LLVM', 'zig.zig'
    )
    foreach ($pkg in $packages) {
        winget install $pkg --silent --accept-source-agreements --accept-package-agreements
    }
}

Write-Host "✅ Environnement prêt. Redémarrez votre shell." -ForegroundColor Green
```

---

## 10. Dépannage

### « Le package installé n'est pas disponible à partir d'une source »

Certains packages (Android Studio, Bun) sont installés via leur propre
installeur et ne sont pas liés à la source winget. C'est normal.

```powershell
# Voir tous les packages, même sans source
winget list --name Google
```

### Version de winget trop ancienne

```powershell
# Vérifier la version
winget --version
# Minimum requis : 1.6+ pour DSC, 1.7+ pour configure
# Mettre à jour
winget upgrade Microsoft.AppInstaller --source winget
```

### PATH non mis à jour après installation

WinGet modifie le PATH système mais le shell actuel ne le voit pas.

```powershell
# Solution 1 : Redémarrer le terminal
# Solution 2 : Recharger le PATH
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
```

### Conflit de versions

```powershell
# Forcer une version spécifique
winget install Google.DartSDK --version 3.11.6 --force

# Désinstaller puis réinstaller
winget uninstall Google.DartSDK
winget install Google.DartSDK --version 3.11.6
```

### Vérifier l'intégrité de l'installation

```powershell
# Vérifier que tous les CLIs sont accessibles
$tools = @('gcloud','dart','firebase','protoc','flatc','cwebp','brotli','osv-scanner','perfetto','adb')
foreach ($t in $tools) {
    $cmd = Get-Command $t -ErrorAction SilentlyContinue
    if ($cmd) { Write-Host "✅ $t → $($cmd.Source)" -ForegroundColor Green }
    else { Write-Host "❌ $t non trouvé" -ForegroundColor Red }
}
```

---

## Fichiers associés

| Fichier | Rôle |
|---|---|
| [`.config/configuration.winget`](../../.config/configuration.winget) | Configuration DSC déclarative (24 packages) |
| [`google.json`](../../google.json) | Inventaire centralisé + config projet |
| [`docs/winget/CATALOG.md`](./CATALOG.md) | Catalogue exhaustif des 40+ packages Google |
| [`docs/winget/CHEATSHEET.md`](./CHEATSHEET.md) | Aide-mémoire commandes rapides |

---

*Licence Apache 2.0 — voir [LICENSE](../../LICENSE).*
