<!--
SPDX-License-Identifier: Apache-2.0
Documentation technique de la pile voix @discordjs/voice (reference JS).
Destination : guider l'implementation de aphrody-hermes (Rust, songbird).
Sources : var/discord/discord.js/packages/voice/src/ + discord-api-types voice/v8.d.ts
-->

# Pile voix Discord -- reference @discordjs/voice 0.19.2

Documentation technique complete de la bibliotheque `@discordjs/voice` telle
qu'observee dans le source TypeScript de reference. Destinee a `aphrody-hermes`
(agent voix-a-voix Discord en Rust, [`serenity-framework-voice.md`](serenity-framework-voice.md))
et aux validations protocolaires etablies dans [`client-poc.md`](client-poc.md).

---

## 1. Vue d'ensemble du flux d'etablissement

```
Bot (VoiceConnection)
  |
  |-- GatewayOpcodes.VoiceStateUpdate --> Main WebSocket (Discord gateway v10)
  |                                       (guild_id, channel_id, self_deaf, self_mute)
  |<-- VOICE_STATE_UPDATE                 (session_id, user_id, channel_id, ...)
  |<-- VOICE_SERVER_UPDATE               (endpoint, guild_id, token)
  |
  |-- wss://<endpoint>?v=8 --> Voice WebSocket (gateway voix v8)
  |     Identify(op=0)       (server_id, user_id, session_id, token, max_dave_protocol_version)
  |<-- Hello(op=8)           (heartbeat_interval)
  |<-- Ready(op=2)           (ssrc, ip, port, modes[])
  |
  |-- UDP socket ouvert vers ip:port
  |     IP discovery (74 octets, type=1, SSRC)
  |<-- IP discovery response (type=2, IP public + port du client)
  |
  |-- SelectProtocol(op=1)   (protocol="udp", address, port, mode)
  |<-- SessionDescription(op=4)  (mode, secret_key[32], dave_protocol_version)
  |
  | [si dave_protocol_version > 0] --> echange MLS (DAVE, opcodes 21-31, binaires)
  |
  |-- Speaking(op=5)         (ssrc, speaking=1, delay=0)
  |-- UDP RTP/Opus chiffre ---> Discord voice server
  |<-- UDP RTP/Opus chiffre (reception)
```

**Deux paquets de signalement** du gateway principal sont requis avant de creer
la connexion voix (`VoiceConnection.ts:228-231`) : `VOICE_STATE_UPDATE` (porte
`session_id`, `user_id`) et `VOICE_SERVER_UPDATE` (porte `endpoint`, `token`).
Voir `DataStore.ts:19-30` pour la construction du payload `VoiceStateUpdate`.

---

## 2. Adaptateur (pont gateway principal)

Le gateway voix est separe du gateway principal. L'architecture prevoit un pont
explicite : `DiscordGatewayAdapterCreator` (`util/adapter.ts:50-52`). L'adaptateur
expose deux methodes cote library :

- `onVoiceServerUpdate(data)` : alimente `VoiceConnection.addServerPacket()`
  (`VoiceConnection.ts:351`)
- `onVoiceStateUpdate(data)` : alimente `VoiceConnection.addStatePacket()`
  (`VoiceConnection.ts:370`)

Et une methode cote implementeur : `sendPayload(payload)` -- envoie un opcode
`VoiceStateUpdate` sur le gateway principal. C'est le seul couplage entre les
deux couches.

---

## 3. Machine a etats VoiceConnection

Definie dans `VoiceConnection.ts:24-49`. Cinq etats :

| Etat | Description |
|---|---|
| `Signalling` | Paquet `VoiceStateUpdate` envoye au gateway principal ; attente des deux paquets de reponse |
| `Connecting` | Les deux paquets recus ; `Networking` en cours d'etablissement |
| `Ready` | Connexion operationnelle, audio jouable |
| `Disconnected` | Connexion coupee (WebSocket close, adaptateur indisponible, endpoint null, ou deconnexion manuelle) |
| `Destroyed` | Definitif, non recuperable |

Transitions notables (`VoiceConnection.ts:464-513`) :

- Code de fermeture `4014` (Disconnected -- ne pas reconnecter) : etat
  `Disconnected` avec raison `WebSocketClose`. L'utilisateur doit appeler
  `rejoin()` manuellement.
- Tout autre code : le bot retourne automatiquement a `Signalling` et renvoie
  un paquet `VoiceStateUpdate` pour tenter de rejoindre le canal.

---

## 4. Machine a etats reseau (Networking)

Classe `Networking` dans `networking/Networking.ts:35-43`. Sept etats internes,
dans l'ordre chronologique d'etablissement :

| Code | Etat | Description |
|---|---|---|
| 0 | `OpeningWs` | Connexion WebSocket au gateway voix en cours |
| 1 | `Identifying` | `Identify` envoye, attente de `Ready` (op=2) |
| 2 | `UdpHandshaking` | Socket UDP ouvert, IP discovery en cours |
| 3 | `SelectingProtocol` | `SelectProtocol` envoye, attente de `SessionDescription` |
| 4 | `Ready` | Connexion entierement etablie, audio possible |
| 5 | `Resuming` | Reprise apres coupure reseau |
| 6 | `Closed` | Definitif |

L'URL du gateway voix est construite dans `Networking.ts:344` :
`wss://<endpoint>?v=8` -- version 8 du gateway voix.

---

## 5. Gateway voix WebSocket (VoiceWebSocket)

Classe `VoiceWebSocket` dans `networking/VoiceWebSocket.ts`. Surcouche de `ws`
qui gere le heartbeat et la distinction messages JSON / binaires.

### 5.1. Opcodes (discord-api-types voice/v8.d.ts, complets et verifies)

| Opcode | Valeur | Direction | Role |
|---|---|---|---|
| `Identify` | 0 | Client -> Serveur | Ouverture de session ; envoie server_id, user_id, session_id, token, max_dave_protocol_version |
| `SelectProtocol` | 1 | Client -> Serveur | Choisit UDP + mode de chiffrement + IP:port locale |
| `Ready` | 2 | Serveur -> Client | Repond a Identify ; donne ssrc, ip, port UDP, modes[] |
| `Heartbeat` | 3 | Client -> Serveur | Nonce horodatage + seq_ack |
| `SessionDescription` | 4 | Serveur -> Client | Cle secrete 32 octets, mode definitif, dave_protocol_version |
| `Speaking` | 5 | Bidirectionnel | Indicateur de prise de parole (ssrc, flags, delay) |
| `HeartbeatAck` | 6 | Serveur -> Client | Accuse de reception du heartbeat |
| `Resume` | 7 | Client -> Serveur | Reprise de session existante (seq_ack, token, session_id) |
| `Hello` | 8 | Serveur -> Client | Premier paquet recu ; donne heartbeat_interval (ms) |
| `Resumed` | 9 | Serveur -> Client | Accuse de reception de Resume |
| `ClientsConnect` | 11 | Serveur -> Client | Un ou plusieurs utilisateurs rejoignent le canal |
| `ClientDisconnect` | 13 | Serveur -> Client | Un utilisateur quitte le canal |
| `DavePrepareTransition` | 21 | Serveur -> Client | Annonce une transition de protocole DAVE imminente |
| `DaveExecuteTransition` | 22 | Serveur -> Client | Ordonne l'execution d'une transition DAVE |
| `DaveTransitionReady` | 23 | Client -> Serveur | Accuse pret pour la transition |
| `DavePrepareEpoch` | 24 | Serveur -> Client | Annonce une nouvelle epoque MLS (epoch=1 declenche reinit) |
| `DaveMlsExternalSender` | 25 | Serveur -> Client | Binaire -- cle publique de l'expediteur externe MLS |
| `DaveMlsKeyPackage` | 26 | Client -> Serveur | Binaire -- key package MLS du client |
| `DaveMlsProposals` | 27 | Serveur -> Client | Binaire -- propositions MLS (ajout/retrait de membres) |
| `DaveMlsCommitWelcome` | 28 | Client -> Serveur | Binaire -- commit + welcome MLS optionnel |
| `DaveMlsAnnounceCommitTransition` | 29 | Serveur -> Client | Binaire -- commit MLS pour transition |
| `DaveMlsWelcome` | 30 | Serveur -> Client | Binaire -- welcome MLS pour transition |
| `DaveMlsInvalidCommitWelcome` | 31 | Client -> Serveur | Signale un commit/welcome invalide ; demande re-ajout |

### 5.2. Handshake complet

1. Connexion WS etablie -> `onWsOpen` (`Networking.ts:407`) : envoie `Identify`
   (op=0) avec `max_dave_protocol_version` issu de `DAVESession.getMaxProtocolVersion()`
   (ou 0 si DAVE desactive). Passage a l'etat `Identifying`.
2. Reception `Hello` (op=8) : `VoiceWebSocket.setHeartbeatInterval(heartbeat_interval)`
   (`VoiceWebSocket.ts:222`) -- demarrage du timer de heartbeat.
3. Reception `Ready` (op=2) : `Networking.ts:479` -- creation du socket UDP, IP
   discovery, passage a `UdpHandshaking`.
4. IP discovery terminee -> envoi `SelectProtocol` (op=1) avec le mode de
   chiffrement choisi parmi ceux proposes par le serveur.
5. Reception `SessionDescription` (op=4) : cle secrete, mode definitif, version
   DAVE. Passage a l'etat `Ready`. Si `dave_protocol_version > 0`, creation d'une
   `DAVESession` (`Networking.ts:527`).

### 5.3. Heartbeat

Le heartbeat (`VoiceWebSocket.ts:202-214`) envoie :
```json
{ "op": 3, "d": { "t": <timestamp_ms>, "seq_ack": <last_seq> } }
```
Le serveur repond avec `HeartbeatAck` (op=6) contenant le meme nonce `t`. Si
trois heartbeats consecutifs restent sans reponse, le WebSocket est ferme
(`VoiceWebSocket.ts:226-229`).

### 5.4. Reprise de session

Codes de fermeture recuperables : `code < 4000` ou `code === 4015`
(`Networking.ts:443`). Dans ce cas, un nouveau WebSocket est ouvert sur le meme
endpoint et envoie `Resume` (op=7) avec `seq_ack` (derniere sequence connue).
Le serveur repond `Resumed` (op=9).

Codes de fermeture notables (`VoiceCloseCodes`) :
- `4014` (Disconnected) : ne pas reconnecter, canal supprime ou kick.
- `4015` (VoiceServerCrashed) : recuperable, tenter resume.
- `4017` (EndToEndEncryptionDAVEProtocolRequired) : le serveur exige DAVE.

### 5.5. Messages binaires

Les opcodes DAVE de type binaire (op 25-31) ne passent pas par JSON. Le format
sur le fil est `[opcode:1 octet][payload:N octets]` (emission) et
`[seq:2 octets][op:1 octet][payload:N octets]` (reception) d'apres
`VoiceWebSocket.ts:129-136`. Le champ `seq` binaire sert au meme compteur de
sequence que les paquets JSON.

---

## 6. UDP -- IP discovery et structure RTP

### 6.1. Socket UDP

`VoiceUDPSocket` (`networking/VoiceUDPSocket.ts`) cree un socket UDP (`udp4`),
en IPv4 uniquement. Un keepalive toutes les 5 secondes (`KEEP_ALIVE_INTERVAL = 5000ms`,
`VoiceUDPSocket.ts:37`) envoie 8 octets (compteur uint32LE zero-padde) pour
maintenir les tables NAT.

### 6.2. IP discovery

Avant de pouvoir negocier le chiffrement, le client doit connaitre son IP et
port publics (tels que vus par le serveur Discord, apres NAT). Protocole
(`VoiceUDPSocket.ts:166-171`) :

- Envoi d'un paquet de 74 octets : `[type=1:2][length=70:2][ssrc:4][zeros:66]`
- Le serveur repond avec `[type=2:2][length=70:2][ssrc:4][ip:null-terminated][...][port:2 (LE, derniers octets)]`
- `parseLocalPacket` (`VoiceUDPSocket.ts:20-31`) extrait l'IP (octets 8 jusqu'au
  premier octet nul) et le port (2 derniers octets big-endian).

L'IP et le port extraits sont ensuite communiques au gateway voix dans
`SelectProtocol`.

### 6.3. Structure d'un paquet RTP emis

Construit dans `Networking.createAudioPacket` (`Networking.ts:747-758`) :

```
[RTP header : 12 octets]
  octet 0 : 0x80  (V=2, P=0, X=0, CC=0)
  octet 1 : 0x78  (payload type Opus = 120, M=0) -- constants.ts:1
  octets 2-3 : sequence number (big-endian, 16 bits, incrementiel)
  octets 4-7 : timestamp (big-endian, 32 bits, increment de 960*2=1920 par trame)
  octets 8-11 : SSRC (big-endian, 32 bits, attribue par le serveur dans Ready)
[payload chiffre : N octets]
[nonce de 4 octets (partie basse du compteur de nonce, big-endian)]
```

L'increment de timestamp est `(48000 / 100) * 2 = 960` par voie * 2 voies =
`TIMESTAMP_INC = 1920` (`Networking.ts:20-21`). Avec une trame de 20 ms a
48 kHz stereo, 960 echantillons par voie.

---

## 7. Chiffrement

### 7.1. Modes supportes

Deux modes de chiffrement modernes sont supportes (`Networking.ts:23-28`) :

| Identifiant | Statut | Cle | Nonce |
|---|---|---|---|
| `aead_aes256_gcm_rtpsize` | Prefere (si AES-GCM disponible en natif) | 32 octets | 12 octets |
| `aead_xchacha20_poly1305_rtpsize` | Fallback universel | 32 octets | 24 octets |

Modes anciens (definis dans `VoiceEncryptionMode` mais marques `@deprecated`) :
`xsalsa20_poly1305_lite_rtpsize`, `aead_aes256_gcm`, `xsalsa20_poly1305`,
`xsalsa20_poly1305_suffix`, `xsalsa20_poly1305_lite`. Ils ne sont plus emis
par le serveur et ne sont pas implementes dans la branche active de `Networking`.

La selection (`Networking.ts:211-218`) choisit le premier mode de la liste
`SUPPORTED_ENCRYPTION_MODES` qui figure dans les `modes[]` retournes par le
serveur. `aead_aes256_gcm_rtpsize` est ajoute en tete de liste uniquement si
`crypto.getCiphers().includes('aes-256-gcm')` est vrai, sinon le seul mode
propose est `aead_xchacha20_poly1305_rtpsize`.

### 7.2. Nonce

Le nonce est un entier 32 bits incremente apres chaque paquet (`Networking.ts:779-784`),
mis a zero quand il depasse `2^32 - 1`. Il est ecrit en big-endian dans les 4
premiers octets d'un buffer de 12 ou 24 octets selon le mode. Seuls ces 4
premiers octets sont utilises comme suffixe dans le paquet UDP (`noncePadding =
nonceBuffer.subarray(0, 4)`, `Networking.ts:784`).

### 7.3. AAD (donnees authentifiees additionnelles)

Dans les deux modes `_rtpsize`, l'en-tete RTP de 12 octets joue le role d'AAD
(Associated Additional Data) passe au chiffrement AEAD (`Networking.ts:793`, `798`).
Il garantit l'integrite de l'en-tete sans le chiffrer.

### 7.4. Structure complete du paquet UDP

```
[RTP header : 12 octets]  <- AAD, non chiffre
[ciphertext : N+16 octets] <- opus chiffre (ou opus+DAVE chiffre) + auth tag 16 octets
[nonce padding : 4 octets] <- 4 premiers octets du compteur de nonce, big-endian
```

### 7.5. Libraries XChaCha20 (secretbox)

`util/Secretbox.ts` tente de charger la premiere disponible dans l'ordre :
1. `sodium-native` (binding natif libsodium)
2. `sodium` (binding libsodium JS)
3. `libsodium-wrappers` (WASM)
4. `@stablelib/xchacha20poly1305` (pur JS)
5. `@noble/ciphers` (pur JS, audit cryptographique noble)

Si aucune n'est installee, toute tentative de chiffrement lance une exception.
AES-256-GCM est traite directement par le module `node:crypto` de Node.js, sans
bibliotheque supplementaire.

---

## 8. DAVE -- chiffrement de bout en bout

### 8.1. Principe

DAVE (Discord Audio & Video End-to-End Encryption) ajoute une couche de
chiffrement au-dessus du chiffrement de transport. Il s'appuie sur **MLS**
(Messaging Layer Security, RFC 9420), un protocole de chiffrement de groupe
assure post-compromission avec une cle derivee du groupe MLS.

DAVE est active par defaut (`joinVoiceChannel.ts:14`, `daveEncryption = true`).
Il peut etre desactive pour la compatibilite (`daveEncryption: false`).

### 8.2. Role de @snazzah/davey

`@snazzah/davey` (`DAVESession.ts:3`) est la bibliotheque JavaScript qui
encapsule l'implementation MLS de DAVE. Elle expose :

- `Davey.DAVESession(protocolVersion, userId, channelId)` -- session MLS de
  groupe pour un canal.
- `Davey.DAVE_PROTOCOL_VERSION` -- version maximale supportee, lue pour
  construire le champ `max_dave_protocol_version` dans `Identify`.
- `Davey.MediaType.AUDIO` -- constante de type media pour le dechiffrement.
- Methodes de session : `getSerializedKeyPackage()`, `setExternalSender()`,
  `processProposals()`, `processCommit()`, `processWelcome()`,
  `encryptOpus()`, `decrypt()`, `voicePrivacyCode`, `getVerificationCode()`,
  `setPassthroughMode()`, `canPassthrough()`, `reset()`, `reinit()`.

**Lien avec le natif** : le module `discord_voice.node` (Electron 37.6.0,
analyse dans [`client-poc.md`](client-poc.md)) exporte `getMLSSigningKey` et
`SupportedSecureFramesProtocolVersion`. Ce sont les points d'entree C++ du meme
protocole DAVE cote client Discord officiel -- l'implementation native de ce que
`@snazzah/davey` reproduit en JS/WASM.

### 8.3. Flux MLS au premier etablissement

1. `DAVESession.reinit()` (`DAVESession.ts:159`) : cree une `Davey.DAVESession`
   et emet immediatement le `keyPackage` (op=26 binaire) vers le serveur.
2. Le serveur envoie `DaveMlsExternalSender` (op=25 binaire) : `setExternalSender()`
   configure l'expediteur externe MLS.
3. Le serveur envoie `DaveMlsProposals` (op=27 binaire) : `processProposals()`
   retourne un commit + welcome optionnel, envoyes en `DaveMlsCommitWelcome`
   (op=28 binaire).
4. Le serveur envoie `DaveMlsAnnounceCommitTransition` (op=29) ou
   `DaveMlsWelcome` (op=30) : traitement de la transition.
5. Une fois le commit/welcome traite avec succes, la session est `ready` ; les
   paquets Opus sont passes dans `daveSession.encrypt()` avant le chiffrement
   de transport.

### 8.4. Transitions et epochs

- `DavePrepareTransition` (op=21) : annonce une transition (changement de version
  ou de composition du groupe). Si `transition_id === 0`, execute immediatement.
  Si `protocol_version === 0`, bascule en mode passthrough (downgrade).
- `DaveExecuteTransition` (op=22) : execute la transition preparee.
- `DavePrepareEpoch` (op=24) : si `epoch === 1`, reinitialise la session avec
  la nouvelle version de protocole (`DAVESession.ts:252-254`).

### 8.5. Tolerance aux echecs de dechiffrement

`DEFAULT_DECRYPTION_FAILURE_TOLERANCE = 36` (`DAVESession.ts:45`). Si 36 echecs
consecutifs sont detectes, la session invoque `recoverFromInvalidTransition()` :
elle reinitialise et emet `DaveMlsInvalidCommitWelcome` (op=31) au serveur pour
demander un re-ajout au groupe MLS.

### 8.6. Double couche de chiffrement en transit

Lorsque DAVE est actif, le paquet Opus passe d'abord dans `daveSession.encrypt()`
(couche MLS, `DAVESession.ts:350-352`), puis le resultat est chiffre par AEAD
(couche de transport). Cote reception, l'ordre inverse est applique dans
`VoiceReceiver.parsePacket` (`receive/VoiceReceiver.ts:137-168`) : AEAD d'abord,
DAVE ensuite.

---

## 9. Pipeline audio (emission)

### 9.1. AudioPlayer

`AudioPlayer` (`audio/AudioPlayer.ts`) est la boucle de lecture. Il est pilote
par un timer global dans `DataStore.ts:116-149` qui appelle `_stepDispatch()` +
`_stepPrepare()` toutes les 20 ms via `setTimeout` a rappel. Ce timer est
partage entre tous les players actifs.

Cinq etats : `Idle`, `Buffering`, `Playing`, `Paused`, `AutoPaused`.

Comportement sans abonne (`noSubscriber`, defaut `Pause`) : si aucune connexion
ne peut recevoir les paquets, le player se met en pause automatique
(`AutoPaused`). Il reprend des que des connexions deviennent disponibles.

### 9.2. AudioResource et TransformerGraph

`AudioResource` (`audio/AudioResource.ts`) emballe un stream Readable object-mode
qui emet des paquets Opus bruts. La construction du pipeline de transcodage est
faite par `findPipeline` (`audio/TransformerGraph.ts:279`) via une recherche
de plus court chemin (Dijkstra simplifie, profondeur 5) sur un graphe de
transformateurs :

| Entree | Transformateur | Sortie |
|---|---|---|
| Raw PCM s16le | `prism.opus.Encoder(48000, 2, 960)` | Opus |
| OggOpus | `prism.opus.OggDemuxer` | Opus |
| WebmOpus | `prism.opus.WebmDemuxer` | Opus |
| Opus | `prism.opus.Decoder(48000, 2, 960)` | Raw |
| Arbitraire | `prism.FFmpeg` (pcm) | Raw |
| Arbitraire | `prism.FFmpeg` (ogg, si libopus) | OggOpus |
| Raw | `prism.VolumeTransformer` | Raw (avec gain) |

`prism-media` encapsule `@discordjs/opus` (binding N-API) ou `opusscript`
(WASM) pour les transformateurs Opus. `frameSize = 960` echantillons a 48 kHz
= 20 ms de trame, conform au protocole Discord.

Le silence est le paquet `[0xf8, 0xff, 0xfe]` (`AudioPlayer.ts:12`), 3 octets
(trame Opus valide signifiant "confort noise" / silence). Il est injecte quand
le player est en pause ou en fin de ressource (5 trames de rembourrage par
defaut, `silencePaddingFrames = 5`).

### 9.3. Envoi

`AudioPlayer._stepPrepare()` lit un paquet Opus de la ressource, appelle
`VoiceConnection.prepareAudioPacket(packet)` sur chaque connexion abonnee.
Puis `_stepDispatch()` appelle `VoiceConnection.dispatchAudio()` qui appelle
`state.udp.send(audioPacket)`. Cette separation `prepare`/`dispatch` permet
de repartir la charge sur plusieurs connexions simultanement.

---

## 10. Reception audio

### 10.1. VoiceReceiver

`VoiceReceiver` (`receive/VoiceReceiver.ts`) est attache a chaque `VoiceConnection`.
Il ecoute deux sources :

- `onWsPacket` : paquets WebSocket `Speaking` (op=5) et `ClientDisconnect`
  (op=13) pour maintenir la `SSRCMap`.
- `onUdpMessage` : paquets UDP entrants contenant les trames RTP des autres
  participants.

### 10.2. SSRCMap

`SSRCMap` (`receive/SSRCMap.ts`) maintient la correspondance `SSRC audio (uint32) <-> userId`.
Elle est mise a jour par les paquets `Speaking` (op=5) du gateway voix, qui
contiennent `ssrc` et `user_id`. La table permet a `onUdpMessage` de retrouver
le `userId` a partir du champ SSRC lu dans le paquet UDP (octets 8-11,
`VoiceReceiver.ts:178`).

### 10.3. Dechiffrement en reception

`VoiceReceiver.parsePacket` (`receive/VoiceReceiver.ts:137`) :

1. Copie les 4 derniers octets du paquet dans le debut du buffer de nonce (12
   ou 24 octets selon le mode). Les 4 derniers octets du paquet sont le
   `nonce padding` mis par l'emetteur.
2. Gere les extensions RTP (bit CSRC, marqueur 0xBEDE) en ajustant la taille
   d'en-tete.
3. Dechiffre avec le mode selectionne : `aead_aes256_gcm_rtpsize` (via
   `crypto.createDecipheriv`) ou `aead_xchacha20_poly1305_rtpsize` (via
   `secretbox.methods`).
4. Retire le rembourrage RFC 3550.
5. Si une `DAVESession` est active, dechiffre la couche DAVE supplementaire
   via `daveSession.decrypt(packet, userId)`.

Seuls les paquets avec `payload type = 0x78` (120, Opus) et version RTP = 2
sont traites (`VoiceReceiver.ts:190-193`).

### 10.4. SpeakingMap

`SpeakingMap` (`receive/SpeakingMap.ts`) deduit l'activite vocale a partir de
l'arrivee de paquets UDP : si un paquet UDP est recu d'un utilisateur, il est
marque comme parlant ; s'il n'y a plus de paquet pendant 100 ms
(`SpeakingMap.DELAY = 100`), il est marque comme ayant arrete. Les evenements
`start` et `end` sont emis.

### 10.5. AudioReceiveStream

Chaque abonnement `VoiceReceiver.subscribe(userId)` retourne un stream
`AudioReceiveStream` (Readable object-mode) qui emet des `Buffer` contenant des
paquets Opus dechiffres. Le stream se ferme automatiquement quand l'abonnement
est annule.

---

## 11. Detection des dependances (generateDependencyReport)

`util/generateDependencyReport.ts` produit un rapport des backends disponibles :

- **Opus** : `@discordjs/opus` (binding N-API, prefere) ou `opusscript` (WASM,
  fallback).
- **Chiffrement AES-256-GCM** : natif (`node:crypto`), detection via
  `getCiphers().includes('aes-256-gcm')`.
- **Chiffrement XChaCha20** : `sodium-native`, `sodium`, `libsodium-wrappers`,
  `@stablelib/xchacha20poly1305`, ou `@noble/ciphers` (premier trouve gagne).
- **DAVE** : `@snazzah/davey`.
- **FFmpeg** : detection via `prism.FFmpeg.getInfo()`, avec verification
  `--enable-libopus` pour les optimisations OGG directes.

---

## 12. Pertinence pour aphrody-hermes (implementation Rust)

`aphrody-hermes` est l'agent voix-a-voix Discord d'aphrody, implemente en Rust.
La pile de reference JS documentee ci-dessus permet d'identifier ce que `songbird`
(la bibliotheque Rust de voix Discord) couvre deja et ce qui reste a implementer.

### 12.1. Ce que songbird couvre deja

Songbird prend en charge les couches de base du protocole :
- Signalement gateway (VOICE_STATE_UPDATE / VOICE_SERVER_UPDATE)
- Machine a etats de connexion voix
- Gateway voix WebSocket v8 (opcodes 0-9, heartbeat, resume)
- Socket UDP, IP discovery, keepalive
- Structure RTP + chiffrement AEAD (modes modernes)
- Pipeline Opus (encodage/decodage via le binding Opus)
- Emission et reception de trames audio

### 12.2. Le delta : DAVE (E2EE)

Le delta significatif entre une implementation songbird de base et la reference
JS est le protocole **DAVE** (opcodes 21-31). Songbird ne l'implemente pas
nativement en 2026. Pour `aphrody-hermes`, les options sont :

1. **Portage Rust de la logique MLS** : reimplementer `DAVESession` en Rust avec
   une bibliotheque MLS Rust (par exemple `openmls`). La specification publique
   est disponible sur `daveprotocol.com`.
2. **Bridge FFI vers @snazzah/davey** : appeler la bibliotheque JS via Bun ou
   Node en FFI. Moins propre, mais operationnel rapidement.
3. **Desactiver DAVE** : passer `max_dave_protocol_version = 0` dans `Identify`.
   Le serveur ne demandera pas DAVE, au prix de perdre le E2EE. Certains canaux
   pourraient exiger DAVE (close code `4017`).

**Lien avec le module natif** : `discord_voice.node` exporte
`getMLSSigningKey` et `SupportedSecureFramesProtocolVersion` (voir
[`client-poc.md`](client-poc.md), bonus PoC). C'est l'implementation C++ de
DAVE dans le client Discord officiel -- le code natif qui signe les key packages
MLS et annonce la version supportee. L'interface est la meme que celle exposee
par `@snazzah/davey` (session MLS de groupe, key package, commit/welcome).

### 12.3. Ce que hermes doit implementer en propre

Au-dela de la couche protocole :

- **Pipeline voice-to-voice** : transcription (STT) entree -> traitement LLM
  (Gemini/Antigravity) -> synthese vocale (TTS) -> emission Opus. La reception
  `VoiceReceiver` + `SSRCMap` + `SpeakingMap` decrivent le modele de detection
  de locuteur a repliquer.
- **Multi-canal** : hermes gere simultanement Discord + X (voir memoire
  `hermes-agent-surpass`). La separation du `AudioPlayer` et du `VoiceConnection`
  en JS (un player -> N connexions) est un pattern utile a reprendre.
- **SSRC et speaking** : emettre `Speaking` (op=5) avant de commencer a envoyer
  des trames, et emettre `speaking=0` apres. Songbird gere cela mais il faut
  s'assurer que hermes le declenche correctement pour les reponses syntheses.

### 12.4. PoC Opus etabli

Le PoC `var/discord/opus-poc/` valide le binding `@discordjs/opus` sous Bun et
Node : `OpusEncoder(48000, 2)`, trame 20 ms = 3840 octets PCM s16le ->
~388 octets Opus (compression ~9,9x), round-trip encode/decode integre. Cote
Rust, l'equivalent est le crate `audiopus` (binding libopus) ou `opus` crate,
avec les memes parametres (48000 Hz, 2 voies, 960 echantillons/trame).

---

## Références internes

- [`serenity-framework-voice.md`](serenity-framework-voice.md) -- crate serenity voice-model (Rust, reference protocole RTP/Discord cote Rust)
- [`client-poc.md`](client-poc.md) -- PoC RE discord_voice.node ; exports DAVE natifs (`getMLSSigningKey`, `SupportedSecureFramesProtocolVersion`)
- [`client-electron-re.md`](client-electron-re.md) -- architecture Electron et modules natifs Discord
- [`web-network-recon.md`](web-network-recon.md) -- gateway principal Discord (ETF/JSON, WSS)

Sources de code inspectees (chemins relatifs a `var/discord/discord.js/packages/voice/src/`) :
- `VoiceConnection.ts` -- machine a etats haute niveau
- `joinVoiceChannel.ts` -- API d'entree publique
- `DataStore.ts` -- boucle audio globale et creation du payload VoiceStateUpdate
- `networking/Networking.ts` -- orchestration reseau, chiffrement, DAVE
- `networking/VoiceWebSocket.ts` -- WebSocket avec heartbeat et binaire
- `networking/VoiceUDPSocket.ts` -- UDP, IP discovery, keepalive
- `networking/DAVESession.ts` -- protocole DAVE / MLS
- `audio/AudioPlayer.ts` -- player et timer 20 ms
- `audio/AudioResource.ts` -- resource + pipeline de transcodage
- `audio/TransformerGraph.ts` -- graphe de transformateurs FFmpeg/Opus
- `receive/VoiceReceiver.ts` -- reception, dechiffrement, DAVE inbound
- `receive/SSRCMap.ts` -- correspondance SSRC <-> userId
- `receive/SpeakingMap.ts` -- detection d'activite vocale
- `util/Secretbox.ts` -- selection de bibliotheque XChaCha20
- `util/constants.ts` -- RTP_OPUS_PAYLOAD_TYPE = 0x78 (120)
- `util/adapter.ts` -- interface adaptateur gateway
- `util/generateDependencyReport.ts` -- detection de backends disponibles

Types de reference : `discord-api-types/voice/v8.d.ts` (version 0.38.43),
`VoiceGatewayVersion = "8"`.
