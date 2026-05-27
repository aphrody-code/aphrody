<!-- SPDX-License-Identifier: Apache-2.0 -->
# Le framework Wiz de Google (via gemini.google.com/app) — leçons pour material-web

Investigation (2026-05-23) du framework web interne de Google **Wiz** (client) +
**Boq** (serveur) tel qu'utilisé par `gemini.google.com/app`, et ce qu'on en
retient pour notre `packages/material-web`. Reconnaissance via `bin/bxc.exe`
v0.3.1 (`recon`/`detect`) + lecture des marqueurs du shell HTML public (sans
auth, sans exfiltration), recoupée avec l'intel déjà en dépôt
(`crates/gemini-web/src/boq.rs`, `docs/python/gemini-web-app-analysis.md`,
`docs/research/gemini-web-{protocol,cdp-exploitation}.md`).

## Preuves recueillies

- `bxc recon https://gemini.google.com/app` → HTTP 200, **660 KB** de shell,
  Server `ESF`, CDN « Google Frontend », CSP à 40 hôtes (`*.gstatic.com`,
  `gemini.gstatic.com`, `*.googleapis.com`…).
- Marqueurs dans le shell : `WIZ_global_data` ×3, `AF_initDataCallback` ×3,
  `boq_assistant-bard-web-server` ×1, `BardChatUi` ×17. Les attributs
  `c-wiz`/`jsaction`/`jscontroller` n'apparaissent **qu'après hydratation**
  (app authentifiée), pas dans le shell statique.
- `crates/gemini-web/src/boq.rs` encode déjà le protocole : build
  `boq_assistant-bard-web-server_20260511.16_p20`, `f.req=[[[rpc_id, inner,
  null, "generic"]]]&at=<SNlM0e>`, réponse préfixée `)]}'` + chunks
  longueur-préfixés `[["wrb.fr",…]]`.

## Wiz — framework client

- **Bootstrap `WIZ_global_data`** : un objet JSON inline en tête de page porte
  les jetons et métadonnées globales (dont `SNlM0e` = jeton anti-forgery `at`,
  `cfb2h` = build label `bl`). Notre client cookie le lit déjà (`gemini_web`).
- **`c-wiz`** : éléments custom rendus côté serveur, **hydratés paresseusement**
  par zones — chaque `c-wiz` est une frontière de chargement/rendu indépendante.
- **`jsaction`** : délégation d'événements **déclarative** par attribut
  (`jsaction="click:Xyz; keydown:Abc"`) ; un dispatcher unique au document route
  vers les contrôleurs, plutôt que des listeners par nœud.
- **`jscontroller`/`jsname`/`jsmodel`** : liaison comportement↔DOM par attribut
  (le contrôleur JS est chargé à la demande).
- **`AF_initDataCallback`** : réponses de data-service **inlinées** dans la page
  (RPC `batchexecute` pré-jouées) → premier rendu sans aller-retour réseau.

## Boq — framework serveur

- Routeur `batchexecute` partagé (Gemini = `BardChatUi`, NotebookLM = même
  surface). RPC identifiés par id, payload positionnel (slots `null` = options
  modèle/pièces-jointes/outil non remplies par le client minimal).
- Anti-XSSI `)]}'`, enveloppes `wrb.fr` + canaux latéraux (`{"11":[titre]}`,
  tokens de continuation, echo cid). Déjà parsé par `boq.rs`/`gemini_web`.

## Ce qu'on en retient pour material-web

Notre stack atteint les mêmes objectifs avec des primitives **standard** (pas le
framework propriétaire) :

| Pattern Wiz | Notre équivalent (standard, déjà en place) |
|---|---|
| `c-wiz` (custom elements server-rendered) | Web components Lit (`md-*`), hydratables, SSR via `@lit-labs/ssr` |
| `jsaction` (délégation d'événements déclarative) | événements DOM déclaratifs dans les templates Lit (`@click=`), `CustomEvent` composés |
| Hydratation paresseuse par zone | imports par composant (`aphrody-components.ts`), `content-visibility`, virtual-scroll (`md-virtual-scroller`) |
| `AF_initDataCallback` (données inlinées) | props data-driven (`.columns`/`.rows` de `md-table`, `.items` de `md-virtual-scroller`) |
| `WIZ_global_data` bootstrap | thème via tokens CSS `--md-sys-*` injectés une fois (`aphrody design tokens`) |
| Top-layer overlays maison | **Popover API** native (cf. `md-snackbar`) |

**Adoptions concrètes recommandées** (non bloquantes) :
1. **Délégation d'événements** pour les conteneurs à N enfants (`md-list`,
   `md-tree`, `md-table`) — un listener au conteneur plutôt que par item, façon
   `jsaction` (réduit le coût mémoire sur grandes listes).
2. **Rendu data-driven + virtual-scroll** déjà en place pour les grandes
   surfaces (table/tree/list) — c'est l'équivalent du lazy-hydration Wiz.
3. **Bootstrap unique des tokens** (une feuille `--md-sys-*` au document) plutôt
   que par composant — déjà le modèle de la fusion `aphrody design tokens`.

On **ne réimplémente pas Wiz** (propriétaire, couplé à Boq/`jsaction`) : nos
primitives plateforme (web components, Popover, scroll-driven, container
queries, view transitions) couvrent les mêmes besoins de façon portable et
Baseline. L'intel Boq reste utile côté **client keyless** (`crates/gemini-web`),
pas côté UI.

## Pointeurs
- Client Boq Rust : `crates/gemini-web/src/{boq,client,auth}.rs`.
- Analyse app : `docs/python/gemini-web-app-analysis.md`,
  `docs/research/gemini-web-protocol.md`.
- Outil recon : `bin/bxc.exe` (`recon`/`detect`/`scrape`/`har`).
- Parité composants : `docs/design/angular-material-parity.md`.
