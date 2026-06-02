<!-- SPDX-License-Identifier: Apache-2.0 -->
# REST & GraphQL APIs du VPS

Ce document détaille les interfaces de programmation (APIs) exposées, configurées ou consommées sur le VPS, y compris la matrice des ports, les schémas d'authentification et le routage frontal Nginx.

---

## 1. Matrice des Ports & Services API

Voici l'inventaire des ports d'écoute configurés sur le VPS pour l'ensemble des APIs :

| Port | Protocole | Service interne | Type d'API / Rôle | Accès public / Proxy |
|---|---|---|---|---|
| `3000` | HTTP / WSS | `bxc.service` | REST & GraphQL Bxc (Elysia + Yoga) | Loopback local (`127.0.0.1`) |
| `3001` | HTTP | `rpb-bot.service` | API du Bot Discord (Bun.serve) | Proxy via `api.rpbey.fr` |
| `3002` | HTTP | `rpbey-web.service` | Standalone Next.js 16 Dashboard | Proxy via `rpbey.fr` |
| `3003` | HTTP | `azalee-web.service` | Standalone Next.js 16 Azalée | Proxy via `azalee.rosegriffon.fr` |
| `5050` | HTTP / WSS | `rpbey-gacha.service` | Express REST & Colyseus Bun WS | Proxy via `api.rpbey.fr/gacha/` |
| `8788` | HTTP / JSON-RPC | `aphrody` A2A | Listener de coordination de l'agent | Loopback local (`127.0.0.1`) |

---

## 2. Bxc Browser Automation API (Port 3000)

Le moteur d'automatisation de navigation `bxc` tourne sous **Elysia** avec un point d'intégration GraphQL via **GraphQL Yoga** (cf. [`bxc/src/server/index.ts`](file:///home/ubuntu/bxc/src/server/index.ts)).

### A. Routes REST
* **`GET /health`** : Vérification d'état (renvoie `{ok: true, service: "bxc-api"}`).
* **`POST /api/scrape`** : Lance une tâche de scraping structurée pour une URL donnée.
* **`GET /api/scrape/recent`** : Historique des derniers scrapings effectués (lu depuis la table SQLite `scrapes`).
* **Scrapers dédiés FIFA FUT** :
  * `GET /api/fut/price` : Prix d'enchères d'un joueur en temps réel.
  * `GET /api/fut/player` : Informations de fiches joueurs.
  * `GET /api/fut/summary` : Statistiques de la base locale de prix FIFA (total joueurs, moyennes, répartitions par rareté/genre).
* **Scrapers dédiés Anime (VoirAnime)** :
  * `GET /api/voiranime/search` : Recherche de séries.
  * `GET /api/voiranime/info` : Fiche détaillée d'une série.
  * `GET /api/voiranime/resolve` : Résolution des liens de lecture vidéo (m3u8/mp4).

### B. Route GraphQL (`POST /graphql`)
Expose une interface typée pour l'intégration agentique de haut niveau. Les résolveurs principaux sont :
* **`ScrapeResolver`** : Requêtes et mutations pour le crawl, le rendu de snapshot et l'extraction de métadonnées de page.
* **`FutResolver`** : Requêtes d'indexation et de recherche sur le marché FIFA Ultimate Team.

---

## 3. RPBEY Gacha Game Server API (Port 5050)

Le serveur de jeu gacha en temps réel (Colyseus + Express) expose les endpoints de jeu à l'adresse publique `api.rpbey.fr/gacha/` (cf. [`rpbey/apps/gacha-server/src/rest.ts`](file:///home/ubuntu/rpbey/apps/gacha-server/src/rest.ts)).

* **Temps réel (WebSockets)** : Route `/gacha` connectant l'état répliqué de la `GachaRoom`.
* **Économie & Inventaire (REST)** :
  * `GET /health` : État et version du service.
  * `POST /api/gacha/gift` : Envoi de cartes Beyblade à un ami.
  * `POST /api/gacha/wishlist/toggle` : Ajout/retrait d'une carte de la liste d'envies.
  * `GET /api/gacha/wishlist` : Récupère la wishlist de l'utilisateur.
  * `GET /api/gacha/history` : Historique des tirages récents.
  * `GET /api/gacha/rates` : Taux de drop actuels (commune, rare, super rare, etc.).
  * `GET /api/gacha/cards/search` : Recherche textuelle dans le catalogue de cartes.
  * `GET /api/gacha/banners` : Bannières de tirage actives.
  * `GET /api/gacha/badges` / `POST /api/gacha/badges/claim` : Succès et récompenses.
  * `GET /api/gacha/fusion/preview` / `POST /api/gacha/fusion` : Aperçu et fusion de doublons de cartes.
  * `GET /api/gacha/cards/:id` : Détails d'une carte Beyblade spécifique.
  * `GET /api/cards/:id/image.png` : Rendu de l'image de la carte (redirection 302 vers l'OG image dynamique du site principal).
* **Administration** :
  * `POST /api/admin/currency/grant` : Ajout manuel de devises virtuelles par un administrateur.

---

## 4. Mécanismes d'Authentification & Sécurité

* **Authentification Utilisateur (Better-Auth)** :
  * Intégré au Dashboard Next.js (`rpbey-web`, port `3002`).
  * Gère l'authentification tierce OAuth (Discord, Twitch, Google).
  * Les sessions actives génèrent un token persistant stocké dans la table `sessions` de PostgreSQL.
* **Authentification de Session de Jeu (Bearer Token)** :
  * Le serveur Gacha (`gacha-server`) sécurise ses routes REST et sa connexion WebSocket en extrayant le header `Authorization: Bearer <token>`.
  * Le token est vérifié directement contre la table partagée `sessions` dans la base `rpb_neon` (vérification de la validité temporelle et du statut banni de l'utilisateur).
* **Jetons & Clés Inter-Services** :
  * **`BOT_API_KEY`** : Clé partagée spécifiée dans les variables d'environnement permettant les appels sécurisés directs entre le bot Discord (`rpb-bot`, port `3001`) et le tableau de bord web.
  * **`RANKING_SYNC_TOKEN`** : Jeton autorisant le bot à pousser les résultats de tournois communautaires et à mettre à jour les ELOs sur le dashboard (`POST /api/admin/ranking/sync`).
