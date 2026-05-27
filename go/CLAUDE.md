<!-- SPDX-License-Identifier: Apache-2.0 -->
# CLAUDE.md

Guide opérationnel pour Claude Code (claude.ai/code) sur le dépôt **aphrody-go**.

**Rôle assigné** : **Hardcore Low-level Go Engineer**
Focus : Go systems programming, HTML extraction, BPE tokenization, Google APIs integration, and safe agent execution.

## 0. Cap projet

Le projet est `aphrody-go`, le compagnon Go d'aphrody. Il fournit :
1. **Unification** : La commande `aphrody-tokenizer-go` et les outils Google Workspace (`gogcli`) sont fusionnés en un seul module et binaire.
2. **Reverse engineering intel** : `antigravity-langserver-re` reproduit le protocole d'interaction avec le language server Google.
3. **Contrôle total Google Account** : Le binaire unifié permet le pilotage de Gmail, Calendar, Drive, Docs, Sheets, Slides, Forms, etc.

## 1. ZÉRO STUB, 100% PRODUCTION

Chaque fichier doit implémenter de vrais comportements sans aucun stub ou `TODO`.

## 2. Commandes de validation

```bash
# Compilation du binaire unifié (à placer dans PATH ou à côté du binaire principal aphrody)
go build -o gogcli/cmd/aphrody-tokenizer-go/aphrody-tokenizer-go.exe ./gogcli/cmd/aphrody-tokenizer-go

# Exécution des tests du workspace
go test ./gogcli/...

# Formater le code
go fmt ./gogcli/... ./antigravity-langserver-re/...
```

## 3. Directives de style et architecture Go

- **Workspace** : Géré via `go.work`. Les modules importent des packages via des chemins canoniques du module (`github.com/steipete/gogcli/...`).
- **Indentation** : Toujours utiliser des tabulations pour le code Go (configuré via `.editorconfig`).
- **Compatibilité OS** : Lors des tests manipulant le dossier utilisateur (`~`), toujours simuler le home directory en surchargeant à la fois `HOME` et `USERPROFILE` pour assurer la portabilité Windows/Linux.

## 4. Dossier d'Assets / Google Drive

Pour téléverser des assets et des fichiers de recherche :
1. Créez un dossier sur votre Drive personnel et partagez-le en tant qu'**Éditeur** avec l'adresse du compte de service : `aphrody-bot@aphrody.iam.gserviceaccount.com`.
2. Configurez la variable d'environnement `GOG_DRIVE_ASSETS_FOLDER_ID` dans le fichier `.env`.
3. Téléversez des fichiers en ligne de commande :
   ```bash
   aphrody-tokenizer-go.exe upload_asset <chemin_local> [nom_affichage]
   ```

