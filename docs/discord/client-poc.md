<!--
SPDX-License-Identifier: Apache-2.0
Preuves de concept (PoC) de reverse engineering du client Discord desktop.
Realisees sur une installation locale legitime (app-1.0.9238), a des fins
d'interop. Aucun secret en clair : tokens masques, ID de compte non reproduit,
artefacts bruts confines a var/ (gitignore). Chemins symboliques uniquement.
-->

# Discord — preuves de concept (PoC) reverse engineering du client

Quatre PoC concrets executes sur l'installation locale (Discord `app-1.0.9238`,
**Electron 37.6.0**), plus un bonus : charger le moteur voix natif hors Discord.
Chaque PoC a produit des artefacts sous `var/discord/poc/0N-*/` (gitignore).

> Cadre : analyse/instrumentation d'une application installee, sur la machine et
> le compte de l'utilisateur, a des fins d'interop (cf. `aphrody-hermes`). Aucun
> secret n'est extrait en clair ; rien n'est exfiltre.

## Synthese

| # | Question | Resultat | Outil |
|---|---|---|---|
| 1 | Langage / framework ? | Electron 37.6.0 / React 19.1.0 / C++ N-API / ETF | `re triage`, CDP, asar |
| 2 | Lire la memoire ? | Oui (CDP heap V8 + ReadProcessMemory) | CDP :9223, RPM Win32 |
| 3 | Plugin de features ? | Oui (plugin BetterDiscord fonctionnel) | BdApi |
| 4 | Inspection par injection ? | Attache + enum oui ; hook **bloque par CIG** | Frida, psapi |
| + | Charger `discord_voice.node` ? | Oui sous Bun et node (95 exports) | `var/discord/discord.ts` |

## PoC 1 — Langage / framework (preuves)

| Composant | Verdict | Preuve |
|---|---|---|
| Shell | **Electron 37.6.0** (devDep 37.6.1) | UA CDP `discord/1.0.9238 ... Electron/37.6.0` ; `package.json` asar |
| Moteur web | **Chromium 138.0.7204.251**, **V8 13.8.258.32** | CDP `/json/version` |
| UI | **React 19.1.0** + React Router 6 + react-dnd | check de version interne dans le bundle web |
| Bundler | **rspack** (compatible webpack) | `__rspack_context`, `webpackChunkdiscord_app` (587 chunks, 3213 modules) |
| Addons natifs | **C++ MSVC `/MT`, N-API** | exports `napi_register_module_v1` ; aucune dep VCRUNTIME/MSVCP (CRT statique) |
| Gateway | **ETF (Erlang External Term Format)** | strings de `discord_erlpack.node` + PDB `...\discord_erlpack\src\decoder.h` |

Conclusion : coquille **Electron** + app **React** chargee a distance ; logique
de performance en **modules natifs C++ N-API** ; le gateway temps reel parle
**ETF**, signature d'un backend de famille Erlang/Elixir.

## PoC 2 — Lecture de la memoire vive (deux voies)

1. **CDP / heap V8** — via le port de debug Electron (`--remote-debugging-port`),
   connexion WebSocket au CDP. Constat d'interop : le `localStorage` et le token
   ne sont pas dans le main world mais dans le **contexte isole Electron**
   (contextId 2). Lecture du heap live : `location.href`, titre de fenetre, 97
   cles de `localStorage`, statistiques de heap (~147 Mo utilises).
2. **ReadProcessMemory** — chaine Win32 `OpenProcess(VM_READ|QUERY_INFORMATION)`
   -> `VirtualQueryEx` (regions `MEM_COMMIT`, hors `PAGE_GUARD`) ->
   `ReadProcessMemory` (recherche native compilee). Sur le process renderer :
   ~367 Mo parcourus, marqueur benin `gateway.discord.gg` localise a 5 adresses.

**Token** : present dans `localStorage` (`token`), type chaine, longueur 150 ;
**uniquement une preview masquee** (4 premiers + 4 derniers caracteres) a ete
manipulee. La valeur complete n'a jamais ete materialisee ni ecrite. Ceci
confirme le modele d'auth de [`web-network-recon.md`](web-network-recon.md) : le
token vit en memoire/`localStorage`, accessible a tout code local — d'ou
l'importance de ne jamais l'exposer.

## PoC 3 — Plugin qui ajoute des fonctionnalites (BetterDiscord)

Plugin reel `AphrodyProbe.plugin.js` ecrit avec l'API BdApi actuelle :
`BdApi.UI.showToast`, `BdApi.Webpack.getStore("GuildStore"/"UserStore"/"ChannelStore")`,
`BdApi.Patcher.after` (hook post-execution non destructif, auto-retire),
`BdApi.ContextMenu.patch` (item de menu), `getSettingsPanel()`. Syntaxe validee
(`node --check` OK), installe dans `%APPDATA%\BetterDiscord\plugins\`.

Etat : **BetterDiscord lui-meme n'est pas installe** sur ce poste
(`discord_desktop_core\index.js` intact) ; le PoC prouve le **mecanisme** (point
d'injection = `mainScreenPreload.js` du `core.asar`, patch webpack via
`webpackChunkdiscord_app`) et produit un plugin correct, chargeable des que le
loader BD est present.

> Mise en garde : les client mods (BetterDiscord, Vencord) violent les CGU de
> Discord. Tolere en pratique pour un usage personnel, mais risque theorique de
> sanction de compte. A eviter sur un compte sensible.

## PoC 4 — Inspection par injection / instrumentation

- **Frida** (17.9.11) : `frida.attach()` reussit ; `Process.enumerateModules()`
  retourne **146 modules** dans le renderer (adresses ASLR de chaque `.node`
  capturees). Confirme independamment via `psapi!EnumProcessModulesEx`.
- **Hook live** (`Interceptor.attach` sur `ws2_32!send`) : **echec — bloque par
  CIG** (`Code Integrity Guard`, politique `MicrosoftSignedOnly=1`). Le kernel
  rejette `frida-agent.dll` (non signe Microsoft) avec
  `STATUS_INVALID_IMAGE_HASH` avant execution. L'enumeration, elle, passe car
  elle n'injecte pas de DLL (handles lecture seule, comme un debugger/AV).
- **Aegis** (`discord_aegis_x64.dll`) : **absent** des process en session normale
  (charge seulement avec la Game Verification d'un serveur). La protection
  anti-injection effective ici est donc **CIG**, pas Aegis.
- `CreateRemoteThread` + `LoadLibrary` : technique documentee, **non executee** ;
  bloquee par le meme CIG (toute DLL non signee MS rejetee au niveau kernel).

Bilan : l'introspection read-only (enumeration, lecture memoire) est possible et
peu detectable ; l'injection de code (DLL/hook) est barree par CIG sur les
process Discord.

## Bonus — Charger `discord_voice.node` hors Discord

Script `var/discord/discord.ts` (Bun + node). Le `.node` est un addon N-API
C++ qui depend de : `mediapipe.dll` (dossier voix), **`ffmpeg.dll`** (dossier
parent `app-<ver>/`) et **`node.exe`** (host des symboles N-API). En ajoutant
ces dossiers au chemin de recherche DLL, l'addon **se charge** :

- sous **Bun 1.3.14** via `require()` ;
- sous **node v26** via `process.dlopen()` ;
- **95 exports** enumeres ; appels reellement executes (`getInputDevices()` leve
  `Invalid argument count: expected 1` => validation native = addon fonctionnel).

Surface d'API notable : `VoiceConnection`/`VoiceReplayConnection`, codecs
(`getCodecCapabilities`), Krisp (`setKrispPath`), VAD (`setEmitVADLevel`),
clips + ML (`saveClip`, `setClipsMLPipelineEnabled`), capture ecran
(`getScreenPreviews`), enregistrement brut (`startRecordingRawSamples`),
selection de region RTC (`rankRtcRegions`), et E2EE :
`getMLSSigningKey` + `SupportedSecureFramesProtocolVersion` (protocole DAVE).

Pour la voix d'aphrody-hermes, la voie portable reste neanmoins
**songbird/serenity** (Rust, RTP/Opus, cf. `serenity-framework-voice.md`) :
ce `.node` est specifique a l'ABI Electron de Discord et a son host.

## Reproductibilite (artefacts gitignore sous `var/discord/poc/`)

| PoC | Dossier / fichiers cles |
|---|---|
| 1 langage | `01-language/evidence.md` |
| 2 memoire | `02-memory/report.md`, `cdp_probe3.mjs`, `rpm_native.ps1` |
| 3 plugin | `03-plugin/AphrodyProbe.plugin.js`, `report.md` |
| 4 injection | `04-injection/report.md`, `probe.js`, `run_probe.py`, `psapi_modules_*.txt` |
| bonus voix | `var/discord/discord.ts` |

## Mises en garde

- Tout a ete realise sur l'application, la machine et le compte de l'utilisateur.
- Aucun secret en clair : tokens masques, ID de compte non reproduit ici, bruts
  confines a `var/` (gitignore, jamais commite).
- Anti-tamper reel : **CIG** sur les process Discord ; CGU pour les client mods.
