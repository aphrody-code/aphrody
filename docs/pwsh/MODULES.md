# Modules PowerShell — Catalogue complet

> Tous les modules installés dans l'environnement PowerShell 7
> de la machine de développement `aphrody`.
> Snapshot au 2026-05-15.

---

## Légende

| Icône | Source |
|---|---|
| 📦 | PSGallery (installé par l'utilisateur) |
| ⚙️ | PowerShell 7 (built-in) |
| 🪟 | Windows (système intégré) |
| 🔧 | Custom (WinClean / local) |

---

## Modules de productivité shell

| Icône | Module | Version | Description |
|---|---|---|---|
| 📦 | **oh-my-posh** | 7.85.2 | Prompt thémé avec Nerd Fonts (segments git, durée, status) |
| 📦 | **posh-git** | 1.1.0 | Info git dans le prompt (branche, dirty, stash) |
| 📦 | **Terminal-Icons** | 0.11.0 | Icônes de fichiers/dossiers dans `Get-ChildItem` |
| 📦 | **ZLocation** | 1.4.3 | Navigation rapide par fréquence (`z aphrody`) |
| 📦 | **PSFzf** | 2.7.10 | Intégration fzf (fuzzy finder) dans le pipeline |
| 📦 | **posh-with** | 1.0.2 | Préfixe de commande persistant (`with cargo`) |
| 📦 | **TabExpansionPlusPlus** | 1.2 | Tab-completion enrichie (arguments, paramètres) |
| 📦 | **PSUtil** | 2.2.39 | Utilitaires de pipeline (clip, select, etc.) |
| 📦 | **psInlineProgress** | 1.1 | Barres de progression inline |

## PSReadLine & Prédiction

| Icône | Module | Version | Description |
|---|---|---|---|
| ⚙️ | **PSReadLine** | 2.4.5 | Édition de ligne, historique, coloration syntaxique |
| 📦 | **CompletionPredictor** | 0.1.1 | Plugin de prédiction par complétion |
| 📦 | **Microsoft.WinGet.CommandNotFound** | 1.0.6.0 | Suggère `winget install` pour commandes inconnues |

## Scripting & Développement

| Icône | Module | Version | Description |
|---|---|---|---|
| 📦 | **PSScriptAnalyzer** | 1.25.0 | Linting et analyse statique de scripts .ps1 |
| 📦 | **PSFramework** | 1.13.426 | Framework avancé (logging, config, runspaces) |
| 📦 | **ps2exe** | 1.0.17 | Compilation de scripts PowerShell en .exe |
| 📦 | **string** | 1.2.13 | Manipulation de chaînes avancée |
| 📦 | **powershell-yaml** | 0.4.12 | Sérialisation/désérialisation YAML |
| 📦 | **psprivilege** | 0.2.0 | Gestion de privilèges Windows (SeDebug, etc.) |

## Données & Rapports

| Icône | Module | Version | Description |
|---|---|---|---|
| 📦 | **ImportExcel** | 7.8.10 | Lecture/écriture Excel sans Microsoft Office |
| 📦 | **PSWriteHTML** | 1.40.0 | Génération de rapports HTML interactifs |
| 📦 | **PowerHTML** | 0.2.0 | Parsing HTML via HtmlAgilityPack |

## Cloud & Infrastructure

| Icône | Module | Version | Description |
|---|---|---|---|
| 📦 | **Az** | 15.1.0 | Azure PowerShell (méta-module, 100+ sous-modules) |
| 📦 | **Microsoft.WinGet.Client** | 1.12.440 | API WinGet native PowerShell |

### Sous-modules Az notables

| Module | Version | Service Azure |
|---|---|---|
| Az.Accounts | 5.3.1 | Authentification |
| Az.Compute | 11.1.0 | VMs |
| Az.Storage | 9.4.0 | Stockage |
| Az.Network | 7.24.0 | Réseau |
| Az.KeyVault | 6.4.1 | Secrets |
| Az.Resources | 9.0.0 | ARM |
| Az.Aks | 7.0.0 | Kubernetes |
| Az.Sql | 6.3.0 | SQL Database |
| Az.Monitor | 7.0.0 | Monitoring |
| Az.Functions | 4.3.0 | Functions |

## Sécurité & Audit

| Icône | Module | Version | Description |
|---|---|---|---|
| 📦 | **SpeculationControl** | 1.0.19 | Audit Spectre/Meltdown/MDS |
| 📦 | **PSnmap** | 1.3.2 | Port scanning réseau |
| 📦 | **Ghidra** | 0.1.0 | Intégration NSA Ghidra (reverse engineering) |

## Système & Notifications

| Icône | Module | Version | Description |
|---|---|---|---|
| 📦 | **BurntToast** | 1.1.0 | Notifications Windows toast |
| 📦 | **PSWindowsUpdate** | 2.2.1.5 | Gestion Windows Update |
| ⚙️ | **Microsoft.PowerShell.ConsoleGuiTools** | 0.7.7 | `Out-ConsoleGridView` (TUI) |

## Modules Windows système (sélection)

| Icône | Module | Version | Description |
|---|---|---|---|
| 🪟 | BitLocker | 1.0.0.0 | Chiffrement de disque |
| 🪟 | Dism | 3.0 | Gestion d'images Windows |
| 🪟 | DnsClient | 1.0.0.0 | Résolution DNS |
| 🪟 | NetAdapter | 2.0.0.0 | Interfaces réseau |
| 🪟 | NetSecurity | 2.0.0.0 | Pare-feu Windows |
| 🪟 | PKI | 1.0.0.0 | Certificats |
| 🪟 | ScheduledTasks | 1.0.0.0 | Tâches planifiées |
| 🪟 | SmbShare | 2.0.0.0 | Partages réseau |
| 🪟 | Storage | 2.0.0.0 | Disques et volumes |

## PowerShell 7 core

| Icône | Module | Version | Description |
|---|---|---|---|
| ⚙️ | CimCmdlets | 7.0.0.0 | WMI/CIM |
| ⚙️ | Microsoft.PowerShell.Archive | 1.2.5 | Compression ZIP |
| ⚙️ | Microsoft.PowerShell.Management | 7.0.0.0 | Fichiers, registre, services |
| ⚙️ | Microsoft.PowerShell.PSResourceGet | 1.2.0 | Gestion de packages/modules |
| ⚙️ | Microsoft.PowerShell.Security | 7.0.0.0 | Execution policy, ACLs |
| ⚙️ | Microsoft.PowerShell.ThreadJob | 2.2.0 | Jobs en arrière-plan (threads) |
| ⚙️ | Microsoft.PowerShell.Utility | 7.0.0.0 | JSON, XML, CSV, dates, etc. |
| ⚙️ | PackageManagement | 1.4.8.1 | Gestion de sources de packages |
| ⚙️ | PowerShellGet | 2.2.5 | Installation de modules |

---

## Statistiques

| Catégorie | Nombre |
|---|---|
| Productivité shell | 9 |
| PSReadLine & prédiction | 3 |
| Scripting & dev | 6 |
| Données & rapports | 3 |
| Cloud & infra | 2 (+100 sous-modules Az) |
| Sécurité & audit | 3 |
| Système & notifications | 3 |
| Windows intégrés | ~50 |
| PowerShell 7 core | 9 |
| **Total** | **~130** |

---

## Installer un nouveau module

```powershell
# Depuis PSGallery
Install-Module -Name ModuleName -Scope CurrentUser

# Vérifier avant d'installer
Find-Module -Name ModuleName | Select-Object Name, Version, Description

# Mettre à jour tous les modules PSGallery
Get-InstalledModule | Update-Module
```

---

*Licence Apache 2.0 — voir [LICENSE](../../LICENSE).*
