# Crawl + RAG X.com (Twitter) — métagame Beyblade X

Système de crawling autonome + RAG **Gemini** pour extraire, stocker et exploiter les
discussions stratégiques Beyblade X depuis x.com (« Quel est le meilleur combo pour Wizard
Rod ? » → réponse ancrée sur de vrais tweets). **Indépendant** du chat web et du moteur de
recherche rpbey : modèle (Gemini, pas llama.cpp/e5), store (SQLite `~/.aphrody`, pas le corpus
web), et vector set (`tweet_embeddings` 768d, pas `rpbey:search:vec` 384d) différents.

> **Où vit le code** : le repo rpbey ne contient **que la doc** (`/home/ubuntu/rpbey/docs/crawling-rag-x.md`,
> source de cette page). Le code opérationnel vit dans le dépôt autonome **x-client**
> (`/home/ubuntu/x-client/ts/`, package `@aphrody-code/x`). Cet écosystème est **déjà
> co-localisé avec aphrody** (store, session et clé Gemini sous `~/.aphrody` et `aphrody/.env`),
> donc cette doc décrit autant l'état rpbey qu'un sous-système aphrody.

## Emplacements (tout hors-repo rpbey)

| Élément | Chemin |
|---------|--------|
| Package crawl/RAG Bun | `/home/ubuntu/x-client/ts/` (`@aphrody-code/x`) ; bins dans `src/bin/` |
| Store SQLite (partagé) | `/home/ubuntu/.aphrody/x-store.sqlite` (~70 Mo) — tables `tweets`, `users`, `edges`, `follows`, FTS5 `tweets_fts`, `tweet_embeddings` (BLOB) |
| Session (cookies, secret) | `/home/ubuntu/.aphrody/x-session.json` *(secret — ne pas lire)* |
| Clé Gemini | `GEMINI_API_KEY` (fallback `GOOGLE_API_KEY`) dans `/home/ubuntu/aphrody/.env` *(secret — ne pas lire)* |
| Vector set Redis | `tweet_embeddings` (768d FP32) |
| Client natif Rust | `aphrody/crates/aphrody-x-client` (bin `aphrody-x`, 158 ops GraphQL) |
| Docs canoniques Rust | `aphrody/docs/x/{README,architecture,commands,store}.md` |

Sourcer l'env avant tout bin (les bins lisent `process.env`) :
```bash
cd /home/ubuntu/x-client/ts
set -a; . /home/ubuntu/aphrody/.env; set +a   # charge GEMINI_API_KEY (secret, ne jamais l'afficher)
```

---

## 1. Session de crawling — `XSession` / `XClient`

- Code : `packages/x/src/core/session.ts` et `client.ts` (dans x-client).
- **Auth** : cookies de session injectés (`auth_token`, `ct0`, `__cf_bm`…), requêtes HTTP
  directes vers l'API interne X (GraphQL + endpoints recherche/timeline) avec en-têtes
  d'impersonation (UA réalistes). L'IP datacenter du VPS fonctionne en GraphQL **authentifié**.

### Crawler ciblé — `run-targeted-crawler.ts`
```bash
bun run src/bin/run-targeted-crawler.ts        # ~3 min/run, ~160 comptes
```
1. Charge la session (`XSession.load()`), instancie `XClient`, `whoami`, connecte Redis.
2. **Seeds codés en dur** : `followTargets = ["rpb_ey","SunAfterTheBey"]` (followings, 100 chacun) ;
   `verifiedFollowersTargets = ["SunAfterTheBey","Beyblade_Espace","x_beyblade"]` (blue verified
   followers, op GraphQL `BlueVerifiedFollowers`, 80 chacun) ; `directTweetTargets = ["x_beyblade"]`.
3. File unique `queueUsernames` (~160 comptes/run).
4. Par handle : résout l'ID, upsert le profil, ~15 derniers tweets (`userTweets`), upsert chaque
   tweet dans SQLite **ET embedding immédiat inline** (Gemini) → SQLite (`tweet_embeddings` BLOB)
   + Redis (`VADD tweet_embeddings FP32 …`).
5. Délai de politesse 2 s/req (5 s sur échec). `database is locked` transitoires possibles (non fatals).

---

## 2. RAG (Retrieval-Augmented Generation)

### Indexation des embeddings
- **Modèle** : `gemini-embedding-001` (Gemini API), `outputDimensionality: 768` (FP32).
- **Double stockage** : SQLite `tweet_embeddings(tweet_id, embedding BLOB, updated_at)` (vérité
  persistante) **+** vector set Redis `tweet_embeddings` (`VADD … FP32 <blob> <tweet_id>`, index similarité).
- Sans clé Gemini → **vecteur mock normalisé** (mode offline) — pas un embedding réel.
- Script de complétion : `run-index-embeddings.ts` — **boucle continue** : resync SQLite→Redis au
  démarrage, puis embedde par lots de 50 les tweets sans embedding (`LEFT JOIN … WHERE e.tweet_id IS NULL`),
  500 ms entre appels, dort 30 s à vide. (Le crawler embeddant déjà inline, il trouve souvent 0 à faire.)

### Moteur RAG — `BeybladeXRag` (`packages/x/src/services/rag.ts`, bin `run-rag.ts`)
```bash
bun run src/bin/run-rag.ts --query "Quel est le meilleur combo pour Wizard Rod ?"
```
`BeybladeXRag.query` :
1. **Retrieval vectoriel** (d'abord) : embedde la question (Gemini 768d) → `VSIM tweet_embeddings
   FP32 <vec> COUNT 15 WITHSCORES` → IDs réhydratés depuis SQLite (`tweets`).
2. **Retrieval textuel FTS** (hybride) : extrait 3-5 mots-clés via `gemini-2.5-flash` (fallback
   tokenisation locale) → `FTS5 MATCH` sur `tweets_fts` → ajouté aux candidats VSIM (dédup par id).
   *Bug mineur connu : un mot-clé avec `#` déclenche `fts5: syntax error near "#"` — non fatal.*
3. **Tri + expansion de thread** : candidats triés par `like_count`, top 15 ; pour chaque seed avec
   `conversation_id`, jusqu'à 10 tweets du même fil ajoutés au contexte.
4. **Génération** : prompt « professional Beyblade X analyst » + contexte (`[Source N] User/Likes/Content`)
   → `gemini-2.5-flash`. Sans clé → réponse mock offline.
5. Retourne `{ query, answer, sources[] }` (sources = `{id, author_username, text, like_count, conversation_id}`).

---

## 3. Runbook

```bash
cd /home/ubuntu/x-client/ts
set -a; . /home/ubuntu/aphrody/.env; set +a

# État du store
sqlite3 /home/ubuntu/.aphrody/x-store.sqlite \
  "SELECT 'tweets',count(*) FROM tweets UNION ALL SELECT 'embeddings',count(*) FROM tweet_embeddings;"
redis-cli VCARD tweet_embeddings

# Crawl frais (borner la durée)
timeout 220 bun run src/bin/run-targeted-crawler.ts
# Compléter les embeddings manquants (boucle)
timeout 45 bun run src/bin/run-index-embeddings.ts
# Interroger le métagame
bun run src/bin/run-rag.ts --query "Quels sont les top tier blades actuels en Beyblade X ?"
```

> Invariant sain : `tweets == embeddings SQLite == Redis VCARD` (0 tweet sans vecteur). Si X
> renvoie un challenge/403/0 nouveau tweet → la session a expiré (rafraîchir `x-session.json`),
> ne pas conclure à un succès silencieux.

> **État live au moment de la doc** : `redis-cli VCARD tweet_embeddings` = **0** (le vector set X
> n'est pas chargé actuellement — un re-run du crawler/indexer le repeuple depuis SQLite, qui
> contient ~7 500 tweets). C'est l'invariant à restaurer avant d'interroger le RAG X.

---

## 4. Écosystème X complet (au-delà du module Bun)

Tout partage le **même store** `~/.aphrody/x-store.sqlite` + la même session + le même vector set Redis.

| Couche | Emplacement | Rôle |
|--------|-------------|------|
| **Crawl/RAG Bun** | `x-client/ts/` (`@aphrody-code/x`) | crawl ciblé + embeddings Gemini 768d + RAG hybride VSIM+FTS5 (ce guide) |
| **Client natif Rust** | `aphrody/crates/aphrody-x-client` (bin `aphrody-x`) | contrôle de compte headless, 158 ops GraphQL, 47 sous-commandes, store local-first, auto-refresh `queryId` |
| **Canal messaging Rust** | `aphrody/crates/aphrody-messaging/src/channels/x.rs` | shell-out le binaire `aphrody-x` (post/timeline) |
| **Classification Python** | `aphrody/scripts/classify_tweets.py` | analytique post-hoc (regex bilingues EN/JP, topics métagame) — **hors chemin RAG** |
| **Cache** | Redis 8.x (vector set) + SQLite (FTS5 + BLOB) | retrieval cosinus + plein-texte |

> Pour la spec détaillée des couches Rust, voir `aphrody/docs/x/`. Cette page reste centrée sur
> le pipeline crawl→RAG Bun consommé côté analyse métagame rpbey.

---

## Lien avec rpbey (web)

Le RAG X.com **n'est pas appelé en direct** par le site. Son influence sur rpbey passe par des
**exports hors-ligne** dans le corpus de recherche :
- `apps/web/scripts/export-x-discussions.ts` / `analyze-x-corpus.ts` → `apps/web/data/x-discussions.json`
  (tweets nettoyés/classés) → indexé en catégorie `discussion` (cherchable, mais **écarté des
  réponses du chat**, cf. [`search.md`](./search.md) et [`chat.md`](./chat.md)).
- `apps/web/data/meta-enrichment.json` (signaux X+Reddit+web par blade) → fusionné dans la
  tier-list méta du corpus (`global-search.ts` § 14).

Donc : le **buzz** X informe le score méta et le recall ; le **contenu brut** des tweets reste un
signal de discussion, jamais présenté comme un fait par le chat.

## Pour aphrody

- Cet écosystème est **déjà à moitié aphrody** (store/session/clé sous `~/.aphrody`, crate Rust
  `aphrody-x-client`, classifier Python aphrody). Le seul morceau « rpbey » est l'export JSON vers
  le corpus web.
- Le RAG X utilise **Gemini** (embedding 768d + génération), pas le LLM local. Si aphrody veut
  souverainiser ce RAG aussi, il faudrait : remplacer `gemini-embedding-001` par un embedder local
  768d (ou ré-indexer à une autre dimension) et `gemini-2.5-flash` par le daemon aphrody. C'est un
  chantier distinct du seam `RPBEY_LLM_URL` du chat web.
