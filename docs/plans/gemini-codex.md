<!-- SPDX-License-Identifier: Apache-2.0 -->
# Plan — « Codex pour Gemini » (agent de code autonome aphrody)

> Statut : **plan de design** (2026-05-27). Issu de la cartographie exhaustive
> du dépôt **OpenAI Codex** (Apache-2.0) cloné dans `var/codex/` (gitignored).
> Objectif : reconstruire l'expérience Codex (CLI + TUI + boucle agentique +
> tools + MCP + skills) **pilotée par Gemini** au lieu d'OpenAI, dans le
> workspace Rust d'aphrody. Codex est Apache-2.0 → portage légal avec en-tête
> SPDX + attribution.

## 0. Vision

Un agent de code **interactif (TUI) et non-interactif (`exec`/JSONL)**, piloté
par Gemini (via `gemini-runtime` REST ou `gemini-web`), avec :
- boucle de turn streaming + tool-calls + ré-injection multi-tour ;
- catalogue d'outils (shell sandboxé, `apply_patch`, file-search, MCP) ;
- skills (`SKILL.md`) injectés dans le prompt ;
- persistence de session (JSONL rollout + reprise) ;
- protocole événementiel NDJSON réutilisable par le CLI **et** l'app Tauri.

aphrody possède déjà les briques moteur : `gemini-runtime`, `gemini-web`,
`aphrody-chat`, `aphrody-tools`, `aphrody-skills`, `aphrody-mcp` (`google_mcp`),
`aphrody-fsindex`, `aphrody-session`, `aphrody-events`, `aphrody-guard`
(hardening + command-safety, livré). Le travail = assembler une **boucle
agentique + protocole + TUI** par-dessus.

## 1. Architecture Codex en 4 couches (source du portage)

### Couche A — CLI + TUI (`var/codex/codex-rs/{cli,arg0,exec,tui}`)
- **`arg0`** (~585 l) : multicall bootstrap (intercept `argv[0]` → `apply_patch`,
  `linux-sandbox`, `execve-wrapper`) + injection PATH d'alias symlinkés.
- **`cli`** (~18 k l) : `MultitoolCli` (clap) ; sous-commandes `exec`, `login`,
  `mcp`, `mcp-server`, `app-server`, `resume`, `fork`, `apply`, `doctor`,
  `sandbox`, `completion`, `cloud`… ; défaut = TUI interactif.
- **`exec`** (~5 k l) : mode non-interactif, sortie `--json` (JSONL) ou human ;
  `EventProcessor` trait + impls human/JSONL ; `InProcessAppServerClient`.
- **`tui`** (~7.7 k l lib, le plus complexe) : **Ratatui + Crossterm** (forks
  `nornagon`, features `scrolling-regions`/`unstable-*`), `syntect` pour la
  coloration. Event loop tokio `select!` sur {crossterm, app-server, app-event,
  frame timer}. État : `App` → `ChatWidget` → `TranscriptState`(cellules
  `HistoryCell`) + `BottomPane`(composer + overlays). Streaming via
  `StreamController`. Keymap configurable (`RuntimeKeymap`, Emacs+Vim).
  **Backend-agnostique** : parle JSON-RPC à l'app-server, ignore le LLM dessous.

### Couche B — Cœur agentique + protocole (`{core,protocol,app-server,thread-store,rollout,state}`)
- **`core`** (~15 k l) : `Session` (état conversation) + `run_turn`
  (`core/src/session/turn.rs:133`) = boucle SQ/EQ streaming :
  `run_sampling_request` → `client.stream(prompt)` → pour chaque `ResponseEvent` :
  `OutputItemDone(FunctionCall)` → `ToolOrchestrator::run` (approval→sandbox→exec)
  → ré-injecte l'output → `needs_follow_up` → reboucle ; sinon `TurnComplete`.
  `CodexThread` expose `submit()` / `next_event()` / `steer_input()`.
- **`protocol`** (~5.4 k l) : `Submission{id,op}` (SQ, client→agent) et
  `Event{id,msg}` (EQ, agent→client). `Op`: `UserInput`, `ExecApproval`,
  `PatchApproval`, `Interrupt`, `Compact`, `Shutdown`… `EventMsg`: `TurnStarted`,
  `AgentMessage`, `AgentMessageContentDelta` (streaming), `ExecCommandBegin/End`,
  `ExecApprovalRequest`, `ApplyPatchApprovalRequest`, `TokenCount`,
  `TurnComplete`… **Wire = NDJSON** (une ligne JSON par event).
- **`app-server`** (~8 k l) : serveur JSON-RPC 2.0-light multiplexé sur
  stdio/UDS/WebSocket ; `ThreadStateManager` (HashMap<ThreadId,ThreadState>),
  une `tokio::task` par thread, 17 `RequestProcessor` (thread/turn/config/mcp…).
- **Persistence** : `thread-store` (trait + SQLite/JSONL), `rollout`
  (actor JSONL `~/.codex/sessions/rollout-*.jsonl`), `state` (sqlx SQLite :
  memories/logs/goals).
- **Approbations** : event `ExecApprovalRequest` → await `oneshot` → `Op::ExecApproval`
  débloque. `Guardian` = reviewer auto sans humain (≈ mode autonome aphrody).

### Couche C — Provider / API réseau (`{model-provider,model-provider-info,codex-api,codex-client,login}`)
- **Trait `ModelProvider`** (`model-provider/src/provider.rs:83`) : `info()`,
  `capabilities()`, `api_auth() -> SharedAuthProvider`, `models_manager()`…
- **Trait `AuthProvider`** (`codex-api/src/auth.rs:30`) : `add_auth_headers()`,
  `apply_auth(Request)`.
- **`ModelProviderInfo`** : `base_url`, `env_key`, `wire_api` (**seul variant =
  `Responses`** — `Chat` retiré), `requires_openai_auth`, `supports_websockets`.
- **Requête** : `build_responses_request` (`core/src/client.rs:717`) →
  `ResponsesApiRequest{model,instructions,input,tools,stream:true,…}`.
- **Streaming SSE** : crate **`eventsource-stream`** ; `process_sse`
  (`codex-api/src/sse/responses.rs:399`) mappe `response.output_text.delta` →
  `OutputTextDelta`, `response.output_item.done` → `OutputItemDone`,
  `response.completed` → `Completed{usage}`.
- **Providers non-OpenAI déjà présents** : `ollama`/`lmstudio` ne sont PAS des
  impls de trait — ce sont des `ModelProviderInfo` data-driven car Ollama≥0.13.4
  expose `/v1/responses` (Responses-compatible). **Gemini n'est PAS
  Responses-compatible** → nécessite une vraie impl (comme `amazon-bedrock`).

### Couche D — Tools / MCP / Skills (`{tools,apply-patch,mcp-server,codex-mcp,rmcp-client,core-skills,hooks}`)
- **`tools`** (~3.5 k l) : `ToolDefinition{name,description,input_schema,…}`,
  trait `ToolExecutor` (`tool_name`/`spec`/`exposure`/`handle`), `ToolSpec`
  (`Function`/`Namespace`/`ToolSearch`/`WebSearch`/`ImageGeneration`/`Freeform`),
  `JsonSchema` avec compaction (budget 4 KB, 3 passes). `ToolExposure` =
  Direct/Deferred/DirectModelOnly/Hidden.
- **`apply-patch`** (~1.6 k l) : format `*** Begin Patch / *** Add|Update|Delete
  File / @@ ctx / +|-| line / *** End Patch`. `seek_sequence` fuzzy Unicode,
  `compute_replacements` (appliqué en ordre décroissant), `StreamingPatchParser`,
  trait `ExecutorFileSystem` (sandboxable). Instructions Markdown injectées au
  system prompt pour les modèles qui ne maîtrisent pas le format.
- **MCP** : Codex est **serveur** (`mcp-server` expose les tools `codex` +
  `codex-reply` avec `threadId`) **et client** (`rmcp-client` consomme des MCP
  externes ; `codex-mcp` normalise les noms : SHA1-dedup 12 c, max 64 c,
  delimiter `__`, allow/deny filter).
- **`core-skills`** : `SKILL.md` frontmatter (`name`/`description`/`metadata`) +
  `agents/openai.yaml` ; discovery DFS profondeur 6 ; invocation implicite ;
  `build_skill_injections` + rendu `## Skills` (budget 8 KB, 3 passes).
- **`hooks`** : 10 events (`PreToolUse`/`PostToolUse`/`Stop`/…), subprocess async
  + matchers ; `HookEventAfterAgent decision:deny` = exactement le pattern
  `agy-loop` d'aphrody.
- **SDK** : `sdk/typescript` + `sdk/python` spawnent le binaire et parsent le
  NDJSON ; `codex-cli/` = wrapper npm pur (spawn binaire platform-specific).
  **Aucune app desktop/IDE dans le repo OSS** — l'unique surface IDE = serveur MCP.

## 2. Réutilisable tel quel vs à réécrire

| Composant Codex | Verdict | Note |
|---|---|---|
| `tui` (rendu/composer/keymap/overlays) | **Réutiliser** (rebrand) | Backend-agnostique : parle JSON-RPC, ignore le LLM |
| `arg0`, `exec`, `exec-server` | **Réutiliser** | Renommer constantes `CODEX_*`→`APHRODY_*` |
| `protocol` (Op/EventMsg/wire NDJSON) | **Réutiliser** (épurer) | Garder l'enveloppe ; retirer types OpenAI-only |
| `app-server-transport`, `app-server-client` | **Réutiliser** | Transport-agnostique |
| `apply-patch`, `seek_sequence` | **Réutiliser** | Parser pur, 0 dep propriétaire → `aphrody-tools` |
| `sandboxing`/`linux-sandbox`/`shell-command` | **Réutiliser** | OS-only (cf. `aphrody-guard` déjà livré) |
| `core` boucle `run_turn` | **Réécrire (minimal)** | Garder le pattern, brancher Gemini |
| `app-server` orchestrateur | **Réécrire** | Cœur du couplage OpenAI |
| `model-provider`/`codex-api` SSE | **Réécrire** | Gemini ≠ Responses API |
| `login`/`keyring-store` | **Remplacer** | Auth Gemini : `GEMINI_API_KEY` / token agy |

## 3. Plan d'intégration Gemini (le vrai travail)

Gemini `streamGenerateContent` ≠ OpenAI Responses. Différences à gérer :

| Aspect | OpenAI Responses | Gemini generateContent |
|---|---|---|
| Endpoint | `POST /v1/responses` | `POST /v1beta/models/{m}:streamGenerateContent` |
| Auth | `Authorization: Bearer` | `?key=` ou header `x-goog-api-key` |
| System | `instructions: String` | `systemInstruction.parts[].text` |
| Messages | `input: [ResponseItem]` roles user/assistant | `contents: [{role:user/model, parts}]` |
| Tool call | `FunctionCall{name,arguments}` | `parts[].functionCall{name,args}` |
| Tool result | `FunctionCallOutput{call_id,output}` | `parts[].functionResponse{name,response}` |
| Tool def | `{type:function,name,parameters}` | `tools[].function_declarations[]` |
| Stream delta | SSE `response.output_text.delta` | SSE `candidates[].content.parts[].text` |
| Stop | `response.completed` + usage | `candidates[].finishReason` + `usageMetadata` |
| Reasoning | `reasoning.effort` | `thinkingConfig.thinkingBudget` (2.5) |

**Deux options d'intégration** :
- **Option A — proxy SSE local** (modèle `responses-api-proxy`) : un serveur
  local traduit Responses↔Gemini → réutilise 100 % du chemin SSE Codex. Rapide
  à brancher, overhead d'un hop local.
- **Option B (recommandée) — client natif** : trait `GeminiModelClient` dans
  `gemini-runtime` exposant `stream_generate(history, tools) -> impl
  Stream<Item=GeminiStreamEvent>` ; la boucle `run_turn` consomme ce stream
  unifié, **sans dépendre de `reqwest` directement**, ce qui permet de basculer
  `gemini-runtime` (REST) ↔ `gemini-web` (cookies) par config.

## 4. Mapping vers les crates aphrody

| Couche Codex | Cible aphrody | Action |
|---|---|---|
| `protocol` (Op/EventMsg) | `crates/aphrody-session` (étendre) | Types SQ/EQ + NDJSON |
| `core/run_turn` | `crates/aphrody-session/src/gemini_turn.rs` | Boucle tool-loop |
| `core/client.rs` | `crates/gemini-runtime` (+trait `GeminiModelClient`) | Streaming unifié |
| `core/codex_thread.rs` | `crates/aphrody-session/src/thread.rs` | Facade submit/next_event |
| `tools/orchestrator.rs` | `crates/aphrody-tools` (+ `aphrody-guard`) | Approval→exec |
| `apply-patch/*` | `crates/aphrody-tools/src/builtin/apply_patch/` | **P0**, parser pur |
| `app-server` | sous-commande `aphrody session serve` (`crates/cli`) | Multi-thread JSON-RPC |
| `app-server-transport` | `crates/aphrody-session/src/transport.rs` | stdio/UDS/WS |
| `thread-store`/`rollout` | `crates/aphrody-session/src/{thread_store,rollout}.rs` | SQLite + JSONL |
| `tui` | nouveau `aphrody-tui` (probablement dépôt `aphrody-ts`/rust-ui ou ici) | Cloner + rebrand |
| `mcp-server` (tool `codex`) | `crates/google_mcp` (tools `gemini`/`gemini-reply`) | threadId |
| `core-skills` injection/render | `crates/aphrody-skills/src/runtime/{inject,render}.rs` | Budget 8 KB |
| `hooks` | `crates/aphrody-skills/src/hooks.rs` (déjà présent) | Aligner events |
| `login`/auth | `gemini-runtime` / `antigravity-sdk` (token agy) | `GEMINI_API_KEY` |

## 5. Ordre d'implémentation (jalons)

1. **GC-1 `apply_patch`** (P0, faible effort) : porter parser + `seek_sequence` +
   `StreamingPatchParser` dans `aphrody-tools`, exposer comme tool dangereux
   gardé. Tests de round-trip.
2. **GC-2 protocole + thread** : types `Op`/`EventMsg`/`Submission`/`Event`
   (sous-ensemble), `Thread{submit,next_event}` (channels tokio) dans
   `aphrody-session`. Wire NDJSON stdin/stdout.
3. **GC-3 `GeminiModelClient`** : trait + impl REST dans `gemini-runtime` ;
   adaptateur `GeminiStreamEvent` (text delta / functionCall / done+usage).
4. **GC-4 `run_turn`** : boucle minimale streaming + tool-loop (exec via
   `aphrody-tools`/MCP), ré-injection des outputs, `TurnComplete`.
5. **GC-5 approbations** : `ExecApprovalRequest` + `oneshot` + `Op::ExecApproval` ;
   mode autonome = auto-approve gardé par `aphrody-guard` (opt-in `APHRODY_GUARD`).
6. **GC-6 CLI** : `aphrody exec` (JSONL/human) + `aphrody session serve`
   (app-server stdio/UDS).
7. **GC-7 persistence** : rollout JSONL `~/.aphrody/sessions/` + reprise.
8. **GC-8 TUI** : cloner `tui`, rebrand, pointer sur le client app-server aphrody.
9. **GC-9 skills injection** : `build_skill_injections` + rendu `## Skills`.
10. **GC-10 MCP server** : tools `gemini`/`gemini-reply` dans `google_mcp`.
11. **GC-11 SDK/desktop** : SDK TS dans `aphrody-ts` ; l'app Tauri appelle le
    Thread in-process (pattern `run_captured`).

## 6. Points critiques

- **TUI = gain majeur** : ~7.7 k l de rendu/composer/keymap réutilisables 1:1 car
  découplés du LLM via JSON-RPC. Ne pas réinventer.
- **`apply_patch` + Gemini** : injecter les instructions Markdown du format au
  system prompt (Gemini 2.5 ne le maîtrise pas nativement), comme Codex le fait
  pour gpt-4.1.
- **Le seul vrai couplage OpenAI** = `app-server` + couche SSE. Tout le reste est
  agnostique. Concentrer l'effort de réécriture là.
- **Autonomie** : le `Guardian` de Codex ≈ mode no-human-in-the-loop d'aphrody
  (§0.1). Les garde-fous (`aphrody-guard`) restent **désactivés par défaut**,
  opt-in `APHRODY_GUARD=1`.
- **Latence** : réutiliser le client Gemini (cf. objectif projet latence
  minimale), streaming dès le premier delta, pas de hop proxy si Option B.

## 7. Références

- Source : `var/codex/codex-rs/` (OpenAI Codex, Apache-2.0, gitignored).
- Briques déjà portées : `crates/aphrody-guard` (hardening + command-safety).
- Voir aussi `docs/PROTOCOL.md`, `docs/ARCHITECTURE.md`, et la mémoire
  `codex-inspiration-port`.
