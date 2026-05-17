# PowerShell 7 — Documentation pour Aphrody

> Configuration complète de l'environnement PowerShell 7 utilisé par le projet `aphrody`.
> Dernière mise à jour : 2026-05-15.

---

## Table des matières

1. [Vue d'ensemble](#1-vue-densemble)
2. [Installation & Versions](#2-installation--versions)
3. [Profils](#3-profils)
4. [Modules installés](#4-modules-installés)
5. [PSReadLine & Prédiction](#5-psreadline--prédiction)
6. [Intégration Aphrody](#6-intégration-google-cli)
7. [Scripts du projet](#7-scripts-du-projet)
8. [Configuration de sécurité](#8-configuration-de-sécurité)
9. [Bonnes pratiques](#9-bonnes-pratiques)
10. [Cheatsheet](#10-cheatsheet)
11. [Dépannage](#11-dépannage)

---

## 1. Vue d'ensemble

Le projet `aphrody` utilise **PowerShell 7 (pwsh)** comme shell principal pour :

| Usage | Contexte |
|---|---|
| **Build automation** | Scripts de build, orchestration CMake, sccache |
| **WinGet management** | `winget configure`, export/import d'environnement |
| **Toolchain scripting** | Lancement de `cargo`, `bun`, `zig`, `gcloud` |
| **System analysis** | Modules WinClean, analyse Chromium, ETW tracing |
| **AI collaboration** | Wrapper Claude Code, MCP server orchestration |

### Pourquoi pwsh 7 et pas PowerShell 5.1 ?

| Critère | PowerShell 5.1 | PowerShell 7 (pwsh) |
|---|---|---|
| Édition | Windows PowerShell | PowerShell Core |
| .NET | .NET Framework 4.x | .NET 9+ |
| Cross-platform | ❌ Windows only | ✅ Windows, Linux, macOS |
| Performance | Baseline | 2-4x plus rapide |
| JSON natif | Basique | Complet (depth, compress) |
| Pipeline parallèle | ❌ | ✅ `ForEach-Object -Parallel` |
| Ternaire | ❌ | ✅ `$a ? $b : $c` |
| Null-coalescing | ❌ | ✅ `$a ?? $b` |
| PSResourceGet | ❌ | ✅ Natif |

---

## 2. Installation & Versions

### Versions installées

| Exécutable | Chemin | Version | Édition |
|---|---|---|---|
| `pwsh` | `C:\Program Files\PowerShell\7\pwsh.exe` | **7.6.1** | Core |
| `powershell` | `C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe` | 5.1 (10.0.28000.4) | Desktop |

### OS

```
Microsoft Windows 10.0.28020 (Windows 11 Insider)
```

### Installer / Mettre à jour pwsh

```powershell
# Via winget (recommandé)
winget install Microsoft.PowerShell --source winget

# Via MSI
# https://github.com/PowerShell/PowerShell/releases

# Vérifier la version
pwsh --version
```

### Statistiques du shell

| Métrique | Valeur |
|---|---|
| Aliases | 135 |
| Functions | 4 177 |
| Cmdlets | 4 462 |
| Modules disponibles | ~130 |
| PSGallery | Trusted |

---

## 3. Profils

PowerShell charge des scripts de profil au démarrage. 4 emplacements possibles,
du plus global au plus spécifique :

| Profil | Chemin | Existe | Priorité |
|---|---|---|---|
| AllUsersAllHosts | `C:\Program Files\PowerShell\7\profile.ps1` | ❌ | 1 (le plus global) |
| AllUsersCurrentHost | `C:\Program Files\PowerShell\7\Microsoft.PowerShell_profile.ps1` | ❌ | 2 |
| **CurrentUserAllHosts** | `~\Documents\PowerShell\profile.ps1` | ✅ | 3 |
| **CurrentUserCurrentHost** | `~\Documents\PowerShell\Microsoft.PowerShell_profile.ps1` | ✅ | 4 (le plus spécifique) |

### `profile.ps1` (CurrentUserAllHosts)

Ce profil s'exécute pour **tous les hôtes** (pwsh, VS Code terminal, Windows Terminal, etc.).

```powershell
# C:\usr prefix (Unix-like local install tree)
$env:Path = "C:\usr\bin;" + $env:Path
$env:LIB = "C:\usr\lib;" + $env:LIB
$env:INCLUDE = "C:\usr\include;C:\usr\include\cairo;C:\usr\include\glib-2.0;..." + $env:INCLUDE
```

**Rôle** : Configure un arbre d'installation Unix-like (`C:\usr\`) pour les
bibliothèques C/C++ natives (cairo, glib, gobject-introspection). Utilisé par
le pipeline CMake de `aphrody`.

### `Microsoft.PowerShell_profile.ps1` (CurrentUserCurrentHost)

```powershell
# 1. Safe startup directory
if ((Get-Location).Path -match '^[A-Za-z]:\\?$|System32') {
    Set-Location $HOME
}

# 2. Claude Code wrapper
function claude {
    $here = (Get-Location).Path
    $safe = $here -match '^[A-Za-z]:\\?$'
    if ($safe) { Push-Location $HOME }
    try   { & "$HOME\.local\bin\claude.exe" @args }
    finally { if ($safe) { Pop-Location } }
}

# 3. PSReadLine (deferred loading)
if ([Environment]::UserInteractive -and -not [Console]::IsInputRedirected) {
    Set-PSReadLineKeyHandler -Key Tab       -Function MenuComplete
    Set-PSReadLineKeyHandler -Key UpArrow   -Function HistorySearchBackward
    Set-PSReadLineKeyHandler -Key DownArrow -Function HistorySearchForward

    $null = Register-EngineEvent -SourceIdentifier PowerShell.OnIdle -MaxTriggerCount 1 -Action {
        Set-PSReadLineOption -PredictionSource HistoryAndPlugin -PredictionViewStyle InlineView
    }
}

# 4. Awesome CLI modules
Import-Module TabExpansionPlusPlus
Import-Module PSUtil
Import-Module posh-with

# 5. PSReadLine overrides
Import-Module PSReadLine
Set-PSReadLineOption -PredictionSource HistoryAndPlugin
Set-PSReadLineOption -PredictionViewStyle ListView

# 6. WinClean module path
$env:PSModulePath = $env:PSModulePath + ";C:\winclean\modules"
```

---

## 4. Modules installés

### Modules utilisateur (PSGallery / manuels)

| Module | Version | Usage dans le projet |
|---|---|---|
| **Az** | 15.1.0 | Azure PowerShell (100+ sous-modules) |
| **BurntToast** | 1.1.0 | Notifications Windows toast |
| **CompletionPredictor** | 0.1.1 | Auto-complétion prédictive |
| **Ghidra** | 0.1.0 | Intégration NSA Ghidra (reverse) |
| **ImportExcel** | 7.8.10 | Lecture/écriture Excel sans Office |
| **Microsoft.PowerShell.ConsoleGuiTools** | 0.7.7 | `Out-ConsoleGridView` (TUI) |
| **Microsoft.WinGet.Client** | 1.12.440 | API WinGet native PowerShell |
| **Microsoft.WinGet.CommandNotFound** | 1.0.6.0 | Suggestion de packages winget |
| **oh-my-posh** | 7.85.2 | Prompt thémé (Nerd Fonts) |
| **posh-git** | 1.1.0 | Intégration Git dans le prompt |
| **posh-with** | 1.0.2 | Préfixe de commande persistant |
| **PowerHTML** | 0.2.0 | Parsing HTML (HtmlAgilityPack) |
| **powershell-yaml** | 0.4.12 | Sérialisation YAML |
| **ps2exe** | 1.0.17 | Compilation script → .exe |
| **PSFzf** | 2.7.10 | Intégration fzf (fuzzy finder) |
| **PSFramework** | 1.13.426 | Framework de scripting avancé |
| **psInlineProgress** | 1.1 | Barres de progression inline |
| **PSnmap** | 1.3.2 | Port scanning réseau |
| **psprivilege** | 0.2.0 | Gestion de privilèges Windows |
| **PSReadLine** | 2.4.5 | Édition de ligne avancée |
| **PSScriptAnalyzer** | 1.25.0 | Linting de scripts PowerShell |
| **PSUtil** | 2.2.39 | Utilitaires de pipeline |
| **PSWindowsUpdate** | 2.2.1.5 | Gestion Windows Update |
| **PSWriteHTML** | 1.40.0 | Génération de rapports HTML |
| **SpeculationControl** | 1.0.19 | Audit Spectre/Meltdown |
| **string** | 1.2.13 | Manipulation de chaînes avancée |
| **TabExpansionPlusPlus** | 1.2 | Tab-completion enrichie |
| **Terminal-Icons** | 0.11.0 | Icônes de fichiers dans `ls` |
| **ZLocation** | 1.4.3 | Navigation rapide (`z`) |

### Modules système (Windows intégrés)

> ~50 modules Windows intégrés : `NetAdapter`, `Storage`, `Dism`,
> `ScheduledTasks`, `BitLocker`, `NetSecurity`, `SmbShare`, etc.
> Disponibles via `Get-Module -ListAvailable`.

### Modules PowerShell 7 (built-in)

| Module | Version |
|---|---|
| CimCmdlets | 7.0.0.0 |
| Microsoft.PowerShell.Archive | 1.2.5 |
| Microsoft.PowerShell.Diagnostics | 7.0.0.0 |
| Microsoft.PowerShell.Host | 7.0.0.0 |
| Microsoft.PowerShell.Management | 7.0.0.0 |
| Microsoft.PowerShell.PSResourceGet | 1.2.0 |
| Microsoft.PowerShell.Security | 7.0.0.0 |
| Microsoft.PowerShell.ThreadJob | 2.2.0 |
| Microsoft.PowerShell.Utility | 7.0.0.0 |
| Microsoft.WSMan.Management | 7.0.0.0 |
| PackageManagement | 1.4.8.1 |
| PowerShellGet | 2.2.5 |
| PSDiagnostics | 7.0.0.0 |
| PSReadLine | 2.4.5 |

---

## 5. PSReadLine & Prédiction

### Configuration actuelle

| Option | Valeur |
|---|---|
| EditMode | Windows |
| PredictionSource | HistoryAndPlugin |
| PredictionViewStyle | ListView |
| ShowToolTips | True |
| BellStyle | Audible |

### Raccourcis clavier configurés

| Touche | Action |
|---|---|
| `Tab` | `MenuComplete` (menu interactif de complétion) |
| `↑` | `HistorySearchBackward` (recherche dans l'historique) |
| `↓` | `HistorySearchForward` |

### Predictors installés

1. **History** — Suggère des commandes basées sur l'historique
2. **CompletionPredictor** — Plugin de prédiction par complétion
3. **WinGet CommandNotFound** — Suggère `winget install` pour les commandes inconnues

### Personnaliser PSReadLine

```powershell
# Changer le style de prédiction
Set-PSReadLineOption -PredictionViewStyle InlineView   # ou ListView

# Ajouter un raccourci
Set-PSReadLineKeyHandler -Key Ctrl+d -Function DeleteCharOrExit

# Voir tous les raccourcis
Get-PSReadLineKeyHandler

# Historique
Get-PSReadLineOption | Select-Object HistorySearchCursorMovesToEnd, MaximumHistoryCount
```

---

## 6. Intégration Aphrody

### Wrapper Claude Code

Le profil définit une fonction `claude` qui :
1. Détecte si le répertoire courant est une racine de lecteur (dangereux)
2. Si oui, bascule temporairement vers `$HOME`
3. Lance `claude.exe` avec tous les arguments passés
4. Restaure le répertoire original

```powershell
claude "fix the build"           # depuis n'importe quel répertoire
claude --model opus "refactor"   # arguments transparents
```

### Module WinClean

Le `PSModulePath` est étendu avec `C:\winclean\modules` pour charger
les modules WinClean custom (MCP server, system analysis, etc.).

### Variables d'environnement C/C++

Le profil `profile.ps1` configure un arbre `C:\usr\` compatible avec le
pipeline CMake de `aphrody` :

```
C:\usr\bin       → binaires natifs (cairo, glib, etc.)
C:\usr\lib       → bibliothèques .lib/.a
C:\usr\include   → headers (cairo, glib-2.0, gobject-introspection)
```

### Commandes Google disponibles dans pwsh

```powershell
gcloud info                      # Google Cloud SDK
gsutil ls gs://bucket            # Cloud Storage
bq query "SELECT 1"             # BigQuery
firebase projects:list           # Firebase
dart --version                   # Dart SDK
adb devices                      # Android Debug Bridge
protoc --version                 # Protocol Buffers
flatc --version                  # FlatBuffers
cwebp -h                        # WebP encoder
osv-scanner --version            # Vulnerability scanner
perfetto --version               # Tracing toolkit
```

---

## 7. Scripts du projet

### Convention de nommage

| Pattern | Usage |
|---|---|
| `Verb-Noun.ps1` | Scripts standards (ex: `Invoke-ChromeAutopsy.ps1`) |
| `*.Tests.ps1` | Tests Pester |
| `profile.ps1` | Profils de session |

### Scripts existants dans le repo

```powershell
# Trouver tous les scripts PowerShell du projet
Get-ChildItem -Path C:\src\aphrody -Filter *.ps1 -Recurse |
    Select-Object FullName, Length, LastWriteTime
```

### Template pour un nouveau script

```powershell
#!/usr/bin/env pwsh
# Copyright 2026 Aphrody Authors
# SPDX-License-Identifier: Apache-2.0

#Requires -Version 7.0

<#
.SYNOPSIS
    Brief description.
.DESCRIPTION
    Detailed description.
.PARAMETER Name
    Parameter description.
.EXAMPLE
    PS> .\MyScript.ps1 -Name "value"
.NOTES
    Part of the aphrody project.
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$Name
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# --- Implementation ---
```

---

## 8. Configuration de sécurité

### Execution Policy

| Scope | Policy |
|---|---|
| MachinePolicy | Undefined |
| UserPolicy | Undefined |
| **Process** | **Bypass** |
| **CurrentUser** | **Bypass** |
| LocalMachine | RemoteSigned |

> **Note :** `Bypass` au niveau `CurrentUser` et `Process` permet l'exécution
> sans restriction de tous les scripts. C'est nécessaire pour le workflow
> autonome (Claude Code, WinClean MCP, etc.).

### Modifier l'Execution Policy

```powershell
# Voir la politique actuelle
Get-ExecutionPolicy -List

# Modifier (nécessite admin pour LocalMachine)
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned

# Bypass temporaire (pour un script)
pwsh -ExecutionPolicy Bypass -File script.ps1
```

### PSGallery

| Repository | Policy | Source |
|---|---|---|
| PSGallery | **Trusted** | `https://www.powershellgallery.com/api/v2` |

---

## 9. Bonnes pratiques

### Pour le projet aphrody

1. **Toujours utiliser `pwsh`**, jamais `powershell` (5.1)
2. **`Set-StrictMode -Version Latest`** en haut de chaque script
3. **`$ErrorActionPreference = 'Stop'`** pour fail-fast
4. **Conventional Commits** dans les messages git (en anglais)
5. **PSScriptAnalyzer** pour le linting avant commit

### Commandes à éviter

| ❌ À éviter | ✅ Utiliser |
|---|---|
| `powershell` | `pwsh` |
| `Write-Host` (dans les fonctions) | `Write-Output` ou `Write-Verbose` |
| `Invoke-Expression` | Splatting ou `& $cmd @args` |
| `$global:var` | `$script:var` ou paramètres |
| `ConvertTo-Json` (sans `-Depth`) | `ConvertTo-Json -Depth 10` |

### Linting

```powershell
# Linter un script
Invoke-ScriptAnalyzer -Path .\script.ps1

# Linter tout le projet
Get-ChildItem -Path C:\src\aphrody -Filter *.ps1 -Recurse |
    Invoke-ScriptAnalyzer
```

---

## 10. Cheatsheet

### Navigation

```powershell
z aphrody                     # ZLocation (jump rapide)
cd -                             # Retour au répertoire précédent
with cargo                       # Préfixe toutes les commandes avec "cargo"
```

### Modules

```powershell
Get-Module                       # Modules chargés
Get-Module -ListAvailable        # Tous les modules disponibles
Install-Module Terminal-Icons     # Installer depuis PSGallery
Update-Module Az                  # Mettre à jour
Find-Module *google*             # Rechercher
Import-Module posh-git            # Charger explicitement
```

### Pipeline parallèle (pwsh 7+)

```powershell
# Traitement parallèle de fichiers
Get-ChildItem -Filter *.cpp | ForEach-Object -Parallel {
    clang-format -i $_.FullName
} -ThrottleLimit 8
```

### JSON natif

```powershell
# Lecture
$data = Get-Content google.json | ConvertFrom-Json

# Écriture (avec profondeur)
$data | ConvertTo-Json -Depth 10 | Set-Content google.json

# Pipeline
Invoke-RestMethod https://api.example.com/data | ConvertTo-Json -Compress
```

### Ternaire & Null-coalescing (pwsh 7+)

```powershell
$result = $value ? "found" : "not found"
$name = $env:USER ?? "unknown"
$list ??= @()
```

### WinGet via PowerShell module

```powershell
# Module natif (plus rapide que le CLI)
Get-WinGetPackage -Name Google              # Lister
Install-WinGetPackage -Id Google.DartSDK    # Installer
Update-WinGetPackage -Id Google.CloudSDK    # Mettre à jour
```

---

## 11. Dépannage

### Le profil ne se charge pas

```powershell
# Vérifier le chemin du profil
$PROFILE | Format-List *

# Tester si le profil existe
Test-Path $PROFILE.CurrentUserCurrentHost

# Recharger le profil manuellement
. $PROFILE
```

### Module introuvable

```powershell
# Vérifier le PSModulePath
$env:PSModulePath -split ';'

# Forcer la réinstallation
Install-Module -Name ModuleName -Force -AllowClobber
```

### pwsh vs powershell dans les scripts

```powershell
# Forcer pwsh dans un script
#!/usr/bin/env pwsh

# Vérifier la version dans un script
if ($PSVersionTable.PSVersion.Major -lt 7) {
    throw "Ce script nécessite PowerShell 7+. Utilisez 'pwsh' au lieu de 'powershell'."
}
```

### Encodage UTF-8

```powershell
# Forcer UTF-8 (nécessaire pour les caractères français)
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$PSDefaultParameterValues['Out-File:Encoding'] = 'utf8'
```

### Performance du démarrage

```powershell
# Mesurer le temps de chargement du profil
Measure-Command { pwsh -NoProfile -Command "exit" }     # baseline
Measure-Command { pwsh -Command "exit" }                  # avec profil

# Identifier les modules lents
Trace-Command -Name Modules -Expression { Import-Module PSReadLine } -PSHost
```

---

## Fichiers associés

| Fichier | Rôle |
|---|---|
| `~\Documents\PowerShell\profile.ps1` | Profil AllHosts (PATH C:\usr) |
| `~\Documents\PowerShell\Microsoft.PowerShell_profile.ps1` | Profil pwsh (modules, Claude, PSReadLine) |
| `C:\winclean\modules\` | Modules WinClean custom |
| [docs/pwsh/MODULES.md](./MODULES.md) | Catalogue des modules installés |
| [docs/pwsh/CHEATSHEET.md](./CHEATSHEET.md) | Aide-mémoire commandes |

---

*Licence Apache 2.0 — voir [LICENSE](../../LICENSE).*
