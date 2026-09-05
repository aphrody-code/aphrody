<!-- SPDX-License-Identifier: Apache-2.0 -->
# Google Cloud — Cost Optimization & Always Free Tier Runbook

Ce document détaille les stratégies et configurations permettant d'exploiter les services de Google Cloud Platform (GCP) à **coût nul ($0.00/mois)** dans le cadre du projet **aphrody** et ses dépendances.

---

## 1. Google Cloud Always Free Tier

GCP propose un catalogue de services avec des quotas gratuits mensuels récurrents. Pour en bénéficier, le projet doit être lié à un compte de facturation actif (pour éviter les abus), mais aucune facturation n'aura lieu tant que les limites ne sont pas dépassées.

### Limites Clés à us-central1 (Région par défaut du projet)

| Service | Seuil Gratuit Mensuel | Usage cible dans le projet |
|---------|------------------------|---------------------------|
| **Cloud Run** | 2 millions de requêtes, 360 000 Go-secondes | Hébergement d'APIs de bots ou de passerelles MCP. |
| **Cloud Storage** | 5 Go de stockage régional (U.S. uniquement) | Backups du store de tweets et fichiers de session. |
| **Cloud Functions** | 2 millions d'invocations, 400 000 Go-secondes | Tâches de cron légères et webhooks d'événements. |
| **Firestore** | 1 Go de stockage, 50k lectures, 20k écritures/jour | Stockage de clés/valeurs et métadonnées légères. |
| **Secret Manager** | 6 secrets actifs, 10 000 requêtes d'accès | Chiffrement sécurisé et injection de jetons runtime. |
| **Cloud Build** | 120 minutes de build par jour | Intégration continue et compilation Docker. |

---

## 2. Optimisation des coûts de l'API Gemini

Le client RAG et l'agent d'enrichissement exploitent fortement les modèles de langage de Google. Pour éviter les coûts de facturation :

### A. Utiliser l'API Developer (Google AI Studio) via API Key
* **Règle d'or** : Privilégiez l'endpoint `generativelanguage` avec la clé API (`GOOGLE_API_KEY` / `GEMINI_API_KEY`) plutôt que l'API payante d'entreprise Vertex AI (`aiplatform`).
* Le tier gratuit de l'API Developer fournit un accès gratuit pour les modèles **Flash** and **Flash-Lite** (ex: `gemini-3-flash-lite`, `gemini-2.5-flash`).
* **⚠️ Isolation de projet** : N'activez pas la facturation sur le projet GCP hébergeant la clé API de développement. Si la facturation est activée sur un projet, les appels Gemini sur ce projet deviennent immédiatement payants dès le premier jeton. Utilisez un projet bac à sable non facturé distinct pour les tests d'inférence gratuits.

### B. Mettre en cache les contextes (Context Caching)
Pour les boucles d'agents de codage autonomes (`agy-loop`) qui soumettent à chaque tour la même structure de code ou de base de connaissances :
* **Réduction de coût** : Jusqu'à **90%** de remise sur les jetons d'entrée.
* **Seuil** : Le cache est applicable pour les invites de plus de 32 768 jetons et persiste pendant un temps spécifié (TTL).

### C. Soumissions par lots (Batch API)
Pour les tâches de maintenance non urgentes (ex: classification nocturne des tweets ou enrichissement du métagame) :
* **Réduction de coût** : **50% de réduction** sur les coûts standard.
* **Fonctionnement** : Les requêtes sont traitées de manière asynchrone sous 24 heures.

---

## 3. Configuration des Alertes de Budget (Garde-fous)

Pour garantir la sécurité financière absolue et éviter le "bill shock" :

### Étape 1 : Créer un budget de $1.00
1. Rendez-vous dans la console GCP : **Facturation > Budgets et alertes**.
2. Créez un budget nommé `Budget-Alerte-Aphrody`.
3. Définissez le type de budget sur **Spécifié** et le montant sur **$1.00**.

### Étape 2 : Définir des déclencheurs de notification
Configurez des alertes par e-mail basées sur le pourcentage de dépenses réelles ou prévues :
* **50% ($0.50)** : Dépense réelle (Alerte informative).
* **90% ($0.90)** : Dépense réelle (Alerte de sécurité).
* **100% ($1.00)** : Dépense réelle ou prévue (Arrêt / désactivation manuelle des instances non-free).

---

## 4. Runbook de Nettoyage Automatique

Pour s'assurer que les ressources temporaires ou les images Docker ne dépassent pas la limite gratuite d'Artifact Registry (500 Mo) ou de Cloud Storage (5 Go) :

### Politique de cycle de vie Cloud Storage
Ajoutez une règle de cycle de vie (`lifecycle.json`) pour supprimer automatiquement les snapshots obsolètes au-delà de 30 jours.
```json
{
  "rule": [
    {
      "action": {"type": "Delete"},
      "condition": {
        "age": 30,
        "isLive": true
      }
    }
  ]
}
```
Appliquez-la au bucket de stockage :
```bash
gcloud storage buckets update gs://aphrody-backups --lifecycle-file=lifecycle.json
```
