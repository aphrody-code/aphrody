# serenity 0.12.5 -- Types du domaine Discord

Documentation des types principaux du module `model`. Sources :
- `var/serenity/src/model/` (tous sous-modules)
- `var/serenity/src/model/id.rs`
- `var/serenity/src/model/permissions.rs`
- `var/serenity/src/model/guild/mod.rs`
- `var/serenity/src/model/channel/message.rs`
- `var/serenity/src/model/channel/guild_channel.rs`
- `var/serenity/src/model/application/interaction.rs`
- `var/serenity/src/model/application/command.rs`

---

## 1. Snowflakes et identifiants

### 1.1. Principe Snowflake Discord

Un snowflake Discord est un entier 64 bits non nul encodant :
- bits 63-22 : timestamp en ms depuis l'époque Discord (1er janvier 2015)
- bits 21-17 : ID de datacenter interne
- bits 16-12 : ID de worker interne
- bits 11-0  : numéro de séquence incrémental

### 1.2. Implémentation dans serenity

Tous les identifiants sont des newtypes sur `InnerId(NonZeroU64)` générés par la
macro `id_u64!` (`var/serenity/src/model/id.rs:32-136`). La contrainte
`NonZeroU64` garantit que l'identifiant 0 ne peut pas exister.

Identifiants disponibles (`var/serenity/src/model/id.rs:188-216`) :

| Type | Usage |
|------|-------|
| `GuildId` | Serveur Discord |
| `ChannelId` | Canal (textuel, vocal, DM, thread...) |
| `MessageId` | Message |
| `UserId` | Utilisateur |
| `RoleId` | Rôle dans un guild |
| `ApplicationId` | Application bot |
| `InteractionId` | Interaction (slash command, bouton...) |
| `CommandId` | Commande slash enregistrée |
| `WebhookId` | Webhook |
| `EmojiId` | Emoji personnalisé |
| `AttachmentId` | Pièce jointe |
| `StickerId` | Sticker |
| `ScheduledEventId` | Événement planifié |
| `RuleId` | Règle d'auto-modération |
| `AuditLogEntryId` | Entrée du journal d'audit |
| `ShardId` | Shard (entier u32, pas de sérialisation Discord) |

### 1.3. Sérialisation

Les snowflakes sont désérialisés depuis JSON en acceptant indifféremment un entier
ou une chaîne de caractères représentant l'entier (`SnowflakeVisitor`,
`var/serenity/src/model/id.rs:153-186`). Discord envoie les IDs comme strings JSON
dans la plupart des contextes pour éviter les problèmes de précision JavaScript.

La sérialisation est toujours produite en chaîne de caractères
(`var/serenity/src/model/id.rs:184`).

La méthode `created_at()` sur chaque identifiant extrait le timestamp Discord :
```rust
let ts: Timestamp = message_id.created_at();
```

---

## 2. Guild

Définie dans `var/serenity/src/model/guild/mod.rs:111`.
`#[derive(Clone, Debug, Default, Deserialize, Serialize)]`
`#[non_exhaustive]` -- nouveaux champs peuvent apparaître sans break.

Champs principaux :

| Champ | Type | Description |
|-------|------|-------------|
| `id` | `GuildId` | Identifiant unique (= ID du rôle @everyone) |
| `name` | `String` | Nom du serveur |
| `owner_id` | `UserId` | Propriétaire |
| `roles` | `HashMap<RoleId, Role>` | Désérialisé via helper `#[serde(with = "roles")]` |
| `emojis` | `HashMap<EmojiId, Emoji>` | Via `#[serde(with = "emojis")]` |
| `members` | `HashMap<UserId, Member>` | Présent dans GUILD_CREATE gateway, vide en HTTP |
| `channels` | `HashMap<ChannelId, GuildChannel>` | Pareil |
| `threads` | `Vec<GuildChannel>` | Threads actifs |
| `presences` | `HashMap<UserId, Presence>` | Si `GUILD_PRESENCES` intent |
| `verification_level` | `VerificationLevel` | Niveau de vérification |
| `default_message_notifications` | `DefaultMessageNotificationLevel` | -- |
| `explicit_content_filter` | `ExplicitContentFilter` | -- |
| `afk_metadata` | `Option<AfkMetadata>` | Canal AFK + timeout (`#[serde(flatten)]`) |
| `premium_tier` | `PremiumType` | Niveau de boost (0-3) |
| `premium_subscription_count` | `Option<u64>` | Nombre de boosts |
| `features` | `Vec<GuildFeature>` | Fonctionnalités Discord (COMMUNITY, DISCOVERABLE...) |

---

## 3. GuildChannel

Défini dans `var/serenity/src/model/channel/guild_channel.rs:44`.

Représente tous les types de canaux d'un guild : texte, vocal, catégorie, annonces,
stage, thread, forum. Le type est déterminé par le champ `kind: ChannelType`.

Champs clés :

| Champ | Type | Note |
|-------|------|------|
| `id` | `ChannelId` | -- |
| `guild_id` | `GuildId` | -- |
| `kind` | `ChannelType` | Text, Voice, Category, News, Stage, Thread... |
| `name` | `String` | -- |
| `parent_id` | `Option<ChannelId>` | Catégorie parente ou canal parent pour thread |
| `permission_overwrites` | `Vec<PermissionOverwrite>` | Permissions par rôle/user |
| `bitrate` | `Option<u32>` | Voix uniquement |
| `user_limit` | `Option<u32>` | Limite utilisateurs voix |
| `rate_limit_per_user` | `Option<u64>` | Slowmode en secondes (None pour News) |
| `topic` | `Option<String>` | Sujet du canal textuel |
| `nsfw` | `bool` | -- |
| `owner_id` | `Option<UserId>` | Threads et forums seulement |
| `message_count` | `Option<u32>` | Threads |
| `member_count` | `Option<u32>` | Threads |

---

## 4. Message

Défini dans `var/serenity/src/model/channel/message.rs:36`.

Représente un message dans tout canal (guild, DM, thread).

Champs principaux :

| Champ | Type | Description |
|-------|------|-------------|
| `id` | `MessageId` | -- |
| `channel_id` | `ChannelId` | -- |
| `author` | `User` | Expéditeur |
| `content` | `String` | Texte du message. Vide si `MESSAGE_CONTENT` intent absent |
| `timestamp` | `Timestamp` | Heure de création |
| `edited_timestamp` | `Option<Timestamp>` | Dernière modification |
| `tts` | `bool` | Texte-to-speech |
| `mention_everyone` | `bool` | -- |
| `mentions` | `Vec<User>` | Utilisateurs mentionnés |
| `mention_roles` | `Vec<RoleId>` | Rôles mentionnés |
| `attachments` | `Vec<Attachment>` | Fichiers joints |
| `embeds` | `Vec<Embed>` | Embeds |
| `reactions` | `Vec<MessageReaction>` | Réactions |
| `pinned` | `bool` | -- |
| `webhook_id` | `Option<WebhookId>` | Si envoyé par webhook |
| `kind` | `MessageType` | Regular, Reply, SlashCommand... |
| `components` | `Vec<ActionRow>` | Composants interactifs (boutons, menus) |
| `sticker_items` | `Vec<StickerItem>` | Stickers |
| `guild_id` | `Option<GuildId>` | Présent si reçu via gateway guild |
| `member` | `Option<Box<PartialMember>>` | Partiel si reçu via gateway |
| `referenced_message` | `Option<Box<Message>>` | Message d'origine pour les réponses |
| `interaction` | `Option<Box<MessageInteraction>>` | Si envoyé en réponse à une interaction |
| `flags` | `Option<MessageFlags>` | Ephemeral, Crossposted, etc. |

La limite de longueur est 2000 points de code Unicode
(`var/serenity/src/constants.rs:19`).

---

## 5. User

Défini dans `var/serenity/src/model/user.rs`. Champs clés :

| Champ | Type | Description |
|-------|------|-------------|
| `id` | `UserId` | -- |
| `name` | `String` | Nom d'utilisateur (username unique depuis 2023) |
| `discriminator` | `Option<NonZeroU16>` | Ancien discriminant #XXXX (suppression en cours) |
| `global_name` | `Option<String>` | Nom d'affichage (Display Name) |
| `avatar` | `Option<ImageHash>` | Hash de l'avatar |
| `bot` | `bool` | -- |
| `system` | `bool` | Compte système Discord |
| `public_flags` | `Option<UserPublicFlags>` | Staff, Partner, HypeSquad... |

`CurrentUser` est un type distinct avec des champs additionnels (email, MFA enabled,
verified) utilisé dans le payload READY.

---

## 6. Member

Défini dans `var/serenity/src/model/guild/member.rs`. Représente un utilisateur
au sein d'un guild spécifique.

| Champ | Type | Description |
|-------|------|-------------|
| `user` | `User` | Données utilisateur de base |
| `nick` | `Option<String>` | Surnom dans le guild |
| `roles` | `Vec<RoleId>` | Rôles attribués |
| `joined_at` | `Option<Timestamp>` | Date d'adhésion |
| `premium_since` | `Option<Timestamp>` | Depuis quand booste |
| `pending` | `bool` | En attente de validation des règles |
| `permissions` | `Option<Permissions>` | Calculées, présentes dans interactions |
| `communication_disabled_until` | `Option<Timestamp>` | Timeout actif |

---

## 7. Role

Défini dans `var/serenity/src/model/guild/role.rs`.

| Champ | Type | Description |
|-------|------|-------------|
| `id` | `RoleId` | -- |
| `guild_id` | `GuildId` | -- |
| `name` | `String` | -- |
| `colour` | `Colour` | Couleur RGB |
| `hoist` | `bool` | Affiché séparément |
| `managed` | `bool` | Géré par une intégration (ne pas modifier) |
| `mentionable` | `bool` | -- |
| `permissions` | `Permissions` | Bitflags u64 |
| `position` | `i64` | Position dans la hiérarchie |
| `tags` | `Option<RoleTags>` | Bot/integration/premium subscriber |

---

## 8. Permissions

Définie dans `var/serenity/src/model/permissions.rs:269` comme newtype `u64`
implémentant `bitflags!`.

Extrait des permissions avec leurs bits (`var/serenity/src/model/permissions.rs:271-450`) :

| Constante | Bit | Description |
|-----------|-----|-------------|
| `CREATE_INSTANT_INVITE` | 1 << 0 | Créer des invitations |
| `KICK_MEMBERS` | 1 << 1 | -- |
| `BAN_MEMBERS` | 1 << 2 | -- |
| `ADMINISTRATOR` | 1 << 3 | Bypasse tous les overwrites |
| `MANAGE_CHANNELS` | 1 << 4 | -- |
| `MANAGE_GUILD` | 1 << 5 | -- |
| `ADD_REACTIONS` | 1 << 6 | -- |
| `VIEW_AUDIT_LOG` | 1 << 7 | -- |
| `PRIORITY_SPEAKER` | 1 << 8 | -- |
| `STREAM` | 1 << 9 | Go Live |
| `VIEW_CHANNEL` | 1 << 10 | Lire le canal |
| `SEND_MESSAGES` | 1 << 11 | -- |
| `MANAGE_MESSAGES` | 1 << 13 | Supprimer les messages d'autrui |
| `EMBED_LINKS` | 1 << 14 | -- |
| `ATTACH_FILES` | 1 << 15 | -- |
| `READ_MESSAGE_HISTORY` | 1 << 16 | -- |
| `MENTION_EVERYONE` | 1 << 17 | -- |
| `CONNECT` | 1 << 20 | Rejoindre vocal |
| `SPEAK` | 1 << 21 | -- |
| `MUTE_MEMBERS` | 1 << 22 | Mute serveur |
| `DEAFEN_MEMBERS` | 1 << 23 | -- |
| `MOVE_MEMBERS` | 1 << 24 | -- |
| `USE_VAD` | 1 << 25 | Voice Activity Detection |
| `MANAGE_ROLES` | 1 << 28 | -- |
| `MANAGE_WEBHOOKS` | 1 << 29 | -- |
| `MANAGE_GUILD_EXPRESSIONS` | 1 << 30 | Emojis/stickers/soundboard |
| `USE_APPLICATION_COMMANDS` | 1 << 31 | Slash commands et context menus |
| `MANAGE_THREADS` | 1 << 34 | -- |
| `CREATE_PUBLIC_THREADS` | 1 << 35 | -- |
| `CREATE_PRIVATE_THREADS` | 1 << 36 | -- |
| `MODERATE_MEMBERS` | 1 << 40 | Timeout |
| `USE_SOUNDBOARD` | 1 << 42 | -- |

Sérialisation : `Permissions` est sérialisé/désérialisé comme un entier ou une
chaîne représentant un `u64` (`var/serenity/src/model/permissions.rs:40-46`),
car JavaScript ne peut pas représenter fidèlement les entiers 64 bits.

Calcul de permissions effectives : les permissions d'un membre résultent de la
combinaison des rôles + overwrites de canal. La méthode `Guild::member_permissions`
effectue ce calcul.

---

## 9. Interaction et ApplicationCommand

### 9.1. Enum Interaction

Définie dans `var/serenity/src/model/application/interaction.rs:26` :

```rust
pub enum Interaction {
    Ping(PingInteraction),
    Command(CommandInteraction),        // slash command
    Autocomplete(CommandInteraction),   // suggestion autocomplete
    Component(ComponentInteraction),    // bouton, menu select
    Modal(ModalInteraction),            // modal submit
}
```

Chaque variante expose `id`, `guild_id`, `channel_id`, `user`/`member`, `token`,
`app_permissions`.

### 9.2. Command (ApplicationCommand)

Défini dans `var/serenity/src/model/application/command.rs:29` :

| Champ | Type | Description |
|-------|------|-------------|
| `id` | `CommandId` | -- |
| `kind` | `CommandType` | ChatInput (slash), User (context menu user), Message (context menu msg) |
| `application_id` | `ApplicationId` | -- |
| `guild_id` | `Option<GuildId>` | Guild command vs global |
| `name` | `String` | Nom de la commande |
| `description` | `String` | Description (ChatInput seulement) |
| `options` | `Vec<CommandOption>` | Paramètres de la commande |
| `default_member_permissions` | `Option<Permissions>` | Permissions requises par défaut |
| `dm_permission` | `Option<bool>` | Disponible en DM (déprécié, utiliser `contexts`) |

### 9.3. Cycle de vie d'une interaction slash

```
1. Discord envoie event t="INTERACTION_CREATE" via gateway
2. serenity appelle EventHandler::interaction_create(ctx, interaction)
3. Le bot doit répondre dans les 3 secondes via InteractionResponse
4. Pour des traitements longs : répondre Defer, puis EditOriginalInteractionResponse
5. Des followups supplémentaires via WebhookFollowupMessages (jusqu'à 15 min)
```

---

## 10. Embed

Défini dans `var/serenity/src/model/channel/embed.rs`. Limite totale : 6000
caractères (`var/serenity/src/constants.rs:4`), maximum 10 embeds par message
(`var/serenity/src/constants.rs:7`).

Champs principaux : `title`, `description`, `url`, `timestamp`, `colour`,
`footer` (`EmbedFooter`), `image` (`EmbedImage`), `thumbnail` (`EmbedThumbnail`),
`author` (`EmbedAuthor`), `fields` (`Vec<EmbedField>`).

Construction via builder (feature `builder`) :

```rust
CreateEmbed::new()
    .title("Titre")
    .description("Corps")
    .colour(Colour::BLUE)
    .field("Champ", "Valeur", true)  // inline
```

---

## 11. Sérialisation serde -- conventions

Tous les types publics dérivent `Serialize` et `Deserialize`.
Conventions observées dans la base de code :

- `#[non_exhaustive]` sur presque tous les structs et enums -- nouveaux champs
  Discord possibles sans breaking change.
- Les IDs sont acceptés comme string ou entier (voir section 1.3).
- `#[serde(rename = "type")]` est utilisé partout où le champ s'appelle `kind`
  en Rust mais `type` dans le JSON Discord.
- `#[serde(default)]` sur les champs optionnels manquants dans certains contextes.
- Les HashMap de guild (roles, members, channels, emojis) ont des helpers de
  désérialisation personnalisés (`#[serde(with = "roles")]` etc.) qui indexent
  par ID directement.
- `#[serde(flatten)]` utilisé pour `AfkMetadata` dans `Guild` et d'autres
  structures composites.
- Les timestamps utilisent le format ISO 8601 / well-known de la crate `time`
  (`var/serenity/Cargo.toml:39`).
- Les couleurs sont des entiers 32 bits non signés (RRGGBB).
