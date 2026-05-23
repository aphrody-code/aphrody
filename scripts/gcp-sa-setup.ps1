# SPDX-License-Identifier: Apache-2.0
#
# gcp-sa-setup.ps1 — provisionnement idempotent du service account aphrody-bot.
#
# Crée/réutilise le SA, garantit roles/owner, active le set d'APIs aphrody,
# (re)génère une clé JSON dans secrets/, verrouille ses ACL, peuple .env, et
# vérifie l'accès en mintant un access token. 100% non-interactif (CLAUDE.md §0.1).
#
# Usage:
#   pwsh scripts/gcp-sa-setup.ps1                 # idempotent (clé conservée)
#   pwsh scripts/gcp-sa-setup.ps1 -Rotate         # force une nouvelle clé
#   pwsh scripts/gcp-sa-setup.ps1 -Project foo -SaName bar
[CmdletBinding()]
param(
    [string]$Project = 'aphrody',
    [string]$SaName  = 'aphrody-bot',
    [string]$Location = 'us-central1',
    [switch]$Rotate
)
$ErrorActionPreference = 'Stop'
$repo    = Split-Path -Parent $PSScriptRoot
$secrets = Join-Path $repo 'secrets'
$keyFile = Join-Path $secrets "$SaName.json"
$apiKeyFile = Join-Path $secrets 'aphrody-api-key.txt'
$apiKeyDisplay = 'aphrody-full'
$envFile = Join-Path $repo '.env'
$saEmail = "$SaName@$Project.iam.gserviceaccount.com"

function Step($m) { Write-Host "==> $m" -ForegroundColor Cyan }

# 1. SA : créer s'il n'existe pas.
Step "Service account $saEmail"
$exists = gcloud iam service-accounts list --project $Project --filter="email:$saEmail" --format='value(email)' 2>$null
if (-not $exists) {
    gcloud iam service-accounts create $SaName --project $Project --display-name $SaName | Out-Null
    Write-Host "    créé."
} else { Write-Host "    déjà présent." }

# 2. roles/owner (max de droits sur le projet).
Step "Liaison IAM roles/owner"
gcloud projects add-iam-policy-binding $Project --member="serviceAccount:$saEmail" --role='roles/owner' --condition=None --quiet | Out-Null
Write-Host "    owner garanti."

# 3. Activation des APIs (lots <= 20).
$apis = @(
    'aiplatform.googleapis.com','generativelanguage.googleapis.com','cloudaicompanion.googleapis.com',
    'storage.googleapis.com','storage-component.googleapis.com','secretmanager.googleapis.com',
    'bigquery.googleapis.com','bigquerystorage.googleapis.com','iam.googleapis.com','iamcredentials.googleapis.com',
    'cloudresourcemanager.googleapis.com','serviceusage.googleapis.com','compute.googleapis.com',
    'run.googleapis.com','cloudfunctions.googleapis.com','cloudbuild.googleapis.com','artifactregistry.googleapis.com',
    'pubsub.googleapis.com','firestore.googleapis.com','cloudkms.googleapis.com',
    'datastore.googleapis.com','translate.googleapis.com','texttospeech.googleapis.com','speech.googleapis.com',
    'vision.googleapis.com','documentai.googleapis.com','language.googleapis.com','videointelligence.googleapis.com',
    'logging.googleapis.com','monitoring.googleapis.com',
    'drive.googleapis.com','sheets.googleapis.com','docs.googleapis.com','gmail.googleapis.com',
    'people.googleapis.com','calendar-json.googleapis.com','customsearch.googleapis.com'
)
Step "Activation de $($apis.Count) APIs (idempotent)"
for ($i = 0; $i -lt $apis.Count; $i += 20) {
    $batch = $apis[$i..([Math]::Min($i + 19, $apis.Count - 1))]
    gcloud services enable @batch --project $Project | Out-Null
}
Write-Host "    APIs activées."

# 4. Clé JSON.
New-Item -ItemType Directory -Force -Path $secrets | Out-Null
if ($Rotate -or -not (Test-Path $keyFile)) {
    Step "Génération clé JSON -> $keyFile"
    gcloud iam service-accounts keys create $keyFile --iam-account $saEmail | Out-Null
} else {
    Step "Clé existante conservée (-Rotate pour régénérer)"
}

# 5. Verrouillage ACL : seulement l'utilisateur courant.
icacls $keyFile /inheritance:r /grant:r "$($env:USERNAME):(R,W)" | Out-Null
Write-Host "    ACL restreintes à $($env:USERNAME)."

# 5b. Clé API non restreinte (réutilisée si déjà présente).
Step "Clé API ($apiKeyDisplay)"
gcloud services enable apikeys.googleapis.com --project $Project | Out-Null
$apiKeyName = gcloud services api-keys list --project $Project --filter="displayName:$apiKeyDisplay" --format='value(name)' 2>$null | Select-Object -First 1
if (-not $apiKeyName) {
    $created = gcloud services api-keys create --display-name $apiKeyDisplay --project $Project --format=json 2>$null | ConvertFrom-Json
    $apiKey = $created.response.keyString
} else {
    $apiKey = gcloud services api-keys get-key-string $apiKeyName --format='value(keyString)' 2>$null
}
Set-Content -Path $apiKeyFile -Value $apiKey -NoNewline -Encoding ascii
icacls $apiKeyFile /inheritance:r /grant:r "$($env:USERNAME):(R,W)" | Out-Null
Write-Host "    clé API prête ($($apiKey.Length) chars)."

# 6. Mise à jour .env (bloc délimité, idempotent).
Step "Mise à jour .env"
$block = @"
# --- GCP service account ($SaName) — géré par scripts/gcp-sa-setup.ps1 ---
GOOGLE_APPLICATION_CREDENTIALS=$keyFile
GOOGLE_CLOUD_PROJECT=$Project
GCLOUD_PROJECT=$Project
CLOUDSDK_CORE_PROJECT=$Project
GCP_SERVICE_ACCOUNT=$saEmail
GOOGLE_CLOUD_LOCATION=$Location
GOOGLE_CLOUD_REGION=$Location
GOOGLE_API_KEY=$apiKey
GEMINI_API_KEY=$apiKey
"@
if (-not (Test-Path $envFile)) { New-Item -ItemType File -Path $envFile | Out-Null }
$cur = Get-Content $envFile -Raw -ErrorAction SilentlyContinue
if ($null -eq $cur) { $cur = '' }
$cleaned = ($cur -split "`r?`n" | Where-Object { $_ -notmatch '^(GOOGLE_APPLICATION_CREDENTIALS|GOOGLE_CLOUD_PROJECT|GCLOUD_PROJECT|CLOUDSDK_CORE_PROJECT|GCP_SERVICE_ACCOUNT|GOOGLE_CLOUD_LOCATION|GOOGLE_CLOUD_REGION|GOOGLE_API_KEY|GEMINI_API_KEY)=' -and $_ -notmatch '^# --- GCP service account' }) -join "`n"
Set-Content -Path $envFile -Value ($cleaned.TrimEnd() + "`n`n" + $block) -NoNewline -Encoding utf8
Write-Host "    .env synchronisé."

# 7. Vérification : mint d'un token dans une config isolée.
Step "Vérification de l'accès"
$prev = gcloud config get-value account 2>$null
gcloud auth activate-service-account --key-file $keyFile 2>&1 | Select-Object -First 1
$tok = gcloud auth print-access-token 2>$null
$nApis = (gcloud services list --enabled --project $Project --format='value(config.name)' 2>$null | Measure-Object).Count
if ($prev) { gcloud config set account $prev 2>&1 | Out-Null }
if ($tok) { Write-Host "    OK : token minté ($($tok.Length) chars), $nApis APIs actives." -ForegroundColor Green }
else { Write-Error "    ECHEC : impossible de minter un token." }

Write-Host "`nTermine. SA=$saEmail  cle=$keyFile  APIs=$nApis" -ForegroundColor Green
