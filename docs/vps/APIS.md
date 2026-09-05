<!-- SPDX-License-Identifier: Apache-2.0 -->
# REST & GraphQL APIs du VPS

Ce document détaille les interfaces de programmation (APIs) exposées, configurées ou consommées sur le VPS, y compris la matrice des ports, les schémas d'authentification et le routage frontal Nginx.

---

## 1. Matrice des Ports & Services API

Voici l'inventaire des ports d'écoute configurés sur le VPS pour l'ensemble des APIs :

| Port | Protocole | Service interne | Type d'API / Rôle | Accès public / Proxy |
|---|---|---|---|---|
| `3000` | HTTP / WSS | `bxc.service` | REST & GraphQL Bxc (Elysia + Yoga) | Loopback local (`127.0.0.1`) |
| `3003` | HTTP | `azalee-web.service` | Standalone Next.js 16 Azalée | Proxy via `azalee.rosegriffon.fr` |
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

## 3. Mécanismes d'authentification et de sécurité

* `bxc.service` et le listener A2A restent liés au loopback ; Nginx ne doit pas les exposer directement.
* Les routes publiques sont authentifiées à leur frontière applicative et n'acceptent aucun secret dans l'URL.
* Les jetons inter-services vivent dans des fichiers d'environnement non suivis, avec permissions minimales et rotation explicite.
