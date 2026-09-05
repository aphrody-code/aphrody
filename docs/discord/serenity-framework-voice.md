# serenity 0.12.5 -- Framework de commandes et voix

Documentation du `StandardFramework` (déprécié), de la proc-macro `command_attr`,
et du modèle de protocole voix (`voice-model`). Sources :
- `var/serenity/src/framework/standard/mod.rs`
- `var/serenity/command_attr/src/lib.rs`
- `var/serenity/voice-model/src/`

---

## 1. StandardFramework -- statut de dépréciation

**Important** : le `StandardFramework` est marqué déprécié dans serenity 0.12
avec la note suivante (`var/serenity/src/framework/standard/mod.rs:3-4`) :

> "The standard framework is deprecated, and will be removed in 0.13. Please
> migrate to `poise` for command handling."

Pour les nouveaux projets aphrody, utiliser directement le trait `EventHandler`
avec `interaction_create` pour les commandes slash, ou la bibliothèque `poise`
qui est le successeur recommandé.

La documentation ci-dessous vaut pour les bots existants ou les cas où le
StandardFramework est utilisé à titre de référence.

---

## 2. StandardFramework

### 2.1. Architecture générale

Le `StandardFramework` (`var/serenity/src/framework/standard/mod.rs:100`) est un
gestionnaire de commandes préfixées (messages texte, style `!commande`). Il se
configure avec un objet `Configuration` et peut être passé au `ClientBuilder`.

Le trait `Framework` (`var/serenity/src/framework/mod.rs`) n'a qu'une méthode :

```rust
#[async_trait]
pub trait Framework: Send + Sync {
    async fn dispatch(&self, ctx: Context, msg: Message, fut: BoxFuture<'_, ()>);
}
```

### 2.2. Hooks disponibles

Le framework expose plusieurs points d'extension sous forme de function pointers :

| Type d'alias | Signature | Déclencheur |
|-------------|-----------|-------------|
| `BeforeHook` | `fn(&Context, &Message, &str) -> BoxFuture<bool>` | Avant chaque commande |
| `AfterHook` | `fn(&Context, &Message, &str, Result<(), CommandError>) -> BoxFuture<()>` | Après chaque commande |
| `DispatchHook` | `fn(&Context, &Message, DispatchError, &str) -> BoxFuture<()>` | En cas d'erreur de dispatch |
| `UnrecognisedHook` | `fn(&Context, &Message, &str) -> BoxFuture<()>` | Commande inconnue |
| `NormalMessageHook` | `fn(&Context, &Message) -> BoxFuture<()>` | Message sans commande |
| `PrefixOnlyHook` | `fn(&Context, &Message) -> BoxFuture<()>` | Message = préfixe seul |

### 2.3. Raisons d'erreur de dispatch

`DispatchError` (`var/serenity/src/framework/standard/mod.rs:49-78`) :

- `CheckFailed(nom, Reason)` : un check personnalisé a échoué
- `Ratelimited(RateLimitInfo)` : bucket rate-limit interne au framework dépassé
- `CommandDisabled` : commande désactivée dans la config
- `BlockedUser` / `BlockedGuild` / `BlockedChannel` : entité bloquée
- `OnlyForDM` / `OnlyForGuilds` : mauvais contexte
- `OnlyForOwners` : réservé aux propriétaires du bot
- `LackingRole` / `LackingPermissions(Permissions)` : permissions insuffisantes
- `NotEnoughArguments { min, given }` / `TooManyArguments { max, given }` : arité

---

## 3. Proc-macro command_attr

Crate séparé `command_attr` (version 0.5.4). Expose les macros attributs pour le
StandardFramework (`var/serenity/command_attr/src/lib.rs:9`) :

```
#[command]   -- définit une commande
#[group]     -- regroupe des commandes
#[check]     -- définit un prérequis
#[help]      -- génère le message d'aide
#[hook]      -- marque un hook (before/after/dispatch_error...)
```

### 3.1. Options de l'attribut #[command]

(`var/serenity/command_attr/src/lib.rs:56-95`)

| Attribut | Signification |
|----------|---------------|
| `#[checks(fn1, fn2)]` | Prérequis à valider avant exécution |
| `#[aliases(nom1, nom2)]` | Noms alternatifs |
| `#[description = "..."]` | Description (aussi via `///`) |
| `#[usage = "..."]` | Usage affiché dans l'aide |
| `#[example = "..."]` | Exemple d'utilisation |
| `#[min_args(n)]` / `#[max_args(n)]` | Contraintes d'arité |
| `#[num_args(n)]` | min = max = n |
| `#[required_permissions(PERM1, PERM2)]` | Permissions Discord nécessaires |
| `#[allowed_roles(role1, role2)]` | Rôles autorisés |
| `#[only_in(guild)]` / `#[only_in(dm)]` | Contexte d'exécution |
| `#[bucket = "nom"]` | Bucket de rate-limiting |
| `#[owners_only]` | Réservé aux owners du bot |
| `#[sub_commands(sub1, sub2)]` | Sous-commandes |

### 3.2. Génération de code

La macro `#[command]` génère deux statics :

```rust
pub static NOMCOMMANDE_COMMAND_OPTIONS: CommandOptions = CommandOptions { ... };
pub static NOMCOMMANDE_COMMAND: Command = Command { options: &NOMCOMMANDE_COMMAND_OPTIONS, ... };
```

Le nom est en majuscules. Ces statics sont passés au `StandardFramework` lors
de la configuration.

---

## 4. Builders

Le module `builder` (`var/serenity/src/builder/`, feature `builder`) fournit des
types de construction fluide (builder pattern) pour toutes les requêtes HTTP.
Ils implémentent le trait `Builder<Context = ..., Built = ...>` :

Builders notables pour aphrody-hermes :

| Builder | Route correspondante |
|---------|----------------------|
| `CreateMessage` | POST `/channels/{id}/messages` |
| `EditMessage` | PATCH `/channels/{id}/messages/{id}` |
| `CreateInteractionResponse` | POST `/interactions/{id}/{token}/callback` |
| `CreateInteractionResponseMessage` | Corps de réponse à une interaction |
| `CreateCommand` | POST `/applications/{id}/commands` |
| `CreateCommandOption` | Paramètre d'une commande slash |
| `CreateEmbed` | Contenu embed dans un message |
| `CreateAttachment` | Upload de fichier |
| `CreateWebhook` | POST `/channels/{id}/webhooks` |
| `EditWebhookMessage` | PATCH d'un message webhook |

---

## 5. Voix -- architecture du protocole

### 5.1. Deux gateways distincts

La voix Discord utilise **deux connexions WebSocket séparées** :

1. **Gateway principal** (op=4 VoiceStateUpdate) : le bot demande à rejoindre un
   canal vocal dans un guild. Discord répond avec deux événements gateway :
   - `VOICE_STATE_UPDATE` : contient le `session_id` de la session voix
   - `VOICE_SERVER_UPDATE` : contient le `token` et l'`endpoint` du serveur voix

2. **Gateway voix** : connexion WebSocket séparée vers `wss://<endpoint>?v=8`,
   gérée indépendamment.

serenity lui-même ne gère pas le flux voix au-delà des structures de données.
Le crate officiel de voix est `songbird`, qui utilise `serenity-voice-model` pour
les types de protocole.

### 5.2. Opcodes du protocole voix

Définis dans `var/serenity/voice-model/src/opcode.rs:11-36` :

| Opcode | Valeur | Direction | Description |
|--------|--------|-----------|-------------|
| `Identify` | 0 | Client -> Serveur | Authentification initiale |
| `SelectProtocol` | 1 | Client -> Serveur | Sélection du protocole UDP + chiffrement |
| `Ready` | 2 | Serveur -> Client | IP, port, SSRC RTP, modes de chiffrement offerts |
| `Heartbeat` | 3 | Bidirectionnel | Nonce u64 aléatoire |
| `SessionDescription` | 4 | Serveur -> Client | Mode de chiffrement confirmé + clé secrète |
| `Speaking` | 5 | Bidirectionnel | Indicateur de parole + SSRC |
| `HeartbeatAck` | 6 | Serveur -> Client | Accusé réception nonce |
| `Resume` | 7 | Client -> Serveur | Reprise de session voix |
| `Hello` | 8 | Serveur -> Client | Intervalle heartbeat |
| `Resumed` | 9 | Serveur -> Client | Session reprise avec succès |
| `ClientConnect` | 12 | Serveur -> Client | Un utilisateur rejoint le canal |
| `ClientDisconnect` | 13 | Serveur -> Client | Un utilisateur quitte le canal |

### 5.3. Séquence de connexion voix

```
1. Via gateway principal :
   Client envoie op=4 VoiceStateUpdate { guild_id, channel_id, self_mute, self_deaf }
   
2. Discord renvoie deux événements gateway :
   t="VOICE_STATE_UPDATE" -> session_id
   t="VOICE_SERVER_UPDATE" -> token + endpoint

3. Client se connecte au gateway voix :
   WebSocket vers wss://<endpoint>?v=8
   
4. Gateway voix envoie op=8 Hello { heartbeat_interval }
   
5. Client envoie op=0 Identify {
       server_id: guild_id,
       user_id,
       session_id,  -- de VOICE_STATE_UPDATE
       token,       -- de VOICE_SERVER_UPDATE
   }
   
6. Serveur envoie op=2 Ready {
       ip,        -- IP du serveur RTP
       port,      -- port UDP RTP
       ssrc,      -- SSRC assigné au client
       modes,     -- modes de chiffrement disponibles
   }
   
7. IP Discovery via UDP (RFC discordtp) pour trouver l'IP externe du client
   
8. Client envoie op=1 SelectProtocol {
       protocol: "udp",
       data: ProtocolData { address, port, mode }
   }
   
9. Serveur envoie op=4 SessionDescription {
       mode,        -- mode de chiffrement confirmé
       secret_key,  -- clé de 32 octets pour le chiffrement
   }
   
10. Client peut maintenant envoyer des paquets RTP UDP chiffrés
    et signaler son état avec op=5 Speaking
```

### 5.4. Payloads voix clés

Tous définis dans `var/serenity/voice-model/src/payload.rs`.

**Speaking** (`var/serenity/voice-model/src/payload.rs:119-134`) :

```rust
pub struct Speaking {
    pub delay: Option<u32>,      // 0 lors de l'envoi depuis le client
    pub speaking: SpeakingState, // bitflags: MICROPHONE | SOUNDSHARE | PRIORITY
    pub ssrc: u32,               // SSRC du client
    pub user_id: Option<UserId>, // présent uniquement dans les messages du serveur
}
```

Le champ `user_id` dans `Speaking` reçu du serveur permet de mapper SSRC -> UserId,
indispensable pour identifier quel utilisateur parle lors de la réception de paquets
RTP.

**SessionDescription** (`var/serenity/voice-model/src/payload.rs:109-116`) :

```rust
pub struct SessionDescription {
    pub mode: String,         // ex. "xsalsa20_poly1305"
    pub secret_key: Vec<u8>,  // 32 octets (NaCl secretbox)
}
```

### 5.5. Chiffrement RTP

Les modes de chiffrement proposés par Discord incluent `xsalsa20_poly1305`,
`xsalsa20_poly1305_suffix`, et `xsalsa20_poly1305_lite`. Le codec audio attendu
est **Opus** à 48 kHz, stéréo ou mono.

serenity ne fournit pas l'implémentation RTP ni le codec Opus. Cela est délégué
à `songbird` (ou à une implémentation custom dans aphrody-hermes via les crates
`audiopus`, `discortp`, `xsalsa20poly1305`).

### 5.6. Fermeture de la connexion voix

Codes de fermeture définis dans `var/serenity/voice-model/src/close_code.rs`,
miroir des codes normaux mais spécifiques à la gateway voix.

---

## 6. Feature `voice` dans serenity

La feature `voice` (`var/serenity/Cargo.toml:121`) active `client` + `model`.
Elle fournit :
- le type `VoiceGatewayManager` (trait objet pour intégrer songbird au Client)
- `VoiceStateUpdate` dans les events gateway
- `gateway::VoiceGatewayManager` passé via `ClientBuilder::voice_manager`

La feature `voice_model` active uniquement le crate `serenity-voice-model` pour
les types de protocole, sans dépendance complète sur le module client.

---

## 7. Collecteurs (feature collector)

Les collecteurs permettent d'attendre des événements inline dans le code sans
configurer un EventHandler global. Utiles pour les workflows interactifs :

```rust
// Attendre une réaction sur un message
let reaction = message
    .await_reaction(&ctx.shard)
    .author_id(user_id)
    .emoji("\u{2705}") // U+2705 (coche) — echappement Unicode, pas d'emoji litteral
    .timeout(Duration::from_secs(30))
    .await;
```

Types disponibles : `MessageCollector`, `ReactionCollector`,
`ComponentInteractionCollector`, `ModalInteractionCollector`.

---

## 8. Récapitulatif pour aphrody-hermes

| Besoin | Solution serenity/songbird |
|--------|---------------------------|
| Commandes slash | `EventHandler::interaction_create` + `CreateCommand` builder |
| Voix en entrée | `songbird` + opcodes `voice-model` + Opus decode |
| Voix en sortie | `songbird` + Opus encode + envoi RTP UDP chiffré |
| Détection de parole | Op=5 Speaking + mapping SSRC->UserId via `ClientConnect` |
| Mute/deafen bot | `VoiceStateUpdate` (op=4) via gateway principal |
| Multimodal Discord+X | `aphrody-hermes` orchestre les deux canaux en parallèle, serenity gère Discord |
