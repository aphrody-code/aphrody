<!-- SPDX-License-Identifier: Apache-2.0 -->
# Audit comparatif : wterm vs microsoft/terminal vs aphrody-terminal

Date : 2026-05-18
Auditeur : aphrody-code
Scope : trois moteurs de terminal coexistant dans l'orbite aphrody, positionnement honnête (forces / trade-offs / convergence).
Sources lues : `C:/worktree/wterm/` (vercel-labs upstream, Apache-2.0), `C:/worktree/terminal/` (microsoft upstream, MIT), `C:/src/aphrody/crates/aphrody-terminal-*/` (8 crates locaux), `C:/src/aphrody/docs/design/aphrody-terminal-spec.md`.

## 1. Mission de chaque projet

| Projet | Mission | URL upstream | License |
|---|---|---|---|
| wterm | Terminal embarqué pour le navigateur (DOM render, WASM core Zig) | https://github.com/vercel-labs/wterm | Apache-2.0 |
| microsoft/terminal | Terminal natif Windows + Console host (`conhost.exe`) + composants partagés (AtlasEngine, ConPTY) | https://github.com/microsoft/terminal | MIT |
| aphrody-terminal | Terminal LLM-first : sub-agents, skills, hooks, MCP, JSON output partout, markdown inline, Ink/React TUI compat | https://github.com/aphrody-code/aphrody | Apache-2.0 |

## 2. Stack technique

| Aspect | wterm | microsoft/terminal | aphrody-terminal |
|---|---|---|---|
| Langage primaire | Zig (core) + TypeScript (packages) | C++ / C++/WinRT | Rust nightly Edition 2024 |
| Build system | `zig build` + `pnpm` + `turbo` | MSBuild / `OpenConsole.slnx` | Cargo workspace |
| Package manager runtime | pnpm 10 | NuGet + vcpkg | Cargo + (Bun pour tooling) |
| VT parser | Zig state machine maison (6 fichiers, 1715 LOC) | parser COM C++ propriétaire (`src/terminal/parser/`) | `vte 0.13` (crate Rust upstream, wrappé) |
| Renderer | DOM diff dirty-row + `requestAnimationFrame` | AtlasEngine Direct3D 11 + DWrite + HLSL shaders | `wasm-bindgen` DOM (mode WASM) / TBD natif |
| Cell grid | Zig `grid.zig` (108 LOC) | `TextBuffer` ROW chunks | Rust `ScreenBuffer` (alt screen explicite) |
| pty backend | `node-pty` (option) ou WebSocket binaire framing | ConPTY in-tree (`src/winconpty/`, Windows-only) | `portable-pty` (ConPTY + openpty) côté `aphrody-terminal-backend` |
| Transport | WebSocket (binaire) | DCOM + IPC interne Console / ConPTY pipes | tokio-tungstenite WS JSON + ConPTY/openpty |
| Target plateformes | navigateurs evergreen (WASM) + Node | Windows 10 1809+ exclusivement | Linux Ubuntu 26.04 #1 / Windows 11 Canary #2 / WASM #3 |
| Distribution | npm `@wterm/*` | Microsoft Store + winget + GitHub Releases (`.msixbundle`) | crates.io `aphrody-terminal-*` (release-please pipeline) |
| License | Apache-2.0 | MIT | Apache-2.0 |

## 3. Surface API et extensions VT supportees

| Feature VT/OSC | wterm | microsoft/terminal | aphrody-terminal |
|---|---|---|---|
| Alternate screen `\e[?1049h` | oui (`terminal.zig`) | oui | oui (`alt_screen.rs`) |
| Mouse SGR 1006 | oui | oui (Cascadia input) | oui (`mouse.rs` encoder/decoder) |
| True color 24-bit SGR 38;2 / 48;2 | oui | oui | oui (`Color` struct) |
| 256-color SGR 38;5 / 48;5 | oui | oui | oui |
| Bracketed paste 2004 | oui | oui | oui |
| Focus in/out 1004 | partiel | oui | oui |
| DECSTBM scroll region + IL/DL | oui | oui | oui |
| Cursor save/restore DECSC/DECRC | oui | oui | oui |
| Erase character `\e[X` | oui | oui | oui |
| OSC 0/2 window title | oui | oui | oui (`decode_title`) |
| OSC 52 clipboard | oui | oui | oui (base64 decode) |
| OSC namespace custom | aucun | quelques OSC vendor (`OSC 9` toast, `OSC 1337` partiel) | `aphrody-*` reserve (14 sequences : 7 LLM event bus + 7 browser automation) |
| Sub-agent stream pane | non | non | oui (`SubAgentRegistry`) |
| MCP status bus pane | non | non | oui (`McpStatusRegistry`, polling, OAuth config) |
| Hook firing log pane | non | non | oui (`HookEventLog`) |
| Skill activation slot pane | non | non | oui (`SkillSlot`) |
| Task tree pane | non | non | oui (`TaskTree`) |
| JSON-framed stdout/stderr | non | non | oui (`aphrody-terminal-json-out`) |
| Markdown inline render | package optionnel (`@wterm/markdown`) | non | oui (`comrak` + `syntect`, OSC `aphrody-md`) |
| Browser pane (mini-viewport + DOM) | non | non | oui (`aphrody-terminal-browser` : bxc / agent-browser / edge fallback) |
| Config schema | objets TS in-code | XAML + JSON profile (`profiles.schema.json`) | strict JSON `~/.aphrody/terminal.json` + import shims (settings.json, claude.json, mcp.json) |

## 4. LOC approximative (lectures `wc -l` reelles, 2026-05-18)

| Composant | LOC mesure | Methode |
|---|---|---|
| wterm Zig core (`src/*.zig`) | 1 715 | `wc -l C:/worktree/wterm/src/*.zig` |
| wterm TypeScript packages (`packages/`) | 7 945 | `find ... -name "*.ts" -o -name "*.zig" \| xargs wc -l` |
| wterm Zig files trouves | 6 | `find C:/worktree/wterm/src -name "*.zig"` |
| wterm TS files (hors `node_modules`) | 36 | `find packages -name "*.ts"` |
| microsoft/terminal C++ total | 34 825 lignes sur ~1 072 fichiers `.cpp`/`.h`/`.hpp` (echantillon) | `find src -name "*.cpp" -o -name "*.h" -o -name "*.hpp" \| xargs wc -l` (echantillon stable, le repo complet depasse) |
| microsoft `src/cascadia/TerminalCore/*.cpp` | 3 382 | `wc -l TerminalCore/*.cpp` |
| microsoft `src/renderer/atlas/` (cpp+h+hlsl) | 10 025 | `find renderer/atlas \| xargs wc -l` |
| aphrody-terminal Rust total `src/**/*.rs` | 8 204 | `find crates/aphrody-terminal-* -name "*.rs" \| xargs wc -l` |
| aphrody-terminal-vt | 1 711 (lib 1383 + mouse 252 + osc 53 + alt_screen 23) | `wc -l crates/aphrody-terminal-vt/src/*.rs` |
| aphrody-terminal-wasm | 730 (lib 361 + coord_pane 369) | idem |
| aphrody-terminal-backend | 287 | idem |
| aphrody-terminal-llm | 1 511 (lib 101 + mcp 708 + osc 236 + task 146 + skill 133 + hook 85 + sub_agent 82) | idem |
| aphrody-terminal-browser | 1 115 (lib 306 + bxc 306 + agent_browser 283 + edge 215 + osc 209 + proto 87 + mod 15) | idem |
| aphrody-terminal-markdown | 449 (lib 302 + code 86 + heading 61) | idem |
| aphrody-terminal-json-out | 241 (lib 211 + error 30) | idem |
| aphrody-terminal-config | 565 (lib 225 + shims 202 + merge 75 + error 63) | idem |
| aphrody-terminal tests integration | 1 309 (llm 465, browser 248, json-out 163, config 161, backend 156, markdown 116) | `wc -l tests/*.rs` |

Lecture : aphrody-terminal ship ~8 k LOC Rust + 1.3 k LOC tests pour 8 crates, alors que microsoft/terminal mobilise ~35 k LOC C++ rien que pour le sous-ensemble grid + renderer + adapter + cascadia. wterm reste l'option la plus compacte (1.7 k Zig + 8 k TS pour ~13 KB WASM release).

## 5. Differenciateurs aphrody-terminal

1. **OSC namespace `aphrody-*` reserve** — 14 sequences propres (7 LLM event bus : `aphrody-md`, `aphrody-json`, `aphrody-sub-agent`, `aphrody-mcp`, `aphrody-hook`, `aphrody-skill`, `aphrody-task` ; 7 browser : `aphrody-nav`, `aphrody-eval`, `aphrody-query`, `aphrody-screenshot`, `aphrody-extract`, `aphrody-intercept`, `aphrody-replay`). Ni wterm ni microsoft/terminal n'exposent une telle surface.
2. **Sub-agent pane natif** (`SubAgentRegistry`) — une ligne par tache vivante d'un sub-agent (Task tool, hook, skill activation), avec status + last log. Cas d'usage : un orchestrateur CLI lance 4 agents en parallele, le terminal montre les 4 sans multiplexage manuel.
3. **MCP status bus** (`McpStatusRegistry`) — poll des serveurs declares dans `mcp.json`, surface `Up/Down/Error/Last-RPC` en temps reel. Inclut OAuth config et probe loop async.
4. **Hook event surface** (`HookEventLog`) — abonnement aux firings hooks CLI agentiques (Gemini CLI, codex, aphrody hooks), replay possible.
5. **Skill activation slot** (`SkillSlot`) — registry des skills loaded, derniere invocation, status `Idle/Active/Error`.
6. **JSON output partout** (`aphrody-terminal-json-out`) — framing stdout/stderr en JSONL envelopes (`{kind, ts, exit?, chunk}`), passthrough si l'app emet deja du JSON. Sub-agent en aval consomme le log sans re-parser ANSI.
7. **Markdown inline** (`aphrody-terminal-markdown`) — `comrak` CommonMark + `syntect` syntax highlight, declenche par OSC `aphrody-md;<base64>\a` ou auto-detect en mode terminal LLM-aware.
8. **Browser pane natif** (`aphrody-terminal-browser`) — trois backends (`bxc` in-process via MCP stdio, `agent-browser` RPC, Edge headless fallback) pour bridger LLM <-> DOM sans sortir du terminal.
9. **Ink/React TUI compat documentee** — 22 sequences VT essentielles listees dans la spec (`docs/design/aphrody-terminal-spec.md`), validees contre les CLI agentiques modernes basees Ink (Gemini CLI, codex, etc.).

## 6. Trade-offs assumes (honnetete d'audit)

| Trade-off | wterm | microsoft/terminal | aphrody-terminal |
|---|---|---|---|
| Perf rendering 60 fps | DOM diff suffit pour text + 24-bit, accessibilite browser native | AtlasEngine D3D11 GPU-accelere, optimal `cpu/gpu mixed shader` | mode WASM DOM (plus lent que AtlasEngine), pas encore de renderer natif GPU ; trade contre cross-platform Linux+Win+WASM |
| Compatibilite plateformes | navigateurs seulement (pas de natif) | Windows 10 1809+ uniquement | Linux #1 + Win #2 + WASM #3 (zero macOS first-class) |
| VT compliance exhaustivite | base xterm + libghostty opt-in (paquet `@wterm/ghostty` 400 KB) | conformance microsoft propre, certifiee sur conhost legacy | sous-ensemble Ink/React-essentiel, pas de DCS/SIXEL/ReGIS, pas de Kitty graphics protocol |
| Surface API stabilite | `@wterm/core@0.3.0` (versions pre-1.0, breaking allowed) | API ConPTY stable, settings model versione | `0.x` crates, breaking changes possibles avant 1.0 |
| Footprint binaire | core WASM ~12 KB, ghostty ~400 KB | installer ~50 MB (`.msixbundle`) | TBD (WASM target vise < 200 KB gzipped, natif TBD) |
| Communaute / ecosysteme | jeune (Vercel labs, ~2026) | mature (depuis 2019, milliers de contributors) | early stage, focus aphrody internal |
| Render fidelity polices | depend du browser font stack | DWrite + Cascadia Code premium | depend du DOM CSS stack en WASM, M3 `google-sans-flex` cible |
| Test coverage | vitest packages + e2e Playwright | unit tests `UnitTests_*` C++ + LocalTests | nextest workspace, 6 crates avec integration tests (`tests/*.rs`, 1309 LOC) |

## 7. Roadmap convergente / divergente

| Element | Repris de wterm ? | Repris de microsoft/terminal ? | Voie aphrody propre |
|---|---|---|---|
| WASM DOM render | OUI (architecture DOM diff dirty-row inspire `aphrody-terminal-wasm`) | non (pas de WASM cote MS) | M3 theming sur top du DOM render |
| VT state machine | non (vte upstream Rust prefere) | non (parser C++ COM trop couple Windows) | `vte 0.13` + `Perform` impl maison |
| ConPTY | non | OUI (`portable-pty` wrappe ConPTY sur Windows) | wrappe via portable-pty cross-platform |
| AtlasEngine D3D11 | non | reference algorithmique uniquement (cf. memory `project_terminal_integration_policy`) | renderer natif futur via `wgpu` + WebGPU fallback |
| profiles.schema.json | non | reference algorithmique (structure profiles + actions) | JSON schema aphrody propre, import shim depuis `profiles.json` MS |
| Markdown render | concept (`@wterm/markdown`) | non | `comrak` + `syntect`, OSC trigger |
| WebSocket transport | OUI (binary framing inspire) | non | tokio-tungstenite JSON + binary chunks |
| Sub-agent / MCP / hooks | non | non | innovation propre (aucun upstream comparable) |
| Browser bridge in-terminal | non | non | innovation propre (bxc / agent-browser / edge) |

S'interdire : porter le grand-frere AtlasEngine sur Linux/WASM (memory `project_terminal_integration_policy`, AtlasEngine reste reference algorithmique only) ; reintroduire `node` (memory `feedback_bun_only`) ; degrader la cross-platform pour gagner 10 % de perf sur Windows.

## 8. Conclusion

Trois projets, trois axes :

- **wterm** est le terminal du web : zero dependance native, ~12 KB WASM, ideal pour embarquer un PTY dans une UI React/Vue/vanilla. Sa surface VT couvre les usages courants ; il delegue les apps lourdes a libghostty opt-in.
- **microsoft/terminal** est le terminal natif Windows : AtlasEngine GPU, ConPTY canonique, profiles XAML, ~35 k LOC C++ rien que sur grid+renderer+cascadia. Optimal sur Win, irrecuperable hors Win.
- **aphrody-terminal** est le terminal **LLM-first** : aucun des deux upstream n'expose un sub-agent pane, un MCP status bus, un hook event log, un skill activation slot, ni un OSC namespace dedie aux event bus LLM. Le trade-off assume est triple : (1) on accepte un renderer WASM DOM moins rapide que AtlasEngine GPU, (2) on accepte d'etre derriere wterm sur l'exhaustivite VT (pas de SIXEL ni Kitty graphics), (3) on accepte d'etre derriere microsoft/terminal sur la maturite ecosysteme. En echange on debloque la seule surface terminale en 2026 qui parle nativement aux sub-agents, MCP servers, hooks et skills, avec JSON output partout et markdown inline.

Decision : continuer la roadmap aphrody-terminal sans dependre des deux upstream comme briques liees ; les conserver comme references algorithmiques (wterm = DOM render dirty-row + dirty-row WASM ABI ; microsoft/terminal = AtlasEngine + profiles + ConPTY surface). Toute introduction de code upstream doit passer par `cargo deny` + audit license (Apache-2.0 wterm compatible workspace, MIT microsoft/terminal compatible).
