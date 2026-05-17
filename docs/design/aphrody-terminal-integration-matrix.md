<!-- SPDX-License-Identifier: Apache-2.0 -->

# aphrody-terminal — integration matrix (every crate has a job)

> **Ambition** : aucun crate du workspace ne reste orphelin. aphrody-terminal
> est le **front-end intégrateur** qui consomme chaque pièce du puzzle. Si une
> crate ne sert pas à la terminal, elle prouve sa valeur ailleurs (ex. `gui`
> desktop Wry+Tao, `aphrody-translate` CLI standalone) — sinon elle a un slot
> ici.

Cette matrice est la **contrat-de-vie** des crates aphrody. Chaque tick de
développement doit pousser plus de cellules de la colonne "Consumer status"
de `⏳` vers `✅`.

## Matrice principale

| Crate                  | Rôle dans aphrody-terminal                                                                | Consumer status                                  |
|---|---|---|
| `base`                 | Primitives no_std (consommé transitivement par `aphrody-terminal-vt`)                     | ✅ via vt                                         |
| `backend`              | `aphrody-terminal-browser` réutilise `backend::network` pour HTTP fetch fallback + `backend::process` pour sub-agent process inspection (pid/name/cmdline → sub-agent pane enrichment) | ⏳ T-INT-backend |
| `a2a`                  | `aphrody-terminal-llm` publie chaque `LlmEvent` AUSSI comme A2A envelope (parallèle au broadcast tokio interne)                                                                       | ⏳ T-INT-a2a     |
| `a2a-client`           | `aphrody-terminal-llm` envoie au peer winclean via `a2a-client::http_jsonrpc`             | ⏳ T-INT-a2a     |
| `a2a-server`           | `aphrody term` héberge un endpoint A2A pour envelopes entrantes (sub-agents externes)     | ⏳ T-INT-a2a     |
| `a2a-grpc`             | Transport gRPC optionnel pour l'event bus terminal-llm (haut-débit)                       | ⏳ T-INT-a2a     |
| `a2a-pb`               | Protos partagés entre `aphrody-terminal-llm::LlmEvent` et A2A envelope                    | ⏳ T-INT-a2a     |
| `mrx-core`             | (transitif via `mrx-watch`)                                                               | ⏳ T-INT-mrx     |
| `mrx-detect`           | (transitif via `mrx-watch`)                                                               | ⏳ T-INT-mrx     |
| `mrx-audit`            | Pane "workspace health" — score audit live dans terminal                                  | ⏳ T-INT-mrx     |
| `mrx-watch`            | Stream watch → pane "Workspace activity" (live diff, file changes, build status)          | ⏳ T-INT-mrx     |
| `mrx-cli`              | Spawnable depuis terminal command palette (`Ctrl+Shift+P → scan workspace`)               | ⏳ T-INT-mrx     |
| `aphrody-translate`    | i18n labels FR/EN switchables dans command palette (`aphrody-translate::convert`)         | ⏳ T-INT-i18n    |
| `aphrody-wasm`         | `aphrody-terminal-wasm` réexporte les helpers crypto/encoding (aes_gcm, base64)           | ⏳ T-INT-wasm    |
| `ievr-tools`           | Pane optionnelle IEVR (game RE workflow, gated `--feature ievr`)                          | ⏳ T-INT-ievr (low prio) |
| `aphrody-summary`      | Pane "docs preview" — `aphrody_summary::generate()` rendu inline via `DocsPreviewPane` (publish `LlmEvent::Markdown` sur l'event bus) | ✅ T-INT-summary |
| `m3-tokens`            | Tokens couleur/typo consommés par `aphrody-terminal-wasm` (déjà câblé)                    | ✅                                                |
| `shadcn-bridge`        | Chrome terminal : header bar, tab strip, command palette, dialogs (M3 segmented buttons, list, fab) | ⏳ T-INT-chrome  |
| `a2a-ui`               | **Embedded as the "A2A coord channel" pane** — mailbox viewer JSONL en live              | ⏳ T-INT-a2a-ui  |
| `aphrody-memory`       | Session memory (JSONL + brute-force HNSW pour semantic recall des commandes passées)      | ✅ T-INT-memory (`SessionMemoryPane` in `aphrody-terminal-llm`, publishes `LlmEvent::SessionRecall`) |
| `aphrody-gateway`      | Routes les LLM calls du browser pane + sub-agent dispatch vers Cloudflare/Vercel/OpenAI-BYOK | ⏳ T-INT-gateway |
| `aphrody-mcp`          | OAuth 2.1 client pour MCP HTTP/SSE servers du pane MCP status                             | ⏳ T-INT-mcp     |
| `aphrody-voice`        | TTS pour hook event audio notifications (configurable per-event)                          | ⏳ T-INT-voice   |
| `aphrody-voice-stt`    | Push-to-talk prompt input (Ctrl+Shift+V long press → STT → inject dans terminal stdin)    | ⏳ T-INT-voice   |
| `gemini-runtime`       | **First-class "spawn Gemini CLI" panel** — bouton dans command palette → tab dédiée       | ⏳ T-INT-gemini  |
| `agui-bridge`          | Sub-agent task tree parle AG-UI protocol au renderer (compat avec écosystème agui)        | ⏳ T-INT-agui    |
| `aphrody-channels`     | Terminal hook bridge : événements hook (build done, test fail, sub-agent complete) → Slack/Telegram/Matrix | ⏳ T-INT-channels |
| `google_mcp`           | Default MCP server registered dans le pane MCP status (Google APIs surface)               | ⏳ T-INT-google-mcp |
| `gui`                  | **Out-of-scope terminal** — desktop Wry+Tao standalone, vit hors aphrody-terminal         | N/A (standalone) |
| `cli`                  | Hôte du subcommand `aphrody term` + autres                                                | ✅                                                |
| `aphrody-terminal-vt`  | VT parser, foundation                                                                     | ✅                                                |
| `aphrody-terminal-wasm`| WASM renderer, foundation                                                                 | ✅                                                |
| `aphrody-terminal-backend` | pty + WS server, foundation                                                           | ✅                                                |
| `aphrody-terminal-llm` | event bus + registries (en vol)                                                           | ⏳ in-flight                                      |
| `aphrody-terminal-browser` | bxc/agent-browser/edge bridge (en vol)                                                | ⏳ in-flight                                      |

## Crates "showcase" wiring (ordre de leverage)

L'objectif est d'ordonner les ticks d'intégration par démo-value-per-effort.

1. **T-INT-a2a-ui** : embed `a2a-ui` comme pane "Coord channel". Visible immédiat, leverage un crate WASM existant.
2. **T-INT-chrome** : `shadcn-bridge` pour le header/tabs/command palette. Polish visible.
3. **T-INT-mcp + T-INT-google-mcp** : pane MCP status alimentée par `aphrody-mcp` (OAuth) + `google_mcp` (server) — un seul tick, deux crates wired.
4. **T-INT-gemini** : "spawn Gemini CLI" tab. Démo concrète "voici ton agent dans ton terminal aphrody".
5. **T-INT-a2a + T-INT-mrx** : workspace activity pane via `mrx-watch` + A2A bridge (4 crates wired ensemble).
6. **T-INT-memory** : session semantic recall — démo "tape Ctrl+R, fuzzy-find ta commande d'il y a 3 semaines".
7. **T-INT-gateway + T-INT-voice + T-INT-channels** : tick "notifications + AI routing" (3 crates).
8. **T-INT-i18n + T-INT-summary + T-INT-wasm** : tick "polish" (3 crates).
9. **T-INT-agui** : AG-UI protocol compat (intéropérabilité écosystème).
10. **T-INT-backend** : process inspection + network fallback (peu visible mais robuste).
11. **T-INT-ievr** : low prio, optionnel (game RE workflow).

## Règles de wiring

- **Pas de stub** : chaque intégration ship réel code production, pas de
  `unimplemented!()` derrière un feature flag.
- **Chaque pane est optionnelle** via `terminal.json` `llm.<pane>: bool`.
  Default = on pour les 4 piliers (sub-agent / mcp / hook / skill), off pour
  les compléments (voice / channels / memory).
- **Toute intégration nouvelle crate** ajoute une row dans cette matrice ET
  un tick `T-INT-<slug>` dans `docs/PLAN.md` Phase T.
- **Test d'intégration obligatoire** : chaque T-INT-* crate doit avoir au
  moins un test qui prouve que la pane reçoit/affiche les events du crate.
  Ex. `cargo test -p aphrody-terminal-llm a2a_envelope_published_on_event`.

## Justification (pourquoi cet effort)

Aujourd'hui le workspace a 28+ crates dont une bonne moitié n'a qu'un
consommateur faible (un `README.md`, un test isolé). Pour le moonshot 100k
stars : un repo qui présente 28 crates dont 18 sont "academic exercises"
perd. Un repo qui présente 28 crates dont **chaque crate alimente une
feature visible du binaire showcase** gagne.

aphrody-terminal devient l'**intégration test live** du workspace entier.
Si on casse un crate, on casse une pane visible → CI le voit, l'utilisateur
le voit. Chaque ticket "ajoute une feature à aphrody-terminal" pousse de
facto sur 1-3 crates.

## Trade-offs assumés

- **Surface API agrandie** : l'utilisateur Linux qui veut "juste un terminal"
  peut désactiver toutes les panes via `terminal.json` `llm.*: false`. Le
  binaire reste lean (lazy-load des panes par feature flag).
- **Maintenance crates × consumers** : chaque crate qui bouge force un test
  côté aphrody-terminal. C'est le but — feedback loop court, pas de drift
  silencieux.
- **`gui` reste standalone** : pas de wrapping forcé. Wry+Tao a un cycle de
  vie différent (desktop natif vs WASM). Mais `gui` peut devenir l'**hôte
  desktop d'aphrody-terminal-wasm** (charge le bundle WASM dans Wry) si on
  veut une app desktop tout-en-un.
