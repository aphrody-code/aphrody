<!-- SPDX-License-Identifier: Apache-2.0 -->
# AGENTS.md — règles aphrody pour le CLI `agy`

Contexte injecté à chaque session `agy` quand l'extension `aphrody-agy` est
active. Discipline de codage **autonome, no-human-in-the-loop**.

## Mode autonome permanent (type /goal)

Tu opères en boucle continue : un objectif persiste jusqu'à complétion. Ne
t'arrête JAMAIS pour demander une confirmation ; décide seul et documente. Le
hook `AfterAgent` (`aphrody agy-loop hook`) te relance tant que le jeton
`APHRODY_LOOP_DONE` n'est pas émis.

## Règles de code

- **Zéro stub, 100 % production** : chaque fonction touchée contient sa logique
  réelle. Aucun `TODO: implement later`, aucun mock.
- **Cross-platform** : Linux #1, puis Windows, puis macOS. Le code OS-spécifique
  est gardé (`cfg`/détection runtime) et ne casse jamais les autres cibles.
- **Vérifie réellement** : `cargo check` ne suffit pas — lance build + tests
  (`clippy -D warnings`, tests) et corrige avant de continuer.
- **Commits** : Conventional Commits, au fil de l'eau.
- **Secrets** : jamais de clé/token en clair dans un fichier tracké.

## Boucle

- Démarrer : `/grind <objectif>` (arme `aphrody agy-loop start`).
- Terminer : `/ship` (désarme + finalise) ou émettre `APHRODY_LOOP_DONE`.
- Arrêt d'urgence : `aphrody agy-loop stop`.
- État : `aphrody agy-loop status`.

## Outils MCP aphrody

Le serveur MCP `aphrody` (`aphrody-mcp`) est monté par l'extension : docs
(`docs_auto_search`, Context7, Microsoft Learn), recon, RE triage, Gemini web,
voix, vision. Utilise `docs_auto_search` AVANT toute question lib/API.
