# Chat IA « Rpbey » — pipeline `/api/chat`

Le chat IA du site rpbey.fr. Un utilisateur pose une question Beyblade en français
(« C'est quoi le meilleur combo méta ? », « Qui est Ryuga ? »), et reçoit une réponse
**streamée**, en français, **groundée** sur un corpus de connaissance Beyblade, avec
**mémoire conversationnelle** (multi-tour). C'est un RAG : *retrieve* hybride →
*generate* par un LLM **local**.

> **Important** : le chat n'est PLUS « zéro LLM », contrairement à ce qu'affirment
> encore un commentaire d'en-tête de `RpbeyChat.tsx` et le titre de `chat-nlp.ts`
> (« ZÉRO LLM »). Ces commentaires sont **stale** : depuis le commit `19745c2`
> (2026-05-29), `prepareTurn` produit des `messages` qui sont envoyés à un vrai LLM
> local (llama.cpp). Le « zéro LLM » ne s'applique plus qu'à la couche NLP de routage
> (`detectIntent`) et au repli extractif déterministe.

## Fichiers

| Rôle | Chemin absolu |
|------|---------------|
| Route HTTP (SSE) | `/home/ubuntu/rpbey/apps/web/src/app/api/chat/route.ts` |
| Cerveau RAG (retrieval + messages) | `/home/ubuntu/rpbey/apps/web/src/server/services/chat.ts` |
| Client LLM (OpenAI-compat) | `/home/ubuntu/rpbey/apps/web/src/server/services/llm.ts` (→ [`llm.md`](./llm.md)) |
| NLP de routage (intents, sans LLM) | `/home/ubuntu/rpbey/apps/web/src/lib/chat-nlp.ts` |
| Ranker hybride partagé | `/home/ubuntu/rpbey/apps/web/src/lib/search-rank.ts` (→ [`search.md`](./search.md)) |
| Couche dense (embeddings) | `/home/ubuntu/rpbey/apps/web/src/server/services/embeddings.ts` |
| Corpus unifié | `/home/ubuntu/rpbey/apps/web/src/server/services/search-corpus.ts` + `global-search.ts` |
| Composant UI client | `/home/ubuntu/rpbey/apps/web/src/components/chat/RpbeyChat.tsx` |

Le chat tape **directement les services in-process** (pas d'aller-retour HTTP vers
`/api/v1/search`). C'est l'équivalent web du bot Discord (`apps/bot/src/lib/rpbey/answer.ts`).

---

## 1. Route `POST /api/chat` — contrat SSE

`route.ts` : `runtime = "nodejs"`, `dynamic = "force-dynamic"`.

**Body de la requête** :
```jsonc
{ "message": "string (2..400 chars)", "history": [{ "role": "user|assistant", "content": "..." }] }
```
- `message` : trimé ; rejet (`type:"error"`) si < 2 chars ; tronqué à 400 chars.
- `history` : **mémoire conversationnelle**, fournie par le client (le backend est
  **stateless**). `sanitizeHistory` garde les 16 derniers tours, rôles `user`/`assistant`
  seulement, contenu capé à 1200 chars.

**Réponse** : flux `text/event-stream` (`Cache-Control: no-cache, no-transform`).
Chaque événement est une ligne `data: {json}\n\n` :

| `type` | Cardinalité | Charge utile |
|--------|-------------|--------------|
| `meta` | 1× (en tête) | `{ intent, found, sources[], followups[] }` |
| `delta` | N× | `{ text }` — le texte de la réponse qui s'écrit token par token |
| `done` | 1× (fin) | `{}` |
| `error` | (cas d'erreur) | `{ message }` |

Logique du `start(controller)` :
1. `prepareTurn(message, history)` → `PreparedTurn` (retrieval + brouillon + messages).
2. Envoie l'événement `meta`.
3. **Branches** :
   - `p.fixed != null` (greeting/thanks/stats/rien-trouvé) → un seul `delta` déterministe, pas de LLM.
   - `p.messages` présent → on **stream** `generateStream(p.messages)` ; chaque chunk = un `delta`. Si le générateur ne yield rien (LLM indisponible/vide), repli sur `p.draft` (brouillon extractif) en un seul `delta` → **jamais d'écran vide**.
   - sinon → `delta` = `p.draft`.
4. `done`.

---

## 2. `prepareTurn` — le cœur du RAG (retrieval + construction des messages)

`services/chat.ts`, fonction exportée `prepareTurn(message, history): Promise<PreparedTurn>`.
Fait le retrieval et construit les messages, **sans** appeler le LLM (partagé entre
streaming et non-stream).

```ts
interface PreparedTurn {
  found: boolean;
  intent: Intent;
  sources: ChatSource[];
  followups: string[];
  fixed?: string;     // réponse déterministe immédiate (pas de LLM)
  draft?: string;     // brouillon extractif (repli si LLM lâche)
  messages?: ChatTurn[]; // à streamer vers le LLM
}
```

### a. Classification d'intention (NLP sans LLM) — `lib/chat-nlp.ts`

`detectIntent(question): Intent` applique une liste ordonnée de regex FR/EN (premier match
gagne). Intents : `greeting`, `thanks`, `combo`, `best`, `meta`, `buy`, `tournament`,
`rules`, `stats`, `compare`, `character`, `define` (repli). Chaque intent biaise une
**catégorie de retrieval** via `INTENT_CATEGORY` (ex. `combo→combo`, `buy→product`,
`character→anime`, `rules→lexicon`, `define→null` = global).

`searchTerms(question)` nettoie l'échafaudage interrogatif (« c'est quoi », « qui est »,
« explique-moi »…) pour ne garder que l'entité cherchée. La compréhension fine (synonymes,
fautes) est déléguée au ranker BM25F (cf. [`search.md`](./search.md)).

### b. Réponses déterministes (`fixed`, pas de LLM)

- `greeting` / `thanks` → message fixe (templates `GREETINGS`).
- `stats` → `answerStats` : **compte réel** par catégorie du corpus (zéro invention).
- `compare` → `answerCompare` : isole deux entités via `COMPARE_SPLIT` (« X vs Y »,
  « X contre Y »…), les récupère, les oppose côte à côte, calcule un verdict si un score
  méta numérique est extractible. Produit un `draft` + `messages` (donc reformulable par le LLM).
- `found:false` (rien trouvé) → message fixe + suggestions cliquables (entités proches).

### c. Retrieval hybride — `retrieve(terms, category, limit)`

C'est le **R de RAG**. Fusionne lexical et dense :
```ts
const index = await getSearchCorpus();              // corpus unifié (Redis-cached)
const lex   = rankSearch(index, terms, {});         // BM25F (lib/search-rank.ts)
const vec   = await searchVectorIds(terms, 120);    // voisins denses (VSIM Redis)
const fused = fuseHybrid(index, lex, vec, { category, limit }); // RRF
```
- **Lexical** : BM25F par champ + synonymes FR/EN/JP + fuzzy Damerau-Levenshtein (→ [`search.md`](./search.md)).
- **Dense** : `searchVectorIds` embed la requête via le **sidecar e5-small 384d**
  (`services/embeddings.ts`, `EMBED_URL`), puis `VSIM` sur le vector set Redis
  `rpbey:search:vec`. Best-effort : sidecar/Redis absent → `[]`, et `fuseHybrid`
  préserve alors l'ordre BM25F (dégradation gracieuse).
- **Fusion** : Reciprocal Rank Fusion (RRF, `k=60`, `lexWeight=1.0`, `vecWeight=0.9`).

**Termes conscients du contexte** (`contextualTerms`) : sur une relance courte/anaphorique
(« et sa toupie ? »), le dernier tour utilisateur est réinjecté dans les termes pour que le
retrieval retrouve la bonne entité (la mémoire vit au niveau du modèle, mais le retrieval
doit suivre).

**Filtrage** : la catégorie `discussion` (X/Reddit, ensemble `CHATTER`) est utile au
*recall* mais **écartée des réponses** (faits non vérifiés). Le salon Discord « Beyblade X »
est déjà hors corpus en amont (cf. `global-search.ts`).

### d. Construction des messages (mémoire + grounding) — `buildMessages`

Produit le tableau `ChatTurn[]` envoyé au LLM :
```
[ system,  ...history (8 derniers tours, capés 1200 chars),  user(question + CONTEXTE RAG) ]
```
- **System prompt** (`LLM_SYSTEM`) : « Tu es Rpbey… réponds TOUJOURS en français…
  N'utilise QUE les faits du CONTEXTE… le contexte est souvent en anglais (wiki Fandom) :
  traduis-le… N'invente JAMAIS… pas d'URL (sources affichées séparément). »
- **Indice par intention** (`LLM_INTENT_HINT`) : cadre la *forme* (liste pour combos,
  classement pour meta, bio pour character…) ; le *fond* reste le contexte.
- **Bloc de faits** (`factsBlock`) : les 6 meilleurs items du retrieval, formatés
  `- Titre (badge · subtitle) : details` — c'est le contexte injecté UNIQUEMENT sur le
  tour courant (frais à chaque question). L'historique ne porte que le texte des échanges.
- **Garde-fous coût CPU** : `MAX_HISTORY_TURNS = 8`, `MAX_TURN_CHARS = 1200` (le LLM tourne
  sur CPU, le prompt doit rester borné).

### e. Brouillon extractif déterministe (`draft`, repli)

Pour chaque intent RAG, un brouillon Markdown est construit **sans LLM** (listes à puces des
items, avec emoji de catégorie). Il sert de repli si le LLM est inactif/indisponible ; sinon
il est reformulé en français naturel par le LLM. **Garantit qu'une réponse existe toujours.**

---

## 3. Génération — `generateStream` / `generate`

Le *G de RAG*. Délégué à `services/llm.ts` (LLM local llama.cpp, OpenAI-compatible).
Détails complets → [`llm.md`](./llm.md). En résumé :
- `generateStream(messages)` : async generator qui yield chaque fragment SSE (utilisé par la route).
- `generate(messages)` : version non-stream (utilisée par `answerQuestion`, repli/clients sans SSE).
- Les deux renvoient `null`/rien (jamais une exception) si le LLM est désactivé
  (`RPBEY_CHAT_LLM=0`), absent, lent (timeout 60 s) ou vide → l'appelant retombe sur `draft`.

`answerQuestion(message, history)` (non-streaming) : `prepareTurn` → si `fixed` l'utilise,
sinon si `messages` + `isLlmEnabled()` appelle `generate`, sinon `draft`.

---

## 4. Côté client — `RpbeyChat.tsx`

Composant React (`"use client"`), style « Gemini app » (sparkle 4-couleurs, bulles, prompt-bar).
- **Mémoire** : tient `messages` en state ; à chaque `send`, transmet les 12 derniers tours
  (hors erreurs) comme `history` dans le body. Le backend reste stateless.
- **Streaming** : lit le `ReadableStream`, parse les frames SSE (`\n\n`), patche la bulle
  assistant en cours (`type:"delta"` → concatène `text` ; `type:"meta"` → pose `sources`/`followups`).
- **Anti-écran-vide** : flux clos sans texte → message de repli ; erreur réseau → message d'erreur.
- Lancement auto si `initialQuery` (depuis la barre de recherche en mode IA).

> Note stale : l'en-tête du composant dit « ZÉRO LLM » — obsolète (voir avertissement en haut).

---

## Pour aphrody

- Le **seul** point à brancher est `RPBEY_LLM_URL` (cf. [`llm.md`](./llm.md)). Tout le retrieval,
  la mémoire, le grounding et le repli restent identiques quel que soit le backend LLM.
- Le contrat de réponse SSE (`meta`/`delta`/`done`) est interne à rpbey ; aphrody n'a pas à
  le connaître — il ne fournit que l'endpoint OpenAI Chat Completions (stream).
- Le grounding est **strict** (system prompt anti-hallucination + repli extractif) : un
  daemon aphrody plus capable améliorerait surtout la fluidité du français et la synthèse,
  pas le rappel factuel (qui vient du retrieval).
