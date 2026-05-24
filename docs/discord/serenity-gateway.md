# serenity 0.12.5 -- Protocole Gateway Discord

Documentation technique du module `gateway`. Source vérifiée dans
`var/serenity/src/gateway/` et `var/serenity/src/model/gateway.rs`.

---

## 1. Vue d'ensemble

Le Gateway Discord est une connexion WebSocket persistante via laquelle le bot
reçoit tous les événements en temps réel (messages, réactions, mises à jour
d'état vocal, etc.). La version de gateway utilisée par serenity est **10**
(`var/serenity/src/constants.rs:13`).

L'URL de connexion n'est pas codée en dur : elle est récupérée via l'endpoint
REST `GET /gateway/bot` avant chaque connexion, puis stockée dans un
`Arc<Mutex<String>>` partagé entre les shards pour permettre la mise à jour
lors d'une reconnexion.

---

## 2. Opcodes Gateway

Définis dans `var/serenity/src/constants.rs:33-68` sous forme d'enum `Opcode`
sérialisé depuis/vers `u8` :

| Opcode | Valeur | Direction | Description |
|--------|--------|-----------|-------------|
| `Dispatch` | 0 | Serveur -> Client | Envoi d'un événement (t + d + s) |
| `Heartbeat` | 1 | Bidirectionnel | Ping de maintien de connexion |
| `Identify` | 2 | Client -> Serveur | Handshake initial avec token + intents |
| `PresenceUpdate` | 3 | Client -> Serveur | Mise à jour du statut/activité |
| `VoiceStateUpdate` | 4 | Client -> Serveur | Rejoindre/quitter un canal vocal |
| `VoiceServerPing` | 5 | Client -> Serveur | Ping vocal (déprécié dans les nouvelles API) |
| `Resume` | 6 | Client -> Serveur | Reprise d'une session précédente |
| `Reconnect` | 7 | Serveur -> Client | Ordre de reconnexion |
| `RequestGuildMembers` | 8 | Client -> Serveur | Demande de membres d'un guild |
| `InvalidSession` | 9 | Serveur -> Client | Session invalide (booléen resumable) |
| `Hello` | 10 | Serveur -> Client | Intervalle heartbeat + infos serveur |
| `HeartbeatAck` | 11 | Serveur -> Client | Accusé réception du heartbeat |
| `ReqeustSoundboardSounds` | 31 | Client -> Serveur | Sons soundboard d'une liste de guilds |

La structure JSON de chaque frame est : `{"op": <u8>, "d": <données>, "s": <seq|null>, "t": <nom|null>}`.
L'opcode 0 (Dispatch) est le seul à porter `s` (numéro de séquence) et `t`
(nom de l'événement comme `"MESSAGE_CREATE"`).

---

## 3. Codes de fermeture WebSocket

Définis dans `var/serenity/src/constants.rs:71-120` (module `close_codes`) :

| Code | Constante | Peut résumer ? |
|------|-----------|----------------|
| 4000 | `UNKNOWN_ERROR` | Oui (reconnexion) |
| 4001 | `UNKNOWN_OPCODE` | Oui (resume) |
| 4002 | `DECODE_ERROR` | Oui (resume) |
| 4003 | `NOT_AUTHENTICATED` | Non |
| 4004 | `AUTHENTICATION_FAILED` | Non |
| 4005 | `ALREADY_AUTHENTICATED` | Oui (reconnexion) |
| 4007 | `INVALID_SEQUENCE` | Oui (reconnexion) |
| 4008 | `RATE_LIMITED` | Oui (resume) |
| 4009 | `SESSION_TIMEOUT` | Oui (reconnexion) |
| 4010 | `INVALID_SHARD` | Non |
| 4011 | `SHARDING_REQUIRED` | Non |
| 4013 | `INVALID_GATEWAY_INTENTS` | Non |
| 4014 | `DISALLOWED_GATEWAY_INTENTS` | Non |

La logique de décision resume/reidentify est dans
`var/serenity/src/gateway/shard.rs:384-391`.

---

## 4. Séquence de handshake

### 4.1. Connexion initiale

```
Client                           Serveur Discord
  |                                    |
  |-- TLS WebSocket connect(url) ----> |
  |                                    |
  |<-- op=10 Hello {"heartbeat_interval": N} --
  |
  | [stage = Handshake -> Identifying]
  |
  |-- op=2 Identify { ------------------->
  |     token, shard, intents,
  |     compress: true,
  |     large_threshold: 250,
  |     properties: {browser:"serenity", device:"serenity", os:<consts::OS>},
  |     presence: {status, activities}
  |   }
  |
  |<-- op=0 t="READY" d={session_id, resume_gateway_url, guilds, ...} --
  |
  | [stage = Connected]
```

Le `large_threshold` est fixé à 250 (`var/serenity/src/constants.rs:16`) : les
guilds avec plus de 250 membres ne sont pas livrés avec leur liste de membres
complète dans GUILD_CREATE -- il faut utiliser `RequestGuildMembers` (op=8).

La compression est activée à `true` dans le payload IDENTIFY
(`var/serenity/src/gateway/ws.rs:276`). Discord peut alors envoyer des frames
binaires compressées en zlib. La décompression est effectuée par `flate2::ZlibDecoder`
dans `WsClient::recv_json` (`var/serenity/src/gateway/ws.rs:128-144`).

### 4.2. Heartbeat

Après réception de l'opcode Hello, serenity stocke l'intervalle heartbeat en ms
(`var/serenity/src/gateway/shard.rs:438`). La méthode `do_heartbeat` est appelée
dans la boucle principale du `ShardRunner` et envoie :

```json
{"op": 1, "d": <seq_actuel_ou_null>}
```

Si le dernier heartbeat n'a pas reçu d'accusé réception (`HeartbeatAck`, op=11)
avant le prochain envoi, le shard se reconnecte automatiquement
(`var/serenity/src/gateway/shard.rs:502-505`).

### 4.3. Reprise de session (RESUME)

Lorsqu'une déconnexion est récupérable (session_id disponible, code != 4004),
serenity envoie :

```json
{"op": 6, "d": {"session_id": "...", "token": "Bot ...", "seq": <seq>}}
```

Si Discord accepte, il émet `t="RESUMED"`. Sinon, il invalide la session via
op=9 et le bot doit recommencer un IDENTIFY complet.

Le champ `resume_gateway_url` du payload READY fournit l'URL à utiliser pour les
reconnexions, distinct de l'URL de connexion initiale
(`var/serenity/src/model/gateway.rs:369`).

---

## 5. Machine d'états des shards

La structure `Shard` maintient un état interne `ConnectionStage`
(`var/serenity/src/gateway/mod.rs:171-189`) :

```
Disconnected
    |
    v
Handshake  --(Hello reçu)--> Identifying  --(READY reçu)--> Connected
                                 |
                         (si déjà en cours)
                                 |
                                 v
                            Resuming  --(RESUMED reçu)--> Connected
```

`Connecting` est un état composite qui regroupe `Handshake`, `Identifying` et
`Resuming` pour les tests `is_connecting()`.

---

## 6. GatewayIntents -- liste exhaustive

Définies comme bitflags `u64` dans `var/serenity/src/model/gateway.rs:438-609`.

| Constante | Bit | Événements activés | Privilégiée |
|-----------|-----|--------------------|-------------|
| `GUILDS` | 1 | GUILD_CREATE/UPDATE/DELETE, CHANNEL_CREATE/UPDATE/DELETE, THREAD_*, STAGE_INSTANCE_* | Non |
| `GUILD_MEMBERS` | 1 << 1 | GUILD_MEMBER_ADD/UPDATE/REMOVE, THREAD_MEMBERS_UPDATE | **Oui** |
| `GUILD_MODERATION` | 1 << 2 | GUILD_AUDIT_LOG_ENTRY_CREATE, GUILD_BAN_ADD/REMOVE | Non |
| `GUILD_EMOJIS_AND_STICKERS` | 1 << 3 | GUILD_EMOJIS_UPDATE, GUILD_STICKERS_UPDATE | Non |
| `GUILD_INTEGRATIONS` | 1 << 4 | GUILD_INTEGRATIONS_UPDATE, INTEGRATION_* | Non |
| `GUILD_WEBHOOKS` | 1 << 5 | WEBHOOKS_UPDATE | Non |
| `GUILD_INVITES` | 1 << 6 | INVITE_CREATE/DELETE | Non |
| `GUILD_VOICE_STATES` | 1 << 7 | VOICE_STATE_UPDATE | Non |
| `GUILD_PRESENCES` | 1 << 8 | PRESENCE_UPDATE | **Oui** |
| `GUILD_MESSAGES` | 1 << 9 | MESSAGE_CREATE/UPDATE/DELETE en guild | Non |
| `GUILD_MESSAGE_REACTIONS` | 1 << 10 | MESSAGE_REACTION_ADD/REMOVE/REMOVE_ALL/REMOVE_EMOJI en guild | Non |
| `GUILD_MESSAGE_TYPING` | 1 << 11 | TYPING_START en guild | Non |
| `DIRECT_MESSAGES` | 1 << 12 | MESSAGE_CREATE/UPDATE/DELETE en DM | Non |
| `DIRECT_MESSAGE_REACTIONS` | 1 << 13 | MESSAGE_REACTION_* en DM | Non |
| `DIRECT_MESSAGE_TYPING` | 1 << 14 | TYPING_START en DM | Non |
| `MESSAGE_CONTENT` | 1 << 15 | Contenu, attachments, embeds, components dans les messages | **Oui** |
| `GUILD_SCHEDULED_EVENTS` | 1 << 16 | GUILD_SCHEDULED_EVENT_* | Non |
| `AUTO_MODERATION_CONFIGURATION` | 1 << 20 | AUTO_MODERATION_RULE_* | Non |
| `AUTO_MODERATION_EXECUTION` | 1 << 21 | AUTO_MODERATION_ACTION_EXECUTION | Non |
| `GUILD_MESSAGE_POLLS` | 1 << 24 | MESSAGE_POLL_VOTE_ADD/REMOVE en guild | Non |
| `DIRECT_MESSAGE_POLLS` | 1 << 25 | MESSAGE_POLL_VOTE_ADD/REMOVE en DM | Non |

**Intents privilégiés** (`var/serenity/src/model/gateway.rs:624-628`) :
- `GUILD_MEMBERS` (bit 1)
- `GUILD_PRESENCES` (bit 8)
- `MESSAGE_CONTENT` (bit 15)

Ils doivent être activés dans le portail développeur Discord et déclarés
explicitement dans le code. Au-delà de 100 guilds, le bot doit être vérifié.

`GatewayIntents::non_privileged()` retourne tous les intents sauf ces trois.
`GatewayIntents::default()` vaut `non_privileged()`.

### Combinaison minimale pour aphrody-hermes

```rust
GatewayIntents::GUILD_MESSAGES
    | GatewayIntents::MESSAGE_CONTENT  // privilégié, requis pour lire le texte
    | GatewayIntents::GUILD_VOICE_STATES  // pour les events voix
    | GatewayIntents::DIRECT_MESSAGES
```

---

## 7. EventHandler et FullEvent

Le trait `EventHandler` est défini via macro dans
`var/serenity/src/client/event_handler.rs:8-81`. Il génère simultanément :
- le trait `EventHandler` avec une méthode `async` par événement (implémentation
  par défaut vide via `drop((...))`),
- l'enum `FullEvent` miroir avec une variante par événement,
- la méthode `FullEvent::dispatch(ctx, handler)` pour le dispatch.

Méthodes clés du trait `EventHandler` :

| Méthode | Événement déclencheur | Intent requis |
|---------|-----------------------|---------------|
| `message` | MESSAGE_CREATE | `GUILD_MESSAGES` ou `DIRECT_MESSAGES` |
| `ready` | READY | aucun |
| `interaction_create` | INTERACTION_CREATE | aucun |
| `voice_state_update` | VOICE_STATE_UPDATE | `GUILD_VOICE_STATES` |
| `guild_create` | GUILD_CREATE | `GUILDS` |
| `guild_member_addition` | GUILD_MEMBER_ADD | `GUILD_MEMBERS` (privilégié) |
| `presence_update` | PRESENCE_UPDATE | `GUILD_PRESENCES` (privilégié) |
| `reaction_add` | MESSAGE_REACTION_ADD | `GUILD_MESSAGE_REACTIONS` |
| `channel_create` | CHANNEL_CREATE | `GUILDS` |
| `cache_ready` | -- (interne, post-init) | `cache` feature |
| `ratelimit` | -- (interne ratelimiter) | aucun |
| `shard_stage_update` | -- (interne connexion shard) | aucun |

L'enum `Event` dans `var/serenity/src/model/event.rs` correspond au mapping
direct des noms d'événements Discord (ex. `Event::MessageCreate`,
`Event::GuildMemberAdd`, etc.).

---

## 8. Sharding

### 8.1. Pourquoi

Discord impose le sharding dès 2500 guilds. La recommandation est d'environ 1000
guilds par shard. Sans sharding, `Client::start()` ouvre un seul shard.

### 8.2. Méthodes de démarrage

```rust
client.start()                   // shard 0 sur 1
client.start_autosharded()       // récupère le nombre de shards recommandé via REST
client.start_shard(id, total)    // shard spécifique
client.start_shards(total)       // tous les shards en une instance
client.start_shard_range(range, total)  // plage de shards
```

### 8.3. Architecture ShardManager

Le `ShardManager` (`var/serenity/src/gateway/bridge/shard_manager.rs`) maintient :
- une file `ShardQueuer` qui lance les shards séquentiellement,
- une `HashMap<ShardId, ShardRunnerInfo>` des shards actifs,
- des canaux `mpsc` pour envoyer des messages aux runners (shutdown, restart, etc.).

Le `ShardMessenger` permet, depuis n'importe quelle tâche, d'envoyer des opcodes
au shard (mise à jour de présence, demande de membres, rejoint vocal).

### 8.4. Calcul d'appartenance shard

La formule Discord pour déterminer le shard d'une guild :
```
shard_id = (guild_id >> 22) % total_shards
```

Ceci n'est pas implémenté directement dans serenity (c'est une formule Discord
standard), mais le `ShardInfo` (`var/serenity/src/model/gateway.rs:393`) stocke
`{id: ShardId, total: u32}` et est sérialisé en tableau JSON `[id, total]` dans
le payload IDENTIFY.
