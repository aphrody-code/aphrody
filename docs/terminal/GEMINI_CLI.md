# Gemini CLI sur Windows Terminal — diagnostic crash et workaround Bun

Document écrit le **2026-05-16**. Cible : faire fonctionner
`packages/gemini-cli/` (workspace npm vendoré dans le monorepo) sur la
machine de dev qui tourne Node v26.1.0, npm 11.13.0, Bun 1.3.14.

## 1. Le problème

Sur cette machine, lancer `gemini-cli` selon n'importe laquelle des
méthodes upstream (`npm install -g @google/gemini-cli`, `npx`,
`node scripts/start.js`) crash systématiquement avec l'un des trois
patterns suivants :

| # | Symptôme observé                                                 | Cause racine                                                                |
|---|------------------------------------------------------------------|------------------------------------------------------------------------------|
| A | `ECOMPROMISED` à `npm install` ou `npx`                          | bug Node v24/v25/v26 + npm v11 sur Windows (file lock du cache npm) : [google-gemini/gemini-cli#14149](https://github.com/google-gemini/gemini-cli/issues/14149) |
| B | `ReferenceError: agent is not defined` dans `windowsTerminal.js` | binding `node-pty` non recompilé pour `NODE_MODULE_VERSION` 137 (Node 26)   |
| C | `Cannot resize a pty that has already exited` (`WindowsPtyAgent.resize`) | race condition `@lydell/node-pty` + ConPTY sous Node 24+ : [#12045](https://github.com/google-gemini/gemini-cli/issues/12045) |
| D | Freeze à `Initializing…` au premier boot                         | binding pty bloqué sur OpenConsole handshake : [#19248](https://github.com/google-gemini/gemini-cli/issues/19248) |

**Validation locale** :

```
$ node --version
v26.1.0
$ npm --version
11.13.0
```

C'est exactement la combinaison cassée. `@lydell/node-pty@1.1.0`
(consommée par `packages/gemini-cli/packages/core/package.json`) n'a
pas de prebuilt pour `NODE_MODULE_VERSION 137` au moment de l'écriture.

## 2. Décision projet (2026-05-16) : on utilise Bun

Plutôt que d'installer un Node manager (nvm-windows / fnm / volta) pour
downgrader vers Node 22 LTS, **on utilise `bun` (1.3.14)** déjà présent
sur la machine. Bun :

- gère `"workspaces"` et la spec `workspace:*` que npm 11 refuse ;
- résout son propre cache (pas de file lock Windows partagé avec npm) ;
- a une ABI N-API compatible avec les modules natifs Node, dont
  `@lydell/node-pty-win32-x64` (testé à `1.2.0-beta.12` upstream) ;
- réduit drastiquement le temps d'install (`bun install` ≈ 5 s vs
  `npm install` ≈ 90 s sur ce monorepo).

### 2.1 Recette `bun` — état des essais 2026-05-16

Trois recettes ont été testées sur cette machine, par ordre de complexité
croissante :

| # | Commande                                                             | Résultat                                                                                                |
|---|----------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------|
| 1 | `bun install`                                                        | échoue dans le `prepare` script qui appelle `execSync('npm install')` (Node v26)                        |
| 2 | `bun install --ignore-scripts`                                       | OK, lockfile écrit. Mais le `prepare` skip empêche aussi l'install transitif des packages workspace     |
| 3 | `bun esbuild.config.js`                                              | échoue : `Could not resolve "execa"`, `"tinycolor2"`, `"ws"` (deps présentes dans `.bun/` cache mais non symlinkées dans `node_modules/`) |

**Conclusion** : le pipeline officiel `gemini-cli` n'est pas réellement
compatible Bun sans patch upstream. Les `scripts/build.js` et
`scripts/build_package.js` font des `execSync('npm install')` et
`execSync('npm run build')` en dur, ce qui force un retour à
Node v26 + npm 11 qui casse.

**Recette qui fonctionnerait** (à valider) — patch minimal de
`scripts/build.js` pour remplacer `npm install` par `bun install` :

```powershell
cd C:\src\aphrody\packages\gemini-cli

# Patch local (pas remonté upstream) : remplacer "npm install" par "bun install"
# dans scripts/build.js et scripts/build_package.js, et "npm run build" par "bun run build".
# Lignes concernées : build.js:30, build.js:42, build_package.js (chercher npm run).

bun install
bun run bundle
bun bundle/gemini.js --version
```

Ce patch est **bloqué tant que `packages/gemini-cli` est vendoré
tel quel** : modifier ces scripts est traçable mais dirty.
À discuter avec l'amont (issue à ouvrir : « support `BUN_INSTALL`
env var to use bun for installs from scripts/build.js »).

### 2.1bis Recette de contournement immédiate

En attendant le patch, **la solution qui marche aujourd'hui sur cette
machine** est le Plan C (Node v22 LTS, cf. § 3). Une fois Node v22
installé :

```powershell
# Avec Node v22 LTS actif (via volta/fnm/PATH dédié) :
cd C:\src\aphrody\packages\gemini-cli
bun install                  # bun gère mieux workspace:* que npm 11
bun run bundle               # scripts/build.js voit Node v22 → npm install OK
node bundle/gemini.js --version
```

Le mix `bun install` + `node bundle/gemini.js` capture le meilleur des
deux : install rapide, runtime Node 22 LTS stable pour `@lydell/node-pty`.

### 2.2 Profil Windows Terminal dédié

Le `settings.json` de Windows Terminal Canary
(`%LOCALAPPDATA%\Packages\Microsoft.WindowsTerminalCanary_8wekyb3d8bbwe\LocalState\settings.json`)
peut recevoir un profil prêt à l'emploi :

```jsonc
{
  "guid": "{a8b1c2d3-4e5f-4a6b-9c8d-ef0123456789}",
  "name": "Gemini CLI (bun)",
  "commandline": "%USERPROFILE%\\.bun\\bin\\bun.exe run %REPO_ROOT%\\packages\\gemini-cli\\bundle\\gemini.js",
  "startingDirectory": "%REPO_ROOT%",
  "icon": "ms-appx:///ProfileIcons/{0caa0dad-35be-5f56-a8ff-afceeeaa6101}.png",
  "colorScheme": "Campbell Powershell",
  "font": { "face": "Google Sans Code", "size": 18, "weight": "bold" },
  "hidden": false
}
```

À insérer dans `profiles.list[]` du `settings.json`.

## 3. Plans de repli

### Plan B — Désactiver complètement node-pty

`gemini-cli` lit la variable d'environnement `GEMINI_PTY_INFO` (cf.
`packages/gemini-cli/packages/core/dist/src/utils/getPty.js`). Si on la
positionne à `child_process`, le CLI tombe back sur un `spawn` standard
sans ConPTY :

```powershell
$env:GEMINI_PTY_INFO = 'child_process'
gemini
```

Conséquence : on perd l'interactivité fine (VT input, redimensionnement
PTY), mais `gemini` reste utilisable pour les requêtes one-shot.

### Plan C — Downgrade Node v22 LTS

Si Bun pose problème sur une dépendance future :

```powershell
winget install OpenJS.NodeJS.LTS  # installe Node 22.x LTS
node --version                     # v22.x
npm install -g @google/gemini-cli
```

À partir de mai 2026, Node 22 LTS est supporté jusqu'à avril 2027.

## 4. Pourquoi pas WSL ?

WSL est une option valide (Node 22 LTS sous Ubuntu, ConPTY remplacé par
PTY POSIX, donc plus de race condition `WindowsPtyAgent.resize`), mais :

- l'expérience est lente côté FS (cross-mount `\\wsl$\`) si le projet vit
  sur `C:\src\` ;
- on perd l'intégration native avec les outils Windows (Visual Studio,
  notebooks `.ps1`, MCP `windows-mcp`) ;
- ce sera de toute façon adressé par `crates/google_os` qui implémente
  un userland POSIX natif sur NT.

## 5. Crash *résiduel* éventuel côté Windows Terminal

Indépendamment de `gemini-cli`, Windows Terminal **lui-même** peut
crasher dans deux cas connus :

1. **Driver GPU intermittent** : « A handful of Intel & Radeon drivers
   intermittently drop the resize event that Atlas needs. » Si tu
   observes des freezes au resize, bascule `rendering.graphicsAPI` de
   `"direct3d11"` vers `"direct2d"` (cf. `BUILD.md` § 4.1 de Terminal
   docs).
2. **WindowsTerminalDev** local non installé après un build : tant que
   `scripts/terminal/build.ps1` produit le `.msix` mais que
   `Add-AppDevPackage.ps1` n'a pas été exécuté, lancer
   `WindowsTerminal.exe` directement échoue avec
   `class not registered`. Voir `docs/terminal/BUILD.md` § 7.5.

## 6. Références

- Issue [#14149](https://github.com/google-gemini/gemini-cli/issues/14149) — ECOMPROMISED Node 24/25/26
- Issue [#19248](https://github.com/google-gemini/gemini-cli/issues/19248) — Freeze Initializing Node 20
- Issue [#14619](https://github.com/google-gemini/gemini-cli/issues/14619) — `agent is not defined`
- Issue [#12045](https://github.com/google-gemini/gemini-cli/issues/12045) — `Cannot resize a pty`
- Issue [#9054](https://github.com/google-gemini/gemini-cli/issues/9054) — EPERM + ERR_INVALID_ARG_TYPE
- [Microsoft Learn — Windows Terminal Rendering Settings](https://learn.microsoft.com/en-us/windows/terminal/customize-settings/rendering)
- [Bun docs — Workspace support](https://bun.sh/docs/install/workspaces)
