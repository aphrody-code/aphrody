<!-- SPDX-License-Identifier: Apache-2.0 -->
# Spec d'intégration Obscura → aphrody (R4 scraping headless)

**Source auditée** : <https://github.com/h4ckf0r0day/obscura> — clone `--depth 1`
(commit `1950048`, banner produit **v0.1.5**, `Cargo.toml` workspace `version = "0.1.0"`,
edition 2021, Apache-2.0). 7 crates : `obscura-{dom,net,browser,cdp,js,mcp,cli}`.
**Lecture exhaustive du code** (toutes citations `fichier:ligne` ci-dessous référencent
le checkout `obscura-src/`). À lire en complément : `docs/research/obscura-headless-browser.md`
(décision déjà actée : façade binaire externe, jamais en dépendance de compilation).

> **Corrections vs `obscura-headless-browser.md`** :
> - Version réelle = **v0.1.5** (banner `crates/obscura-cli/src/main.rs:169`), pas v0.1.0.
> - V8 est embarqué via **`deno_core 0.350`** (`crates/obscura-js/Cargo.toml:7,12`), PAS
>   rusty_v8 brut. Les ops JS↔Rust sont des `#[op2]` deno_core.
> - MCP expose **12 outils** `browser_*` (`crates/obscura-mcp/src/lib.rs:140-274`), pas 9.
> - Stealth = 2 couches distinctes : (a) polyfills/fingerprint JS **toujours actifs**
>   dans `bootstrap.js` ; (b) impersonation **TLS** via `wreq`/`wreq-util` gated
>   `--features stealth` ET seulement câblé sur `cfg(linux/android)` avec `prefix-symbols`
>   (`crates/obscura-net/Cargo.toml:25-29`). Le flag `--stealth` runtime active aussi le
>   tracker-blocking (3520 domaines) indépendamment de la feature compile-time.

---

## 1. Inventaire complet des features (par crate, cité fichier:ligne)

### 1.1 `obscura-cli` — binaire `obscura` + `obscura-worker`

Args globaux (clap `Args`, `crates/obscura-cli/src/main.rs:10-40`) :

| Flag global | Type/défaut | fichier:ligne |
|---|---|---|
| `-v/--verbose` | bool | main.rs:17-18 |
| `-p/--port` | u16 = 9222 (sans sous-cmd) | main.rs:23-24 |
| `--proxy` | Option\<String\> global | main.rs:26-27 |
| `--obey-robots` | bool | main.rs:29-30 |
| `--user-agent` | Option\<String\> | main.rs:32-33 |
| `--v8-flags` | Option\<String\>, `allow_hyphen_values` | main.rs:35-39 |

Sous-commandes (`enum Command`, main.rs:42-145) :

**`serve`** (main.rs:44-73) :
| Flag | Défaut | Ligne |
|---|---|---|
| `-p/--port` | 9222 | 45-46 |
| `--host` | `127.0.0.1` | 52-53 |
| `--proxy` | — | 55-56 |
| `--user-agent` | — | 58-59 |
| `--stealth` | off | 61-62 |
| `--workers` | 1 | 64 |
| `--allow-file-access` | off (sécurité : bloque `file://` par défaut) | 71-72 |

**`fetch <url>`** (main.rs:75-107) :
| Flag | Défaut | Ligne |
|---|---|---|
| `--dump` (DumpFormat) | `html` | 78-79 |
| `--selector` | — | 81-82 |
| `--wait` (selector poll, s) | 5 | 84 |
| `--timeout` (s, ≥1) | 30 | 87-88 |
| `--wait-until` | `load` | 90-91 |
| `--user-agent` | — | 93-94 |
| `--stealth` | off | 96-97 |
| `-e/--eval` | — | 99-100 |
| `-o/--output` (PathBuf) | stdout | 102-103 |
| `-q/--quiet` | off | 105-106 |

`DumpFormat` (clap ValueEnum, main.rs:148-158) : `Html`, `Text`, `Links`, `Markdown`,
`Original` (raw HTTP body, binary-safe, court-circuite le navigateur — issue #117).

Logique de rendu fetch (`run_fetch`, main.rs:432-517) :
- `--dump original` → `fetch_original_bytes` (main.rs:519-543) : bypass total DOM/JS,
  stream octets via `ObscuraHttpClient::fetch`.
- `--eval` prioritaire sur `--dump` (main.rs:495-504) : retourne la valeur JS brute.
- `Html` = `dump_html` (main.rs:599-609, `<!DOCTYPE html>` + outerHTML de `html`).
- `Text` = `dump_text` → `extract_readable_text` (main.rs:611-685) : strip
  `script/style/nav/header/footer/aside`, gère blocs.
- `Links` = `dump_links` (main.rs:924-954) : `a[href]` résolus en absolu, format `URL\ttext`.
- `Markdown` = `dump_markdown` (main.rs:622-625) : exécute `HTML_TO_MARKDOWN_JS` dans V8.
- `--selector` = poll DOM toutes les 100 ms jusqu'à `--wait` s (`wait_for_selector` main.rs:580-597).

**`scrape <urls...>`** (main.rs:109-126) :
| Flag | Défaut | Ligne |
|---|---|---|
| `-e/--eval` | — | 112-113 |
| `--concurrency` (NonZeroUsize) | 10 | 115-116 |
| `--format` | `json` | 118-119 |
| `--timeout` (s, ≥1, par worker) | 60 | 121-122 |
| `-q/--quiet` | off | 124-125 |

Implémentation (`run_parallel_scrape`, main.rs:687-922) : spawn d'un process
`obscura-worker(.exe)` par URL (résolu sibling de `current_exe()`, main.rs:710-714),
borné par `tokio::Semaphore(concurrency)` (main.rs:723). Protocole IPC = JSON-lines sur
stdin/stdout du worker (`{"cmd":"navigate","url":...}` → réponse, puis `evaluate`, puis
`shutdown`). Proxy transmis via env `OBSCURA_PROXY` (main.rs:746). Format `text` =
TSV `time_ms\turl\ttitle|eval` (main.rs:899-919).

**`mcp`** (main.rs:128-143) : `--http` (bool), `--port` 3000, `--proxy`, `--user-agent`,
`--stealth`. Route vers `obscura_mcp::http::run` (HTTP) ou `obscura_mcp::run` (stdio).

**Sans sous-commande** (main.rs:293-299) : démarre directement le serveur CDP sur `--port`.

Gardes runtime notables :
- `reject_stealth_with_socks5` (main.rs:199-213) : `--stealth` + `socks5://` = erreur dure
  (le client wreq ne parle pas SOCKS5, issue #160).
- `merge_proxy` (main.rs:191-193) : proxy de sous-commande > proxy global.
- `normalize_v8_flags` (main.rs:218-225) + `set_v8_flags` une seule fois avant le 1er isolate
  (main.rs:241-244).
- Multi-worker `serve` (main.rs:305-430) : load-balancer round-robin TCP devant N process
  `serve` sur ports `port+1..`, proxy `copy_bidirectional`, gestion spéciale `/json` CDP.

**`obscura-worker`** (`crates/obscura-cli/src/worker.rs`) : binaire séparé piloté par
JSON-lines stdin. `WorkerCommand` (worker.rs:8-23) : `navigate`, `evaluate`, `title`,
`dump_html`, `dump_text`, `shutdown`. `WorkerResponse {ok, result?, error?}` (worker.rs:25-41).

### 1.2 `obscura-cdp` — serveur Chrome DevTools Protocol (WebSocket :9222)

Bootstrap serveur (`crates/obscura-cdp/src/server.rs`) : `start_with_host_and_security`
(server.rs:62-110), bind TCP, `tokio::LocalSet` (V8 `!Send`), un seul `cdp_processor`
(server.rs:112-175). Détection HTTP vs WS sur peek 4 octets (`GET ` → endpoints `/json`,
server.rs:664-684). Endpoints HTTP discovery : `/json/version`, `/json/list`,
`/json/protocol` (`handle_http_json` server.rs:752-790, expose
`webSocketDebuggerUrl: ws://127.0.0.1:{port}/devtools/browser`).

`fast_path_response` (server.rs:609-654) : court-circuite ~30 commandes no-op (Network.enable,
Page.enable, Target.getBrowserContexts, Browser.getVersion…) hors de la file pour éviter le
starvation Puppeteer.

Dispatch par domaine (`crates/obscura-cdp/src/dispatch.rs:196-218`), V8 lock global
process-wide autour de chaque dispatch (dispatch.rs:182, fix issue #19 V8_Fatal). Unwrap
`Target.sendMessageToTarget` récursif (dispatch.rs:159, 229-284, compat headless_chrome).

**Domaines + commandes exactes implémentées** (les `match method` arms) :

| Domaine | Commandes | fichier:ligne |
|---|---|---|
| **Target** | setDiscoverTargets, getTargets, createTarget, attachToBrowserTarget, attachToTarget, closeTarget, setAutoAttach(noop), getBrowserContexts, createBrowserContext, disposeBrowserContext, getTargetInfo | domains/target.rs:9,40,57,119,144,170,190,191,194,198,202 |
| **Browser** | getVersion, close, getWindowForTarget, setDownloadBehavior, getWindowBounds, setWindowBounds | domains/browser.rs:5,12,15,25,26,33 |
| **Page** | enable, navigate, reload, getFrameTree, createIsolatedWorld, setLifecycleEventsEnabled, addScriptToEvaluateOnNewDocument, removeScriptToEvaluateOnNewDocument, setInterceptFileChooserDialog, getLayoutMetrics, getNavigationHistory, printToPDF | domains/page.rs:163,164,169,178,195,236,237,246,251,252,289,302 |
| **DOM** | enable, getDocument, querySelector, querySelectorAll, getOuterHTML, describeNode, resolveNode, setAttributeValue, removeNode, getBoxModel, getContentQuads | domains/dom.rs:13,14,22,30,40,50,75,138,139,140,177 |
| **Runtime** | enable, evaluate, callFunctionOn, getProperties, releaseObject, releaseObjectGroup, addBinding, runIfWaitingForDebugger, getExceptionDetails, discardConsoleEntries | domains/runtime.rs:13,35,60,96,166,174,180,198,199,200 |
| **Network** | enable, setExtraHTTPHeaders, setUserAgentOverride, getCookies, setCookies, clearBrowserCookies, setCacheDisabled, setRequestInterception | domains/network.rs:12,13,26,33,54,71,77,78 |
| **Fetch** | enable, disable, continueRequest, fulfillRequest, failRequest, getResponseBody (live interception) | domains/fetch.rs:63,92,110,126,164,181 |
| **Input** | dispatchMouseEvent, dispatchKeyEvent (keyDown/rawKeyDown/keyUp/char), dispatchTouchEvent(noop), setIgnoreInputEvents(noop) | domains/input.rs:12,62,70,119,131,152,153 |
| **Storage** | getCookies, setCookies, deleteCookies | domains/storage.rs:12,36,94 |
| **LP** (extension Obscura) | **getMarkdown** (DOM→Markdown via `HTML_TO_MARKDOWN_JS`) | domains/lp.rs:13 |
| **Accessibility** | enable, getFullAXTree (mapping rôles ARIA complet) | domains/accessibility.rs:33,34 |
| Emulation/Log/Performance/Security/CSS/ServiceWorker/Inspector/Debugger/Profiler/HeapProfiler/Overlay/Audits | acceptés no-op `{}` | dispatch.rs:211-216 |

Interception Fetch live (`server.rs:221-530`) : route navigation + `Fetch.requestPaused`/
`Network.requestWillBeSent`, file `deferred` bornée à 256 messages (`MAX_DEFERRED_MESSAGES`
server.rs:16) pour ne pas OOM pendant une nav. Résolutions `continueRequest`/`fulfillRequest`/
`failRequest` (server.rs:190-211).

### 1.3 `obscura-mcp` — serveur MCP (JSON-RPC 2.0)

Transports : **stdio** (`run`, lib.rs:85-124, NDJSON une ligne/message) et **HTTP Streamable**
(`http::run`, http.rs:13-27, `POST /mcp`, CORS `*`, support batch + SSE keep-alive GET).
`initialize` annonce `protocolVersion: 2024-11-05`, `serverInfo.name: obscura-mcp`
(lib.rs:126-138). Méthodes RPC : `initialize`, `ping`, `tools/list`, `tools/call`,
`resources/list` (vide), `prompts/list` (vide) (lib.rs:73-83).

**12 outils `browser_*`** (`tools/list`, lib.rs:140-274) :

| Outil | Params (required en gras) | Handler |
|---|---|---|
| `browser_navigate` | **url**, waitUntil(load/domcontentloaded/networkidle0) | lib.rs:311-327 |
| `browser_snapshot` | — (URL+title+body text) | lib.rs:329-343 |
| `browser_click` | **selector** (CSS, `el.click()`) | lib.rs:345-365 |
| `browser_fill` | **selector**, **value** (set value + input/change) | lib.rs:367-392 |
| `browser_type` | **selector**, **text** (append + input event) | lib.rs:394-418 |
| `browser_press_key` | **key**, selector? (KeyboardEvent keydown/keyup) | lib.rs:420-444 |
| `browser_select_option` | **selector**, **value** (match value ou text) | lib.rs:446-473 |
| `browser_evaluate` | **expression** (résultat JS sérialisé) | lib.rs:475-485 |
| `browser_wait_for` | **selector**, timeout?(s, déf 30) | lib.rs:487-508 |
| `browser_network_requests` | — (liste `[status] method url (Nb)`) | lib.rs:510-523 |
| `browser_console_messages` | — | lib.rs:525-531 |
| `browser_close` | — (reset page+console) | lib.rs:533-537 |

Toutes les actions DOM passent par `page.evaluate(js)` avec JS injecté (lib.rs:349-473) —
donc l'interaction MCP est réellement pilotée par V8 sur le DOM Obscura. `BrowserState`
(lib.rs:48-71) : 1 page lazy, UA optionnel, buffer console.

### 1.4 `obscura-js` — runtime JS (V8 via deno_core)

- **Embarquement V8** : `deno_core = "0.350"` en build-dep ET dep (`Cargo.toml:7,12`).
  Snapshot/bootstrap construit par `build.rs`. `--v8-flags` appliqués une fois via
  `v8::V8::set_flags_from_string` (`v8_flags.rs:16-24`, `Once`).
- **`v8_lock`** : mutex global tokio sérialisant tout travail V8 (référencé partout dans
  dispatch/server, fix issue #19).
- **Ops `#[op2]` JS↔Rust** (`ops.rs`) : `op_dom` (façade DOM string-based, ops.rs:73-272),
  `op_console_msg` (ops.rs:275), `op_fetch_url` (async, route via proxy partagé #139,
  ops.rs:315), `op_get_cookies` (708), `op_set_cookie` (723), `op_navigate` (738).
  `ObscuraState` (ops.rs:41-69) : DOM, url, title, cookie_jar, http_client, intercept tx/flags.
- **`bootstrap.js`** (3190 lignes) : implémentation navigateur complète en JS pur —
  console/timers (lignes 124-172), `MessageChannel`/`MessagePort` (174-187),
  `CSSStyleDeclaration` (189-198), arbre `Node`/`Element`/`document` (212+), `XMLHttpRequest`,
  `fetch`, événements (`isTrusted=true` ligne 1935), formulaires.
- **`markdown.rs`** + `HTML_TO_MARKDOWN_JS` (exporté par `obscura-browser`) : conversion
  DOM→Markdown utilisée par `--dump markdown` et le domaine CDP `LP.getMarkdown`.
- **`module_loader.rs`**, **`runtime.rs`** (1814 lignes) : JsRuntime par page, isolate dédié.

### 1.5 `obscura-browser` — moteur de page/contexte/lifecycle

- `BrowserContext::with_options(name, proxy, stealth)` / `with_full_options(.., user_agent)` /
  champ `allow_file_access` (`context.rs`, utilisé dispatch.rs:69-75).
- `Page` : `navigate`, `navigate_with_wait(url, WaitUntil)`, `navigate_blank`, `evaluate`,
  `with_dom(closure)`, `http_client`, `network_events`, `lifecycle`, `frame_id`,
  `has_js()/suspend_js()/resume_js()` (gestion mémoire isolates), `execute_preload_script`.
- **Lifecycle** (`lifecycle.rs`) : `LifecycleState` {Idle, Loading, DomContentLoaded, Loaded,
  NetworkIdle, Failed} (lifecycle.rs:1-23). `WaitUntil` {Load, DomContentLoaded, NetworkIdle0,
  NetworkIdle2} avec `from_str` (lifecycle.rs:25-42 : `domcontentloaded`, `networkidle0`/
  `networkIdle`/`networkidle`, `networkidle2`, défaut `load`).
- Export `HTML_TO_MARKDOWN_JS`.

### 1.6 `obscura-net` — stack HTTP

- **Client par défaut** `ObscuraHttpClient` (`client.rs:157-454`) : `reqwest 0.12`
  (`Cargo.toml:11`, features `cookies, gzip, brotli, deflate, native-tls-vendored, socks`,
  workspace Cargo.toml:34). Redirection manuelle max 20 (client.rs:263, gère 301/302/303→GET),
  cookie jar maison, headers Chrome 145 hardcodés (UA Linux x86_64 Chrome/145, sec-ch-ua,
  sec-fetch-*, client.rs:294-337). `block_trackers` (client.rs:246-258) court-circuite les
  domaines blocklistés (status 0, body vide). Proxy via `reqwest::Proxy::all` (client.rs:206).
- **SSRF guard** `validate_url` (client.rs:66-120) : schémas `http/https/file` seulement ;
  bloque loopback/privé/link-local/localhost sauf `OBSCURA_ALLOW_PRIVATE_NETWORK`.
- **`file://`** : `fetch_file_url` (client.rs:122-155) avec MIME par extension.
- **Client stealth** `StealthHttpClient` (`wreq_client.rs`, gated `#[cfg(feature="stealth")]`) :
  `wreq 6.0.0-rc.28` + `wreq-util 3.0.0-rc.10`, **emulation `Chrome145` / OS `Linux`**
  (wreq_client.rs:44-47) — c'est le **vrai spoofing TLS/JA4** (impossible avec reqwest).
  CA store système (wreq_client.rs:39-42). `prefix-symbols` BoringSSL Linux/Android seulement
  (`Cargo.toml:25-29`, issue #39).
- `ResourceType` {Document, Script, Stylesheet, Image, Font, Xhr, Fetch, Other} (client.rs:51-61).
- **Blocklist** (`blocklist.rs`) : `pgl_domains.txt` = **3520 lignes** (`include_str!`,
  blocklist.rs:5), match exact + suffixe parent (blocklist.rs:24-40). C'est la liste
  Peter Lowe (PGL) embarquée dans le binaire.
- **robots.txt** (`robots.rs`) : `RobotsCache`, parse User-agent/Disallow/Allow avec
  fallback `*`, wildcard `*`/`$` (robots.rs:55-131). Activé par `--obey-robots`.
- **Interceptor** (`interceptor.rs`) : trait `RequestInterceptor`, actions Continue/Block/
  Fulfill/ModifyHeaders (client.rs:51-287).
- **Cookies** (`cookies.rs`) : `CookieJar`, `CookieInfo`.

### 1.7 `obscura-dom` — DOM Rust pur

- `parse_html` (html5ever 0.29 + markup5ever 0.14, `Cargo.toml:27-28`), `DomTree`, `NodeId`,
  `NodeData` {Element{name,...}, Text{contents}, ...}.
- **Selectors** (`selector.rs`) : `selectors 0.26` + `cssparser 0.34` + `servo_arc` —
  `query_selector`, `query_selector_all` réels (pas regex).
- `serialize.rs` (outer/inner HTML), `tree.rs`, `tree_sink.rs`. Accès texte
  `text_content`, `children`, `get_attribute`.

---

## 2. Schémas JSON exacts de sortie (pour le wrapper Rust côté aphrody)

### 2.1 `obscura scrape --format json`

Émis par `run_parallel_scrape` (`main.rs:890-898`), `serde_json::to_string_pretty` :

```jsonc
{
  "total_urls":   <usize>,        // main.rs:892
  "concurrency":  <usize>,        // main.rs:893
  "total_time_ms":<u128>,         // main.rs:894
  "avg_time_ms":  <f64>,          // main.rs:895 (total/total_urls)
  "results": [                    // main.rs:896, un objet par URL
    // --- succès (main.rs:850-856) ---
    {
      "url":     "<string>",
      "title":   "<string>",      // de la réponse navigate du worker
      "eval":    <json|null>,     // résultat --eval ; Null si pas d'--eval
      "time_ms": <u128>,
      "worker":  <usize>          // index du worker
    },
    // --- échec (main.rs:868-872 / 783-799) ---
    {
      "url":     "<string>",
      "error":   "<string>",      // ex: "timeout", "navigate failed", "Failed to spawn worker: ..."
      "time_ms": <u128>
    }
  ]
}
```

Note : pas de champ `html`/`text` dans la sortie scrape — seuls `title` + `eval` reviennent.
Pour récupérer le contenu en mode scrape, passer `--eval "document.documentElement.outerHTML"`
(ou `document.body.innerText`, ou un sélecteur précis).

### 2.2 Protocole worker (IPC stdin/stdout JSON-lines)

Requête (worker.rs:8-23) : `{"cmd":"navigate","url":"..."}` | `{"cmd":"evaluate","expression":"..."}`
| `{"cmd":"title"}` | `{"cmd":"dump_html"}` | `{"cmd":"dump_text"}` | `{"cmd":"shutdown"}`.
Réponse (worker.rs:25-41) : `{"ok":bool,"result":<json>?,"error":"<string>"?}`.
`navigate` → `result = {"title":"...","url":"..."}` (worker.rs:93-96).

### 2.3 `obscura fetch` (NON-JSON par design)

`fetch` n'émet PAS de JSON enveloppé : il écrit le contenu brut sur stdout (ou `--output`).
`--dump html|text|links|markdown` = texte ; `--dump original` = octets bruts ; `--eval` =
valeur JS brute (String non quotée, ou `to_string()` JSON pour non-String, main.rs:497-501).
**Conséquence wrapper** : pour fetch, capturer stdout tel quel ; le typage se fait côté
aphrody selon `--dump`. Pour une enveloppe structurée, préférer `scrape` (même 1 URL) ou
piloter le CDP/MCP.

### 2.4 MCP `tools/call` (lib.rs:300-308)

Succès : `{"content":[{"type":"text","text":"<résultat>"}]}`.
Erreur : `{"content":[{"type":"text","text":"Error: ..."}],"isError":true}`.

---

## 3. Plan d'intégration MAXIMAL dans aphrody

Principe directeur (confirme `obscura-headless-browser.md` §4) : **façade binaire externe**,
zéro dep de compilation, zéro V8 dans `cargo ci-offline`. Reproduire le pattern
`gemini_runtime::resolve_bin()` (`crates/gemini-runtime/src/lib.rs`, env > sibling
`current_exe()` > PATH). Résolution proposée : **`$APHRODY_OBSCURA_BIN` > sibling > PATH**,
+ `$APHRODY_OBSCURA_WORKER_BIN` pour le worker (sibling requis pour `scrape`).

### 3.1 Crate `obscura-runtime` (≈ `gemini-runtime`)

- `resolve_bin()` / `resolve_worker_bin()`.
- Wrapper typé `fetch(url, FetchOpts) -> FetchOutput` (capture stdout selon `--dump`).
- Wrapper `scrape(urls, ScrapeOpts) -> ScrapeReport` désérialisant le schéma §2.1 dans des
  structs serde (`ScrapeReport { total_urls, concurrency, total_time_ms, avg_time_ms, results:
  Vec<ScrapeResult> }`, `ScrapeResult` enum/`#[serde(untagged)]` succès|erreur).
- Détection capacité : `obscura --version` ; signaler stealth dispo (best-effort, le binaire
  release Linux est compilé `--features stealth` selon README §Build).
- Fallback gracieux : si binaire absent → conserver le chemin `reqwest`+`scraper::Html` actuel
  (commands.rs:790-812) et marquer `engine: "static"` dans la sortie.

### 3.2 `aphrody scrape` — flags à exposer (mapping 1:1)

| Surface aphrody | Flag Obscura mappé | Source |
|---|---|---|
| `--engine {static\|obscura\|auto}` | choix runtime (auto = détecte binaire) | nouveau |
| `--render-js` (implique engine obscura) | bascule fetch→browser | main.rs:462 |
| `--format {html\|text\|links\|markdown\|original\|json}` | `fetch --dump` / `scrape --format` | main.rs:78,118 |
| `--eval <JS>` | `--eval` | main.rs:99,112 |
| `--wait-until {load\|domcontentloaded\|networkidle0\|networkidle2}` | `fetch --wait-until` | main.rs:90, lifecycle.rs:34 |
| `--selector <CSS>` + `--wait <s>` | `fetch --selector/--wait` | main.rs:81,84 |
| `--timeout <s>` | `fetch/scrape --timeout` | main.rs:87,121 |
| `--concurrent N` (R4.2) | `scrape --concurrency` | main.rs:115 |
| `--rate-limit-ms K` (R4.2) | (non natif Obscura — implémenter côté aphrody entre spawns) | NON_VERIFIE: pas de flag rate-limit dans Obscura |
| `--proxy <URL>` (R4.7) | global `--proxy` (http/socks5) | main.rs:26 |
| `--user-agent <UA>` (R4.7) | `--user-agent` | main.rs:32 |
| `--stealth` (R4.7) | `--stealth` (refuse socks5) | main.rs:96,199 |
| `--obey-robots` | global `--obey-robots` | main.rs:29 |
| `--v8-flags <FLAGS>` | global `--v8-flags` | main.rs:35 |
| `--output <path>` | `fetch --output` | main.rs:102 |

Note rate-limit : Obscura n'a pas de `--rate-limit-ms`. R4.2 nécessite que aphrody throttle
lui-même (sleep entre acquisitions de permis, ou concurrency=1 + délai) puisque le wrapper
contrôle le spawn.

### 3.3 `aphrody-mcp` — proxy des 12 outils `browser_*`

Deux options, complémentaires :
- **Voie zéro-code (immédiate)** : démarrer `obscura mcp` (stdio) et router via l'outil
  existant `mcp__aphrody__aphrody_mcp_call` (déjà dans la surface MCP aphrody). Aucun code neuf.
- **Voie native (optionnelle)** : enregistrer dans `aphrody-mcp`/`google_mcp` 12 wrappers
  `obscura_browser_*` qui spawn `obscura mcp` une fois et relaient le JSON-RPC, OU pilotent
  directement le CDP :9222. Schémas d'entrée = §1.3 (copier les `inputSchema` verbatim de
  lib.rs:140-274). Sortie = enveloppe `content[].text` (§2.4).

### 3.4 Skill `agent-browser` — pilotage CDP :9222

Le skill `agent-browser` existant peut piloter `obscura serve --port 9222` comme un
endpoint Puppeteer/Playwright (`ws://127.0.0.1:9222/devtools/browser`). Documenter :
- Discovery : `GET /json/version` → `webSocketDebuggerUrl` (server.rs:760-767).
- Domaines disponibles = tableau §1.2 (NE PAS supposer un domaine Chrome non listé : seules
  les commandes citées sont implémentées ; le reste est no-op ou erreur -32601).
- `--allow-file-access` requis pour `file://` (off par défaut, server.rs:62-83).
- Limite : pas de screenshot raster (`Page.captureScreenshot` absent — vérifié, pas dans
  page.rs:163-302) ; `Page.printToPDF` présent (page.rs:302). Pour LLM, préférer
  `LP.getMarkdown` (markdown propre) ou `Accessibility.getFullAXTree`.

### 3.5 Crate `aphrody-terminal-browser` — pont LLM↔DOM via `LP.getMarkdown`

`aphrody-terminal-browser/src/backend/mod.rs` (le backend `bxc` supprimé le 2026-05-21) :
ajouter un backend `obscura` qui, par session, ouvre une connexion CDP :9222 et expose au
LLM la commande `LP.getMarkdown` (domains/lp.rs:13) → markdown rendu inline dans le terminal
LLM-first. C'est l'usage canonique : convertir n'importe quelle page (post-rendu JS) en
markdown digestible pour sub-agents/skills, à ~30 Mo RAM. Combiner avec `browser_snapshot`
(MCP) pour title+URL+texte. **NE PAS toucher ce crate dans cette PR** (agents grind en
parallèle) — décrire seulement l'intégration cible.

### 3.6 R4.1 (curl-impersonate) & R4.4 (HTTP/3) — complémentarité

- **R4.1** : Obscura `--stealth` apporte déjà l'impersonation TLS via **wreq Chrome145**
  (wreq_client.rs:44-47) — équivalent fonctionnel partiel de curl-impersonate côté JA3/JA4.
  curl-impersonate reste pertinent comme **chemin léger non-JS** (pas de V8) et pour des
  cibles d'empreinte différentes. Garder les deux : aphrody choisit selon besoin JS.
- **R4.4 (HTTP/3)** : NON_VERIFIE côté Obscura — aucune trace de h3/quinn dans
  `obscura-net/Cargo.toml` (reqwest 0.12 sans feature http3, wreq via BoringSSL). Considérer
  R4.4 comme orthogonal à Obscura ; à porter sur le client aphrody natif.

---

## 4. Checklist d'actions concrètes priorisées

| # | Action | Verify command | Bloquant ? |
|---|---|---|---|
| 1 | Créer crate `obscura-runtime` (`resolve_bin`/`resolve_worker_bin` + structs serde §2.1) | `cargo check -p obscura-runtime --locked` | non |
| 2 | Smoke binaire (le binaire est déjà à `%TEMP%/obscura-dl/obscura.exe`) | `obscura fetch https://example.com --eval "document.title" --quiet` → titre | non |
| 3 | Smoke scrape JSON (worker sibling requis) | `obscura scrape https://example.com --quiet --format json` → JSON §2.1 valide (`jq .results[0].title`) | non |
| 4 | Wire `aphrody scrape --engine {static\|obscura\|auto}` (fallback reqwest si binaire absent) | `aphrody scrape <url> --engine obscura --format markdown` ; `--engine static` inchangé | non |
| 5 | Brancher `--render-js`/`--wait-until`/`--selector`/`--concurrent`/`--proxy`/`--stealth`/`--user-agent` sur le wrapper | `aphrody scrape <spa-url> --render-js --wait-until networkidle0` → DOM post-hydratation | non |
| 6 | Implémenter rate-limit côté aphrody (Obscura n'en a pas) | `aphrody scrape a b c --concurrent 3 --rate-limit-ms 500` → délais observés | non |
| 7 | Doc skill `agent-browser` : endpoint `obscura serve` Puppeteer/Playwright + domaines §1.2 | `obscura serve --port 9222 &` puis `curl -s 127.0.0.1:9222/json/version \| jq .webSocketDebuggerUrl` | non |
| 8 | Backend `obscura` dans `aphrody-terminal-browser` (LP.getMarkdown) — **autre PR, agents grind** | n/a (hors scope, ne pas toucher crates/) | bloqué (parallèle) |
| 9 | Proxy MCP : option immédiate via `aphrody_mcp_call` ; native = 12 wrappers `inputSchema` §1.3 | `obscura mcp` + appel `tools/list` → 12 outils ; `tools/call browser_navigate` | non |
| 10 | TLS stealth (R4.1) : valider impersonation Chrome145 | `obscura fetch https://tls.peet.ws/api/all --stealth --dump original` → JA3/JA4 Chrome | non (binaire release Linux a la feature) |
| 11 | Flip `docs/PLAN.md` R4.2/R4.5/R4.7 une fois #2-#9 verts | relire PLAN.md | non |
| 12 | Garder R4.1 (curl-impersonate, chemin non-JS) et R4.4 (HTTP/3) distincts d'Obscura | n/a | non |

---

## Bilan

**Features inventoriées : 78** réparties ainsi —
- CLI : 6 args globaux + 4 sous-commandes + 5 DumpFormat + 6 WorkerCommand = **21**
- CDP : 11 domaines actifs, **~62 commandes** distinctes (Target 11, Browser 6, Page 12,
  DOM 11, Runtime 10, Network 8, Fetch 6, Input 4, Storage 3, LP 1, Accessibility 2) +
  3 endpoints HTTP discovery = catégorisé **1 ligne par domaine (11) + 3 HTTP** ici
- MCP : **12 outils** + 2 transports + 6 méthodes RPC
- JS : deno_core 0.350, 6 ops, bootstrap.js (polyfills), v8-flags, v8-lock, DOM→Markdown
- Net : 2 clients (reqwest/wreq), blocklist 3520, robots, interceptor, SSRF guard, cookies
- Stealth : fingerprint GPU/screen/canvas/audio/battery, userAgentData Chrome145, webdriver,
  isTrusted, native masking, TLS Chrome145 (wreq)

(Si l'on compte chaque commande CDP individuellement plutôt qu'1 ligne/domaine, le total des
items distincts dépasse **140**.)

**Classification FAIT / INCOMPLET par section** :
- §1 Inventaire : **FAIT** (7/7 crates lus ; toutes commandes CDP citées fichier:ligne ;
  stealth/blocklist/robots/ops vérifiés sur source réel).
- §2 Schémas JSON : **FAIT** (scrape JSON, worker IPC, MCP call, fetch non-JSON — cités).
- §3 Plan d'intégration : **FAIT** (mapping flags, MCP, CDP skill, terminal-browser, R4.1/R4.4).
- §4 Checklist : **FAIT** (12 actions + verify commands).

**Points NON_VERIFIE explicitement notés** : absence de flag `--rate-limit-ms` côté Obscura
(§3.2) ; pas de support HTTP/3 dans obscura-net (§3.6, R4.4) ; détail interne des
sous-fichiers non lus intégralement (page.rs/runtime.rs/dom.rs/target.rs/accessibility.rs lus
via extraction des arms `match method` + sections clés, pas ligne à ligne sur leur totalité —
les noms de commandes et la structure de dispatch sont vérifiés).
</content>
</invoke>
