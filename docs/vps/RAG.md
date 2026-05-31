<!-- SPDX-License-Identifier: Apache-2.0 -->
# Moteurs RAG & Inférence IA du VPS

Le VPS déploie deux architectures distinctes de RAG (Retrieval-Augmented Generation) optimisant soit l'exécution locale pour respecter la souveraineté des données, soit l'intégration Cloud pour des performances étendues.

---

## 1. RAG Souverain (Écosystème RPBEY)

Ce pipeline est conçu pour fonctionner de manière autonome sur le VPS sans dépendre d'API payantes tierces, en combinant une recherche vectorielle hybride locale et un LLM auto-hébergé.

### A. Pipeline d'Embeddings (Dense)
* **Modèle** : `Xenova/multilingual-e5-small` (384 dimensions) chargé en mémoire CPU via ONNX Runtime et Transformers.js.
* **Sidecar** : `rpbey-embed.service` écoutant sur `http://127.0.0.1:7077`. Ce service isole les bibliothèques d'inférence natives de Next.js pour éviter des échecs de compilation de bundle.
* **Convention de formatage** : Ajout du préfixe `query: ` pour les requêtes utilisateurs et `passage: ` pour les documents indexés.
* **Persistance** : Les embeddings générés en masse par lot via `scripts/build-search-vectors.ts` sont stockés dans le set Redis `rpbey:search:vec` sous forme de BLOB Float32.

### B. Algorithme de Recherche Hybride
À la réception d'une question sur `POST /api/chat` :
1. **Dense Search** : La requête est vectorisée en 384 dimensions par le sidecar. Un appel Redis `VSIM` récupère les identifiants des $N$ documents les plus proches sémantiquement.
2. **Lexical Search** : Une recherche par mot-clé (BM25F) est lancée parallèlement.
3. **Fusion (RRF)** : Les résultats du sémantique et du lexical sont fusionnés en calculant un score de réciprocité de rang (RRF - Reciprocal Rank Fusion) avec $k = 60$.
4. **Fallback** : Si Redis ou le sidecar d'embeddings ne répond pas, le système dégrade proprement en mode BM25F lexical pur pour éviter toute coupure de service.

### C. Inférence & Génération (LLM)
* **Moteur** : `llama.cpp` (exécuté par `rpbey-llm.service` sur le port `8080`).
* **Modèle** : `Llama-3.2-3B Q4` (fichier de poids local quantifié au format GGUF).
* **Protocole** : API compatible OpenAI (`/v1/chat/completions`).
* **Rendu** : Streaming en flux SSE (Server-Sent Events) pour masquer le temps de réponse CPU (~6 s de pré-remplissage à 11 tokens/seconde). L'historique des conversations est renvoyé par le client pour garder le service stateless.
* **Repli** : Un kill switch `RPBEY_CHAT_LLM=0` bascule automatiquement vers une réponse extractive déterministe si le démon LLM local s'arrête.

---

## 2. RAG Agentique Hybride (Écosystème Bxc / Autopilot)

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
