<!-- SPDX-License-Identifier: Apache-2.0 -->

# A2A et MCP C# côté peer winclean — pilotage Gemini via agy.exe (Antigravity CLI)

Documentation technique vérifiée sur le code réel du repo peer
`C:\src\winclean` (branche `master`) et sur la configuration globale
Antigravity / Gemini de la machine (`C:\Users\yohan\.gemini`,
`C:\Users\yohan\AppData\Local\agy`). Lecture seule, zéro écriture côté
peer.

> Note de localisation : CLAUDE.md §6.0 et l'ancienne doc référencent
> `C:\winclean\`. Ce chemin **n'existe pas** sur la machine ; le repo réel
> est `C:\src\winclean\` (vérifié : `Test-Path C:\winclean` = False,
> `Test-Path C:\src\winclean` = True). Tous les chemins ci-dessous sont les
> chemins réels. Le repo a des fichiers non commités (notamment
> `src/Winclean.Mcp/NativeMethods.json` untracked et
> `plugins/winclean/bin/Winclean.Mcp.exe` modifié) — non touchés.

## Vue d'ensemble : deux serveurs, un seul état disque

Le peer expose les **mêmes capacités via deux protocoles distincts**, qui
partagent le répertoire `.coord/` comme surface de synchronisation :

- `Winclean.A2a.exe` — bridge **A2A 1.0** sur **HTTP/JSON-RPC, loopback
  `127.0.0.1:5151`** (ASP.NET Core Kestrel, JIT). Cible Gemini.
- `Winclean.Mcp.exe` — serveur **MCP stdio JSON-RPC** (NativeAOT win-x64),
  176 outils P/Invoke. Cible Claude Code (`.mcp.json`) et, par procuration,
  Gemini (le bridge A2A le lance en sous-process).

`Winclean.A2a` ne réimplémente pas les outils : il **spawn
`Winclean.Mcp.exe`** en sous-process stdio et proxifie via les skills
`mcp-list` / `mcp-call` (`src/Winclean.A2a/Program.cs:29-31`,
`src/Winclean.A2a/CoordAgentHandler.cs:101-158`).

---

## 1. Winclean.A2a.exe — transport A2A

### 1.1. Projet et dépendances

`src/Winclean.A2a/Winclean.A2a.csproj` :
- SDK `Microsoft.NET.Sdk.Web`, cible `net10.0-windows10.0.26100.0`,
  RID `win-x64` (`:1-9`).
- `PublishAot=false` (JIT) : le commentaire `:11-17` indique que le paquet
  `A2A` 1.0.0-preview2 est reflection-heavy (System.Text.Json non
  AOT-validé) ; le binaire étant loopback local, le coût de démarrage est
  jugé négligeable.
- Packages : `A2A` 1.0.0-preview2 + `A2A.AspNetCore` 1.0.0-preview2
  (`:28-29`).

### 1.2. Transport et écoute

`src/Winclean.A2a/Program.cs` :
- `WebApplication.CreateBuilder(args)` (`:17`).
- Port : `WINCLEAN_A2A_PORT` env var, défaut **5151** (`:19`).
- **Loopback only** : `ConfigureKestrel(o => o.ListenLocalhost(port))`
  (`:22`) — pas d'API publique.
- Base URL : `WINCLEAN_A2A_BASEURL` ou `http://localhost:{port}` (`:33`).

Le transport est donc **HTTP/JSON-RPC** (et non gRPC, et non un mailbox
`.jsonl` direct — le mailbox est l'état backing, voir §1.5). CLAUDE.md
mentionnait `:8788` pour winclean : c'est inexact pour ce bridge. `:8788`
est en réalité le **listener Bun côté aphrody** (`/.coord/listener.ts:21`,
voir §4).

### 1.3. AgentCard et skills publiés

`Program.cs:35-104` construit un `AgentCard` :
- `Name = "winclean-coord"`, `Version = "1.0.0"` (`:37,39`).
- `SupportedInterfaces` : un `AgentInterface` `Url = {baseUrl}/coord`,
  `ProtocolBinding = "JSONRPC"`, `ProtocolVersion = "1.0"` (`:42-47`).
- `DefaultInputModes = ["text/plain","application/json"]`,
  `DefaultOutputModes = ["application/json"]` (`:49-50`).
- `Capabilities.Streaming = false` (`:51`).
- **7 skills** déclarés (`:52-103`) : `report-status`, `read-status`,
  `assign-task`, `read-task`, `query-stop`, `mcp-list`, `mcp-call`.

Routage (`Program.cs:107-118`) :
- `AddA2AAgent<CoordAgentHandler>(agentCard, _ => {})` (`:107`).
- `app.UseMiddleware<SpecBridgeMiddleware>()` (`:113`).
- `app.MapA2A("/coord")` — endpoint JSON-RPC (`:115`).
- `app.MapWellKnownAgentCard(agentCard)` — publie la carte sur
  `/.well-known/agent-card.json` (`:116`).
- `app.MapGet("/health", ...)` → `{ok, coordRoot, port}` (`:118`).

### 1.4. Schéma d'enveloppe et sérialisation

Le wire format est du **JSON-RPC 2.0** (pas une enveloppe propriétaire
fact/ask/answer ; cette taxonomie fact/ask/answer existe côté aphrody dans
les fichiers `.coord/*.jsonl`, voir §4). Le contrat A2A est :

```http
POST /coord
Content-Type: application/json
{"jsonrpc":"2.0","id":1,"method":"message/send",
 "params":{"message":{"messageId":"<uuid>","role":"user",
   "parts":[{"text":"<skill-id>\n<payload...>"}]}}}
```

Le **dispatch interne** est encodé dans le texte du premier `part`
(`CoordAgentHandler.cs`) :
- `ParseInvocation` (`:160-168`) : 1re ligne = skill id, reste = payload.
- `ExecuteAsync` (`:36-56`) : `switch` sur le skill, réponse renvoyée via
  `MessageResponder.ReplyAsync` (`:38,55`).
- Toutes les réponses sont du **JSON valide même en erreur**
  (`{"ok":false,"error":"..."}` — ex. `:52,66,105,130`).

Sérialisation **System.Text.Json source-generated, AOT-friendly** :
`src/Winclean.A2a/A2aJsonContext.cs:5-8` déclare un `JsonSerializerContext`
partiel (`string`, `string[]`). Le handler s'en sert pour échapper les
chaînes (`CoordAgentHandler.cs:80,92,111,116,...` via
`A2aJsonContext.Default.String`). Les payloads d'arguments MCP transitent
en `System.Text.Json.Nodes.JsonNode` (`:137-152`).

#### Bridge de dialecte — SpecBridgeMiddleware

`src/Winclean.A2a/SpecBridgeMiddleware.cs` traduit le wire format canonique
A2A v1.0 vers le dialecte que le SDK preview2 désérialise réellement :
- N'agit que sur `POST` `application/json` vers `/coord` (`:36-48`).
- Bufferise le body (`EnableBuffering`, `:50-54`) puis réécrit si besoin
  (`NormalizeIfNeeded`, `:87-135`).
- Méthodes : `message/send`→`SendMessage`, `message/stream`→
  `SendStreamingMessage`, `tasks/get`→`GetTask`, `tasks/list`→`ListTasks`,
  `tasks/cancel`→`CancelTask`, `tasks/subscribe`→`SubscribeToTask`
  (`:99-108`).
- Rôles : `"user"`→`1`, `"agent"`→`2` (le SDK attend l'enum entier)
  (`:118-131`).
- JSON malformé : laissé au SDK (`-32700`/`-32602`) (`:67-72`).

### 1.5. Backing store et interface avec le peer aphrody

`src/Winclean.A2a/Coord/CoordStore.cs` — wrapper fin sur le **mailbox
fichier `.coord/`** :
- Racine `coordRoot` injectée en DI singleton
  (`Program.cs:26-27`), résolue par `WincleanPaths.CoordRoot`.
- `ReadStatusAsync`/`WriteStatusAsync` (`:27-55`) lisent/écrivent
  `.coord/STATUS_<AGENT>.txt`, sérialisés par un `SemaphoreSlim` (`:17`).
- `StatusPath` (`:73-82`) canonise : `claude`/`winclean-claude` →
  `STATUS_CLAUDE.txt`, `gemini` → `STATUS_GEMINI.txt`.
- `ReadCurrentTaskAsync`/`WriteCurrentTaskAsync` → `.coord/CURRENT_TASK.md`
  (`:57-69`).
- `IsStopRequested` → présence de `.coord/STOP` (kill switch) (`:71`).

`src/Winclean.A2a/WincleanPaths.cs` résout les chemins :
- `Root` : env `WINCLEAN_ROOT`, sinon remonte depuis
  `AppContext.BaseDirectory` puis `CurrentDirectory` jusqu'à trouver `.git`
  ou `.coord` (`:19-44,79-97`).
- `CoordRoot` : env `WINCLEAN_COORD_ROOT`, sinon `<root>/.coord` (`:46-54`).
- `McpExe` : env `WINCLEAN_MCP_EXE`, sinon 3 candidats — publish Release,
  Debug, ou `plugins/winclean/bin/Winclean.Mcp.exe` (`:56-77`).

L'interface concrète avec aphrody passe par les **fichiers** que le
`CoordStore` lit/écrit. Le commentaire `CoordStore.cs:8-13` est explicite :
le bridge lit/écrit le même état que `aphrody-claude` poll en filesystem
pendant la migration. Le mailbox réel observé contient
`.coord/inbox-from-aphrody.jsonl` (envelopes émises par aphrody, ex.
`apx-handshake-1` à `apx-fact-winclean-unification`),
`.coord/inbox-from-winclean.jsonl`, `.coord/heartbeat-aphrody.txt`,
`.coord/heartbeat-winclean.txt`. (Ces `.jsonl` sont produits côté aphrody,
pas par le code C# A2A — le C# manipule `STATUS_*.txt`, `CURRENT_TASK.md`,
`STOP`.)

### 1.6. Pas de heartbeat dans le code A2A

Le projet A2A n'écrit **pas** de heartbeat lui-même. Le seul writer de
`heartbeat-aphrody.txt` observé est le listener Bun aphrody
(`/.coord/listener.ts:29-31,44`). Le `Capabilities.Streaming = false`
(`Program.cs:51`) confirme l'absence de canal de heartbeat A2A natif.

---

## 2. Winclean.Mcp.exe — serveur MCP et outils

### 2.1. Projet et contraintes AOT

`src/Winclean.Mcp/Winclean.Mcp.csproj` :
- SDK `Microsoft.NET.Sdk`, `OutputType=Exe`, `net10.0`, RID `win-x64`
  (`:30-34`).
- **`PublishAot=true`** + `InvariantGlobalization`, `IlcOptimizationPreference=Speed`
  (`:37-45`).
- Packages clés : `ModelContextProtocol` 1.3.0 (`:15`),
  `Microsoft.Windows.CsWin32` 0.3.275 (P/Invoke source-gen, `:11-14`),
  `Microsoft.Extensions.Hosting` 10.0.8, `DirectNAot` 1.6.1,
  `System.ServiceProcess.ServiceController`, `System.Diagnostics.EventLog`,
  `Microsoft.ML.OnnxRuntime.DirectML` 1.24.4 (WinML),
  `Microsoft.Data.Sqlite` 10.0.8 (Steam DB) (`:8-23`).
- `NoWarn` documente les exemptions trim (IL2026/IL3050) et crypto
  forensique (CA5350/CA5351/CA5358) (`:46-54`).

### 2.2. Hôte et transport stdio

`src/Winclean.Mcp/Program.cs` :
- `Host.CreateApplicationBuilder(args)` (`:15`).
- **Pureté STDIO** : `Logging.ClearProviders()` puis console **redirigée
  vers stderr** (`LogToStandardErrorThreshold = LogLevel.Trace`), min level
  `Warning` (`:19-24`). Tout log sur stdout corromprait le pipeline
  JSON-RPC.
- `AddMcpServer().WithStdioServerTransport()` (`:26-27`).
- Enregistrement des outils par **`.WithTools<T>()`** chaîné — 30 classes
  tool-type listées (`:28-57`) : `WincleanMcpTools`, `AppTools`,
  `ShellTools`, `FileSystemTools`, `ProcessTools`, `ProcessControlTools`,
  `GpuRegistryTools`, `PowerPlanTools`, `ClipboardTools`, `RegistryTools`,
  `NotificationTools`, `InputTools`, `SnapshotTools`, `MultiTools`,
  `ScrapeTools`, `ServiceTools`, `NetworkTools`, `SystemInfoTools`,
  `EventLogTools`, `ScheduledTaskTools`, `DefenderTools`, `WindowsMlTools`,
  `SteamTools`, `SteamLauncherTools`, `GamepadTools`, `MemoryTools`,
  `CoordTools`, `WindowTools`, `BinaryTools`, `PerfTools`.
  (`MemoryToolsWrite` est un `partial` de `MemoryTools`, donc inclus.)

Le handshake observé en live (log Antigravity) :
`{"result":{"protocolVersion":"2024-11-05","capabilities":{"logging":{},"tools":{"listChanged":true}},"serverInfo":{"name":"Winclean.Mcp","version":"1.0.0.0"}},...}`
(`C:\Users\yohan\.gemini\antigravity-cli\brain\fc00b276-.../.system_generated/tasks/task-667.log:1`).

### 2.3. Pattern d'enregistrement et structure d'un outil

Pattern par **attributs du SDK ModelContextProtocol** (réflexion minimale,
source-gen côté SDK), illustré par
`src/Winclean.Mcp/Tools/SystemInfoTools.cs` :
- Classe `internal sealed partial`, attribut `[McpServerToolType]`
  (`:21-23`).
- P/Invoke via `[LibraryImport(...)] private static partial` (source-gen,
  AOT-safe), ex. `GetLogicalProcessorInformation` (`:49-51`).
- Méthode publique statique décorée
  `[McpServerTool, Description("...")]`, ex. `GetCpuInfo()` (`:53-56`),
  `GetCpuUsage(int intervalMs = 250)` (`:139-145`).
- Paramètres documentés par `[Description(...)]` (`:145`).
- **Retour = `string` JSON** sérialisé via le contexte source-gen :
  `JsonSerializer.Serialize(dto, McpJsonContext.Default.CpuInfoDto)`
  (`:136`).
- Garde plateforme `OperatingSystem.IsWindows()` renvoyant une string
  d'erreur sur non-Windows (`:58-61`).

Sérialisation : `src/Winclean.Mcp/Tools/McpJsonContext.cs` —
`JsonSourceGenerationOptions(GenerationMode = Serialization, ...,
PropertyNamingPolicy = CamelCase)` (`:12-16`), avec ~40+ DTO records
`[JsonSerializable]` (`:17-67`) et leurs définitions records (`:69-298`).
Le commentaire `:6-11` insiste : **aucune sérialisation par réflexion**,
tout passe par ce contexte (AOT-safe).

Décompte réel : **176 attributs `[McpServerTool]`** sur 31 fichiers tool
(grep `[McpServerTool` dans `src/Winclean.Mcp/Tools`). Les docs internes
parlent de « 146 tools » (`.coord/A2A_BRIDGE.md:39,46`,
`GEMINI.md:83`) — snapshot antérieur ; le surplus vient des passes
d'extension (`MemoryTools`+`MemoryToolsWrite` 13+13, `WindowTools` 14,
`InputTools` 12, `BinaryTools`/`FileSystemTools` 9, etc.).

Domaines couverts (extraits par préfixe, cf. `.coord/A2A_BRIDGE.md:53-61`)
incluent process memory R/W (`mem_*`, ReadProcessMemory/VirtualQueryEx —
`MemoryTools.cs:54-68`), perf (`perf_*`), window control (`window_*`),
input (`mouse_*`/`keyboard_*`), binary/RE (`binary_*`, IEVR/CRIWARE), Steam,
WinML, registry, services, etc.

### 2.4. Erreurs structurées

Deux conventions coexistent :
- DTO tools : string `"Error: ..."` en cas d'échec natif
  (`SystemInfoTools.cs:60,155,159`) ou exceptions Win32/IO catchées
  renvoyant `"Error querying ...: {ex.Message}"` (`:259-266,305-316`).
- `CoordTools` : JSON structuré `{"ok":false,"action":...,"error":...}` via
  `JsonError(action, message)` (`CoordTools.cs:143-144`), avec un encodeur
  JSON manuel NativeAOT-safe `JsonString` (`:146-175`) qui échappe
  `"`, `\`, contrôles `< 0x20` → `\uXXXX`.

Côté client A2A, `McpStdioClient.DispatchInbound` (`:292-305`) transforme un
`error` JSON-RPC entrant en `InvalidOperationException("MCP error: ...")`,
que le handler reconvertit en `{"ok":false,...,"error":...}`
(`CoordAgentHandler.cs:154-157`).

### 2.5. CoordTools — miroir MCP des skills A2A

`src/Winclean.Mcp/Tools/CoordTools.cs` expose **5 outils `coord_*`** qui
lisent/écrivent le **même `.coord/`** que le bridge A2A
(`:14-19,24`) : `CoordReportStatus` (`:33`), `CoordReadStatus` (`:62`),
`CoordAssignTask` (`:84`), `CoordReadTask` (`:106`), `CoordQueryStop`
(`:122`). C'est la **passerelle cross-protocole** : un client MCP (Claude)
et un client A2A (Gemini) coordonnent via le même fichier disque.

---

## 3. Pilotage Gemini via agy.exe (Antigravity CLI)

### 3.1. Le binaire agy

`agy.exe` **existe** mais **hors du repo winclean** (config utilisateur
globale) :
- `C:\Users\yohan\AppData\Local\agy\bin\agy.exe` (binaire).
- `C:\Users\yohan\.local\bin\agy.cmd` (wrapper PATH) :
  `"...\agy.exe" --dangerously-skip-permissions %*` — lance agy en mode YOLO
  (skip permissions), cohérent avec la philosophie « Max Autonomy »
  documentée (`apps/mcp/docs/best-practices/antigravity.md:5`).
- Application desktop associée :
  `C:\Users\yohan\AppData\Local\Programs\Antigravity\Antigravity.exe`
  (Electron).

Aucune trace de `agy`/Antigravity **dans le code source winclean** (le grep
`agy` dans le repo ne ramène que des chaînes base64 non liées). La glue est
donc : (a) le `.mcp.json` du repo, (b) la config globale d'Antigravity CLI.

### 3.2. Configuration MCP que agy lit réellement

Fichier vivant utilisé par Antigravity CLI :
`C:\Users\yohan\.gemini\antigravity-cli\mcp_config.json` :

```json
{
  "mcpServers": {
    "winclean-core": {
      "command": "C:\\src\\winclean\\src\\Winclean.Mcp\\bin\\Release\\net10.0\\win-x64\\publish\\Winclean.Mcp.exe",
      "args": [],
      "env": { "WINCLEAN_ROOT": "C:\\src\\winclean", "RELOAD_TRIGGER": "2" }
    },
    "microsoft-learn": { "serverUrl": "https://learn.microsoft.com/api/mcp" }
  }
}
```

Donc agy découvre `winclean-core` comme **serveur MCP local stdio**,
commande = chemin absolu vers `Winclean.Mcp.exe`, plus un serveur HTTP/SSE
distant `microsoft-learn`. (`C:\Users\yohan\.gemini\config\mcp_config.json`
existe mais est vide/1 ligne — c'est `antigravity-cli/mcp_config.json` qui
fait foi.)

Le repo fournit aussi un `.mcp.json` à sa racine
(`C:\src\winclean\.mcp.json:3-11`) avec la **même** définition
`winclean-core` (sans `microsoft-learn`) — c'est le manifest workspace que
Claude Code charge ; le rapport agy
(`mcp_configuration_report.md:22`) indique que la config globale a été
obtenue en « fusionnant les définitions du workspace dans la config globale
Antigravity CLI ».

Le projet est aussi enregistré côté Antigravity comme dossier git avec
write :
`C:\Users\yohan\.gemini\config\projects\0249e89b-...json:6-12`
(`folderUri: file://C:/src/winclean`, `allowWrite: true`), lié dans le repo
par le symlink `C:\src\winclean\.antigravitycli\<uuid>.json` →
`C:\Users\yohan\.gemini\config\projects\<uuid>.json`.

### 3.3. Flux découvert dans les artefacts agy

Un rapport produit par une vraie session agy le documente :
`C:\Users\yohan\.gemini\antigravity-cli\brain\fc00b276-.../mcp_configuration_report.md`
- Diagramme (`:54-60`) :
  `Google Antigravity CLI --JSON-RPC via STDIO--> winclean-core MCP -->
  Native APIs --> Windows OS` ; et `--HTTP/SSE--> microsoft-learn`.
- Best practices Antigravity (`:13-16`) : modularité, chemins absolus,
  interpolation `${VAR}`, pureté STDIO.
- Hot-reload : « Antigravity recharge automatiquement la config à la
  sauvegarde de `mcp_config.json` » (`:62-63`) — d'où le `RELOAD_TRIGGER`.

Logs de session (preuves d'exécution réelle) :
- `.../brain/fc00b276-.../.system_generated/tasks/task-67.log:2` :
  `MCP subprocess ready: ...\publish\Winclean.Mcp.exe pid=7632`.
- `.../task-667.log:1` : handshake `initialize` retournant
  `serverInfo.name=Winclean.Mcp`.
- `.../brain/8e3efd86-.../walkthrough.md:37` : agy a vérifié l'intégration
  en appelant la skill A2A `mcp-list` qui a énuméré « 146 native MCP
  tools ».

### 3.4. Directives Gemini (GEMINI.md)

`C:\src\winclean\GEMINI.md` est le brief Gemini CLI :
- Rôle : orchestrateur ML + extracteur binaire pour la RE de `nie.exe`
  (`:13-32`).
- Recommande de parler **au bridge A2A `localhost:5151`** plutôt que de
  scruter `.coord/` à la main (`:36-49`), avec exemples `curl` `health`,
  `agent-card.json`, et `POST /coord` (`:42-48,86-109`).
- Tableau des 7 skills A2A (`:74-84`), dont `mcp-call` pour piloter
  directement un outil natif (ex. `binary_parse_pe_header` sur `nie.exe`,
  `:104-108`).
- Mode YOLO, kill-switch `.coord/STOP` (`:143-152`).
- Interdits : `C:\src\aphrody` intouchable, commits autonomes interdits
  (`:158,164`).

### 3.5. Note sur a2a_orchestrator.py

`src/Winclean.MlCore/winclean_ml/pipeline/a2a_orchestrator.py` est un **PoC
Python mocké**, pas la glue agy réelle : il écrit `CURRENT_TASK.md` /
`STATUS_*.txt` et **simule** l'extraction (`:72-78`) ; l'appel `gemini`/
`claude` est commenté (`:71,87,92`). Il n'invoque pas `agy.exe`. À ne pas
confondre avec le pilotage Antigravity décrit en §3.2-3.3.

---

## 4. Flux end-to-end A2A aphrody ↔ winclean ↔ gemini

### 4.1. Topologie des canaux

D'après `C:\src\winclean\.coord\ai.json` (`kind:"coord"`) et le code :

| Canal | Owner | Endpoint / Path | Rôle |
| --- | --- | --- | --- |
| `file_jsonl` (mailbox) | bidirectionnel | `.coord/inbox-from-aphrody.jsonl` + `inbox-from-winclean.jsonl` | mailbox durable primaire (`ai.json:25-32`) |
| `http_jsonrpc` (aphrody) | aphrody | `http://localhost:8788` `/ping` `/msg` `/inbox` | listener Bun aphrody (`ai.json:33-43`, `listener.ts:21,46-88`) |
| A2A HTTP/JSON-RPC (winclean) | winclean | `http://localhost:5151/coord` + `/.well-known/agent-card.json` | bridge C# `Winclean.A2a` (`Program.cs:19,115-116`) |
| MCP stdio (winclean) | winclean | `Winclean.Mcp.exe` | 176 outils, lancé par `.mcp.json`/agy ou par le bridge A2A |
| `heartbeat_file` | bidirectionnel | `.coord/heartbeat-{aphrody,winclean}.txt` | TTL 600s (`ai.json:44-50`) |
| `git_tag`, `markdown_doc`, `process_inspect` | bidirectionnel | tags `aphrody-*`/`winclean-*`, `COLLABORATION-APHRODY.md` | signaux secondaires (`ai.json:51-68`) |

Taxonomie d'enveloppe (côté aphrody, dans les `.jsonl`) :
`type`/`kind` ∈ {`ask`, `fact`, `ack`, `ping`}, champs
`id,ts,from,to,re,subject,body,channel_hint`
(`inbox-from-aphrody.jsonl:1-2`, schéma
`ai.json:71-77` : `ack_required_for:["ask"]`, `max_message_kb:64`,
`rate_limit_per_minute:60`, `preferred_language:"fr"`).

### 4.2. Chemin nominal « Gemini appelle un outil natif via A2A »

1. agy (Antigravity CLI) démarre, lit
   `~/.gemini/antigravity-cli/mcp_config.json`, et **spawn
   `Winclean.Mcp.exe`** en stdio (`winclean-core`) → handshake `initialize`
   `2024-11-05` (preuve : `task-67.log`, `task-667.log`).
2. En parallèle, Gemini peut viser le **bridge A2A** `localhost:5151`
   (recommandé par `GEMINI.md:36-49`). Le bridge a, à son boot, lui-même
   spawné une **2e** instance `Winclean.Mcp.exe` comme `IHostedService`
   (`Program.cs:30-31`, `McpStdioClient.StartAsync:47-104`) avec handshake
   `initialize` + `notifications/initialized` (`:78-96`).
3. Gemini `POST /coord` avec `parts[0].text = "mcp-call\n<tool>\n<args
   JSON>"`. `SpecBridgeMiddleware` normalise le dialecte
   (`SpecBridgeMiddleware.cs:99-131`).
4. `CoordAgentHandler.HandleMcpCallAsync` (`:120-158`) parse nom+args et
   appelle `McpStdioClient.CallToolAsync` → `tools/call` JSON-RPC sur le
   sous-process (`McpStdioClient.cs:154-163`, framing newline-delimited
   `WriteFrameAsync:210-224`, dispatch par id `DispatchInbound:292-305`).
5. L'outil natif s'exécute (P/Invoke Win32), renvoie une string JSON ;
   le handler répond `{"ok":true,"tool":...,"result":<json>}`
   (`CoordAgentHandler.cs:149-152`).

### 4.3. Chemin de coordination « aphrody ↔ winclean »

- Aphrody écrit ses envelopes dans `.coord/inbox-from-aphrody.jsonl` et
  bump `heartbeat-aphrody.txt` (et historiquement via son listener Bun
  `:8788`).
- Côté winclean, n'importe quel client (Claude via MCP `coord_*`, Gemini
  via skills A2A `report-status`/`read-status`, ou le bridge directement)
  lit/écrit `STATUS_*.txt`, `CURRENT_TASK.md`, `STOP` — c'est-à-dire le même
  `.coord/` que `CoordStore`/`CoordTools` manipulent. La synchronisation se
  fait **sur disque** (`A2A_BRIDGE.md:49`, `CoordTools.cs:14-19`).
- Migration in-tree aphrody : aphrody a déplacé sa source de vérité vers
  `C:\src\aphrody\ai\` tout en gardant un miroir best-effort vers
  `C:\src\winclean\.coord\inbox-from-aphrody.jsonl`
  (`inbox-from-aphrody.jsonl:38`).

---

## Classification de fiabilité par section

- **(1) Winclean.A2a.exe transport — FAIT.** Code source complet lu et cité
  (Program/Handler/CoordStore/McpStdioClient/SpecBridgeMiddleware/
  WincleanPaths/A2aJsonContext). Correction notable : port réel **5151**
  (pas 8788), transport **HTTP/JSON-RPC** (pas gRPC), enveloppe **JSON-RPC
  A2A** (la taxonomie fact/ask/answer vit côté aphrody dans les `.jsonl`).
- **(2) MCP C# serveur + tools — FAIT.** Program.cs, McpJsonContext,
  CoordTools, SystemInfoTools, MemoryTools lus ; pattern attributs +
  `.WithTools<T>()` + source-gen JSON confirmé ; **176** `[McpServerTool]`
  réels (docs disaient 146 = snapshot antérieur, écart documenté).
- **(3) Pilotage Gemini via agy.exe Antigravity — FAIT.** `agy.exe` et son
  wrapper `.cmd` localisés (hors repo, config globale) ; `mcp_config.json`
  vivant lu ; rapport + logs de session agy prouvant le spawn et le
  handshake de `Winclean.Mcp.exe`. Précision : aucune référence à agy dans
  le code du repo ; `a2a_orchestrator.py` est un PoC mocké, pas la glue agy.
- **(4) Flux end-to-end — FAIT.** Topologie reconstruite depuis `.coord/
  ai.json`, le code A2A/MCP et les logs agy réels. Précision : `:8788` =
  listener Bun aphrody (`listener.ts`), distinct du bridge C# `:5151`.
