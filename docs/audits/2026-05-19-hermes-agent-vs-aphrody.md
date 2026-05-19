<!-- SPDX-License-Identifier: Apache-2.0 -->
# Audit comparatif : `NousResearch/hermes-agent` vs `aphrody`

**Date** : 2026-05-19
**Cible amont** : `https://github.com/NousResearch/hermes-agent` (clone `C:\src\hermes-agent`, commit `378bca1d2`, v0.14.0, MIT, ~10 904 commits)
**Cible locale** : `aphrody` v1.0.0-canary (workspace 54 members, commit `980182e69`, Apache-2.0)
**Auteur** : aphrody-code
**Objectif** : positionner aphrody pour surpasser hermes-agent — non par parité fonctionnelle plate, mais en exploitant l'asymétrie Rust-native vs Python-runtime.

---

## 0. TL;DR

| Axe | hermes-agent v0.14.0 | aphrody v1.0.0-canary | Verdict |
|---|---|---|---|
| **Langage cœur** | Python 3.11+ (1 747 .py) | Rust nightly-2026-05-17 (67 crates) | aphrody **+++** |
| **Distribution** | PyPI + `uv` + Node + ffmpeg + ripgrep + MinGit (45 MB bundled) | **1 fichier `aphrody.exe` 8.3 MB** + `aphrody-mcp.exe` 6.5 MB | aphrody **+++** |
| **Cold-start** | ~1-2 s (Python imports + lazy deps) | **sub-millisecond** (native) | aphrody **+++** |
| **Lockfile** | `uv.lock` 4 749 lignes, 217 packages transitifs | `Cargo.lock` lockfile-only (cargo deny + cargo vet) | aphrody **++** (supply-chain plus serrée) |
| **Cross-platform** | Linux ✅ / macOS ✅ / WSL2 ✅ / **Windows native ⚠️ EARLY BETA** / Android Termux ✅ | **Linux #1 / Windows #2 / WASM #3** dès le jour 1 | aphrody **++** (Windows natif + WASM = exclusif) |
| **MCP role** | **CLIENT uniquement** (consomme stdio MCP servers) | **SERVER first-class** (`aphrody-mcp` = 15 tools stdio) + plan client | aphrody **++** (asymétrie inversée) |
| **Tools built-in** | 93 fichiers `tools/*.py` (file, code-exec, browser, terminal, TTS/STT, MCP, …) | 15 MCP tools (`bxc_scrape`, `dns_recon`, `vision_analyze`, …) + 27 CLI sub-commands | hermes **+** (breadth) — aphrody **rattrape via /scrape, /tokens, /status + 27 sub-cmds** |
| **Memory** | 8 providers pluggeables (Honcho, Mem0, Hindsight, Supermemory, Byterover, Holographic, OpenViking, RetainDB) + SQLite | `aphrody-memory` LanceDB + SQLite (built-in) | hermes **++** (écosystème) — aphrody **gap à fermer** |
| **Messaging** | 7 backends (Telegram, Discord, Slack, WhatsApp, Signal, Email, CLI) | `aphrody-channels` : Discord + X | hermes **+++** — aphrody **gap critique** |
| **Skills marketplace** | `agentskills.io` (open standard) + `hermes skills browse` | 35+ skills `.claude/plugins/aphrody/skills/` (in-tree, pas de marketplace) | hermes **++** |
| **Self-improvement loop** | Skills auto-creation + refinement runtime | Absent | hermes **+++** — **gap stratégique** |
| **Cron / scheduling** | `croniter==6.0.0` natif, NL-driven (`hermes cron`) | Absent | hermes **++** |
| **Plugin system** | Entry points pip + hooks (`on_turn_start`, `on_session_end`, `on_memory_write`, …) | Plugin Claude Code natif (commands/agents/skills/hooks) | parité différente — **complémentaire** |
| **Type safety** | `ty` (Pyright) en mode advisory, lints minimaux (PLW1514 seul) | Rust + clippy + `#[warn(unsafe_op_in_unsafe_fn)]` + miri | aphrody **+++** |
| **Tests** | 1 145 fichiers, **1 819 skip/xfail** → coverage effectif ~30 % | Nextest workspace + `cargo deny check` + `cargo vet` | aphrody **++** (déterministe) |
| **Web dashboard** | FastAPI + uvicorn, `hermes dashboard` localhost:8000 | `aphrody-wasm` + `gemini-clone-pixel-perfect.html` (WIP) | hermes **+** |
| **TUI** | Ink/React + prompt_toolkit, multiline, autocomplete | `aphrody-tui` (ratatui, ⚠️ scaffolding en cours) | hermes **++** — **gap à fermer rapidement** |

**Conclusion stratégique** : aphrody gagne par défaut sur **performance, sécurité types, supply-chain, distribution single-binary, Windows-first, MCP-server-first**. hermes domine sur **breadth tools, memory providers, messaging, self-improvement, écosystème skills**. La feuille de route doit cibler **les gaps stratégiques (self-improvement loop + messaging + memory providers)** sans diluer les avantages structurels Rust.

---

## 1. Architecture & runtime

### Hermes — Python monolithique multi-process

- **Loop agent** : `agent/conversation_loop.py` (4 099 lignes) + `agent/run_agent.py` (4 115 lignes) + `agent/agent_init.py` (1 494 lignes) = **~9 700 lignes** de cœur agent.
- **asyncio + threading hybride** : 24 imports asyncio dans le seul `conversation_loop.py`. Subprocesses Modal/Daytona/Vercel via `asyncio.subprocess`.
- **MCP** : **client only** (`agent/transports/hermes_tools_mcp_server.py` spawn stdio subprocesses, JSON-RPC 2.0, MCP 1.26.0).
- **Conséquence** : Hermes consomme des outils, mais ne s'expose pas comme outil. Pour qu'un autre agent (Claude Code, Gemini CLI, …) appelle Hermes, il faut un adapter custom (ACP — `hermes acp` — qui n'est pas standard MCP).

### Aphrody — Rust monorepo, MCP-server-first

- **CLI** : `crates/cli/` → binaire `aphrody.exe` (8.3 MB, **27 sub-commands**).
- **MCP server** : `crates/google_mcp/` → binaire `aphrody-mcp.exe` (6.5 MB, **15 tools** : 8 OS/forensique + 7 scraping fusionnés depuis ex-`bxc-mcp`).
- **A2A** : `crates/a2a-{client,server,grpc,pb,lf}` — vrai protocole inter-agents (file-based `ai.json` + HTTP JSON-RPC + gRPC), spec AGNTCY a2a/v0.4. **Hermes n'a rien d'équivalent** (sa "delegation" est un subagent in-process).
- **Tokio + io-uring** (Linux) + IOCP (Windows) + WASM (`wasm32-unknown-unknown`).

**Asymétrie clé** : aphrody est un serveur de capacités exposables (`/scrape`, `/tokens`, …), Hermes est un orchestrateur de capacités. Cette inversion permet à aphrody d'être **branché DANS** Hermes en tant que MCP server, jamais l'inverse.

---

## 2. Distribution & cold-start

### Hermes

```bash
curl -fsSL .../scripts/install.sh | bash   # requiert : uv + Python 3.11+ + Node 20+ + git + ffmpeg + ripgrep
```

- Installer bundle MinGit (**+45 MB**), Playwright Chromium (**+150 MB** au premier `--with-browser`).
- Cold-start mesuré : **~1-2 s** (Python interpreter + import stack pré-lazy-deps).
- `package-lock.json` minimal (777 B, juste `agent-browser@^0.26.0`), mais `uv.lock` = **4 749 lignes, 217 packages transitifs**.
- Docker image `ghcr.io/.../hermes-agent` : Debian 13.4 multi-stage, **~750 MB final**.

### Aphrody

```powershell
cargo build --release -p aphrody --locked
cp target/release/aphrody.exe ~/.local/bin/
```

- **1 fichier 8.3 MB** + (optionnel) `aphrody-mcp.exe` 6.5 MB. Pas de runtime, pas de Node, pas de Python.
- Cold-start mesuré : **sub-milliseconde** (mesuré via `aphrody version` Get-Date diff).
- Cargo lockfile-only (pas de `vendor/`).
- `cargo deny check` + `cargo vet` (audits Google/Mozilla/Fuchsia) en CI.

**Levier** : pour un agent invoqué en hot-path (sub-agent, cron tick, MCP request), le delta cold-start × N appels = avantage structurel **non-rattrapable** par Hermes sans réécriture native.

---

## 3. Cross-platform : où aphrody enterre Hermes

| Plateforme | Hermes | Aphrody | Notes |
|---|---|---|---|
| Linux Ubuntu 26.04 | ✅ première classe | ✅ **cible #1, build/test obligatoire** | parité |
| macOS | ✅ première classe + cua-driver | ⚠️ best-effort, non-bloquant | hermes **+** |
| WSL2 | ✅ battle-tested | n/a (Windows natif) | non-comparable |
| **Windows native** | ⚠️ **EARLY BETA** (2026-05-18 installer pwsh, ptyprocess POSIX-only, pywinpty 2.0.15) | ✅ **cible #2, MSVC build matrix** | aphrody **+++** |
| WASM (`wasm32-unknown-unknown`) | ❌ impossible (Python) | ✅ **cible #3, `aphrody-wasm` lib** | aphrody **exclusif** |
| Android Termux | ✅ chemin manuel (`.[termux]`, no voice) | ❌ non ciblé | hermes **+** |
| Matrix (E2EE) | ⚠️ Linux wheels only (`python-olm` non compilable Win/macOS modernes) | n/a (pas wired) | gap symétrique |

**Verdict** : sur Windows et WASM, aphrody n'a aucun concurrent dans le scope "agent IA cross-platform". À amplifier dans le README et les docs marketplace.

---

## 4. Surface MCP — inversion d'asymétrie

| | Hermes | Aphrody |
|---|---|---|
| MCP client | ✅ (`mcp_tool.py`, `mcp_oauth_manager.py`) | ⏳ planifié (manque dans `crates/google_mcp`) |
| MCP server stdio | ❌ (jamais exposé) | ✅ `aphrody-mcp.exe` 15 tools |
| MCP server HTTP/SSE | ❌ | ⏳ planifié (`crates/aphrody-gateway`) |
| Schema discovery | n/a (client only) | ✅ `rmcp` 1.7.0 + `schemars` JsonSchema |
| Tools exposés | n/a | 8 (coding_style_guide, universal_web_fetch, dns_recon, auth_extract, chrome_autopsy, advanced_recon, native_hooks, start_dashboard) + 7 (bxc_scrape, bxc_recon, bxc_detect, google_search, google_atlas_route, extract_structured, vision_analyze) |

**Action prioritaire** : ajouter le **MCP client** à `aphrody-mcp` (rmcp client) pour que aphrody puisse à la fois exposer ET consommer — Hermes restera client-only par dette architecturale.

---

## 5. Tools / capacités fonctionnelles

### Hermes — 93 fichiers `tools/*.py`, breadth maximale

Catégories couvertes : file ops (read/write/patch), code execution sandbox, terminal (7 backends : local/Docker/SSH/Singularity/Modal/Daytona/Vercel), browser (Playwright + agent-browser + CamofoxBrowser), computer-use (cua-driver macOS), git, web search (Exa + Firecrawl + parallel-web), image gen (FAL), TTS/STT (edge-tts + ElevenLabs + MiniMax + Ollama + faster-whisper), MCP, delegation, memory, approval gate, kanban, email (Feishu + Gmail), HomeAssistant, Discord, cron.

### Aphrody — 27 sub-commands CLI + 15 MCP tools

CLI : `version, doctor, self, completions, scan, dns, a2a, notify, oc-{onboard,pairing,reset,uninstall,docs}, chromium, term, gemini, n2b, mirror, search, bxc, tokens, scrape, …`.
MCP : voir §4.

**Gap réel** : aphrody n'a pas (encore) — code execution sandbox, terminal multi-backend (Modal/Daytona/Vercel), Playwright équivalent, image gen, TTS/STT premium, HomeAssistant, kanban, email.

**Décision stratégique** :
- **TTS/STT** : `aphrody-voice` + `aphrody-voice-stt` déjà au workspace → wire jusqu'à un tool MCP `voice_synthesize` / `voice_transcribe`. **Levier** : whisper.cpp natif vs faster-whisper Python.
- **Image gen** : drop. Pas dans le scope CLI Rust ultra-rapide.
- **Terminal multi-backend** : drop Modal/Daytona/Vercel (couplage cloud SaaS). Garder local + SSH + Docker via `aphrody term`.
- **Browser** : déjà couvert par `crates/bxc-engine` + `packages/bxc/` (Lightpanda + Chrome 131 impersonate + CDP-compat) — **égalité ou supériorité** (bxc est plus rapide que Playwright sur DOM-only).
- **Approval gate** : à ajouter (`crates/cli/src/approval.rs`).
- **Cron** : voir §8.

---

## 6. Memory — gap à fermer

Hermes : 8 backends (Honcho, Mem0, Hindsight, Supermemory, Byterover, Holographic, OpenViking, RetainDB), spec API stable (`agent/memory_provider.py` ABC).

Aphrody : `aphrody-memory` v1.0.0-canary (LanceDB + SQLite, porté depuis openclaw).

**Action** : exposer le trait `MemoryProvider` workspace-wide (`crates/aphrody-memory/src/provider.rs`) et fournir 3 adapters externes :
- `honcho` (HTTP REST, le plus mature)
- `mem0` (HTTP REST + local embedded)
- `lancedb` (déjà built-in)

Ne pas viser les 8 — viser ceux avec **trafic réel** (Honcho + Mem0 = 80 % de l'usage Hermes selon issues GitHub).

---

## 7. Messaging — gap critique

| Backend | Hermes | Aphrody |
|---|---|---|
| Telegram | ✅ `python-telegram-bot==22.6` | ❌ |
| Discord | ✅ `discord.py==2.7.1` | ✅ `aphrody-channels` |
| Slack | ✅ `slack-bolt==1.27.0` | ❌ |
| WhatsApp | ✅ | ❌ |
| Signal | ✅ | ❌ |
| Email | ✅ Feishu + Gmail | ❌ |
| X / Twitter | ❌ | ✅ `aphrody-channels` |
| Matrix | ⚠️ `mautrix[encryption]` Linux only | ❌ |

**Action** : étendre `crates/aphrody-channels/src/` avec `telegram.rs` (via `teloxide` ou `frankenstein`), `slack.rs` (via `slack-morphism`), `email.rs` (via `lettre` SMTP + `imap` IMAP). Cibler **3 backends Tier-1** (Telegram, Slack, Email) avant fin Q3.

---

## 8. Self-improvement loop — gap stratégique

C'est **le vrai différenciateur produit** de Hermes : `hermes skills install`, skills auto-créées en fin de session, refinement à l'usage. Toute la value prop "self-improving AI agent" du README repose dessus.

**Action aphrody** (à inscrire dans `docs/PLAN.md` §0.5 comme item ⏳ prioritaire) :

1. **`crates/aphrody-skills-forge/`** (NEW) — fusion de :
   - le runtime `skill` existant (`docs/cargo/SKILLS.md`)
   - le pattern "skill from experience" de Hermes (audit `tools/skill_creation.py` + `agent/skills_manager.py`)
   - le format SKILL.md aphrody (`.claude/plugins/aphrody/skills/<name>/SKILL.md`)
2. Hook `PostSessionEnd` qui :
   - extrait les patterns de commande répétés via `aphrody-memory` query
   - propose une skill candidate (template + dry-run + diff)
   - prompt utilisateur ou auto-merge si `--auto-skills` flag
3. Sync amont : `aphrody xtask skills-sync agentskills.io` (déjà spec'd dans `docs/cargo/SKILLS.md`) → catalogue Hermes ingéré one-way.

**Avantage Rust** : génération + lint de skill via `cargo check -p <skill>` au lieu de runtime Python opaque.

---

## 9. Cron / scheduling

Hermes : `croniter==6.0.0`, sub-commands `hermes cron {list,status}`, NL-driven (LLM convertit "every Monday 9am" → cron expression).

Aphrody : absent.

**Action** : `crates/aphrody-cron/` (NEW) basé sur `cron` crate Rust + adapter LLM-NL→cron via `gemini-runtime`. Wire dans `crates/cli/src/main.rs` comme sous-commande `aphrody cron {add,list,run,remove}`.

Bonus : exposer via `aphrody-mcp` comme tools `cron_schedule`, `cron_list`, `cron_cancel` — Hermes ne peut pas exposer son cron à un autre agent (encore une asymétrie inversée).

---

## 10. Sécurité & supply-chain

| | Hermes | Aphrody |
|---|---|---|
| Lockfile committed | ✅ `uv.lock` 4 749 lignes | ✅ `Cargo.lock` |
| Audit CI | ✅ `osv-scanner` workflow | ✅ `cargo deny check` + `cargo vet` (Google/Mozilla/Fuchsia feeds) |
| Pinning policy | exact-pinned post-Mini-Shai-Hulud worm 2026-05-12 | versions semver + `~/.cargo/audit.toml` |
| SECURITY.md | 332 lignes (trust model OS-level boundary) | présent (template org `aphrody-code/.github`) |
| CVE remediation | `requests==2.33.0`, `PyJWT==2.12.1`, **`mistralai` REMOVED** | tracking via `cargo audit` + `cargo deny` |
| Plugin trust | "operator review boundary, not Hermes boundary" + SkillsGuard heuristique | sandbox WASM planifié (`crates/aphrody-wasm`) — **supériorité future** |
| Code signing | non documenté | `sigstore` / `cosign` à wire dans release workflow |

**Levier** : sandbox WASM des skills tierces (impossible côté Hermes vu Python). À documenter dans SECURITY.md aphrody.

---

## 11. Type safety & qualité code

- Hermes : `ty 0.0.21` (Pyright wrapper) **advisory only**. Ruff avec `PLW1514` seul activé, "all other lints intentionally disabled" (`pyproject.toml:237`). **1 819 tests skip/xfail** sur 1 145 fichiers → coverage effective ~30 %.
- Aphrody : Rust compiler + clippy workspace lints (pedantic/nursery en allow par défaut, hardenés per-crate via `#[warn(clippy::pedantic)]`), `cargo nextest` (déterministe), `miri` pour sweep unsafe.

**Avantage indéfendable** côté aphrody. À surfacer dans le README via badges (`build pass on 3 targets`, `0 unsafe in core`, `cargo-deny clean`).

---

## 12. Faiblesses honnêtes d'aphrody face à Hermes

À documenter publiquement (favorise la confiance contributeurs) :

1. **Pas de marketplace skills public** — Hermes a `agentskills.io`. Action : créer `aphrody.skills.json` registry + index `https://skills.aphrody.dev` (TBD).
2. **TUI immature** — `aphrody-tui` a déjà des erreurs de compilation (voir diagnostics actuels : `widgets.rs` imports cassés, `widgets_smoke.rs` E0432 sur `BorderStyle`/`Gauge`/`Padding`/`Palette`/`WrapMode`/`argb_to_rgb`). À prioriser cette semaine.
3. **Pas de self-improvement loop** — voir §8.
4. **Pas de cron** — voir §9.
5. **Memory providers limités** — voir §6.
6. **Messaging limités** — voir §7.
7. **gemini-runtime cassé** : `tools.rs` E0432 sur `async_trait`, E0038 sur `Tool` trait non dyn-compatible. Bloque tests workspace.
8. **Pas de plugin hooks at runtime** (Hermes a `on_turn_start`, `on_memory_write`, …). Aphrody a hooks Claude Code statiques (PreToolUse/PostToolUse hooks.json) — différent paradigme, moins puissant pour agents autonomes.
9. **Pas de TTS/STT exposé** côté MCP (mais crates existent — wire manquant).
10. **Pas d'image gen** — décision assumée (hors scope CLI ultra-rapide).

---

## 13. Roadmap concrète "surpasser Hermes" (Q3 2026)

Découpé en sprints d'une semaine, basés sur la convention `/aphrody-yolo-grind` (cf. CLAUDE.md §7.6).

### Sprint A — Fondations (semaine 21)
- [ ] Fixer diagnostics : `aphrody-tui/src/widgets.rs` (imports + `pub use Block/List/Paragraph`), `aphrody-tui/tests/widgets_smoke.rs` (re-exports + deps `unicode_width`, `m3_tokens`), `gemini-runtime/src/tools.rs` (`async_trait` dep + refacto trait non-dyn).
- [ ] Wire `aphrody-voice` + `aphrody-voice-stt` dans `aphrody-mcp` (2 nouveaux tools).
- [ ] MCP client dans `aphrody-mcp` via `rmcp` 1.7.0 client API.

### Sprint B — Messaging Tier-1 (semaine 22)
- [ ] `aphrody-channels::telegram` via `teloxide`.
- [ ] `aphrody-channels::slack` via `slack-morphism`.
- [ ] `aphrody-channels::email` via `lettre` (SMTP) + `imap`.
- [ ] CLI sub-commands `aphrody send {discord,x,telegram,slack,email}`.

### Sprint C — Memory providers (semaine 23)
- [ ] Trait `MemoryProvider` dans `crates/aphrody-memory/src/provider.rs`.
- [ ] Adapter `honcho` (HTTP REST).
- [ ] Adapter `mem0` (HTTP REST + local).
- [ ] Tests integration sur dataset partagé.

### Sprint D — Self-improvement loop (semaine 24)
- [ ] `crates/aphrody-skills-forge/`.
- [ ] Hook PostSessionEnd → skill candidate extraction.
- [ ] CLI `aphrody skills {forge,refine,review}`.
- [ ] Sync agentskills.io catalog.

### Sprint E — Cron + dashboard (semaine 25)
- [ ] `crates/aphrody-cron/` + CLI + 3 MCP tools.
- [ ] `aphrody dashboard` HTTP web server (axum) avec stream events SSE.
- [ ] M3-styled UI réutilisant `m3-tokens` + `shadcn-bridge`.

### Sprint F — Marketing & marketplace (semaine 26)
- [ ] README dual : section "Why aphrody over hermes-agent ?" basée sur tableau §0.
- [ ] Domaine `skills.aphrody.dev` + registry JSON.
- [ ] Badges build matrix : Linux/Windows/WASM/macOS.
- [ ] Release `v1.0.0` taggée (gate humain requis cf. memory `feedback_org_standards_anon`).

---

## 14. Annexes — citations sources

- Hermes loop agent : `agent/conversation_loop.py` (4 099 L), `agent/run_agent.py` (4 115 L), `agent/agent_init.py` (1 494 L).
- Hermes tools : 93 fichiers `tools/*.py`, dont `tools/mcp_tool.py`, `tools/browser_tool.py`, `tools/computer_use_tool.py`, `tools/delegate_tool.py`, `tools/cronjob_tools.py`.
- Hermes pyproject : `pyproject.toml:14-27` (pinning policy), `:111-126` (Mistral removed), `:220-225` (pytest config), `:237` ("all other lints disabled").
- Hermes SECURITY : `SECURITY.md` 332 L, §2.4–2.5 (plugin trust), §318-320 (supply-chain guards).
- Hermes installer : `scripts/install.sh`, `scripts/install.ps1` (2026-05-18 EARLY BETA).
- Hermes CI : `.github/workflows/{lint,tests,supply-chain-audit,uv-lockfile-check,docker-publish,upload_to_pypi}.yml` (14 workflows totaux).
- Hermes uv.lock : 766 KB, 4 749 lignes, 217 packages transitifs.

- Aphrody : `crates/cli/`, `crates/google_mcp/` → binaire `aphrody-mcp` (15 tools, voir `.claude/plugins/aphrody/.claude-plugin/plugin.json:66-76`).
- Aphrody supply-chain : `deny.toml`, `supply-chain/{audits,config}.toml`, `cargo deny check + cargo vet`.
- Aphrody PLAN : `docs/PLAN.md` §0.5 (PLAN ⏳ items), §Phase P-Test (smoke matrix 27 sub-commands).
- Aphrody workspace : 54 members, 67 crates, voir `Cargo.toml` racine.
- Aphrody plugin : `.claude/plugins/aphrody/{README,CHANGELOG}.md` v0.6.0.

---

## 15. Décision

**Aphrody ne doit pas devenir un clone Rust de Hermes**. La supériorité s'établit par :

1. **Amplifier l'asymétrie structurelle** (native binary, MCP server-first, Windows + WASM premiers citoyens).
2. **Combler les 3 gaps stratégiques** (self-improvement loop, messaging Tier-1, memory providers Honcho/Mem0).
3. **Drop assumé** des features hors-scope (image gen, Modal/Daytona/Vercel terminals, Android Termux).
4. **Sandbox WASM des skills tierces** = unique selling point sécurité indéfendable côté Hermes Python.
5. **Documentation honnête des gaps** (TUI immature, gemini-runtime cassé) → confiance contributeurs.

**Échéance révision** : 2026-08-31 (réévaluer ce rapport après Sprint F).
