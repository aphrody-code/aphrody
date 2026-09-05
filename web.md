---
spec_id: aphrody.web-platform
schema_version: 2
status: normative
audience: codex
last_repo_verification: 2026-09-04
last_live_probe_utc: 2026-09-04T14:57:55Z
scope:
  - aphrody
  - bxc
  - n2b
  - nie
root_ui_policy: blank
---

# Aphrody Web — contrat d’exécution Codex

## 0. Mode d’emploi et autorité

Ce fichier est une spécification d’exécution, pas une page marketing. Un agent
DOIT transformer chaque exigence en preuve vérifiable et NE DOIT PAS présenter
un état cible comme déjà déployé.

Les mots **DOIT**, **NE DOIT PAS**, **DEVRAIT** et **PEUT** ont leur sens RFC
2119. Les états autorisés sont :

- `ACTIF` : source, test, smoke local et smoke externe concordent ;
- `OBSERVÉ` : un probe horodaté décrit la surface, mais les quatre preuves
  nécessaires à `ACTIF` ne sont pas toutes consignées ;
- `NON_CONFORME` : surface publiquement active en violation d’un invariant ;
  aucune extension n’est autorisée avant correction ou fermeture ;
- `RÉSERVÉ` : nom ou route volontairement occupé, sans fonction publiée ;
- `CIBLE` : contrat approuvé mais artefact non livré ;
- `BLOQUÉ` : une dépendance ou une autorisation externe manque ;
- `RETIRÉ` : composant hors du périmètre et absent du chemin de production.

Ordre de priorité en cas de conflit :

1. demande explicite de l’opérateur et [AGENTS.md](AGENTS.md) ;
2. code, tests et configuration déployable pour décrire l’état `ACTIF` ;
3. ce fichier pour l’état web cible ;
4. [docs/SITES-PLATFORM.md](docs/SITES-PLATFORM.md) pour le contrat inter-projets ;
5. autres documents du dépôt.

Le code ne transforme pas automatiquement une mauvaise configuration en cible
souhaitable. Inversement, une case cochée ou une phrase au futur ne prouve pas
un déploiement.

### Boucle obligatoire d’un agent

Pour chaque lot :

1. inventorier le dépôt, le service, le DNS et les ports concernés ;
2. annoncer les fichiers revendiqués si plusieurs agents travaillent en parallèle ;
3. écrire le plus petit changement cohérent ;
4. exécuter les gates de la section 18 ;
5. déployer atomiquement seulement si le lot le requiert ;
6. exécuter les smokes locaux puis externes ;
7. consigner chaque livrable comme `FAIT`, `INCOMPLET` ou `NON_FAIT`, avec preuve ;
8. restaurer l’artefact précédent si un smoke post-déploiement échoue.

Une route ne passe à `ACTIF` qu’après les quatre preuves suivantes : chemin de
source, test automatisé, réponse locale attendue et réponse publique attendue.

## 1. Résultat attendu et invariants

Aphrody est une couche de distribution et d’interopérabilité machine-first pour
Aphrody, BXC, N2B et Niers. Le domaine sert des contrats stables, des artefacts
signés, des API lisibles par agent et des vitrines statiques sobres. Rose
Griffon/RG n’appartient pas à cette plateforme coordonnée et NE DOIT PAS fuir
dans son identité, ses certificats, ses réponses MCP ou ses métadonnées.

Invariants non négociables :

- `https://aphrody.com/` reste un document HTML valide dont le corps est
  exactement vide : `<body></body>` ;
- la racine ne charge aucun JavaScript, CSS, webfont, cookie, pixel, analytics
  ou requête tierce ;
- toute route inconnue répond `404`, jamais une fausse page applicative `200` ;
- les origines applicatives écoutent uniquement sur loopback ; Nginx termine TLS ;
- le chemin web de production est Rust ; Bun PEUT rester un outil de build ou
  de validation, jamais une dépendance d’exécution du site ;
- aucun framework frontend, base SQL, moteur vectoriel ou système de session
  n’est ajouté sans besoin utilisateur, ADR et budget mesuré ;
- aucun secret, jeton, adresse personnelle, chemin utilisateur absolu ou corps
  de requête sensible n’entre dans Git, HTML, logs ou métriques ;
- les enregistrements MX, SPF et DMARC existants sont préservés ; l’absence de
  DKIM prouvé reste un écart et aucun agent n’invente sa présence ;
- `admin.aphrody.com` et `bot.aphrody.com` doivent répondre `404` sans upstream ;
  `frontends.aphrody.com` reste sans DNS ou répond de la même façon tant qu’un
  contrat et une authentification propres n’existent pas ;
- BXC, Niers et N2B restent sur le fallback vierge Aphrody jusqu’à réussite de
  leurs gates locales et externes ;
- chaque nouveau crate porte `publish = false` jusqu’à revue explicite de la
  chaîne de publication.

## 2. Sources de vérité à lire avant toute modification

Lecture minimale :

- [CLAUDE.md](CLAUDE.md) et [AGENTS.md](AGENTS.md) ;
- [DEPLOY.md](DEPLOY.md) et
  [docs/agent-stack/DEPLOY.md](docs/agent-stack/DEPLOY.md) ;
- [docs/SITES-PLATFORM.md](docs/SITES-PLATFORM.md) ;
- [docs/SOURCE_OF_TRUTH.md](docs/SOURCE_OF_TRUTH.md) ;
- [docs/DOMAIN.md](docs/DOMAIN.md) et
  [docs/APHRODY-COM-STATUS.md](docs/APHRODY-COM-STATUS.md) ;
- [docs/MAIL-STACK.md](docs/MAIL-STACK.md) ;
- [docs/api-unified-pattern.md](docs/api-unified-pattern.md) ;
- [crates/aphrody-site/src/main.rs](crates/aphrody-site/src/main.rs) et
  [crates/aphrody-site/Cargo.toml](crates/aphrody-site/Cargo.toml) ;
- [crates/cli/src/package_cmd.rs](crates/cli/src/package_cmd.rs) ;
- [deploy/nginx/aphrody.com.conf](deploy/nginx/aphrody.com.conf) et
  [deploy/systemd/aphrody-site.service](deploy/systemd/aphrody-site.service).

## 3. État audité au 2026-09-04

Les résultats ci-dessous sont des observations ponctuelles, pas un moniteur ni
une preuve suffisante d’état `ACTIF`.

| Surface | État | Observation live/configuration | Écart prioritaire |
|---|---|---|---|
| `aphrody.com/` | `OBSERVÉ` | `200`, document de 265 octets, élément `<body>` vide | HSTS absent ; test strict DOM/headers à compléter |
| `www.aphrody.com/` | `RÉSERVÉ` | alias présent dans Nginx, probe live non consigné | redirection apex et TLS à prouver |
| `aphrody.com/health` | `OBSERVÉ` | `200 ok`, alias edge vers `/healthz` | conserver l’alias seulement à l’edge ; smoke local à consigner |
| `api.aphrody.com/health` | `OBSERVÉ` | `200 ok` | aucune API métier publiée |
| `api.aphrody.com/api/v1/catalog` | `CIBLE` | `404` | implémentation et OpenAPI absents |
| `downloads.aphrody.com/downloads/` | `CIBLE` | `404` | index/manifests signés absents |
| `cdn.aphrody.com/` | `RÉSERVÉ` | `200` vierge via l’origine Aphrody | politique d’assets non implémentée |
| `admin.aphrody.com/` | `NON_CONFORME` | `200` vierge via `:8083` | retourner `404` sans upstream |
| `bot.aphrody.com/` | `NON_CONFORME` | `200` vierge via `:8083` | retourner `404` sans upstream |
| `frontends.aphrody.com/` | `CIBLE` | DNS absent/NXDOMAIN au probe | conserver absent ou créer DNS/TLS puis `404` sans upstream |
| `bxc.aphrody.com/` | `RÉSERVÉ` | `200` vierge via `:8083` | service BXC `:8084` non promu |
| `nie.aphrody.com/` | `RÉSERVÉ` | `200` vierge via `:8083` | service Niers `:8085` non promu |
| `n2b.aphrody.com/` | `RÉSERVÉ` | `200` vierge via `:8083` | service N2B `:8086` non promu |
| `mcp.aphrody.com/health` | `NON_CONFORME` | `200`, identité `rose-griffon`, 36 outils, 4 prompts | fermer ou remplacer immédiatement par un MCP Aphrody minimal |
| `mcp.aphrody.com/mcp` sans jeton | `NON_CONFORME` | `401 Bearer`, realm `rg-mcp` | auth effective mais tenant/realm incorrect |
| `contact@aphrody.com` | `BLOQUÉ` | création non prouvée | droits/API OVH Mail nécessaires |

L’origine Rust actuelle expose seulement `/`, `/healthz`, `/robots.txt` et
`/.well-known/security.txt`. La configuration Nginx regroupe encore plusieurs
alias sur `127.0.0.1:8083`. Aucun crate partagé `aphrody-web` n’existe encore.

## 4. Identité des produits et contrat CLI

| Product ID | Nom affiché | Dépôt | Package Cargo/binaire | Sous-commande façade |
|---|---|---|---|---|
| `aphrody` | Aphrody | `aphrody-code/aphrody` | `aphrody` | `aphrody package …` |
| `bxc` | BXC | `aphrody-code/bxc` | défini par le dépôt BXC | `aphrody bxc` |
| `n2b` | N2B | `aphrody-code/n2b` | défini par le dépôt N2B | `aphrody n2b` |
| `nie` | Niers | `aphrody-code/nie` | package `nie-cli`, binaire `niers` | `aphrody nie` |

`nie` est l’identifiant stable ; `Niers` est le nom affiché ; `niers` est le
binaire et le nom de checkout local conventionnel. Les agents NE DOIVENT PAS
créer un second produit pour résoudre cette différence de noms.

Les commandes de gestion déjà canoniques sont :

```text
aphrody package catalog
aphrody package status [aphrody|bxc|n2b|nie|all]
aphrody package install <produit>
aphrody package update <produit>
aphrody package uninstall <produit> --yes [--purge]
aphrody package doctor
```

Les anciennes formes `aphrody install`, `aphrody remove` et `aphrody status`
NE DOIVENT PAS être documentées comme actives. Des alias futurs nécessitent des
tests de compatibilité et une fenêtre de dépréciation.

## 5. Topologie cible

```text
Internet
   |
   v
Nginx :443  -- TLS, limites, headers, cache, request-id
   |
   +-- aphrody.com / api / downloads / cdn --> aphrody-site :8083
   +-- bxc.aphrody.com ---------------------> bxc-site     :8084
   +-- nie.aphrody.com ---------------------> nie-site     :8085
   +-- n2b.aphrody.com ---------------------> n2b-site     :8086
   +-- mcp.aphrody.com/mcp -----------------> aphrody-mcp  :8808
   +-- admin / bot ------------------------> 404 local, aucun upstream
   +-- frontends --------------------------> DNS absent ou 404 local
```

| Hôte | Origine cible | Exposition | Promotion |
|---|---:|---|---|
| `aphrody.com` | `127.0.0.1:8083` | public statique | observé ; `ACTIF` après quatre preuves |
| `www.aphrody.com` | redirection permanente vers apex | public | après validation TLS |
| `api.aphrody.com` | `127.0.0.1:8083` | public GET/HEAD | après contrats API |
| `downloads.aphrody.com` | `127.0.0.1:8083` | public GET/HEAD | après signatures et Range |
| `cdn.aphrody.com` | `127.0.0.1:8083` | assets publics immuables | après pipeline média |
| `bxc.aphrody.com` | `127.0.0.1:8084` | vitrine statique | gates BXC réussies |
| `nie.aphrody.com` | `127.0.0.1:8085` | vitrine statique | gates Niers/licences réussies |
| `n2b.aphrody.com` | `127.0.0.1:8086` | vitrine statique | gates N2B réussies |
| `mcp.aphrody.com` | `127.0.0.1:8808` | health public, MCP authentifié | correction P0 d’identité |
| `admin.aphrody.com` | aucun | réservé | vhost exact `return 404`, sans proxy |
| `bot.aphrody.com` | aucun | réservé | vhost exact `return 404`, sans proxy |
| `frontends.aphrody.com` | aucun | non publié | DNS absent, ou vhost exact `return 404` |

Chaque service DOIT écouter sur une adresse loopback explicite. Aucun port
`8083..8086` ou `8808` ne doit être ouvert par le pare-feu public.

## 6. Architecture Rust partagée

### 6.1 Crate `aphrody-web` (`CIBLE`)

Créer un crate partagé uniquement lorsque la première extraction réelle est
prête. Il contient des contrats, pas une plateforme abstraite spéculative :

- tokens de design typés et métadonnées produit ;
- handlers communs `/healthz`, `/version`, robots et security.txt ;
- types d’erreur API, pagination, ETag et request-id ;
- validation de chemin d’asset et métadonnées de release ;
- middleware de sécurité et observabilité sans donnée sensible ;
- helpers de test pour le corps vierge, les headers et les `404`.

Le crate déclare `default = []`. Chaque binaire consommateur sélectionne
explicitement ses features ; les quatre sites utilisent `features = ["axum"]`.

| Feature | Contenu | Règle |
|---|---|---|
| `axum` | Router et handlers HTTP | active dans les quatre binaires site |
| `openapi` | dérives et document brut | API seulement |
| `templates` | rendu Askama | différé jusqu’à une page structurée |
| `metrics-prometheus` | métriques RED | endpoint loopback/protégé seulement |
| `signed-manifests` | vérification Ed25519 | releases seulement |

`rmcp`, `rustls`, `sqlx`, un ORM, un moteur vectoriel et un framework WASM ne
font pas partie de ce crate. MCP reste un service séparé ; TLS reste à l’edge ;
le site public reste stateless.

Le crate porte `publish = false`. Les dépôts consommateurs pinent une révision
Git complète de 40 caractères. La promotion utilise un tag `web-vX.Y.Z` et une
matrice CI qui teste la révision courante puis la candidate sur les quatre
produits avant bascule.

### 6.2 Dépendances fact-checkées

Les versions « présentes » viennent de `Cargo.toml`/`Cargo.lock`. Les versions
« candidates » ont été vérifiées le 2026-09-04 ; elles NE DOIVENT PAS être
copiées sans nouvelle résolution Context7, MSRV, licence, `cargo deny` et revue
du graphe au moment de l’implémentation.

| Composant | État / décision | ID Context7 | Documentation primaire |
|---|---|---|---|
| Axum | présent `0.8` (`0.8.9` lock) ; conserver | `/websites/rs_axum` | [Router Axum](https://docs.rs/axum/latest/axum/struct.Router.html) |
| Tokio | présent `1.52` (`1.52.3` lock) ; remplacer `full` par `macros,rt-multi-thread,net,signal` dans le site si le graphe le permet | `/websites/rs_tokio_tokio` | [Tokio](https://docs.rs/tokio/latest/tokio/) |
| tower-http | présent workspace `0.6` (`0.6.10` lock), absent du site ; ajouter seulement `trace,request-id,timeout,sensitive-headers,set-header` et compression utile | `/websites/rs_tower-http_tower_http` | [tower-http](https://docs.rs/tower-http/latest/tower_http/) |
| Utoipa | absent ; candidat `5.5.0`, avec `utoipa-axum 0.2.0` compatible Axum `^0.8`, uniquement avec la feature `openapi` | `/websites/rs_utoipa` | [Utoipa OpenAPI](https://docs.rs/utoipa/latest/utoipa/derive.OpenApi.html) |
| Askama | absent ; candidat `0.16.1`, MSRV annoncé `1.88` ; différer tant que du HTML constant suffit | `/askama-rs/askama` | [Askama](https://docs.rs/askama/latest/askama/) |
| Schemars | plusieurs versions transitives (`0.8.22`, `0.9.0`, `1.2.1`) ; ne pas l’ajouter au site si `ToSchema` Utoipa suffit | `/gresau/schemars` | [Schemars](https://docs.rs/schemars/latest/schemars/) |
| metrics | absent ; candidats `metrics 0.24.6` et `metrics-exporter-prometheus 0.18.3`, feature optionnelle | `/metrics-rs/metrics` | [metrics](https://docs.rs/metrics/latest/metrics/) |
| ed25519-dalek | transitif `2.2.0`, absent direct ; candidat majeur `3.0.0` à valider avant feature `signed-manifests` | `/websites/rs_ed25519-dalek_2_2_0_ed25519_dalek` | [ed25519-dalek](https://docs.rs/ed25519-dalek/latest/ed25519_dalek/) |
| rustls | présent workspace `0.23` (`0.23.40` lock) ; ne pas ajouter au site derrière Nginx | `/websites/rs_rustls_rustls` | [rustls](https://docs.rs/rustls/latest/rustls/) |
| rmcp | présent ailleurs, révision Git verrouillée, lock `1.7.0` ; ne pas ajouter au site | `/websites/rs_rmcp_rmcp` | [transports rmcp](https://docs.rs/rmcp/latest/rmcp/transport/io/index.html) |

Précisions d’implémentation :

- `tower-http` ne fournit pas la politique de rate limiting Nginx attendue ; ne
  pas inventer une feature `limit` ;
- `include_bytes!` ou des constantes suffisent pour les rares assets courants ;
  ne pas ajouter `rust-embed`/`include_dir` sans preuve de besoin ;
- `reqwest`, SQLx et une base n’ont aucun rôle dans l’origine stateless ;
- une API OpenAPI sert le JSON brut ; aucune Swagger UI n’est publiée en prod ;
- la documentation Context7 d’Ed25519 résolue est versionnée `2.2.0` alors que
  le candidat crates.io est `3.0.0` : aucune API ne doit être extrapolée entre
  ces majeures sans une nouvelle lecture de documentation.

### 6.3 Contrat Context7 local

La clé est fournie au processus par `CONTEXT7_API_KEY`, stockée uniquement dans
le `.env` local ignoré en mode `0600`. Sa présence se vérifie par un compte ou
une longueur, jamais en affichant sa valeur. Le binaire local `aphrody-mcp`
charge le `.env` le plus proche ; le MCP distant ne consomme pas ce fichier.

Le flux obligatoire est `context7_resolve_library_id` puis
`context7_query_docs`. L’implémentation Rust de référence est dans
[crates/google_mcp/src/main.rs](crates/google_mcp/src/main.rs) et utilise
`https://context7.com/api/v1`, pas l’ancien hôte `mcp.context7.com`. Cela ne
préjuge pas du cycle de vie de la surface distincte `context7.com/api/v2`.
Le skill maintenu est
[plugins/aphrody/skills/context7-mcp/SKILL.md](plugins/aphrody/skills/context7-mcp/SKILL.md).
Une requête Context7 ne contient ni secret, ni code propriétaire, ni donnée
personnelle. Les IDs et URLs de documentation retenus sont consignés dans la
table précédente afin que l’agent puisse répéter le fact-check.

## 7. Contrat HTTP et API

### 7.1 Routes

| Hôte et route | État | Méthodes | Auth | Cache cible | Limite cible |
|---|---|---|---|---|---|
| `aphrody.com/` | `OBSERVÉ` | GET, HEAD | aucune | `max-age=300` ou revalidation | 60 req/min/IP |
| `/healthz` origine | `OBSERVÉ` | GET, HEAD | aucune | `no-store` | 120 req/min/IP |
| `/health` edge | `OBSERVÉ` | GET, HEAD | aucune | `no-store` | alias de compatibilité vers `/healthz` |
| `/robots.txt` | `OBSERVÉ` | GET, HEAD | aucune | 1 h | 60 req/min/IP |
| `/.well-known/security.txt` | `NON_CONFORME` | GET, HEAD | aucune | 1 h | publie un contact dont la réception n’est pas prouvée |
| `/version` | `CIBLE` | GET, HEAD | aucune | `no-store` | 60 req/min/IP |
| `api.aphrody.com/api/v1/catalog` | `CIBLE` | GET, HEAD | aucune | revalidation/ETag | 60 req/min/IP |
| `api.aphrody.com/api/v1/projects` | `CIBLE` | GET, HEAD | aucune | revalidation/ETag | 60 req/min/IP |
| `api.aphrody.com/api/v1/releases` | `CIBLE` | GET, HEAD | aucune | revalidation/ETag | 60 req/min/IP |
| `api.aphrody.com/api/v1/status` | `CIBLE` | GET, HEAD | aucune | `no-store` | 60 req/min/IP |
| `api.aphrody.com/openapi.json` | `CIBLE` | GET, HEAD | aucune | ETag | 30 req/min/IP |
| `downloads.aphrody.com/downloads/{version}/{artifact}` | `CIBLE` | GET, HEAD | aucune | immutable si versionné | 120 req/min/IP |
| `cdn.aphrody.com/assets/{digest}/{name}` | `CIBLE` | GET, HEAD | aucune | immutable 1 an | 300 req/min/IP |

`/healthz` est la liveness canonique du processus. `/health` est uniquement un
alias de compatibilité à l’edge. Ne pas créer `/api/v1/health` : l’état métier
versionné est `/api/v1/status`, distinct de la liveness.

Contrat `CIBLE` : toute autre méthode retourne `405` avec `Allow`. Une route
statique inconnue peut garder un `404` vide ; une route API inconnue retourne un
problème JSON. Les redirections de téléchargement sont interdites vers un hôte
non déclaré dans la configuration.

### 7.2 Schémas API (`CIBLE`)

- OpenAPI cible : `3.1`, généré du même code que les handlers ;
- erreur : `application/problem+json` conforme RFC 9457 avec `type`, `title`,
  `status`, `detail` non sensible, `instance` et `request_id` ;
- toutes les listes ont `items`, `next_cursor` nullable et `schema_version` ;
- pagination par curseur opaque, `limit` par défaut `20`, maximum `100` ;
- dates au format RFC 3339 UTC ; tailles en octets ; digests en hex lowercase ;
- `ETag` fort pour manifestes/JSON immuables, conditionnel `If-None-Match` ;
- compatibilité N-1 sur `/api/v1` ; annonce de dépréciation au moins 90 jours ;
- aucun champ libre ne contient du HTML, une URL arbitraire ou un chemin local.

## 8. Distribution et manifests signés

L’état actuel construit les produits depuis leur dépôt. La cible ajoute des
binaires précompilés sans supprimer immédiatement le fallback source.

Chaque artefact supporté possède :

```text
releases/{product}/{version}/{target}/{artifact}
releases/{product}/{version}/{target}/manifest.json
releases/{product}/{version}/{target}/manifest.json.sig
```

Le manifest contient au minimum :

```json
{
  "schema_version": 1,
  "product": "bxc",
  "version": "X.Y.Z",
  "channel": "stable",
  "os": "linux",
  "arch": "x86_64",
  "repository": "aphrody-code/bxc",
  "commit": "40-hex-characters",
  "artifact": "bxc",
  "url": "https://downloads.aphrody.com/...",
  "size": 0,
  "sha256": "64-hex-characters",
  "issued_at": "YYYY-MM-DDTHH:MM:SSZ",
  "expires_at": "YYYY-MM-DDTHH:MM:SSZ",
  "signer_identity": "aphrody-release",
  "signature_algorithm": "Ed25519",
  "key_id": "release-key-YYYY",
  "signature_url": "https://downloads.aphrody.com/.../manifest.json.sig"
}
```

Le producteur sérialise un format JSON canonique versionné une seule fois, puis
la signature détachée couvre exactement ces octets de `manifest.json`. Le CLI :

1. télécharge le manifest et sa signature depuis des hôtes allowlistés ;
2. vérifie la racine de confiance embarquée, l’identité du signataire, les
   métadonnées signées de rotation/révocation, l’expiration et la signature ;
3. télécharge dans un fichier temporaire sur le même filesystem que la cible ;
4. vérifie taille puis SHA-256 ;
5. installe par renommage atomique et conserve une version de rollback ;
6. refuse une version inférieure à la version installée, sauf rollback local
   explicitement demandé et borné ;
7. échoue fermé avant exécution si une preuve diverge.

SHA-256 prouve l’intégrité après acquisition fiable du digest. Ed25519 prouve
seulement la possession de la clé privée associée à une clé publique déjà
approuvée. La provenance exige aussi la racine de confiance, l’identité attendue,
la rotation/révocation signée, l’expiration et l’anti-rollback. BLAKE3 reste
l’identifiant interne des médias et NE REMPLACE PAS la preuve de release.

Cette section entière reste `CIBLE`. La promotion exige aussi des actions CI
pinées par SHA, permissions minimales, identité OIDC courte, SBOM SPDX ou
CycloneDX, attestation de provenance et tests négatifs du vérificateur CLI.

Matrice binaire cible : Linux `x86_64/aarch64`, macOS `x86_64/aarch64`, Windows
`x86_64`. Une plateforme absente déclenche explicitement le fallback source ou
une erreur exploitable ; elle ne télécharge jamais un target approchant.

## 9. MCP public et surface agent-native

Le MCP reste séparé de `aphrody-site` sur `127.0.0.1:8808`.

Contrat cible :

- endpoint Streamable HTTP unique : `POST/GET/DELETE /mcp` selon le protocole ;
- la racine `/` retourne `404`, sans alias implicite vers `/mcp` ;
- `/healthz` public et minimal ; `/mcp` authentifié par Bearer ;
- audience `mcp.aphrody.com`, scope minimal `mcp:read`, jetons courts lorsque
  le fournisseur d’identité le permet ;
- serveur et realm nommés `aphrody-mcp`, jamais `rose-griffon` ou `rg-mcp` ;
- catalogue, compatibilité, documentation, releases et statut en lecture seule ;
- schémas JSON fermés (`additionalProperties: false`) et sorties paginées ;
- entrée maximale 64 KiB, sortie maximale 1 MiB, timeout 15 s, page maximale
  100 éléments, concurrence maximale 4 par identité ;
- aucune commande shell, lecture/écriture de fichiers, SQL, Git, mutation DNS,
  fetch d’URL arbitraire, secret, log brut ou donnée personnelle ;
- journalisation : request-id, outil, durée, verdict et taille ; jamais jeton,
  arguments sensibles, IP brute durable ou réponse complète.

La correction de l’identité live `rose-griffon`/`rg-mcp` est un P0 et précède
l’ajout de tout outil. Après correction, vérifier santé, refus sans jeton, refus
avec mauvais scope et succès avec un jeton de test révoqué après le smoke.

WebMCP navigateur est hors périmètre courant : une page de présentation
statique sans état ni action n’a aucun outil utile à exposer. Il ne sera ajouté
qu’avec une interaction locale réelle et un consentement utilisateur explicite.

## 10. CDN, médias et cache

- HTML vierge : revalidation fréquente, aucune dépendance d’asset ;
- health, version, readiness et status : `Cache-Control: no-store` ;
- catalogues et OpenAPI : ETag, revalidation, pas de cache opaque long ;
- assets adressés par digest : `public, max-age=31536000, immutable` ;
- téléchargements versionnés : `Accept-Ranges: bytes`, type explicite,
  `Content-Disposition`, checksum publié et taille connue ;
- compression Brotli/gzip uniquement si le type et la taille le justifient ;
- aucune compression secondaire des archives déjà compressées ;
- CORS absent par défaut ; origins, méthodes et headers explicitement allowlistés
  lorsqu’un client navigateur documenté l’exige ;
- pas de purge des URLs immuables : publier une nouvelle URL ;
- aucun upload public dans la première version.

Le pipeline média commun normalise le nom, le type MIME, les dimensions, le
poids, la licence, le digest BLAKE3 et le statut de provenance. Niers NE DOIT
PAS publier un média sans licence/provenance validée.

## 11. Sécurité, authentification et confidentialité

### Edge

Nginx DOIT appliquer au minimum :

- TLS moderne, redirection HTTP vers HTTPS et OCSP selon support ;
- HSTS seulement après vérification HTTPS de tous les sous-domaines inclus ;
- `X-Content-Type-Options: nosniff`, `Referrer-Policy`,
  `Permissions-Policy` et CSP restrictive ;
- limites distinctes pour statique, API, downloads et MCP ;
- taille maximale de corps par surface ; `0` pour routes GET-only ;
- propagation d’un request-id validé ou généré ;
- masquage des headers d’autorisation et cookies dans les logs.

Pour la racine vierge, CSP cible : `default-src 'none'; base-uri 'none';
frame-ancestors 'none'; form-action 'none'`. Toute extension future utilise des
hashes/nonces ; jamais `unsafe-inline` ou `unsafe-eval` par commodité.

### Auth

Les routes publiques listées section 7 n’ont pas d’auth. MCP utilise une
identité dédiée en lecture seule. `admin` et `bot` restent `404` jusqu’à une ADR
qui définit utilisateurs, sessions, révocation, MFA, audit et récupération.

Better Auth, NextAuth ou un serveur de session JavaScript NE DOIT PAS être
ajouté à l’origine Rust pour anticiper un besoin. Si une interface admin devient
nécessaire, elle consomme un fournisseur OIDC isolé et le service Rust valide
localement audience, issuer, expiration, signature et scopes via JWKS mis en
cache ; le choix exact reste une ADR et une revue de menace.

### Données

- aucune donnée personnelle dans GitHub, OpenAPI, MCP ou catalogue ;
- aucun token dans `.env.example`, tests, snapshots, commits ou URLs ;
- logs structurés avec allowlist de champs, rétention bornée et IP tronquée ou
  absente ;
- scans avant commit via [scripts/scan-repo.sh](scripts/scan-repo.sh) ;
- tout secret précédemment exposé est considéré compromis et doit être révoqué
  côté fournisseur, indépendamment du caractère privé de la conversation.
- tant que `contact@aphrody.com` n’a pas réussi un test de réception, la route
  `security.txt` utilise un canal du domaine réellement surveillé ou reste
  `NON_CONFORME` ; une adresse syntaxiquement valide n’est pas une preuve.

## 12. DNS et mail

Avant toute mutation DNS, produire un snapshot horodaté contenant, pour chaque
entrée, l’ID OVH, le type, le sous-domaine, la cible et le TTL : A, AAAA, CNAME,
MX, TXT, CAA, `_dmarc` et chaque sélecteur DKIM obtenu de l’inventaire
OVH/fournisseur. Une entrée ou un sélecteur inconnu est noté `ABSENT` ou
`INCOMPLET`, jamais supposé présent.

Règles :

- ne jamais remplacer la zone complète pour ajouter un sous-domaine ;
- ne jamais créer un second SPF ; fusionner dans l’enregistrement existant ;
- ne modifier ni MX, DKIM, DMARC, autodiscover ni validation de fournisseur
  pendant un lot web ;
- abaisser le TTL au moins un TTL complet avant une bascule planifiée ;
- restaurer l’ancienne cible si TLS ou smoke externe échoue ;
- après propagation, comparer depuis au moins deux résolveurs indépendants ;
- la boîte `contact@aphrody.com` reste `BLOQUÉE` tant que création, émission,
  réception, SPF, DKIM et DMARC ne sont pas tous prouvés ;
- aucun mot de passe mail ne doit être manipulé par le code du site.

## 13. Charte graphique future

La charte ne s’applique pas à la racine vierge. Elle gouverne les futures pages
catalogue, téléchargement et vitrines après autorisation explicite.

### Tokens sémantiques

| Token | Valeur sombre de référence | Usage |
|---|---|---|
| `color.canvas` | `#0B0D10` | fond principal |
| `color.surface` | `#12161C` | surface |
| `color.surface-raised` | `#1B222B` | élévation |
| `color.text` | `#F4F1EA` | texte principal |
| `color.text-muted` | `#AAB2BC` | texte secondaire accessible |
| `color.accent` | `#78A8FF` | action Aphrody |
| `color.system` | `#62E6D7` | état agent/système |
| `color.success` | `#7FE0A2` | succès |
| `color.warning` | `#F2C879` | alerte |
| `color.danger` | `#EF8B8B` | erreur |

Les composants utilisent des rôles, jamais une couleur hexadécimale locale.
Toute paire texte/fond est validée WCAG 2.2 AA. L’état ne dépend jamais de la
couleur seule.

### Géométrie et typographie

- grille d’espacement de 4 px ; rayons `8/12/16` px ; bordure 1 px ;
- corps >= 16 px, labels >= 14 px, hauteur de ligne >= 1.4 ;
- police système par défaut ; monospace système pour commandes et digests ;
- cibles tactiles >= 48 × 48 px, sauf densité compacte explicitement testée ;
- breakpoints de référence `600/840/1200/1600` px, guidés par le contenu ;
- largeur de lecture 60–75 caractères ; aucune ligne de texte pleine largeur ;
- mouvement <= 200 ms et désactivé par `prefers-reduced-motion` ;
- aucun gradient décoratif, splash, carousel, scroll hijacking ou skeleton
  bloquant.

### Accessibilité

Toute future page DOIT réussir : navigation clavier complète, focus visible,
ordre DOM logique, landmark/main unique, titres hiérarchiques, noms accessibles,
reflow à 320 px, zoom 200 %, contraste AA et mode réduction de mouvement. Les
assets non décoratifs ont une alternative ; les médias animés sont contrôlables.

Lighthouse, axe ou pa11y ne sont pas installés actuellement sur le VPS. Un agent
DOIT d’abord choisir et épingler un runner de CI reproductible ; jusque-là le gate
automatique accessibilité/performance est `INCOMPLET`, même si la revue manuelle
passe.

## 14. Budgets de performance

| Surface | Budget |
|---|---:|
| racine `aphrody.com/` | HTML brut <= 1 KiB ; 0 JS ; 0 CSS ; 0 font ; 1 requête |
| future page statique | HTML gzip <= 20 KiB ; CSS gzip <= 25 KiB ; JS gzip <= 80 KiB ; <= 15 requêtes |
| API JSON standard | <= 128 KiB avant pagination |
| MCP réponse | <= 1 MiB |
| health origine p95 | <= 100 ms |
| TTFB origine public p95 | <= 300 ms |
| LCP p75 | <= 2,5 s |
| INP p75 | <= 200 ms |
| CLS p75 | <= 0,1 |

Le JavaScript cible des pages publiques est zéro. Le budget de 80 KiB est un
plafond d’exception, pas une invitation. Toute dépendance tierce bloquante,
webfont distante ou hydration globale échoue le gate.

Protocole minimal reproductible : cinq warmups puis 50 requêtes locales
séquentielles pour health/TTFB, horloge monotone, p50/p95/p99 consignés avec
commit et machine ; taille calculée sur les octets réellement transférés ; HTML
parsé pour prouver l’absence de sous-ressource, script et feuille de style. Les
Core Web Vitals p75 proviennent de données terrain sur 30 jours. Lighthouse CI
reste une mesure lab et NE DOIT PAS être présentée comme un p75 terrain.

## 15. Observabilité et santé

- `/healthz` teste uniquement la boucle/processus et ne révèle aucune dépendance ;
- `/readyz` PEUT tester les dépendances futures, reste privé ou sans détail ;
- métriques Prometheus servies sur loopback/protégées, jamais sur l’hôte public ;
- API et MCP émettent logs JSON et métriques RED : débit, erreurs, durée ;
- labels de métriques bornés : route normalisée, méthode, classe de statut ;
- jamais URL brute, query string, user-agent complet, token, email ou product ID
  contrôlé par l’appelant comme label ;
- accès racine vierge sans log applicatif détaillé ; log edge agrégé seulement ;
- alertes sur erreur 5xx, latence p95, absence de health et espace disque ;
- les dashboards/alertes ne passent `ACTIF` qu’avec URL interne ou capture de
  configuration versionnée, pas avec une affirmation.

## 16. Déploiement, SLO, reprise et secours

Le service versionné exécute actuellement un binaire installé directement, sans
symlink de release ni rollback atomique prouvé. Le déploiement suivant est donc
`CIBLE` : build verrouillé, copie sous `/opt/aphrody/releases/<version>/`,
validation, bascule atomique de `/opt/aphrody/releases/current`, restart systemd,
smoke local puis externe. Consigner `readlink` avant/après et tester la commande
inverse. Conserver l’artefact précédent pendant la fenêtre d’observation.

Objectifs :

| Classe | RTO | RPO |
|---|---:|---:|
| statique, API dérivée de Git/releases | <= 15 min | 0 |
| futur état persistant | <= 4 h | <= 24 h |

SLO cibles, non actifs avant disponibilité d’un moniteur : apex `99,95 %` par
mois ; API et MCP `99,9 %` par mois ; taux 5xx, p95/p99 et disponibilité mesurés
sur 30 jours depuis au moins trois régions. Les objectifs de latence de la
section 14 et ces SLO possèdent un error budget et un point de mesure versionnés.
Les RTO/RPO restent eux aussi `CIBLE` jusqu’à un exercice chronométré.

Le fallback des vitrines reste `aphrody-site:8083` tant que leur binaire n’est
pas promu. Une panne d’un site frère ne doit pas rendre Nginx invalide.

Vercel ou OpenAI Sites est une cible de secours statique seulement : bundle
HTML/assets généré, aucun secret, loopback, base, worker, Next.js runtime ou
prétendue réplication stateful. Aucun `.openai/hosting.json` ni déploiement
Vercel actif n’est actuellement une preuve dans ce dépôt ; le secours reste
`CIBLE` jusqu’à exercice chronométré de bascule et retour.

## 17. Plan d’exécution ordonné

Chaque phase produit un commit isolable, un verdict de livraison et une commande
de rollback. Ne pas commencer une phase dépendante si son prédécesseur est
`BLOQUÉ`.

| Phase | Actions | Sortie exigée | Gate/rollback |
|---:|---|---|---|
| 0 | inventaire ports, DNS, mail, systemd, Nginx, binaires, secrets et probes | snapshot daté sans secret | aucune mutation ; corriger l’inventaire |
| 1 | fermer ou corriger identité/auth/allowlist MCP | aucun marqueur RG, tests négatifs/positifs | restaurer service puis fermer `/mcp` |
| 2 | fermer admin/bot, décider frontends, ajouter limites et headers edge | vhosts fail-closed et sécurité testée | restaurer conf Nginx versionnée |
| 3 | extraire le noyau réellement partagé `aphrody-web` | crate `publish=false`, tests, révision pinable | gates Cargo ; revert du commit |
| 4 | migrer `aphrody-site` sur le noyau sans changer l’élément `body` | parité byte/DOM/header/404 | restaurer ancien binaire ou release |
| 5a | ajouter `/version` et `/api/v1/status` | schémas et smokes | désactiver les deux routes |
| 5b | ajouter catalogue, projets et OpenAPI | contrats, fixtures et ETag | désactiver les routes API |
| 5c | ajouter manifests, signatures, vérificateur CLI, downloads et Range | fixtures positives/négatives | restaurer fallback source |
| 5d | ajouter pipeline média et cache CDN | asset licencié, digest et headers | retirer le nouvel asset/route |
| 6 | livrer N2B sur `:8086` | service, vitrine statique, docs repo | remettre proxy sur `:8083` |
| 7 | livrer BXC sur `:8084` sans exposer ses protocoles internes | service et vitrine | remettre proxy sur `:8083` |
| 8 | livrer Niers sur `:8085`, licences média prouvées | service et vitrine | remettre proxy sur `:8083` |
| 9a | ajouter logs, métriques, moniteurs et alertes | SLO mesurables | désactiver export/alertes |
| 9b | préparer puis exercer le secours statique | runbook et chronométrage | retour DNS/VPS prouvé |
| 10 | audit final multi-repo et retrait des fallbacks devenus inutiles | matrice `FAIT/INCOMPLET/NON_FAIT` | ne retirer aucun fallback non prouvé |

Tant que le MCP est `NON_CONFORME`, les phases d’extension API, downloads, CDN
et vitrines NE DOIVENT PAS commencer. La fermeture `admin`/`bot` et le
durcissement edge précèdent également toute nouvelle surface publique.

## 18. Gates et définition de fini

### Gates dépôt obligatoires

Sur Linux/VPS, la variable historique `CARGO_CONFIG` n’est pas une preuve que
Cargo a chargé le fichier. Chaque commande neutralise à la fois une variable
`RUSTC_WRAPPER` héritée et le wrapper de la config, puis charge explicitement la
config Linux. Le gate normatif reste offline ; un diagnostic online ne le
remplace pas.

```bash
git diff --check
git diff --check --cached
python3 scripts/scan-doc-links.py
scripts/scan-repo.sh
env RUSTC_WRAPPER= cargo --config .cargo/config.linux-vps.toml --config "build.rustc-wrapper=''" check --workspace --all-targets --locked --offline
env RUSTC_WRAPPER= cargo --config .cargo/config.linux-vps.toml --config "build.rustc-wrapper=''" clippy --workspace --all-targets --locked --offline -- -D warnings
cargo deny check
```

Pour une modification du site :

```bash
env RUSTC_WRAPPER= cargo --config .cargo/config.linux-vps.toml --config "build.rustc-wrapper=''" test -p aphrody-site --locked --offline
```

Après création/modification de `aphrody-web`, exécuter en plus des gates
workspace :

```bash
env RUSTC_WRAPPER= cargo --config .cargo/config.linux-vps.toml --config "build.rustc-wrapper=''" nextest run -p aphrody-web --locked --offline
```

Un runner axe/pa11y ou équivalent et Lighthouse CI sont pinés avant la première
vitrine, exécutés en CI et restent hors des dépendances runtime.

Dans un arbre initialement non conforme, établir une baseline horodatée avant
édition. Toute défaillance préexistante garde le verdict global `INCOMPLET` ; le
rapport sépare dette baseline et régression du lot. Aucun changement tiers non
staged ne doit être attribué au lot.

### Gates de déploiement

Sur l’hôte autorisé, avec privilèges permettant de lire les certificats :

```bash
sudo nginx -t
systemctl is-active aphrody-site
curl --fail --silent http://127.0.0.1:8083/healthz
curl --fail --silent https://aphrody.com/health
ss -ltn
```

Matrice de probes à automatiser ; chaque assertion consigne code, headers utiles,
digest de réponse et timestamp, jamais un jeton :

| Probe | Attendu |
|---|---|
| `GET/HEAD https://aphrody.com/` | `200`, document HTML <= 1 KiB, DOM `body` sans nœud enfant, aucun sous-asset |
| `GET https://aphrody.com/__unknown__` | `404` |
| `POST/OPTIONS` sur route GET-only | `405` + `Allow` lorsque le contrat cible est livré |
| `GET https://aphrody.com/health` | `200 ok`, `Cache-Control: no-store` |
| `GET admin` et `GET bot` | `404`, aucun upstream |
| résolution/GET `frontends` | NXDOMAIN/absence DNS, ou `404` si réservé |
| `GET mcp.aphrody.com/` | `404` |
| `/mcp` sans Bearer | `401`, realm Aphrody |
| `/mcp` mauvais scope | `403` |
| `/mcp` scope lecture de test | succès borné, puis jeton révoqué |
| BXC/Niers/N2B | fallback `200` vierge avant promotion, origine dédiée après |
| TLS chaque hôte | SAN exact, chaîne valide, expiration > 30 jours |
| ports `8083..8086`, `8808` | écoute loopback dans `ss`; fermés depuis l’extérieur |

Pour la racine, ajouter un test Rust qui parse le document et vérifie un unique
`body`, sans attribut ni nœud enfant, puis compare séparément le rendu source
canonique à la séquence exacte `<body></body>`. Le simple
`contains("<body></body>")` ne suffit pas. Mesurer taille et latence avec le
protocole de la section 14. Les commandes privilégiées exigent l’autorisation
du runbook et ne sont jamais simulées dans un rapport.

### Définition de fini globale

Un lot est `FAIT` seulement si :

- les états `ACTIF/OBSERVÉ/NON_CONFORME/RÉSERVÉ/CIBLE/BLOQUÉ/RETIRÉ` ont été mis
  à jour avec preuves datées ;
- tous les liens, schémas et commandes documentés existent ;
- Cargo check, clippy, deny et tests affectés sortent `0` ;
- aucun secret, donnée personnelle ou identité RG n’est introduit ;
- les budgets des surfaces affectées, la sécurité et la performance sont respectés ;
- le déploiement requis a un smoke local, un smoke externe et un rollback prouvé ;
- le commit est conventionnel, sans trailer ni empreinte d’agent ;
- le rapport final énumère séparément `FAIT`, `INCOMPLET` et `NON_FAIT`.

Un lot documentaire n’autorise pas à déclarer les services cibles déployés. Il
peut être `FAIT` comme contrat tout en laissant les implémentations correspondantes
`INCOMPLET` ou `NON_FAIT`.
