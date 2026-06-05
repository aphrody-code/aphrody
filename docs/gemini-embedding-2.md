<!-- SPDX-License-Identifier: Apache-2.0 -->

# Gemini Embedding 2

Gemini Embedding 2 est le modèle de génération d'embeddings multimodaux de pointe de Google. Conçu pour des tâches complexes de récupération (retrieval), de classification, de regroupement (clustering) et de recommandation, il prend en charge des entrées combinant texte, images, audio, vidéo et documents (PDF). Le modèle mappe sémantiquement l'ensemble de ces modalités dans un espace vectoriel unifié et partagé de même dimensionnalité.

## Caractéristiques clés

- **Niveaux d'entrées acceptés** : Texte, images, audio, vidéo, documents (PDF).
- **Espace vectoriel unifié** : Permet de comparer ou de rechercher sémantiquement des modalités croisées (par exemple, rechercher une image ou un segment vidéo à l'aide d'une requête textuelle).
- **Matryoshka Representation Learning (MRL)** : La dimension par défaut est de 3 072, mais le modèle supporte des tailles de sortie ajustables (de 128 à 3 072 dimensions) tout en conservant une grande précision sémantique aux tailles inférieures.
- **Instructions de tâches personnalisées** : Permet d'optimiser la pertinence des embeddings en spécifiant la tâche attendue (par exemple, recherche, questions-réponses, classification, etc.) au sein du prompt d'entrée.
- **Reconnaissance Optique de Caractères (OCR)** : Capacité intégrée pour extraire le texte et l'analyser à partir de documents PDF ou d'images.
- **Extraction de pistes audio** : Permet d'extraire la piste audio d'une vidéo et de l'entrelacer avec les frames visuels pour une représentation multimodale complète.

---

## Spécifications Techniques et Limites

| Paramètre | Spécification / Limite |
|---|---|
| **Identifiant du modèle** | `gemini-embedding-2` / `gemini-embedding-2-preview` |
| **Date de sortie (GA)** | 22 avril 2026 (Preview depuis le 10 mars 2026) |
| **Limite de jetons d'entrée** | 8 192 jetons (troncation automatique au-delà) |
| **Dimensionnalité de sortie** | Ajustable de 128 à 3 072 (3 072 par défaut) |
| **Options de consommation** | Standard PayGo uniquement (non compatible avec le débit provisionné ou Flex PayGo) |
| **Régions disponibles** | us (États-Unis), eu (Europe), global (Monde) |
| **Date limite des connaissances** | Novembre 2025 |

### Limites par modalité

- **Images** : Maximum 6 images par requête. Taille maximale d'image de 16 384 x 16 384 pixels. Types MIME : `image/png`, `image/jpeg`, `image/webp`, `image/bmp`, `image/heic`, `image/heif`, `image/avif`.
- **Documents (PDF)** : Maximum 1 document par requête, limité à 6 pages. Format : `application/pdf`. (Recommandation : 1 page pour une qualité optimale).
- **Vidéo** : Maximum 1 vidéo par requête. Durée max de 80 secondes avec audio, et 120 secondes sans audio (basé sur 1 FPS). Types MIME : `video/mpeg`, `video/mp4`.
- **Audio** : Maximum 1 fichier audio par requête. Durée maximale de 180 secondes. Types MIME : `audio/mp3`, `audio/wav`.

---

## Calcul du coût en jetons d'entrée

Toutes les modalités partagent la fenêtre de contexte unique de 8 192 jetons. Le coût de chaque modalité est comptabilisé de la façon suivante :

- **Texte** : Standard (en fonction des tokens du tokenizer).
- **Audio** : 25 jetons par seconde.
- **Image** : 258 jetons par image.
- **Vidéo (visuel)** : 66 jetons par frame.
- **Document (PDF)** : Rendu sous forme d'image (258 jetons par page) + jetons supplémentaires pour le texte extrait via l'OCR.

### Exemple avec extraction audio active
Pour une vidéo de 1 FPS avec la piste audio extraite :
- Coût par seconde = 66 jetons (1 frame de vidéo) + 25 jetons (1 seconde d'audio) + 10 jetons (codes temporels) = 101 jetons par seconde.
- Durée maximale théorique = 8 192 / 101 ≈ 81 secondes maximum avant troncation.

---

## Structuration des instructions de tâches (Task Instructions)

Le modèle `gemini-embedding-2` n'utilise pas le paramètre `task_type` traditionnel des modèles plus anciens. Les tâches et la structure des documents doivent être directement fournies sous forme d'instructions textuelles selon les formats suivants :

### Cas d'utilisation de recherche et récupération (Format asymétrique)

Dans ces cas, le document recherché doit utiliser la structure de document.

| Tâche | Requête d'entrée (Query) | Structure du document (Document) |
|---|---|---|
| **Recherche générale** | `task: search result \| query: {contenu}` | `title: {titre} \| text: {contenu}` (si pas de titre, utiliser `title: none`) |
| **Questions-Réponses** | `task: question answering \| query: {contenu}` | `title: {titre} \| text: {contenu}` |
| **Vérification de faits (Fact-checking)** | `task: fact checking \| query: {contenu}` | `title: {titre} \| text: {contenu}` |
| **Recherche de code** | `task: code retrieval \| query: {contenu}` | `title: {titre} \| text: {contenu}` |

### Cas d'utilisation à entrée unique (Format symétrique)

| Tâche | Structure de l'entrée |
|---|---|
| **Classification** | `task: classification \| query: {contenu}` |
| **Clustering** | `task: clustering \| query: {contenu}` |
| **Similarité sémantique** | `task: sentence similarity \| query: {contenu}` (ne pas utiliser pour la recherche ou récupération) |

---

## Résultats des Benchmarks

Comparaison des performances de Gemini Embedding 2 face à d'autres modèles d'embeddings (valeurs issues des benchmarks officiels Google DeepMind) :

| Type de métrique | Nom de la métrique | Gemini Embedding 2 | gemini-embedding-001 (Legacy Text) | multimodalembedding@001 (Legacy Multimodal) | Amazon Nova 2 Multimodal | Voyage Multimodal 3.5 |
|---|---|---|---|---|---|---|
| **Text-Text** | MTEB (Multilingual) Mean | **69,9** | 68,4 | — | 63,8 | 58,5 |
| | MTEB (Code) Mean | **84,0** | 76,0 | — | * | * |
| **Text-Image** | TextCaps recall@1 | **89,6** | — | 74,0 | 76,0 | 79,4 |
| | Docci recall@1 | **93,4** | — | 84,0 | 83,8 | * |
| **Image-Text** | TextCaps recall@1 | **97,4** | — | 88,1 | 88,9 | 88,6 |
| | Docci recall@1 | **91,3** | — | 76,5 | 77,4 | * |
| **Text-Document** | ViDoRe v2 ndcg@10 | 64,9 | — | 28,9 | 60,6 | **65,5** |
| **Text-Video** | Vatex ndcg@10 | **68,8** | — | 54,9 | 60,3 | 55,2 |
| | MSR-VTT ndcg@10 | **68,0** | — | 57,9 | 67,0 | 63,0 |
| | Youcook2 ndcg@10 | **52,5** | — | 34,9 | 34,7 | 31,4 |
| **Speech-Text** | MSEB mrr@10 | **73,9** | — | — | * | — |
| | MSEB (ASR) mrr@10 | **70,4** | — | — | * | — |

*(* score non disponible / non communiqué)*

---

## Exemples d'intégration et d'usage

### Exemple Python (SDK Google GenAI)

Voici comment initialiser le client et générer un embedding avec le SDK officiel `google-genai` pour une entrée audio :

```python
from google import genai
from google.genai import types

# Initialisation du client avec l'environnement Vertex AI
client = genai.Client(vertexai=True, project="YOUR_PROJECT_ID", location="us")

# Définition du contenu multimodal (texte + audio)
content = types.Content(
    parts=[
        types.Part.from_text(text="Audio AI"),
        types.Part.from_uri(
            file_uri="gs://cloud-samples-data/generative-ai/audio/Chirp-3-Docs-Dive.mp3",
            mime_type="audio/mpeg",
        ),
    ],
)

# Appel de l'API pour générer l'embedding
response = client.models.embed_content(
    model="gemini-embedding-2",
    contents=[content]
)

# Récupération du vecteur d'embedding
embedding_vector = response.embeddings[0].values
print(f"Dimension de l'embedding : {len(embedding_vector)}")
```

### Paramétrage de la dimensionnalité (MRL) et normalisation L2

```python
import numpy as np
from google import genai
from google.genai import types

client = genai.Client(vertexai=True, project="YOUR_PROJECT_ID", location="us")

# Demander une dimension réduite à 128
response = client.models.embed_content(
    model="gemini-embedding-2",
    contents=[content],
    config=types.EmbedContentConfig(output_dimensionality=128),
)

embedding_values_np = np.array(response.embeddings[0].values)
print(f"Longueur : {len(embedding_values_np)}")
# Les embeddings réduits de gemini-embedding-2 sont déjà normalisés L2 par défaut
print(f"Norme L2 : {np.linalg.norm(embedding_values_np):.6f}") # Devrait être très proche de 1.0
```

### Métadonnées et échantillonnage vidéo

```python
from google import genai
from google.genai import types

client = genai.Client(vertexai=True, project="YOUR_PROJECT_ID", location="us")

content = types.Content(
    parts=[
        types.Part(
            file_data=types.FileData(
                file_uri="gs://cloud-samples-data/generative-ai/video/pixel8.mp4",
                mime_type="video/mp4",
            ),
            # Spécification des offsets de début/fin et du taux de FPS
            video_metadata=types.VideoMetadata(
                fps=0.5,
                start_offset="10s",
                end_offset="20s",
            ),
        ),
    ]
)

response = client.models.embed_content(
    model="gemini-embedding-2",
    contents=[content]
)
```

### Exemple REST (cURL)

Appel de l'API REST via cURL en utilisant des données stockées sur Google Cloud Storage :

```bash
PROJECT_ID="YOUR_PROJECT_ID"
LOCATION="us"

curl -X POST \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  -H "Content-Type: application/json" \
  "https://aiplatform.${LOCATION}.rep.googleapis.com/v1/projects/${PROJECT_ID}/locations/${LOCATION}/publishers/google/models/gemini-embedding-2:embedContent" \
  -d '{
  "content": {
    "parts": [
      {
        "text": "Whats this"
      },
      {
        "file_data": {
          "mime_type": "video/mp4",
          "file_uri": "gs://cloud-samples-data/generative-ai/video/pixel8.mp4"
        }
      }
    ]
  }
}'
```

---

## Liens et Références Associés

- [RAG.md](file:///home/ubuntu/aphrody/docs/RAG.md) : Modèle conceptuel du pipeline RAG d'Aphrody.
- [rag-unified-pattern.md](file:///home/ubuntu/aphrody/docs/rag-unified-pattern.md) : Contrat d'interface et d'intégration RAG unifié.
