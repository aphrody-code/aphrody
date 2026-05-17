<!-- SPDX-License-Identifier: Apache-2.0 -->
# WinGet — Cheatsheet

> Aide-mémoire des commandes WinGet les plus utilisées pour `aphrody`.

---

## 🚀 Setup rapide

```powershell
# Bootstrap complet (DSC)
winget configure .config/configuration.winget

# Installation manuelle rapide (Google uniquement)
winget install Google.CloudSDK Google.DartSDK Google.FirebaseCLI Google.Protobuf Google.flatbuffers Google.Libwebp Google.Brotli Google.OSVScanner Google.Perfetto Google.PlatformTools
```

---

## 🔍 Recherche

```powershell
winget search protobuf                     # Par nom
winget search --id Google --source winget  # Par préfixe ID
winget show Google.CloudSDK                # Détails d'un package
winget show Google.DartSDK --versions      # Toutes les versions
```

## 📥 Installation

```powershell
winget install Google.Protobuf                             # Standard
winget install Google.DartSDK --version 3.11.6             # Version pinée
winget install Google.DartSDK --silent                     # Silencieux
winget install Google.DartSDK -h                           # Help du package
winget install A B C --accept-source-agreements --silent   # Batch CI
```

## 📋 Gestion

```powershell
winget list --source winget                # Installés (source winget)
winget list --name Google                  # Filtrer Google
winget list --id Google.CloudSDK           # Package spécifique
winget uninstall Google.IAPDesktop         # Désinstaller
```

## ⬆️ Mise à jour

```powershell
winget upgrade --source winget             # Voir les MAJ disponibles
winget upgrade Google.CloudSDK             # Mettre à jour un package
winget upgrade --all                       # Tout mettre à jour
winget upgrade --all --include-unknown     # Inclure les versions inconnues
```

## 💾 Export / Import

```powershell
winget export -o snapshot.json --source winget          # Exporter
winget import -i snapshot.json --accept-package-agreements  # Importer
```

## ⚙️ Configuration DSC

```powershell
winget configure .config/configuration.winget                          # Appliquer
winget configure show .config/configuration.winget                     # Prévisualiser
winget configure .config/configuration.winget --accept-configuration-agreements  # CI
```

## 🔧 Dépannage

```powershell
winget --version                           # Version de winget
winget source list                         # Sources configurées
winget source update                       # Rafraîchir les sources
winget source reset --force                # Reset sources (fix erreurs)

# Recharger PATH après installation (sans redémarrer le shell)
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# Vérifier que les CLIs sont accessibles
@('gcloud','dart','firebase','protoc','flatc','cwebp','brotli','osv-scanner','perfetto','adb') | ForEach-Object {
  $c = Get-Command $_ -EA SilentlyContinue
  if ($c) { "✅ $_ → $($c.Source)" } else { "❌ $_ NOT FOUND" }
}
```

## 📂 Fichiers du projet

| Fichier | Usage |
|---|---|
| `.config/configuration.winget` | `winget configure` — DSC déclaratif |
| `google.json` | Inventaire complet + config projet |
| `docs/winget/README.md` | Documentation complète |
| `docs/winget/CATALOG.md` | Catalogue 40+ packages Google |

---

## 📊 Schémas JSON/YAML utiles

| Schéma | URL |
|---|---|
| Export/Import JSON | `https://aka.ms/winget-packages.schema.2.0.json` |
| Configuration DSC | `https://aka.ms/configuration-dsc-schema/0.2` |
| Manifest Version | `https://aka.ms/winget-manifest.version.1.9.0.schema.json` |
| Manifest Installer | `https://aka.ms/winget-manifest.installer.1.9.0.schema.json` |
| Manifest Locale | `https://aka.ms/winget-manifest.defaultLocale.1.9.0.schema.json` |

---

*Licence Apache 2.0 — voir [LICENSE](../../LICENSE).*
