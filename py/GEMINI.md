<!-- SPDX-License-Identifier: Apache-2.0 -->
# GEMINI.md

Guide de configuration et règles de développement pour le dépôt **aphrody-py**.
Ce fichier est un miroir de [CLAUDE.md](file:///C:/src/aphrody-py/CLAUDE.md).

**VPS deploy (Python systemd site `:8082`)** : voir [`../DEPLOY.md`](../DEPLOY.md) §3 et `py/aphrody/deploy/deploy-vps.sh`.

Veuillez vous référer à [CLAUDE.md](CLAUDE.md) pour :
1. La règle d'**Autonomie Totale** (zéro humain dans la boucle).
2. Les commandes de validation obligatoires (`uv run pytest -m "not live_api"`, `uv run ruff check`, `uv run ruff format`).
3. Les consignes de sécurité (permissions `0600`/`0700` et masquage des tokens).
4. Les consignes d'optimisation (lazy-loading des dépendances lourdes pour garder le démarrage de la CLI <100ms).
