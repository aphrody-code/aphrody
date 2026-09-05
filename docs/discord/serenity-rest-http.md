# serenity 0.12.5 -- Client HTTP REST

Documentation technique du module `http`. Sources :
- `var/serenity/src/http/client.rs`
- `var/serenity/src/http/ratelimiting.rs`
- `var/serenity/src/http/routing.rs`
- `var/serenity/src/http/mod.rs`

---

## 1. Version de l'API REST Discord

La constante `GATEWAY_VERSION = 10` (`var/serenity/src/constants.rs:13`) vaut
également pour l'API REST. L'URL de base est construite via la macro `api!` définie
dans `var/serenity/src/http/routing.rs` et se présente sous la forme :

```
https://discord.com/api/v10/<chemin>
```

Le `User-Agent` envoyé avec chaque requête est :
```
DiscordBot (https://github.com/serenity-rs/serenity, 0.12.5)
```
(`var/serenity/src/constants.rs:27-31`)

---

## 2. Authentification

### 2.1. Bot token

Le token est passé sous forme de header HTTP `Authorization`. serenity préfixe
automatiquement `"Bot "` si le token ne commence pas déjà par `"Bot "` ou
`"Bearer "` (`var/serenity/src/http/client.rs:169-177`) :

```rust
// Interne à parse_token()
if token.starts_with("Bot ") || token.starts_with("Bearer ") {
    token.to_string()
} else {
    format!("Bot {token}")
}
```

Le token est stocké dans un `SecretString` (crate `secrecy`) pour éviter toute
fuite accidentelle dans les logs.

### 2.2. Bearer token (OAuth2)

Les requêtes OAuth2 utilisateur peuvent être faites avec un token `Bearer` en
passant directement `"Bearer <token>"` au constructeur `Http::new()`. Serenity ne
gère pas lui-même le flux OAuth2, uniquement l'utilisation des tokens résultants.

---

## 3. Structure Http

Définie dans `var/serenity/src/http/client.rs:197-204` :

```rust
pub struct Http {
    pub(crate) client: Client,       // reqwest::Client réutilisé
    pub ratelimiter: Option<Ratelimiter>,
    pub proxy: Option<String>,
    token: SecretString,
    application_id: AtomicU64,       // défini après le READY
    pub default_allowed_mentions: Option<CreateAllowedMentions>,
}
```

L'`application_id` est un `AtomicU64` initialisé à 0 au démarrage. Il est renseigné
automatiquement lors de la réception de l'événement READY, via un callback enregistré
dans le shard (`var/serenity/src/gateway/shard.rs:160-168`).

Construction via `HttpBuilder` (builder pattern) :

```rust
let http = HttpBuilder::new("Bot TOKEN")
    .proxy("http://127.0.0.1:3000")  // optionnel : proxy HTTP
    .ratelimiter_disabled(true)       // optionnel : déléguer au proxy
    .build();
```

---

## 4. Rate-limiting

### 4.1. Principe

Discord impose des limites de débit par route (bucket). Les paramètres clés sont
transmis via des headers de réponse HTTP :

| Header | Signification |
|--------|---------------|
| `x-ratelimit-limit` | Nombre total de requêtes autorisées dans la fenêtre |
| `x-ratelimit-remaining` | Requêtes restantes dans la fenêtre courante |
| `x-ratelimit-reset` | Timestamp UNIX absolu de fin de fenêtre (secondes décimales) |
| `x-ratelimit-reset-after` | Durée jusqu'à la fin de la fenêtre (secondes décimales) |
| `x-ratelimit-global` | Présent si la limite globale est atteinte |
| `retry-after` | Secondes d'attente en cas de 429 |

Source : `var/serenity/src/http/ratelimiting.rs:337-383`.

### 4.2. Paramètres majeurs (bucketing)

Les routes sont groupées par combinaison `(discriminant de route, paramètre majeur)`.
Les paramètres majeurs sont `channel_id`, `guild_id` et `webhook_id`
(`var/serenity/src/http/ratelimiting.rs:6-15`).

Ainsi :
- `GET /channels/4/messages/7` et `GET /channels/5/messages/8` ont des buckets
  distincts car le `channel_id` diffère.
- `GET /channels/10/messages/11` et `GET /channels/10/messages/12` partagent le
  même bucket.

### 4.3. Rate-limiting pré-emptif

Avant d'exécuter une requête, `Ratelimiter::perform` vérifie le bucket
correspondant. Si `remaining == 0`, il dort jusqu'à la prochaine réinitialisation
(`var/serenity/src/http/ratelimiting.rs:304-321`). Cela évite les 429 en prévenant
les dépassements.

En cas de 429 (limite globale), `x-ratelimit-global` est présent et serenity dort
pendant `retry-after` secondes.

### 4.4. Désactivation / proxy

Pour les bots très actifs avec un proxy de rate-limiting (ex. `twilight-http-proxy`),
il est possible de désactiver le rate-limiter interne :

```rust
HttpBuilder::new(token)
    .proxy("http://mon-proxy:3000")
    .ratelimiter_disabled(true)
    .build()
```

---

## 5. Routes définies

L'enum `Route` (`var/serenity/src/http/routing.rs:88-498`) recense toutes les
routes REST wrappées. Voici les catégories principales :

### Channels et messages

| Route | Chemin |
|-------|--------|
| `Channel { channel_id }` | `/channels/{channel_id}` |
| `ChannelMessages { channel_id }` | `/channels/{channel_id}/messages` |
| `ChannelMessage { channel_id, message_id }` | `/channels/{channel_id}/messages/{message_id}` |
| `ChannelMessageCrosspost` | `/channels/{channel_id}/messages/{message_id}/crosspost` |
| `ChannelMessagesBulkDelete` | `/channels/{channel_id}/messages/bulk-delete` |
| `ChannelMessageReaction { ..., reaction }` | `/channels/{channel_id}/messages/{message_id}/reactions/{reaction}/{user_id}` |
| `ChannelMessageReactionMe` | `.../reactions/{reaction}/@me` |
| `ChannelPins { channel_id }` | `/channels/{channel_id}/pins` |
| `ChannelPin { channel_id, message_id }` | `/channels/{channel_id}/pins/{message_id}` |
| `ChannelTyping` | `/channels/{channel_id}/typing` |
| `ChannelWebhooks` | `/channels/{channel_id}/webhooks` |
| `ChannelThreads` | `/channels/{channel_id}/threads` |
| `ChannelForumPosts` | `/channels/{channel_id}/threads` |
| `ChannelPollGetAnswerVoters` | `/channels/{channel_id}/polls/{message_id}/answers/{answer_id}` |
| `ChannelVoiceStatus` | `/channels/{channel_id}/voice-status` |

### Guilds

| Route | Chemin |
|-------|--------|
| `Guild { guild_id }` | `/guilds/{guild_id}` |
| `GuildChannels { guild_id }` | `/guilds/{guild_id}/channels` |
| `GuildMembers { guild_id }` | `/guilds/{guild_id}/members` |
| `GuildMember { guild_id, user_id }` | `/guilds/{guild_id}/members/{user_id}` |
| `GuildMemberRole { guild_id, user_id, role_id }` | `/guilds/{guild_id}/members/{user_id}/roles/{role_id}` |
| `GuildRoles { guild_id }` | `/guilds/{guild_id}/roles` |
| `GuildBan { guild_id, user_id }` | `/guilds/{guild_id}/bans/{user_id}` |
| `GuildBulkBan { guild_id }` | `/guilds/{guild_id}/bulk-ban` |
| `GuildAuditLogs { guild_id }` | `/guilds/{guild_id}/audit-logs` |
| `GuildAutomodRule { guild_id, rule_id }` | `/guilds/{guild_id}/auto-moderation/rules/{rule_id}` |
| `GuildScheduledEvent { guild_id, event_id }` | `/guilds/{guild_id}/scheduled-events/{event_id}` |
| `GuildVoiceStates { guild_id, user_id }` | `/guilds/{guild_id}/voice-states/{user_id}` |
| `GuildThreadsActive { guild_id }` | `/guilds/{guild_id}/threads/active` |
| `GuildWebhooks { guild_id }` | `/guilds/{guild_id}/webhooks` |

### Interactions et commandes slash

| Route | Chemin |
|-------|--------|
| `InteractionResponse { interaction_id, token }` | `/interactions/{interaction_id}/{token}/callback` |
| `WebhookOriginalInteractionResponse { application_id, token }` | `/webhooks/{application_id}/{token}/messages/@original` |
| `WebhookFollowupMessage { application_id, token, message_id }` | `/webhooks/{application_id}/{token}/messages/{message_id}` |
| `Command { application_id, command_id }` | `/applications/{application_id}/commands/{command_id}` |
| `Commands { application_id }` | `/applications/{application_id}/commands` |
| `GuildCommand { application_id, guild_id, command_id }` | `/applications/{application_id}/guilds/{guild_id}/commands/{command_id}` |
| `GuildCommands { application_id, guild_id }` | `/applications/{application_id}/guilds/{guild_id}/commands` |

### Webhooks

| Route | Chemin |
|-------|--------|
| `Webhook { webhook_id }` | `/webhooks/{webhook_id}` |
| `WebhookWithToken { webhook_id, token }` | `/webhooks/{webhook_id}/{token}` |
| `WebhookMessage { webhook_id, token, message_id }` | `/webhooks/{webhook_id}/{token}/messages/{message_id}` |

### Gateway et infra

| Route | Chemin |
|-------|--------|
| `Gateway` | `/gateway` |
| `GatewayBot` | `/gateway/bot` |
| `UserMe` | `/users/@me` |
| `UserMeGuilds` | `/users/@me/guilds` |
| `VoiceRegions` | `/voice/regions` |

---

## 6. Exemples d'appels réels

### Envoyer un message

Via la méthode de haut niveau sur `ChannelId` (feature `model`) :

```rust
channel_id.say(&ctx.http, "Pong!").await?;
```

Ou via `Http` directement :

```rust
http.send_message(channel_id, vec![], &json!({
    "content": "Pong!"
})).await?;
```

### Créer une commande slash globale

```rust
Command::create_global_command(&ctx.http, CreateCommand::new("ping")
    .description("Répond Pong!"))
    .await?;
```

### Répondre à une interaction

```rust
let data = CreateInteractionResponseMessage::new().content("Pong!");
let builder = CreateInteractionResponse::Message(data);
command.create_response(&ctx.http, builder).await?;
```

### Bannir un utilisateur

```rust
http.ban_user(guild_id, user_id, 0, Some("raison")).await?;
```

---

## 7. Trait CacheHttp

Le trait `CacheHttp` (`var/serenity/src/http/mod.rs:61-69`) abstrait l'accès
combiné au cache et au client HTTP :

```rust
pub trait CacheHttp: Send + Sync {
    fn http(&self) -> &Http;
    fn cache(&self) -> Option<&Arc<Cache>> { None }
}
```

Implémenté par `Context`, `Http`, `Arc<Http>`, et `(&Arc<Cache>, &Http)`.
Les méthodes de modèles (ex. `Message::delete`, `Guild::edit`) acceptent
`impl CacheHttp` pour exploiter le cache si disponible, sinon effectuer la
requête HTTP directement.

---

## 8. Gestion des requêtes multipart

Les uploads de fichiers (`CreateAttachment`) utilisent `reqwest::multipart`.
Le module `var/serenity/src/http/multipart.rs` construit les formulaires multipart
pour les endpoints qui acceptent les pièces jointes (envoi de message avec fichier,
création de sticker...).

---

## 9. Indicateur de frappe (Typing)

`http.broadcast_typing(channel_id)` envoie `POST /channels/{id}/typing`. La
classe `Typing` (`var/serenity/src/http/typing.rs`) encapsule un renouvellement
automatique toutes les 10 secondes :

```rust
let typing = channel_id.start_typing(&ctx.http);
// ... traitement long ...
typing.stop();
```

---

## 10. Relation avec aphrody-hermes

Pour le bot vocal, les routes les plus pertinentes sont :
- `GuildVoiceStates` : modifier l'état vocal du bot (mute/deafen self)
- `GuildVoiceStateMe` : même chose via `@me`
- `InteractionResponse` + `WebhookFollowupMessage` : répondre aux commandes slash
- `ChannelMessages` : lecture de l'historique pour le contexte conversationnel
- `VoiceRegions` : connaître les régions disponibles pour choisir le serveur vocal

Le `Ratelimiter` est partagé via `Arc` et thread-safe : un seul `Http` peut être
cloné et partagé entre plusieurs tâches tokio sans risque de double rate-limiting.
