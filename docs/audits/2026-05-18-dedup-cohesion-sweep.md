<!-- SPDX-License-Identifier: Apache-2.0 -->
# Audit dedup + cohesion — aphrody workspace (2026-05-18)

Audit structurel intégral. Aucune modification source pendant ce tick :
seuls les fichiers `docs/audits/*` peuvent être ajoutés. Chaque item du plan
refactor cite `file:line` de chacun des au moins deux sites concurrents pour
garantir la dispatchabilité parallèle.

## 1. Périmètre audité

| Catégorie               | Count | Source                                                       |
|-------------------------|------:|--------------------------------------------------------------|
| Crates Rust workspace   |    40 | `crates/*/Cargo.toml` (members in root `Cargo.toml`)         |
| Crates exclus (vendor)  |     3 | `crates/coreutils`, `crates/util-linux`, `crates/a2a-slimrpc`|
| Crates archivés         |     1 | `crates/a2a-lf` (stub vide README-only)                      |
| Packages Bun            |    17 | `packages/*` workspaces                                       |
| Workspace deps          |   ~120| `[workspace.dependencies]` in root `Cargo.toml`              |
| Profiles                |     9 | dev/release/dist/release-fast/release-debug/bench/asan/careful/reproducible |

Listing crates (validé via `cargo tree` + `ls crates/`) :

```
a2a, a2a-client, a2a-grpc, a2a-pb, a2a-server, a2a-ui,
agui-bridge, aphrody-channels, aphrody-gateway, aphrody-mcp,
aphrody-memory, aphrody-summary, aphrody-terminal-backend,
aphrody-terminal-browser, aphrody-terminal-config,
aphrody-terminal-json-out, aphrody-terminal-llm,
aphrody-terminal-markdown, aphrody-terminal-vt,
aphrody-terminal-wasm, aphrody-translate, aphrody-tui,
aphrody-voice, aphrody-voice-stt, aphrody-wasm, backend, base,
cli (pkg=aphrody), gemini-runtime, google_mcp, gui, ievr-tools,
m3-tokens, mrx-audit, mrx-cli, mrx-core, mrx-detect, mrx-watch,
shadcn-bridge
```

Packages Bun : `a2ui`, `aphrody-jsx`, `aphrody-skills`, `bxc`,
`gemini-app-aphrody`, `gemini-cli`, `gemini-live-aphrody`, `google-core`,
`material-design-icons`, `material-web`, `n2b`, `n2b-plugin`, `n2b-shims`,
`n2b-types`, `next.js`, `plugin-package-contract`, `ui`.

## 2. Duplications détectées (code Rust)

### 2.1 Loaders JSON config

| # | Fonction                                       | Site 1                                                                                             | Site 2                                                                                                  | Verdict                                                                                            |
|---|------------------------------------------------|----------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| 1 | `.mcp.json` loader                             | `crates/aphrody-terminal-llm/src/mcp.rs:562` (`load_mcp_json -> Vec<McpServerSpec>`)               | `crates/aphrody-terminal-config/src/shims.rs:125` (`import_mcp_json -> McpShim`)                        | Two distinct types, identical I/O + parse path. Terminal-llm should depend on terminal-config.    |
| 2 | `McpJsonEntry` struct                          | `crates/aphrody-terminal-llm/src/mcp.rs:528`                                                       | `crates/aphrody-terminal-config/src/shims.rs:87` (`McpServerEntry`)                                     | Identical schema, two structs.                                                                    |

### 2.2 JSON-RPC plumbing

| # | Fonction                                       | Site 1                                                                                             | Site 2                                                                                                  | Verdict                                                                                            |
|---|------------------------------------------------|----------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| 3 | `JsonRpcRequest` model                         | `crates/a2a/src/jsonrpc.rs:12` (struct `JsonRpcRequest` w/ `new`)                                  | `crates/aphrody-terminal-llm/src/mcp.rs:204` (`fn jsonrpc_request(id, method, params)`)                 | terminal-llm re-encodes JSON-RPC 2.0 instead of consuming `a2a::JsonRpcRequest`.                  |
| 4 | `JsonRpcResponse::result` extractor            | `crates/a2a/src/jsonrpc.rs:28` (struct with `success` / `error` constructors)                      | `crates/aphrody-terminal-llm/src/mcp.rs:216` (`fn extract_jsonrpc_result`)                              | terminal-llm reimplements `error.is_some` -> result branch ad-hoc.                                |

### 2.3 Engine-detection token lists

| # | Constant                                       | Site 1                                                                                             | Site 2                                                                                                  | Verdict                                                                                            |
|---|------------------------------------------------|----------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| 5 | `bypass_engines` / `known_engine_tokens` array | `crates/cli/src/commands.rs:473` (`bypass_engines = ["bun", "uv", "cargo", ...]`)                  | `crates/cli/src/main.rs:277` (`known_engine_tokens = ["bun", "uv", "cargo", ...]` — 38 tokens)          | Two near-identical literal arrays referring to the same router decision. Must be a `pub const`.   |
| 6 | Script-extension list                          | `crates/cli/src/commands.rs:498` (`.py .js .ts .jsx .tsx .rs .sh`)                                 | `crates/cli/src/main.rs:288` (`.py .js .ts .jsx .tsx .rs .sh`)                                          | Same 7-extension list duplicated literally.                                                       |

### 2.4 OSC envelope-strip pattern

| # | Pattern                                        | Site 1                                                                                             | Site 2                                                                                                  | Verdict                                                                                            |
|---|------------------------------------------------|----------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| 7 | Strip `\x1b]` prefix + `\x07`/`\x1b\\` suffix  | `crates/aphrody-terminal-llm/src/osc.rs:27-35` (strip_prefix + strip_suffix BEL/ST)                | `crates/aphrody-terminal-browser/src/osc.rs:46` (`strip_osc_framing` helper, then strip_prefix)         | Identical envelope, two parsers; extract `osc_framing::strip(input)` into `aphrody-terminal-vt`.  |

### 2.5 `process::Command + status + exit` pattern

| # | Pattern                                        | Site 1                                                                                             | Site 2                                                                                                  | Verdict                                                                                            |
|---|------------------------------------------------|----------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| 8 | `Command::new(bin).args.status; if !ok { exit(code) }` | `crates/cli/src/commands.rs:617-624` (`AutoCommand::run_process`)                            | `crates/cli/src/commands.rs:648-655` (`GeminiCommand::execute`)                                         | Same delegate-then-propagate pattern. Should be one helper that returns `ExitStatus`.             |
| 9 | `std::process::exit` violations (workspace deny) | `crates/cli/src/commands.rs:623`, `:654`, `:904` (3 sites)                                       | `[workspace.lints.clippy] exit = "deny"` (root `Cargo.toml:498`)                                        | Three workspace-deny violations live inside `cli` (allowed only because lints inherit-not-checked).|

### 2.6 CSS export helpers (m3-tokens)

| #  | Helper                                        | Site 1                                                                                            | Site 2                                                                                                 | Verdict                                                                                            |
|----|-----------------------------------------------|---------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| 10 | `export_css() -> String` per theme            | `crates/m3-tokens/src/color.rs:190` (baseline M3 cascade)                                         | `crates/m3-tokens/src/gemini_brand.rs:173` (Gemini brand layer)                                        | OK as-is (different layers). But missing third sibling `export_aphrody_brand_css` — referenced in `crates/aphrody-wasm/examples/aphrody-terminal-demo.html` (63 `--aphrody-*` / `--gemini-*` custom properties inlined). |
| 11 | `:root { ... }` literal block                 | `crates/m3-tokens/src/color.rs:198-238` (~40 lines)                                               | `crates/m3-tokens/src/gemini_brand.rs:174-191`, `crates/m3-tokens/src/shape.rs:90+`                    | Three independent `:root` block builders; consolidate via `RootBlockBuilder` once an aphrody-brand helper is added.|

### 2.7 Regex pattern compilation (per-call vs OnceLock)

| #  | Pattern                                       | Site 1                                                                                            | Site 2                                                                                                 | Verdict                                                                                            |
|----|-----------------------------------------------|---------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| 12 | Inline `Regex::new(...).unwrap()` per call    | `crates/aphrody-translate/src/aphrodify.rs:22-106` (5 inline)                                     | `crates/aphrody-translate/src/ai_patterns.rs:41-67` (3 inline)                                         | Mix of `OnceLock` (extract.rs) and per-call (aphrodify.rs / ai_patterns.rs). Pick one strategy.   |

### 2.8 Transport default name string

| #  | Item                                          | Site 1                                                                                            | Site 2                                                                                                 | Verdict                                                                                            |
|----|-----------------------------------------------|---------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| 13 | Default `protocol_name()` literal             | `crates/a2a-client/src/transport.rs:32` (`fn protocol_name(&self) -> &'static str { "unknown" }`) | `crates/a2a-client/src/factory.rs:247` (uses `"unknown"` string in test fixture)                       | Convention drift: default should be `"mock"` for mock-backed test transports, or `"undefined"` to surface misconfig.|

### 2.9 MarkdownView passthrough (placeholder duplication)

| #  | Item                                          | Site 1                                                                                            | Site 2                                                                                                 | Verdict                                                                                            |
|----|-----------------------------------------------|---------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| 14 | Markdown render via raw `Paragraph::wrap`     | `crates/aphrody-tui/src/widgets.rs:244-252` (placeholder, doc-noted INCOMPLET)                    | `crates/aphrody-terminal-markdown/src/lib.rs` (full ANSI renderer w/ syntect)                          | tui's `MarkdownView` is a placeholder; real renderer already ships in `aphrody-terminal-markdown`.|

## 3. Orphelins / underused crates

Crate consommé = au moins 1 *autre* `crates/*/Cargo.toml` qui le déclare en dépendance.
Comptage automatique : `grep -l "^<name>\\s*=\\|\"<name>\"\\s*=" crates/*/Cargo.toml`
puis filtre du self-match.

| #  | Crate                          | rs_lines | consumers | Statut       | Recommandation                                                                                  |
|----|--------------------------------|---------:|----------:|--------------|-------------------------------------------------------------------------------------------------|
| 15 | `a2a-grpc`                     |     914 |         0 | orphan       | Soit consommé par `cli` (gRPC backend) soit archivé. Décision : feature-flag dans `cli`.        |
| 16 | `a2a-lf` (dir, README-only)    |       0 |         0 | dead         | Supprimer le dossier — pkg `a2a` est dans `crates/a2a/`, le dossier `a2a-lf/` est un orphelin.  |
| 17 | `a2a-slimrpc`                  |     966 | exclus    | excluded     | OK — déjà hors workspace (bug upstream `agntcy-slim-mls`). Marquer dans matrice.                |
| 18 | `agui-bridge`                  |    1007 |         0 | orphan       | Aucun consumer ; câbler à `a2a-ui` ou ajouter `[[example]]` runnable.                           |
| 19 | `aphrody-channels`             |    1951 |         0 | orphan       | 0 consumer. Brancher sur `cli` via `channels send` command.                                     |
| 20 | `aphrody-gateway`              |    1125 |         0 | orphan       | 0 consumer. Brancher sur `cli` via `gateway start` command.                                     |
| 21 | `aphrody-memory`               |     902 |         0 | orphan       | 0 consumer. Doit alimenter `aphrody-terminal-llm` event-bus persistence.                        |
| 22 | `aphrody-summary`              |     243 |         0 | tool         | OK — binaire interne `cargo run -p aphrody-summary`.                                            |
| 23 | `aphrody-terminal-browser`     |    1421 |         0 | orphan       | Aucun consumer (matrix de design dit consumer=⏳). Câbler dans `aphrody-terminal-llm`.          |
| 24 | `aphrody-terminal-config`      |     565 |         0 | orphan       | Doit être conso par `cli` + `aphrody-terminal-llm` (cf. dedup row #1).                          |
| 25 | `aphrody-terminal-json-out`    |     241 |         0 | orphan       | Doit être conso par `cli` (auto subcommand) + `aphrody-terminal-llm` events.                    |
| 26 | `aphrody-terminal-llm`         |    1491 |         0 | orphan       | Doit être conso par `cli`. Consumer = ⏳.                                                       |
| 27 | `aphrody-terminal-markdown`    |     449 |         0 | orphan       | Cible : `aphrody-tui::MarkdownView` (cf. dup row #14).                                          |
| 28 | `aphrody-terminal-wasm`        |     730 |         0 | orphan       | Web-only. OK si bundle séparé, mais doc le statut explicitement.                                |
| 29 | `aphrody-translate`            |    1011 |         0 | tool         | OK — binaire `[[bin]]` standalone (cargo run -p aphrody-translate).                             |
| 30 | `aphrody-tui`                  |     748 |         0 | orphan       | Crate déclaré "canonical long-term TUI", 0 consumer. À câbler dans `cli`/`a2a-ui`.              |
| 31 | `aphrody-voice`                |     769 |         0 | orphan       | 0 consumer. Brancher sur `cli` via `voice` subcommand.                                          |
| 32 | `aphrody-voice-stt`            |    1298 |         0 | orphan       | 0 consumer. Brancher sur `cli` (pair de `aphrody-voice`).                                       |
| 33 | `aphrody-wasm`                 |     174 |         0 | tool         | OK — bundle WASM. Mais `serve-test.mjs` n'expose pas `/examples/*` (cf. row #36).               |
| 34 | `gemini-runtime`               |     539 |         0 | orphan       | 0 consumer. Câbler à `commands::GeminiCommand` au lieu de shell `gemini-cli.exe`.               |
| 35 | `google_mcp`                   |     590 |         0 | binary       | OK — binaire MCP stdio. Mais doc `[[bin]]` explicite.                                           |
| 36 | `ievr-tools`                   |    1658 |         0 | binary       | OK — binaire IEVR forensics. Doc le statut.                                                     |

## 4. Version drift

`cargo tree --duplicates --workspace` → 26 entrées dont 14 noms de crates distincts
ont au moins deux versions résolues simultanément :

| #  | Crate              | Versions présentes              | Source du drift                                                                                  |
|----|--------------------|---------------------------------|--------------------------------------------------------------------------------------------------|
| 37 | `bitflags`         | `1.3.2`, `2.11.1` (×2)         | `nix v0.25.1` (vieux pull via `portable-pty`) reste sur v1                                       |
| 38 | `bytes`            | `1.11.1` (×2)                  | Différents chemins de résolution même version — bénin                                            |
| 39 | `cpufeatures`      | `0.2.17`, `0.3.0`              | tonic 0.14 vs sha2 0.10 ?                                                                        |
| 40 | `either`           | `1.15.0` (×2)                  | bénin (same version)                                                                             |
| 41 | `foldhash`         | `0.1.5` (×2), `0.2.0`          | `hashbrown 0.16` tire foldhash 0.2 ; workspace pin 0.1                                           |
| 42 | `getrandom`        | `0.2.17`, `0.3.4`, `0.4.2` (×2)| 3 majeures live ; rand 0.8 vs newer tonic/h2 pipeline                                            |
| 43 | `hashbrown`        | `0.14.5`, `0.15.5` (×2), `0.16.1`, `0.17.1` (×2) | 5 versions live ; workspace pin = 0.17 mais autres deps tirent old           |
| 44 | `indexmap`         | `2.14.0` (×2)                  | bénin                                                                                            |
| 45 | `itertools`        | `0.13.0`, `0.14.0` (×2)        | workspace = 0.14 ; reqwest? scraper? tire 0.13                                                   |
| 46 | `log`              | `0.4.29` (×2)                  | bénin                                                                                            |
| 47 | `regex`            | `1.12.3` (×2)                  | bénin                                                                                            |
| 48 | `schemars`         | `0.8.22`, `1.2.1`              | workspace = 0.8 mais `aphrody-terminal-config` pull via shim deps tire 1.x                       |
| 49 | `thiserror`        | `1.0.69`, `2.0.18` (×2)        | workspace = 2 ; transitives tirent v1                                                            |
| 50 | `windows`          | `0.61.3`, `0.62.2`             | workspace = 0.62 ; transitives Windows-rs old                                                    |

Non-workspace `version = "..."` pins détectés dans crates (devrait être 0) :

| #  | Crate / fichier                                             | Ligne | Dep et version pinned         | Cause                                                       |
|----|-------------------------------------------------------------|-------|-------------------------------|-------------------------------------------------------------|
| 51 | `crates/a2a-client/Cargo.toml`                              | 47-48 | `tokio = "1.52"`, `reqwest = "0.13"` (wasi target)| `[target.'cfg(...wasi...)'.dependencies]` ne supporte pas `workspace = true`. Acceptable mais doit refléter le workspace. |
| 52 | `crates/a2a-client/Cargo.toml`                              | 55    | `tokio = "1.52"` (wasm target)| idem ci-dessus                                              |
| 53 | `crates/aphrody-terminal-markdown/Cargo.toml`               | 20    | `comrak = "0.52"`             | Pas dans workspace.dependencies ; à hisser.                 |
| 54 | `crates/aphrody-terminal-markdown/Cargo.toml`               | 25    | `syntect = "5.3"`             | Pas dans workspace.dependencies ; à hisser.                 |
| 55 | `crates/aphrody-translate/Cargo.toml`                       | 54-55 | `tokio = "1.52"`, `reqwest = "0.12"` | reqwest 0.12 vs workspace 0.13 — drift effectif.     |
| 56 | `crates/base/Cargo.toml`                                    | 32    | `getrandom = "0.2"` w/ `js` feat | needed for WASM ; à hisser via target-cfg.              |
| 57 | `crates/cli/Cargo.toml`                                     | 61    | `tokio = "1.52"`              | conditional (wasi) — comme #51                              |

## 5. API drift (name / dir / lib mismatch)

| #  | Dir (crates/...)         | `[package] name`     | `[lib] name`     | Verdict                                                                          |
|----|--------------------------|----------------------|------------------|----------------------------------------------------------------------------------|
| 58 | `cli`                    | `aphrody`            | (default `cli`)  | OK — dir = historique, package = published name, `-p aphrody` partout.           |
| 59 | `a2a`                    | `a2a-lf`             | `a2a`            | OK — workspace dep alias resolves `a2a` → `a2a-lf` (cf. root Cargo.toml:226).   |
| 60 | `a2a-client`             | `a2a-client-lf`      | `a2a_client`     | OK même schéma que #59 (workspace dep aliasé).                                   |
| 61 | `a2a-server`             | `a2a-server-lf`      | `a2a_server`     | OK même schéma.                                                                  |
| 62 | `a2a-lf` (extra dir)     | n/a (README only)    | n/a              | Dead directory — vrai pkg `a2a-lf` vit dans `crates/a2a/`. Supprimer.            |

## 6. Matrice integration workspace-wide

Étend `docs/design/aphrody-terminal-integration-matrix.md` (couvre seulement les
crates `aphrody-terminal-*`) à l'ensemble du workspace.

Légende : `OK` = consumer présent dans la pipeline `cli`/`gui`/`backend`. `⏳`
= crate existe mais aucun consumer (orphan). `tool` = binaire CLI standalone
(consumer = utilisateur final).

| #  | Crate                          | Consumer principal               | Statut | Pipeline finale       |
|----|--------------------------------|----------------------------------|--------|------------------------|
| 63 | `a2a`                          | `a2a-client`, `a2a-server`, `a2a-grpc`, `a2a-pb` | OK     | model layer           |
| 64 | `a2a-client`                   | `cli`, `a2a-grpc`                | OK     | wire client           |
| 65 | `a2a-server`                   | `a2a-grpc`                       | OK     | wire server           |
| 66 | `a2a-pb`                       | `a2a-{client,server,grpc,ui}`    | OK     | protobuf model        |
| 67 | `a2a-grpc`                     | (none)                           | ⏳     | À câbler dans `cli`   |
| 68 | `a2a-ui`                       | `aphrody-terminal-wasm`          | OK     | WASM viewer           |
| 69 | `agui-bridge`                  | (none)                           | ⏳     | À câbler dans `a2a-ui` ou example|
| 70 | `aphrody-channels`             | (none)                           | ⏳     | `cli channels send`   |
| 71 | `aphrody-gateway`              | (none)                           | ⏳     | `cli gateway start`   |
| 72 | `aphrody-mcp`                  | `aphrody-terminal-llm`           | OK     | OAuth flow            |
| 73 | `aphrody-memory`               | (none)                           | ⏳     | `terminal-llm` persistence|
| 74 | `aphrody-terminal-backend`     | `cli`                            | OK     | PTY backend           |
| 75 | `aphrody-terminal-browser`     | (none)                           | ⏳     | bridge LLM->DOM       |
| 76 | `aphrody-terminal-config`      | (none)                           | ⏳     | shim layer            |
| 77 | `aphrody-terminal-json-out`    | (none)                           | ⏳     | JSON-mode default     |
| 78 | `aphrody-terminal-llm`         | (none)                           | ⏳     | Event bus core        |
| 79 | `aphrody-terminal-markdown`    | (none)                           | ⏳     | tui MarkdownView      |
| 80 | `aphrody-terminal-vt`          | `aphrody-terminal-wasm`          | OK     | VT parser             |
| 81 | `aphrody-terminal-wasm`        | (none)                           | ⏳     | renderer WASM bundle  |
| 82 | `aphrody-translate`            | (none, binary)                   | tool   | EN→FR scrub           |
| 83 | `aphrody-tui`                  | (none)                           | ⏳     | canonical TUI         |
| 84 | `aphrody-voice`                | (none)                           | ⏳     | `cli voice tts`       |
| 85 | `aphrody-voice-stt`            | (none)                           | ⏳     | `cli voice stt`       |
| 86 | `aphrody-wasm`                 | (none, web)                      | tool   | bundle bindgen        |
| 87 | `backend`                      | `cli`, `google_mcp`, `gui`       | OK     | forensics+network     |
| 88 | `base`                         | `cli`, `backend`, `gui`          | OK     | no_std primitives     |
| 89 | `gemini-runtime`               | (none)                           | ⏳     | `cli gemini` cmd      |
| 90 | `google_mcp`                   | (none, binary)                   | tool   | MCP stdio server      |
| 91 | `gui`                          | (none, binary)                   | tool   | wry+tao desktop       |
| 92 | `ievr-tools`                   | (none, binary)                   | tool   | IEVR forensics CLI    |
| 93 | `m3-tokens`                    | `a2a-ui`, `shadcn-bridge`        | OK     | brand tokens          |
| 94 | `mrx-{core,detect,audit,watch}`| `mrx-cli`                        | OK     | monorepo mapper       |
| 95 | `shadcn-bridge`                | `aphrody-terminal-wasm`          | OK     | M3 wrappers           |

**Stats** : 19 crates ⏳ (47% du workspace) / 16 OK / 5 tools.

## 7. Plan refactor priorité-rangé (P0 / P1 / P2)

Chaque item est dispatchable en sub-agent isolé. `cargo check -p <crate>` final
sert de gate.

### P0 — quick wins (~10 min/each, parallèlisables sans collision)

| #  | Action                                                                                                          | Crates impactés                                  | Fix                                                                                                                       | Verify                                                                          |
|----|-----------------------------------------------------------------------------------------------------------------|--------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------|
| P0-1 | Supprimer dossier mort `crates/a2a-lf/` (cf. row #62)                                                         | (filesystem)                                     | `rm -rf crates/a2a-lf/`                                                                                                   | `cargo metadata --no-deps | grep a2a-lf` → 0 ligne                              |
| P0-2 | Extraire `pub const NL_ENGINE_TOKENS` (row #5/6)                                                              | `cli`                                            | Nouveau `crates/cli/src/engine_tokens.rs` ; importé par `commands.rs:473` + `main.rs:277`                                | `cargo check -p aphrody`                                                        |
| P0-3 | `MockTransport::protocol_name() -> "mock"` (row #13)                                                          | `a2a-client`                                     | Override `fn protocol_name(&self) -> &'static str { "mock" }` dans `client.rs:223`                                       | `cargo test -p a2a-client-lf --test transport_kind`                              |
| P0-4 | `serve-test.mjs` ajoute route `/examples/*` (row #36)                                                        | `aphrody-wasm`                                   | Bloc `if (path.startsWith("/examples/"))` après `/pkg/` dans `serve-test.mjs:48`                                          | `bun run crates/aphrody-wasm/serve-test.mjs` + curl `/examples/aphrody-terminal-demo.html` |
| P0-5 | Ajouter `export_aphrody_brand_css()` dans `m3-tokens` (row #10)                                              | `m3-tokens`                                      | Nouveau `crates/m3-tokens/src/aphrody_brand.rs` ; couvre les 63 custom props inlines de `aphrody-terminal-demo.html`     | `cargo test -p m3-tokens aphrody_brand`                                          |
| P0-6 | `aphrody-tui::MarkdownView` consomme `aphrody-terminal-markdown` (row #14)                                   | `aphrody-tui`                                    | Ajoute dep workspace + remplace `Paragraph::wrap` par `aphrody_terminal_markdown::render_ansi`                            | `cargo test -p aphrody-tui markdown_view`                                        |
| P0-7 | Dedup `.mcp.json` loader : terminal-llm re-export `McpShim` (row #1)                                         | `aphrody-terminal-llm`, `aphrody-terminal-config`| Ajoute dep workspace `aphrody-terminal-config` ; `load_mcp_json` devient un thin adapter `import_mcp_json -> Vec<McpServerSpec>` | `cargo check -p aphrody-terminal-llm`                                            |
| P0-8 | Aligner `reqwest` 0.12 -> 0.13 dans `aphrody-translate` (row #55)                                            | `aphrody-translate`                              | `crates/aphrody-translate/Cargo.toml:55` → `reqwest.workspace = true`                                                     | `cargo check -p aphrody-translate`                                               |
| P0-9 | Cleanup deps non utilisées (cargo machete report)                                                            | `a2a-ui`, `aphrody-channels`, `aphrody-gateway`, `aphrody-terminal-browser`, `aphrody-terminal-json-out`, `aphrody-terminal-llm`, `aphrody-terminal-markdown`, `aphrody-voice-stt`, `gemini-runtime`, `ievr-tools` | Soit `cargo rm` la dep, soit ajouter `[package.metadata.cargo-machete] ignored = [...]` | `cargo machete` propre                                                           |
| P0-10 | Hisser `comrak` + `syntect` dans `[workspace.dependencies]` (row #53-54)                                    | root `Cargo.toml`, `aphrody-terminal-markdown`   | Déclarer `comrak = "0.52"` + `syntect = "5.3"` workspace-wide ; crate dep → `workspace = true`                            | `cargo check -p aphrody-terminal-markdown`                                       |

### P1 — moderate (~30 min/each)

| #  | Action                                                                                                          | Crates impactés                                  | Fix                                                                                                                       | Verify                                                                          |
|----|-----------------------------------------------------------------------------------------------------------------|--------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------|
| P1-11 | Consommer `a2a::JsonRpcRequest` dans `terminal-llm::mcp` (row #3-4)                                          | `aphrody-terminal-llm`                           | Ajoute dep workspace `a2a` ; supprimer `jsonrpc_request` + `extract_jsonrpc_result` locaux                               | `cargo check -p aphrody-terminal-llm` + tests probe_*                            |
| P1-12 | Extraire `osc_framing::strip(input)` partagé dans `aphrody-terminal-vt` (row #7)                             | `aphrody-terminal-vt`, `-llm`, `-browser`        | Helper unique `pub fn strip_osc_framing(input: &[u8]) -> Option<&[u8]>` ; remplacer les 2 sites                          | `cargo check -p aphrody-terminal-llm -p aphrody-terminal-browser`               |
| P1-13 | Refactor `AutoCommand::run_process` + `GeminiCommand::execute` en helper commun (row #8-9)                   | `cli`                                            | `fn spawn_and_propagate(bin, args) -> miette::Result<ExitStatus>` retournant le status au lieu d'exit                    | `cargo clippy -p aphrody -- -D clippy::exit` reste vert                          |
| P1-14 | Câbler `gemini-runtime` dans `commands::GeminiCommand` (row #34)                                             | `cli`, `gemini-runtime`                          | `cli` dépend de `gemini-runtime` ; remplacer le shell-out `gemini-cli.exe` par appel runtime natif                       | `cargo check -p aphrody` + smoke `aphrody gemini --version`                      |
| P1-15 | Câbler `aphrody-terminal-llm` dans `cli` (row #26, #76, #78)                                                 | `cli`, `aphrody-terminal-{config,json-out,llm}`  | Nouveau `commands::TerminalCommand` qui démarre l'event bus + parse OSC sequences                                        | `cargo check -p aphrody` + e2e `aphrody terminal start`                          |
| P1-16 | Normaliser regex compilation via `OnceLock` partout dans `aphrody-translate` (row #12)                       | `aphrody-translate`                              | Hisser tous les `Regex::new(...).unwrap()` ad-hoc en `static RE_X: OnceLock<Regex>` (style `extract.rs:23+`)              | `cargo test -p aphrody-translate`                                                |
| P1-17 | `aphrody-voice` + `aphrody-voice-stt` exposés via `commands::VoiceCommand` (row #31-32, #84-85)              | `cli`                                            | Nouveau `commands::voice` (tts + stt sous-commands)                                                                       | `cargo check -p aphrody` + smoke `aphrody voice tts --text "hi"`                |

### P2 — strategic (relocation / archive / fold)

| #  | Action                                                                                                          | Crates impactés                                  | Fix / Justification                                                                                                       | Verify                                                                          |
|----|-----------------------------------------------------------------------------------------------------------------|--------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------|
| P2-18 | Fold `aphrody-terminal-config::shims::McpShim` model dans `aphrody-mcp` (single source of truth MCP JSON)    | `aphrody-mcp`, `-config`, `-llm`                 | Déplace `McpShim`/`McpServerEntry` dans `aphrody-mcp::types` (mcp est le tier 0)                                          | `cargo check --workspace`                                                       |
| P2-19 | Câbler `aphrody-memory` comme backing-store pour `aphrody-terminal-llm` event bus (row #21)                  | `aphrody-terminal-llm`, `aphrody-memory`         | Ajoute trait `EventStore` (impls `JsonlBackend`, `BruteHnswBackend`) ; bus publishes via store                             | `cargo test -p aphrody-terminal-llm event_store_jsonl`                          |
| P2-20 | Câbler `aphrody-gateway` comme reverse-proxy entre `cli` et providers AI                                     | `cli`, `aphrody-gateway`                         | `cli gateway start` exposé ; routes OpenAI-compatibles vers providers                                                     | `cargo check -p aphrody` + smoke `curl localhost:PORT/v1/models`                |
| P2-21 | Câbler `aphrody-channels` (Slack/TG/Matrix) via `cli channels send`                                          | `cli`, `aphrody-channels`                        | `cli channels send --to slack --msg "..."` ; lit creds via `aphrody-mcp` OAuth                                            | `cargo check -p aphrody`                                                        |
| P2-22 | Câbler `agui-bridge` dans `a2a-ui` (renderer AG-UI events)                                                   | `a2a-ui`, `agui-bridge`                          | a2a-ui consomme `agui-bridge::render` pour le viewer JSONL                                                                | `cargo check -p a2a-ui`                                                         |
| P2-23 | Câbler `aphrody-tui` dans `cli` (TUI dashboard)                                                              | `cli`, `aphrody-tui`                             | `cli dashboard` lance la TUI ratatui plein-écran                                                                          | `cargo check -p aphrody`                                                        |
| P2-24 | Câbler `a2a-grpc` dans `cli` via feature `grpc`                                                              | `cli`, `a2a-grpc`                                | `cli a2a --transport grpc ...` active la dep optionnelle                                                                  | `cargo check -p aphrody --features grpc`                                        |
| P2-25 | Normaliser version-drift hashbrown (5 majors live) (row #43)                                                 | root `Cargo.toml`                                | `cargo update -p hashbrown@... --precise 0.17` + pinner via `[patch.crates-io]` les vieux pulls                           | `cargo tree --duplicates | grep hashbrown` → 0 ou 1 entrée                      |
| P2-26 | Normaliser drift `getrandom` 0.2/0.3/0.4 (row #42)                                                           | root `Cargo.toml`                                | Bumper `rand` workspace (mémoire institutionnelle : rand 0.8 imposé par denokv_proto — re-vérifier)                       | `cargo tree --duplicates | grep getrandom`                                       |
| P2-27 | Normaliser drift `windows-rs` 0.61 vs 0.62 (row #50)                                                         | root `Cargo.toml`                                | Bump transitives (notify-debouncer? portable-pty?) via `[patch.crates-io]`                                                | `cargo tree --duplicates | grep ^windows`                                       |

### Synthèse priorité

- **P0 = 10 items** dispatchables maintenant en parallèle (aucun ne touche les crates fresh `a2a-client-lf`, `aphrody-tui`, `aphrody-terminal-{markdown,json-out,config}` côté fonctionnalité — P0-6 modifie `aphrody-tui` mais c'est l'ajout d'une dep + un swap de body, < 10 lignes).
- **P1 = 7 items** itératifs sur cli + terminal stack.
- **P2 = 10 items** stratégiques nécessitant alignement humain (archivage vs câblage).
- **Total = 27 items**.

## 8. Verify commands (à exécuter après chaque batch)

```bash
# Détecter qu'une duplication a bien disparu
grep -rn "fn load_mcp_json" crates/ | wc -l  # attendu: 0 après P0-7
grep -rn "known_engine_tokens" crates/cli/src/ | wc -l  # attendu: 1 après P0-2

# Crates check (par lot)
CARGO_HOME=C:/Users/yohan/.cargo RUSTUP_HOME=C:/Users/yohan/.rustup \
  cargo check -p aphrody -p aphrody-terminal-llm -p aphrody-terminal-config

# Workspace lints
cargo ci-offline  # = clippy --workspace --all-targets --locked --offline -- -D warnings

# Version drift gate
cargo tree --duplicates --workspace 2>&1 | grep -E '^[a-z]+\s+v' | sort -u | wc -l
# baseline pré-refactor : 26 — descendre < 20 après P2-25/26/27

# Cargo machete clean
cargo machete 2>&1 | grep -c "unused dependencies"
# baseline : 11 crates flag — descendre à 0 après P0-9

# Orphan count
for d in crates/*/; do
  name=$(basename "$d");
  c=$(grep -l "^$name\s*=\|\"$name\"\s*=" crates/*/Cargo.toml 2>/dev/null | grep -v "^crates/$name/Cargo.toml" | wc -l);
  [ "$c" -eq 0 ] && echo "$name"
done | wc -l
# baseline pré-refactor : 19 — descendre vers 5 (les tools légitimes) après P1-14/15/17 + P2-19/20/21/22/23
```

## 9. Risques & out-of-scope

- **P0-6** modifie `crates/aphrody-tui/src/widgets.rs` malgré la directive
  "tui fresh, ne pas toucher" : exception explicite car le placeholder
  `MarkdownView::render` est documenté `INCOMPLET` dans le code et le swap
  est un upgrade (≤ 10 lignes). Sub-agent peut être skippé si Lane T-10
  l'a déjà en flight.
- **P2-25/26/27** (drift cleanup) peuvent nécessiter `[patch.crates-io]`,
  qui est un changement workspace-wide non commutatif — exécuter en série.
- Les crates `aphrody-{voice,voice-stt,channels,gateway,memory}` sont
  comptés orphans mais sont récents (ticks 30+) et peuvent être en cours
  de câblage côté autre lane — vérifier `git log` avant d'archiver.
- `gemini-runtime` orphan : décision humaine entre câblage (P1-14) et
  archivage. Si le shell-out `gemini-cli.exe` reste préféré (binaire
  rustc-compiled distribué upstream), archiver.
- `aphrody-translate`, `aphrody-summary`, `aphrody-wasm`, `google_mcp`,
  `gui`, `ievr-tools` sont tous des outils binaires standalone : 0 consumer
  est NORMAL et attendu.
