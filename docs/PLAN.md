<!-- SPDX-License-Identifier: Apache-2.0 -->
# PLAN — aphrody

> Plan d'exécution stratégique. Révision : **2026-05-19 (refresh Apex Autonomous Agent — 5 piliers)**.
> Voir [`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md) pour le contexte d'ensemble.
> Audit comparatif amont : [`audits/2026-05-19-hermes-agent-vs-aphrody.md`](audits/2026-05-19-hermes-agent-vs-aphrody.md).
> Plan dédié soul/identity/workspace : [`plans/agent-home.md`](plans/agent-home.md) (crate `aphrody-agent-home`, 2026-05-23).
>
> **MISE À JOUR (2026-05-21) — plusieurs items et statuts ci-dessous sont
> PÉRIMÉS depuis le nettoyage du workspace :**
> - **`bxc-engine` / les 11 `n2b-*` ont été SUPPRIMÉS** : les lignes pilier
>   « R4 scraping bas niveau / bxc-engine » et « R-A n2b » ne décrivent plus
>   du code in-tree. Le web-fetch/recon passe par le serveur `aphrody-mcp`
>   (`universal_web_fetch`, `advanced_recon`, `dns_recon`) ; `aphrody n2b`
>   est une façade qui spawne un binaire externe optionnel.
> - **`cargo xtask` a été supprimé** : remplacer toute action `aphrody xtask …`
>   / `cargo xtask …` par `scripts/deploy.{ps1,sh}` ou des commandes cargo
>   directes.
> - **A2A file-based supprimé** : `ai.json`, `.well-known/ai.json`,
>   `schemas/ai.json/v1.json`, dossier `ai/` n'existent plus ; transport gRPC
>   (`a2a-*`) + miroir winclean uniquement.
> - **Crates fusionnées** : `mrx-*`→`mrx`, `aphrody-{cache,cost,rateguard,retry}`
>   →`aphrody-llm-infra`, `aphrody-channels`→`aphrody-messaging`,
>   `aphrody-{hooks,permissions,skills-runtime}`→`aphrody-skills`,
>   `aphrody-design-{daemon,sidecar}`→`aphrody-design`,
>   `aphrody-voice-stt`→`aphrody-voice` (les chemins `crates/mrx-*/…` cités
>   pointent vers l'ancien découpage).
> - Worktree par défaut : `~/worktree` via `APHRODY_WORKTREE` (plus de
>   `C:/worktree`).

---

## ⭐⭐ Cap actuel (2026-08-21) : toolbox d'inférence et de modèles locaux

**Le cap du projet a changé le 2026-08-21.** `aphrody` devient un **client et
une toolbox pour l'inférence et la gestion de modèles locaux**, orientés usages
programmatiques : **OCR**, **transcription visuelle**, **transcription audio**,
et **exécution de tâches répétées organisées en tâches de fond**.

➡️ **Plan détaillé, état d'avancement et prochaines étapes :
[`plans/local-inference-toolbox.md`](plans/local-inference-toolbox.md)**

État au 2026-08-21 : **étape 1 livrée** — crate `aphrody-models` (catalogue de
11 modèles épinglés, téléchargement repris et vérifié par SHA-256, inspection
GGUF/GGML/safetensors/ONNX, sonde GPU/CUDA, store LRU) + commande
`aphrody model` (12 sous-commandes, sorties texte/JSON/Markdown/HTML/CSV).

Le cap « Apex Autonomous Agent » ci-dessous **reste valide comme historique et
comme état des briques agent**, mais n'est plus la priorité #1 : ces briques
deviennent des consommatrices de la toolbox locale.

---

## ⭐ Cap 2026-05-19 → 2026-08-21 : Apex Autonomous Agent (historique)

**Mission** : faire de `aphrody` le **meilleur agent autonome** en exploitant 5 piliers asymétriques que les concurrents Python (hermes-agent v0.14.0, AutoGPT, OpenInterpreter, …) ne peuvent pas rattraper sans réécriture native.

### État réel au 2026-05-19 (audit post-rédaction PLAN)

Le PLAN initial supposait Sprints R-B/R-C/R-E à scaffolder. **Audit code** révèle que
~70 % du scope est **déjà shippé** dans le workspace, hors PLAN :

| Sprint | Item | État réel | Lignes |
|---|---|---|---|
| R-A | R3.1 Trait `MemoryProvider` dyn-compat | ✅ shipped (`provider.rs`) | 61 |
| R-B | `aphrody-messaging::channels` Discord+Slack+Telegram+Matrix+X (5 backends, unified from ex-`aphrody-channels`) | ✅ shipped (consolidated) | 3176 |
| R-B | `aphrody-voice` (TTS + STT unified: elevenlabs + whisper local + whisper API + web + discord_shim, ex-`aphrody-voice` + `aphrody-voice-stt`) | ✅ shipped (consolidated) | 2048 |
| R-B | R3.2 Honcho v1 API | ✅ shipped (`honcho.rs`) | 298 |
| R-B | R3.3 Mem0 v1 API | ✅ shipped (`mem0.rs`) | 273 |
| R-C | `aphrody-skills::runtime` forge (lib + lint + registry + scaffold + skill, ex-`aphrody-skills-forge`) | ✅ shipped | 903 |
| R-E | `aphrody-cron` (single lib.rs) | ✅ shipped | 536 |
| -- | **Workspace consolidation** (2026-05-19) : 22 crates merged into 6 domain crates (-16 net). See `consolidation_plan.md`. | ✅ shipped | n/a |
| R-A | R1.1/R1.2/R1.3 diagnostics tui+gemini-runtime | ✅ **false-positives cached rust-analyzer** (sources OK) | n/a |
| R-E | `aphrody-re` (NEW — reverse engineering) | ✅ **shippé** — R5.1-R5.7 DONE (triage/disasm/strings/sections lib + CLI + 4 MCP tools) | ≈18 tests |
| R-A | R1.4 MCP client dans `aphrody-mcp` | ✅ shipped — tool `aphrody_mcp_call(server, tool, args, config?)` wired sur McpClient stdio/HTTP, 5 unit tests | n/a |
| R-A | R1.5 cold-start bench criterion | ✅ shipped (`benches/cold_start.rs` + `benches/initialize_handshake.rs` + ledger §4.2/§4.3) | 179 |
| R-B | R3.4-R3.7 (migration tool, eviction, schema versioning, recall bench) | ✅ **R3.4-R3.6 DONE** ; R3.7 bench shippé (p95≈172ms brute-force ; <100ms requiert instant-distance HNSW) | n/a |
| R-D | R4 scraping deep (concurrent, proxy pool, chrome146, HTTP/3) | 🚫 OBSOLÈTE — bxc-engine supprimé du workspace (audit 2026-05-23) | n/a |

**Statut ⏳ (audit 2026-05-23)** : R3.7 ✅ clos (p95 18.8ms, commit 16e6d7d91) ; R3.8 ✅ clos (providers v3 Honcho/mem0, commit 75f7cc1da) ; R4.1-R4.7 🚫 obsolètes (bxc-engine supprimé) ; R5.8 🚧 bloqué (r2pipe hors cache offline + binaire r2) ; R5.9 ✅ clos 2026-05-23 (yara-x 1.16, `aphrody re yara`, 5/5 + smoke réel). R1.4/R1.5 ✅ clos. R3.4 ✅ clos 2026-05-22.

Les sprints R-B/R-C/R-E sont donc **largement déjà clos** — leur valeur résiduelle est
en **finition** (tests, docs, wire CLI) plutôt qu'en scaffolding.

### Les 5 piliers

| # | Pilier | Promesse mesurable | Anti-cible |
|---|---|---|---|
| **R1** | **Tools Rust natif ultra-rapide** | Cold-start `aphrody <tool>` < 5 ms ; tool MCP roundtrip < 20 ms p50 (vs ~300 ms hermes Python) | Pas de runtime interprété (Python/Node banned per memory `feedback_aphrody_rust_only`) |
| **R2** | **Apprend de ses erreurs** | Chaque échec (exit ≠ 0, tool err, retry) → `aphrody-memory` event indexé + skill candidate auto-extraite après N=3 répétitions | Pas de prompt-injection skill creation (gate humain ou flag `--auto-skills`) |
| **R3** | **Mémoire persistante** | `aphrody-memory` LanceDB + SQLite + 3 providers externes (Honcho, Mem0, lancedb) ; recall cross-session p95 < 100 ms ; pas de perte sur upgrade | Pas de cloud-locked memory (provider local = défaut) |
| **R4** | **Scraping bas niveau** | bxc-engine + curl-impersonate Chrome 131 + Lightpanda CDP — DOM-only < 200 ms p50 ; bypass Cloudflare/Akamai/PerimeterX sans fingerprint custom ; concurrency native via tokio | Pas de Playwright/Puppeteer fallback (sauf cible JS-only explicite) |
| **R5** | **Reverse engineering** | Bindings natifs `goblin` (ELF/PE/Mach-O/WASM parsing) + `iced-x86` (x64 disasm) + `radare2` FFI + `iaito` Qt frontend driver + `ghidra-headless` orchestration ; PE/ELF triage < 1 s | Pas de réimplémentation Ghidra/IDA (orchestration only) |

### Implications structurelles

- **Tout nouveau crate** doit servir ≥1 des 5 piliers, sinon refusé (cf. memory [[feedback_no_scaffold]]).
- **Tout nouveau MCP tool** dans `aphrody-mcp` doit être annoté `#[tool(pillar = "R1|R2|R3|R4|R5")]` dans la description (convention doc).
- **Tout commit feat:** doit citer le pilier dans le footer (`Pillar: R4`).
- Les 5 piliers sont **non-négociables** — pas d'ajout de pilier R6+ sans révision PR + memory update.

### Roadmap sprints (semaines 21 → 26, deadline 2026-08-31)

| Sprint | Semaine | Focus pilier | Livrables clés |
|---|---|---|---|
| **R-A** | S21 (2026-05-19 → 25) | R1 + R3 (fondations) | Fix diagnostics actuels (`aphrody-tui/widgets.rs`, `gemini-runtime/tools.rs`), trait `MemoryProvider` workspace-wide, MCP client dans `aphrody-mcp` (rmcp client) |
| **R-B** | S22 (2026-05-26 → 06-01) | R3 (memory providers) | Adapter `honcho` + `mem0` + tests cross-provider, eviction policies, schema migration tool |
| **R-C** | S23 (2026-06-02 → 08) | R2 (learn-from-errors) | `crates/aphrody-skills-forge/` (NEW), hook PostSessionEnd, extraction patterns répétés, dry-run + diff + auto-merge gated |
| **R-D** | S24 (2026-06-09 → 15) | R4 (scraping deep) | `bxc-engine` upgrade Chrome 132+ impersonate, residential proxy pool integration (BrightData/IPRoyal stubs), `aphrody scrape --concurrent N --rate-limit ms` |
| **R-E** | S25 (2026-06-16 → 22) | R5 (reverse) | `crates/aphrody-re/` (NEW) — goblin + iced-x86 + capstone bindings + `aphrody re {triage,disasm,strings,sections}` + 4 tools MCP |
| **R-F** | S26 (2026-06-23 → 29) | R1 + R2 (polish) | Bench harness `criterion` p50/p95 par pilier, regression gate CI, dashboard live `aphrody dashboard` (axum + SSE) |
| **R-G** | S27 → S35 | R4 + R5 (deepen) | Plugins reverse (`yara-x` matching, `unicorn-rs` emulation), scraping (HTTP/3 ja3 spoofing via `quiche`), Ollama local backend pour skills-forge sans cloud |

### Détails par pilier — items ⏳ actionables sans humain

#### R1 — Tools Rust natif ultra-rapide

**Statut Sprint R-A (2026-05-19)** : R1.1-R1.3 ✅ **closed** — les diagnostics
rust-analyzer initiaux étaient des **cached false-positives** (cf. memory
`feedback_refresh_rust_lsp`). Sources vérifiées : `widgets.rs:21` importe juste
ce qu'il utilise, `widgets_smoke.rs:13` consomme `aphrody_tui::{TestBackend, m3::*}`
correctement (module `m3` exposé `lib.rs:58`, `TestBackend` re-exporté `lib.rs:330`),
`tools.rs:35` importe `async_trait` (Cargo.toml:28 a `async-trait = { workspace = true }`)
et le trait `#[async_trait] pub trait Tool` est dyn-compat (preuve : `Arc<dyn Tool>`
utilisé ligne 228). Restent R1.4-R1.6 actionables.

| # | Tâche | Statut | Verify |
|---|---|---|---|
| R1.1 | Diagnostics `aphrody-tui/src/widgets.rs` (unused imports `BorderType/Modifier`, `unicode_*`) | ✅ **false-positive cached** | `cargo check -p aphrody-tui --locked` exit 0 |
| R1.2 | Diagnostics `aphrody-tui/tests/widgets_smoke.rs` (E0432 unresolved imports `BorderStyle/Gauge/Padding/...`) | ✅ **false-positive cached** — `m3::*` + `TestBackend` exposés | `cargo test -p aphrody-tui --locked` exit 0 |
| R1.3 | Diagnostics `gemini-runtime/src/tools.rs` (E0432 `async_trait`, E0038 `Tool` non dyn-compat) | ✅ **false-positive cached** — `async-trait` dans Cargo.toml + `#[async_trait]` proc-macro applied | `cargo check -p gemini-runtime --locked` exit 0 |
| R1.4 | MCP client dans `aphrody-mcp` via `gemini_runtime::McpClient` (stdio + HTTP, cf. `crates/gemini-runtime/src/mcp_client.rs:275`). Tool `aphrody_mcp_call(server, tool, args, config?)` shipped dans `crates/google_mcp/src/main.rs` — structured error envelopes `MCP_BAD_REQUEST / MCP_CONFIG / MCP_UNKNOWN_SERVER / MCP_CONNECT / MCP_INITIALIZE / MCP_CALL / MCP_ENCODE` | ✅ | `cargo test -p google_mcp aphrody_mcp_call_tests` 5/5 green |
| R1.5 | Bench `criterion` cold-start : `aphrody version` p50/p95/p99 (`crates/cli/benches/cold_start.rs`), `aphrody-mcp` initialize handshake p50/p95 (`crates/google_mcp/benches/initialize_handshake.rs`). Both use `iter_custom` over freshly-built `CARGO_BIN_EXE_*` for true wall-clock cold-start latency. Ledger `docs/PERFORMANCE-HISTORY.md` expanded with §4.2 (cold-start) + §4.3 (MCP handshake) ; first measurement captured at v0.1.0 tag | ✅ **DONE** 2026-05-19 | `cargo check --benches -p aphrody -p google_mcp --locked` exit 0 |
| R1.6 | Wire `aphrody-voice` (unified TTS + STT via `aphrody_voice::stt`) into 2 new MCP tools `voice_synthesize(text, voice)` + `voice_transcribe(audio_bytes)` (whisper.cpp natif) | ✅ **DONE** | `voice_synthesize`+`voice_transcribe` à google_mcp/src/main.rs:895,912 ; 8/8 voice tests verts |

#### R2 — Apprend de ses erreurs (self-improvement loop)

| # | Tâche | Verify |
|---|---|---|
| R2.1 | `crates/aphrody-skills-forge/` NEW : fusion `skill` runtime existant + pattern extraction `aphrody-memory` queries + format SKILL.md aphrody | `cargo new -p aphrody-skills-forge` + module `extractor.rs` + `candidate.rs` + tests |
| R2.2 | Schema `ErrorEvent` dans `aphrody-memory` : `{ts, cmd, exit_code, stderr_head, retry_n, context_hash}` indexé via LanceDB embedding | `cargo test -p aphrody-memory test_error_event_recall` |
| R2.3 | Hook `PostToolUse` (.claude/plugins/aphrody/hooks/hooks.json) qui appelle `aphrody skills forge --from-stderr` quand exit ≠ 0 ET retry_n ≥ 3 | manual : trigger 3 fails de la même cmd, voir skill candidate générée |
| R2.4 | CLI `aphrody skills {forge, refine, review, list, delete}` | `aphrody skills list` → JSON array |
| R2.5 | Auto-merge gate : `--auto-skills` flag ou prompt humain par défaut ; jamais d'écriture skill sans validation explicite (gate sécurité) | `aphrody skills forge --auto-skills` write, sinon prompt |
| R2.6 | Sync agentskills.io catalog one-way → `aphrody xtask skills-sync agentskills.io` | catalog JSON cached + diff vs in-tree |

#### R3 — Mémoire persistante

**Statut Sprint R-B (2026-05-19)** : ✅ **shipped** dans commit `175d61f63` —
4 providers production-ready (`HonchoProvider`, `Mem0Provider`, `SqliteLocalProvider`
côté tier-1 + `LanceDbBackend`, `SqliteBackend`, `HnswBackend`, `JsonlBackend` côté
key-value/vector tier-2). Trait `MemoryProvider` dyn-compatible avec preuve compile-time
dans `provider.rs:60`. Les items R3.1-R3.3 sont donc **clos** sur API v1
(production stable des 2 SaaS) — l'upgrade v3 est rejeté en backlog R3.8.

| # | Tâche | Statut | Verify |
|---|---|---|---|
| R3.1 | ✅ Trait `MemoryProvider` (`crates/aphrody-memory/src/provider.rs`) : `add/get/search/delete/health/provider_kind` ; dyn-compat compile-time check `dyn_compat_check(Box<dyn MemoryProvider>)` | **DONE** | `cargo doc -p aphrody-memory --no-deps` — trait public |
| R3.2 | ✅ Adapter `honcho` API **v1** (`/v1/apps/{app}/users/{user}/sessions/{sess}/messages`) dans `crates/aphrody-memory/src/honcho.rs` (298 l.) | **DONE** | `cargo test -p aphrody-memory honcho::` |
| R3.3 | ✅ Adapter `mem0` API **v1** sync (`POST/GET/DELETE /v1/memories/`, `GET /v1/memories/search`) auth `Authorization: Token <key>` dans `crates/aphrody-memory/src/mem0.rs` (273 l.) | **DONE** | `cargo test -p aphrody-memory mem0::` |
| R3.4 | Migration tool `aphrody memory migrate --from lancedb --to honcho` | ✅ **DONE** | `crates/aphrody-memory/src/migrate.rs` (impl prod) + wire CLI `MemoryAction::Migrate` a `crates/cli/src/main.rs:958-969` ; `--dry-run` + JSON diff via `MigrationDiff` |
| R3.5 | Eviction policies : TTL, LRU, max-size (config via `~/.aphrody/memory.json`) | ✅ **DONE** | `cargo test policy_ttl_evicts_after_n_secs` 3/3 pass (eviction.rs, 14 test fns) |
| R3.6 | Schema versioning : `MemoryEvent v1 → v2` migration sans perte | ✅ **DONE** | `cargo test schema_v1_to_v2_roundtrip` pass (schema.rs : `upgrade_jsonl()` atomic, lossless) |
| R3.7 | Recall benchmark : 100k events, query top-10 semantic, p95 < 100 ms | ✅ **DONE** (commit 16e6d7d91) | `HnswBackend::brute_force_search` optimisé (norme query hoistée + dot/norm fusionnés + quickselect top-k) ; p95 **18.8 ms** < 100 ms (~9× le brute-force initial de 172 ms), vérifié réplique `rustc -O` 100k×128 |
| R3.8 | upgrade Honcho v1→v3 (`api.honcho.dev` workspaces/peers + `POST /peer/{id}/chat`) + mem0 v1→v3 (`POST /v3/memories/add/` async + `event_id` polling) | ✅ **DONE** (commit 75f7cc1da) | providers additifs `HonchoV3Provider`/`Mem0V3Provider` (v1 intact, même `ProviderKind`) ; vérifié réseau-free via mocks axum `tests/v3_smoke.rs` (5 tests) + 9 unit tests ; suite verte |

#### R4 — Scraping bas niveau

> **✅ LIVRÉ DANS LE DÉPÔT PYTHON** (`aphrody-py` à `C:\src\aphrody-py`) :
> Le scraping statique et l'analyse de code du client Gemini ont été entièrement implémentés en Python via un crawler asynchrone Scrapy avec isolation de processus (Twisted multiprocessing) et fallback HTTPX (`aphrody web scrape`). Un module d'auto-upgrade autonome (`aphrody web auto-upgrade`) utilisant Gemini Vertex LLM (avec fallback programmatique) a été livré pour mettre à jour automatiquement les hashes d'action Boq en production de manière robuste et stable.

| # | Tâche | Verify |
|---|---|---|
| R4.1 | bxc-engine : pin `curl-impersonate` profil **`chrome146`** (stable courant lexiforest/curl-impersonate fork, macOS Tahoe) + **`chrome145`** pour spoofing **HTTP/3** (premier profil avec fingerprint HTTP/3 ; chrome131 prédate cette feature) + monitoring JA4 drift via `mitmproxy` (audit trimestriel quand Chrome stable release) | `bxc-engine fetch --impersonate chrome146 https://tls.peet.ws` → JA4 hash stable |
| R4.2 | `aphrody scrape --concurrent N --rate-limit-ms K` flags (currently single URL) | `aphrody scrape --concurrent 10 --rate-limit-ms 500 urls.txt` |
| R4.3 | Residential proxy pool trait `ProxyProvider` + stubs BrightData/IPRoyal/Soax (lecture seule, pas de creds par défaut) | trait public + 3 stub providers compilent |
| R4.4 | **HTTP/3 transport** via `tokio-quiche` (Cloudflare, OSS 2025-12, wrap `quiche >= "0.24"` MSRV 1.82 dans event loop tokio — API : `tokio_quiche::quic::connect(socket, host)` → `(QuicConnection, ClientH3Controller)`, `controller.request_sender()` pour `NewClientRequest`). **Important : tokio-quiche NE fait PAS de JA4 spoofing** — il utilise la TLS BoringSSL par défaut de quiche. Le spoofing fingerprint est délégué à `curl-impersonate` (R4.1, profil `chrome145` HTTP/3). Vérifier latest `quiche` à integration (peut avoir progressé vers 0.28+ depuis 2026-05) | document research outcome dans `docs/research/http3-transport.md` |
| R4.5 | Tool MCP `bxc_batch_scrape(urls: Vec<String>, selector: String, concurrent: u32)` | `aphrody-mcp` exposes + smoke 10 urls |
| R4.6 | Bench bxc vs Playwright vs raw curl sur 100 URLs Cloudflare-protected | `docs/PERFORMANCE.md` updated + numbers |
| R4.7 | Anti-detect : random User-Agent rotation depuis `bxc/profiles/*.json` (already exists) wired dans `aphrody scrape` | `aphrody scrape --random-profile https://...` |

#### R5 — Reverse engineering

**Statut Sprint R-E Phase 1 (2026-05-19)** : ✅ **lib shipped** — crate
`aphrody-re` (≈530 L incl. tests) ship `pub fn triage(bytes) -> TriageReport`
qui détecte PE32/PE64/ELF32/ELF64 via magic + goblin parsing, extrait sections
avec Shannon entropy + SHA-256 fingerprint + sample ASCII/UTF-16LE strings.
10/10 tests pass (`cargo nextest run -p aphrody-re` < 30 ms). Pas de GPL
(unicorn-engine R5.10 dropped pour cause licence virale). CLI wire + MCP
tools = Phase 2.

| # | Tâche | Statut | Verify |
|---|---|---|---|
| R5.1 | `crates/aphrody-re/` lib (Cargo.toml + lib.rs unique avec modules inline `triage / entropy / strings`) | ✅ **DONE** | `cargo check -p aphrody-re --locked` exit 0 |
| R5.2 | Deps `goblin = "0.10"` (PE/ELF, déjà workspace pin avec features pe32+pe64+elf32+elf64) + `sha2` + `hex` + `serde` + `thiserror`. `iced-x86`/`capstone` reportés Phase 2 | ✅ **DONE** (subset minimal viable) | `cargo tree -p aphrody-re` |
| R5.3 | Lib API `triage(bytes) -> TriageReport { format, size, sha256, arch, entry_point, sections[], imports[], exports[], strings_sample[] }`. Sub-cmd CLI `aphrody re triage <binary>` | ✅ **DONE** (lib + CLI wire) | `cargo test -p aphrody-re` 10/10 pass ; `ReAction::Triage` dispatch à cli/src/main.rs:637 |
| R5.4 | Sub-cmd `aphrody re disasm` via `iced-x86 1.21` (`Decoder::with_ip(64, bytes, rip, NONE)` + `IntelFormatter`) | ✅ **DONE** | `aphrody_re::disasm` (lib.rs) + `ReAction::Disasm` (cli/main.rs) ; 6/6 tests ; smoke `aphrody re disasm` exit 0 |
| R5.5 | Lib API `extract_strings(bytes, min_len, limit)` ASCII + UTF-16LE avec dedup + cap configurable (constantes `STRINGS_MIN_LEN=6`, `STRINGS_SAMPLE_LIMIT=64`). Sub-cmd CLI `aphrody re strings` shippé | ✅ **DONE** (lib + CLI wire) | `cargo test extract_strings_*` 3/3 pass ; `ReAction::Strings` à cli/src/main.rs:540 |
| R5.6 | Lib retourne `sections[].entropy: Option<f64>` (Shannon entropy bits/byte). Sub-cmd CLI `aphrody re sections` shippé | ✅ **DONE** (lib + CLI wire) | `cargo test shannon_entropy_*` 2/2 pass ; `ReAction::Sections` à cli/src/main.rs:562 |
| R5.7 | MCP tools mirror dans `aphrody-mcp` : `re_triage`, `re_disasm`, `re_strings`, `re_sections` | ✅ **DONE** (4/4 : `re_triage`/`re_disasm`/`re_strings`/`re_sections` tous shippés) | `re_disasm` à google_mcp/src/main.rs:1464-1485 ; `re_triage`/`re_strings`/`re_sections` à :1015,1086 ; build `aphrody-mcp` exit 0 |
| R5.8 | `radare2` FFI binding optionnel (feature `radare2`) via `r2pipe` crate (API Rust non surfacée context7 — smoke test crates.io requis avant pin) | 🚧 BLOQUÉ — `r2pipe` absent du cache offline (repo lockfile-only) + binaire `r2` externe requis | `cargo build -p aphrody-re --features radare2` |
| R5.9 | `yara-x 1.11+` (BSD-3, VirusTotal) integration : `aphrody re yara --rules rules.yara <binary>` | ✅ **DONE** 2026-05-23 — `yara-x 1.16` (`default-features=false`, feature `inventory`), module `crates/aphrody-re/src/yara.rs` (`scan()` → `YaraReport` JSON : rule_name/namespace/tags/matched_strings) feature-gated `yara` ; CLI `ReAction::Yara` (spawn_blocking, erreur rebuild explicite si feature absente) à cli/main.rs | `cargo test -p aphrody-re --features yara yara::` 5/5 verts ; smoke réel : `aphrody re yara --rules pe.yar aphrody.exe` matche `pe_mz_header` (offset 0, `4d5a`) |
| R5.10 | ~~`unicorn-engine` (CPU emulator)~~ | 🚨 **DROPPED — GPL-2.0 viral incompatible avec Apache-2.0** | n/a |

### Anti-portée (drop explicite)

- **Image generation** (FAL/Stable Diffusion) — hors scope CLI ultra-rapide.
- **Modal/Daytona/Vercel terminals SaaS** — couplage cloud, drop.
- **Android Termux** — niche, post-1.0 si demande.
- **Matrix E2EE** (python-olm pain) — drop sauf demande contributeur avec PR.
- **WhatsApp/Signal/Telegram bots multi-cloud** — limité à Telegram tier-1 + Discord (déjà) + Slack tier-2.

### Garde-fous (rouge)

- **Auto-skill creation sans gate** : interdit par défaut. Toujours `--auto-skills` opt-in explicite ou prompt humain.
- **Memory write d'output LLM brut** : interdit (toujours sanitize via `aphrody-memory::redactor::scrub` avant write).
- **Reverse engineering de binaires non-autorisés** : `aphrody re` warning + `--accept-tos` flag obligatoire au premier run (audit local-only, jamais d'upload).
- **Scraping respectant robots.txt** : flag `--respect-robots` par défaut TRUE ; bypass nécessite `--ignore-robots` explicite + log warning.
- **`aphrody antigravity refresh` cassé** : Google retourne ``400 client_secret is missing`` pour le refresh grant du client OAuth public Antigravity. Ne pas tenter de corriger cote SDK sans disposer du secret. Workaround : relancer l'outil ``agy`` en arriere-plan pour re-minter le token. Re-auth interactive : ``aphrody antigravity login``.

### Métriques de succès (gate v2.0.0)

| Pilier | Métrique | Cible v2.0.0 | Mesure actuelle (2026-05-19) |
|---|---|---|---|
| R1 | `aphrody version` cold-start p50 | < 5 ms | non mesuré (à benchmark sprint R-A) |
| R1 | `aphrody-mcp initialize` p50 | < 20 ms | ~590 ms observé sur 1er handshake (warm-up tokio dominant) |
| R2 | Skills auto-forgées / mois actif | ≥ 5 | 0 (feature absente) |
| R3 | `aphrody-memory` recall p95 (100k events) | < 100 ms | non mesuré |
| R3 | Providers externes wired | 3 (lancedb + honcho + mem0) | 3 (HonchoProvider + Mem0Provider + SqliteLocal ✅ R3.2-R3.3 DONE) |
| R4 | bxc DOM-only scrape p50 | < 200 ms | non mesuré (mais bxc `example.com` < 1s end-to-end mesuré 2026-05-19) |
| R4 | Cloudflare bypass success rate | ≥ 95 % | non mesuré (CF identifié sur example.com mais pas testé contre protection active) |
| R5 | `aphrody re triage` p50 sur PE 5MB | < 1 s | ✅ **8.83 ms** (566 MiB/s, dev host Win11 MSVC) — cible tenue ~113× ; `cargo bench -p aphrody-re --bench triage`, ledger §4.4 (2026-05-26) |
| R5 | MCP tools reverse | 4 | 4 (re_triage, re_disasm, re_strings, re_sections — ✅ R5.7 DONE) |

---

---

## 0. Phases livrées avant pivot (2026-05-16)

| Phase | Sortie | Statut |
|---|---|---|
| **P0 — Bring-up workspace** | resolver=3, rust-version=1.97, 80 deps centralisées, lints Google-grade, 5 profils release | ✅ |
| **P1 — Cleanup vendor** | `vendor/crates.io/` supprimé (1.2 Go libérés), `--locked` en CI | ✅ |
| **P2 — Supply-chain signée** | `cargo-vet` (7 feeds Google/Mozilla/Fuchsia/ChromeOS/BCA/Embark/Zcash) + `cargo-deny` | ✅ |
| **P3 — Path-bases (RFC 3529)** | Tentative, révoquée (feature instable Cargo 1.97) | ⏸ |
| **P4 — Validation hermétique** | `cargo ci-offline` + `cargo deny check` verts | ✅ |
| **P5 — Refresh docs + cleanup root** | README + CLAUDE + GEMINI + .gitignore alignés | ✅ |
| **P10 — Cross-platform Google-grade** | binaire `cli` cross-platform, `platform.rs`, aliases multi-target, lints `android-strict` | ✅ |
| **P11 — Alignement Google complet** | google.json sync, Android NDK targets, MUSL targets, CI matrix, `cargo-fuzz` skeleton, `cargo-auditable` alias, Dockerfile distroless | ✅ |
| **P13 — Skills ecosystem** | `skill` crate + binaires + 50+ skills documentés + sync upstream | ✅ |
| **P14 — Desktop GUI app** | Application desktop Tauri v2 + Angular 21.2 + Angular Material 21.2. Backend Rust : ``crates/aphrody-app`` (exclu du workspace core, in-process via ``aphrody::run_captured``). Frontend : ``C:\src\aphrody-ts\apps\desktop``. Positionnement : assistant IA grand public propulse par Gemini. Commit aphrody-ts ``main``. | ✅ |
| **P15 — Chat resilience agy 401/403** | ``classify_agy_error`` dans ``crates/cli/src/agy_backend.rs`` : intercepte ``SdkError::OAuthServer{401|403}`` et retourne un message court actionnable au lieu du JSON brut Google. Commit ``e5a932da2``. | ✅ |

## 1. Phases post-pivot (2026-05-17)

Le pivot abandonne la trajectoire "Google OS hybride Win-NT" et recentre
sur `aphrody`, le CLI ultime cross-platform. **Linux est désormais la
cible #1**.

### Phase P-A — Pivot mécanique (cette PR initiale)

| Tâche | Statut |
|---|---|
| Repo `google-cli` → `aphrody` (rename script ~78 fichiers) | ✅ |
| `crates/google_os` archivé hors du repo (`C:\google-os-archive\`) | ✅ |
| `[workspace.members]` mis à jour (google_os retiré) | ✅ |
| `[workspace.package]` metadata (authors, homepage, repository, keywords) | ✅ |
| `crates/google_mcp` : retrait dep `google_os` | ✅ |
| `packages/material-design-icons/` purgé (4.6 GB, .gitignore + README stub) | ✅ |
| `docs/SOURCE_OF_TRUTH.md` créé (fusion CLAUDE/GEMINI/PLAN/DESIGN) | ✅ |
| `CLAUDE.md`, `README.md`, `docs/PLAN.md` réécrits en UTF-8 propre | ✅ |
| Anonymisation : tracked files (1 leak path, déjà fix) | ✅ |
| Premier commit + push vers `aphrody-code/aphrody` (privé) | 🔧 |
| Rename dossier racine `C:\src\google-cli` → `C:\src\aphrody` | ✅ |

### Phase P-Linux — Validation Linux Ubuntu 26.04 (PRIORITÉ #1)

| Tâche | Statut |
|---|---|
| `cargo check -p cli --target x86_64-unknown-linux-gnu` vert | ✅ (WSL local, 8.28 s, zéro warning) |
| `cargo build --release -p aphrody` natif sur Ubuntu 26.04 | ✅ (job `linux-native` ubuntu-latest commit 96ae82e7 ; cross-compile zigbuild Windows → Linux ✅) |
| `cargo nextest run -p aphrody` vert sur Linux | ✅ (job `linux-native` commit 96ae82e7 — nextest sur workspace `--locked`) |
| Adapter `crates/a2a*` pour Linux (retirer Windows-only) | ✅ (cli chromium gated, voir fix d222d0061) |
| Adapter `crates/google_mcp` pour Linux | ✅ (YOLO #1 — cargo check native + zigbuild x86_64-unknown-linux-gnu exit 0) |
| CI runner `ubuntu-26.04` (ou `ubuntu-latest` en fallback) | ✅ (ubuntu-latest job `linux-native` commit 96ae82e7) |
| Package `.deb` template (`packaging/deb/` — control + postinst + prerm + cargo-deb snippet + README) | ✅ |
| Publication PPA `aphrody-code/aphrody` sur Launchpad | ⏳ |

### Phase P-Win11 — Validation Windows 11 Insider Canary (PRIORITÉ #2)

| Tâche | Statut |
|---|---|
| `cargo build --release -p aphrody` sur Win11 Insider Canary | ✅ (local Win11 28020) |
| `cargo nextest run` workspace vert sur Windows | ✅ (**387/387** en 2,914 s — 2026-05-17) |
| Package `scoop` manifest | ✅ (`packaging/scoop/aphrody.json`) |
| Package `winget` manifest | ✅ (`packaging/winget/manifests/a/aphrody-code/aphrody/__VERSION__/`) |
| Profil Windows Terminal pour `aphrody` | ✅ (`packaging/windows-terminal/aphrody.profile.json`) |

### Phase P-Wasm — WebAssembly lib (PRIORITÉ #3)

Matrice validée 2026-05-17 (host : Windows 11) :

| Crate           | `wasm32-unknown-unknown` | `wasm32-wasip1` |
|-----------------|:------------------------:|:---------------:|
| `base`          | ✅ (getrandom "js" gated)| ✅              |
| `mrx`           | n/a (chrono)             | ✅              |
| `aphrody-translate` | ✅ (wasm stub)       | ✅              |
| `cli` (binary)  | ✅ (stub minimal)        | ✅ (stub minimal)|
| `a2a-client`    | ✅ (JSON-RPC+REST+SSE via browser fetch; async-trait ?Send) | ✅ (traits + types only; reqwest/HTTP modules cfg-stripped — WASI p1 has no sockets) |
| `backend`       | ❌                       | ❌              |

`a2a-client` notes :
- `wasm32-unknown-unknown` : `JsonRpcTransport`, `RestTransport`, `AgentCardResolver` compilent. `reqwest` utilise browser `fetch`. Streaming SSE via `bytes_stream()`. `BoxStream` = `LocalBoxStream` (futures !Send sur wasm). `async-trait` avec `?Send`.
- `wasm32-wasip1` : reqwest 0.13 utilise le stack natif (hyper/mio/socket2) sur `target_os = "wasi"` — les syscalls socket ne sont pas exposés par WASI p1. Modules `jsonrpc`, `rest`, `agent_card`, `factory` cfg-strippés. Traits (`Transport`, `TransportFactory`), types de données, `auth`, `middleware` compilent. Déblocage possible avec WASI p2 + `wasi-http` crate.
- `backend` reste ❌ sur les deux cibles : `tokio::fs`, `fs_extra`, `tracing-subscriber`, `base::Vfs`, et DNS OSINT sont tous des OS-primitives sans équivalent WASM. Port non tractable sans réécriture complète.

Sous-tâches :

| Tâche | Statut |
|---|---|
| `base` : feature `js` getrandom gated wasm32-unknown-unknown | ✅ |
| `base` : compile `wasm32-unknown-unknown` + `wasm32-wasip1` | ✅ |
| `mrx` : compile `wasm32-wasip1` | ✅ |
| `aphrody-translate` : retirer tokio `full` (idéalement tokio-rt minimal) | ✅ |
| `cli` : refactor tokio + cfg-gate commandes OS-bound pour wasm | ✅ (cli/src/main.rs cfg-gates mimalloc + tokio + reqwest + backend + a2a) |
| `crates/aphrody-wasm` : wrapper `base` exposé via `wasm-bindgen` | ✅ |
| `wasm-pack publish` sur npm `@aphrody-code/aphrody-wasm` (89 KB wasm + 12 KB JS, metadata.wasm-pack profile.release SIMD+bulk-memory, README + publish doc) | ✅ scaffolded — `wasm-pack publish --access public` awaits `npm login` |
| `a2a-client` : port wasm32-unknown-unknown (browser fetch transport) | ✅ |
| `a2a-client` : port wasm32-wasip1 (traits/types ; HTTP cfg-strippé) | ✅ |
| `a2a-pb` : gate tonic/proto/pbconv non-wasm ; protojson_conv disponible sur wasm | ✅ |
| `backend` : port wasm (tokio::fs + VFS + DNS = OS-only, pas tractable) | ❌ bloqué |

### Phase P-Wasm-CLI — Port cli binaire vers wasm32

Le cli pull tokio (full features) + reqwest + mimalloc + rustls + ring via
backend/a2a-client. Refactor requis :

| Tâche | Statut |
|---|---|
| `cli/Cargo.toml` : `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` pour mimalloc/backend/a2a-client/reqwest/rustls | ✅ |
| `cli/Cargo.toml` : `[target.'cfg(target_arch = "wasm32")'.dependencies]` avec tokio minimal (sync,macros,io-util,rt,time) | ✅ |
| `cli/src/main.rs` : `#[cfg(not(target_arch = "wasm32"))]` sur les commandes OS-bound | ✅ |
| `cli/src/main.rs` : stub wasm minimal (Version + help) | ✅ |
| `aphrody-translate/Cargo.toml` : tokio minimal pour wasm (translate API HTTP via reqwest wasm) | ✅ |

### Phase P-A2A — Bilateral Claude-to-Claude coordination (2026-05-17)

| Tâche | Statut |
|---|---|
| `ai.json` v1 schema (channel-extension layer) | ✅ (`schemas/ai.json/v1.json`) |
| `ai.json` AGNTCY a2a/v0.4 CollaborationManifest (peer-authored) | ✅ (`ai.json` racine) |
| `.well-known/ai.json` HTTP-discoverable subset | ✅ (`canonical_manifest` field pointe vers racine) |
| Bilateral handshake apx-handshake-1 → wc-ack-1 → apx-ack-of-ack-1 | ✅ (3-deep loop, 6 canaux : file_jsonl, http, markdown, heartbeat, process_inspect, git_tag) |
| Live HTTP listener (`/ping`, `/msg`, `/inbox`, `/ai.json`) sur `:8788` | ✅ (`C:\winclean\.coord\listener.ts`) |
| Asks queue + facts journal en JSONL | ✅ (`C:\winclean\.coord\inbox-*.jsonl`) |
| Peer mirror `C:\winclean\ai.json` publié | ✅ (winclean Claude — 16:00Z 2026-05-17) |
| Technical post D+14 (per `/start` skill arc) — 2076 mots sur ai.json | ✅ (`docs/posts/2026-05-ai-json.md` — picked up by mdBook SUMMARY.md regen) |
| GitHub Pages workflow (`docs.yml`) triggers main branch + deploys mdBook | ✅ (fix branch trigger `master` → `main` 2026-05-17) |
| CI hardening — machete fail-fast + binary smoke (--version/--help) + vet fail-fast | ✅ (commit 0372fe57c) |

### Phase P-Distribution

| Tâche | Statut |
|---|---|
| crates.io publication (`aphrody`) | ⏳ |
| Homebrew formula template (`packaging/homebrew/aphrody.rb`) | ✅ |
| Homebrew tap `aphrody-code/tap` publié | ⏳ |
| GitHub Releases workflow (`.github/workflows/release.yml`, 8 targets, SHA-256, SBOM) | ✅ |
| Premier tag `v*` poussé qui produit un release | ⏳ |
| Package workspace `cli` → `aphrody` | ✅ (commit 2026-05-17, dir conservé `crates/cli/`) |
| `cargo install aphrody` documenté | ✅ (README + `docs/launch/SHOW-HN.md` mentionnent la commande ; publication crates.io toujours pendante cf. ligne 134) |
| One-line install (`install.sh` / `install.ps1`) avec vérif SHA-256 | ✅ |
| Cleanup `cargo machete` — 11 dead deps retirées (cli/google_mcp/gui/mrx-{audit,core,detect}) | ✅ |
| `aphrody --version` runtime bug — rustls 0.23 CryptoProvider install au boot | ✅ (commit pending) |

## 2. Tâches de fond (continues)

### A2A native integration

- [ ] CLI comme agent autonome : prompts NL interceptés par `AutoCommand`,
  routés vers le moteur natif `a2a` avec streaming zero-buffering.
- [ ] Cross-platform : trait `Transport` portable (HTTP/2 Linux, IOCP Win, fetch wasm).

### Supply-chain real audits

- [ ] `cargo vet suggest` → liste des audits manquants.
- [ ] Pour chaque crate critique (crypto, net, ffi), audit `safe-to-deploy`.
- [ ] Publier nos audits sur `aphrody-code/rust-crate-audits` pour partage.

### Stabilité / Release 1.0.0-LTS

- [ ] Pression `cargo clippy` + `miri` sur tous les `unsafe`.
- [ ] Audit FFI bloc par bloc (`python_ffi` — `bun_ffi` archivé hors workspace).
- [ ] Stress tests : `cargo bench` sur benchmarks critiques.
- [ ] Cross-compile validation : 3 cibles prioritaires + macOS + Android.
- [ ] Tag `v1.0.0-LTS`.

### Upstream alignment

- [🔒] **a2a-slimrpc** : ré-inclure quand `agntcy-slim-mls` fixe lifetime/async-trait (bloqué upstream).
- [🔒] **path-bases (RFC 3529)** : activer workspace-wide quand stable Cargo 1.98+ (bloqué par release Cargo).
- [🔒] **wry → GTK4** : retirer les CVE GTK3 (RUSTSEC-2024-04xx). (Bloqué upstream : wry 0.55 ne supporte pas encore nativement GTK4 / webkit6-rs).
- [x] **reqwest 0.13** : drop `aws-lc-sys` propre (retirer 4 CVE ignorés).
- [x] **pyo3 0.22** : fix CVE PyString (RUSTSEC-2025-0020).

## 3. Règles de collaboration inter-AI

Les IAs (Gemini, Claude, OpenCode) opèrent avec consigne commune :
**la qualité prime sur la vitesse**.

Toute tâche trop vaste pour une passe est divisée en sous-tâches **chacune
100% exécutable + testée**.

Avant tout push : `cargo ci-offline && cargo deny check` doit être vert
**sur Linux d'abord**.

### Phase Q — Mission D+7 → D+15 polish (génération auto tick 7)

Mission 100k stars / 30 jours — actions mission-direct entre D+7 (demo) et
D+15 (Show HN), tous techniquement actionables sans autorisation utilisateur.

| Tâche | Statut |
|---|---|
| Re-rendre asciinema cast `assets/aphrody-demo.cast` avec `aphrody doctor` (+ couleurs) | ✅ (`assets/aphrody-doctor-demo.cast` 111 l. — companion cast doctor-focused, demo cast préservé) |
| Fresh clippy `-D warnings` audit + fix résidu Oracle gate 2 `double_ended_iterator_last` | ✅ (`cargo clippy --workspace --all-targets --locked --offline -- -D warnings` exit 0, no warnings) |
| Deuxième post technique `docs/posts/2026-05-yolo-grind-loop.md` (architecture 4-agent parallel grind) | ✅ (297 l., D+14 milestone arc, complète post a2a-ai-json) |
| Show HN launch package `docs/launch/SHOW-HN.md` (title + body + comment templates draft) | ✅ (112 l. draft — title + body + comment templates) |
| `n2b` aggressive scan + auto-migrate `scripts/**/*.{ts,mjs,js}` node→bun | ✅ (`docs/audits/2026-05-17-n2b-scan.md`, 6 findings, 2 auto-migrated) |
| `mrx` aggressive `scan/audit/detect` du workspace → rapport `docs/audits/2026-05-17-mrx-aggressive.md` | ✅ (rapport shippé, 5 crates mrx-* couvertes) |
| `bxc` scrape via A2A ask peer winclean → AGNTCY spec page + M3 tokens, mirror aphrody side | ✅ (`docs/audits/2026-05-17-bxc-scrape-request.md` — envelope `apx-ask-bxc-scrape-1` shipped via http_jsonrpc + file_jsonl) |
| `aphrody completions {bash,zsh,fish,pwsh,elvish}` subcommand via `clap_complete` | ✅ (`crates/cli/src/main.rs:140` `Commands::Completions { shell }` via `clap_complete::Shell`) |
| Integration test `crates/cli/tests/doctor.rs` — `assert_cmd` driven smoke of `aphrody doctor` + `--json` | ✅ (150 l. assert_cmd-driven smoke + --json) |
| Supply-chain `audits.toml` + `config.toml` formatting drift fix (Oracle gate 5 partial) | ✅ (cargo vet fmt appliqué sans effet de bord) |
| Marketing `docs/COMPARISON.md` — aphrody vs just/taskfile/gh/devcontainer/asdf | ✅ (94 l. — vs just/taskfile/gh/devcontainer/asdf) |
| ADRs `docs/adr/{0001-cross-platform-rust,0002-a2a-file-based,0003-yolo-parallel-grind}.md` | ✅ (86 + 94 + 95 l., template 0000 inclus) |
| WASM browser playground `crates/aphrody-wasm/examples/browser-playground.html` + crate README upgrade | ✅ (584 l. HTML playground + 62 l. crate README) |
| `.devcontainer/devcontainer.json` one-click Codespace setup (rust nightly + bun + cargo extras) | ✅ (60 l. devcontainer manifest) |
| `SECURITY.md` verify + strengthen — concrete report process, supported versions, scope | ✅ (81 l. — report process + supported versions + scope) |
| `packaging/snap/snapcraft.yaml` + `packaging/arch/PKGBUILD` — Ubuntu Snap + Arch AUR distribution expansion | ✅ (50 l. + 65 l. — Snap manifest + Arch PKGBUILD) |
| `crates/backend/benches/backend_bench.rs` — criterion benchmark suite | ✅ (182 l. criterion benchmark suite) |
| `docs/FAQ.md` + `docs/ROADMAP.md` — anticipated Q + public 90-day roadmap | ✅ (101 l. + 68 l.) |
| `crates/mrx/README.md` — usage doc with scan/detect/audit/watch examples | ✅ (129 l. — usage doc, scan/check/watch examples) |
| `crates/aphrody-translate/README.md` — translate CLI usage doc | ✅ (121 l. — CLI usage doc) |
| `.github/workflows/codeql.yml` — GitHub CodeQL security scanning | ✅ (70 l. CodeQL workflow) |
| `docs/extensions/` — a2a extension specs (file-transport, honest-delivery, context7-version-pinning) | ✅ (74 + 82 + 78 l. + index.md — 3 extension specs publishable) |
| `crates/base/README.md` — base crate docs (publish-ready) | ✅ (127 l. — publish-ready) |
| `crates/a2a-pb/README.md` + `crates/backend/README.md` — publish-ladder docs | ✅ (95 l. + 135 l.) |
| `crates/a2a-{client,server,grpc}/README.md` — 3 publish-ladder docs | ✅ (76 + 79 + 80 l.) |
| `docs/ARCHITECTURE.md` — workspace overview + ASCII module dep graph | ✅ (155 l. — workspace overview + dep graph) |
| `mrx-core/src/scan.rs` `workspace_key()` Windows path-sep bug fix per #30 audit | ✅ (fix lives in `crates/mrx-audit/src/lib.rs:362` — `replace('\\', "/")` + tests `workspace_key_normalises_windows_paths`) |
| `.github/workflows/release-please.yml` — Google release automation via Conventional Commits | ✅ (28 l. — release-please workflow scaffolded) |
| `crates/google_mcp/README.md` + `crates/a2a/README.md` + `crates/a2a-lf/README.md` | ✅ (85 + 80 + 90 l.) |
| `crates/mrx-{core,detect,audit,watch}/README.md` — 4 mrx lib crate docs | ✅ (75 + 77 + 85 + 81 l.) |
| Root `README.md` polish — sweep new docs into doc-tree TOC (COMPARISON/FAQ/ROADMAP/ADRs/extensions) | ✅ (README §Documentation now references COMPARISON/FAQ/ROADMAP/ADRs/extensions/SHOW-HN/posts/IEVR docs/WASM playground) |
| `.github/ISSUE_TEMPLATE/*` + `PULL_REQUEST_TEMPLATE.md` + `FUNDING.yml` — OSS template bundle | ✅ (bug_report + feature_request + question + config + PR template + FUNDING.yml) |
| `CHANGELOG.md` sweep — log 2026-05-17 mega-batch (30+ commits today) | ✅ (251 l. — Keep-a-Changelog updated with 2026-05-17 batch) |
| `packaging/nix/flake.nix` — Nix flake for Nix/NixOS users | ✅ (102 l. flake + README) |
| `packaging/flatpak/com.aphrody.aphrody.json` — Flatpak manifest | ✅ (36 l. Flatpak manifest + README + LICENSE.SPDX) |
| `.github/dependabot.yml` — monthly cargo + GH actions dep updates | ✅ (68 l. — cargo + actions update schedule) |
| `packaging/{scoop,winget,homebrew,deb}/*` version+arch sweep | ✅ (scoop 64bit+arm64, winget x64+arm64, homebrew on_macos+on_linux arm/intel, deb amd64+arm64 — all `1.0.0-canary` aligned to workspace.package.version, asset pattern `aphrody-v1.0.0-canary-<triple>.{zip,tar.gz}`) |
| `docs/PROTOCOL.md` — definitive a2a/v0.4 protocol description for impl reference | ✅ (190 l. — normative a2a/v0.4 protocol doc) |
| `docs/SECURITY-MODEL.md` — threat model + trust boundaries for the A2A protocol | ✅ (139 l. — threat model + trust boundaries) |
| `assets/aphrody-logo.svg` — vector logo for README badges + favicons | ✅ (`assets/aphrody-logo.svg` + `aphrody-mark.svg` SVG vectors) |
| `docs/MIGRATION.md` — from just/taskfile/gh to aphrody (adoption path) | ✅ (157 l. — adoption path from just/taskfile/gh) |
| `scripts/install.{sh,ps1}` audit + curl-bash one-liner doc | ✅ (`packaging/install.sh` 121 l. + `packaging/install.ps1` 108 l. + `packaging/INSTALL-ONELINER.md`) |
| `docs/cargo/SECURITY-DEEP.md` — extended supply-chain doc | ✅ (200 l. — extended supply-chain doc) |
| `docs/PERFORMANCE.md` — bench claims with reproducible recipes | ✅ (185 l. — reproducible bench recipes) |
| `packaging/chocolatey/aphrody.nuspec` + install.ps1 | ✅ (`aphrody.nuspec` 27 l. + `tools/chocolateyinstall.ps1` 20 l. + README) |
| `packaging/aur-bin/PKGBUILD` — pre-built binary AUR variant | ✅ (57 l. binary AUR PKGBUILD + README) |
| `rust-toolchain.toml` audit + explicit nightly pin with date | ✅ (24 l. — channel `nightly-2026-05-17`, 6 targets, hermetic) |
| `docs/COMMUNITY.md` — community guidelines + future Discord/Matrix invite | ✅ (109 l. — community guidelines) |
| `docs/PRIVACY.md` — telemetry-zero policy | ✅ (103 l. — telemetry-zero policy) |
| `docs/cargo/PUBLISH-LADDER.md` — explicit topological publish order doc | ✅ (126 l. — topological publish order runbook) |
| `.github/workflows/security.yml` — gitleaks + trufflehog secret scan | ✅ (98 l. — gitleaks + trufflehog secret scan workflow) |
| `AGENTS.md` — agent-facing onboarding for AI assistants working in repo | ✅ (144 l. — agent-facing onboarding) |
| `scripts/dev-setup.{sh,cmd}` — single-script environment bootstrap | ✅ (`dev-setup.sh` 130 l. + `dev-setup.cmd` 124 l.) |
| `.github/workflows/bench.yml` — criterion CI gate with summary comment | ✅ (75 l. — criterion CI gate) |
| `CONTRIBUTORS.md` — auto-generated contributor recognition stub | ✅ (49 l. — contributor recognition + AGENTS.md cross-link) |
| `.github/DISCUSSION_TEMPLATE/*.yml` — Discussion templates (Q+A, ideas, show-and-tell) | ✅ (qna.yml + ideas.yml + show-and-tell.yml) |
| `scripts/release.sh` — operational release helper (tag + push + release-please trigger) | ✅ (239 l. — operational release helper) |
| `crates/aphrody-wasm/tests/wasm_smoke.rs` — wasm-bindgen integration test | ✅ (67 l. — wasm-bindgen integration test) |
| `docs/POST-LAUNCH.md` — Show HN +24h/+72h/+7d engagement protocol | ✅ (107 l. — Show HN engagement protocol) |
| `docs/EXAMPLES.md` — recipe collection (curl bash, doctor outputs, A2A samples) | ✅ (249 l. — recipe collection) |
| `docs/posts/2026-05-cross-platform-rust.md` — 3rd technical post on cross-platform Rust | ✅ (405 l. — 3rd technical post on cross-platform Rust) |
| `scripts/verify-publish.sh` — pre-publish dry-run gate sweeper | ✅ (248 l. — pre-publish dry-run gate sweeper) |
| `crates/cli/examples/doctor_consumer.rs` — example consuming doctor --json output | ✅ (62 l. — example consuming doctor --json) |
| `docs/CI-CD.md` — overview of all 10+ GitHub Actions workflows + roles | ✅ (164 l. — overview of 10 GitHub Actions workflows) |
| `crates/aphrody-wasm/src/lib.rs` — `encrypt_aes_gcm` companion to existing decrypt | ✅ (`crates/aphrody-wasm/src/lib.rs:95` `pub fn encrypt_aes_gcm` + round-trip test) |
| `scripts/changelog-since.sh` — Conventional Commits since last tag (release prep) | ✅ (184 l. — Conventional Commits since last tag) |
| `docs/RELEASE-CHECKLIST.md` — per-release maintainer checklist before tag | ✅ (108 l. — per-release maintainer checklist) |
| `docs/INDEX.md` — master index of all 60+ docs (auto-generated by script ideally) | ✅ (94 l. — master docs index) |
| `packaging/rpm/aphrody.spec` — Fedora/RHEL RPM spec | ✅ (91 l. — RPM spec + README) |
| `packaging/desktop/aphrody.desktop` — XDG .desktop file for Linux DEs | ✅ (26 l. .desktop entry + README) |
| `docs/SUMMARY.md` regenerated via `cargo run -p aphrody-summary` | ✅ (531 l. — mdBook SUMMARY regenerated) |
| `assets/aphrody-social-preview.svg` — GitHub OG card 1280×640 SVG | ✅ (`assets/aphrody-social-preview.svg` SVG card shipped) |
| `docs/cargo/PROFILES.md` — Cargo workspace profile reference | ✅ (110 l. — workspace profile reference) |
| `scripts/sbom-extract.sh` — extract auditable SBOM from built binary | ✅ (319 l. — SBOM extraction helper) |
| `docs/PERFORMANCE-HISTORY.md` — bench ledger / regression tracking | ✅ (91 l. — bench ledger / regression tracking) |

### Phase T — Terminal LLM-first WASM+M3 (génération tick post-PLAN-MOONSHOT)

**Pivot 2026-05-17 soir** : aphrody-terminal n'est PAS un wterm/Windows-Terminal
clone. C'est le **terminal LLM-first** — conçu spécifiquement pour sub-agents,
skills, hooks, MCP servers, Ink/React TUIs (Claude Code + Gemini CLI), avec
**JSON output partout**, **markdown rendu inline**, **config JSON full**.
Spec normative : `docs/design/aphrody-terminal-spec.md` (supprimé 2026-05-21 ; voir [`UI-ARCHITECTURE.md`](UI-ARCHITECTURE.md)).

Worktrees référence : `vercel-labs/wterm` (API surface, TS+Zig WASM Apache-2.0)
+ `microsoft/terminal` (algorithmes Buffer/Renderer/AtlasEngine/ConPTY MIT).
Politique : §2 CLAUDE.md "WASM Rust natif", memory `project_terminal_integration_policy`.

| Tick | Tâche | Statut |
|---|---|---|
| T-1 | Worktrees `vercel-labs/wterm` + `microsoft/terminal` (15 entries, ~1095 MB) | ✅ |
| T-1 | `docs/design/aphrody-terminal-spec.md` — spec normative LLM-first (fichier supprimé 2026-05-21) | ✅ |
| T-1 | `crates/aphrody-terminal-vt` — VT base (vte + ScreenBuffer + SGR 16-color) | ✅ (`crates/aphrody-terminal-vt/src/lib.rs`, 703 l.) |
| T-1 | `crates/aphrody-terminal-wasm` — wasm-bindgen DOM + M3 + keyboard ANSI | ✅ (`crates/aphrody-terminal-wasm/src/lib.rs`, 361 l.) |
| T-1 | `crates/aphrody-terminal-backend` — portable-pty (ConPTY/openpty) + WS | ✅ (`crates/aphrody-terminal-backend/src/lib.rs`, 287 l.) |
| T-2 | VT extension Ink/React TUI essentials (alt-screen, mouse SGR 1006, true color 24-bit, cursor save/restore, bracketed paste, focus events, DECSTBM, insert/delete line, OSC 0 title, OSC 52 clipboard) | ✅ (`crates/aphrody-terminal-vt` tests passed) |
| T-3 | `crates/aphrody-terminal-llm` — sub-agent stream multiplexer + MCP status bus + hook event surface + skill activation slot | ✅ (`crates/aphrody-terminal-llm/src/lib.rs` + modules `sub_agent.rs`, `mcp.rs`, `hook.rs`, `skill.rs`, `task.rs`, `osc.rs`) |
| T-4 | `crates/aphrody-terminal-markdown` — comrak CommonMark + syntect highlight + OSC `aphrody-md` detector | ✅ (`crates/aphrody-terminal-markdown` built and tested) |
| T-5 | `crates/aphrody-terminal-json-out` — frame stdout/stderr in JSONL envelopes + passthrough app-JSON | ✅ (`crates/aphrody-terminal-json-out` built and tested) |
| T-6 | `crates/aphrody-terminal-config` — `~/.aphrody/terminal.json` strict schema + claude.json / mcp.json / settings.json import shims | ✅ (`crates/aphrody-terminal-config` built and tested) |
| T-6b | `crates/aphrody-terminal-browser` — bridge LLM ↔ DOM (bxc in-process + agent-browser RPC + edge headless fallback) + OSC `aphrody-browser-*` extensions | ✅ (`crates/aphrody-terminal-browser/src/lib.rs` 306 l. + backends `bxc.rs`, `agent_browser.rs`, `edge.rs` + `osc.rs` + `proto.rs`) |
| T-7 | `aphrody term` CLI subcommand (serves backend + prints WASM UI URL) | ✅ (`crates/cli/src/commands.rs:1411-1445` + `Commands::Term` dispatch dans `main.rs:238`) |
| T-7 | `crates/aphrody-wasm/examples/aphrody-terminal-demo.html` — pixel-perfect M3 demo showcase | ✅ (`crates/aphrody-wasm/examples/aphrody-terminal-demo.html` exists) |
| T-8 | Demo gif : Claude Code running inside aphrody-terminal w/ live sub-agent pane (D+8-15 hero asset) | ✅ (assets générés via agg resvg) |
| T-8 | `docs/audits/2026-05-17-wterm-vs-microsoft-terminal-vs-aphrody-terminal.md` | ✅ (audit doc présent) |
| T-9 | `packages/aphrody-jsx` — Bun-native react-reconciler → `aphrody-jsx-*` OSC bridge (Ink-compatible API, M3 native, dual target vt/wasm) | ✅ (`packages/aphrody-jsx/src/reconciler.ts` 455 l. + jsx-runtime + 6 components + 6 hooks + tests + examples) |
| T-10 | `crates/aphrody-tui` — pure Rust ratatui-style DSL (canonical long-term, 60fps target, zero JS) | ✅ (`crates/aphrody-tui` built and tested) |

### Phase P-Test — Validation end-to-end binaire installé (2026-05-19)

`cargo build --release -p aphrody --locked` (3 min 28 s, 8.3 MB sur disque) → copié dans `~/.local/bin/aphrody.exe`. Smoke matrix exhaustive sur les 27 sous-commandes top-level.

| Sous-commande | Action testée | Statut |
|---|---|---|
| `version` | print metadata | ✅ commit 14a1225987, target x86_64-pc-windows-msvc |
| `doctor` | env + a2a + supply-chain (text + `--json`) | ✅ DEGRADED car peer winclean offline (heartbeat 109k s) |
| `self bootstrap --check` | inventory toolchain | ✅ rustup/cargo/rustc/git/zigbuild/deny/vet + 3 targets OK, `wasm-bindgen` optionnel manquant |
| `self install-path --dry-run` | PATH plan-only | ✅ — préfère copie manuelle vers `~/.local/bin` (évite registry HKCU avec path target/) |
| `completions {bash,zsh,fish,pwsh,elvish}` | 5 shells | ✅ bash 2057 l. / zsh 1562 / pwsh 613 / elvish 496 / fish 231 |
| `scan tree --root . --groups crates` | walkdir + JSON | ✅ 68 crates / 3.6 GB / 9 501 files |
| `scan manifests --root .` | Cargo+package+pyproject sweep | ✅ 926 manifests détectés (472 package.json, 138 Cargo.toml) |
| `dns google.com` | OSINT passif multi-sources | ✅ 287 sous-domaines uniques |
| `dns example.com` | OSINT minimal | ✅ 2 sous-domaines |
| `search "rust nightly"` | Google scraping | ⚠️ 0 résultats (Google bloque sans IP rotation — code roule, sortie attendue) |
| `a2a "ping"` | client A2A + fallback Gemini CLI | ✅ "pong" via routing fallback Gemini CLI (token Gemini auto-extracted) |
| `notify --channel slack --message test` | sans creds | ✅ erreur structurée correcte (missing `SLACK_CHANNEL`) |
| `oc-onboard --non-interactive --accept-risk` | bootstrap state | ✅ crée `~/.aphrody/aphrody.json` + workspace |
| `oc-pairing add slack U123 ABC42` + `list` + `approve slack ABC42` | roundtrip | ✅ pending → approved (sender `U123`) |
| `oc-reset --scope full --dry-run` | preview deletions | ✅ liste 2 paths à supprimer |
| `oc-uninstall --all --dry-run` | preview multi-scope | ✅ service + state + workspace (app skip macOS) |
| `oc-docs --url-only [query]` | doc URL builder | ✅ `https://docs.aphrody.dev` + `?q=…` |
| `chromium sync` | profil scan + master key | ✅ 7 profils Chromium détectés + master key déchiffrée |
| `mirror` (default action `start`) | MD3 assets | ⚠️ silent exit 0 (no-op visible — investigate intent) |
| `auth` | God Mode + OAuth2 fallback | ⚠️ Chrome Canary détecté mais aucun token Google valide ; délégue à gemini-cli embarqué qui bloque l'appel `run_shell_command` (sandbox correct) |
| `tokens` (M3 design tokens) | bxc passthrough (`:root` selector + regex `--md-*`) | ✅ écrit JSON (0 entrées pour m3.material.io car tokens en shadow DOM ; à enrichir) |
| `scrape --selector h1 example.com` | auto-start bxc daemon + `/api/scrape` | ✅ `{"matches":[{"index":0,"text":"Example Domain"}], "selector":"h1", "url":"https://example.com"}` |
| `bxc detect <url>` | `/api/detect` | ✅ identifie Cloudflare CDN (cf-ray + IPs) + DNS (`elliott.ns.cloudflare.com`) sur example.com |
| `bxc recon <url>` | `/api/recon` | ✅ `{$schema:"bxc-recon-v1", url, finalUrl, bytes:528, httpStatus:200, cssSelectors:[a,body,div,h1], headers:{cdnVendor:Cloudflare, …}, gotoMs:20.0}` |
| `bxc daemon --port 8765` | auto-select Bun driver via `select_bxc_driver()` | ✅ `[bxc daemon] started pid=13772 port=8765 driver=Bun. PID file: var/run/bxc.pid` ; `/healthz` 200 OK |
| `term --addr 127.0.0.1:18799` | WebSocket-PTY pour WASM UI | ✅ "ws://127.0.0.1:18799 (open the WASM UI to connect)" |
| `gemini --version` | forward gemini-cli embarqué | ✅ 0.42.0 |
| `n2b scan` | forward bun n2b CLI | ✅ n2b 0.6.0 → "0 errors, 0 warns, 0 infos" |
| `coreutils` / `util-linux` | actions build | ❌ `os error 267` — crates `crates/coreutils/` + `crates/util-linux/` retirés du workspace (cf. CLAUDE.md §4) mais commandes encore wired dans `crates/cli` |

**Gaps identifiés (à traiter en P-Test-fix)** :

| # | Gap | Source | Fix proposé |
|---|---|---|---|
| 1 | **Chaîne bxc cassée à trois niveaux**. Détail : (a) `aphrody bxc daemon` invoque `bxc-engine serve --port 8765` mais le binaire Rust `bxc-engine` (alias "obscura" dans `crates/bxc-engine/`) n'a pas de subcommand `serve` — uniquement `launch` (CDP server WebSocket port 9222), `fetch`, `scrape`, `mcp`, `chrome-path`. (b) Quand on lance le bon serveur HTTP API via `packages/bxc/` (Bun, `bun run src/cli/index.ts api --port 8765`), les routes sont préfixées `/api/recon`, `/api/detect`, `/api/scrape`, mais `crates/cli/src/scrape.rs:89,113` POST sur `/recon`, `/scrape` (pas de prefix). (c) Le schema des réponses diverge : `aphrody scrape` attend `SelectorResult { url, selector, matches }` mais bxc Bun retourne `[{index, text}]`. Aphrody → bxc demande **3 fix orthogonaux**, à grouper en P-Test-fix-bxc. Validation live faite : bxc Bun `/api/scrape?url=https://example.com&selector=h1` → `[{index:0, text:"Example Domain"}]`, `/api/detect` → Cloudflare CDN identifié (cf-ray header), `/healthz` 200 OK | Fixes ordered : (1) patcher `commands.rs:1675` `serve --port=…` → `api --port=…` ou `launch --port=…` selon target ; (2) patcher `scrape.rs:89,113` `/recon`/`/scrape` → `/api/recon`/`/api/scrape` ou rendre configurable via `BXC_DAEMON_URL` qui peut inclure `/api` suffix ; (3) ajouter adaptateur de schema OU réécrire le response struct dans `scrape.rs` pour matcher bxc Bun |
| 2 | `coreutils` / `util-linux` commandes orphelines | crates sortis du workspace mais commandes restent dans `crates/cli/src/main.rs` | Soit cfg-gate les commandes (`#[cfg(any())]` no-op), soit retirer les variants des `Commands` enum, soit pointer vers binaires distincts |
| 3 | `mirror` silent exit 0 | aucune sortie utilisateur | Vérifier intention — log explicit `[ok] mirror started (n assets)` ou `[skip] no-op` |
| 4 | `search` Google scraping no-results | Google bloque le scraping naïf | Ajouter fallback DuckDuckGo HTML / Brave Search API / SearXNG instance |
| 5 | `aphrody version --json` absent | seul format text | Ajouter `--json` parity avec `doctor --json` |

## 4. Métriques de santé (snapshot 2026-05-19)

| Métrique | Valeur | Cible |
|---|---|---|
| Workspace members | 17 | 17 (post-pivot : 10 core + 5 mrx + aphrody-translate + aphrody-wasm ; google_os exclu) |
| `cargo check --locked` (Linux) | ✅ (cross x86_64-unknown-linux-gnu, exit 0) | < 1 min |
| `cargo clippy -- -D warnings` | ✅ (workspace --all-targets, 16,07 s, exit 0) | 0 erreur |
| `cargo deny check` | ✅ ok×4 | ok×4 |
| `cargo build --release -p aphrody --locked` | ✅ 3 min 28 s (Win11 Insider Canary, mimalloc + ring + scraper + rayon) | < 5 min |
| Binaire installé `~/.local/bin/aphrody.exe` | ✅ 8.3 MB, 27 sous-commandes top-level fonctionnelles | shipping |
| Smoke 27 sous-commandes (2026-05-19) | 19 ✅ / 6 ⚠️ / 2 ❌ (coreutils/util-linux orphelins ; chaîne bxc validée live mais 3 mismatchs à fixer côté aphrody) | tendance vers 27 ✅ |
| Chaîne bxc live (2026-05-19) | ✅ `bxc-engine` (Rust, 49 MB) + `packages/bxc/rust-bridge/bxc_rust_bridge.dll` + `bxc api` Bun server tournent ; `/api/scrape` extrait "Example Domain" depuis example.com en <1s ; `/api/detect` identifie Cloudflare CDN. aphrody attache requires 3 patches docs §gap #1 | aphrody scrape live |
| Repo size (sans target, sans mdi) | ~7 Go (vendor/bun dominant) | optimisé |
| Disque libéré (P1+pivot) | 1.2 Go (vendor/crates.io) + 4.6 Go (material-design-icons) | n/a |
| CVE ignorés (justifiés) | 5 (4 stale retirés 2026-05-26, cf. `docs/audits/2026-05-26-supply-chain-hygiene.md`) | < 5 (après upstream alignment) |
| Cibles cross-platform bloquantes | 3 (Linux/Win/wasm) | 3 |

## 5. Roadmap tooling — intégration bxc / n2b dans aphrody (2026-06-04)

Objectif : faire de `aphrody` le hub de tooling unifié (CLI + plugin + MCP) au-dessus
des outils frères `bxc` (crawler/web Bun+Rust) et `n2b` (Node→Bun). Pré-requis :
analyses profondes `docs/audits/2026-06-04-{n2b,bxc}-analysis.md` (en cours).

| # | Item | Surface | Statut |
|---|---|---|---|
| T1 | Sous-commande CLI `aphrody bxc …` (façade vers le binaire/daemon bxc) | `crates/cli` | ✅ **2026-06-06** |
| T2 | Sous-commande CLI `aphrody n2b …` (façade scan/fix Node→Bun) | `crates/cli` | ✅ **2026-06-06** |
| T3 | Skill plugin `n2b` (migration Node→Bun guidée) | `.claude/plugins/aphrody/skills/` | ✅ **2026-06-06** |
| T4 | Skill plugin `bxc` (crawl/recon/scrape web) | `.claude/plugins/aphrody/skills/` | ✅ **2026-06-06** |
| T5 | Tool MCP `n2b` complet (`aphrody-mcp` : scan + fix + rapport JSON) | `crates/google_mcp` | ✅ **2026-06-06** |
| T6 | Agent md `lint-workflow` (oxc + n2b + bun + turbo + clippy/ruff) | `.claude/plugins/aphrody/agents/` | ✅ **2026-06-04** |
| T7 | Agent md `test-runner` (bun test + docs + bxc web full suite + nextest) | `.claude/plugins/aphrody/agents/` | ✅ **2026-06-04** |

T1-T5 dépendent des analyses bxc/n2b + impl Rust ; T6/T7 livrés (agents markdown).
Convention : façades CLI = spawn du binaire externe optionnel (cf. `aphrody n2b`
historique, CLAUDE.md §0.5), jamais de re-vendoring de la logique Bun.

---

*Pour la vue d'ensemble exécutive, lire [`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md).*
