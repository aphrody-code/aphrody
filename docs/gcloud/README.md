<!-- SPDX-License-Identifier: Apache-2.0 -->
# Google Cloud — référence aphrody

Configuration GCP complète du projet **aphrody** : identités, credentials,
APIs activées, automatisation, sécurité et usage. Tout est **non-interactif**
et reproductible (CLAUDE.md §0.1).

### Documentation Complète de l'Écosystème GCP :
* **[Configuration CI/CD & Workload Identity Federation](WIF_GITHUB_ACTIONS.md)** — Guide de déploiement et authentification GitHub Actions sans clé statique.
* **[Optimisation des Coûts & Quotas Always Free](COST_OPTIMIZATION.md)** — Runbook détaillé pour exploiter le catalogue GCP à $0.00/mois.

> **Provisionné le 2026-05-23.** Source de vérité opérationnelle : les scripts
> [`scripts/gcp-sa-setup.{ps1,sh}`](../../scripts) et
> [`scripts/gcp-enable-all.sh`](../../scripts/gcp-enable-all.sh). Cette doc
> décrit l'état cible ; relancer un script suffit à le reconstituer.

---

## 1. Projet

| Champ | Valeur |
|-------|--------|
| Project ID | `aphrody` |
| Project number | `468000409790` |
| Compte propriétaire | `yohanpierre15@gmail.com` (`roles/owner`) |
| Billing | actif — `billingAccounts/01C99E-70AD11-2A7C91` |
| Région par défaut | `us-central1` (Vertex AI ; `global` requis pour `gemini-3-pro-image-preview`) |

```bash
gcloud config set project aphrody
gcloud config get-value account     # doit rester yohanpierre15@gmail.com
```

---

## 2. Identités & credentials

Trois mécanismes d'authentification cohabitent. **Aucun n'est committé** :
tous les fichiers vivent dans [`secrets/`](../../secrets) (gitignoré, cf. §6).

| Credential | Fichier | Type | Usage |
|------------|---------|------|-------|
| **Service account ADC** | `secrets/aphrody-bot.json` | clé JSON RSA | APIs Cloud nécessitant OAuth/SA (Vertex AI, Storage, BigQuery, Secret Manager…) |
| **Clé API non restreinte** | `secrets/aphrody-api-key.txt` | API key (`AIza…`) | `generativelanguage` (Gemini Developer API), Maps, Custom Search, services à clé |
| **gcloud CLI user** | CredMan / `~/.config/gcloud` | OAuth user | administration interactive (toi) |

### Service account `aphrody-bot`

| Champ | Valeur |
|-------|--------|
| Email | `aphrody-bot@aphrody.iam.gserviceaccount.com` |
| Rôle | `roles/owner` (droits maximaux sur le projet) |
| Clé active | `secrets/aphrody-bot.json` (ACL restreintes à l'utilisateur courant) |

```bash
# Lister les rôles du SA
gcloud projects get-iam-policy aphrody \
  --flatten="bindings[].members" \
  --filter="bindings.members:aphrody-bot@aphrody.iam.gserviceaccount.com" \
  --format="value(bindings.role)"

# Lister les clés
gcloud iam service-accounts keys list \
  --iam-account=aphrody-bot@aphrody.iam.gserviceaccount.com
```

### Clé API `aphrody-full`

Clé API **non restreinte** (aucune restriction d'API ni d'application).

| Champ | Valeur |
|-------|--------|
| Display name | `aphrody-full` |
| uid | `6fcb027a-084d-4f2e-b1c8-a449c747e185` |
| Resource | `projects/468000409790/locations/global/keys/6fcb027a-…` |

```bash
# Récupérer la valeur de la clé (keyString)
gcloud services api-keys get-key-string \
  projects/468000409790/locations/global/keys/6fcb027a-084d-4f2e-b1c8-a449c747e185 \
  --format='value(keyString)'
```

> ⚠️ **Bearer credential** : quiconque possède la chaîne `AIza…` peut appeler
> toutes les APIs activées dans la limite des quotas. Pour durcir sans casser
> Gemini : `gcloud services api-keys update <name> --api-target=service=generativelanguage.googleapis.com`.

---

## 3. APIs activées

**Toutes les APIs Google first-party** (`*.googleapis.com`) sont activées —
**525 services activés, 0 échec** (2026-05-23 ; 526 actives au total avec
l'API de management).

> Les ~10 700 autres entrées de `gcloud services list --available` sont des
> services **tiers** du Marketplace (`*.endpoints.*.cloud.goog`,
> `*.cloudpartnerservices.goog` — A10, Neo4j, MongoDB, Wowza…). Ce **ne sont
> pas** des APIs Google et ils ne sont volontairement **pas** activés (images
> VM payantes, conditions commerciales vendeur).

```bash
# Compter / lister les APIs actives
gcloud services list --enabled --project aphrody --format='value(config.name)' | wc -l
gcloud services list --enabled --project aphrody

# (Re)activer TOUTES les APIs Google first-party — idempotent
bash scripts/gcp-enable-all.sh
```

APIs clés pour aphrody : `aiplatform` (Vertex AI), `generativelanguage`
(Gemini Developer), `cloudaicompanion`, `secretmanager`, `storage`,
`bigquery`, `run`, `firestore`, `vision`, `speech`, `texttospeech`,
`translate`, `documentai`, `drive`/`sheets`/`docs`/`gmail`, `customsearch`.

---

## 4. Automatisation

| Script | Plateforme | Rôle |
|--------|------------|------|
| [`scripts/gcp-sa-setup.ps1`](../../scripts/gcp-sa-setup.ps1) | Windows (hôte) | SA + owner + 37 APIs cœur + clé JSON + clé API + `.env` + vérif |
| [`scripts/gcp-sa-setup.sh`](../../scripts/gcp-sa-setup.sh) | Linux (cible #1) | idem, miroir bash |
| [`scripts/gcp-enable-all.sh`](../../scripts/gcp-enable-all.sh) | Linux/bash | active **toutes** les 525 APIs first-party (lots de 20, repli individuel) |

Tous **idempotents** et **non-interactifs**.

```bash
# Provisionnement complet (SA + clé + clé API + .env + vérif)
pwsh scripts/gcp-sa-setup.ps1            # Windows
scripts/gcp-sa-setup.sh                  # Linux

# Forcer la rotation de la clé SA
pwsh scripts/gcp-sa-setup.ps1 -Rotate
ROTATE=1 scripts/gcp-sa-setup.sh

# Activer tout le catalogue Google
bash scripts/gcp-enable-all.sh           # log -> /tmp/gcp-enable-all.log
```

Chaque exécution de `gcp-sa-setup` vérifie l'accès en mintant un access token
et **restaure** le compte gcloud actif sur l'utilisateur.

---

## 5. Variables d'environnement (`.env`)

`.env` est gitignoré ; `.env.example` est le template versionné.

| Variable | Valeur | Consommée par |
|----------|--------|---------------|
| `GOOGLE_APPLICATION_CREDENTIALS` | `secrets/aphrody-bot.json` | ADC (toutes les libs Google, google-cloud-rust) |
| `GOOGLE_CLOUD_PROJECT` / `GCLOUD_PROJECT` / `CLOUDSDK_CORE_PROJECT` | `aphrody` | gcloud, ADC, SDKs |
| `GCP_SERVICE_ACCOUNT` | `aphrody-bot@…` | scripts, impersonation |
| `GOOGLE_CLOUD_LOCATION` / `GOOGLE_CLOUD_REGION` | `us-central1` | Vertex AI |
| `GOOGLE_API_KEY` / `GEMINI_API_KEY` | `AIza…` | generativelanguage, Maps, Custom Search |

```bash
# Charger .env dans le shell (bash)
set -a; . ./.env; set +a
```

---

## 6. Sécurité

- **gitignore** (`.gitignore` §7) couvre `secrets/`, `.secrets/`, `.env`,
  `.env.*`, `*secrets.json`. Vérifié : `git check-ignore secrets/aphrody-bot.json .env`.
- **ACL fichiers** : `icacls … /inheritance:r /grant:r "$USER:(R,W)"` (Windows),
  `chmod 600` (Linux). Aucune hérédité, lecture/écriture utilisateur uniquement.
- **Jamais** committer une clé. Le `git status` ne doit montrer que `scripts/`
  et `.env.example`.

### Révocation / rotation

```bash
# Révoquer une clé SA compromise
gcloud iam service-accounts keys delete <KEY_ID> \
  --iam-account=aphrody-bot@aphrody.iam.gserviceaccount.com

# Régénérer une clé SA propre
pwsh scripts/gcp-sa-setup.ps1 -Rotate

# Supprimer / recréer la clé API
gcloud services api-keys delete projects/468000409790/locations/global/keys/6fcb027a-…
gcloud services api-keys create --display-name=aphrody-full --project=aphrody
```

---

## 7. Usage applicatif

### ADC depuis Rust (`google-cloud-rust`)

Les client libraries officielles (`google-cloud-auth` 1.10+, MSRV 1.87,
Apache-2.0) lisent automatiquement `GOOGLE_APPLICATION_CREDENTIALS`.

```toml
# Cargo.toml (dépendance optionnelle, feature-gated)
[dependencies]
google-cloud-auth = { version = "1.10", optional = true }
```

### Gemini Developer API via clé API

```bash
curl "https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent?key=$GEMINI_API_KEY" \
  -H 'Content-Type: application/json' \
  -d '{"contents":[{"parts":[{"text":"Bonjour"}]}]}'
```

### gcloud-mcp (serveur MCP)

Serveur MCP `gcloud` configuré au scope user (`bunx -y @google-cloud/gcloud-mcp`),
s'appuie sur les credentials `gcloud auth` ci-dessus.

```bash
claude mcp get gcloud
```

---

## 8. Coûts

- **Activer une API ne coûte rien** — seule la *consommation* est facturée.
- Surveiller : `gcloud billing accounts list`, console Billing.
- APIs gratuites en quota : generativelanguage (tier gratuit), Custom Search
  (100 req/j), la plupart des APIs Workspace en lecture.

---

## 9. Runbook / dépannage

| Symptôme | Cause probable | Fix |
|----------|----------------|-----|
| `PERMISSION_DENIED` malgré owner | API non activée | `gcloud services enable <api>` |
| `API key not valid` | clé restreinte / mauvaise API | vérifier `api-keys describe`, ou recréer non restreinte |
| `invalid_grant` ADC | clé SA expirée/supprimée | `pwsh scripts/gcp-sa-setup.ps1 -Rotate` |
| `SU_MAX_BATCH_SIZE_EXCEEDED` | > 20 services par `enable` | lots de 20 (déjà géré par les scripts) |
| compte gcloud changé après script | activation SA | `gcloud config set account yohanpierre15@gmail.com` (le script restaure auto) |

---

*Mémoire associée : `gcp-service-account-setup` (index mémoire projet).*
