<!-- SPDX-License-Identifier: Apache-2.0 -->
# Architecture et Topologie du VPS

Ce document présente l'architecture globale et l'orchestration des services hébergés sur le serveur VPS principal de production (Ubuntu 26.04). Le VPS fait tourner les écosystèmes interconnectés `aphrody`, `bxc` et `rg` / Azalée, supportés par des infrastructures locales d'inférence IA, de base de données relationnelle PostgreSQL, de cache Redis et de stockage SQLite.

## Table des Matières

1. [Infrastructures & Services Système](#infrastructures--services-système)
2. [Écosystème Azalée (`rg`)](#écosystème-azalée-rg)
3. [Écosystème Bxc / Autopilot (`aphrody`)](#écosystème-bxc--autopilot-aphrody)
4. [Réseau & Reverse-Proxies Nginx](#réseau--reverse-proxies-nginx)

---

## Infrastructures & Services Système

Le serveur s'appuie sur plusieurs services système managés via `systemd` :

* **Base de données relationnelle** :
  * `postgresql@18-main.service` : Serveur PostgreSQL 18 local écoutant sur le port `5432` et via socket Unix (`/var/run/postgresql`). Utilisé pour le stockage principal et l'authentification partagée.
* **Cache & Base de données clé-valeur** :
  * `redis-server.service` : Instance Redis locale (port `6379`). Utilisée pour le cache rapide, les files d'attente hors-ligne et le stockage vectoriel léger (VSIM).
---

## Écosystème Azalée (`rg`)

Géré dans `/home/ubuntu/rg`, c'est la plateforme d'organisation communautaire :

* **`azalee-web.service`** : Dashboard web Next.js 16 configuré pour tourner sur le port `3003`.
* **`rg-bot.service`** : Bot Discord communautaire s'exécutant depuis `/home/ubuntu/rg-bot-prod` et utilisant Drizzle ORM avec une instance de base de données PostgreSQL distante (Supabase Cloud) et une DB Redis locale (`redis://127.0.0.1:6379/1`).
* **`rg-cron.service`** : Service d'exécution des tâches d'arrière-plan et d'agrégation statistique pour Azalée.

---

## Écosystème Bxc / Autopilot (`aphrody`)

Le binaire principal `aphrody` et son moteur de scraping/navigation `bxc` orchestrent les processus autonomes du VPS :

* **`bxc.service`** : Serveur d'automatisation de navigateur Zero-Spawn (Bun + V8 bindings + Lightpanda fusion). Il écoute sur le port `3000` (Elysia + Yoga) pour exécuter des tâches de reconnaissance web, de RAG et de scraping.
* **Autopilot & RAG** : Des scripts programmés en tâche de fond (`run-targeted-crawler.ts`, `run-index-embeddings.ts`) maintiennent des stores de vecteurs locaux via SQLite et Redis pour alimenter la recherche sémantique des agents.
* **Agents unifiés (2026-06)** : Claude Code, Grok Build, Gemini (`agy`) partagent `~/.config/aphrody/mcp.json` (`aphrody-mcp` + `bxc-mcp`). Doc : [`docs/agent-stack/README.md`](../agent-stack/README.md). Sync : `bash scripts/vps-sync-agent-stack.sh`.
* **X Pro + Radar** : `pro.x.com/i/decks` (Gryphon GraphQL), `x.com/i/radar` — `@aphrody-code/x` 1.0.6 et MCP `bxc_xpro_deck`.

---

## Réseau & Reverse-Proxies Nginx

Nginx (écoutant sur les ports `80` et `443` avec SSL Certbot) sert de reverse-proxy frontal pour acheminer les requêtes publiques vers les ports loopback des applications :

| Domaine public | Protocole | Service interne / Cible | Port VPS |
|---|---|---|---|
| `azalee.rosegriffon.fr` | HTTPS | `azalee-web` (Next.js) | `3003` |
| `127.0.0.1` | Local loopback | `bxc-api` (Elysia REST/GraphQL) | `3000` |

---

## Documents Détaillés

Pour obtenir plus d'informations spécifiques sur les différentes couches, consultez les fiches techniques suivantes :

* 📁 **[Databases & Stockage](file:///home/ubuntu/aphrody/docs/vps/DATABASES.md)** : Fichiers SQLite, caches Redis, schémas PostgreSQL et Drizzle ORM.
* 📁 **[Moteurs de RAG & Inférence](file:///home/ubuntu/aphrody/docs/vps/RAG.md)** : Recherche hybride, vectorisation E5 / Gemini et modèles de synthèse LLM locaux.
* 📁 **[API REST & GraphQL](file:///home/ubuntu/aphrody/docs/vps/APIS.md)** : Endpoints, serveurs d'API (Elysia, Yoga, Colyseus), routage Nginx et politiques d'authentification.
