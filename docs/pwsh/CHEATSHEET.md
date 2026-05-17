<!-- SPDX-License-Identifier: Apache-2.0 -->
# PowerShell 7 — Cheatsheet

> Aide-mémoire des commandes et patterns pwsh pour `aphrody`.

---

## 🚀 Navigation rapide

```powershell
z aphrody                     # ZLocation — saut par fréquence
z src                            # Partiel : premier match
cd -                             # Retour au répertoire précédent
Push-Location C:\src; Pop-Location  # Stack de répertoires
with cargo                       # Préfixe persistant (posh-with)
```

## 📁 Fichiers & Répertoires

```powershell
# Lister avec icônes (Terminal-Icons)
ls                               # Get-ChildItem avec icônes
ls -Force                        # Inclure les fichiers cachés
ls -Recurse -Filter *.rs         # Récursif avec filtre

# Recherche
Get-ChildItem -Recurse -Filter *.cpp | Select-Object FullName
rg "pattern" --type rust         # ripgrep (plus rapide)

# Taille d'un répertoire
(Get-ChildItem -Recurse | Measure-Object Length -Sum).Sum / 1MB
```

## 📋 Pipeline & Manipulation

```powershell
# Pipeline parallèle (pwsh 7+)
1..100 | ForEach-Object -Parallel { $_ * 2 } -ThrottleLimit 8

# Filtrage
Get-Process | Where-Object CPU -gt 10 | Sort-Object CPU -Descending

# Sélection
Get-Process | Select-Object Name, CPU, WorkingSet64 -First 10

# Groupement
Get-ChildItem | Group-Object Extension | Sort-Object Count -Descending

# Mesure
Get-ChildItem -Recurse -File | Measure-Object Length -Sum -Average -Maximum
```

## 🔤 Syntaxe pwsh 7+

```powershell
# Ternaire
$result = ($value -gt 0) ? "positif" : "négatif"

# Null-coalescing
$name = $env:USER ?? "unknown"
$list ??= @()                   # Assignation si null

# Null-conditional
${obj}?.Method()

# Pipeline chain
command1 && command2             # Si 1 réussit, exécuter 2
command1 || command2             # Si 1 échoue, exécuter 2

# Switch expression
$color = switch ($status) {
    'OK'    { 'Green' }
    'Warn'  { 'Yellow' }
    'Error' { 'Red' }
    default { 'White' }
}
```

## 📊 JSON

```powershell
# Lecture
$data = Get-Content google.json | ConvertFrom-Json

# Accès
$data.winget_packages.google_installed | Format-Table

# Écriture (TOUJOURS spécifier -Depth)
$data | ConvertTo-Json -Depth 10 | Set-Content google.json

# Depuis une API
$response = Invoke-RestMethod https://api.example.com/data
```

## 🌐 Web & API

```powershell
# GET simple
Invoke-RestMethod https://api.github.com/repos/aphrody-code/aphrody

# POST avec body
$body = @{ name = "test" } | ConvertTo-Json
Invoke-RestMethod -Uri $url -Method Post -Body $body -ContentType 'application/json'

# Téléchargement
Invoke-WebRequest -Uri $url -OutFile output.zip

# Scraping HTML (PowerHTML)
Import-Module PowerHTML
$html = Invoke-WebRequest $url | ConvertFrom-Html
$html.SelectNodes("//title").InnerText
```

## 🔧 Système

```powershell
# Processus
Get-Process | Sort-Object CPU -Descending | Select-Object -First 10
Stop-Process -Name "processname"

# Services
Get-Service | Where-Object Status -eq Running
Restart-Service -Name wuauserv

# Registre
Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion"

# Variables d'environnement
$env:Path -split ';'
[Environment]::SetEnvironmentVariable("KEY", "VALUE", "User")

# Informations système
Get-ComputerInfo | Select-Object OsName, OsVersion, CsProcessors
```

## 📦 Modules

```powershell
# Rechercher
Find-Module *google* | Format-Table Name, Version, Description

# Installer
Install-Module Terminal-Icons -Scope CurrentUser

# Mettre à jour
Update-Module Az

# Lister les installés
Get-InstalledModule | Format-Table Name, Version

# Désinstaller
Uninstall-Module -Name ModuleName
```

## 🪄 WinGet (via module natif)

```powershell
# Plus rapide que le CLI winget
Import-Module Microsoft.WinGet.Client

Get-WinGetPackage -Name Google              # Installés
Find-WinGetPackage -Id Google.DartSDK       # Rechercher
Install-WinGetPackage -Id Google.Protobuf   # Installer
Update-WinGetPackage -Id Google.CloudSDK    # Mettre à jour
```

## 🧪 Tests (Pester)

```powershell
# Exécuter les tests
Invoke-Pester -Path .\tests\

# Test spécifique
Invoke-Pester -Path .\tests\Build.Tests.ps1

# Avec couverture de code
Invoke-Pester -CodeCoverage .\src\*.ps1
```

## 🔍 Linting (PSScriptAnalyzer)

```powershell
# Linter un fichier
Invoke-ScriptAnalyzer -Path .\script.ps1

# Linter tout le projet
Get-ChildItem -Recurse -Filter *.ps1 | Invoke-ScriptAnalyzer

# Avec sévérité minimum
Invoke-ScriptAnalyzer -Path . -Severity Warning

# Fixer automatiquement
Invoke-ScriptAnalyzer -Path .\script.ps1 -Fix
```

## 📤 Export & Rapports

```powershell
# Excel (ImportExcel)
$data | Export-Excel -Path report.xlsx -AutoSize -FreezeTopRow

# HTML (PSWriteHTML)
New-HTML -FilePath report.html {
    New-HTMLTable -DataTable $data
}

# CSV
$data | Export-Csv -Path output.csv -NoTypeInformation

# Clipboard
$data | Set-Clipboard
Get-Clipboard
```

## ⌨️ PSReadLine

```powershell
# Raccourcis
Get-PSReadLineKeyHandler                    # Voir tous les raccourcis

# Configuration
Set-PSReadLineOption -PredictionViewStyle ListView
Set-PSReadLineOption -PredictionSource HistoryAndPlugin

# Historique
Get-History                                 # Session courante
Get-Content (Get-PSReadLineOption).HistorySavePath | Select-Object -Last 20
```

## 🔐 Sécurité

```powershell
# Execution policy
Get-ExecutionPolicy -List
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned

# Exécuter un script non signé
pwsh -ExecutionPolicy Bypass -File script.ps1

# Certificats
Get-ChildItem Cert:\CurrentUser\My

# Privilèges (psprivilege)
Import-Module psprivilege
Get-ProcessPrivilege
```

## 🎨 Prompt & Thème

```powershell
# oh-my-posh
oh-my-posh get shell               # Shell actuel
oh-my-posh print primary           # Tester le prompt
Get-PoshThemes                     # Voir les thèmes

# posh-git
$GitPromptSettings                 # Configuration du prompt git
```

## ⚡ Astuces de performance

```powershell
# Mesurer le temps d'exécution
Measure-Command { cargo build 2>&1 }

# Jobs en arrière-plan (ThreadJob — plus rapide que Start-Job)
$job = Start-ThreadJob { cargo build --release }
$job | Receive-Job -Wait

# Streaming (pas de buffering)
& cargo build 2>&1 | ForEach-Object { $_ }

# Profil de démarrage
Measure-Command { pwsh -NoProfile -Command exit }  # baseline
Measure-Command { pwsh -Command exit }              # avec profil
```

---

## 📂 Fichiers du projet

| Fichier | Usage |
|---|---|
| `~\Documents\PowerShell\profile.ps1` | Profil AllHosts (C:\usr PATH) |
| `~\Documents\PowerShell\Microsoft.PowerShell_profile.ps1` | Profil pwsh (Claude, PSReadLine, modules) |
| `docs/pwsh/README.md` | Documentation complète |
| `docs/pwsh/MODULES.md` | Catalogue des ~130 modules |
| `docs/pwsh/CHEATSHEET.md` | Ce fichier |

---

*Licence Apache 2.0 — voir [LICENSE](../../LICENSE).*
