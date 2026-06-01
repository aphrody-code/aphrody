# Crawler de connaissance Beyblade — `crawl-fandom.ts`

Le corpus de fond du RAG rpbey. Un crawler MediaWiki **exhaustif** du Beyblade Wiki (Fandom)
qui accumule TOUTE la connaissance Beyblade, **toutes générations** (Original/Plastic, HMS,
Metal Saga, Burst, Beyblade X) : toupies, pièces, personnages, anime (séries + épisodes), jeux
vidéo, accessoires, lore. ~8 500 entités classées (image + résumé) → `beyblade-knowledge.json`,
câblé dans la recherche et le chat.

## Fichiers

| Rôle | Chemin absolu |
|------|---------------|
| Crawler | `/home/ubuntu/rpbey/apps/web/scripts/crawl-fandom.ts` |
| Sortie | `/home/ubuntu/rpbey/apps/web/data/beyblade-knowledge.json` (~10 Mo) |
| Checkpoint résumable | `/home/ubuntu/rpbey/apps/web/data/.fandom-crawl-state.json` |
| Schéma (Zod) | `WikiEntitySchema` / `WikiEntity` dans `@rpbey/api-contract` |
| Intégration au corpus | `loadWikiKnowledge()` dans `apps/web/src/server/services/global-search.ts` |
| Graphe d'entités (consommateur) | `apps/web/src/server/services/entity-graph.ts` |

---

## 1. Source & méthode

- **Source** : `beyblade.fandom.com` (MediaWiki 1.43, ~8 500 articles). L'API `api.php` est
  **joignable depuis le VPS** (contrairement aux pages HTML protégées Cloudflare) → on tape l'API
  JSON directement (UA Chrome), la voie la plus robuste et complète pour un wiki.
- **Endpoint** : `https://beyblade.fandom.com/api.php` (format JSON, `maxlag=5`).

### Pipeline (« le meilleur crawler possible »)
1. Énumère TOUTES les pages de l'espace principal (`list=allpages`, non-redirects).
2. Récupère en **lot (50/req)** : catégories + image pleine résolution + wikitext.
3. Parse l'infobox (1er template `{{… | k = v …}}`, accolades équilibrées).
4. **Classe** chaque page : TYPE (`bey`/`character`/`part`/`anime`/`episode`/`game`/`accessory`/`lore`)
   + génération + système + sens de rotation + type de combat + nom JP, depuis catégories + infobox.
5. Dérive un **résumé texte clair** du wikitext (TextExtracts absent sur Fandom).

### Robustesse
- Requêtes **sérielles** + `maxlag`, retry/backoff exponentiel sur 429/5xx/maxlag/réseau (≤5 tentatives, ≤30 s).
- **Checkpoint résumable** (`data/.fandom-crawl-state.json`) → reprise après interruption.
- Écriture **NON-destructive** (jamais d'écrasement par du vide).
- Validation **Zod** (`WikiEntitySchema`) avant écriture.
- Politesse : `DELAY_MS = 150` entre requêtes, UA Chrome.

### Invocation
```bash
cd /home/ubuntu/rpbey
bun apps/web/scripts/crawl-fandom.ts            # crawl complet
FANDOM_LIMIT=300 bun apps/web/scripts/crawl-fandom.ts   # échantillon (test)
FANDOM_RESET=1   bun apps/web/scripts/crawl-fandom.ts   # ignore le checkpoint
```
Env : `FANDOM_LIMIT` (défaut Infinity), `FANDOM_RESET` (1 = ignore checkpoint).

---

## 2. Le schéma de sortie — `WikiEntity`

`beyblade-knowledge.json` = `{ entities: WikiEntity[] }`. Chaque entité porte (champs observés) :
`id`, `title`, `url`, `type` (`bey|character|part|anime|episode|game|accessory|lore`),
`generation` (`ORIGINAL|HMS|METAL|BURST|X`), `system`, `beyType`, `jpName`, `summary`, `imageUrl`.

---

## 3. Intégration au corpus de recherche — `loadWikiKnowledge()`

Dans `global-search.ts` (étapes 10/11). Mappe le type wiki vers les **catégories existantes** du
contrat de recherche (pas de nouvelle catégorie → onglets inchangés) :

| Type wiki | Catégorie de recherche | Badge |
|-----------|------------------------|-------|
| `bey` | `product` | `Bey · <génération>` |
| `part` | `part` | `Pièce · <génération>` |
| `character` | `anime` | `Personnage` |
| `anime` | `anime` | `Anime` |
| `episode` | `anime` | `Épisode` |
| `game` | `product` | `Jeu vidéo` |
| `accessory` | `product` | `Accessoire` |
| (défaut/lore) | `lexicon` | `Lore` |

- **Dédup** : beys & pièces fusionnés par **clé canonique** (`canonicalKey`) vs catalogue/DB
  (mute les doublons « Dran Sword » ⇔ « dran-sword ») ; le reste dédupliqué par titre exact.
- **Borne de payload** : le `summary` est tronqué à ~220 chars dans l'item de recherche (l'index
  complet est fetché côté client par `/search`, ~8 400 entités → on borne). Le résumé intégral
  reste dans `beyblade-knowledge.json` pour le graphe d'entités et les pages détail.

Ce corpus wiki **subsume** les anciens streams `universe_beys`/`characters`. Il est en **anglais**
(Fandom) — c'est pourquoi le system prompt du chat demande au LLM de **traduire + reformuler en
français** sans rien inventer (cf. [`chat.md`](./chat.md)).

---

## 4. Place dans le corpus global

`beyblade-knowledge.json` est **une** des ~15 sources assemblées par `buildGlobalSearchIndex()`
(cf. [`search.md`](./search.md) § corpus). Il représente la majorité du volume (~8 500 entités sur
~15 200 au total). Après un re-crawl, invalider le corpus consolidé :
```bash
bun apps/web/scripts/refresh-search-corpus.ts        # vide rpbey:search:corpus:v1
bun apps/web/scripts/build-search-vectors.ts          # ré-embed le corpus → rpbey:search:vec
```

> Note CLAUDE.md/mémoire : ce crawler a fait passer le corpus de recherche de ~9 018 à ~17 082
> entités lors de son intégration (le chiffre exact varie selon les sources actives au moment du build).

---

## Autres scrapers alimentant le corpus (pour contexte)

`crawl-fandom` est le plus gros, mais d'autres scripts produisent des `apps/web/data/*.json`
indexés (tous non-IA, scraping pur — voir `apps/web/scripts/`) :
`scrape-wbo.ts`, `scrape-bbx-weekly.ts`, `scrape-beyblade-library.ts`, `scrape-bx-shops.ts`,
`scrape-zenmarket.ts`, `scrape-amazon-fr.ts`, `scrape-reddit-discussions.ts`,
`export-x-discussions.ts`, `scrape-fandom-frames.ts`, `enrich-combos.ts`, `enrich-meta.ts`.

## Pour aphrody

- Ce crawler est **purement algorithmique** (MediaWiki API + parse, zéro IA). aphrody n'a rien à
  fournir ici. C'est le **fournisseur de faits** du RAG : la qualité des réponses du chat dépend
  d'abord de la fraîcheur de `beyblade-knowledge.json`, pas du LLM.
- Si aphrody industrialise le crawl/RAG, ce JSON est un bon corpus de référence (entités typées,
  résumés, génération, images) à chunker/embedder dans le pipeline RAG natif d'aphrody ([`../RAG.md`](../RAG.md)).
