<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: 2026 aphrody contributors -->
<!--
  Analyse du SDK Python google-antigravity en vue d'un port Rust.
  Source primaire : clone local du dépôt à
  C:\src\aphrody\libs\antigravity-sdk-python
  HEAD = f95239fe241cc910c9b73f1eeba66f89dc6e242d (2026-05-20), tag v0.1.0 présent.
  Document de recherche : lecture seule, aucun code modifié hors ce fichier.
-->

# Analyse du SDK `google-antigravity` (Python) pour port Rust

## 0.0 Confirmation par RE du client desktop Antigravity 2.0.1 (2026-05-21)

> Addendum : faits **vérifiés sur le client installé** (RE forensique locale,
> machine + compte du propriétaire, aucun secret commité ni transmis). Complète
> l'analyse du SDK Python ci-dessous, qui ne couvrait pas le wire réel.

- **Ce qu'est Antigravity** : l'IDE agentique de **Codeium / Windsurf reskinné
  pour Google**, pas un client Gemini léger. Le moteur est le
  `language_server.exe` Go de Codeium (~136 Mo, gRPC protobuf `exa.*_pb`,
  symboles `codeium_common_go_proto`), enveloppé d'un shell Electron mince
  (`app.asar` 2.0.1, author = Google) + un workbench fork de VSCode. Le shell
  spawn et supervise le LS, qui détient l'auth et tout le trafic cloud.
- **Modèle d'auth (ce dont le SDK Rust a besoin)** : source de vérité unique =
  **Windows Credential Manager, cible `gemini:antigravity`** (GENERIC, blob
  JSON `{token:{access_token, token_type:"Bearer", refresh_token, expiry},
  auth_method}`). OAuth2 Google standard + refresh offline. **Aucune auth dans
  les cookies / localStorage** ; `state.vscdb` ne miroite que l'état de sign-in.
  → c'est exactement ce qu'implémente `crates/antigravity-sdk::auth`.
- **Flow OAuth live** : authorize `accounts.google.com/o/oauth2/v2/auth`, token
  `oauth2.googleapis.com/token`, client_id primaire
  `1071006060591-…apps.googleusercontent.com`, redirect
  `http://localhost:9109/oauth-callback`, scopes `cloud-platform`,
  `userinfo.email`, `userinfo.profile`, `cclog`, `experimentsandconfigs`.
- **Endpoints API** : Cloud Code `cloudcode-pa.googleapis.com` (prod) /
  `daily-cloudcode-pa.googleapis.com` (défaut LS) — méthodes
  `v1internal:loadCodeAssist`, `:fetchAvailableModels`, `:onboardUser`. Gemini
  `generativelanguage.googleapis.com`. Vertex `aiplatform.googleapis.com`
  (publishers google + anthropic).
- **Transport LS local** : HTTPS auto-signé sur `127.0.0.1`, port assigné par
  l'OS, token CSRF, **cert pinné** `sha256/sTZpQemOWEytaZqa7P/y/dNXbHMdOAzMvzHEhUwHZXw=`.

**Deux chemins d'interop pour le SDK Rust** (le crate `antigravity-sdk`
implémente le #1) :
1. **Cloud direct (recommandé)** : token OAuth2 → `POST
   cloudcode-pa.googleapis.com/v1internal:loadCodeAssist` avec
   `Authorization: Bearer …`. Voir `antigravity_sdk::endpoints::CloudCodeEndpoint`
   + `AntigravityClient::{load_code_assist, fetch_available_models}`.
2. **gRPC LS local** : spawn `language_server.exe --standalone --subclient_type
   hub …`, parser le port sur stdout, appeler
   `exa.language_server_pb.LanguageServerService` (surface agent complète :
   Cascade / MCP / worktrees) — nécessite le binaire propriétaire. Non porté
   (le #1 couvre les besoins sans dépendance binaire).

## 0. Métadonnées et méthode

- **Acquisition** : le SDK était déjà cloné localement par un autre process à
  `C:\src\aphrody\libs\antigravity-sdk-python` (étape 1 de l'ordre d'acquisition).
  Lecture effectuée sur place. Étapes 2 (clone) et 3 (fetch web) non nécessaires.
- **Version** : `google-antigravity` `0.1.1`
  (`pyproject.toml:21-22`), statut `3 - Alpha` (`pyproject.toml:28`).
- **HEAD du clone** : `f95239fe241cc910c9b73f1eeba66f89dc6e242d`, daté `2026-05-20`,
  message « No public description » ; tag `v0.1.0` présent dans `packed-refs`.
- **Licence** : Apache-2.0 (`pyproject.toml:24`) — compatible aphrody, aucune
  contamination GPL.
- **Python requis** : `>=3.10` (`pyproject.toml:25`).
- **Dépôt amont** : `https://github.com/Google-Antigravity/antigravity-sdk-python`
  (`pyproject.toml:51`).

> Chaque fait ci-dessous cite `fichier:ligne` du clone local. Les éléments
> fournis dans la consigne (token OAuth dans Windows Credential Manager, client_id,
> scopes) **ne sont pas trouvés dans ce SDK** — voir §2 (modèle d'auth) pour la
> clarification, c'est un point critique anti-hallucination.

## 1. Surface API publique

### 1.1 Point d'entrée du paquet (`google/antigravity/__init__.py`)

Symboles ré-exportés au niveau racine (`__init__.py:33-45`, `__all__`) :

| Symbole | Type | Source | Rôle |
|---|---|---|---|
| `Agent` | classe | `agent.py:32` | Client principal, API « Layer 1 » |
| `AgentConfig` | classe abstraite (pydantic) | `connections/connection.py:34` | Config de base abstraite |
| `LocalAgentConfig` | classe (pydantic) | `connections/local/local_connection_config.py:34` | Config concrète backend local (défaut) |
| `CapabilitiesConfig` | modèle pydantic | `types.py:312` | Active/désactive les outils builtin |
| `GeminiConfig` | modèle pydantic | `types.py:158` | Clé API + sélection de modèles |
| `GenerationConfig` | modèle pydantic | `types.py:102` | `thinking_level` |
| `ModelConfig` | modèle pydantic | `types.py:136` | Slots `default` / `image_generation` |
| `ModelEntry` | modèle pydantic | `types.py:120` | Nom modèle + clé + génération |
| `ThinkingLevel` | `StrEnum` | `types.py:82` | MINIMAL/LOW/MEDIUM/HIGH |
| `ToolContext` | classe | `tools/tool_context.py:42` | Contexte injecté dans les outils custom |
| `UsageMetadata` | modèle pydantic | `types.py:473` | Comptage de tokens |

### 1.2 Client principal — `Agent` (`agent.py:32-239`)

- **Constructeur** : `Agent(config: AgentConfig)` (`agent.py:35`). Copie profonde de
  la config (`agent.py:41`), injecte `response_schema` dans
  `capabilities.finish_tool_schema_json` si présent (`agent.py:42-45`). Capture
  les hooks/triggers en attente (`agent.py:56-57`).
- **Cycle de vie** : context manager asynchrone.
  - `async def __aenter__` (`agent.py:88-182`) : enregistre les hooks, applique les
    policies, **refuse de démarrer si des outils d'écriture ou des serveurs MCP
    sont actifs sans policy de sécurité ni hook de décision** (`agent.py:121-131`),
    connecte les serveurs MCP (`agent.py:138-145`), crée `ToolRunner` /
    `ConnectionStrategy` / `Conversation` (`agent.py:147-157`), démarre les
    triggers (`agent.py:160-170`), câble `ToolContext` (`agent.py:174-176`).
  - `async def __aexit__` (`agent.py:184-193`) : ferme la pile `AsyncExitStack`.
- **Méthodes / propriétés publiques** :
  - `register_hook(hook)` (`agent.py:60`).
  - `register_trigger(trigger)` (`agent.py:71`) — interdit après démarrage.
  - `async chat(prompt) -> ChatResponse` (`agent.py:195`) — délègue à `Conversation`.
  - `is_started -> bool` (`agent.py:206`).
  - `conversation -> Conversation` (`agent.py:211`).
  - `conversation_id -> str | None` (`agent.py:228`) — pour reprendre une session via
    `conversation_id`.

### 1.3 Session « Layer 2 » — `Conversation` (`conversation/conversation.py:59-371`)

- Construction normale via `Conversation.create(strategy)` (asynccontextmanager,
  `conversation.py:88-103`).
- Méthodes : `send(prompt, **kwargs)` (`:109`), `receive_steps()` (`:139`),
  `receive_chunks()` (`:160`), `chat(prompt)` (`:211`),
  `get_last_structured_output()` (`:200`), `cancel/delete/signal_idle/wait_for_idle/wait_for_wakeup/disconnect`
  (`:338-370`).
- Propriétés : `history` (`:233`), `last_response` (`:242`), `turn_count` (`:250`),
  `compaction_indices` (`:255`), `connection` (`:291`), `is_idle` (`:302`),
  `conversation_id` (`:307`), `total_usage` (`:312`), `last_turn_usage` (`:321`).
- `ChatResponse` (`types.py:764-895`) : flux asynchrone à curseurs indépendants ;
  `chunks` (`:790`), `__aiter__` → str (`:835`), `thoughts` (`:842`),
  `tool_calls` (`:852`), `resolve()` (`:863`), `text()` (`:871`),
  `structured_output()` (`:882`), `usage_metadata` (`:892`).

### 1.4 Couche transport « Layer 3 » — connections (`connections/connection.py`)

ABCs (`connection.py`) :
- `Connection` (`:113`) : `send` (`:131`), `receive_steps` (`:141`), `disconnect`
  (`:155`), `cancel` (`:159`), `delete` (`:163`), `signal_idle` (`:167`),
  `wait_for_idle` (`:171`), `wait_for_wakeup` (`:175`), `send_tool_results`
  (`:186`), `send_trigger_notification` (`:197`), `is_idle` (`:120`),
  `conversation_id` (`:126`).
- `ConnectionStrategy` (`:207`) : `connect()` (`:215`), `__aenter__` / `__aexit__`
  (`:230` / `:235`).
- `AgentConfig` (`:34`) : champs `system_instructions`, `capabilities`, `tools`,
  `policies`, `hooks`, `triggers`, `mcp_servers`, `workspaces`, `conversation_id`,
  `save_dir`, `app_data_dir`, `response_schema`, `skills_paths`
  (`connection.py:44-64`), méthode abstraite `create_strategy` (`:88`).

Implémentation concrète : `LocalConnection` /
`LocalConnectionStrategy` (`connections/local/local_connection.py:426` / `:1447`).
Re-exports publics dans `connections/local/__init__.py`.

### 1.5 Outils — `tools/`

- `ToolRunner` (`tools/tool_runner.py:126`) : `register` (`:152`), `unregister`
  (`:176`), `execute` (`:256`), `process_tool_calls` (`:279`, exécution
  concurrente via `asyncio.gather`), `get_public_callable` (`:200`, masque
  `ToolContext` du schéma), `set_context` (`:144`), `tool_names`/`tools` props.
- `ToolWithSchema` (`tool_runner.py:103`) : wrapper callable + JSON Schema explicite.
- `ToolContext` (`tools/tool_context.py:42`) : `conversation_id` (`:65`), `is_idle`
  (`:70`), `async send(message)` (`:75`), `get_state`/`set_state` (`:86`/`:98`).
- Type `PythonTool = Callable[..., Any]` (`types.py:465`).

### 1.6 Hooks et policies — `hooks/`

- Contextes : `HookContext` / `SessionContext` / `TurnContext` / `OperationContext`
  (`hooks/hooks.py:36-87`).
- Hooks abstraits : `InspectHook` / `DecideHook` / `TransformHook`
  (`hooks.py:97-142`) ; concrets : `OnSessionStartHook`, `OnSessionEndHook`,
  `PreTurnHook`, `PostTurnHook`, `PreToolCallDecideHook`, `PostToolCallHook`,
  `OnToolErrorHook`, `OnInteractionHook`, `OnCompactionHook`
  (`hooks.py:152-240`).
- Décorateurs : `pre_turn`, `pre_tool_call_decide`, `on_interaction`,
  `on_compaction`, `on_session_start`, `on_session_end`, `post_turn`,
  `post_tool_call`, `on_tool_error` (`hooks.py:281-289`).
- `HookRunner` (`hooks/hook_runner.py:39`) : `register_hook` (`:136`) +
  `dispatch_*` (session start/end, pre/post turn, pre/post tool call, on tool
  error, interaction, compaction) (`hook_runner.py:152-298`).
- Policies (`hooks/policy.py`) : `Decision` enum (`:98`), dataclass `Policy`
  (`:106`), builders `allow` (`:132`), `deny` (`:151`), `ask_user` (`:170`),
  `allow_all` (`:197`), `safe_defaults` (`:209`), `deny_all` (`:225`),
  `confirm_run_command` (`:239`), `workspace_only` (`:348`), `enforce` (`:596`).
  Priorité d'évaluation : Specific Deny > Specific Ask > Specific Allow >
  Wildcard Deny > Wildcard Ask > Wildcard Allow (`policy.py:18-24`, `:381-406`).

### 1.7 Triggers — `triggers/`

- `TriggerContext` (`triggers/triggers.py:28`) avec `async send(content)` (`:41`).
- `Trigger = Callable[[TriggerContext], Awaitable[None]]` (`triggers.py:54`).
- Décorateur `trigger` (`triggers.py:57`) : exige une coroutine à 1 paramètre.
- Helpers : `every(interval_seconds, callback)` (`triggers/helpers.py:39`),
  `on_file_change(path, callback)` (`helpers.py:75`, dépend de `watchfiles` en
  import paresseux). `TriggerRunner` dans `triggers/trigger_runner.py`.

### 1.8 Types de données publics (`types.py`, `__all__` `:32-72`)

Erreurs : `AntigravityConnectionError` (`:665`), `AntigravityValidationError`
(`:673`). Étapes : `Step` (`:542`), enums `StepType` (`:502`), `StepSource`
(`:513`), `StepTarget` (`:522`), `StepStatus` (`:531`). Outils builtin :
`BuiltinTools` `StrEnum` (`:213`, 11 membres + helpers `read_only/nondestructive/
all_tools/file_tools/none`). MCP : `McpStdioServer` (`:368`), `McpSseServer`
(`:382`), `McpStreamableHttpServer` (`:396`), union `McpServerConfig` (`:416`).
Instructions système : `SystemInstructionSection` (`:171`),
`CustomSystemInstructions` (`:178`), `TemplatedSystemInstructions` (`:194`),
union `SystemInstructions` (`:210`). Multimodal : `Image`/`Document`/`Audio`/
`Video` (`:1026`-`:1071`), `from_file` (`:1091`), unions `ContentPrimitive` /
`Content` (`:1074-1075`). Streaming : `StreamChunk` (`:744`), `Thought` (`:751`),
`Text` (`:758`). Interaction : `AskQuestionOption` (`:633`), `AskQuestionEntry`
(`:642`), `AskQuestionInteractionSpec` (`:652`), `QuestionResponse` (`:603`),
`QuestionHookResult` (`:619`). Divers : `ToolCall` (`:424`), `ToolResult`
(`:442`), `HookResult` (`:589`), `TriggerDelivery` (`:708`), `FileChangeKind`
(`:717`), `FileChange` (`:725`), `UsageMetadata` (`:473`).

### 1.9 MCP — sous-module `mcp/` (cf. §4)

`McpBridge` (`mcp/bridge.py:62`), `get_mcp_tools` (`mcp/bridge.py:31`).

## 2. Modèle d'authentification — POINT CRITIQUE

**Le SDK n'implémente AUCUN flux OAuth, ni rafraîchissement de token, ni lecture
de credential store.** L'authentification réelle vers Gemini est déléguée au
binaire Go `localharness` ; le SDK ne fait que **lui transmettre une clé API
Gemini**.

Faits (cités) :
- La clé est portée par `GeminiConfig.api_key` (`types.py:167`) ou
  `ModelEntry.api_key` par modèle (`types.py:130`), avec repli sur la variable
  d'environnement `$GEMINI_API_KEY` (`local_connection.py:1640-1648`,
  `types.py:162-163`, `README.md:27`).
- Échec rapide si aucune clé : `AntigravityValidationError` levée dans
  `__aenter__` (`local_connection.py:1643-1648`).
- La clé est encodée dans `GeminiConfig` proto et passée au harness
  (`local_connection.py:1534-1545`).
- **Auth WebSocket interne** : le harness renvoie une `api_key` dans son
  `OutputConfig` (handshake stdin/stdout) ; le SDK l'utilise comme header
  `x-goog-api-key` pour se connecter à `ws://localhost:<port>/`
  (`local_connection.py:1688-1690`). C'est une clé éphémère locale process↔process,
  pas un token cloud.
- Erreurs HTTP fatales remontées : 400 / 401 / 403 → `AntigravityConnectionError`
  (`local_connection.py:586-594`).

**Conséquence anti-hallucination** : les éléments fournis dans la consigne
(token dans Windows Credential Manager `gemini:antigravity`, JSON
`{"token":{access_token, refresh_token, ...}}`, client_id
`1071006060591-...apps.googleusercontent.com`, scopes `cloud-platform
userinfo.email userinfo.profile cclog experimentsandconfigs`) **n'apparaissent
nulle part dans ce SDK** (vérifié : aucune occurrence d'OAuth, refresh, client_id,
credential manager dans l'arbre `google/antigravity/`). Ces artefacts relèvent de
**l'application desktop Antigravity** (qui embarque/alimente le harness), pas du
SDK Python. Pour un port Rust, l'auth OAuth est donc à traiter **séparément**
si l'on veut parler directement au backend cloud sans passer par `localharness` —
ce SDK ne fournit pas ce code.

## 3. Endpoints et transport

Le SDK ne contacte **aucune URL HTTPS distante d'API Antigravity directement**.
Le transport unique livré (`v0.1.1`) est **local**, via le binaire Go
`localharness` :

1. **Découverte du binaire** (`local_connection.py:1392-1444`,
   `_get_default_binary_path`) : `$ANTIGRAVITY_HARNESS_PATH` →
   `importlib.metadata` (wheel `google-antigravity`, chemin
   `google/antigravity/bin/localharness`) → `importlib.resources` →
   `shutil.which("localharness")`.
2. **Spawn process** : `subprocess.Popen([binary], stdin/stdout/stderr=PIPE)`
   (`local_connection.py:1655-1660`).
3. **Handshake longueur-préfixée** sur stdin/stdout : `InputConfig` sérialisé
   protobuf, préfixe `struct.pack("<I", len)` (uint32 little-endian)
   (`local_connection.py:1662-1677`) ; réponse `OutputConfig` (port WS + api_key).
4. **WebSocket** `ws://localhost:<port>/` avec header `x-goog-api-key`, retry
   exponentiel ×5 (`local_connection.py:1678-1701`).
5. **Initialisation** : envoi `InitializeConversationEvent{config: HarnessConfig}`
   (`local_connection.py:1705-1708`).
6. **Protocole de messages** : protobuf **sérialisé en JSON** (et non binaire) via
   `google.protobuf.json_format.MessageToJson` / `Parse` (ex.
   `local_connection.py:535`, `:767`, `:1708`).

REST vs gRPC : **ni l'un ni l'autre** côté SDK — c'est du WebSocket + protobuf-JSON
process-local. (Le `ConnectionStrategy` est conçu pour accueillir d'autres backends
distants, mais aucun n'est livré ; cf. `connections/README.md:64-68`.)

Messages protobuf principaux (du wrapper `localharness_pb2`, usage cité dans
`local_connection.py`) : `InputConfig`, `OutputConfig`, `HarnessConfig`,
`InitializeConversationEvent`, `InputEvent` (`user_input`, `complex_user_input`,
`question_response`, `tool_confirmation`, `tool_response`, `automated_trigger`,
`halt_request`), `OutputEvent` (`step_update`, `trajectory_state_update`,
`tool_call`, `usage_metadata`), `StepUpdate`, `TrajectoryStateUpdate`, `ToolCall`,
`ToolResponse`, `UserInput`(+`Part`/`Media`), `UserQuestionsResponse`,
`ToolConfirmation`, `SystemInstructions`, `GeminiConfig`, `Workspace`,
`HarnessSideTools` + sous-configs par outil.

## 4. MCP — ce qu'expose `mcp/`

`mcp/bridge.py` (`README` : `mcp/README.md`) :
- `McpBridge` (`bridge.py:62`) : gère le cycle de vie des sessions MCP client.
  `connect(server_cfg)` dispatch par type (`bridge.py:74-96`) :
  `connect_stdio(command, args)` (`:98`), `connect_sse(url, headers)` (`:108`),
  `connect_streamable_http(url, headers, timeout, sse_read_timeout,
  terminate_on_close)` (`:120`). `stop()` (`:159`). Propriété `tools` (`:69`).
- `get_mcp_tools(session_group)` (`bridge.py:31`) : convertit les outils d'un
  `ClientSessionGroup` (paquet `mcp` amont) en `ToolWithSchema`.
- S'appuie sur le paquet PyPI `mcp>=1.0` (`pyproject.toml:41`) :
  `mcp.client.stdio`, `mcp.client.session_group`
  (`bridge.py:23-28`).
- Intégration : `Agent.__aenter__` connecte chaque `mcp_servers` et fusionne les
  outils dans le `ToolRunner` (`agent.py:138-145`).

Note : le sous-module `mcp/` du SDK est un **client MCP** (consomme des serveurs
externes), pas un serveur MCP.

## 5. Modèles de données à porter (priorité)

Tous des `pydantic.BaseModel` V2 (sauf enums `StrEnum` et `ChatResponse`).
Mapping de portage Rust recommandé (serde) :

| Modèle Python | Rust suggéré |
|---|---|
| `GeminiConfig`, `ModelConfig`, `ModelEntry`, `GenerationConfig` | structs `serde` |
| `CapabilitiesConfig` | struct + validation mutuelle `enabled/disabled_tools` |
| `BuiltinTools` (StrEnum, 11) | `enum` + `#[serde(rename=...)]` |
| `ThinkingLevel`, `StepType/Source/Target/Status`, `TriggerDelivery`, `FileChangeKind`, `Decision` | `enum` Rust |
| `ToolCall`, `ToolResult` | structs `serde` (`args`/`result` → `serde_json::Value`) |
| `Step`, `UsageMetadata` | structs `serde`, champs `Option<T>` |
| `Mcp{Stdio,Sse,StreamableHttp}Server` | enum tagué `#[serde(tag="type")]` |
| `SystemInstructions` union | `enum` (Custom / Templated) |
| `Image/Document/Audio/Video` + `from_file` | enum `Media` + validation MIME |
| `Policy`, builders, `enforce` | struct + builders, prédicats = closures Rust |
| `HookResult`, `QuestionResponse`, `QuestionHookResult`, `AskQuestion*` | structs `serde` |

## 6. Plan de port Rust

### 6.1 Décision d'architecture

Deux stratégies possibles, à trancher selon l'objectif aphrody :

- **(A) Réimplémenter le client du harness** (parité 1:1 avec ce SDK) : porter la
  couche `LocalConnection` (spawn `localharness`, handshake longueur-préfixée WS +
  protobuf-JSON). Nécessite le binaire Go `localharness` (livré seulement dans les
  wheels PyPI — `README.md:14-19`). Auth = clé API Gemini, **pas d'OAuth**.
- **(B) Parler directement au backend cloud** (hors périmètre de ce SDK) :
  imposerait d'implémenter soi-même OAuth (token Credential Manager / refresh,
  client_id, scopes de la consigne). **Aucun code de référence dans ce SDK** — à
  reverse-engineer ailleurs (app desktop). À documenter comme tâche distincte.

Le port direct du SDK = stratégie (A).

### 6.2 Mapping module Python → module Rust (crate `antigravity` proposé)

| Module Python | Module Rust |
|---|---|
| `google/antigravity/types.py` | `src/types.rs` (modèles serde, enums, médias) |
| `agent.py` | `src/agent.rs` (`Agent`, builder, cycle de vie async) |
| `conversation/conversation.py` | `src/conversation.rs` (`Conversation`, `ChatResponse` → `Stream`) |
| `connections/connection.py` | `src/connection/mod.rs` (traits `Connection`, `ConnectionStrategy`, `AgentConfig`) |
| `connections/local/local_connection.py` | `src/connection/local.rs` (spawn process, WS, codec longueur-préfixée) |
| `connections/local/local_connection_config.py` | `src/connection/local_config.rs` |
| `connections/local/localharness_pb2.py` | `src/proto/` généré par `prost-build` depuis le `.proto` (à récupérer) |
| `connections/local/types.py` | `src/connection/local_types.rs` (résultats d'outils structurés) |
| `tools/tool_runner.py`, `tool_context.py` | `src/tools.rs` (registre + exécution ; injection de contexte via trait) |
| `hooks/hooks.py`, `hook_runner.py`, `policy.py` | `src/hooks/` (`mod.rs`, `runner.rs`, `policy.rs`) |
| `triggers/triggers.py`, `helpers.py`, `trigger_runner.py` | `src/triggers.rs` |
| `mcp/bridge.py` | `src/mcp.rs` (client MCP) |

### 6.3 Dépendances Rust suggérées

- **Async runtime** : `tokio` (process, WS, timers, `asyncio.gather` → `join_all`).
- **WebSocket** : `tokio-tungstenite` (équivalent de `websockets`).
- **Protobuf** : `prost` + `prost-build` ; JSON-mapping protobuf → `prost-types` +
  sérialisation JSON canonique (le SDK envoie le **JSON** protobuf, pas le binaire,
  cf. §3 ; vérifier la compat du mapping JSON proto3 côté harness).
- **Sérialisation** : `serde` + `serde_json` (tous les modèles pydantic).
- **HTTP / client MCP** : `reqwest` (SSE/Streamable HTTP) + crate `rmcp` (MCP Rust
  officiel) pour le bridge MCP.
- **Process** : `tokio::process::Command` (spawn `localharness`, stdin/stdout
  longueur-préfixée `u32` LE via `tokio::io`).
- **Validation** : logique manuelle dans les constructeurs/`TryFrom` (pas d'équiv.
  direct des `model_validator` pydantic).
- **Erreurs** : `thiserror` (`AntigravityConnectionError`,
  `AntigravityValidationError`).
- **File watching** (trigger `on_file_change`) : `notify` (équiv. `watchfiles`).
- **Streaming** : `futures::Stream` / `async-stream` pour `ChatResponse`,
  `receive_steps`, `receive_chunks`.

### 6.4 Pièges de portage identifiés

- **Codec stdin/stdout** : préfixe longueur `u32` little-endian
  (`local_connection.py:1667`, `:1675`) — reproduire exactement.
- **Concurrence du reader loop** : `_ws_reader_loop` route step_update /
  trajectory_state_update / tool_call et gère sous-agents + idle (≈
  `local_connection.py:761-959`) — porter avec une `mpsc` queue + `tokio::select!`.
- **Mutuelle exclusion** `enabled_tools` / `disabled_tools` (`types.py:359-365`).
- **Policy par défaut** de `LocalAgentConfig` : `confirm_run_command()` deny
  `run_command` + `workspace_only` ajouté automatiquement quand `workspaces` non
  vide (`local_connection_config.py:99-116`).
- **Refus de démarrage** sans policy quand outils d'écriture/MCP actifs
  (`agent.py:121-131`) — règle de sécurité à conserver.
- **Binaire `localharness`** non versionné dans le dépôt (livré par wheel) : le
  port Rust doit le localiser/embarquer ou exposer `ANTIGRAVITY_HARNESS_PATH`.

## 7. Récapitulatif

### Symboles publics inventoriés

Décompte des symboles publics exportés / nommés (classes, enums, fonctions
top-level, méthodes/propriétés publiques recensées) :

- `__all__` racine : **11** (`__init__.py:33-45`).
- `__all__` de `types.py` : **39** entrées (`types.py:32-72`) + types médias et
  unions non listés dans `__all__` (`Image`, `Document`, `Audio`, `Video`,
  `from_file`, `ContentPrimitive`, `Content`, `Step*` enums) ≈ **48** symboles types.
- Couche connections : `Connection`, `ConnectionStrategy`, `AgentConfig`,
  `LocalConnection`, `LocalConnectionStrategy`, `LocalAgentConfig`,
  `LocalConnectionStep` + types résultats outils (`local/types.py` : 7) = **14**.
- Conversation / réponse : `Conversation`, `ChatResponse` = **2** (+ ≈ 18
  méthodes/propriétés publiques sur `Conversation`).
- Tools : `ToolRunner`, `ToolWithSchema`, `ToolContext`, `PythonTool` = **4**.
- Hooks : 4 contextes + 3 ABC + 9 hooks concrets + 9 décorateurs + `HookRunner`
  + `HookResult` = **27**.
- Policies : `Decision`, `Policy`, + 9 fonctions builder/`enforce` = **11**.
- Triggers : `TriggerContext`, `Trigger`, `trigger`, `every`, `on_file_change`,
  `TriggerRunner` = **6**.
- MCP : `McpBridge`, `get_mcp_tools` = **2**.

**Total des symboles publics distincts inventoriés : ≈ 125** (top-level classes,
enums, fonctions, unions de types ; les méthodes/propriétés d'instance ne sont pas
toutes recomptées individuellement dans ce total).

### État par section

| Section | État |
|---|---|
| 1. Surface API | FAIT (modules, classes, méthodes du client `Agent` et constructeurs cités) |
| 2. Auth | FAIT (modèle réel = clé API Gemini ; absence d'OAuth dans le SDK documentée et vérifiée — divergence avec la consigne explicitée) |
| 3. Endpoints / transport | FAIT (transport local WS + protobuf-JSON ; ni REST ni gRPC distant ; messages proto listés via usage) |
| 4. MCP | FAIT (`McpBridge` + `get_mcp_tools`, 3 transports) |
| 5. Modèles de données | FAIT (table de portage) |
| 6. Plan de port Rust | FAIT (mapping module-à-module + deps + pièges) |
| Récap. symboles | FAIT |

**Limites / INCOMPLET** :
- Le fichier `localharness_pb2.py` est un descriptor sérialisé (pas de `.proto`
  lisible dans le clone) ; les noms de messages proto sont déduits de leur **usage**
  dans `local_connection.py` (fiable) mais le `.proto` source exact reste à
  récupérer pour `prost-build` (point 6.3).
- Modules de runtime non lus en détail (mais publics listés) : `triggers/
  trigger_runner.py`, `utils/interactive.py`, fichiers `*_test.py` — non
  nécessaires au port de la surface API.
- Auth OAuth cloud (token Credential Manager, client_id, scopes de la consigne) :
  **hors périmètre du SDK**, donc INCOMPLET ici par nature — à reverse-engineer
  depuis l'app desktop Antigravity si la stratégie (B) est retenue.

**Aucune modification hors `docs/research/antigravity-sdk-analysis.md`. Aucun commit.**
