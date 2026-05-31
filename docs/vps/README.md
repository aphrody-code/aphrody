<!-- SPDX-License-Identifier: Apache-2.0 -->
# Architecture et Topologie du VPS

Ce document présente l'architecture globale et l'orchestration des services hébergés sur le serveur VPS principal de production (Ubuntu 26.04). Le VPS fait tourner trois écosystèmes majeurs interconnectés (`aphrody`, `rpbey`, et `rg` / Azalée), supportés par des infrastructures locales d'inférence IA, de base de données relationnelle PostgreSQL, de cache Redis et de stockage SQLite.

## Table des Matières

1. [Infrastructures & Services Système](#infrastructures--services-système)
2. [Écosystème RPBEY (République Populaire du Beyblade)](#écosystème-rpbey-république-populaire-du-beyblade)
3. [Écosystème Azalée (`rg`)](#écosystème-azalée-rg)
4. [Écosystème Bxc / Autopilot (`aphrody`)](#écosystème-bxc--autopilot-aphrody)
5. [Réseau & Reverse-Proxies Nginx](#réseau--reverse-proxies-nginx)

---

## Infrastructures & Services Système

Le serveur s'appuie sur plusieurs services système managés via `systemd` :

* **Base de données relationnelle** :
  * `postgresql@18-main.service` : Serveur PostgreSQL 18 local écoutant sur le port `5432` et via socket Unix (`/var/run/postgresql`). Utilisé pour le stockage principal et l'authentification partagée.
* **Cache & Base de données clé-valeur** :
  * `redis-server.service` : Instance Redis locale (port `6379`). Utilisée pour le cache rapide, les files d'attente hors-ligne et le stockage vectoriel léger (VSIM).
* **Serveurs d'Inférence IA Locaux (Sidecars)** :
  * `rpbey-embed.service` : Moteur d'embeddings local (`Xenova/multilingual-e5-small`, 384 dimensions) propulsé par Transformers.js/ONNX et Bun. Écoute sur `127.0.0.1:7077`.
  * `rpbey-llm.service` : Serveur d'inférence LLM local (`llama-server` avec le modèle `Llama-3.2-3B Q4` quantifié). Écoute sur `127.0.0.1:8080` avec une API compatible OpenAI.

---

## Écosystème RPBEY (République Populaire du Beyblade)

L'écosystème RPBEY est géré au sein du dépôt monorepo `/home/ubuntu/rpbey` et est déployé à travers plusieurs services Bun autonomes :

* **`rpbey-web.service`** : Dashboard web principal construit avec **Next.js 16** (App Router, Turbopack, standalone). Écoute sur le port loopback `3002` et utilise `better-auth` pour la gestion des sessions utilisateur.
* **`rpb-bot.service`** : Bot Discord algorithmique basé sur `discordx` (écrit en TypeScript). Écoute sur le port `3001` pour son API HTTP locale.
* **`rpbey-gacha.service`** : Serveur de jeu gacha en temps réel utilisant **Colyseus 0.17** monté sur un runtime Bun (BunWebSockets). Il écoute sur le port `5050` et est backé par la base de données PostgreSQL partagée.
* **`cdn-assets-refresh.service`** : Service périodique rafraîchissant les assets statiques et pré-compressant les ressources pour le bot et le gacha.

---

## Écosystème Azalée (`rg`)

Géré dans `/home/ubuntu/rg`, c'est la plateforme d'organisation communautaire :

* **`azalee-web.service`** : Dashboard web Next.js 16 similaire au setup de RPBEY, configuré pour tourner sur le port `3003`.
* **`rg-bot.service`** : Bot Discord communautaire s'exécutant depuis `/home/ubuntu/rg-bot-prod` et utilisant Drizzle ORM avec une instance de base de données PostgreSQL distante (Supabase Cloud) et une DB Redis locale (`redis://127.0.0.1:6379/1`).
* **`rg-cron.service`** : Service d'exécution des tâches d'arrière-plan et d'agrégation statistique pour Azalée.

---

## Écosystème Bxc / Autopilot (`aphrody`)

Le binaire principal `aphrody` et son moteur de scraping/navigation `bxc` orchestrent les processus autonomes du VPS :

* **`bxc.service`** : Serveur d'automatisation de navigateur Zero-Spawn (Bun + V8 bindings + Lightpanda fusion). Il écoute sur le port `3000` (Elysia + Yoga) pour exécuter des tâches de reconnaissance web, de RAG et de scraping.
* **Autopilot & RAG** : Des scripts programmés en tâche de fond (`run-targeted-crawler.ts`, `run-index-embeddings.ts`) maintiennent des stores de vecteurs locaux via SQLite et Redis pour alimenter la recherche sémantique des agents.

---

## Réseau & Reverse-Proxies Nginx

Nginx (écoutant sur les ports `80` et `443` avec SSL Certbot) sert de reverse-proxy frontal pour acheminer les requêtes publiques vers les ports loopback des applications :

| Domaine public | Protocole | Service interne / Cible | Port VPS |
|---|---|---|---|
| `rpbey.fr` / `www.rpbey.fr` | HTTPS | `rpbey-web` (Next.js) | `3002` |
| `api.rpbey.fr/gacha/` | WSS / HTTPS | `rpbey-gacha` (Colyseus) | `5050` |
| `azalee.rosegriffon.fr` | HTTPS | `azalee-web` (Next.js) | `3003` |
| `api.rpbey.fr` | HTTPS | `rpb-bot` (Express/Bun API) | `3001` |
| `127.0.0.1` | Local loopback | `bxc-api` (Elysia REST/GraphQL) | `3000` |
| `127.0.0.1` | Local loopback | `embed-sidecar` (E5 Embedding API) | `7077` |
| `127.0.0.1` | Local loopback | `llama-server` (Llama.cpp OpenAI API) | `8080` |

---

## Documents Détaillés

Pour obtenir plus d'informations spécifiques sur les différentes couches, consultez les fiches techniques suivantes :

* 📁 **[Databases & Stockage](file:///home/ubuntu/aphrody/docs/vps/DATABASES.md)** : Fichiers SQLite, caches Redis, schémas PostgreSQL et Drizzle ORM.
* 📁 **[Moteurs de RAG & Inférence](file:///home/ubuntu/aphrody/docs/vps/RAG.md)** : Recherche hybride, vectorisation E5 / Gemini et modèles de synthèse LLM locaux.
* 📁 **[API REST & GraphQL](file:///home/ubuntu/aphrody/docs/vps/APIS.md)** : Endpoints, serveurs d'API (Elysia, Yoga, Colyseus), routage Nginx et politiques d'authentification.
