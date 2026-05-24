<!--
SPDX-License-Identifier: Apache-2.0
Reverse engineering du client Discord Desktop (Electron / V8).
Source : scan + `aphrody re` sur une installation locale (app-1.0.9238).
Chemins symboliques uniquement (aucune donnee personnelle).
-->

# Client Discord Desktop — reverse engineering (Electron / V8)

Cette note documente l'**anatomie du client Discord pour Windows** telle
qu'observee par les outils d'aphrody (`aphrody re`, scan PowerShell). Elle
complete les notes `serenity-*.md` (cote bibliotheque/bot) : ici on regarde le
**binaire client** lui-meme, pas le protocole vu depuis un bot.

> Perimetre : analyse statique d'une installation locale legitime, a des fins
> d'interoperabilite (le crate `aphrody-hermes` parle a Discord). Aucune
> donnee de compte, aucun contournement de protection serveur.

## 1. Methode et provenance

| Etape | Outil | Artefact |
|---|---|---|
| Inventaire complet de l'install | `var/discord/scan-discord.ps1` (pwsh) | `var/discord/discord-scan.json` |
| RE en masse des binaires PE | `aphrody re auto <dir> --json` | `var/discord/re/auto-batch.json` |
| Detection de famille Electron/V8 | `aphrody re google <fichier> --pretty` | `var/discord/re/google-*.json` |
| Structure interne des archives | parse d'en-tete asar (pwsh) | `var/discord/asar-*.list.txt` |

Tous les artefacts vivent sous `var/` (gitignore) ; cette note en est le
distillat versionnable.

## 2. Vue d'ensemble

- **Version analysee** : `app-1.0.9238` (canal Stable).
- **Runtime** : **Electron**, **Chromium `138.0.7204.251`** (`aphrody re google`
  sur `Discord.exe` — marqueurs `app.asar`, `electron`, `chrome_100_percent.pak`,
  `ELECTRON_RUN_AS_NODE`).
- **Installeur / updater** : **Squirrel.Windows** (`Update.exe`, PE32/i386).
- **Empreinte disque** : 1075 fichiers, ~514 Mo.
- **Disposition** :

```
%LOCALAPPDATA%\Discord\
  Update.exe                 <- bootstrapper Squirrel (lance la derniere app-*)
  app-1.0.9238\
    Discord.exe              <- binaire Electron principal (~195 Mo)
    *.dll, *.pak, *.bin      <- runtime Chromium (cf. section 4)
    resources\app.asar       <- coquille bootstrap + splash
    modules\
      discord_desktop_core-1\...\core.asar   <- coeur desktop (bridge natif)
      discord_voice-1\, discord_krisp-1\, ... <- modules natifs (.node)
```

> Note sur la signature : `aphrody re google` rapporte
> `code_sign_subject: "Google Inc"` pour `Discord.exe`. C'est une heuristique
> orientee Google de la commande, qui matche des chaines Chromium embarquees ;
> l'editeur Authenticode reel est Discord Inc. A verifier via la chaine de
> certificats si la provenance importe.

## 3. Coquille Electron a deux niveaux (archives asar)

Le client est une **coquille mince** : il embarque peu de logique applicative
et **charge l'application web a distance** a l'execution. Deux archives `asar`
(format d'archive d'Electron : en-tete JSON + contenus concatenes) :

### `resources/app.asar` (bootstrap + ecran de demarrage) — 15 entrees
```
bundle.js                       <- logique d'amorcage (minifiee, webpack)
splashScreenPreload.js          <- preload du splash (contexte isole)
splash/index.html, index.css    <- ecran "Connexion..."
splash/videos/connecting.webm
splash/fonts/ggsans-*.woff2     <- police maison "gg sans"
data/quotes_copy.json           <- citations affichees au chargement
package.json
```

### `modules/.../discord_desktop_core/core.asar` (coeur desktop) — 6 entrees
```
bundle.js                       <- coeur desktop minifie (gestion fenetre, IPC,
                                   ponts natifs, auto-update, overlay)
mainScreenPreload.js            <- preload de la fenetre principale
data/cacert.pem                 <- bundle de CA racine
data/riotgames.pem              <- CA Riot Games (epinglage/integration heritee)
package.json
```

**Consequence RE majeure** : ni les routes `/api/vN`, ni l'URL `wss://` du
gateway ne sont codees en dur dans ces asar. Le shell ne connait que les
**hotes de base** (section 6) ; la version d'API et l'URL du gateway sont
fixees par le bundle web telecharge depuis `discord.com`. Pour la surface API
stable cote protocole, voir [`serenity-rest-http.md`](serenity-rest-http.md)
(API v10) et [`serenity-gateway.md`](serenity-gateway.md).

## 4. Binaires natifs (passe `aphrody re auto` — 32 PE)

`aphrody re auto` a triage 32 binaires PE (format, sections + entropie,
imports/exports, echantillon de strings, SHA-256). Tous en `x86_64` sauf
mention contraire.

### Runtime Chromium / Electron
| Fichier | Role |
|---|---|
| `Discord.exe` | binaire Electron principal (14 sections) |
| `libEGL.dll`, `libGLESv2.dll` | ANGLE (OpenGL ES sur Direct3D) |
| `vulkan-1.dll`, `vk_swiftshader.dll` | Vulkan + rasterisation logicielle SwiftShader |
| `d3dcompiler_47.dll` | compilation de shaders HLSL |
| `ffmpeg.dll` | decodage media |
| `discord_wer.dll` | hook Windows Error Reporting |

### Modules natifs Discord (`.node` = DLL PE chargee par Node)
| Module | Taille | Role observe |
|---|---|---|
| `discord_voice.node` | 14,5 Mo | pile voix/video (WebRTC, RTP/Opus) |
| `discord_krisp.node` | 15,1 Mo | suppression de bruit Krisp |
| `discord_dispatch.node` | 10,1 Mo | dispatch/IPC + telechargement de modules |
| `discord_cloudsync.node` | 4,4 Mo | synchronisation de parametres |
| `updater.node` | 4,4 Mo | moteur de mise a jour cote app |
| `discord_utils.node` | 2,1 Mo | utilitaires natifs (10 sections) |
| `discord_media.node` | 0,7 Mo | capture/encodage media |
| `discord_erlpack.node` | 0,6 Mo | (de)serialisation **ETF** (External Term Format Erlang) du gateway |
| `discord_zstd.node` | 0,7 Mo | compression Zstandard du flux gateway |
| `discord_overlay2.node`, `discord_desktop_overlay.node` | | overlay in-game |
| `discord_game_utils.node`, `discord_modules.node` | | hooks de jeu / gestion de modules |
| `cld.node` | 2,0 Mo | Compact Language Detector (correcteur orthographique) |

### Sous-modules tiers notables
- `discord_game_sdk_x64.dll` + `discord_game_sdk_x86.dll` — Game SDK Discord.
- `discord_aegis_x64.dll` + `discord_aegis_x86.dll` — composant Aegis
  (anti-falsification / integrite).
- `mediapipe.dll` — Google MediaPipe (effets video, flou d'arriere-plan).
- Helpers hors-processus : `DiscordSystemHelper.exe`, `gpu_encoder_helper.exe`,
  `audio_effects_helper.exe`.

> `aphrody re auto` rapporte `go.func_count = 0` partout : aucun binaire Go
> (contrairement a l'IDE Antigravity ou aux sidecars Go). C'est du C/C++/Rust
> natif compile en PE, plus le JS dans les asar.

## 5. Snapshots V8

Deux snapshots de demarrage V8 (acceleration du boot Electron) :
`snapshot_blob.bin` (314 Kio) et `v8_context_snapshot.bin` (683 Kio).
`aphrody re google` les classe en famille `electron` (marqueur `electron`),
sans version Chromium isolable (les snapshots ne portent pas la chaine de
version, contrairement a `Discord.exe`).

## 6. Hotes contactes (extraits du JS des asar)

Hotes en dur trouves dans `bundle.js` des deux asar :

| Hote | Usage |
|---|---|
| `discord.com` | app web + API REST |
| `canary.discord.com` | canal Canary |
| `cdn.discordapp.com` | CDN (pieces jointes, avatars, emojis) |
| `discordapp.com` | domaine media/CDN historique |
| `updates.discord.com` | flux d'auto-update Squirrel |

Cartographie reseau complete (Cloudflare, sous-domaines, gateway, ports) :
[`web-network-recon.md`](web-network-recon.md).

## 7. Chaine de mise a jour (Squirrel.Windows)

1. `Update.exe` (Squirrel, PE32) est le point d'entree installe ; il lance la
   derniere `app-<version>\Discord.exe`.
2. L'app interroge `updates.discord.com` ; `updater.node` +
   `discord_dispatch.node` recuperent et appliquent les deltas.
3. Chaque version cohabite dans son propre dossier `app-<version>\`, ce qui
   permet le rollback (l'ancien dossier reste jusqu'au nettoyage).

## 8. Pertinence pour aphrody

- **`aphrody re google` / `re auto`** reconnaissent ce profil Electron/V8 de
  bout en bout : c'est exactement le pipeline a rejouer sur tout client
  Chromium/Electron (cf. memoire « Magika opt-in + WebView2 RE » pour la
  distinction WebView2/Electron).
- **`aphrody-hermes`** (agent Discord) ne touche pas a ce client : il parle
  directement au gateway et a l'API REST via la pile decrite dans les notes
  `serenity-*.md`. Le client desktop n'est utile ici que pour comprendre
  l'authentification reelle (token en `localStorage`, pas en cookie — cf.
  [`web-network-recon.md`](web-network-recon.md) section auth).

## 9. Reproduire

```powershell
# 1. Inventaire complet -> JSON
pwsh -File var/discord/scan-discord.ps1

# 2. RE en masse de tous les PE de l'install
aphrody re auto "$env:LOCALAPPDATA\Discord" --json --limit 1500 > auto-batch.json

# 3. Famille Electron/V8 + version Chromium d'un binaire precis
aphrody re google "$env:LOCALAPPDATA\Discord\app-1.0.9238\Discord.exe" --pretty
```
