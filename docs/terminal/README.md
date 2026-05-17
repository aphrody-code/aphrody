# `vendor/terminal` — Microsoft Terminal en sous-module

Document d'index, écrit le **2026-05-16**.

## Pourquoi Terminal est-il dans `vendor/`

Microsoft Terminal est cloné comme sous-module Git à l'emplacement
`vendor/terminal/`. C'est un dépôt monolithique qui contient à la fois
`conhost.exe` (le serveur de console NT historique de Windows), la pile
ConPTY (`winconpty.dll`), la nouvelle application `WindowsTerminal.exe`
(WinUI 2 + DirectWrite + Direct3D) et un ensemble de bibliothèques
réutilisables : parser VT, text buffer, framework `til`, etc. Cette base
est utile à `aphrody` pour trois raisons :

1. fournir un terminal d'avant-plan moderne pour nos CLIs / REPL ;
2. récupérer les composants C++ statiques (`vtparser`, `bufferout`,
   `winconptylib`, `til`) à linker depuis nos crates Rust via
   `windows-rs` + `cc-rs` + `mimalloc` ;
3. servir de référence d'implémentation du serveur ConDrv NT pour aider
   `google_os` à émuler un pseudo-terminal cohérent côté Linux/POSIX.

Le sous-module est **en lecture seule** côté `aphrody`. Toute
modification se fait amont, dans le fork si nécessaire.

## Version clonée

```
commit  8fe6c21ef88a73a7985b5968ee18936928ccac69
date    2026-05-15 13:56:48 -0500
title   Keep the font size delta across settings reloads (#20230)
```

Branche officielle : `microsoft/terminal:main`. Licence : **MIT**
(`vendor/terminal/LICENSE`). Notices tierces : `vendor/terminal/NOTICE.md`.

## Documents disponibles

| Fichier                                  | Rôle                                                                                                            |
|------------------------------------------|-----------------------------------------------------------------------------------------------------------------|
| [`ARCHITECTURE.md`](./ARCHITECTURE.md)   | Cartographie complète : binaires produits, dossiers de `src/`, couches, composants réutilisables, tests, specs. |
| [`BUILD.md`](./BUILD.md)                 | Build officiel Microsoft + procédure spécifique `scripts/terminal/build.ps1` (toolset v145 + SDK 10.0.26100.0). |
| [`INTEGRATION.md`](./INTEGRATION.md)     | Matrice d'intégration pour `google_os` / `aphrody` : composants utiles, stratégie FFI, sécurité, go/no-go.   |
| [`PATCHES.diff`](./PATCHES.diff)         | Patches locaux appliqués au sous-module (overlays vcpkg v143→v145, warning C4875). Réappliquer : `cd vendor/terminal && git apply ../../docs/terminal/PATCHES.diff`. |
| [`GEMINI_CLI.md`](./GEMINI_CLI.md)       | Diagnostic du crash `gemini-cli` sur Windows (Node v26 + npm 11 incompatibles `node-pty`). Workaround Bun documenté + plans de repli. |

## Build local

Pré-requis sur la machine cible : VS 2026 Community Insiders 18.7 (toolset
v145, MSVC 14.51), Windows SDK 10.0.26100.0, PowerShell 7.6.1, .NET 10.0.300.

Construction d'un sous-projet de fumée (par défaut : `OpenConsole.exe`,
alias dev de `conhost.exe`) :

```powershell
pwsh -File scripts/terminal/build.ps1
```

Plein détails dans `BUILD.md`.

## Politiques de lecture / écriture

- **Aucun fichier de notre cru** ne vit dans `vendor/terminal/`. Le
  wrapper de build (`scripts/terminal/build.ps1`) et notre doc
  (`docs/terminal/`) sont à la racine du repo parent.
- Le sous-module reste **dirty** après build : patches obligatoires
  (`PATCHES.diff`) et artifacts générés (`bin/`, `obj/`, `packages/`).
  Ces patches sont locaux, jamais propagés à l'upstream Microsoft.
- Pour mettre à jour la version vendorisée :
  `git submodule update --remote vendor/terminal`, ré-appliquer
  `PATCHES.diff`, puis `git add` du sous-module dans le parent.
- Toute documentation française additionnelle va dans `docs/terminal/`,
  jamais dans `vendor/terminal/`.

## À consulter en complément

- [`../design/aphrody-terminal-spec.md`](../design/aphrody-terminal-spec.md) :
  **spec normative aphrody-terminal LLM-first** (5 piliers, WASM-native,
  M3-themed) — successeur Rust pur du modèle vendor/terminal Win-only.
- [`../design/aphrody-terminal-integration-matrix.md`](../design/aphrody-terminal-integration-matrix.md) :
  matrice contract-de-vie par crate (chaque crate du workspace a un slot
  dans `aphrody-terminal`).
- [`../PLAN-MOONSHOT.md`](../PLAN-MOONSHOT.md) : plan 30 jours qui drive
  l'ambition `aphrody-terminal`.
- `vendor/terminal/README.md` : README officiel Microsoft.
- `vendor/terminal/doc/building.md` : procédure de build amont.
- `vendor/terminal/doc/ORGANIZATION.md` : description de l'organisation
  du code par Microsoft.
- `vendor/terminal/doc/STYLE.md`, `EXCEPTIONS.md`, `WIL.md`,
  `virtual-dtors.md` : règles de codage.
- `vendor/terminal/doc/specs/` : 60+ specs de features VT et UI.
</content>
</invoke>
