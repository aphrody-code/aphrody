# Antigravity IDE — Reverse-engineering approfondi

Cible : `C:\Users\<user>\AppData\Local\Programs\Antigravity IDE`
Analyse : read-only, machine perso (logiciel installé par le user, analyse autorisée).
Date : 2026-05-22. Artefacts bruts : `var/data/antigravity-ide-re/`.

Classification de complétude (par phase) :

| Phase | Sujet | Statut |
|-------|-------|--------|
| 1 | Deep RE filesystem (extensions, Jetski, language server) | **FAIT** |
| 2 | Pipeline binaire `aphrody re` | **FAIT** (cargo check exit 0, pipeline exécuté) |
| 3 | Corrélation `antigravity-sdk` + gaps | **FAIT** |

Découverte majeure : **Antigravity est un fork Google de Windsurf / Codeium** (et non un produit greenfield). Tout l'agent et l'auth transitent par un language server natif Go de 133 Mo (`exa.*` protobuf, package `cortex`, moteur `Cascade`), pas par le JS de l'extension. C'est le centre de gravité de la RE.

---

## 1. Identité / versions

Source : `var/data/antigravity-ide-re/tree-map.json` (l.4-8) + `re-google.json`.

- IDE **Antigravity 2.0.2**, base **VSCode 1.107.0**, commit `bd0307c171dbaf4cd6135192515e160af7d9d132` (2026-05-21).
- **Electron 39.2.3**, **Chromium 142.0.7444.175** (confirmé par `re google` sur l'exe : `re-google.json` l.3).
- Code-sign subject : **Google Inc** (`re-google.json` l.7).
- Binaire principal `Antigravity IDE.exe` : 210 848 768 octets (~211 Mo).
- Gallery extensions : **open-vsx.org** (`package.json` cœur, `marketplaceExtensionGalleryServiceURL`).
- Publisher de toutes les extensions Antigravity : `google`.

---

## 2. Arbre & artefacts

Réf. `tree-map.json` (1578 entrées top-level IDE) + `scan-tree.json` (récursif `resources/app` : **38 914 fichiers, 697 Mo, 25 685 `.js`**).

Sous-dossiers cœur `resources/app/extensions/antigravity/` :
- `dist/extension.js` — 1.96 Mo, point d'entrée VSCode (mince : juste de la glu).
- `bin/language_server_windows_x64.exe` — **133 599 744 octets (~133 Mo)** : le vrai cerveau (Go, Codeium-derived).
- `bin/sandbox-wrapper.sh` + `.LICENSE` — wrapper d'exécution sandboxée du code agent.
- `dist/languageServer/cert.pem` — 1250 octets : certificat pour le endpoint HTTPS local self-signed du LS.
- `out/jetskiAgent/main.js` — **12.4 Mo** : webview de l'agent UI « Jetski ».
- `cascade-panel.html`, `auth-success-jetski.html` — webviews.
- `schemas/mcp_config.schema.json`, `customEditor/`, `assets/auth/google_signin.svg`.

Note interne révélée par le schema MCP (`mcp_config.schema.json`) :
`$comment: Keep in sync with google3/third_party/jetski/cortex/utils/mcp/config.schema.json`
→ chemin source Google3 : **`google3/third_party/jetski/`**, module **`cortex`**.

---

## 3. Extensions Antigravity (5)

### 3.1 `antigravity` (cœur agent) — `package.json`

- `name: antigravity`, `publisher: google`, `version: 0.2.0`, `main: ./dist/extension.js`, `activationEvents: ["*"]`.
- `enabledApiProposals`: `contribSourceControlInputBoxMenu`, `inlineCompletionsAdditions`, **`antigravityUnifiedStateSync`** (API proposal propriétaire pour la synchro d'état).
- **Authentication provider** : `id: antigravity_auth`, `label: Antigravity`.
- **Custom editors** : `antigravity.workflowEditor` (`.agent/workflows`, `.gemini/jetski*/global_workflows`, `.gemini/antigravity*/global_workflows`), `antigravity.ruleEditor` (`.agent/rules`).
- **Commandes** (extraits notables) :
  - `antigravity.login`, `antigravity.loginWithAuthToken` (« Backup Login » = token manuel),
  - `antigravity.copyApiKey` (« Copy API Key to Clipboard »),
  - `antigravity.prioritized.chat.open` (chat agent), keybindings `antigravity.prioritized.agent*` (alt+j/k/enter pour accept/reject de hunks), `supercompleteAccept`/`Escape` (Tab/Esc),
  - `antigravity.restartLanguageServer`, `killLanguageServerAndReloadWindow`, `togglePersistentLanguageServer`, `openPersistentLanguageServerLog`,
  - `antigravity.openBrowser`, `showBrowserAllowlist` (navigateur agentique allowlisté),
  - Imports : VSCode / Cursor / **Windsurf** / Cider (interne Google) settings + extensions,
  - `startDemoMode` / `endDemoMode` (Beta).
- **Settings** : `antigravity.searchMaxWorkspaceFileCount` (def 5000 — *« Jetski will attempt to compute embeddings for workspaces up to this many files »*), `marketplaceExtensionGalleryServiceURL`/`GalleryItemURL` (open-vsx), `persistentLanguageServer`, `enableCursorImportCursor`.
- **MCP** : langage `jsonc` mappé sur `mcp_config.json` + jsonValidation vers `schemas/mcp_config.schema.json`.
- **Indice d'origine Codeium** : menu `editor/context` groupe **`CodeiumGroup@1`** ; resolution `@exa/agent-ui-toolkit: file:../../../exa/agent_ui_toolkit` ; importOrder prettier `^@exa/(.*)$`. **`exa` = nom de code Codeium**.

### 3.2 `antigravity-code-executor`

Mince. Une commande : `antigravity-code-executor.executeCode`. Exécute le code généré par l'agent (couplé au `sandbox-wrapper.sh`).

### 3.3 / 3.4 / 3.5 `antigravity-dev-containers` / `-remote-openssh` / `-remote-wsl`

Forks des extensions **open-remote-ssh / open-remote-wsl** (cf. `LICENSE.open-remote-*.txt`). Renommage Codeium → Antigravity. Commandes standard de remote dev (`reopenInContainer`, `closeSSHProcess`, `connectUsingDistro`, …). Pas de surface AI ; intéressantes seulement pour le remoting de l'agent.

---

## 4. Agent Jetski

- **Jetski** = nom de code de l'agent UI (front webview). Le runtime Go côté serveur s'appelle **`cortex`** ; le moteur d'exécution agentique **`Cascade`** (hérité de Windsurf).
- Webview chargée via `workbench-jetski-agent.html` → `jetskiAgent.js` → `out/jetskiAgent/main.js` (12.4 Mo) + `jetskiMain.tailwind.css`. CSP `trusted-types ... google#safe`.
- **Hosts de feature-flagging exposés dans la CSP** (`workbench-jetski-agent.html` l.48-49) :
  `http://jetski-unleash.corp.goog/` et `http://antigravity-unleash.goog/` (Unleash = système de feature flags ; `corp.goog` = réseau interne Google).
- Protocole workbench↔agent : webview standard VSCode (`postMessage`, 11 occurrences dans main.js). L'agent parle au LS via le proto **`exa.cortex_pb`** (référencé dans `main.js`).
- Concepts UI confirmés dans `main.js` : `BYOM` (Bring-Your-Own-Model — `gemini-v3-byom`), `BattleMode` / `battle mode` (comparaison de modèles côte-à-côte ; côté LS `v1internal:battleModeOverrides`, `cortex.EndBattleModeError`), crédits `g1-credits` / `g1-activity` (système de quota « G1 »).
- Modèles référencés dans le JS Jetski (`main.js` l.3874) : `gemini-2.5-flash-image`, `gemini-3-pro-image`, `gemini-3.1-flash-image`, et (LS) `gemini-3.1-pro`, `gemini-3-pro-preview`, `gemini-3-flash-preview`, `gemini-2.5-pro`, `gemini-2.5-pro-windsurf` (!), `gemini-coder`.
- Doc/légal pointés : `antigravity.google/docs/{faq,enterprise,strict-mode}`, `antigravity.google/{faq,terms,g1-credits,g1-activity}`.
- Sidecars / config agent : `.gemini/config/sidecars/`, OAuth scope Drive (`www.googleapis.com/auth/drive`) → l'agent peut lire Google Drive.

---

## 5. Endpoints / RPC / auth (extraits du language server natif)

Tous extraits par `strings` + grep sur `bin/language_server_windows_x64.exe`. Aucun token/secret en clair recopié.

### 5.1 Hosts backend Google
- `https://cloudcode-pa.googleapis.com` (**Cloud Code Private API — surface principale**)
- `https://daily-cloudcode-pa.googleapis.com` (variante daily/runtime ; le crate SDK la traite comme défaut)
- `https://aiplatform.googleapis.com` (Vertex AI : publishers/google **et** publishers/anthropic — Claude exposé via Vertex)
- `aicode.googleapis.com:443` (gRPC dédié)
- `https://feedback-pa.googleapis.com`, `play.googleapis.com/log`, `alkalimakersuiteapplets.pa.googleapis.com` (telemetry / feedback)
- `docs.googleapis.com`, `secretmanager.googleapis.com`, `cloudkms.googleapis.com`, `iamcredentials.googleapis.com`, `modelarmor.googleapis.com` (sécurité / enterprise)

### 5.2 Méthodes RPC Cloud Code `v1internal:*` — **37 méthodes** (liste complète : `var/data/antigravity-ide-re/v1internal-methods.txt`)

Génération / agent :
`generateContent`, `streamGenerateContent`, `generateChat`, `streamGenerateChat`, `generateCode`, `completeCode`, `transformCode`, `tabChat`, **`internalAtomicAgenticChat`** (cœur agentique), `countTokens`.

Session / compte / quota :
`loadCodeAssist`, `onboardUser`, `onboardUserBackgroundTasks`, `fetchAvailableModels`, `listModelConfigs`, `listAgents`, `fetchUserInfo`, `retrieveUserQuota`, `getCodeAssistGlobalUserSetting`, `setCodeAssistGlobalUserSetting`, `setUserSettings`, `fetchCodeCustomizationState`, `fetchAdminControls`, `listCloudAICompanionProjects`, `listExperiments`.

Agent / repo / contexte :
`searchSnippets`, `listRemoteRepositories`, `fetchFromTrawlerCache`, `migrateDatabaseCode`, `rewriteUri`, `checkUrlDenylist`, `battleModeOverrides`.

Telemetry / feedback :
`recordClientEvent`, `recordCodeAssistMetrics`, `recordSmartchoicesFeedback`, `recordTrajectoryAnalytics`, `registerInteraction`.

Transcoding gRPC↔REST via connect-go (`/v1/connect.*`).

### 5.3 Proto Codeium / exa (gRPC interne LS)
- `exa.api_server_pb.ApiServerService` — service principal (centaines de messages : `Cascade*`, `AgentPlugin*`, `Trajectory*`, `CheckHybridDeploymentStatus`, `*OidcProvider`, `*TeamOrganizationalControls`, …).
- `exa.analytics_pb.AnalyticsService` — `RecordCompletions`, `RecordCortexTrajectory(Step)`, `RecordCommandUsage`, `RecordPrompt`.
- `exa.cortex_pb` (référencé côté Jetski JS), `exa.codeium_common_pb.ImageData`.
- Package Go : `cortex/agentapi`, `cortex/cascade_manager.go`, `cortex/battlemode.go`, `cortex/annotations_manager.go`, `cortex/artifacts`, `cortex.stateSyncSummariesStore`, `cortex.trajectoryFetcherImpl`, `cortex.providerServerURLMap`.

### 5.4 Auth OAuth 2.0
- **Deux client IDs publics** embarqués (non secrets, présents dans toute copie distribuée) :
  - `1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com` (= `ANTIGRAVITY_CLIENT_ID` du SDK — **confirmé identique**)
  - `884354919052-36trc1jjb3tguiac32ov6cod268c5blh.apps.googleusercontent.com` (= `ANTIGRAVITY_CLIENT_ID_SECONDARY` du SDK — **confirmé identique**)
- Endpoints : `accounts.google.com/o/oauth2/...`, `oauth2.googleapis.com/token`, `oauth2.mtls.googleapis.com/token` (mTLS), **`oauth2.googleapis.com/device/code`** (device-code flow — non couvert par le SDK), redirect `oauth-callback`.
- Scopes (présents dans le LS) : `cloud-platform`, `userinfo.email`, `userinfo.profile`, `cclog`, **`experimentsandconfigs`**, **`aicode`**, + une famille **Drive** large (`drive`, `drive.appdata`, `drive.file`, `drive.metadata.readonly`, `drive.meet.readonly`, `drive.photos.readonly`, `drive.scripts`, `drive.apps.readonly`).
- `authProviderType: google_credentials` pour MCP (`mcp_config.schema.json`), + bloc `oauth {clientId, clientSecret}` par serveur MCP.
- État persistant : API proposal `antigravityUnifiedStateSync` + `UnifiedStateSync`/`stateSync` côté LS (cf. décodeur `state_sync.rs` du SDK).

### 5.5 Secrets — inventaire (aucune valeur recopiée)
- Token utilisateur : **non présent dans les binaires** ; stocké au runtime (Credential Manager `gemini:antigravity` / state.vscdb). Conforme à l'invariant sécurité du SDK.
- Les deux `clientId` OAuth sont publics par conception (desktop = Auth-Code + PKCE, pas de client secret confidentiel). `cert.pem` (1250 o) est un cert local self-signed du LS, pas une clé privée d'auth Google.

---

## 6. Sortie pipeline `aphrody re`

`cargo check -p aphrody --offline` → **exit 0** (build OK ; warnings de profil swc non bloquants). Pipeline exécuté, artefacts dans `var/data/antigravity-ide-re/` :

- `re-triage.json` (256 Ko) — triage de `Antigravity IDE.exe`.
- `re-google.json` — `family: electron`, `chromium_version: 142.0.7444.175`, `code_sign_subject: "Google Inc"`. **`oauth_client_ids`, `google_endpoints`, `grpc_*` vides** : le détecteur `re google` cible l'exe Electron, où ces artefacts sont absents.
- `re-strings.txt` — strings de l'exe Electron (4000 entrées, surtout symboles natifs).
- `scan-tree.json` — 38 914 fichiers / 697 Mo sous `resources/app`.
- `v1internal-methods.txt` — **artefact le plus utile** : les 37 méthodes RPC (extraites du language server, pas via `re`).

**Limite identifiée du pipeline `aphrody re`** : il a été pointé sur l'exécutable Electron (`Antigravity IDE.exe`), mais 100 % de la surface réseau/RPC/auth vit dans `extensions/antigravity/bin/language_server_windows_x64.exe` (Go, 133 Mo). `re google`/`re strings` sur l'exe Electron ne remontent donc rien d'exploitable. Toute la section 5 a été produite par `strings` + grep manuel sur le LS. Magika non lancé (optionnel, lourd ; skip assumé).

---

## 7. Corrélation avec `crates/antigravity-sdk/` + gaps

Ce que le SDK couvre **et qui est confirmé exact** par cette RE :
- `ANTIGRAVITY_CLIENT_ID` + `_SECONDARY` : **les deux exacts**, byte-for-byte.
- Hosts `cloudcode-pa` + `daily-cloudcode-pa` + `aiplatform` : **confirmés**.
- `oauth2.googleapis.com/token`, `accounts.google.com/o/oauth2/v2/auth` : confirmés.
- Credential Manager `gemini:antigravity`, décodeur `UnifiedStateSync` : confirmés (clés présentes dans le LS).
- Scopes `cloud-platform`/`userinfo.*`/`cclog`/`experimentsandconfigs` : confirmés.
- 3 méthodes typées : `loadCodeAssist`, `fetchAvailableModels`, `onboardUser` — corrects.

### Gaps exploitables (ce que l'IDE utilise et que le SDK ne couvre PAS)

**(A) Méthodes RPC manquantes — le SDK n'a que 3/37.** Manquent les méthodes à plus forte valeur :
- Génération : `streamGenerateContent`, `generateChat`/`streamGenerateChat`, `completeCode`, `tabChat`, **`internalAtomicAgenticChat`** (la boucle agent réelle), `countTokens`.
- Compte/quota : `fetchUserInfo`, `retrieveUserQuota`, `getCodeAssistGlobalUserSetting`/`set...`, `listModelConfigs`, `listAgents`, `listExperiments`, `fetchAdminControls`, `listCloudAICompanionProjects`.
- Contexte : `searchSnippets`, `listRemoteRepositories`, `fetchFromTrawlerCache`, `checkUrlDenylist`.
- Telemetry : `recordClientEvent`, `recordTrajectoryAnalytics`, `registerInteraction`.

**(B) Scope `aicode` + famille Drive.** Le SDK ne demande ni `aicode` ni les scopes Drive, alors que l'agent les utilise (sidecars, lecture Drive). À ajouter à `ANTIGRAVITY_SCOPES` si on veut la parité fonctionnelle.

**(C) Device-code flow.** Le LS supporte `oauth2.googleapis.com/device/code` (login headless sans navigateur, idéal CLI/CI). Le SDK ne fait que le loopback PKCE. **À ajouter** — fort intérêt pour le mode autonome/headless d'aphrody.

**(D) mTLS.** `oauth2.mtls.googleapis.com/token` présent ; le SDK n'utilise que le token endpoint clair.

**(E) Modèles à jour.** Le SDK documente `gemini-2.0-flash` ; l'IDE expose `gemini-3.1-pro`, `gemini-3-pro-preview`, `gemini-3-flash-preview`, `gemini-3.1-flash-image`, `gemini-3-pro-image`, `gemini-coder`, `gemini-v3-byom`. La liste de modèles du SDK est en retard d'une génération.

**(F) Transport gRPC `exa.*`.** Le SDK (par choix documenté) tape les endpoints Google directement en REST/Bearer et ignore le bridge gRPC local `exa.api_server_pb.ApiServerService`. C'est plus portable, mais on perd l'accès à des features purement-locales (cache Trawler, Cascade trajectory, BattleMode). Décision à conserver, mais documenter le gap.

**(G) Concepts produit non modélisés.** `BattleMode` (`battleModeOverrides`), crédits **G1** (`retrieveUserQuota` + `g1-credits`), `BYOM` (`gemini-v3-byom`), Trawler cache, sidecars. Aucune représentation côté SDK.

---

## 8. Recommandations pour `aphrody agy` / `aphrody antigravity` / `aphrody ide`

1. **Étendre `endpoints.rs` aux 37 méthodes `v1internal`** (au moins en constantes `METHOD_*`), en priorité `streamGenerateContent`, `internalAtomicAgenticChat`, `countTokens`, `retrieveUserQuota`, `fetchUserInfo`, `listModelConfigs`. Source de vérité : `var/data/antigravity-ide-re/v1internal-methods.txt`.
2. **Ajouter le device-code flow** (`oauth2.googleapis.com/device/code`) à `oauth.rs` → débloque `aphrody agy login --headless` pour CI / VPS, aligné §0.1 (zéro humain dans la boucle).
3. **Mettre à jour `ANTIGRAVITY_SCOPES`** : ajouter `aicode` (et, derrière un flag, la famille Drive) pour parité agent.
4. **Rafraîchir la liste de modèles** (`models.rs` doc + une éventuelle table) vers la gamme gemini-3.x, et modéliser `BYOM`.
5. **Sous-commande `aphrody ide inspect <path>`** : généraliser cette RE. Le détecteur `re google` actuel ne regarde que l'exe Electron — **le rendre récursif sur `resources/app/extensions/*/bin/*.exe`** pour attraper les language servers Go embarqués (c'est là que vivent les endpoints). C'est le bug de couverture #1 du pipeline.
6. **`aphrody re` : ajouter un mode « deep strings sur sidecar binary »** qui extrait hosts `*.googleapis.com`, `v1internal:*`, scopes `auth/*`, client IDs `*.apps.googleusercontent.com`, et services proto `exa.*_pb.*` — exactement les regex utilisés ici. Automatiserait toute la §5.
7. **Modéliser `retrieveUserQuota` / crédits G1** dans `models.rs` pour exposer le quota Ultra/Enterprise au CLI.
8. **Conserver** le choix « REST direct vs bridge gRPC local », mais documenter explicitement dans `endpoints.rs` que `exa.api_server_pb.ApiServerService` (cache Trawler, Cascade, BattleMode) est hors périmètre par conception.

---

### Annexe — origine confirmée

Antigravity est un **fork Google de Windsurf / Codeium** : marqueurs `CodeiumGroup@1`, namespace proto `exa.*`, dépendance `@exa/agent-ui-toolkit`, modèle `gemini-2.5-pro-windsurf`, forks `open-remote-ssh/wsl`, commande d'import Windsurf. Le nom de code Google interne est **Jetski** (`google3/third_party/jetski/`), le runtime agent **cortex** (Go), le moteur d'exécution **Cascade** (Windsurf).
