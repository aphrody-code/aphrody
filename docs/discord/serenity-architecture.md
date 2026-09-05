# serenity 0.12.5 -- Architecture générale

Documentation technique de référence. Source : `var/serenity/` (cloné en local).
Destiné à l'intégration Discord d'aphrody (crate `aphrody-hermes` et extensions futures).

---

## 1. Coordonnées du projet

| Champ | Valeur |
|-------|--------|
| Crate | `serenity` |
| Version | 0.12.5 |
| Licence | ISC |
| Edition Rust | 2021 |
| MSRV | 1.74 |
| Dépôt | https://github.com/serenity-rs/serenity |

Source : `var/serenity/Cargo.toml:1-27`.

---

## 2. Modèle asynchrone

serenity repose intégralement sur **tokio** (version 1.34+). Les features tokio
requises sont `fs`, `macros`, `rt`, `sync`, `time`, `io-util`
(`var/serenity/Cargo.toml:37`). Aucune compatibilité async-std ou smol.

La bibliothèque utilise `async-trait` (0.1.74) pour les traits objets asynchrones,
en particulier `EventHandler` et `Framework`. Toutes les méthodes de rappel du
gateway sont des futures `async`.

Les shards sont gérés par un pool de tâches tokio : chaque `ShardRunner` tourne
dans sa propre tâche (`spawn_named`), et `ShardManager` orchestre l'ensemble via
des canaux `mpsc` non bornés (`futures::channel::mpsc`).

---

## 3. Arbre des modules

```
serenity (lib.rs)
├── constants              -- opcodes, limites, user-agent, close codes
│   var/serenity/src/constants.rs:1
├── json                   -- wrappers serde_json / simd-json
│   var/serenity/src/json.rs:1
├── model                  -- types du domaine Discord
│   var/serenity/src/model/mod.rs:1
│   ├── application/       -- Command, Interaction, Component
│   ├── channel/           -- Message, GuildChannel, Embed, Attachment
│   ├── colour             -- type Colour / Color
│   ├── connection         -- OAuth2 connection types
│   ├── event              -- tous les Event* structs + enum Event
│   ├── gateway            -- BotGateway, Ready, GatewayIntents, Activity
│   ├── guild/             -- Guild, Member, Role, PartialGuild, Emoji, AutoMod
│   ├── id                 -- snowflakes (GuildId, ChannelId, MessageId, ...)
│   ├── invite             -- RichInvite
│   ├── mention            -- Mentionable trait
│   ├── misc               -- ImageHash, EmojiIdentifier
│   ├── monetization       -- Entitlement, Sku
│   ├── permissions        -- bitflags Permissions (u64)
│   ├── soundboard         -- SoundboardSound
│   ├── sticker            -- Sticker, StickerItem
│   ├── timestamp          -- newtype Timestamp (time crate)
│   ├── user               -- User, CurrentUser, OnlineStatus
│   ├── voice              -- VoiceState
│   └── webhook            -- Webhook
├── prelude                -- re-exports pratiques
│   var/serenity/src/prelude.rs:1
│
│   -- Les modules suivants sont feature-gated
│
├── builder    [feature=builder]     -- builders HTTP (CreateMessage, EditGuild...)
│   var/serenity/src/builder/
├── cache      [feature=cache]       -- Cache en mémoire (DashMap + parking_lot)
│   var/serenity/src/cache/mod.rs:1
├── client     [feature=client]      -- Client, Context, EventHandler
│   var/serenity/src/client/mod.rs:1
│   ├── context.rs
│   ├── dispatch.rs
│   ├── error.rs
│   └── event_handler.rs
├── collector  [feature=collector]   -- collecteurs d'interactions/réactions
│   var/serenity/src/collector.rs:1
├── framework  [feature=framework]   -- trait Framework
│   var/serenity/src/framework/mod.rs:1
│   └── standard/ [feature=standard_framework]  -- StandardFramework (DÉPRÉCIÉ en 0.12)
├── gateway    [feature=gateway]     -- Shard, ShardManager, WsClient
│   var/serenity/src/gateway/mod.rs:1
│   ├── bridge/
│   │   ├── shard_manager.rs
│   │   ├── shard_messenger.rs
│   │   ├── shard_queuer.rs
│   │   ├── shard_runner.rs
│   │   ├── shard_runner_message.rs
│   │   ├── event.rs
│   │   └── voice.rs
│   ├── shard.rs
│   └── ws.rs
├── http       [feature=http]        -- Http, Ratelimiter, Route
│   var/serenity/src/http/mod.rs:1
│   ├── client.rs
│   ├── error.rs
│   ├── multipart.rs
│   ├── ratelimiting.rs
│   ├── request.rs
│   ├── routing.rs
│   └── typing.rs
├── interactions_endpoint [feature=interactions_endpoint]
│   -- validation ed25519-dalek des webhooks d'interactions
│   var/serenity/src/interactions_endpoint.rs:1
└── utils      [feature=utils]       -- validation de token, misc

Workspaces membres séparés :
├── command_attr/          -- proc-macros #[command], #[group], #[check]...
│   var/serenity/command_attr/src/lib.rs:1
└── voice-model/           -- types gateway voix (opcodes, payloads RTP)
    var/serenity/voice-model/src/lib.rs:1
```

---

## 4. Features Cargo

### 4.1. Features de capacité

| Feature | Ce qu'elle active | Dépendances optionnelles activées |
|---------|-------------------|-----------------------------------|
| `builder` | structs de construction des requêtes HTTP | -- |
| `cache` | cache en mémoire | `rustc-hash`, `dashmap`, `parking_lot` |
| `collector` | collecteurs d'events/réactions inline | `gateway`, `model` |
| `client` | Client + Context | `http`, `typemap_rev` |
| `framework` | trait Framework | `client`, `model`, `utils` |
| `gateway` | connexion WebSocket | `flate2` |
| `http` | client REST | `mime_guess`, `percent-encoding` |
| `model` | méthodes helper sur les types | `builder`, `http`, `utils` |
| `standard_framework` | StandardFramework (DÉPRÉCIÉ) | `framework`, `uwl`, `levenshtein`, `command_attr`, `static_assertions`, `parking_lot` |
| `voice` | infrastructure voix client | `client`, `model` |
| `voice_model` | types de payloads voix | `serenity-voice-model` |
| `unstable_discord_api` | API Discord instables | -- |
| `interactions_endpoint` | validation ed25519 d'un endpoint d'interactions | `ed25519-dalek` |
| `collector` | collecteurs inline | `gateway`, `model` |
| `simd_json` | parsing JSON SIMD | `simd-json` |
| `temp_cache` | cache temporaire HTTP | `cache`, `mini-moka` |
| `chrono` | timestamps via chrono | `chrono` |
| `tokio_task_builder` | nommage des tâches tokio | `tokio/tracing` |

Source : `var/serenity/Cargo.toml:72-136`.

### 4.2. Backends TLS

| Feature | Transport | Crates activées |
|---------|-----------|-----------------|
| `rustls_backend` (**défaut**) | rustls + WebPKI roots | `reqwest/rustls-tls`, `tokio-tungstenite/rustls-tls-webpki-roots`, `bytes` |
| `native_tls_backend` | TLS natif (OpenSSL/SChannel) | `reqwest/native-tls`, `tokio-tungstenite/native-tls`, `bytes` |

La feature `default` vaut `["default_no_backend", "rustls_backend"]`.
`default_no_backend` active : `builder`, `cache`, `chrono`, `client`, `framework`, `gateway`, `model`, `http`, `standard_framework`, `utils`.

Pour aphrody (cible Linux + Windows), utiliser `rustls_backend` (pas de dépendance OpenSSL système).

### 4.3. Feature full

```toml
full = ["default", "collector", "unstable_discord_api", "voice", "voice_model", "interactions_endpoint"]
```

C'est la feature utilisée pour la documentation `docs.rs` (`var/serenity/Cargo.toml:157`).

---

## 5. Flux d'exécution : client -> gateway -> http

```
tokio::main
  └─ Client::builder(token, intents).event_handler(H).await  [client/mod.rs]
       ├─ Http::new(token)               [http/client.rs]
       │    └─ Ratelimiter::new(...)     [http/ratelimiting.rs]
       └─ ClientBuilder::into_future()
            ├─ Http.get_gateway_bot()    [REST GET /gateway/bot]
            └─ ShardManager::new(...)    [gateway/bridge/shard_manager.rs]
                 └─ ShardQueuer (tâche tokio)
                      └─ pour chaque shard :
                           ShardRunner::new() (tâche tokio)
                             └─ Shard::new(ws_url, token, info, intents)  [gateway/shard.rs]
                                  └─ WsClient::connect(url)  [gateway/ws.rs]
                                       └─ tokio-tungstenite connect_async_with_config

  Boucle de réception (ShardRunner) :
    loop {
      shard.do_heartbeat()
      event = ws_client.recv_json()          -- timeout 500ms
      action = shard.handle_event(event)     -- machine d'état
      match action {
        Identify  => ws.send_identify(...)
        Heartbeat => ws.send_heartbeat(...)
        Reconnect(Resume)      => ws.send_resume(...)
        Reconnect(Reidentify)  => reconnecter + send_identify
      }
      dispatch::dispatch(event, cache, http, event_handlers)
    }
```

La méthode `dispatch` (`client/dispatch.rs`) met à jour le cache si actif, puis
appelle chaque `EventHandler` et `RawEventHandler` enregistré dans des tâches
tokio dédiées.

---

## 6. Dépendances clés

| Crate | Rôle |
|-------|------|
| `tokio 1.34` | runtime async, timers, channels |
| `tokio-tungstenite 0.21` | WebSocket sur tokio |
| `reqwest >=0.11.22` | client HTTP REST |
| `serde / serde_json 1` | sérialisation JSON |
| `bitflags 2.4` | `GatewayIntents`, `Permissions`, `ActivityFlags` |
| `secrecy 0.8` | stockage sécurisé du token (`SecretString`) |
| `flate2` | décompression zlib des frames WebSocket (gateway) |
| `dashmap 5.5` | cache concurrent sans lock global |
| `parking_lot 0.12` | RwLock performant pour le cache |
| `async-trait 0.1` | traits async pour EventHandler, Framework |
| `tracing 0.1` | logs structurés |
| `time 0.3` | type `Timestamp` Discord |
| `arrayvec 0.7` | petits vecteurs sur la pile |
| `ed25519-dalek 2` | validation signature interactions endpoint |

---

## 7. Pertinence pour aphrody-hermes

Le crate `aphrody-hermes` (agent Discord voice-to-voice) doit activer au minimum :
- `gateway` + `client` + `http` + `model` + `voice` + `voice_model` + `rustls_backend`
- `MESSAGE_CONTENT` intent (privilégié) pour lire le contenu des messages
- `GUILD_VOICE_STATES` intent pour détecter les changements d'état vocal

Le `ShardManager` gère automatiquement la reconnexion et le heartbeat -- aucun code
supplémentaire n'est requis côté aphrody-hermes pour la résilience de la connexion.
