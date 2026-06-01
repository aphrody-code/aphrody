# rpbey — environnement RAG / IA (référence pour aphrody)

> **But de ce dossier.** Documenter EXHAUSTIVEMENT comment le projet **rpbey**
> (communauté Beyblade « République Populaire du Beyblade », repo
> `/home/ubuntu/rpbey`) fait du RAG et de l'IA **aujourd'hui**, à destination du
> projet **aphrody** (le backend IA souverain du VPS qui, à terme, servira rpbey).
> Un dev aphrody doit pouvoir lire ces docs et comprendre 100 % du pipeline + savoir
> exactement **quel seam brancher** sur le daemon aphrody (réponse courte :
> `RPBEY_LLM_URL`, un endpoint OpenAI-compatible — cf. [`llm.md`](./llm.md)).
>
> Ces docs sont **descriptives** (état au 2026-06-01). Elles ne modifient rien dans
> rpbey. Voir aussi la doc RAG native d'aphrody : [`../RAG.md`](../RAG.md) (pipeline
> RAPTOR/GraphRAG d'aphrody, indépendant de ce qui est décrit ici).

---

## Vue d'ensemble : rpbey a TROIS systèmes IA/RAG distincts

rpbey n'a pas « un » RAG, mais trois sous-systèmes qui se recoupent partiellement.
Ne pas les confondre — ils ont des **modèles, des stores et des buts différents**.

| # | Système | Modèle(s) | Store / index | But | Doc |
|---|---------|-----------|---------------|-----|-----|
| **1** | **Chat IA « Rpbey »** (`/api/chat`) | LLM **local** llama.cpp (Llama-3.2-3B Q4) + sidecar embeddings e5-small 384d | corpus unifié Redis + vector set Redis `rpbey:search:vec` | répondre en français aux questions Beyblade des utilisateurs du site, en streaming, avec mémoire conversationnelle | [`chat.md`](./chat.md), [`llm.md`](./llm.md) |
| **2** | **Moteur de recherche** (`/api/v1/search`, page `/recherche`) | sidecar embeddings e5-small 384d (dense) + BM25F pur (lexical) | corpus unifié `rpbey:search:corpus:v1` + vector set `rpbey:search:vec` | recherche hybride sur ~15 200 entités Beyblade (catalogue, wiki, méta, combos, tournois, discussions) | [`search.md`](./search.md) |
| **3** | **Crawl + RAG X.com** (métagame Twitter) | **Gemini** (`gemini-embedding-001` 768d + `gemini-2.5-flash`) | SQLite `~/.aphrody/x-store.sqlite` (FTS5 + BLOB) + vector set Redis `tweet_embeddings` | analyser le métagame Beyblade X depuis les discussions x.com (RAG hybride VSIM+FTS5) | [`crawling-x.md`](./crawling-x.md) |

Et **un** crawler de connaissance qui alimente surtout (1) et (2) :

| Crawler | Source | Sortie | Consommé par | Doc |
|---------|--------|--------|--------------|-----|
| **crawl-fandom** | `beyblade.fandom.com` (MediaWiki API) | `apps/web/data/beyblade-knowledge.json` (~8 500 entités) | corpus de recherche (système 1 & 2) | [`knowledge.md`](./knowledge.md) |

L'**infra** (services systemd, ports, stores, fichiers hors-repo, clés) est consolidée
dans [`infra.md`](./infra.md).

---

## Schéma de flux (qui parle à quoi)

```
                         ┌──────────────────────────────────────────────┐
   Utilisateur web ─────▶│  apps/web (Next.js 16, :3002, rpbey-web)     │
   (rpbey.fr)            │                                              │
                         │  /api/chat  ──▶ services/chat.ts (prepareTurn)│
                         │     │            ├─ retrieve() hybride        │
                         │     │            │   ├─ BM25F (lib/search-rank)│
                         │     │            │   └─ dense (services/embeddings)
                         │     │            └─ buildMessages (mémoire)    │
                         │     └──▶ services/llm.ts ──HTTP─┐             │
                         │                                 │             │
                         │  /api/v1/search ─▶ rankSearch ⊕ fuseHybrid    │
                         └─────────┬───────────────┬───────┼─────────────┘
                                   │               │       │
                  ┌────────────────▼──┐  ┌─────────▼──┐  ┌─▼──────────────────────┐
                  │ Redis :6379       │  │ embed       │  │ rpbey-llm.service :8080│
                  │ rpbey:search:vec  │  │ sidecar     │  │ llama.cpp (OpenAI API) │
                  │ rpbey:search:corpus│ │ :7077       │  │ Llama-3.2-3B Q4        │
                  │ tweet_embeddings  │  │ e5-small    │  │  ◀── SEAM aphrody       │
                  └───────────────────┘  └─────────────┘  └────────────────────────┘
                                   ▲
                                   │ (alimenté hors-ligne par scripts)
   ┌───────────────────────────────┴───────────────────────────────────┐
   │ crawl-fandom.ts → beyblade-knowledge.json                          │
   │ build-search-vectors.ts → VADD rpbey:search:vec                    │
   │ scrape-*.ts (X, Reddit, WBO, shops) → apps/web/data/*.json         │
   └────────────────────────────────────────────────────────────────────┘

   X.com RAG (séparé, repo x-client, store ~/.aphrody) :
   run-targeted-crawler.ts → x-store.sqlite + Redis tweet_embeddings → run-rag.ts (Gemini)
```

---

## Le seam à connaître pour aphrody (TL;DR)

Le chat rpbey appelle son LLM via **un seul point de configuration** :

- **Variable** : `RPBEY_LLM_URL` (défaut `http://127.0.0.1:8080/v1/chat/completions`).
- **Contrat** : **OpenAI Chat Completions** (`POST` JSON `{model, messages, stream, temperature, max_tokens, top_p, repeat_penalty}` ; réponse standard ou SSE `data: {...}` / `data: [DONE]`).
- **Pour rebrancher rpbey sur aphrody** : il suffit que le daemon aphrody expose un
  endpoint OpenAI-compatible et de pointer `RPBEY_LLM_URL` dessus. Aucun autre changement
  côté rpbey. Le code (`apps/web/src/server/services/llm.ts`) est déjà écrit pour ça —
  cf. [`llm.md`](./llm.md) § « Cible long terme : daemon aphrody ».

La couche **dense d'embeddings** (sidecar e5-small 384d) est un second seam plus discret :
elle parle à `EMBED_URL` (défaut `http://127.0.0.1:7077/embed`). aphrody pourrait aussi
fournir cet endpoint — voir [`search.md`](./search.md) § sidecar.

---

## Index des fichiers

| Fichier | Contenu |
|---------|---------|
| [`chat.md`](./chat.md) | Pipeline `/api/chat` : `prepareTurn`, retrieval hybride, intents NLP, mémoire conversationnelle, streaming SSE, repli extractif |
| [`llm.md`](./llm.md) | LLM local llama.cpp : `rpbey-llm.service`, modèle, client OpenAI-compat (`generate`/`generateStream`), kill switch, cible aphrody |
| [`search.md`](./search.md) | Moteur de recherche BM25F : `lib/search-rank.ts`, corpus `global-search.ts`, `/api/v1/search`, page `/recherche`, sidecar e5, synonymes, fuzzy |
| [`crawling-x.md`](./crawling-x.md) | Crawl + RAG X.com/Twitter : session, indexation Redis/SQLite, RAG Gemini sur le métagame |
| [`knowledge.md`](./knowledge.md) | Crawler de connaissance Beyblade : `crawl-fandom.ts`, `beyblade-knowledge.json`, intégration au corpus |
| [`infra.md`](./infra.md) | Services systemd, timers, ports, fichiers hors-repo, stores (Redis, SQLite x-store, modèles llama) |

## Sources canoniques côté rpbey

- `/home/ubuntu/rpbey/CLAUDE.md` — sections « Chat IA Rpbey » et « Crawling & RAG X.com ».
- `/home/ubuntu/rpbey/docs/crawling-rag-x.md` — guide complet X.com (source de [`crawling-x.md`](./crawling-x.md)).
- `/home/ubuntu/rpbey/apps/web/AGENTS.md` — guide de l'appli web.
