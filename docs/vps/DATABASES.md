<!-- SPDX-License-Identifier: Apache-2.0 -->
# Bases de Données & Stockage du VPS

Le VPS héberge une combinaison hybride de bases de données relationnelles, d'instances clé-valeur en mémoire et de bases de données intégrées SQLite, optimisant ainsi la persistance, le temps de réponse et la flexibilité pour chaque application.

---

## 1. PostgreSQL (SGBDR Principal)

Le système utilise PostgreSQL 18 géré par le service `postgresql@18-main.service`. La connexion se fait localement via une socket Unix `/var/run/postgresql` (port `5432`) avec authentification `peer` pour l'utilisateur local `ubuntu`.

### A. Base de données `rose_griffon` (Azalée)
* Utilisée par l'environnement local d'Azalée (`rg`).
* L'application de production du bot (`rg-bot-prod`) utilise quant à elle une base de données PostgreSQL gérée à distance sur **Supabase Cloud** :
  `DATABASE_URL=postgresql://postgres:***@db.isbkaeltubqittywibeh.supabase.co:5432/postgres`

---

## 2. Redis (Cache & Stockage Vectoriel)

L'instance Redis locale tourne sous `redis-server.service` sur le port par défaut `6379` sans mot de passe requis pour le localhost.

### A. Cache de session et files (base de données 1)
Utilisé spécifiquement par `rg-bot-prod` pour la file d'attente hors-ligne, le verrouillage distribué et la persistance temporaire des états Discord :
* **Connexion** : `redis://127.0.0.1:6379/1`

---

## 3. SQLite (Bases de Données Embarquées)

Pour les charges de travail d'inférence, d'indexation de crawling et de persistance des logs de scraping, le système s'appuie sur des instances `bun:sqlite` locales.

### A. `x-store.sqlite` (Crawling & RAG X.com)
* **Chemin** : `/home/ubuntu/.aphrody/x-store.sqlite` (taille ~69 Mo)
* **Configuration** : Mode **WAL** (Write-Ahead Logging) activé pour des lectures concurrentes rapides sans bloquer les processus d'écriture, et `PRAGMA synchronous = NORMAL`.
* **Schéma** :
  * `tweets` : ID, username, text, created_at, counts (likes, retweets, replies), conversation_id, JSON brut.
  * `users` : Métadonnées complètes des comptes X.com indexés.
  * `edges` : Relations orientées entre les comptes de crawling et les tweets (`authored`, `liked`, `bookmarked`, `timeline`, `mention`).
  * `follows` : Liens bidirectionnels de suivi (followers/following).
  * `tweets_fts` : Table virtuelle **FTS5** optimisant la recherche textuelle lexicale.
  * `tweet_embeddings` : Table associant `tweet_id` et `embedding` (type **BLOB** stockant les vecteurs de dimension 768 issus de `gemini-embedding-001`).

### B. `bxc.sqlite` & `bxc-memory.sqlite` (Moteur de Scraping)
* **Chemin** : `/home/ubuntu/bxc/data/bxc.sqlite` et `/home/ubuntu/bxc-memory.sqlite`
* **Configuration** : Mode WAL, busy timeout de 5s, cache mémoire de 8 Mo.
* **Schéma** :
  * `scrapes` : `id` (INTEGER PK AUTOINCREMENT), `url` (TEXT), `profile` (TEXT), `status` (INTEGER), `content` (TEXT), `metadata` (JSON), `timestamp` (DATETIME).
  * `cookie_jars` : `id` (TEXT PK), `data` (JSON contenant les cookies du navigateur), `updated_at` (DATETIME).
  * **Index** : `idx_scrapes_url` sur `scrapes(url)` et `idx_scrapes_timestamp` sur `scrapes(timestamp)`.
