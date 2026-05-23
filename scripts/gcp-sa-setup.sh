#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# gcp-sa-setup.sh — provisionnement idempotent du service account aphrody-bot.
#
# Miroir Linux (cible #1) de gcp-sa-setup.ps1 : crée/réutilise le SA, garantit
# roles/owner, active le set d'APIs aphrody, (re)génère une clé JSON dans
# secrets/, chmod 600, peuple .env, et vérifie en mintant un access token.
# 100% non-interactif (CLAUDE.md §0.1).
#
# Usage:
#   scripts/gcp-sa-setup.sh                # idempotent (clé conservée)
#   ROTATE=1 scripts/gcp-sa-setup.sh       # force une nouvelle clé
#   PROJECT=foo SA_NAME=bar scripts/gcp-sa-setup.sh
set -euo pipefail

PROJECT="${PROJECT:-aphrody}"
SA_NAME="${SA_NAME:-aphrody-bot}"
LOCATION="${LOCATION:-us-central1}"
ROTATE="${ROTATE:-0}"

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SECRETS="$REPO/secrets"
KEY_FILE="$SECRETS/$SA_NAME.json"
API_KEY_FILE="$SECRETS/aphrody-api-key.txt"
API_KEY_DISPLAY="aphrody-full"
ENV_FILE="$REPO/.env"
SA_EMAIL="$SA_NAME@$PROJECT.iam.gserviceaccount.com"

step() { printf '\033[36m==> %s\033[0m\n' "$1"; }

# 1. SA.
step "Service account $SA_EMAIL"
if ! gcloud iam service-accounts list --project "$PROJECT" --filter="email:$SA_EMAIL" --format='value(email)' | grep -q .; then
    gcloud iam service-accounts create "$SA_NAME" --project "$PROJECT" --display-name "$SA_NAME" >/dev/null
    echo "    créé."
else
    echo "    déjà présent."
fi

# 2. roles/owner.
step "Liaison IAM roles/owner"
gcloud projects add-iam-policy-binding "$PROJECT" \
    --member="serviceAccount:$SA_EMAIL" --role='roles/owner' --condition=None --quiet >/dev/null
echo "    owner garanti."

# 3. APIs (lots <= 20).
APIS=(
    aiplatform.googleapis.com generativelanguage.googleapis.com cloudaicompanion.googleapis.com
    storage.googleapis.com storage-component.googleapis.com secretmanager.googleapis.com
    bigquery.googleapis.com bigquerystorage.googleapis.com iam.googleapis.com iamcredentials.googleapis.com
    cloudresourcemanager.googleapis.com serviceusage.googleapis.com compute.googleapis.com
    run.googleapis.com cloudfunctions.googleapis.com cloudbuild.googleapis.com artifactregistry.googleapis.com
    pubsub.googleapis.com firestore.googleapis.com cloudkms.googleapis.com
    datastore.googleapis.com translate.googleapis.com texttospeech.googleapis.com speech.googleapis.com
    vision.googleapis.com documentai.googleapis.com language.googleapis.com videointelligence.googleapis.com
    logging.googleapis.com monitoring.googleapis.com
    drive.googleapis.com sheets.googleapis.com docs.googleapis.com gmail.googleapis.com
    people.googleapis.com calendar-json.googleapis.com customsearch.googleapis.com
)
step "Activation de ${#APIS[@]} APIs (idempotent)"
for ((i = 0; i < ${#APIS[@]}; i += 20)); do
    gcloud services enable "${APIS[@]:i:20}" --project "$PROJECT" >/dev/null
done
echo "    APIs activées."

# 4. Clé JSON.
mkdir -p "$SECRETS"
if [[ "$ROTATE" == "1" || ! -f "$KEY_FILE" ]]; then
    step "Génération clé JSON -> $KEY_FILE"
    gcloud iam service-accounts keys create "$KEY_FILE" --iam-account "$SA_EMAIL" >/dev/null
else
    step "Clé existante conservée (ROTATE=1 pour régénérer)"
fi
chmod 600 "$KEY_FILE"
echo "    chmod 600."

# 4b. Clé API non restreinte (réutilisée si déjà présente).
step "Clé API ($API_KEY_DISPLAY)"
gcloud services enable apikeys.googleapis.com --project "$PROJECT" >/dev/null
API_KEY_NAME="$(gcloud services api-keys list --project "$PROJECT" --filter="displayName:$API_KEY_DISPLAY" --format='value(name)' 2>/dev/null | head -1)"
if [[ -z "$API_KEY_NAME" ]]; then
    API_KEY="$(gcloud services api-keys create --display-name "$API_KEY_DISPLAY" --project "$PROJECT" --format='value(response.keyString)' 2>/dev/null)"
else
    API_KEY="$(gcloud services api-keys get-key-string "$API_KEY_NAME" --format='value(keyString)' 2>/dev/null)"
fi
printf '%s' "$API_KEY" > "$API_KEY_FILE"
chmod 600 "$API_KEY_FILE"
echo "    clé API prête (${#API_KEY} chars)."

# 5. .env (bloc idempotent).
step "Mise à jour .env"
touch "$ENV_FILE"
grep -vE '^(GOOGLE_APPLICATION_CREDENTIALS|GOOGLE_CLOUD_PROJECT|GCLOUD_PROJECT|CLOUDSDK_CORE_PROJECT|GCP_SERVICE_ACCOUNT|GOOGLE_CLOUD_LOCATION|GOOGLE_CLOUD_REGION|GOOGLE_API_KEY|GEMINI_API_KEY)=|^# --- GCP service account' "$ENV_FILE" > "$ENV_FILE.tmp" || true
{
    sed -e :a -e '/^\n*$/{$d;N;ba' -e '}' "$ENV_FILE.tmp"
    printf '\n# --- GCP service account (%s) — géré par scripts/gcp-sa-setup.sh ---\n' "$SA_NAME"
    printf 'GOOGLE_APPLICATION_CREDENTIALS=%s\n' "$KEY_FILE"
    printf 'GOOGLE_CLOUD_PROJECT=%s\n' "$PROJECT"
    printf 'GCLOUD_PROJECT=%s\n' "$PROJECT"
    printf 'CLOUDSDK_CORE_PROJECT=%s\n' "$PROJECT"
    printf 'GCP_SERVICE_ACCOUNT=%s\n' "$SA_EMAIL"
    printf 'GOOGLE_CLOUD_LOCATION=%s\n' "$LOCATION"
    printf 'GOOGLE_CLOUD_REGION=%s\n' "$LOCATION"
    printf 'GOOGLE_API_KEY=%s\n' "$API_KEY"
    printf 'GEMINI_API_KEY=%s\n' "$API_KEY"
} > "$ENV_FILE"
rm -f "$ENV_FILE.tmp"
echo "    .env synchronisé."

# 6. Vérification.
step "Vérification de l'accès"
PREV="$(gcloud config get-value account 2>/dev/null || true)"
gcloud auth activate-service-account --key-file "$KEY_FILE" 2>&1 | head -1
TOK="$(gcloud auth print-access-token 2>/dev/null || true)"
N_APIS="$(gcloud services list --enabled --project "$PROJECT" --format='value(config.name)' 2>/dev/null | wc -l | tr -d ' ')"
[[ -n "$PREV" ]] && gcloud config set account "$PREV" >/dev/null 2>&1 || true
if [[ -n "$TOK" ]]; then
    printf '\033[32m    OK : token minté (%s chars), %s APIs actives.\033[0m\n' "${#TOK}" "$N_APIS"
else
    echo "    ECHEC : impossible de minter un token." >&2
    exit 1
fi

printf '\033[32m\nTerminé. SA=%s  clé=%s  APIs=%s\033[0m\n' "$SA_EMAIL" "$KEY_FILE" "$N_APIS"
