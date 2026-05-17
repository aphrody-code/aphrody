<!-- SPDX-License-Identifier: Apache-2.0 -->

# PLAN Status Audit — 2026-05-18

> Auditeur : sub-agent "PLAN status audit" (Opus 4.7).
> Scope : `docs/PLAN.md` + `docs/PLAN-MOONSHOT.md`.
> Mission cible : 100 000 stars en 30 jours (cf. `.claude/skills/start/`).
> Mode : pure execution, zéro commit, single atomic edit sur PLAN.md.

## 1. Compteurs ⏳ avant / après

| Document | ⏳ avant | ✅ flipped | ⏳ après |
|---|---:|---:|---:|
| `docs/PLAN.md` | 19 | 8 | 11 |
| `docs/PLAN-MOONSHOT.md` | 5 | 0 | 5 |

PLAN.md : 8 lignes flippées sur 19 (42 %). PLAN-MOONSHOT.md : aucun flip
(les 5 ⏳ du tableau "1-click install" reflètent fidèlement l'état réel
des registres externes ; aucune action distribution réalisée depuis tick
précédent).

## 2. Lignes flippées ⏳ → ✅ (preuves)

| PLAN.md L# | Tâche | Preuve |
|---:|---|---|
| 140 | `cargo install aphrody` documenté | README + `docs/launch/SHOW-HN.md` mentionnent la commande (grep 3 hits) ; flip avec annotation que la publication crates.io reste pendante cf. L134 |
| 285 | T-1 `crates/aphrody-terminal-vt` | `crates/aphrody-terminal-vt/src/lib.rs` 703 l. (vte + ScreenBuffer + SGR 16-color) — commit `9355ddc57` |
| 286 | T-1 `crates/aphrody-terminal-wasm` | `crates/aphrody-terminal-wasm/src/lib.rs` 361 l. + `coord_pane.rs` |
| 287 | T-1 `crates/aphrody-terminal-backend` | `crates/aphrody-terminal-backend/src/lib.rs` 287 l. + `tests/it.rs` |
| 289 | T-3 `crates/aphrody-terminal-llm` | lib.rs + 6 modules (`sub_agent.rs`, `mcp.rs`, `hook.rs`, `skill.rs`, `task.rs`, `osc.rs`) + tests it.rs |
| 293 | T-6b `crates/aphrody-terminal-browser` | lib.rs 306 l. + 4 backends (`bxc.rs`, `agent_browser.rs`, `edge.rs`, `mod.rs`) + `osc.rs` + `proto.rs` |
| 294 | T-7 `aphrody term` CLI subcommand | `crates/cli/src/commands.rs:1411-1445` + dispatch `main.rs:238` (`Commands::Term { addr, shell, cwd }`) — commit `77ddbff85` |
| 298 | T-9 `packages/aphrody-jsx` | `packages/aphrody-jsx/src/reconciler.ts` 455 l. + jsx-runtime + 6 components + 6 hooks + 3 test files + 2 examples |

## 3. Top-10 ⏳ restants — priorisés mission

Ranking par leverage sur arc 100 k stars / 30 j (D+3 README↔code, D+7 demo
gif, D+10 CI green, D+14 post, D+15 Show HN, D+18 cross-post, D+21 user
feature).

| Rang | PLAN.md L# | Tâche | Criticité | Justification |
|---:|---:|---|---|---|
| 1 | 138 | Premier tag `v*` poussé | **CI-blocker** | Sans tag, `release.yml` (8 targets + SHA-256 + SBOM) ne s'exécute jamais → aucun binaire downloadable pour Show HN D+15. |
| 2 | 134 | crates.io publication `aphrody` | **mission-direct** | Pré-requis dur de L140 + ligne moonshot L598 (`cargo install` doit fonctionner D+11). |
| 3 | 296 | Demo gif Claude Code in aphrody-terminal | **mission-direct** | Hero asset D+8-15 — moonshot R2 explicite : "no gif → Show HN top reply 'looks like vapor' kills the thread". |
| 4 | 297 | Audit wterm vs microsoft/terminal vs aphrody-terminal | **mission-direct** | Justification produit pour Show HN — comparable au benchmark table de `bxc` (P11). |
| 5 | 295 | wasm demo HTML `aphrody-terminal-demo.html` | **mission-direct** | Permet "open in browser" preview — leverage P4 (hero demo). |
| 6 | 288 | T-2 VT extensions Ink essentials (alt-screen, OSC 52, mouse 1006, …) | **mission-direct** | Bloque l'usage Ink/Claude Code dans aphrody-terminal — toute la promesse "LLM-first terminal" repose dessus. |
| 7 | 290 | T-4 `aphrody-terminal-markdown` (comrak + syntect) | hygiene | Sans rendu markdown inline, le pitch "markdown rendu inline" du spec est unfulfilled. |
| 8 | 292 | T-6 `aphrody-terminal-config` (terminal.json schema + import shims) | hygiene | Discoverability config + onboarding ; loader `mcp.json` existe déjà côté `terminal-llm` mais pas le schema strict. |
| 9 | 291 | T-5 `aphrody-terminal-json-out` (JSONL envelopes) | hygiene | Promesse "JSON output partout" du spec. |
| 10 | 136 | Homebrew tap `aphrody-code/tap` publié | mission-direct | Moonshot L601 D+12. Formula locale existe (`packaging/homebrew/aphrody.rb`) ; il manque le repo `aphrody-code/homebrew-tap`. |

⏳ restants non-top-10 :
- L56 PPA Launchpad (nice-to-have — Ubuntu PPA aurait du sens post-snap).
- L202 vet fmt drift (hygiene — agent #21 a noté side-effects, à traiter avec soin).
- L299 T-10 `crates/aphrody-tui` (long-terme — explicitement "canonical long-term" dans la table).

## 4. Top-5 contradictions PLAN ↔ code

| # | PLAN dit | Code montre | Verdict |
|---:|---|---|---|
| 1 | L285-287 `⏳ (rust-architect en vol)` | Crates VT/wasm/backend déjà mergés dans commit `9355ddc57` (feat terminal-llm-first) | PLAN stale d'~24 h ; flip appliqué. |
| 2 | L294 `aphrody term` ⏳ | `commands.rs:1411` `pub async fn cmd_term(args: TermArgs)` complet + branch `Some(Commands::Term { addr, shell, cwd })` dans main.rs:238 | PLAN stale ; flip appliqué (commit `77ddbff85`). |
| 3 | L298 `packages/aphrody-jsx` ⏳ | Package complet : reconciler.ts (455 l.), jsx-runtime, 6 components (Box/Text/Newline/Static/Transform/Spacer), 6 hooks, tests + examples | PLAN stale ; flip appliqué. |
| 4 | L289 T-3 `terminal-llm` ⏳ | Crate avec 6 modules + `default_server_specs` feature + `load_mcp_json` + tokio broadcast bus | PLAN stale ; flip appliqué. |
| 5 | L140 `cargo install aphrody documenté ⏳ (besoin de publier...)` | Confusion 2 concepts : "documenté" (texte) vs "fonctionnel" (publication). Le texte existe (README+SHOW-HN) ; la publication est L134 | Flip avec annotation cross-ref vers L134. |

## 5. PLAN-MOONSHOT.md (§6 — 50-item punch list)

Les 50 items du punch list sont au format `- [ ] D+N` (checkbox markdown,
PAS ⏳). Aucun n'a été coché historiquement (revue du tableau L585-635 :
50 checkboxes vides). Hors scope de cet audit (le scope explicite est
les lignes ⏳, et le punch list n'utilise pas ce marqueur). À recommander
au prochain tick : audit séparé pour cocher les items déjà shippés
(au moins D+1 readme positioning, D+3 mrx shield, et l'item devcontainer
qui est déjà en ✅ dans PLAN.md L206).

## 6. Verdict global

**Mission-direct (Phase Q + Phase T)** : ~70 % completion.
- Phase Q (D+7→D+15 polish) : 100 % shippé (tous ✅ avant audit).
- Phase T (Terminal LLM-first) : 7/16 ✅ (44 %) après flip — la moitié
  fondations (VT/wasm/backend/LLM/browser/CLI subcommand/JSX) shippée ;
  l'autre moitié (extensions VT, markdown, json-out, config, demo HTML,
  demo gif, audit doc, ratatui DSL) reste.

**Hygiene (Phase P-Distribution + fond)** : ~50 % completion.
- Crates.io publication, premier tag `v*`, Homebrew tap, PPA Launchpad
  toutes pendantes — bloque le moonshot D+11/D+12/D+15.

**Recommandation prochain tick** : lancer un sub-agent "tag-and-release"
qui (1) bump version → 1.0.0 dans workspace.package, (2) publish ladder
crates.io topologique via `scripts/verify-publish.sh`, (3) tag `v1.0.0`
+ push, (4) attendre `release.yml` artefacts, (5) ouvrir PR
`aphrody-code/homebrew-tap`. Cela débloque 4 ⏳ d'un coup et arme l'arc
D+11→D+15 du moonshot.

---

*Audit produit par sub-agent Opus 4.7, single PLAN.md edit, zéro commit.*
