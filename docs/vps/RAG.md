<!-- SPDX-License-Identifier: Apache-2.0 -->
# Moteurs RAG & Inférence IA du VPS

Le VPS déploie une architecture RAG agentique hybride pour le crawling, l'indexation et la synthèse des connaissances.

---

## 1. RAG agentique hybride (écosystème Bxc / Autopilot)

Ce système alimente le crawling ciblé, l'analyse automatique de dépôt et la synthèse des connaissances des sous-agents agentiques.

### A. Génération d'Embeddings
* **Modèle** : Gemini API `gemini-embedding-001` (768 dimensions).
* **Fallback** : Si aucune clé API n'est disponible (mode hors-ligne), le système génère des vecteurs normalisés de bruit blanc pour les tests, évitant ainsi d'interrompre l'exécution.

### B. Algorithme de Recherche & Similarité Cosinus
Le moteur s'appuie sur une structure de base de données SQLite (`x-store.sqlite`) combinant :
* **Sémantique** : Recherche vectorielle Redis `VSIM` sur l'index `tweet_embeddings` (stockant l'ID et l'embedding normalisé).
* **Lexical** : Correspondance FTS5 MATCH sur la table virtuelle SQLite `tweets_fts`.
* **Formule de Score Hybride** :
  $$\text{Score} = (\text{Similarité Cosinus} \times 0.6) + (\text{Ratio Mots Clés} \times 0.4)$$
* **Boost de Phrases Exactes** :
  * Match exact dans le titre : $+0.25$ de boost de score.
  * Match exact dans le texte : $+0.12$ de boost de score.
* **Pondération Linguistique** (Ajuste le score selon la langue pour favoriser les résultats locaux) :
  * Français (`fr`) : $\times 1.35$
  * Anglais (`en`) : $\times 1.10$
  * Japonais (`ja`) : $\times 0.95$
  * Autres langues : $\times 0.40$

### C. Expansion Graphique de Contexte
Après avoir récupéré les tweets les plus pertinents :
1. Recherche des identifiants de fils de discussion (`conversation_id`).
2. Extraction de jusqu'à 10 tweets reliés de manière séquentielle pour reconstituer le fil conducteur de la discussion (le contexte temporel).
3. Formatage structuré intégrant l'auteur `@username`, son nom d'affichage, son nombre de likes et le texte.

### D. Synthèse LLM Cloud
* **Modèle** : `gemini-2.5-flash` via le SDK Google Generative AI (authentifié par compte de service ou clé d'API).
* **Prompt** : Synthèse analytique en français avec citations automatiques (ex: `[@username]`).
