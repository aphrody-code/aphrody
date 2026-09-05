<!-- SPDX-License-Identifier: Apache-2.0 -->
# Google Cloud — Workload Identity Federation (WIF) Setup

Ce guide décrit comment configurer la fédération d'identité de charge de travail (Workload Identity Federation) pour permettre à GitHub Actions d'interagir de manière sécurisée et sans clé d'API statique avec le projet Google Cloud **aphrody**.

---

## 1. Concepts & Avantages
La fédération d'identité permet d'utiliser des jetons OIDC (OpenID Connect) éphémères émis par GitHub Actions pour s'authentifier auprès de Google Cloud.
* **Aucun secret persistant** : Aucun fichier `secrets/aphrody-bot.json` n'est enregistré dans les secrets GitHub.
* **Expiration automatique** : Les jetons ont une durée de vie maximale d'une heure.
* **Auditabilité** : Chaque action CI/CD est liée à un jeton d'identité unique et traçable.

---

## 2. Configuration Google Cloud (gcloud CLI)

Exécutez ces étapes avec un compte utilisateur propriétaire (`roles/owner`) dans le projet `aphrody`.

### Étape A : Activer l'API de fédération d'identité de domaine
```bash
gcloud services enable iamcredentials.googleapis.com --project=aphrody
```

### Étape B : Créer un pool d'identité de charge de travail (Workload Identity Pool)
```bash
gcloud iam workload-identity-pools create "github-pool" \
    --project="aphrody" \
    --location="global" \
    --display-name="GitHub Action Pool"
```

### Étape C : Créer un fournisseur de pool d'identité (Workload Identity Provider)
Associez le fournisseur d'identité OIDC de GitHub au pool créé.
```bash
gcloud iam workload-identity-pools providers create-oidc "github-provider" \
    --project="aphrody" \
    --location="global" \
    --workload-identity-pool="github-pool" \
    --display-name="GitHub Action Provider" \
    --issuer-uri="https://token.actions.githubusercontent.com" \
    --attribute-mapping="google.subject=assertion.subject,attribute.actor=assertion.actor,attribute.repository=assertion.repository"
```

### Étape D : Autoriser le dépôt GitHub à usurper le compte de service
Liez le compte de service `aphrody-bot` à l'identité GitHub. Remplacez `<owner>/<repo>` par votre dépôt (ex. `aphrody-code/aphrody` ou `rose-griffon/rg`).
```bash
# Récupérer l'identifiant complet du pool
POOL_ID=$(gcloud iam workload-identity-pools describe "github-pool" \
    --project="aphrody" \
    --location="global" \
    --format="value(name)")

# Autoriser le dépôt GitHub
gcloud iam service-accounts add-iam-policy-binding "aphrody-bot@aphrody.iam.gserviceaccount.com" \
    --project="aphrody" \
    --role="roles/iam.workloadIdentityUser" \
    --member="principalSet://iam.googleapis.com/${POOL_ID}/attribute.repository/<owner>/<repo>"
```

---

## 3. Configuration du Workflow GitHub Actions

Configurez le fichier de workflow YAML (`.github/workflows/ci.yml`) dans votre dépôt.

### Permissions du workflow
Le workflow doit obligatoirement avoir les permissions d'écriture pour `id-token` afin de pouvoir demander le jeton OIDC à GitHub.
```yaml
permissions:
  contents: 'read'
  id-token: 'write'
```

### Étape d'authentification
Utilisez l'action officielle `google-github-actions/auth` pour acquérir le jeton d'accès temporaire GCP.
```yaml
jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
    - name: Checkout Code
      uses: actions/checkout@v4

    - name: Authenticate to Google Cloud
      uses: google-github-actions/auth@v2
      with:
        workload_identity_provider: 'projects/468000409790/locations/global/workloadIdentityPools/github-pool/providers/github-provider'
        service_account: 'aphrody-bot@aphrody.iam.gserviceaccount.com'

    - name: Set up Cloud SDK
      uses: google-github-actions/setup-gcloud@v2

    - name: Use gcloud CLI
      run: |
        gcloud info
        gcloud storage ls
```

---

## 4. Runbook de Diagnostic

| Problème | Cause Probable | Solution |
|----------|----------------|----------|
| `Missing id-token permission` | Le workflow n'a pas la permission `id-token: write` | Vérifiez que les permissions sont explicitement déclarées dans le fichier YAML. |
| `Could not identity federate` | Dépôt non autorisé dans les IAM bindings | Vérifiez que le champ `attribute.repository` correspond exactement au dépôt GitHub configuré dans la politique IAM. |
| `Token expired` | Le jeton d'accès a expiré (limite 1h) | Si votre tâche dure plus d'une heure, configurez `access_token_lifetime` dans l'action d'authentification. |
