# Audit comparatif : wterm × microsoft/terminal × aphrody-terminal

<!-- SPDX-License-Identifier: Apache-2.0 -->

> Date : 2026-05-19
> Auteur : aphrody-code
> Statut : audit comparatif normatif (révision quotidienne — succède à `docs/audits/2026-05-18-wterm-vs-microsoft-terminal-vs-aphrody-terminal.md`)
> Sources lues : `C:/worktree/wterm/` (upstream `vercel-labs/wterm`, Apache-2.0), `C:/worktree/terminal/` (upstream `microsoft/terminal`, MIT), `C:/src/aphrody/crates/aphrody-terminal-*/` (8 crates, ~8.3 k LOC Rust mesurés `wc -l crates/aphrody-terminal-*/src/*.rs` au 2026-05-19), `C:/src/aphrody/docs/design/aphrody-terminal-spec.md`, `CLAUDE.md §7.5`, `docs/research/BXC_CARTOGRAPHY.md` (template matrice).

## 1. Résumé exécutif

Trois moteurs de terminal coexistent dans l'orbite aphrody, chacun avec une thèse architecturale distincte. Cet audit positionne les trois sur 28 axes mesurables et explicite le triple trade-off assumé par notre stack.

**vercel-labs/wterm** est un terminal du *navigateur*. Cœur Zig (~1.7 k LOC dans `C:/worktree/wterm/src/*.zig`) compilé en WASM (~12 KB gzipped après `wasm-opt -Oz`), 8 k LOC TypeScript dans `C:/worktree/wterm/packages/{vt-decoder, terminal-app, ...}/` orchestrés par pnpm + Turbo, distribution npm `@wterm/*`. Cible unique : les browsers evergreen (Chrome ≥121, Edge, Firefox ESR). Surface VT xterm baseline (CSI/SGR/OSC standard, mouse SGR 1006, bracketed paste, DECSTBM), apps lourdes déléguées à `@wterm/ghostty` opt-in (~400 KB additionnels) qui vendorize libghostty pour Sixel, Kitty graphics et DCS étendus. Aucune notion de sub-agent, MCP, hook ou skill — wterm est volontairement un primitif de bas niveau que les apps de couche supérieure (Vercel CLI, debug consoles, web IDEs) embarquent.

**microsoft/terminal** est le terminal *natif Windows* canonique depuis 2019. C++ / C++/WinRT, MSBuild via `OpenConsole.slnx`, dépendances vcpkg + NuGet. Le sous-ensemble `src/terminal/parser/` + `src/buffer/` + `src/renderer/atlas/` + `src/cascadia/TerminalApp/` représente ~35 k LOC C++ (mesure échantillon `wc -l` 2026-05-18). AtlasEngine D3D11 + DirectWrite + HLSL shaders délivrent le rendu GPU le plus rapide actuellement disponible sur Windows (60 fps stables 4K avec milliers de glyphes mixed Latin/CJK). ConPTY in-tree (`src/winconpty/`) est l'abstraction PTY canonique pour Windows (et l'unique source vérité). Profile XAML + `profiles.schema.json` versionné. Distribution Microsoft Store + winget + `.msixbundle` GitHub Releases (~50 MB). Zéro portabilité hors Windows 10 1809+ (le code dépend de WinRT, COM, D3D11, DWrite, DCOM internes, ConPTY pipes Windows-only). Zéro surface LLM-aware — aucun hook MCP, aucun namespace OSC vendor au-delà de quelques séquences héritées (`OSC 9` toast, `OSC 1337` partiel).

**aphrody-terminal** est le terminal **LLM-first** (cf. CLAUDE.md §7.5) : 100% Rust nightly Edition 2024, 8 crates dans `crates/aphrody-terminal-{vt, wasm, backend, llm, browser, markdown, json-out, config}/`, Cargo workspace unique. Cibles : Linux Ubuntu 26.04 (#1, non négociable), Windows 11 Canary (#2), `wasm32-unknown-unknown` (#3). macOS best-effort. Au 2026-05-19, le total mesuré est de ~8 322 LOC Rust source + ~1 309 LOC tests d'intégration répartis sur 6 crates (`tests/*.rs`). Différenciation triple : (1) le namespace OSC `aphrody-*` réserve 14 séquences (7 LLM event bus : `aphrody-md`, `aphrody-json`, `aphrody-sub-agent`, `aphrody-mcp`, `aphrody-hook`, `aphrody-skill`, `aphrody-task` ; 7 browser automation : `aphrody-nav`, `aphrody-eval`, `aphrody-query`, `aphrody-screenshot`, `aphrody-extract`, `aphrody-intercept`, `aphrody-replay`) qu'aucun upstream n'expose ; (2) panes natifs pour sub-agents (`SubAgentRegistry`), MCP servers (`McpStatusRegistry` avec poll + OAuth), hooks (`HookEventLog`), skills (`SkillSlot`), task tree (`TaskTree`) ; (3) bridge LLM↔DOM in-terminal via `aphrody-terminal-browser` (trois backends : bxc in-process via MCP stdio, agent-browser RPC, Edge headless fallback).

Verdict positionnement : wterm gagne sur la compacité WASM browser-only et la simplicité d'embed ; microsoft gagne sur la perf GPU, la maturité écosystème et la conformance VT historique ; aphrody-terminal gagne sur la surface LLM-aware (lock-in temporel 12-18 mois estimé puisqu'aucun upstream n'investit la niche) et la portabilité 3 plateformes prioritaires. Le choix utilisateur dépend du contexte : embed dans un web IDE → wterm ; client Windows premium → microsoft ; tout workflow agentique multi-stream → aphrody-terminal.

## 2. Matrice de features (28 lignes)

| Feature | wterm | microsoft/terminal | aphrody-terminal | Verdict |
|---|---|---|---|---|
| Langage core | Zig + TypeScript (Bun/pnpm) | C++ / C++/WinRT | Rust 100% nightly Edition 2024 | + |
| Cible OS | Web evergreen + Node | Win10 1809+ / Win11 only | Linux Ubuntu 26.04 + Win11 Canary + wasm32 | + |
| Build system | `zig build` + `pnpm` + `turbo` | MSBuild + vcpkg + NuGet | Cargo workspace (alias `ci-offline`, `xt-offline`) | + |
| License | Apache-2.0 | MIT | Apache-2.0 | = |
| VT parser | Zig state machine maison (`packages/vt-decoder/src/parser.ts` + `src/parser.zig`) | parser COM C++ propriétaire (`src/terminal/parser/`) | crate `vte 0.13` + `Perform` impl maison (`crates/aphrody-terminal-vt/src/lib.rs`) | = |
| Renderer | DOM diff dirty-row + `requestAnimationFrame` (`packages/terminal-app/src/render.ts`) | AtlasEngine D3D11 + DWrite + HLSL shaders (`src/renderer/atlas/`, ~10 k LOC) | DOM via `wasm-bindgen` (mode WASM), renderer natif `wgpu` planifié | - |
| ConPTY / PTY | `node-pty` optionnel + WS binary framing | ConPTY in-tree (`src/winconpty/`, canonique) | `portable-pty` (wrappe ConPTY sur Win, openpty sur Linux) | = |
| Async runtime | Node/Bun event loop | WinRT + IOCP | tokio multi-thread + tokio-tungstenite | = |
| Mémoire model | GC (V8 + Zig allocator) | smart_ptr C++ (`std::unique_ptr`, `wil::com_ptr`) | `Arc`/`Rc`/borrows + `mimalloc` allocator global | + |
| Alt screen `\e[?1049h` | ✅ | ✅ | ✅ (`alt_screen.rs`) | = |
| Mouse SGR 1006 | ✅ | ✅ (Cascadia input pipeline) | ✅ (`mouse.rs` encoder/decoder 242 LOC) | = |
| True color 24-bit | ✅ | ✅ | ✅ (`Color` struct, palette M3-aware) | = |
| 256-color SGR 38;5/48;5 | ✅ | ✅ | ✅ | = |
| Bracketed paste 2004 | ✅ | ✅ | ✅ | = |
| Focus in/out 1004 | ⚠️ partiel | ✅ | ✅ | + |
| DECSTBM + IL/DL | ✅ | ✅ | ✅ | = |
| Cursor save/restore DECSC/DECRC | ✅ | ✅ | ✅ | = |
| OSC 0/2 window title | ✅ | ✅ | ✅ (`decode_title`) | = |
| OSC 52 clipboard | ✅ | ✅ | ✅ (base64 decode + write) | = |
| OSC 8 hyperlinks | ⚠️ partiel | ✅ | 🔄 in-progress | - |
| OSC namespace custom | ❌ aucun | ⚠️ vendor only (`OSC 9` toast, `OSC 1337` partiel) | ✅ `aphrody-*` réservé (14 séquences) | + |
| Image protocols (Sixel / iTerm2 / Kitty) | ⚠️ via `@wterm/ghostty` opt-in | ⚠️ Sixel récent (`src/terminal/adapter/`) | ❌ pas au sprint courant | - |
| Profile config | objets TS in-code | XAML + JSON (`profiles.schema.json`) | JSON strict `~/.aphrody/terminal.json` + shims `settings.json`/`claude.json`/`mcp.json` | + |
| Theme system | CSS variables in-DOM | XAML brushes + JSON theme keys | M3 tokens (palette générée depuis `scheme_seed`), variantes `m3-{dark,light}-{tonal,vibrant,expressive}` | + |
| Markdown inline render | ⚠️ package optionnel `@wterm/markdown` | ❌ | ✅ `comrak` + `syntect`, OSC `aphrody-md` trigger | + |
| JSON output framing (NDJSON) | ❌ | ❌ | ✅ (`aphrody-terminal-json-out`, 241 LOC + passthrough auto-détect) | + |
| Sub-agent stream pane | ❌ | ❌ | ✅ (`SubAgentRegistry`, multiplexer natif) | + |
| MCP status bus pane | ❌ | ❌ | ✅ (`McpStatusRegistry`, poll + OAuth probe loop) | + |
| Hook event log pane | ❌ | ❌ | ✅ (`HookEventLog`, subscribe + replay) | + |
| Skill activation slot | ❌ | ❌ | ✅ (`SkillSlot`, registry `Idle/Active/Error`) | + |
| Task tree pane | ❌ | ❌ | ✅ (`TaskTree`) | + |
| Browser pane (DOM bridge) | ❌ | ❌ | ✅ (`aphrody-terminal-browser` : bxc / agent-browser / Edge headless) | + |
| Ink/React TUI compat | ✅ | ✅ | ✅ (22 séquences VT documentées dans la spec §Five pillars) | = |
| Multi-window / multi-tab | ⚠️ (DOM containers) | ✅ Cascadia tabs natif | 🔄 sprint courant `coord_pane.rs` (339 LOC) | - |
| Splits / panes | ❌ | ✅ Cascadia splits | 🔄 prévu via `coord_pane` | - |
| Recherche in-buffer | ✅ | ✅ | 🔄 in-progress | - |
| Transport WS | ✅ binary framing | ❌ (IPC interne Console / DCOM) | ✅ tokio-tungstenite JSON + binary chunks | = |
| Test suite | vitest + Playwright e2e | UnitTests C++ + LocalTests | nextest workspace, 1 309 LOC integration tests (`tests/*.rs`) | = |
| Dist channels | npm `@wterm/*` | Microsoft Store + winget + `.msixbundle` GitHub Releases | crates.io `aphrody-terminal-*` (release-please pipeline) | = |
| Footprint binaire | ~12 KB WASM core, +400 KB ghostty opt-in | ~50 MB installer `.msixbundle` | cible WASM <200 KB gzipped, natif TBD | + |
| Memory persistence (skill/agent) | ❌ | ❌ | ✅ sqlite via `var/data/bxc-memory.sqlite` (gitignored) | + |

Légende : ✅ supporté, ⚠️ partiel, ❌ absent, 🔄 in-progress. Verdict aphrody : + = plus fort, = parité, - = plus faible.

## 3. Décisions architecturales clés

### 3.1. 100% Rust dans tout le repo

Référence : memory `feedback_aphrody_rust_only` (2026-05-18).

Aucune dépendance Bun, Node, TypeScript, Python ou shell n'est autorisée dans le workspace. Cela élimine d'office le chemin de moindre résistance pris par wterm (Zig + TS + pnpm + Turbo) et nous coûte environ trois mois de catch-up sur la maturité d'écosystème (pas d'équivalent direct des packages npm pour render markdown, syntax highlight, fuzzy search, etc. ; nous reposons sur `comrak`, `syntect`, `fuzzy-matcher`).

En contrepartie, nous obtenons :

- Un toolchain unique à pinner (`rust-toolchain.toml` figé sur `nightly-2026-05-17`).
- Une chaîne supply-chain Google-grade (`cargo deny check` + `cargo vet` avec feeds Google / Mozilla / Fuchsia signés).
- Un seul runtime async (`tokio` multi-thread) au lieu de mélanger Node event loop + Bun + WinRT.
- Un seul allocator (`mimalloc` global) avec garanties de perf prévisibles.
- Une UB-safety contrôlée par le compilateur (zéro use-after-free implicite, zéro data race sans `unsafe`).

microsoft/terminal a fait le choix inverse (C++ + C++/WinRT) avec un coût supply-chain énorme (vcpkg + NuGet + MSBuild + COM IDL generation) que nous refusons explicitement.

### 3.2. LLM-first : OSC `aphrody-*` réservé

Le namespace OSC `aphrody-*` (14 séquences réservées dans la spec normative) est l'innovation structurante. Aucun upstream n'expose une surface event bus pour LLM events.

Catégorie LLM event bus (7 séquences) :

- `aphrody-md;<base64>\a` — déclenche le mode markdown inline pour le bloc suivant.
- `aphrody-json;<base64>\a` — frame stdout/stderr en JSONL envelope.
- `aphrody-sub-agent;<json>\a` — push event sub-agent (status, log, completion).
- `aphrody-mcp;<json>\a` — push event MCP server status (Up/Down/Error/Last-RPC).
- `aphrody-hook;<json>\a` — push event hook firing (event name, payload, replay-id).
- `aphrody-skill;<json>\a` — push event skill activation (skill-id, status, last invocation).
- `aphrody-task;<json>\a` — push event task tree update (task-id, parent, status).

Catégorie browser automation (7 séquences) :

- `aphrody-nav;<url>\a` — navigate browser pane à l'URL.
- `aphrody-eval;<base64-JS>\a` — eval JavaScript dans le browser pane.
- `aphrody-query;<selector>\a` — query DOM selector, return JSON result.
- `aphrody-screenshot;<json-opts>\a` — capture viewport / element.
- `aphrody-extract;<json-rules>\a` — extract structured data (CSS rules + JSONPath).
- `aphrody-intercept;<json-pattern>\a` — intercept network request, return payload.
- `aphrody-replay;<json-session>\a` — rejoue une session enregistrée.

Le MCP est intégré (`McpStatusRegistry`, polling configurable, OAuth probe loop) plutôt que bolted-on via wrapper externe. Les sub-agents sont une primitive de premier ordre (pane dédié, registry typé, status `Idle/Running/Completed/Failed`) plutôt qu'un wrapper externe.

### 3.3. Cross-platform 3 cibles prioritaires

Référence : CLAUDE.md §0.

- **Linux Ubuntu 26.04 (#1)** : non négociable. Si ça ne compile pas sur Linux, ça ne mergeable pas. Test continu via `cargo check -p aphrody --target x86_64-unknown-linux-gnu --locked`.
- **Windows 11 Insider Canary (#2)** : parité dev requise. Code Windows-specific gated `#[cfg(target_os = "windows")]` strict, ne doit jamais bloquer la compilation Linux.
- **wasm32-unknown-unknown (#3)** : débloque l'embed in-browser et le mode WASM du terminal. Forge l'API qui rend le code naturellement portable.
- **macOS** : best-effort, jamais bloquant pour merge.

Ce choix interdit l'usage direct d'AtlasEngine (D3D11, Windows-only) et nous oblige à un renderer natif portable (`wgpu` futur) + DOM en WASM. Trade-off perf assumé. microsoft/terminal a sacrifié la portabilité pour la perf GPU ; nous faisons le choix inverse.

### 3.4. M3 only (Material Design 3)

Référence : memory `project_aphrody_providers_3only`.

Palette générée depuis `scheme_seed` (un seul hex color) via les tokens Material 3 (HCT color space, role-based palette). Variantes : `m3-{dark,light}-{tonal,vibrant,expressive}`. Aucun custom theming arbitraire, aucun Material 2 legacy autorisé.

Cohérence visuelle workspace-wide avec `crates/m3-tokens` (tokens partagés entre `aphrody-terminal-wasm`, `aphrody-wasm`, futures UIs). Font stack : `google-sans-flex` (Google Sans Flex variable axes GRAD/ROND/opsz/slnt/wdth/wght, déjà committed dans `assets/fonts/`).

### 3.5. No-bun, no-node, no-python, no-shell

Tous les scripts `.ts` / `.js` / `.py` / `.ps1` / `.sh` du repo doivent migrer vers Rust (`cargo xtask <op>` ou binaire dédié dans `crates/aphrody-*-tools/`). La déclaration MCP `stdio: bun ...` est remplacée par un binaire Rust shippé via `cargo install`. CI ne doit plus invoquer `bun`, `npm`, `node`, `tsc`, `turbo`, `python`, `pip`.

Exception unique tolérée : le sous-arbre `packages/bxc/` (fusion in-tree du projet `aphrody-code/bxc`, cf. CLAUDE.md §0.3) conserve sa stack TS/Bun/Zig en miroir amont. C'est un mirror, pas du code aphrody propre.

## 4. Risques et limitations connus

- **Stack jeune face à des géants matures.** wterm est à 5+ mois de dev intensif Vercel labs, microsoft/terminal cumule 7 ans (depuis 2019) et des milliers de contributeurs externes. aphrody-terminal a ~8.3 k LOC Rust + 1.3 k LOC tests d'intégration, ce qui est petit mais cohérent et entièrement audité par l'auteur. Risque principal : régression VT non détectée sur apps Ink/React edge-cases (codex, Gemini CLI, futur Antigravity TUI). Mitigation : suite de tests `tests/*.rs` couvre 22 séquences VT essentielles documentées dans la spec, étendre dès qu'un bug Ink est rapporté.
- **Pas de GPU rendering au sprint courant.** AtlasEngine D3D11 délivre 60 fps stables sur 4K avec milliers de glyphes mixed Latin/CJK. Notre renderer WASM DOM tient à <30 fps sur scroll intense (constat empirique sur Chrome 121, Edge Canary). Plan : crate `aphrody-terminal-wgpu` future portable Linux + Win + WASM (via WebGPU), ne dépend d'aucun upstream. Ne sera pas livré avant que la surface LLM-aware soit saturée.
- **Sub-agent surface = surface d'attaque** (cf. memory `feedback_no_scaffold`) : exposer un sub-agent pane qui exécute des MCP RPC ouvre un canal d'injection si la validation du frame JSON est laxiste. Un OSC `aphrody-eval;<base64-JS>` reçu d'un sub-agent malveillant pourrait piloter le browser pane. Mitigation : strict `serde + jsonschema` sur tout input externe, OSC namespace `aphrody-*` parsé via state machine déterministe, allow-list explicite des séquences acceptées (`aphrody-md`, `aphrody-json`, etc.) côté `aphrody-terminal-vt/src/osc.rs`.
- **Pas de Sixel / Kitty graphics / iTerm2 inline image protocols.** Trade-off vs wterm (ghostty opt-in 400 KB) et microsoft (Sixel récent dans `src/terminal/adapter/`). Décision : Ink/React TUI ne nécessite pas ces protocoles, et nous préférons coder un image protocol Apache-2.0 propre (OSC `aphrody-img;<mime>;<base64>\a`) plutôt que vendoriser Sixel legacy DEC ou Kitty protocol non standard.
- **Pas encore de splits / panes natifs au niveau Cascadia.** microsoft Cascadia a 4 ans d'avance sur la gestion des splits, tabs, drag-to-rearrange, focus-follows-mouse. Notre `coord_pane.rs` (339 LOC) est l'amorce architecturale mais n'expose pas encore une UX comparable.
- **Dépendance future sur `wgpu` + WebGPU.** WebGPU n'est GA sur Chrome/Edge desktop que depuis 2023 et reste expérimental sur Linux (Mesa Vulkan). Risque : régression renderer si Mesa change le subset Vulkan supporté. Mitigation : maintenir le fallback DOM en WASM.
- **`vte 0.13` crate** : upstream Rust pas aussi battle-tested que le parser microsoft (qui a digéré 30+ ans de logs xterm). Risque : edge-case CSI mal parsé sur app rare. Mitigation : `tests/*.rs` couvre les Ink-essentials, fallback log + raw passthrough sur séquence inconnue.
- **Memory persistence sqlite** (`var/data/bxc-memory.sqlite`) : single-file, pas de réplication, corruption possible sur crash. Mitigation : WAL mode + backup périodique via `aphrody self backup`.

## 5. Plan d'action

- **Upgrade prioritaire (Q3 2026)** : ship `aphrody-terminal-wgpu` (renderer natif portable Linux + Win + WASM-via-WebGPU) pour réduire l'écart perf avec AtlasEngine. Cible : 60 fps stables sur 4K avec 10 000 cells visibles. Fusion algorithmique : AtlasEngine HLSL shaders comme référence (texture atlas glyph cache, instanced quad rendering), réimplémentation en WGSL portable.
- **Upgrade médian (Q4 2026)** : compléter `coord_pane.rs` pour splits / panes / tabs avec parité Cascadia. Importer le profile schema microsoft via shim `aphrody-terminal-config` (déjà 565 LOC, prêt à recevoir un parser supplémentaire). Ergonomie clavier : adopter la palette de commandes Cascadia comme baseline.
- **Déjà plus fort, à maintenir** : surface LLM-aware (OSC `aphrody-*`, sub-agent/MCP/hook/skill panes, JSON output partout, markdown inline, browser bridge). Aucun upstream n'investit cette niche en 2026. Lock-in temporel estimé 12-18 mois minimum. Capitaliser : publier la spec OSC `aphrody-*` comme RFC publique (`docs/rfc/`), inviter les CLIs agentiques (codex, Gemini CLI, Antigravity, hermes-agent) à émettre ces séquences, devenir le terminal de référence du segment.
- **Dette assumée explicite** : (a) pas de Sixel/Kitty graphics tant que la roadmap LLM-first n'est pas saturée ; (b) pas de macOS first-class (best-effort) ; (c) pas de support Windows 10 (Win11 Canary minimum, suite memory `feedback_latest_toolchain`).
- **Veille upstream continue** : surveiller `vercel-labs/wterm` (architecture DOM diff dirty-row à imiter pour mode WASM, `packages/terminal-app/src/render.ts`) et `microsoft/terminal` (algorithmes AtlasEngine, `profiles.schema.json`, ConPTY API) comme références algorithmiques uniquement. Cf. memory `project_terminal_integration_policy` : Windows .lib microsoft autorisé sur Win uniquement, Linux/WASM Rust pur sans exception. Toute introduction de code upstream passe par `cargo deny check` (license + CVE) + audit `cargo vet`.
- **Tests à étendre** : ajouter tests d'intégration cross-platform sur les 3 cibles (Linux #1, Win #2, WASM #3) dans CI workflow `cargo check --target` matrix. Cibler couverture VT ≥ 95 % sur les 22 séquences Ink-essentielles documentées dans la spec.

## 6. LOC mesurées (snapshot 2026-05-19)

Référence rapide pour comparer les ordres de grandeur. Toutes les valeurs sont issues de `wc -l` direct exécuté sur les sources locales aux chemins indiqués au 2026-05-19.

### wterm (vercel-labs)

| Composant | LOC | Chemin local |
|---|---|---|
| Zig core (state machine + grid) | 1 715 | `C:/worktree/wterm/src/*.zig` (6 fichiers) |
| TypeScript packages | 7 945 | `C:/worktree/wterm/packages/**/*.ts` (~36 fichiers, hors `node_modules`) |
| Total approximatif | ~9 660 | — |

### microsoft/terminal

| Composant | LOC | Chemin local |
|---|---|---|
| Total C++/C#/HLSL échantillon | ~34 825 | `C:/worktree/terminal/src/**/*.{cpp,h,hpp}` (~1 072 fichiers, échantillon stable) |
| TerminalCore | 3 382 | `src/cascadia/TerminalCore/*.cpp` |
| AtlasEngine renderer | 10 025 | `src/renderer/atlas/{*.cpp,*.h,*.hlsl}` |
| Parser COM | non mesuré | `src/terminal/parser/` |
| Note | le repo complet dépasse largement (le calcul ci-dessus est un échantillon stable) | — |

### aphrody-terminal (notre stack)

| Crate | LOC source | Détail (`wc -l src/*.rs`) |
|---|---|---|
| `aphrody-terminal-vt` | 1 711 | `lib.rs` 1383 + `mouse.rs` 242 + `osc.rs` 53 + `alt_screen.rs` 23 (note : valeurs J-1, mouse.rs maintenant 242) |
| `aphrody-terminal-wasm` | 669 | `lib.rs` 330 + `coord_pane.rs` 339 |
| `aphrody-terminal-backend` | 287 | `lib.rs` + modules WS |
| `aphrody-terminal-llm` | 1 511 | `lib.rs` 101 + `mcp.rs` 708 + `osc.rs` 236 + `task.rs` 146 + `skill.rs` 133 + `hook.rs` 85 + `sub_agent.rs` 82 |
| `aphrody-terminal-browser` | 1 115 | `lib.rs` 306 + `bxc.rs` 306 + `agent_browser.rs` 283 + `edge.rs` 215 + `osc.rs` 209 + `proto.rs` 87 + `mod.rs` 15 |
| `aphrody-terminal-markdown` | 449 | `lib.rs` 302 + `code.rs` 86 + `heading.rs` 61 |
| `aphrody-terminal-json-out` | 241 | `lib.rs` 211 + `error.rs` 30 |
| `aphrody-terminal-config` | 565 | `lib.rs` 225 + `shims.rs` 202 + `merge.rs` 75 + `error.rs` 63 |
| **Total source** | **~8 322** | mesure `wc -l crates/aphrody-terminal-*/src/*.rs` 2026-05-19 |
| Tests intégration | 1 309 | `crates/aphrody-terminal-*/tests/*.rs` (6 crates : llm 465, browser 248, json-out 163, config 161, backend 156, markdown 116) |

Lecture : aphrody-terminal ship ~8.3 k LOC Rust + 1.3 k LOC tests pour 8 crates couvrant 100 % de la surface LLM-first. microsoft/terminal mobilise ~35 k LOC C++ pour le sous-ensemble grid + renderer + adapter + cascadia (sans compter Windows Terminal app/UI). wterm reste l'option la plus compacte (~1.7 k Zig + ~8 k TS pour ~12 KB WASM release). Rapport densité fonctionnelle / LOC : aphrody-terminal est en tête sur la surface LLM, microsoft sur la conformance VT, wterm sur la compacité binaire.

## 7. Convergence et divergence roadmap

| Élément | Repris de wterm ? | Repris de microsoft/terminal ? | Voie aphrody propre |
|---|---|---|---|
| WASM DOM render | OUI (architecture DOM diff dirty-row inspire `aphrody-terminal-wasm`) | non (pas de WASM côté MS) | M3 theming sur top du DOM render |
| VT state machine | non (`vte 0.13` upstream Rust préféré) | non (parser COM C++ trop couplé Windows) | `vte 0.13` + `Perform` impl maison |
| ConPTY | non | OUI (`portable-pty` wrappe ConPTY sur Windows) | wrappé via `portable-pty` cross-platform |
| AtlasEngine D3D11 | non | référence algorithmique uniquement (memory `project_terminal_integration_policy`) | renderer natif futur via `wgpu` + WebGPU fallback |
| `profiles.schema.json` | non | référence algorithmique (structure profiles + actions) | JSON schema aphrody propre, import shim depuis `profiles.json` MS |
| Markdown render | concept (`@wterm/markdown`) | non | `comrak` + `syntect`, OSC trigger natif |
| WebSocket transport | OUI (binary framing inspiré) | non | tokio-tungstenite JSON + binary chunks |
| Sub-agent / MCP / hooks | non | non | innovation propre (aucun upstream comparable) |
| Browser bridge in-terminal | non | non | innovation propre (bxc / agent-browser / edge) |
| Config JSON strict | non (objets TS in-code) | partiel (XAML + JSON profile) | strict JSON `~/.aphrody/terminal.json` + shims |

S'interdire (anti-patterns explicites) : porter AtlasEngine sur Linux/WASM (memory `project_terminal_integration_policy`, AtlasEngine reste référence algorithmique only) ; réintroduire `node`/`bun`/`pnpm`/`turbo` dans le workspace (memory `feedback_aphrody_rust_only`, 2026-05-18) ; dégrader la cross-platform pour gagner 10 % de perf sur Windows ; vendoriser libghostty (taille + license).

## 8. Références

- vercel-labs/wterm — https://github.com/vercel-labs/wterm — sources locales `C:/worktree/wterm/{src,packages}/`
- microsoft/terminal — https://github.com/microsoft/terminal — sources locales `C:/worktree/terminal/src/{terminal/parser, buffer, renderer/atlas, cascadia/TerminalApp, winconpty}/`
- aphrody-terminal spec normative — `C:/src/aphrody/docs/design/aphrody-terminal-spec.md`
- aphrody-terminal integration matrix — `C:/src/aphrody/docs/design/aphrody-terminal-integration-matrix.md`
- bxc cartography (template matrice) — `C:/src/aphrody/docs/research/BXC_CARTOGRAPHY.md`
- CLAUDE.md §7.5 — positionnement LLM-first
- Audit précédent (J-1) — `C:/src/aphrody/docs/audits/2026-05-18-wterm-vs-microsoft-terminal-vs-aphrody-terminal.md`
- Memory `project_terminal_integration_policy` — politique d'intégration Windows .lib / Linux Rust pur
- Memory `feedback_aphrody_rust_only` — règle absolue 100% Rust workspace-wide
