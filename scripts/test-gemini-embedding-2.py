#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors

"""Script de test et de configuration complet pour Gemini Embedding 2.

Ce script vérifie la configuration de l'environnement, initialise le client
GenAI (soit via l'API Key, soit via les identifiants Vertex AI / ADC), et
génère un embedding de test pour valider le bon fonctionnement.
"""

import os
import sys

# Charger .env si présent dans le répertoire parent
try:
    from pathlib import Path
    dotenv_path = Path(__file__).resolve().parents[1] / ".env"
    if dotenv_path.exists():
        for line in dotenv_path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line and not line.startswith("#") and "=" in line:
                key, val = line.split("=", 1)
                os.environ.setdefault(key.strip(), val.strip())
except Exception as e:
    print(f"Avertissement lors de la lecture du fichier .env: {e}")

try:
    import numpy as np
except ImportError:
    np = None

try:
    from google import genai
    from google.genai import types
except ImportError:
    print("Erreur: Le SDK 'google-genai' n'est pas installé.")
    print("Veuillez installer les dépendances en exécutant:")
    print("  pip install google-genai numpy")
    print("  ou en utilisant uv:")
    print("  uv pip install google-genai numpy")
    sys.exit(1)

def test_gemini_embedding_2():
    # Détection de la configuration des credentials
    api_key = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
    adc_credentials = os.environ.get("GOOGLE_APPLICATION_CREDENTIALS")
    project_id = os.environ.get("GOOGLE_CLOUD_PROJECT") or os.environ.get("GCLOUD_PROJECT")
    location = os.environ.get("GOOGLE_CLOUD_LOCATION") or "us"

    print("--- Vérification de l'environnement ---")
    if adc_credentials:
        print(f"Identifiants de service (ADC) : {adc_credentials}")
        print(f"Projet Google Cloud : {project_id}")
        print(f"Localisation : {location}")
        # Initialisation via Vertex AI
        client = genai.Client(vertexai=True, project=project_id, location=location)
        print("Client configuré pour utiliser l'API Vertex AI (sans clé API).")
    elif api_key:
        print(f"Clé API configurée (longueur: {len(api_key)} caractères)")
        # Initialisation via Clé API (Google AI Studio)
        client = genai.Client(api_key=api_key)
        print("Client configuré pour utiliser l'API Google AI Studio avec clé API.")
    else:
        print("Erreur: Aucun credentials trouvé dans l'environnement.")
        print("Veuillez configurer soit 'GOOGLE_APPLICATION_CREDENTIALS' (pour Vertex AI),")
        print("soit 'GEMINI_API_KEY' (pour AI Studio) dans vos variables d'environnement ou le fichier .env.")
        sys.exit(1)

    print("\n--- Test 1 : Embedding de texte (Dimensions par défaut : 3072) ---")
    text_input = "task: sentence similarity | query: Bonjour, comment puis-je configurer Gemini Embedding 2 ?"
    print(f"Entrée : {text_input}")
    
    try:
        response = client.models.embed_content(
            model="gemini-embedding-2",
            contents=text_input
        )
        embedding = response.embeddings[0].values
        print(f"Succès ! Embedding généré avec succès.")
        print(f"Dimension obtenue : {len(embedding)}")
        if np:
            norm = np.linalg.norm(embedding)
            print(f"Norme L2 : {norm:.6f}")
        else:
            print("Note: Installez 'numpy' pour calculer la norme de l'embedding.")
    except Exception as e:
        print(f"Erreur lors de la génération de l'embedding : {e}")
        sys.exit(1)

    print("\n--- Test 2 : Réduction de dimensionnalité via Matryoshka (MRL) à 128 ---")
    try:
        response_reduced = client.models.embed_content(
            model="gemini-embedding-2",
            contents=text_input,
            config=types.EmbedContentConfig(output_dimensionality=128),
        )
        embedding_reduced = response_reduced.embeddings[0].values
        print(f"Succès ! Embedding réduit généré.")
        print(f"Dimension obtenue : {len(embedding_reduced)}")
        if np:
            norm_reduced = np.linalg.norm(embedding_reduced)
            print(f"Norme L2 : {norm_reduced:.6f} (doit être très proche de 1.0 car auto-normalisé)")
    except Exception as e:
        print(f"Erreur lors de la génération de l'embedding réduit : {e}")

    print("\nConfiguration et validation terminées avec succès !")

if __name__ == "__main__":
    test_gemini_embedding_2()
