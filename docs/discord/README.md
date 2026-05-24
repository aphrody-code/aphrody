<!--
SPDX-License-Identifier: Apache-2.0
Index de la documentation Discord d'aphrody.
-->

# Discord — reference aphrody

Documentation Discord pour aphrody, sous **deux angles complementaires** :

1. **Cote bibliotheque / bot** — le protocole Discord vu depuis Rust, distille
   du code source de **serenity 0.12.5** (clone dans `var/serenity`). C'est la
   reference pour faire evoluer `aphrody-hermes` (l'agent Discord multi-canaux).
2. **Cote client / reseau** — reverse engineering du **client desktop**
   (Electron/V8) et **cartographie reseau** de l'infrastructure, produits par
   `aphrody re`, le scan PowerShell et `aphrody dns_recon`/`advanced_recon`.

## Organisation

### Bibliotheque / protocole (serenity)
| Document | Contenu |
|---|---|
| [`serenity-architecture.md`](serenity-architecture.md) | Modules, features Cargo, modele async tokio, flux client -> gateway -> http |
| [`serenity-gateway.md`](serenity-gateway.md) | Passerelle WebSocket : IDENTIFY/READY, heartbeat, RESUME, sharding, 21 intents, 12 opcodes |
| [`serenity-rest-http.md`](serenity-rest-http.md) | Client HTTP REST, API v10, auth Bot/Bearer, rate-limiting par bucket |
| [`serenity-models.md`](serenity-models.md) | Types du domaine : Guild, Channel, Message, User, Permissions, Id/Snowflake |
| [`serenity-framework-voice.md`](serenity-framework-voice.md) | Framework de commandes (deprecie -> poise), collecteurs, voix (songbird, RTP/Opus) |

### Client / reseau (reverse engineering)
| Document | Contenu |
|---|---|
| [`client-electron-re.md`](client-electron-re.md) | Anatomie du client desktop : Electron, Chromium 138, asar, 15 modules natifs, V8, Squirrel |
| [`web-network-recon.md`](web-network-recon.md) | DNS/IP/Cloudflare, 51 sous-domaines, frameworks frontend/backend, modele d'authentification |

## Faits cles (synthese)

- **API REST** : `https://discord.com/api/v10/` (v10). **Gateway** :
  `wss://gateway.discord.gg`.
- **21 intents** (3 privilegies : `GUILD_MEMBERS`, `GUILD_PRESENCES`,
  `MESSAGE_CONTENT`), **12 opcodes** gateway ; encodage **ETF** (erlpack) +
  compression Zstd sur le flux.
- **Client desktop** = coquille **Electron** mince (Chromium `138.0.7204.251`)
  qui charge l'app web React a distance ; logique native dans des modules
  `.node` (voice, krisp, dispatch, erlpack...).
- **Reseau** : tout est fronte par **Cloudflare** ; backend gateway de famille
  **Erlang/Elixir** (empreinte ETF directe cote client).
- **Authentification** : le token utilisateur n'est **pas** un cookie -> il vit
  dans le **`localStorage`** de `discord.com` (en-tete `Authorization`).

## Integration `aphrody-hermes`

`aphrody-hermes` (crate Rust, agent voice-to-voice Discord + X) doit, d'apres
ces notes :

- s'authentifier en **bot** (`Authorization: Bot <token>`) ou en utilisateur
  (token nu), selon le mode ;
- ouvrir une connexion **gateway** avec les intents requis (notamment
  `MESSAGE_CONTENT` pour lire le contenu des messages) ;
- repondre via `EventHandler::interaction_create` pour les commandes slash
  (le `StandardFramework` texte est deprecie en serenity 0.12) ;
- pour la voix, suivre `songbird` + les types `voice-model` (gateway voix
  separe, RTP/Opus) — cf. `serenity-framework-voice.md`.

## Methode et reproductibilite

| Source | Emplacement (gitignore) |
|---|---|
| Code serenity clone | `var/serenity/` |
| Inventaire de l'install client | `var/discord/discord-scan.json` (+ `scan-discord.ps1`) |
| Sorties `aphrody re` | `var/discord/re/` |
| Listings asar | `var/discord/asar-*.list.txt` |
| Recon DNS/IP | `var/discord/recon-discord.json` |
| Cookies (metadonnees, sans valeurs) | `var/discord/cookies.json` |

Les artefacts bruts restent hors versionnement (`var/`) ; ces documents en
sont le distillat versionnable.
