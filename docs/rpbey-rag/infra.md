# Infra RAG / IA rpbey — services, ports, stores, fichiers hors-repo

Consolidation opérationnelle de tout ce que les systèmes RAG/IA de rpbey utilisent au runtime :
services systemd, timers, ports, stores Redis/SQLite, modèles, et variables d'environnement.
Tout est **loopback** (rien d'exposé publiquement côté IA) et **VPS-only** (`51.77.147.152`).

---

## 1. Services systemd

| Service | État | Rôle | Bind / port | Lance |
|---------|------|------|-------------|-------|
| `rpbey-llm.service` | active | **LLM local** llama.cpp (chat IA) | `127.0.0.1:8080` | `llama.cpp/build/bin/llama-server -m Llama-3.2-3B-Instruct-Q4_K_M.gguf` |
| `rpbey-embed.service` | active | **Sidecar embeddings** (couche dense recherche) e5-small 384d | `127.0.0.1:7077` | `bun apps/embed-sidecar/server.ts` |
| `rpbey-web.service` | active | App Next.js 16 (`/api/chat`, `/api/v1/search`) | `127.0.0.1:3002` | `bun .next/standalone/apps/web/server.js` |
| `rpbey-gacha.service` | active | Serveur gacha Colyseus (non-IA) | `127.0.0.1:5050` | — |
| `rpb-bot.service` | active | Bot Discord (RAG algorithmique, **aucun LLM**) | `:3001` | — |
| `llm-thp.service` | active (exited) | THP `always` pour l'inférence (oneshot) | — | `echo always > .../transparent_hugepage/enabled` |

> `shenron-embed.service` / `shenron-neon-sync.timer` appartiennent à **un autre projet** (bot
> Dragon Ball « Shenron »), pas à rpbey — ne pas les confondre.

### Détails clés des unités IA

**`rpbey-llm.service`** — flags llama-server : `-c 8192` (contexte), `-t 10` (threads CPU),
`--parallel 1`, `-fa on` (flash attn), `--cache-reuse 256`, `--no-webui`. `MemoryMax=6G`,
`Restart=always`, `Nice=5`. Détails complets → [`llm.md`](./llm.md).

**`rpbey-embed.service`** — `Environment=EMBED_PORT=7077`,
`EMBED_CACHE_DIR=/home/ubuntu/.cache/rpbey-embed-models`, `MemoryMax=1500M`,
`After=… redis-server.service`. Détails → [`search.md`](./search.md).

**`rpbey-web.service`** — `EnvironmentFile=-/home/ubuntu/rpbey/apps/web/.env` (optionnel ;
le `-` = ne pas échouer s'il manque). Env inline : `PORT=3002`, `HOSTNAME=127.0.0.1`,
`PGHOST=/var/run/postgresql`, `PGDATABASE=rpb_neon`, `PGUSER=ubuntu`. C'est **ici** qu'on
surcharge `RPBEY_LLM_URL` / `EMBED_URL` / `REDIS_URL` / `RPBEY_CHAT_LLM` pour repointer ou
désactiver (via le `.env`). Restart : `systemctl restart rpbey-web`.

---

## 2. Timers systemd (alimentation des données, non-IA)

| Timer | Cadence | Service | Effet |
|-------|---------|---------|-------|
| `rpbey-staff-sync.timer` | quotidien 04:30 (+≤5 min) | `rpbey-staff-sync.service` | sync `staff_members` (avatars/pseudo/nom) Discord → `rpb_neon` |
| `rpbey-profile-sync.timer` | quotidien 05:00 (+≤5 min) | `rpbey-profile-sync.service` | enrichissement profils Discord + recalcul classement global (`scripts/sync-profiles.sh`) |

> Il n'existe **pas** de timer systemd pour le crawl X.com, le crawl Fandom, ni le rebuild des
> vecteurs de recherche — ces opérations sont **manuelles / on-demand** (scripts, cf. docs
> correspondantes). Les timers ci-dessus ne touchent pas le RAG.

---

## 3. Ports (tous loopback)

| Port | Service | Protocole |
|------|---------|-----------|
| `8080` | `rpbey-llm` (llama.cpp) | HTTP OpenAI-compatible (`/v1/chat/completions`, `/v1/models`) |
| `7077` | `rpbey-embed` (sidecar) | HTTP (`/health`, `/embed`) |
| `3002` | `rpbey-web` | HTTP (Next.js ; `/api/chat`, `/api/v1/search`) |
| `6379` | Redis | RESP (corpus, vector sets) |
| `5050` | `rpbey-gacha` | WS (non-IA) |
| `3001` | `rpb-bot` | HTTP (non-IA) |

nginx : `rpbey.fr` → `127.0.0.1:3002`. Les services IA (8080/7077) ne sont **pas** proxiés.

---

## 4. Stores

### Redis (`127.0.0.1:6379`)

| Clé | Type | Dim | Système | Alimenté par |
|-----|------|-----|---------|--------------|
| `rpbey:search:corpus:v1` | string (JSON) | — | recherche + chat (corpus unifié) | `getSearchCorpus()` (TTL 1 h) |
| `rpbey:search:vec` | vector set | **384** | recherche + chat (couche dense) | `scripts/build-search-vectors.ts` (sidecar e5) |
| `tweet_embeddings` | vector set | **768** | RAG X.com (séparé) | `run-targeted-crawler.ts` / `run-index-embeddings.ts` (Gemini) |

État vérifié (2026-06-01) : `VCARD rpbey:search:vec` = **15 232**, `VDIM` = **384** ;
`VCARD tweet_embeddings` = **0** (vide actuellement — repeuplé par un re-run du crawler/indexer X
depuis le SQLite, cf. [`crawling-x.md`](./crawling-x.md)).

> ⚠️ Deux vector sets aux **dimensions différentes** cohabitent (384 e5 vs 768 Gemini). Ne pas
> mélanger : commandes Redis identiques (`VADD`/`VSIM`/`VCARD`/`VDIM`), modèles et usages distincts.

### SQLite

| Fichier | Taille | Contenu | Système |
|---------|--------|---------|---------|
| `/home/ubuntu/.aphrody/x-store.sqlite` | ~70 Mo | `tweets`, `users`, `edges`, `follows`, FTS5 `tweets_fts`, `tweet_embeddings` (BLOB) | RAG X.com (partagé avec aphrody) |

### Postgres (non-IA mais source du corpus)
Socket `/var/run/postgresql`, base `rpb_neon`, user `ubuntu`, via `@rpbey/db` (Drizzle). Fournit
pièces, tournois, rankings, anime, frames, staff au corpus de recherche (`global-search.ts`).

### Fichiers de données hors-DB (corpus)
`/home/ubuntu/rpbey/apps/web/data/*.json` — produits par les scrapers, lus par `global-search.ts` :
`beyblade-knowledge.json` (~10 Mo, wiki), `wbo-combos.json` / `wbo-combos-enriched.json`,
`bbx-weekly.json`, `meta-enrichment.json`, `x-discussions.json`, `reddit-discussions.json`,
`beyblade-lexique.json`, `beyblade-sites.json`. (`discord-discussions.json` existe mais est
**exclu** du corpus — vocabulaire d'alias seulement.)

### Modèles (hors-repo, non versionnés)
| Chemin | Taille | Usage |
|--------|--------|-------|
| `/home/ubuntu/llm/models/Llama-3.2-3B-Instruct-Q4_K_M.gguf` | ~2.0 Go | **servi** par rpbey-llm (chat) |
| `/home/ubuntu/llm/models/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf` | ~986 Mo | présent, non servi (modèle alternatif swappable) |
| `/home/ubuntu/.cache/rpbey-embed-models/` | — | poids ONNX e5-small (sidecar, téléchargés une fois) |

---

## 5. Variables d'environnement (récap — noms seulement)

> Aucune valeur de secret n'est listée. Les fichiers `.env` ne sont **pas** versionnés.

### Chat / LLM (lues par `apps/web`, surchargeables via `apps/web/.env`)
| Variable | Défaut | Doc |
|----------|--------|-----|
| `RPBEY_LLM_URL` | `http://127.0.0.1:8080/v1/chat/completions` | **seam aphrody** ([`llm.md`](./llm.md)) |
| `RPBEY_LLM_MODEL` | `rpbey-local` | champ `model` |
| `RPBEY_LLM_TIMEOUT_MS` | `60000` | timeout |
| `RPBEY_CHAT_LLM` | (unset=actif) | kill switch (`=0` → repli extractif) |
| `RPBEY_CHAT_MODEL` | (présent dans `.env`) | hérité ; le code lit `RPBEY_LLM_MODEL` |

### Recherche / embeddings
| Variable | Défaut | Service |
|----------|--------|---------|
| `EMBED_URL` | `http://127.0.0.1:7077` | client web → sidecar |
| `EMBED_PORT` | `7077` | sidecar (bind) |
| `EMBED_MODEL` | `Xenova/multilingual-e5-small` | sidecar |
| `EMBED_CACHE_DIR` | `/home/ubuntu/.cache/rpbey-embed-models` | sidecar |
| `EMBED_INDEX_URL` | `http://127.0.0.1:3002/api/v1/search` | `build-search-vectors` |
| `REDIS_URL` | `redis://127.0.0.1:6379` | corpus + vector set |

### RAG X.com (lues par x-client, dans `/home/ubuntu/aphrody/.env` — hors-repo, **secret**)
| Variable | Rôle |
|----------|------|
| `GEMINI_API_KEY` (fallback `GOOGLE_API_KEY`) | embeddings 768d + génération `gemini-2.5-flash` |

> Le fichier `/home/ubuntu/aphrody/.env` contient aussi `GOOGLE_APPLICATION_CREDENTIALS`,
> `GOOGLE_CLOUD_PROJECT/LOCATION/REGION`, `ANTIGRAVITY_API_KEY`, `GCP_SERVICE_ACCOUNT`, etc.
> (noms seulement — **ne jamais lire les valeurs**). Pour le RAG X, seule la clé Gemini compte.

---

## 6. Vérifs rapides (sans secret)

```bash
# Services IA
systemctl status rpbey-llm rpbey-embed --no-pager
curl -s http://127.0.0.1:8080/v1/models | head
curl -s http://127.0.0.1:7077/health

# Stores
redis-cli VCARD rpbey:search:vec      # ~15232 attendu (recherche)
redis-cli VDIM  rpbey:search:vec      # 384
redis-cli VCARD tweet_embeddings      # RAG X (0 si non chargé)
redis-cli TTL   rpbey:search:corpus:v1
sqlite3 /home/ubuntu/.aphrody/x-store.sqlite "SELECT count(*) FROM tweets;"
```

## Pour aphrody

- **Un seul service à remplacer** pour souverainiser le chat : `rpbey-llm.service` → pointer
  `RPBEY_LLM_URL` (dans `apps/web/.env`) vers l'endpoint OpenAI-compat d'aphrody, `systemctl
  restart rpbey-web`. Optionnel : remplacer `rpbey-embed.service` (sidecar e5) via `EMBED_URL`.
- Le RAG X.com est déjà **co-localisé** avec aphrody (`~/.aphrody`, crate `aphrody-x-client`, clé
  Gemini dans `aphrody/.env`) — c'est plus un sous-système aphrody qu'un service rpbey.
- Rien d'autre n'a besoin d'aphrody : recherche BM25F, corpus, crawl Fandom et timers de sync sont
  algorithmiques et autonomes.
