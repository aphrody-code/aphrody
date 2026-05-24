<!--
SPDX-License-Identifier: Apache-2.0
Cartographie reseau de Discord (DNS / IP / sous-domaines / frameworks / auth).
Source : aphrody dns_recon + advanced_recon (MCP) + extraits asar du client.
-->

# Discord — cartographie reseau, frameworks et authentification

Releve de reconnaissance sur l'infrastructure publique de Discord, produit par
`aphrody dns_recon` + `aphrody advanced_recon`, corrobore par les chaines
extraites du client desktop (cf. [`client-electron-re.md`](client-electron-re.md)).

> Perimetre : recon passive/legere (resolution DNS, sondage des ports web
> standards 80/443) sur des services publics, a des fins de documentation
> d'interoperabilite. Aucun test intrusif, aucune charge, aucun endpoint
> authentifie n'est appele.

## 1. Domaines et edge

| Cible | Constat |
|---|---|
| `discord.com` (A) | `162.159.136.232`, `.137.232`, `.135.232`, `.138.232`, `162.159.128.233` |
| Edge | **Cloudflare** (plage `162.159.0.0/16`) — les A sont des IP edge, pas l'origine |
| Ports | `80` ouvert, `443` ouvert |
| `gateway.discord.gg` | endpoint **WebSocket** du gateway temps reel (domaine `.gg` distinct) |

Discord est integralement fronte par Cloudflare : l'origine reelle n'est pas
exposee, et le TLS/anti-DDoS/cache est gere au bord.

## 2. Sous-domaines (`discord.com` — 51 trouves)

Regroupes par fonction :

- **Application / API** : `discord.com`, `app.discord.com`, `i18n.discord.com`.
- **Canaux de release** : `canary.discord.com` (Canary), `ptb.discord.com`
  (Public Test Build) — en plus du canal Stable. Confirme les trois canaux
  vus cote client (`client-electron-re.md`).
- **Assets statiques** : `static.discord.com`, `static-edge.discord.com`.
- **Voix / video** : `rtc-sfu-finder.discord.com` — decouverte des serveurs
  media (SFU, Selective Forwarding Unit).
- **Telemetrie / ops** : `otelcol.discord.com` (collecteur OpenTelemetry),
  `androiddiag.discord.com`, `prod-wf1/2/3.discord.com`.
- **Marketing / support** : `blog`, `merch`, `events`, `feedback`, `docs`,
  `creator-support`, `ads`, `click`, `creator-support`.

CDN (hors zone `discord.com`, vus dans le client) : `cdn.discordapp.com`
(pieces jointes, avatars, emojis) et le domaine media historique
`discordapp.com`.

## 3. Surface API et gateway

L'URL de l'API et celle du gateway ne sont **pas** codees en dur dans le shell
desktop (cf. `client-electron-re.md` section 3) : elles proviennent du bundle
web. Cote protocole stable, la reference est la pile serenity :

- **API REST** : base `https://discord.com/api/v10/` (API **v10**) — voir
  [`serenity-rest-http.md`](serenity-rest-http.md) (auth Bot/Bearer,
  rate-limiting par bucket, routes messages/channels/guilds/interactions).
- **Gateway** : `wss://gateway.discord.gg` — voir
  [`serenity-gateway.md`](serenity-gateway.md) (IDENTIFY/READY, heartbeat,
  RESUME, 12 opcodes, 21 intents).

## 4. Frameworks

### Frontend (preuve mixte)
- **Hote** : preuve directe — le client desktop est un Chromium `138.0.7204.251`
  qui charge l'app web a distance ; les asar embarquent des bundles `webpack`
  minifies (`bundle.js`) et la police maison `gg sans`.
- **Couche applicative** : l'app web de Discord est une **SPA React** avec un
  store de type Flux (architecture publiquement documentee). Notre RE confirme
  l'hote Chromium/Electron et le packaging webpack ; le code React lui-meme
  reside dans le bundle servi par `discord.com`, hors des asar locaux.

### Backend (preuve mixte)
- **Edge** : preuve directe — **Cloudflare** (IP A, section 1).
- **Gateway temps reel** : forte presomption **Erlang/Elixir**, corroboree par
  le RE du client : le module natif `discord_erlpack.node` implemente l'**ETF**
  (External Term Format d'Erlang), format d'encodage du flux gateway, double
  d'une compression `discord_zstd.node` (Zstandard). L'usage natif de l'ETF
  cote client est l'empreinte directe d'un backend de famille Erlang.
- **API REST** : derriere Cloudflare ; l'architecture historiquement decrite
  par Discord est un service applicatif Python en voie de migration vers des
  services Rust/Elixir. Non verifiable en boite noire au-dela de l'edge.

## 5. Modele d'authentification (important)

Point de RE souvent mal compris, central pour toute interop :

- **Le token d'authentification de l'utilisateur N'EST PAS un cookie.** Le
  client web/desktop stocke le token de session dans le **`localStorage`**
  (cle `token`) de l'origine `discord.com`. Sur le profil Chrome, cela vit
  dans la base LevelDB `Local Storage\leveldb\` du profil.
- **Les cookies `discord.com`** sont surtout des cookies d'edge/CDN
  (Cloudflare : `__cfruid`, `__dcfduid`, `__sdcfduid`, `cf_clearance`) et de
  preference (`locale`). Ils ne portent pas la session applicative.
- **Consequence** : extraire les cookies (`aphrody chromium`) capture l'etat
  Cloudflare/CDN mais **pas** le token d'API. Pour un client authentifie, c'est
  la cle `token` du `localStorage` qui fait foi (et qui est envoyee dans
  l'en-tete `Authorization` des requetes `/api/v10/`, sans prefixe pour un
  token utilisateur, prefixe `Bot ` pour un bot).

### Cookies observes (extraction locale de validation, sans valeurs)

Extraction via `aphrody chromium` (cookies App-Bound Encryption Chrome 127+,
schema v20, dechiffres par `IElevator::DecryptData` COM ; deux cookies v10 en
DPAPI). Base verrouillee par Chrome en cours d'execution -> contournement par
copie VSS (shadow copy). Les cookies `discord.com` observes confirment le
modele : aucun ne porte la session applicative.

| Cookie | Famille | Role |
|---|---|---|
| `cf_clearance`, `__cf_bm`, `_cfuvid` | Cloudflare | anti-bot / challenge edge |
| `__dcfduid`, `__sdcfduid` | Discord + Cloudflare | identifiants appareil/edge (v10/DPAPI) |
| `_ga`, `_ga_*` | Google Analytics | mesure d'audience |
| `locale` | preference | langue de l'UI |

Le token d'API n'apparait dans **aucun** cookie ; il reste dans le
`localStorage` de l'origine `https://discord.com`, soit la base LevelDB :

```
%LOCALAPPDATA%\Google\Chrome\User Data\<profil>\Local Storage\leveldb\
```

> Le releve brut (noms, domaines, flags, expirations — jamais les valeurs) est
> conserve hors versionnement sous `var/discord/cookies.json` (gitignore).
> Cette note ne reproduit aucune valeur de secret.

## 6. Reproduire

```bash
# DNS OSINT + sous-domaines
aphrody dns recon discord.com
aphrody dns recon gateway.discord.gg

# DNS + sondage TCP des ports web
aphrody dns advanced discord.com --ports 80,443   # selon la CLI; cf. MCP advanced_recon
```

Artefact brut : `var/discord/recon-discord.json`.
