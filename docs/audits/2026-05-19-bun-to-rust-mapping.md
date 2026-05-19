# Audit Strict : Cartographie de Migration TS/Bun vers Rust Natif (Workspace Aphrody)

> Date: 2026-05-19
> Conformité: Règle 100% Rust (`PLAN_RUST_ONLY.md`)

Ce document dresse l'inventaire exhaustif des anciens packages TypeScript/Bun (présents dans `packages/` ou récemment purgés) et leur mapping direct vers l'écosystème modulaire Rust de `crates/`. 
Aucune disgression. Uniquement la structure et la destination des responsabilités techniques.

## 1. Écosystème Web Scraping & Automatisation
| Composant TS/Bun (Origine) | Crate Rust (Destination) | Statut / Observation |
|---|---|---|
| `packages/bxc` (Daemon & Driver) | `crates/bxc-engine` | **[PORTÉ]** Serveur HTTP asynchrone Rust natif pour le scraping. |
| (Puppeteer / Playwright ts) | `crates/obscura-*` (`cdp`, `dom`, `js`, `net`, `browser`) | **[PORTÉ]** Pilotage natif du navigateur via le protocole CDP en Rust sans surcouche Node. |

## 2. Écosystème Agentique & CLI (Fork Gemini)
| Composant TS/Bun (Origine) | Crate Rust (Destination) | Statut / Observation |
|---|---|---|
| `packages/gemini-cli` (Fork amont) | `crates/aphrody-chat` | **[PORTÉ]** Orchestrateur de la boucle de discussion, hooks, events. |
| `packages/gemini-cli/core` | `crates/gemini-runtime` | **[PORTÉ]** Interaction avec le binaire/backend Gemini. |
| `packages/gemini-cli/tools` | `crates/aphrody-tools` | **[PORTÉ]** Outils intégrés (read/write/shell/search). Remplacés par des appels Rust natifs via `aphrody-shell` et `aphrody-sandbox`. |
| `packages/gemini-cli/mcp` | `crates/aphrody-mcp` | **[PORTÉ]** Serveur et client MCP natif (complété par `google_mcp`, `obscura-mcp`). |
| `packages/gemini-cli/memory` | `crates/aphrody-memory` & `aphrody-session` | **[PORTÉ]** Gestion de l'historique et des fenêtres de contexte. |

## 3. Écosystème Interfaces Web & UI (Next.js / React)
| Composant TS/Bun (Origine) | Crate Rust (Destination) | Statut / Observation |
|---|---|---|
| `packages/aphrody-jsx` | `crates/aphrody-react-reconciler` | **[PORTÉ]** Réécriture du reconciler React (Fibers) en Rust pur `no_std` compilable pour WASM. |
| `packages/ui` (Shadcn TS) | `crates/shadcn-bridge` & `crates/aphrody-tui` | **[REMPLACÉ]** Migration vers les interfaces TUI (Terminal UI) natives ou ponts web. |
| `packages/gemini` (Clone Next.js) | `crates/aphrody-wgpu-material` & `crates/aphrody-terminal-wasm` | **[EN COURS]** Migration planifiée pour compiler l'UI en WebAssembly assisté de WebGPU. |
| `packages/a2ui` & `material-web` | `crates/agui-bridge` & `crates/aphrody-design-material` | **[PORTÉ]** Interopérabilité Material Design native au lieu des stubs JS. |

## 4. Écosystème N2B (Migration Node-to-Bun)
| Composant TS/Bun (Origine) | Crate Rust (Destination) | Statut / Observation |
|---|---|---|
| `packages/n2b` (Outil principal) | `crates/n2b-core`, `crates/n2b-cli` | **[PORTÉ]** Logique centrale de refactoring portée en Rust. |
| `packages/n2b/scanners` | `crates/n2b-scanners`, `crates/n2b-rules` | **[PORTÉ]** Moteur d'analyse AST réécrit pour la vélocité. |
| `packages/n2b/reporting` | `crates/n2b-report`, `crates/n2b-ai` | **[PORTÉ]** Générateurs de rapports natifs assistés par LLM. |

## 5. Écosystème MRX (Monorepo Mapper)
| Composant TS/Bun (Origine) | Crate Rust (Destination) | Statut / Observation |
|---|---|---|
| `packages/mrx` (Scraping local TS) | `crates/mrx-core`, `crates/mrx-cli` | **[PORTÉ]** Logique de mapping monorepo convertie en Rust (`mrx scan`). |
| `packages/mrx/watchers` | `crates/mrx-watch`, `crates/mrx-detect` | **[PORTÉ]** Détection de changements (`notify`) en Rust natif. |

## 6. Scripts & Extensions Internes
| Composant TS/Bun (Origine) | Crate Rust (Destination) | Statut / Observation |
|---|---|---|
| `packages/aphrody-skills` | `crates/aphrody-skills-runtime` & `forge` | **[PORTÉ]** Moteur d'exécution des fichiers `SKILL.md` et validation JSON Schema natif. |
| `scripts/*.ts` (Utilitaires) | `crates/aphrody-xtask` | **[PORTÉ]** Architecture `cargo xtask` reprenant les 20 scripts métier. |
| `packages/google-core` | `crates/backend`, `crates/aphrody-gateway` | **[REMPLACÉ]** Déporté sur l'implémentation backend cross-platform. |
| Coordination A2A (TS/Bun) | `crates/a2a-*` (`server`, `client`, `grpc`) | **[PORTÉ]** Protocole A2A et gestion des daemons réécrits en binaires Rust. |

## Conclusion de l'Audit
La quasi-totalité de l'arbre `packages/` historique a été déconstruite et scindée en **97 crates Rust spécialisées**. 
Le seul survivant à migrer est `packages/gemini` (Next.js) qui fusionnera vers `crates/aphrody-wgpu-material` pour un rendu client WASM 100% Rust. Tout le reste est FAIT ou remplacé structurellement.
