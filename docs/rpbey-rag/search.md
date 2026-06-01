# Moteur de recherche rpbey — BM25F ⊕ dense (hybride RRF)

Le moteur de recherche de rpbey.fr indexe **~15 200 entités Beyblade** (catalogue produits,
pièces, combos, tournois, bladers, méta, anime, frames, lexique, discussions X/Reddit, pages,
wiki toutes saisons) et les classe par un **BM25F** lexical fusionné en **RRF** avec une couche
**dense** (embeddings e5-small 384d). C'est aussi le `retrieve()` du chat IA (cf. [`chat.md`](./chat.md)).

## Fichiers

| Rôle | Chemin absolu |
|------|---------------|
| Ranker BM25F + fusion (pur, client+serveur) | `/home/ubuntu/rpbey/apps/web/src/lib/search-rank.ts` |
| Corpus unifié (assemblage ~15 sources) | `/home/ubuntu/rpbey/apps/web/src/server/services/global-search.ts` |
| Cache du corpus (Redis + memo) | `/home/ubuntu/rpbey/apps/web/src/server/services/search-corpus.ts` |
| Couche dense (embeddings/VSIM) | `/home/ubuntu/rpbey/apps/web/src/server/services/embeddings.ts` |
| Route API | `/home/ubuntu/rpbey/apps/web/src/app/api/v1/search/route.ts` |
| Sidecar d'embeddings (service Bun isolé) | `/home/ubuntu/rpbey/apps/embed-sidecar/server.ts` |
| Indexation vectorielle (script) | `/home/ubuntu/rpbey/apps/web/scripts/build-search-vectors.ts` |
| Invalidation du corpus (script) | `/home/ubuntu/rpbey/apps/web/scripts/refresh-search-corpus.ts` |
| Lexique d'alias communautaires (généré) | `/home/ubuntu/rpbey/apps/web/src/lib/discord-lexicon.generated.ts` |
| Page recherche (canonique) | `/home/ubuntu/rpbey/apps/web/src/app/(marketing)/search/page.tsx` |
| Alias FR `/recherche` → `/search` | `/home/ubuntu/rpbey/apps/web/src/app/(marketing)/recherche/page.tsx` |
| Éval qualité (script) | `/home/ubuntu/rpbey/apps/web/scripts/eval-search.ts` |

---

## 1. Le ranker BM25F — `lib/search-rank.ts`

**Pur**, sans aucun import server-only → partagé client (suggestions/SSR) et serveur (retrieval).
Opère sur `GlobalSearchItem[]` (du contrat `@rpbey/api-contract`).

### Tokenisation & normalisation
- `normalize(s)` : minuscules, NFD + suppression des diacritiques, espaces compactés.
- `tokenize(s)` : split sur ponctuation, tokens ≥ 2 chars (ou bloc CJK/chiffre), hors stop-words FR/EN.

### BM25F (Okapi BM25 par champ)
- **Champs + boosts** : `title ×3`, `subtitle ×1.6`, `badge ×1.6`, `details ×1`.
- Constantes : `K1 = 1.2`, `B = 0.75`. `idf = log(1 + (n - df + 0.5)/(df + 0.5))`.
- Corpus **mémoïsé par référence de tableau** (`WeakMap`) → recalcul O(1) entre frappes
  tant que l'index ne change pas (le corpus est une référence stable, cf. cache).

### Expansion de requête (recall)
1. **Synonymes/alias FR/EN/JP** (`SYNONYM_GROUPS`) : groupes curés Beyblade
   (ex. `["wizard rod","wiz rod","ウィザードロッド","wizard arrow"]`, `["cobalt dragoon","cobalt drake","コバルトドレイク","drake","dragoon"]`,
   bits/drivers `ball/needle/flat/taper/orb/rush/gear ball(gb)/high taper(ht)/low flat(lf)`,
   généraux `stamina/attack/defense/combo/tournoi/classement/lanceur/deck/boutique/anime`).
   Un hit sur n'importe quel membre étend la requête aux autres (poids `0.55`).
2. **Alias communautaires** (`COMMUNITY_ALIASES`, depuis `discord-lexicon.generated.ts`) :
   initialismes/contractions **minés hors-ligne** dans le salon Discord Beyblade X. ⚠️ Le
   *contenu* Discord n'entre **jamais** dans le corpus ni les réponses — seul ce **vocabulaire
   d'alias** informe l'expansion (query-time only).
3. **Tolérance aux typos** : Damerau-Levenshtein bornée par longueur (`fuzzyBudget` : 0 si ≤3
   chars, 1 si ≤6, 2 sinon). Token absent du vocabulaire → corrigé vers le terme connu le plus
   proche (poids `0.45`).

### Signaux de boost (après BM25F)
- **Correspondance littérale** sur le titre : exact `+50`, préfixe `+18`, inclus `+8` (gère noms courts/SKU/requêtes 1 mot).
- **Tier** (`tier S/A/B/C` dans badge/subtitle) : `S +3`, `A +2`, `B +1`, `C +0.25`.
- **Prix** disponible : `+1`. **Popularité** (likes tweet, score Reddit, fréquence combo) : `+min(2, log10(1+pop))`.
- **Boost de catégorie** (`CATEGORY_BOOST`) : `product 1.15`, `part 1.1`, `meta 1.08`, `combo 1.05`, … `discussion 0.78`, `page 0.75`.

### Fonctions exportées
`rankSearch(items, query, opts)`, `fuseHybrid(...)`, `scoreItem`, `facetCounts`, `suggest`,
`normalize`, `tokenize`, `expandSynonyms`.

---

## 2. Recherche hybride — `fuseHybrid` (Reciprocal Rank Fusion)

Fusionne le classement **lexical** BM25F (`lexRanked`) et le classement **dense** (`vecRanked`,
voisins sémantiques VSIM) **sans réconcilier des échelles de score incompatibles** : chaque
liste contribue `poids / (k + rang)`.

- Paramètres : `rrfK = 60`, `lexWeight = 1.0`, `vecWeight = 0.9`.
- Le **filtre de catégorie est appliqué AVANT le calcul des rangs RRF** sur les deux listes
  (sinon les rangs globaux fausseraient la fusion within-category).
- **Dégradation gracieuse** : `vecRanked` vide → la fusion préserve exactement l'ordre BM25F.
  Donc sidecar/Redis absents = recherche lexicale pure, zéro panne.

Bénéfice de l'hybride : un item présent dans les deux listes remonte ; un item *seulement dense*
élargit le recall (paraphrase, cross-lingue FR↔EN↔JP) ; un item *seulement lexical* garde la
précision sur les littéraux (codes, SKU).

---

## 3. Le corpus unifié — `global-search.ts` + `search-corpus.ts`

### Assemblage — `buildGlobalSearchIndex()` (~15 sources)

Produit `GlobalSearchItem[]` (`{ id, title, subtitle?, details?, badge?, category, url, thumbnail?, price?, popularity?, source }`).
UI-agnostic. Sources, dans l'ordre :

1. **Produits catalogue** (groupés, `bx-catalog`) — prix multi-boutiques.
2. **Pièces** (DB Drizzle, tiers via `beyblade-entity`).
3. **Tournois** (DB).
4. **Bladers** (rankings SATR / Stardust / WB, dédup par nom).
5. **Lexique** Beyblade X (`data/beyblade-lexique.json`).
6. **Anime** (séries publiées, DB).
7. **Sites** Beyblade du monde (`data/beyblade-sites.json`).
8. **Pages** du site (navigation, `SITE_PAGES`).
9. **Combos gagnants WBO** (`data/wbo-combos.json` + `data/wbo-combos-enriched.json` : tier, score méta, victoires, buzz).
10/11. **Connaissance wiki** (`data/beyblade-knowledge.json`, ~8 500 entités toutes générations — cf. [`knowledge.md`](./knowledge.md)). Dédup canonique vs catalogue/DB.
12. **Frames d'anime** (galerie, DB, jusqu'à 3000).
12b. **Staff RPB** (DB, page « Notre équipe »).
13. **Discussions** X.com (`data/x-discussions.json`) + Reddit (`data/reddit-discussions.json`) — catégorie `discussion`, cherchables plein-texte mais écartées des réponses du chat.
14. **Métagame WBO** (`data/bbx-weekly.json` + `data/meta-enrichment.json` : score 0-100 par composant, fusion `max(scoreWBO, communityScore)`, synergies).

> **Exclusion volontaire** : le salon Discord « Beyblade X » (`data/discord-discussions.json`)
> n'est PAS dans le corpus — ni recherche ni chat. Il ne sert qu'à miner le **vocabulaire
> d'alias** (`scripts/build-discord-lexicon.ts` → `discord-lexicon.generated.ts`), query-time only.

### Cache — `getSearchCorpus()`

Assembler ~15 sources à chaque requête (`force-dynamic`) est coûteux. Deux couches :
1. **In-process memo** : même référence de tableau pendant **60 s** → réutilise le cache BM25F (`WeakMap`).
2. **Redis** : corpus sérialisé dans la clé **`rpbey:search:corpus:v1`**, TTL **1 h**, partagé entre process.

Best-effort total : Redis indisponible → fallback assemblage live. `Bun.RedisClient` résolu via
`globalThis.Bun` au runtime (le builtin `bun` ne s'importe pas dans le bundle Next).
`invalidateSearchCorpus()` / `scripts/refresh-search-corpus.ts` vident la clé après un refresh de data.

---

## 4. La couche dense — sidecar e5-small + `embeddings.ts`

### Sidecar d'embeddings — `apps/embed-sidecar/server.ts`

Service Bun **isolé** du bundle Next (le moteur ONNX ne doit jamais entrer dans webpack).
- **Modèle** : `Xenova/multilingual-e5-small` (**384 dims**), multilingue FR/EN/JP, CPU-friendly,
  via `@huggingface/transformers` (ONNX Runtime). Convention E5 : préfixe `query: ` / `passage: `.
- **Bind** : `127.0.0.1:7077` (loopback), `EMBED_PORT` (défaut 7077).
- **Cache poids** : `EMBED_CACHE_DIR` (défaut `/home/ubuntu/.cache/rpbey-embed-models`), téléchargé une fois.
- **Endpoints** : `GET /health` → `{ ok, model, dim, ready }` ; `POST /embed {texts, kind}` → `{ dim, vectors: number[][] }` (`kind: "query"|"passage"`, mean-pooling + normalize L2, max 256 textes, 1200 chars/texte).
- **Service systemd** : `rpbey-embed.service` (`MemoryMax=1500M`).

### Pont sémantique — `services/embeddings.ts` (`import "server-only"`)
- `embedQuery(text)` : fetch `EMBED_URL/embed` (défaut `http://127.0.0.1:7077`), timeout 1500 ms → `Float32Array` 384d ou `null`.
- `searchVectorIds(query, count=120)` : embed la requête puis `VSIM rpbey:search:vec FP32 <blob> WITHSCORES COUNT n` → `{id, sim}[]`.
- `vectorNeighborsById(id, count)` : voisins d'un item déjà indexé (`VSIM … ELE`), **sans** embedding de requête → utilisable au build SSG (sert « produits liés »).
- **Best-effort** : sidecar/Redis absent → `[]`. `parseVsim` gère RESP2/3.

### Index vectoriel — `scripts/build-search-vectors.ts`
Récupère le corpus (`/api/v1/search` sans `q`, fallback clé Redis), embed chaque item via le
sidecar (`kind:"passage"`, lots de 64, retry/backoff), `DEL` puis `VADD` dans le vector set
Redis **`rpbey:search:vec`** (`FP32`, élément = `item.id`). État vérifié : `VCARD=15232`, `VDIM=384`.

Env : `EMBED_URL`, `EMBED_INDEX_URL` (défaut `http://127.0.0.1:3002/api/v1/search`), `REDIS_URL`.
À relancer après un rebuild du corpus pour réaligner les vecteurs sur les items.

---

## 5. La route API — `/api/v1/search`

`GET`, `runtime="nodejs"`, `dynamic="force-dynamic"`. Contrat Zod (`SearchQuerySchema`/`SearchResponseSchema`).
- Sans `q` → renvoie l'index complet (`{ count, data }`) — c'est la source du `build-search-vectors`.
- Avec `q` → `facetCounts` + `rankSearch` (BM25F) + `searchVectorIds` (VSIM) + `fuseHybrid` (RRF),
  filtré par `category`, `limit` (défaut 50). Réponse `{ count, data, query, facets }`.

Page `/search` (`SearchClient`) consomme cet index ; `/recherche` est un **alias FR** qui
redirige vers `/search` en préservant `?q=` et `?mode=ai` (mode IA = ouvre le chat avec la requête).

---

## Pour aphrody

- La recherche est **fonctionnellement indépendante du LLM** : BM25F + dense, aucun appel au
  chat. aphrody n'a rien à fournir pour la recherche **sauf** s'il veut remplacer le **sidecar
  e5-small** (couche dense). Dans ce cas : exposer un endpoint `POST /embed` compatible
  (`{texts, kind}` → `{dim, vectors}`), même dimension (384) ou ré-indexer (`build-search-vectors`)
  si la dimension change, et pointer `EMBED_URL` dessus.
- Le **vector set Redis** `rpbey:search:vec` (384d) est l'index de la recherche ; le vector set
  `tweet_embeddings` (768d, Gemini) est celui du RAG X.com (cf. [`crawling-x.md`](./crawling-x.md)) —
  **ne pas confondre** (dimensions et modèles différents, mêmes commandes Redis VADD/VSIM).
