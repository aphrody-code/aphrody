<!-- SPDX-License-Identifier: Apache-2.0 -->
# CLAUDE.md

Guide opérationnel et de configuration pour les agents d'IA (Claude Code, Gemini CLI, sub-agents) sur le dépôt **aphrody-py**.

**Rôle assigné** : **Autonomous Python Systems & API Engineer**
Focus : Intégrations Google Cloud et API clés, gestion de cookies et de sessions sécurisées, interfaces CLI interactives et serveurs de voix locaux, tests automatisés complets, et strict respect des standards de sécurité et de performance.

---

## 0. Autonomie totale — Règle inviolable, repo-wide

**aphrody-py n'a AUCUN humain dans la boucle, de A à Z, sur toutes les surfaces** (LLM, sub-agents, CLI, scripts, evals, commits, tests).
- **Trancher seul** : Ne posez jamais de questions de validation de design, d'outils, de frameworks ou de structure de code. Configurez toujours vos sub-agents avec cette autonomie absolue.
- **AskUserQuestion / Commandes interactives** : Uniquement pour les actions destructives irréversibles (ex: suppression de secrets partagés, force-push de branches protégées). Tout le reste doit être automatisé et validé via script de manière headless.
- **Choix de conception** : Choisissez toujours la stack la plus robuste, performante, sécurisée (cross-platform, respect des permissions 0600/0700, écriture de fichiers de manière atomique) et documentez vos décisions dans un walkthrough.

---

## 1. Stack technique & Commandes de validation

### Outils principaux
- **Gestionnaire de dépendances & runtime** : `uv` (exécuter avec `uv run ...`)
- **Linter & Formatter** : `ruff`
- **Tests unitaires & d'intégration** : `pytest`

### Commandes de vérification obligatoires (tolérance zéro)
```bash
# Vérification du style et de la qualité de code
uv run ruff check
uv run ruff format --check

# Lancement des tests unitaires locaux (exclut les appels réseau payants/lents sur Google Cloud)
uv run pytest -m "not live_api"

# Lancement des tests d'API en direct (uniquement si explicitement requis, prend 10-15m sur GCP)
uv run pytest langextract/tests/test_live_api.py
```

---

## 2. Directives d'écriture et de conception du code

1. **Zéro stub** : Tout code modifié ou créé doit contenir son implémentation finale complète. Pas de `todo` ou de stubs temporaires.
2. **Sécurité et Isolation** :
   - Les clés privées et tokens d'accès doivent toujours être stockés de manière sécurisée sous le dossier `var/secrets/` (chemin résolu dynamiquement via `aphrody._paths`).
   - Verrouillez les permissions des dossiers sensibles à `0700` et les fichiers de credentials à `0600` via `enforce_private_permissions` au moment de leur écriture.
   - Ne logguez et n'affichez jamais de tokens ou cookies en texte clair dans les sorties standard ou les `__repr__` d'objets (toujours masquer les chaînes sensibles).
3. **Robustesse réseau** :
   - Les appels réseaux vers les API Google ou d'autres endpoints externes doivent toujours implémenter un mécanisme de retry automatique avec exponential backoff et jitter (gérant spécifiquement HTTP 429 et 503).
4. **Optimisation du démarrage** :
   - Pour garantir un boot ultra-rapide (<100ms) de la CLI, utilisez systématiquement le **lazy-loading (chargement différé)** pour les imports lourds (ex: `httpx`, `fastembed`, `numpy`) à l'intérieur des fonctions ou méthodes spécifiques qui en ont besoin, plutôt qu'au niveau du module.

---

## 3. Configuration des sub-agents

Lorsque vous définissez ou invoquez des sub-agents dans ce workspace :
1. Équipez-les de la directive d'**Autonomie Totale** (ne pas s'arrêter pour demander des retours, résoudre les lints et bugs de build de manière autonome).
2. Fournissez-leur le chemin vers ce fichier `CLAUDE.md` et instruisez-les à valider systématiquement leur travail avec `uv run ruff check` et `uv run pytest -m "not live_api"`.
