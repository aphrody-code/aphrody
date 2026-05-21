<!-- SPDX-License-Identifier: Apache-2.0 -->
# Évaluation : Obscura comme moteur headless du pilier R4 (scraping)

**Source** : <https://github.com/h4ckf0r0day/obscura> (évalué 2026-05-21).
**Statut** : recommandation de recherche. Aucune dépendance ajoutée — décision
d'intégration via façade binaire (voir §4).

## 1. Ce qu'est Obscura

Moteur de navigateur headless **100 % Rust**, **Apache-2.0** (compatible
aphrody), pensé pour l'automatisation à l'échelle, pas le desktop browsing.

- **Workspace** 7 crates : `obscura-{dom,net,browser,cdp,js,mcp,cli}`, v0.1.0,
  edition 2021, Rust 1.75+. **Non publié sur crates.io** (consommé en binaire).
- **V8 embarqué** (JS réel ; build initial ~5 min puis caché ; `--v8-flags`
  façon `--js-flags` Chromium pour le heap).
- **Serveur CDP** (`serve`, WebSocket :9222 par défaut) couvrant Target, Page,
  Runtime, DOM, Network, Fetch, Storage, Input + un domaine `LP`
  DOM→Markdown. Drop-in pour Puppeteer/Playwright.
- **MCP** (`mcp`, stdio par défaut ou HTTP `--http --port`) exposant 9 outils
  `browser_*` (navigate, snapshot, click, fill, type, press_key,
  select_option, evaluate, wait_for, network_requests, console_messages,
  close) — surface alignée sur le Playwright MCP.
- **CLI** : `serve`, `fetch <URL>` (formats html/text/links/markdown/original,
  `--eval`, wait strategies), `scrape <URL...>` (parallèle, concurrence
  configurable), `mcp`.
- **Stealth** (`--features stealth`) : randomisation de fingerprint
  (GPU/screen/canvas/audio/battery), `navigator.userAgentData` réaliste,
  `event.isTrusted=true`, `navigator.webdriver` undefined, masquage de
  fonctions natives, blocklist tracker de 3 520 domaines.
- **Empreinte** : ~30 Mo RAM vs 200+ Mo Chrome, ~85 ms vs ~500 ms de chargement.
- **Install** : releases GitHub (Linux x86_64/ARM64, macOS, Windows), AUR
  (`obscura-browser`), Docker (`h4ckf0r0day/obscura`).

## 2. Le manque qu'il comble

Depuis la suppression de `crates/bxc-engine` (2026-05-21), aphrody n'a **plus
aucun rendu JavaScript** : `aphrody scrape` (`crates/cli/src/commands.rs`)
parse du HTML statique via `reqwest` + `scraper::Html`. Tout le pilier **R4**
de `docs/PLAN.md` est donc orphelin de moteur :

| Item R4 | Besoin | Couverture Obscura |
|---|---|---|
| R4.1 | spoofing fingerprint (curl-impersonate `chrome146`) | partiel — stealth JS-level (complémentaire au TLS de curl-impersonate, pas un substitut JA4) |
| R4.2 | `aphrody scrape --concurrent N --rate-limit-ms K` | **oui** — `scrape <URL...>` parallèle |
| R4.5 | tool MCP `*_batch_scrape(urls, selector, concurrent)` | **oui** — MCP `browser_*` + `scrape` (proxiable via `aphrody_mcp_call`) |
| R4.7 | anti-detect, rotation User-Agent | **oui** — stealth + UA override + proxy |

Obscura apporte en plus le rendu JS réel (V8) que bxc-engine fournissait via
Chromium, mais à ~1/7 de l'empreinte mémoire.

## 3. Frictions

- **Edition 2021** (aphrody = 2024), **non publié crates.io** → vendoring en
  membre du workspace est exclu (politique lockfile-only §5 + build V8 ~5 min
  qui plomberait `cargo ci-offline`).
- **V8** = toolchain lourde (gn/ninja) ; ne doit jamais entrer dans le graphe
  de compilation par défaut.
- **Cross-platform** : releases Linux #1 + Windows #2 OK ; pas de cible wasm
  (#3) — cohérent, un moteur de navigateur n'est pas une lib wasm.

## 4. Décision d'intégration recommandée — façade binaire externe

Reproduire le pattern éprouvé `gemini_runtime::resolve_bin()`
(`crates/gemini-runtime/src/lib.rs:430`, env `APHRODY_GEMINI_BIN` > sibling de
`current_exe()` > PATH), déjà utilisé pour gemini / bxc / n2b :

1. `aphrody scrape --engine obscura` (ou auto si binaire présent) spawn le
   binaire `obscura` résolu via **`$APHRODY_OBSCURA_BIN` > sibling > PATH**,
   sous-commande `scrape`/`fetch`, parse le JSON/markdown de sortie.
2. **Zéro dépendance de compilation**, zéro V8 dans `cargo ci-offline`, zéro
   contamination de licence (Apache-2.0 de toute façon).
3. Fallback gracieux : si `obscura` absent, garder le chemin `reqwest` statique
   actuel (HTML non-JS) et signaler l'absence du moteur dans la sortie.
4. Surface MCP : `aphrody-mcp` peut déjà proxifier le serveur MCP Obscura via
   l'outil existant `aphrody_mcp_call` — aucun code neuf requis pour le volet
   AI-agent.

Anti-pattern à éviter : ajouter `obscura-*` comme membres du workspace ou deps
git (build V8, edition mismatch, lock bloat).

## 5. Prochaines étapes actionnables (si retenu)

- [ ] Crate `obscura-runtime` (≈ `gemini-runtime`) : `resolve_bin()` + wrapper
      typé autour de `obscura fetch`/`scrape` (JSON out), erreurs structurées.
- [ ] Wire `aphrody scrape --engine {static|obscura}` dans
      `crates/cli/src/commands.rs` (défaut auto-détecté).
- [ ] Flip R4.2/R4.5/R4.7 de `docs/PLAN.md` une fois le wrapper + smoke verts
      (`obscura fetch https://tls.peet.ws --format json`).
- [ ] Garder R4.1 (curl-impersonate, spoofing JA4 TLS) distinct : Obscura
      stealth opère au niveau JS/DOM, pas TLS — les deux sont complémentaires.

## 6. Verdict

**Recommandé** comme moteur headless du pilier R4, via **façade binaire
externe** (jamais en dépendance de compilation). License, langage et surface
(CDP + MCP + stealth + scrape parallèle) sont un fit direct pour le besoin
laissé vacant par bxc-engine, à une fraction de l'empreinte de Chromium.
