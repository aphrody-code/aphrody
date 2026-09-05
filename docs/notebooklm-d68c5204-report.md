<!-- SPDX-License-Identifier: Apache-2.0 -->
# Rapport surface NotebookLM — notebook `d68c5204`

Sonde read-only de la surface complète du notebook NotebookLM
`d68c5204-a2b3-4864-8f65-278844ade83d`
(<https://notebooklm.google.com/notebook/d68c5204-a2b3-4864-8f65-278844ade83d>),
via le client Rust in-tree `crates/notebooklm` (Boq RPC pur-HTTP), pilotée le
2026-05-21. Toutes les données ci-dessous proviennent d'appels live réels
(aucune fabrication). Les secrets de session sont masqués.

Outil de sonde : `crates/notebooklm/examples/nblm_surface.rs` (read-only) ;
dumper brut de re-mapping wire : `crates/notebooklm/examples/nblm_raw.rs`.

## 1. Auth bootstrap

- **Flavour** : cookie jar Google (`NOTEBOOKLM_COOKIES`, format Cookie-Editor),
  parsé par `notebooklm::auth::Auth::from_chromium_export`. Cookies requis
  présents : `__Secure-1PSID=<redacted>`, `SAPISID=<redacted>` (+ `SID`,
  `HSID`, `SSID`, `APISID`, `__Secure-3PSID`, `*SIDCC`, `*SIDTS`, `NID`).
- **Cookie décisif** : `OSID` / `__Secure-OSID` scopés `notebooklm.google.com`.
  Sans eux, le bootstrap de page renvoie `302 → accounts.google.com/ServiceLogin`
  (`osid=1`). Avec eux : `200`, page de 308 994 octets.
- **Tokens de session** scrapés du blob `WIZ_global_data` de la page loggée :
  - `NOTEBOOKLM_AT_TOKEN` = `SNlM0e` (42 c) — masqué.
  - `NOTEBOOKLM_BL_TOKEN` = `cfb2h` (41 c) — masqué.
  - `NOTEBOOKLM_FSID_TOKEN` = `FdrFJe` (19 c) — masqué.

**Verdict : FAIT.**

## 2. Métadonnées notebook

| Champ | Valeur live |
|---|---|
| `id` | `d68c5204-a2b3-4864-8f65-278844ade83d` |
| `title` | **Google I/O 2026 : L'Ère de l'IA Agentique et Antigravity** |
| emoji | (robot) |
| `source_count` | **63** |
| Présent dans `list_notebooks` | oui (`list_count = 2`, le compte contient aussi un notebook vide `99a3004a…`) |

**Verdict : FAIT.**

## 3. Sources (63)

Répartition par type : **url = 55, file (PDF) = 4, you_tube = 3, text = 1**.
Chaque source expose `id`, `title`, `kind`, `url`, `word_count` (live).

Thèmes dominants du corpus :

- **Gemini 3.5 Flash** : model cards Google DeepMind, benchmarks (LLM-Stats,
  VERTU « Flash vs Pro »), évals méthodologie (PDF), lancement (TNW,
  SearchEngineLand « Google Search now powered by Gemini 3.5 Flash »), Gemini API docs.
- **Google Antigravity / AGY CLI** : blog `google-io-2026`, SDK (`introducing`,
  PyPI `google-antigravity`, GitHub `antigravity-sdk-python`), docs
  (`/docs/home|mcp|plugins|cli-using`), changelog, releases, issues
  `antigravity-cli`, article « Google Moves Gemini CLI Into Antigravity CLI »,
  guide sfeir.dev, README MCP du SDK.
- **Android 17** : Beta 3 (Reddit), Desktop Mode (YouTube), hardware
  requirements, AOSP build/setup, Gemini Nano, ROM A14+ (Ubuntu 24.04).
- **Material Design** : « Material is Compose-first », « What's new at I/O 26 »,
  commits `material-web`, Gemini AI Visual Design.
- **Divers Google AI** : Progress Report (PDF), Guide to Generative AI Controls
  (PDF), « The Gemini app becomes more agentic », Google Cloud release notes.

Échantillon (id/url réels) :

| # | kind | titre | url / vidéo |
|---|---|---|---|
| 3 | you_tube | Android 17 Desktop Mode is Finally WORTH Using! | `youtube.com/watch?v=MewJC7DQhY0` |
| 9 | url | Antigravity Agent \| Gemini API | `ai.google.dev/gemini-api/docs/antigravity-agent` |
| 17 | url | Gemini 3.5 Flash - Model Card - DeepMind | `deepmind.google/models/model-cards/gemini-3-5-flash/` |
| 52 | file | Model Evaluation Gemini 3.5 Flash (PDF) | `deepmind.google/models/evals-methodology/gemini-3-5-flash` |
| 58 | url | Using AGY CLI - Antigravity Documentation | `antigravity.google/docs/cli-using` |

Note : plusieurs sources apparaissent en double (deux ingestions à des
timestamps différents) — c'est l'état réel côté NotebookLM, non un artefact du parseur.

**Verdict : FAIT** (énumération + métadonnées). Le détail par-source
(`summary`/`content`) est **INCOMPLET** — voir §6.

## 4. Threads de chat

- 1 thread : `53ac8730-df6f-4e5f-9d55-d5dfbe15c36f`.
- Liste obtenue via `list_chat_threads` (`get_artifacts_filtered` pour les artifacts).

**Verdict : FAIT** (liste). La réponse du message sonde est **INCOMPLET** — §6.

## 5. Artifacts (5)

| kind | titre | id |
|---|---|---|
| audio | Gemini Flash et les agents locaux | `48da60cd-…` |
| video | Architecture Web Moderne | `5a9cd1ab-…` |
| infographic | Écosystème technologique futuriste de 2026 | `3828b099-…` |
| data_table | Comparaison des Benchmarks des Modèles d'IA Gemini 3 | `9ddd8039-…` |
| quiz | Android Carte | `eaf396f0-…` |

Seul l'artifact audio expose des `source_ids` (URLs `lh3.googleusercontent.com`
de l'overview audio) ; les autres renvoient `source_ids: []` côté wire.

**Verdict : FAIT** (liste + métadonnées). Pas de génération déclenchée
(non destructif).

## 6. Lacunes (INCOMPLET) et corrections de production appliquées

### Bugs réels corrigés dans le crate (cette session)

1. **`transport.rs:118`** — `HeaderName::from_static("Cookie")` panique
   (`from_static` exige des noms minuscules). Corrigé en `HeaderName::from_bytes`
   (case-insensitive, `Result`). Sans ce fix, **toute** requête authentifiée
   panique. **FAIT.**
2. **`notebooks.rs::list_notebooks`** — `id` et `title` lus aux mauvais index
   (layout wire obsolète : le parseur lisait `id=items[0]` (=titre) et
   `title=items[2]` (=id)). Réalité : `[titre, [sources], id, emoji, …]`.
   Corrigé → `in_list` matche désormais. **FAIT.**
3. **`notebooks.rs::get_notebook`** — parseur ne déballait pas le niveau
   `envelope[0]` et lisait des index faux pour titre + sources (renvoyait
   titre vide, 0 source). Réalité : `envelope[0] = [titre, [source…], id, …]`,
   source = `[[id], titre, [meta…]]` avec `meta[4]`=type, `meta[7][0]`/`meta[5][0]`=url.
   Corrigé → 63 sources énumérées avec id/titre/kind/url/word_count. **FAIT.**
4. **`notebooks.rs::get_notebook`** (URL YouTube) — `meta[7]` vaut JSON `null`
   pour les vidéos, ce qui bloquait le fallback `or_else` vers `meta[5][0]`.
   Corrigé avec `.filter(|v| v.is_array())`. **FAIT.**

### Lacunes restantes (drift wire non corrigé)

- **`get_source_summary` / `get_source_content`** : renvoient
  `parse failure: no wrb.fr envelope in response`. Ces RPC
  (`VfAZjd` summary, content) utilisent un wrapper de réponse différent du
  `)]}'` + `wrb.fr` attendu par `boq::strip_xssi`/le parseur de chunks.
  Re-mapping nécessaire (dump brut à faire comme pour `get_notebook`).
  **INCOMPLET.**
- **`send_message`** (chat) : retourne `text: ""`, `thread_id: ""` même avec
  les 63 `source_ids` passés. Le parseur de flux chat
  (`chat::parse_chat_stream`) ne matche pas le format de stream courant.
  **INCOMPLET.**

## 7. Reproduire

```powershell
# 1. Cookies (export Cookie-Editor des cookies .google.com + OSID notebooklm) :
$env:NOTEBOOKLM_COOKIES = (Get-Content cookies.json -Raw)
# 2. Tokens depuis la page loggee (WIZ_global_data) :
$env:NOTEBOOKLM_AT_TOKEN = '<SNlM0e>'
$env:NOTEBOOKLM_BL_TOKEN = '<cfb2h>'
$env:NOTEBOOKLM_FSID_TOKEN = '<FdrFJe>'
# 3. Sonde :
cargo run -q -p notebooklm --example nblm_surface -- d68c5204-a2b3-4864-8f65-278844ade83d
```

## 8. Classification finale

| Section | Verdict |
|---|---|
| 1. Auth bootstrap | FAIT |
| 2. Métadonnées notebook | FAIT |
| 3. Sources (énumération + métadonnées) | FAIT |
| 3bis. Sources (summary/content détaillé) | INCOMPLET (drift wire `wrb.fr`) |
| 4. Threads de chat (liste) | FAIT |
| 4bis. Réponse chat (send_message) | INCOMPLET (parseur de stream) |
| 5. Artifacts (liste + métadonnées) | FAIT |
| 6. Bugs de production | 4 corrigés (FAIT) |
